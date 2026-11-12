//! Deliberate coverage classifications, not a default "ignored" arm. Sources: ocgcore-wasm OcgMessageType and
//! the native dispatchers documented in docs/protocol.md. Interaction coverage is not a claim to prove legal menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage { State, Interaction, Presentation, Unsupported, Unknown }
pub fn native(id: u32) -> Coverage {
    match id {
        5 | 6 | 0xA | 0xC..=0x10 | 0x12 | 0x16 | 0x1A..=0x1C | 0x1F | 0x27 | 1000..=1003 | 1017 => Coverage::State,
        4 | 0x36 | 0x37 | 0x4C | 1004 | 1005 | 1007 | 1008 | 1010 | 1011 => Coverage::Interaction,
        // 7, 0x1E, 0x22: the field's background, the Set and activation effects; 0x43-0x4D: the avatars' speech.
        1 | 2 | 7 | 0x13 | 0x1E | 0x22 | 0x26 | 0x33 | 0x34 | 0x38 | 0x39 | 0x43 | 0x44 | 0x47 | 0x48 | 0x4D | 1500..=1505
            | super::native::CHECKPOINT => Coverage::Presentation,
        _ => Coverage::Unknown,
    }
}
pub fn engine(id: u64) -> Coverage {
    match id {
        33 | 35 | 36 | 40 | 41 | 50 | 53 | 55 | 64 | 90..=94 | 96 | 97 | 100..=102 | 110..=114 => Coverage::State,
        10..=16 | 18..=26 | 31 | 132 | 140..=143 => Coverage::Interaction,
        // Movement/queries carry SET and completed summon state; these messages add no further inventory changes.
        1..=5 | 32 | 54 | 60..=63 | 65 | 70..=76 | 80 | 81 | 83 | 120..=123 | 130 | 131 | 133
            | 160 | 163..=165 => Coverage::Presentation,
        // Includes deck reveals/reversal, disabled zones, tag/reload and new message types: not silently ignored.
        6..=8 | 30 | 34 | 37..=39 | 42 | 56 | 161 | 162 | 170 | 180 | 190 => Coverage::Unsupported,
        _ => Coverage::Unknown,
    }
}
