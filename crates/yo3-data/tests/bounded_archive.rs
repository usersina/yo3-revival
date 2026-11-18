//! Image reads must not trust an archive's advertised allocation or inflated size.
use std::{io::Write, path::PathBuf};
fn archive(name: &str, method: u16, size: u32, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("yo3-bounded-{name}-{}.ypk", std::process::id()));
    let mut h = [0u8;30]; h[..4].copy_from_slice(b"YO\x03\x04");
    h[8..10].copy_from_slice(&method.to_le_bytes()); h[18..22].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    h[22..26].copy_from_slice(&size.to_le_bytes()); h[26..28].copy_from_slice(&5u16.to_le_bytes());
    std::fs::write(&path, [h.as_slice(), b"a.png", bytes].concat()).unwrap(); path
}
#[test]
fn bounded_reads_reject_oversized_dishonest_or_unsupported_entries() {
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&[0;4096]).unwrap(); let compressed = encoder.finish().unwrap();
    for (name, method, size, bytes) in [("large",0,4096,vec![0;4096]), ("bomb",8,1,compressed), ("short",0,7,vec![1]), ("method",9,1,vec![1])] {
        let path=archive(name,method,size,&bytes);
        assert!(yo3_data::Ypk::open(&path).unwrap().read_bounded("a.png",1024).is_err(),"{name}");
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn bounded_reads_preserve_valid_bytes_and_missing_entries() {
    let path=archive("valid",0,3,b"png"); let ypk=yo3_data::Ypk::open(&path).unwrap();
    assert_eq!(ypk.read_bounded("a.png",3).unwrap(),Some(b"png".to_vec()));
    assert_eq!(ypk.read_bounded("absent",3).unwrap(),None);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn bounded_reads_accept_normal_deflated_images() {
    let mut encoder=flate2::write::DeflateEncoder::new(Vec::new(),flate2::Compression::default());
    encoder.write_all(b"png").unwrap();
    let path=archive("deflated",8,3,&encoder.finish().unwrap());
    assert_eq!(yo3_data::Ypk::open(&path).unwrap().read_bounded("a.png",16).unwrap(),Some(b"png".to_vec()));
    std::fs::remove_file(path).unwrap();
}
