//! The words on the player's path (#31): one plain sentence per state and error. Paths, flag names, file names of our
//! own parts and error chains belong behind **Details** only. [`technical`] is the check the UI text tests and the
//! error presentation share.

/// Why `text` isn't fit for the player's path, if it isn't: a path, a flag, an internal file or term, an error chain.
pub fn technical(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    if text.as_bytes().windows(3).any(|w| w[0] == b'-' && w[1] == b'-' && w[2].is_ascii_alphanumeric()) { return Some("a flag name") }
    for token in text.split_whitespace() {
        let token = token.trim_matches(|c: char| matches!(c, '(' | ')' | '"' | '\'' | ',' | '.' | ';' | '`' | '«' | '»'));
        if token.contains("://") { continue }
        if (token.contains('/') || token.contains('\\')) && token.chars().any(char::is_alphabetic) { return Some("a path") }
        if token.len() > 2 && token.as_bytes()[1] == b':' && token.as_bytes()[0].is_ascii_alphabetic() && matches!(token.as_bytes()[2], b'\\' | b'/') { return Some("a path") }
    }
    for (word, why) in [("os error", "an error chain"), ("exit status", "an error chain"), ("exit code", "an error chain"), ("stage=", "native evidence"),
        ("0x", "a hexadecimal code"), (".dll", "an internal file"), (".json", "an internal file"), (".db", "an internal file"), (".ini", "an internal file"),
        (".crt", "an internal file"), (".toml", "an internal file"), (".log", "an internal file"), ("account key", "the old account key"),
        ("20-character", "the old account key"), ("winmm", "an internal file"), ("sha-256", "a checksum"), ("sha256", "a checksum"), ("manifest", "an internal term"),
        ("mod conflict", "an internal term"), ("rules engine unavailable", "an internal term"), ("<app", "a masked path"), ("<game>", "a masked path"),
        ("<user>", "a masked path"), ("<server data>", "a masked path"), ("<profile>", "a masked path"), ("{error", "an error chain"), ("{e:#", "an error chain")] {
        if lower.contains(word) { return Some(why) }
    }
    if text.matches(": ").count() >= 2 { return Some("an error chain") }
    None
}

#[cfg(test)]
mod tests {
    use super::technical;

    #[test]
    fn paths_flags_internal_files_and_chains_are_technical() {
        for text in ["C:\\Games\\Yu-Gi-Oh! ONLINE 3", "/home/me/.wine", "Use --bind 0.0.0.0", "winmm.dll is missing", "your 20-character account key",
            "Mod conflict", "Connection check failed: TLS handshake: reset (os error 10054)", "stage=game-exit", "<app data>/logs", "The launcher.db is locked"] {
            assert!(technical(text).is_some(), "{text}");
        }
        for text in ["We can't find your game.", "Paste an invite link (yo3revival://…) a host sent you.", "Press PLAY.", "1024 × 768",
            "The User ID or password is wrong.", "Opening the game without the launcher needs permission once.", "play.example.com or 203.0.113.7"] {
            assert_eq!(technical(text), None, "{text}");
        }
    }
}
