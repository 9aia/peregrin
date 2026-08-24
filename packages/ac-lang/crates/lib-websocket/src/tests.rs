use super::*;

// ── OpCode ──────────────────────────────────────────────────────────

#[test]
fn opcode_roundtrip() {
    for code in [
        OpCode::Continuation,
        OpCode::Text,
        OpCode::Binary,
        OpCode::Close,
        OpCode::Ping,
        OpCode::Pong,
    ] {
        assert_eq!(OpCode::from_u8(code.as_u8()).unwrap(), code);
    }
}

#[test]
fn opcode_invalid() {
    assert!(matches!(OpCode::from_u8(0x0B), Err(Error::InvalidOpCode(0x0B))));
    assert!(matches!(OpCode::from_u8(0xFF), Err(Error::InvalidOpCode(0xFF))));
}

#[test]
fn opcode_is_control() {
    assert!(OpCode::Close.is_control());
    assert!(OpCode::Ping.is_control());
    assert!(OpCode::Pong.is_control());
    assert!(!OpCode::Text.is_control());
    assert!(!OpCode::Binary.is_control());
    assert!(!OpCode::Continuation.is_control());
}

#[test]
fn opcode_is_data() {
    assert!(OpCode::Text.is_data());
    assert!(OpCode::Binary.is_data());
    assert!(OpCode::Continuation.is_data());
    assert!(!OpCode::Close.is_data());
    assert!(!OpCode::Ping.is_data());
    assert!(!OpCode::Pong.is_data());
}

// ── CloseCode / CloseFrame ──────────────────────────────────────────

#[test]
fn close_code_constants() {
    assert_eq!(CloseCode::NORMAL.0, 1000);
    assert_eq!(CloseCode::GOING_AWAY.0, 1001);
    assert_eq!(CloseCode::PROTOCOL_ERROR.0, 1002);
    assert_eq!(CloseCode::UNSUPPORTED_DATA.0, 1003);
    assert_eq!(CloseCode::MESSAGE_TOO_BIG.0, 1009);
    assert_eq!(CloseCode::TLS_HANDSHAKE.0, 1015);
}

#[test]
fn close_code_bytes_roundtrip() {
    let code = CloseCode::from(3000);
    assert_eq!(CloseCode::from_be_bytes(code.to_be_bytes()), code);
}

