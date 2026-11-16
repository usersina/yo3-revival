//! The check over the UI texts (#31): no path, flag name, internal file or error chain on the player's path. Every
//! text the window's code writes is checked with `words::technical`, and so is every sentence an error becomes: the
//! library's own error messages are fed through the error presentation, whose sentence must be plain (their technical
//! text goes behind Details).
use yo3_desktop::words::technical;

/// The window's code, up to its tests.
const SCREENS: [(&str, &str); 7] = [
    ("app.rs", include_str!("app.rs")), ("home.rs", include_str!("home.rs")), ("multi.rs", include_str!("multi.rs")), ("setup.rs", include_str!("setup.rs")),
    ("settings.rs", include_str!("settings.rs")), ("help.rs", include_str!("help.rs")), ("host.rs", include_str!("host.rs")),
];

/// Library code whose error messages can reach a notice.
const LIBRARY: [(&str, &str); 8] = [
    ("lib.rs", include_str!("lib.rs")), ("singleplayer.rs", include_str!("singleplayer.rs")), ("direct.rs", include_str!("direct.rs")),
    ("game_data.rs", include_str!("game_data.rs")), ("options.rs", include_str!("options.rs")), ("hosting.rs", include_str!("hosting.rs")),
    ("reports.rs", include_str!("reports.rs")), ("external.rs", include_str!("external.rs")),
];

/// The string literals of `source` before its tests, with their line numbers.
pub fn literals(source: &str) -> Vec<(usize, String)> {
    let source = source.split("#[cfg(test)]").next().unwrap_or(source);
    let mut out = Vec::new();
    let bytes: Vec<char> = source.chars().collect();
    let (mut i, mut line) = (0, 1);
    while i < bytes.len() {
        let c = bytes[i];
        if c == '\n' { line += 1 }
        // Comments and character literals hold no texts.
        if c == '/' && bytes.get(i + 1) == Some(&'/') { while i < bytes.len() && bytes[i] != '\n' { i += 1 } continue }
        if c == '\'' {
            if bytes.get(i + 2) == Some(&'\'') { i += 3; continue }
            if bytes.get(i + 1) == Some(&'\\') && let Some(end) = (i + 2..(i + 12).min(bytes.len())).find(|&j| bytes[j] == '\'') { i = end + 1; continue }
            i += 1; continue;
        }
        if c == '"' {
            let (start, mut text) = (line, String::new());
            i += 1;
            while i < bytes.len() && bytes[i] != '"' {
                if bytes[i] == '\\' {
                    i += 1;
                    match bytes.get(i) {
                        Some('n') => text.push('\n'),
                        Some('\n') => { line += 1; i += 1; while i < bytes.len() && bytes[i].is_whitespace() { if bytes[i] == '\n' { line += 1 } i += 1 } continue }
                        Some(other) => text.push(*other),
                        None => {}
                    }
                } else {
                    if bytes[i] == '\n' { line += 1 }
                    text.push(bytes[i]);
                }
                i += 1;
            }
            out.push((start, text));
        }
        i += 1;
    }
    out
}

