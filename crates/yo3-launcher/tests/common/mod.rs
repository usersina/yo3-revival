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

pub fn game_fixture() -> Vec<u8> {
    // Synthetic PE with only the four existing native hook contracts, not a game binary.
    let mut bytes = vec![0; 0x120000]; let header = pe_fixture(2); bytes[..512].copy_from_slice(&header[..512]);
    bytes[180..184].copy_from_slice(&0x400000u32.to_le_bytes());
    bytes[208..212].copy_from_slice(&0x500000u32.to_le_bytes());
    let section = 128 + 24 + 224; bytes[section..section+8].copy_from_slice(b".text\0\0\0");
    bytes[section+8..section+12].copy_from_slice(&0x11f000u32.to_le_bytes());
    bytes[section+12..section+16].copy_from_slice(&0x1000u32.to_le_bytes());
    bytes[section+16..section+20].copy_from_slice(&0x11f000u32.to_le_bytes());
    bytes[section+20..section+24].copy_from_slice(&0x1000u32.to_le_bytes());
    bytes[section+36..section+40].copy_from_slice(&0x60000020u32.to_le_bytes());
    for (at, signature) in [(0x6964b, [0xe8,0x50,0x6c,0x08,0]), (0xf44bd, [0xe8,0x3e,0x05,0,0]),
        (0x114212, [0xe8,0x79,0,0,0]), (0x1159ea, [0xb8,0x30,0x5a,0x51,0])] {
        bytes[at..at+5].copy_from_slice(&signature);
    }
    bytes
}
