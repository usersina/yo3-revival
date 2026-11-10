pub fn pe_fixture(abi: u32) -> Vec<u8> {
    let mut bytes = vec![0; 768]; bytes[..2].copy_from_slice(b"MZ"); bytes[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
    bytes[128..132].copy_from_slice(b"PE\0\0"); bytes[132..134].copy_from_slice(&0x14cu16.to_le_bytes());
    bytes[134..136].copy_from_slice(&1u16.to_le_bytes()); bytes[148..150].copy_from_slice(&224u16.to_le_bytes());
    bytes[152..154].copy_from_slice(&0x10bu16.to_le_bytes());
    let section = 128 + 24 + 224; bytes[section..section+4].copy_from_slice(b".yo3");
    bytes[section+16..section+20].copy_from_slice(&128u32.to_le_bytes()); bytes[section+20..section+24].copy_from_slice(&512u32.to_le_bytes());
    bytes[512..520].copy_from_slice(b"YO3MOD02"); bytes[520..524].copy_from_slice(&abi.to_le_bytes());
    let version = yo3_tunnel::BUILD_VERSION.as_bytes(); bytes[524..524+version.len()].copy_from_slice(version);
    let commit = yo3_tunnel::BUILD_COMMIT.as_bytes(); bytes[589..589+commit.len().min(40)].copy_from_slice(&commit[..commit.len().min(40)]);
    bytes
}