/// Whether the literal on `line` of `source` is shown to the player: not a diagnostics line, a link, a key or a
/// format string that only carries a value.
fn shown(source: &str, line: usize, text: &str) -> bool {
    let code = source.lines().nth(line - 1).unwrap_or_default();
    let words = text.split_whitespace().count();
    // `→ Details` marks an error text handed to a notice, whose sentence the next test checks.
    !(code.contains("diagnostics.line(") || code.contains("→ Details") || code.contains("include_bytes!") || text.starts_with('<') && text.ends_with('>') || text.starts_with("https://") || text.starts_with("http://")
        || text.contains("{e:#}") && words <= 1 || text.contains("{error:#}") && words <= 1 || words == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_presentation::{Hint, present};

    #[test]
    fn no_text_on_the_players_path_is_technical() {
        let mut found = Vec::new();
        for (file, source) in SCREENS {
            for (line, text) in literals(source) {
                if shown(source, line, &text) && let Some(why) = technical(&text) { found.push(format!("{file}:{line}: {why}: {text:?}")) }
            }
        }
        // Setup's checklist: each line's sentence (its details are behind Details).
        let checklist = include_str!("setup_check.rs");
        for (line, text) in literals(checklist) {
            let code = checklist.lines().nth(line - 1).unwrap_or_default();
            if code.contains("Line::new(") && shown(checklist, line, &text) && let Some(why) = technical(&text) { found.push(format!("setup_check.rs:{line}: {why}: {text:?}")) }
        }
        for (n, line) in include_str!("help.txt").lines().enumerate() {
            let text = line.split(" | https://").next().unwrap_or(line);
            if let Some(why) = technical(text) { found.push(format!("help.txt:{}: {why}: {line:?}", n + 1)) }
        }
        assert!(found.is_empty(), "technical words on the player's path (move them behind Details):\n{}", found.join("\n"));
    }

    #[test]
    fn every_error_message_becomes_a_plain_sentence() {
        let mut found = Vec::new();
        let mut checked = 0;
        for (file, source) in LIBRARY {
            for (line, text) in literals(source) {
                // Error messages are sentences: a capital, a few words.
                if text.split_whitespace().count() < 3 || !text.starts_with(|c: char| c.is_uppercase()) { continue }
                let message = text.replace("{}", "/home/someone/.local/share/yo3-revival/x").replace("{:?}", "\"x\"");
                for hint in [Hint::General, Hint::Component, Hint::GameFolder] {
                    let shown = present(&format!("{message}: The system cannot find the file specified. (os error 2)"), hint, &[]);
                    checked += 1;
                    if let Some(why) = technical(&shown.summary) { found.push(format!("{file}:{line}: {why}: {:?} -> {:?}", text, shown.summary)) }
                }
            }
        }
        assert!(checked > 300, "the library's messages were found ({checked})");
        assert!(found.is_empty(), "an error's sentence is technical:\n{}", found.join("\n"));
    }

    #[test]
    fn every_play_state_and_server_problem_reads_plainly() {
        use yo3_desktop::multiplayer::Problem;
        for problem in [Problem::WrongUserOrPassword, Problem::TooManyTries { wait_seconds: 90 }, Problem::Banned { reason: "x".into(), until: None },
            Problem::Restricted { reason: String::new(), until: Some(0) }, Problem::Maintenance { message: String::new() }, Problem::Full,
            Problem::NeedsUpdate { min: "0.2.0".into() }, Problem::IdentityChanged, Problem::Unreachable, Problem::SignInAgain, Problem::SessionEnded,
            Problem::InviteNeeded, Problem::InviteInvalid, Problem::RegistrationClosed, Problem::UserIdTaken, Problem::UserIdInvalid, Problem::PasswordTooShort,
            Problem::PasswordTooLong, Problem::PasswordTooCommon, Problem::PasswordCharacters, Problem::PasswordsDiffer, Problem::RulesNotAccepted, Problem::RecoveryInvalid,
            Problem::Other("server_error".into())] {
            assert_eq!(technical(&problem.sentence()), None, "{problem:?}");
        }
        for play in [crate::home::Play::Checking, crate::home::Play::SetUp, crate::home::Play::Repair, crate::home::Play::Retry, crate::home::Play::Waiting] {
            assert_eq!(technical(play.status()), None);
        }
    }

    #[test]
    fn the_antivirus_page_is_linked_wherever_our_files_need_repair() {
        use crate::app::ANTIVIRUS;
        use crate::help::{Block, topic_for, topics};
        let page = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs").join(ANTIVIRUS.rsplit('/').next().unwrap());
        assert!(ANTIVIRUS.starts_with("https://github.com/usersina/yo3-revival/blob/main/docs/") && page.is_file(), "{ANTIVIRUS}");
        let links = |id: &str| topics().into_iter().find(|t| t.id == id).is_some_and(|t| t.blocks.iter().any(|b| matches!(b, Block::Link(_, url) if url == ANTIVIRUS)));
        // A missing or changed file of ours: the notice's Help opens a topic that links the page (its Repair, the dialog below).
        for message in ["Missing or invalid 32-bit injector: Required component not found: C:\\Revival\\yo3-auth-launch.exe. Repair the installation.",
            "Connection DLL validation failed: /home/test/Revival/yo3-auth.dll: Connection DLL release v0.0.5 does not match application v0.0.6."] {
            let shown = present(message, Hint::Component, &[]);
            assert!(links(topic_for(&shown.summary)), "{:?} -> {}", shown.summary, topic_for(&shown.summary));
        }
        assert!(links("repair") && links("antivirus"));
        assert_eq!(topic_for("Your antivirus removed a file."), "antivirus");
        // The Repair dialog (Play's REPAIR, Setup's row, the notice's Repair) has a button for it.
        let app = include_str!("app.rs");
        let repair = &app[app.find("Dialog::Repair => (").expect("the Repair dialog")..];
        assert!(repair[..repair.find("Dialog::Backups").unwrap()].contains("OpenLink(ANTIVIRUS.into())"));
    }

    #[test]
    fn the_literal_reader_finds_texts_and_their_lines() {
        let found = literals("let a = \"one\";\n// \"not this\"\nlet b = 'x'; let c = \"two \\\n    lines\";\n#[cfg(test)]\n\"tests\"");
        assert_eq!(found, [(1, "one".to_string()), (3, "two lines".to_string())]);
    }
}
