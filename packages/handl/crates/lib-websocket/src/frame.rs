use crate::{Error, OpCode, mask};

/// A single WebSocket frame.
///
/// The payload is always stored unmasked, regardless of whether `mask`
/// is set.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// FIN bit: `true` if this is the final fragment.
    pub fin: bool,
    /// RSV1 bit.
    pub rsv1: bool,
    /// RSV2 bit.
    pub rsv2: bool,
    /// RSV3 bit.
    pub rsv3: bool,
    /// Frame opcode.
    pub opcode: OpCode,
    /// Masking key when the frame is masked. The payload is unmasked.
    pub mask: Option<[u8; 4]>,
    /// Unmasked payload bytes.
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new, final, unmasked frame.
    pub fn new(opcode: OpCode, payload: Vec<u8>) -> Self {
        Self {
            fin: true,
            rsv1: false,
            rsv2: false,
            rsv3: false,
            opcode,
            mask: None,
            payload,
        }
    }

    /// Set the FIN bit.
    pub fn with_fin(mut self, fin: bool) -> Self {
        self.fin = fin;
        self
    }

    /// Set the masking key.
    pub fn with_mask(mut self, key: [u8; 4]) -> Self {
        self.mask = Some(key);
        self
    }

    /// Set the masking key from an `Option`.
    pub fn with_mask_opt(mut self, key: Option<[u8; 4]>) -> Self {
        self.mask = key;
        self
    }

    /// Set the reserved bits.
    pub fn with_rsv(mut self, rsv1: bool, rsv2: bool, rsv3: bool) -> Self {
        self.rsv1 = rsv1;
        self.rsv2 = rsv2;
        self.rsv3 = rsv3;
        self
    }

    /// Return the total wire size of this frame in bytes.
    pub fn wire_size(&self) -> usize {
        2 + self.mask.map_or(0, |_| 4) + self.payload_len_bytes() + self.payload.len()
    }

    fn payload_len_bytes(&self) -> usize {
        match self.payload.len() {
            0..=125 => 0,
            126..=65535 => 2,
            _ => 8,
        }
    }

    /// Encode this frame into a byte vector.
    ///
    /// If a mask is present the encoded payload is masked on the fly.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.wire_size());

        let mut b0 = 0u8;
        if self.fin {
            b0 |= 0x80;
        }
        if self.rsv1 {
            b0 |= 0x40;
        }
        if self.rsv2 {
            b0 |= 0x20;
        }
        if self.rsv3 {
            b0 |= 0x10;
        }
        b0 |= self.opcode.as_u8();
        out.push(b0);

        let len = self.payload.len();
        let mut b1 = if self.mask.is_some() { 0x80 } else { 0x00 };
        match len {
            n @ 0..=125 => {
                b1 |= n as u8;
                out.push(b1);
            }
            n @ 126..=65535 => {
                b1 |= 126;
                out.push(b1);
                out.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                b1 |= 127;
                out.push(b1);
                out.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }

        if let Some(key) = self.mask {
            out.extend_from_slice(&key);
            for (i, b) in self.payload.iter().enumerate() {
                out.push(b ^ key[i % 4]);
            }
        } else {
            out.extend_from_slice(&self.payload);
        }

        out
    }

    /// Decode a single frame from a byte slice.
    ///
    /// Returns the decoded frame and the number of consumed bytes.
    pub fn decode(input: &[u8]) -> Result<(Self, usize), Error> {
        if input.len() < 2 {
            return Err(Error::Incomplete);
        }

        let b0 = input[0];
        let fin = b0 & 0x80 != 0;
        let rsv1 = b0 & 0x40 != 0;
        let rsv2 = b0 & 0x20 != 0;
        let rsv3 = b0 & 0x10 != 0;
        let opcode = OpCode::from_u8(b0 & 0x0F)?;

        if rsv1 || rsv2 || rsv3 {
            return Err(Error::ReservedBits);
        }

        let b1 = input[1];
        let masked = b1 & 0x80 != 0;
        let mut payload_len: u64 = (b1 & 0x7F) as u64;
        let mut cursor: usize = 2;

        match payload_len {
            126 => {
                if input.len() < 4 {
                    return Err(Error::Incomplete);
                }
                payload_len = u16::from_be_bytes([input[2], input[3]]) as u64;
                cursor = 4;
            }
            127 => {
                if input.len() < 10 {
                    return Err(Error::Incomplete);
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&input[2..10]);
                payload_len = u64::from_be_bytes(bytes);
                cursor = 10;
            }
            _ => {}
        }

        if opcode.is_control() && (!fin || payload_len > 125) {
            return Err(Error::FragmentedControl);
        }

        let len = usize::try_from(payload_len).map_err(|_| Error::InvalidPayloadLength)?;

        let mut mask_key = None;
        if masked {
            if input.len() < cursor + 4 {
                return Err(Error::Incomplete);
            }
            mask_key = Some([
                input[cursor],
                input[cursor + 1],
                input[cursor + 2],
                input[cursor + 3],
            ]);
            cursor += 4;
        }

        if input.len() < cursor + len {
            return Err(Error::Incomplete);
        }

        let mut payload = input[cursor..cursor + len].to_vec();
        if let Some(key) = mask_key {
            mask::apply(&mut payload, key);
        }
        cursor += len;

        Ok((
            Self {
                fin,
                rsv1,
                rsv2,
                rsv3,
                opcode,
                mask: mask_key,
                payload,
            },
            cursor,
        ))
    }
}