#[test]
fn close_frame_roundtrip() {
    let original = CloseFrame::new(CloseCode::NORMAL, "goodbye");
    let bytes = original.encode();
    assert_eq!(bytes, [0x03, 0xE8, b'g', b'o', b'o', b'd', b'p', b'y', b'e']);
    let decoded = CloseFrame::decode(&bytes).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn close_frame_empty_reason() {
    let frame = CloseFrame::new(CloseCode::NORMAL, "");
    assert_eq!(frame.encode(), [0x03, 0xE8]);
    let decoded = CloseFrame::decode(&[0x03, 0xE8]).unwrap();
    assert_eq!(decoded.reason, "");
}

#[test]
fn close_frame_too_short() {
    assert!(matches!(CloseFrame::decode(&[0x03]), Err(Error::InvalidCloseFrame)));
}

#[test]
fn close_frame_invalid_utf8() {
    let bytes = vec![0x03, 0xE8, 0xC0, 0x80];
    assert!(matches!(CloseFrame::decode(&bytes), Err(Error::InvalidUtf8)));
}

// ── Mask ────────────────────────────────────────────────────────────

#[test]
fn mask_double_apply_is_identity() {
    let key = [0x12, 0x34, 0x56, 0x78];
    let original = b"The quick brown fox jumps over the lazy dog".to_vec();
    let mut masked = original.clone();
    mask::apply(&mut masked, key);
    assert_ne!(masked, original);
    mask::apply(&mut masked, key);
    assert_eq!(masked, original);
}

#[test]
fn mask_empty_payload() {
    let mut payload = Vec::new();
    mask::apply(&mut payload, [1, 2, 3, 4]);
    assert!(payload.is_empty());
}

#[test]
fn mask_short_payload() {
    let mut payload = vec![0x00, 0x01, 0x02];
    mask::apply(&mut payload, [0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(payload, vec![0xFF, 0xFE, 0xFD]);
}

// ── Frame encoding / decoding ───────────────────────────────────────

#[test]
fn frame_roundtrip_text_unmasked() {
    let frame = Frame::new(OpCode::Text, b"hello".to_vec());
    let bytes = frame.encode();
    let (decoded, consumed) = Frame::decode(&bytes).unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(decoded, frame);
    assert_eq!(decoded.payload, b"hello");
}

#[test]
fn frame_roundtrip_text_masked() {
    let frame = Frame::new(OpCode::Text, b"hello".to_vec()).with_mask([0xAB, 0xCD, 0xEF, 0x01]);
    let bytes = frame.encode();
    let (decoded, consumed) = Frame::decode(&bytes).unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(decoded, frame);
    assert_eq!(decoded.payload, b"hello");
    assert_eq!(decoded.mask, Some([0xAB, 0xCD, 0xEF, 0x01]));
}

#[test]
fn frame_roundtrip_binary() {
    let payload: Vec<u8> = (0..=255).collect();
    let frame = Frame::new(OpCode::Binary, payload.clone());
    let bytes = frame.encode();
    let (decoded, _) = Frame::decode(&bytes).unwrap();
    assert_eq!(decoded.payload, payload);
}

#[test]
fn frame_roundtrip_ping() {
    let frame = Frame::new(OpCode::Ping, vec![0x01, 0x02, 0x03]);
    let bytes = frame.encode();
    let (decoded, _) = Frame::decode(&bytes).unwrap();
    assert_eq!(decoded, frame);
}

#[test]
fn frame_roundtrip_close() {
    let close = CloseFrame::new(CloseCode::NORMAL, "done");
    let frame = Frame::new(OpCode::Close, close.encode());
    let bytes = frame.encode();
    let (decoded, _) = Frame::decode(&bytes).unwrap();
    assert_eq!(decoded, frame);
}

#[test]
fn frame_small_payload_length_126() {
    let payload = vec![0xABu8; 200];
    let frame = Frame::new(OpCode::Binary, payload.clone()).with_mask([1, 2, 3, 4]);
    let bytes = frame.encode();
    // 2 base header + 2 extended length + 4 mask + 200 payload
    assert_eq!(bytes.len(), 2 + 2 + 4 + 200);
    let (decoded, _) = Frame::decode(&bytes).unwrap();
    assert_eq!(decoded.payload, payload);
}

#[test]
fn frame_large_payload_length_127() {
    let payload = vec![0xCDu8; 65536];
    let frame = Frame::new(OpCode::Binary, payload.clone());
    let bytes = frame.encode();
    // 2 base header + 8 extended length + 65536 payload (no mask)
    assert_eq!(bytes.len(), 2 + 8 + 65536);
    let (decoded, _) = Frame::decode(&bytes).unwrap();
    assert_eq!(decoded.payload, payload);
}

#[test]
fn frame_incomplete_header() {
    assert!(matches!(Frame::decode(&[]), Err(Error::Incomplete)));
    assert!(matches!(Frame::decode(&[0x81]), Err(Error::Incomplete)));
}

#[test]
fn frame_incomplete_126_length() {
    let bytes = [0x81, 0x7E, 0x00];
    assert!(matches!(Frame::decode(&bytes), Err(Error::Incomplete)));
}

#[test]
fn frame_incomplete_127_length() {
    let bytes = [0x81, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(matches!(Frame::decode(&bytes), Err(Error::Incomplete)));
}

#[test]
fn frame_incomplete_payload() {
    let bytes = [0x81, 0x05, b'h', b'e', b'l', b'l'];
    assert!(matches!(Frame::decode(&bytes), Err(Error::Incomplete)));
}

#[test]
fn frame_incomplete_mask() {
    let bytes = [0x81, 0x85, b'h', b'e'];
    assert!(matches!(Frame::decode(&bytes), Err(Error::Incomplete)));
}

#[test]
fn frame_reserved_bits_rejected() {
    let frame = Frame::new(OpCode::Text, b"x".to_vec()).with_rsv(true, false, false);
    let bytes = frame.encode();
    assert!(matches!(Frame::decode(&bytes), Err(Error::ReservedBits)));
}

#[test]
fn frame_fragmented_control_rejected() {
    let payload = vec![0u8; 126];
    let frame = Frame::new(OpCode::Ping, payload);
    let bytes = frame.encode();
    assert!(matches!(Frame::decode(&bytes), Err(Error::FragmentedControl)));
}

#[test]
fn frame_non_final_control_rejected() {
    let frame = Frame::new(OpCode::Ping, vec![]).with_fin(false);
    let bytes = frame.encode();
    assert!(matches!(Frame::decode(&bytes), Err(Error::FragmentedControl)));
}

#[test]
fn frame_invalid_opcode_rejected() {
    let bytes = [0x8B, 0x80, 0x00, 0x00, 0x00, 0x00];
    assert!(matches!(Frame::decode(&bytes), Err(Error::InvalidOpCode(0x0B))));
}

// ── Message ─────────────────────────────────────────────────────────

#[test]
fn message_text_roundtrip_unmasked() {
    let msg = Message::Text("hello".to_string());
    let frame = msg.into_frame(None);
    let (decoded, _) = Frame::decode(&frame.encode()).unwrap();
    let decoded_msg = Message::from_frame(&decoded).unwrap();
    assert_eq!(decoded_msg, Message::Text("hello".to_string()));
}

#[test]
fn message_text_roundtrip_masked() {
    let msg = Message::Text("hello".to_string());
    let frame = msg.into_frame(Some([0xFF, 0x00, 0xFF, 0x00]));
    let (decoded, _) = Frame::decode(&frame.encode()).unwrap();
    let decoded_msg = Message::from_frame(&decoded).unwrap();
    assert_eq!(decoded_msg, Message::Text("hello".to_string()));
}

#[test]
fn message_binary_roundtrip() {
    let payload = vec![0u8, 1, 2, 255];
    let msg = Message::Binary(payload);
    let frame = msg.clone().into_frame(None);
    let (decoded, _) = Frame::decode(&frame.encode()).unwrap();
    let decoded_msg = Message::from_frame(&decoded).unwrap();
    assert_eq!(decoded_msg, msg);
}

#[test]
fn message_ping_roundtrip() {
    let msg = Message::Ping(vec![0x01, 0x02]);
    let frame = msg.clone().into_frame(None);
    let (decoded, _) = Frame::decode(&frame.encode()).unwrap();
    let decoded_msg = Message::from_frame(&decoded).unwrap();
    assert_eq!(decoded_msg, msg);
}

#[test]
fn message_close_roundtrip_with_reason() {
    let msg = Message::Close(Some(CloseFrame::new(CloseCode::NORMAL, "done")));
    let frame = msg.clone().into_frame(None);
    let (decoded, _) = Frame::decode(&frame.encode()).unwrap();
    let decoded_msg = Message::from_frame(&decoded).unwrap();
    assert_eq!(decoded_msg, msg);
}

#[test]
fn message_close_roundtrip_no_reason() {
    let msg = Message::Close(None);
    let frame = msg.clone().into_frame(None);
    let (decoded, _) = Frame::decode(&frame.encode()).unwrap();
    let decoded_msg = Message::from_frame(&decoded).unwrap();
    assert_eq!(decoded_msg, msg);
}

#[test]
fn message_invalid_utf8_rejected() {
    let invalid = vec![0x80, 0x81, 0x82];
    let frame = Frame::new(OpCode::Text, invalid);
    assert!(matches!(Message::from_frame(&frame), Err(Error::InvalidUtf8)));
}

#[test]
fn message_unfragmented_continuation_rejected() {
    let frame = Frame::new(OpCode::Continuation, b"x".to_vec());
    assert!(matches!(Message::from_frame(&frame), Err(Error::UnexpectedContinuation)));
}

// ── Reassembler ─────────────────────────────────────────────────────

#[test]
fn reassembler_single_text() {
    let mut asm = Reassembler::new();
    let frame = Frame::new(OpCode::Text, b"hello".to_vec());
    let msg = asm.feed(&frame).unwrap();
    assert_eq!(msg, Some(Message::Text("hello".to_string())));
    assert!(asm.buffer.is_empty());
    assert!(asm.start_opcode.is_none());
}

#[test]
fn reassembler_fragmented_text() {
    let mut asm = Reassembler::new();
    let start = Frame::new(OpCode::Text, b"hel".to_vec()).with_fin(false);
    let cont = Frame::new(OpCode::Continuation, b"lo".to_vec());

    assert_eq!(asm.feed(&start).unwrap(), None);
    assert_eq!(asm.feed(&cont).unwrap(), Some(Message::Text("hello".to_string())));
}

#[test]
fn reassembler_fragmented_binary() {
    let mut asm = Reassembler::new();
    let start = Frame::new(OpCode::Binary, vec![0, 1, 2]).with_fin(false);
    let cont = Frame::new(OpCode::Continuation, vec![3, 4, 5]);

    assert_eq!(asm.feed(&start).unwrap(), None);
    assert_eq!(asm.feed(&cont).unwrap(), Some(Message::Binary(vec![0, 1, 2, 3, 4, 5])));
}

#[test]
fn reassembler_interleaved_control() {
    let mut asm = Reassembler::new();
    let start = Frame::new(OpCode::Text, b"he".to_vec()).with_fin(false);
    let ping = Frame::new(OpCode::Ping, vec![0x01]);
    let cont = Frame::new(OpCode::Continuation, b"llo".to_vec());

    assert_eq!(asm.feed(&start).unwrap(), None);
    assert_eq!(asm.feed(&ping).unwrap(), Some(Message::Ping(vec![0x01])));
    assert_eq!(asm.feed(&cont).unwrap(), Some(Message::Text("hello".to_string())));
}

#[test]
fn reassembler_unexpected_continuation() {
    let mut asm = Reassembler::new();
    let cont = Frame::new(OpCode::Continuation, b"x".to_vec());
    assert!(matches!(asm.feed(&cont), Err(Error::UnexpectedContinuation)));
}

#[test]
fn reassembler_unexpected_data_during_reassembly() {
    let mut asm = Reassembler::new();
    let start = Frame::new(OpCode::Text, b"he".to_vec()).with_fin(false);
    let bad = Frame::new(OpCode::Text, b"x".to_vec()).with_fin(false);

    assert_eq!(asm.feed(&start).unwrap(), None);
    assert!(matches!(asm.feed(&bad), Err(Error::UnexpectedContinuation)));
}

#[test]
fn reassembler_fragmented_control_rejected() {
    let mut asm = Reassembler::new();
    let ping = Frame::new(OpCode::Ping, vec![0x01]).with_fin(false);
    assert!(matches!(asm.feed(&ping), Err(Error::FragmentedControl)));
}

#[test]
fn reassembler_oversized_control_rejected() {
    let mut asm = Reassembler::new();
    let ping = Frame::new(OpCode::Ping, vec![0u8; 126]);
    assert!(matches!(asm.feed(&ping), Err(Error::FragmentedControl)));
}
