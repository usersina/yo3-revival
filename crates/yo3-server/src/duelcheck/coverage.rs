//! Deliberate coverage classifications, not a default "ignored" arm. Sources: ocgcore-wasm OcgMessageType and
//! the native dispatchers documented in docs/protocol.md. Interaction coverage is not a claim to prove legal menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage { State, Interaction, Presentation, Unsupported, Unknown }
/// What the client model makes of a command the server sent: the table's ([`crate::client_commands`]) for one it marks
/// sent, Unknown for any other (the duel check fails on it). The server's own markers aren't client commands: the
/// native checkpoint and the clocks' settle marker (#11), both answered by the connection DLL.
pub fn native(id: u32) -> Coverage {
    if id == super::native::CHECKPOINT || id == crate::clock::SETTLE {
        return Coverage::Presentation;
    }
    match crate::client_commands::command(id).map(|c| c.usage) {
        Some(crate::client_commands::Use::Sent(coverage)) => coverage,
        _ => Coverage::Unknown,
    }
}
pub fn engine(id: u64) -> Coverage {
    match id {
        33 | 35 | 36 | 40 | 41 | 50 | 53 | 55 | 64 | 90..=94 | 96 | 97 | 100..=102 | 110..=114 => Coverage::State,
        10..=16 | 18..=26 | 31 | 132 | 140..=143 => Coverage::Interaction,
        // Movement/queries carry SET and completed summon state; these messages add no further inventory changes.
        1..=5 | 30 | 32 | 37 | 38 | 54 | 56 | 60..=63 | 65 | 70..=76 | 80 | 81 | 83 | 120..=123 | 130 | 131 | 133
            | 160 | 163..=165 => Coverage::Presentation,
        // Includes deck reveals/reversal, disabled zones, tag/reload and new message types: not silently ignored.
        6..=8 | 34 | 39 | 42 | 161 | 162 | 170 | 180 | 190 => Coverage::Unsupported,
        _ => Coverage::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_settle_marker_is_presentation() {
        // 0x5F1 {seq} ends a batch for the clocks (#11): the connection DLL answers once it has played, and no card moves.
        // Unlisted, every checked duel failed at its first batch (the monkey's soak with --players, 2026-11-16).
        assert_eq!(native(crate::clock::SETTLE), Coverage::Presentation);
    }
}
