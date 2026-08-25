use handl_lib_archive::{Archive, ArchiveEntry};

use super::TarArchive;

#[test]
fn roundtrip_single() {
    let entries = vec![ArchiveEntry::new("hello.txt", b"hello, world")];
    let packed = TarArchive.pack(&entries).unwrap();
    assert!(!packed.is_empty());

    let unpacked = TarArchive.unpack(&packed).unwrap();
    assert_eq!(unpacked, entries);
}

#[test]
fn roundtrip_multiple() {
    let entries = vec![
        ArchiveEntry::new("a.txt", b"alpha"),
        ArchiveEntry::new("b/c.txt", b"nested"),
        ArchiveEntry::new("empty", b""),
    ];
    let packed = TarArchive.pack(&entries).unwrap();
    let unpacked = TarArchive.unpack(&packed).unwrap();
    assert_eq!(unpacked, entries);
}

#[test]
fn invalid_data_fails_unpack() {
    assert!(TarArchive.unpack(b"not a tar archive").is_err());
}
