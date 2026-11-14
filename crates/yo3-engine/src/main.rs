//! The rules engine: EDOPro's core, built natively into this program, one duel per process, spoken to in JSON lines on
//! stdin and stdout. It answers exactly as `engine/duelcore.ts` (the Node sidecar it replaces) did, so the server's
//! translator reads the same messages:
//!
//!   {"op":"new","seed":7,"lp":8000,"decks":[{"main":[cid…],"extra":[cid…]},{…}],"names":{"cid":"English name"}}
//!                                    team 0 goes first; `names` finds other prints of a card (cards::Prints::resolve);
//!                                    `"skills":[{"player":team,"skill":"heart_of_the_cards"}]` gives NPCs their powers
//!   {"op":"respond","response":{…}}                                       an OcgResponse (encode.rs)
//!   {"op":"state"}                   where every card is now (read only; the server's duel check)
//!   {"op":"cards","cards":[{"cid":…,"name":…}]}   how the core loads each card (the server's card check)
//!   {"op":"identity"}                fingerprints of the engine and its data, for replaying a recorded duel
//!   {"op":"think"}                   the duel bot's answer to the pending prompt (bot/), for an NPC: the server then
//!                                    gives it with `respond`. Answer {"status":"thought","response":{…},"ms":…}
//!
//! Answer: {"messages":[…],"status":"await"|"end","errors":[…]}: the core's messages up to the next prompt (as
//! ocgcore-wasm 0.1.6 parsed them, decode.rs), with a Konami id (`cid`) beside every passcode. The engine directory (the
//! one holding `vendor/`, from `engine/fetch.sh`) is the first argument, else this program's own directory's parent
//! when it sits in `engine/bin/`, else `engine/` under the current directory.
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

mod bot;
mod cards;
mod core;
mod decode;
mod encode;
mod session;

/// The engine directory (see the module docs).
fn engine_dir() -> PathBuf {
    if let Some(dir) = std::env::args_os().nth(1) {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent()?.parent().map(Path::to_path_buf))
        && dir.join("vendor").is_dir()
    {
        return dir;
    }
    PathBuf::from("engine")
}

fn sha256(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes).as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn file_sha256(path: &Path) -> String {
    sha256(&std::fs::read(path).unwrap_or_default())
}

/// Every `.lua` under `dir`, as paths relative to it with `/`, sorted.
fn lua_files(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        let path = entry.path();
        if path.is_dir() {
            lua_files(&path, &rel, out);
        } else if name.ends_with(".lua") {
            out.push(rel);
        }
    }
}

/// The engine's exact dependencies, for replaying a recorded duel: this program (the core and its bindings are built
/// into it), the card scripts (ProjectIgnis' and our overrides), the card database and the Konami ids.
pub fn identity(vendor: &Path) -> serde_json::Value {
    let exe = std::env::current_exe().map(|e| file_sha256(&e)).unwrap_or_else(|_| sha256(b""));
    let mut scripts = ring::digest::Context::new(&ring::digest::SHA256);
    // ProjectIgnis' scripts, then ours (engine/overrides/, which take precedence) under a prefix of their own.
    for (dir, prefix) in [(vendor.join("CardScripts"), ""), (cards::overrides_dir(vendor), "overrides/")] {
        let mut names = vec![];
        lua_files(&dir, "", &mut names);
        names.sort();
        for name in &names {
            scripts.update(format!("{prefix}{name}").as_bytes());
            scripts.update(b"\0");
            scripts.update(&std::fs::read(dir.join(name)).unwrap_or_default());
        }
    }
    let scripts: String = scripts.finish().as_ref().iter().map(|b| format!("{b:02x}")).collect();
    serde_json::json!({
        "node": format!("native {}", env!("CARGO_PKG_VERSION")),
        "sidecar": exe,
        "core": sha256(env!("YO3_CORE_COMMIT").as_bytes()),
        "bindings": exe,
        "scripts": scripts,
        "database": file_sha256(&vendor.join("BabelCDB/cards.cdb")),
        "card_ids": file_sha256(&vendor.join("cardinfo.json")),
    })
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("yo3-engine {} (EDOPro core {})", env!("CARGO_PKG_VERSION"), env!("YO3_CORE_COMMIT"));
        return;
    }
    if !core::BUILT_WITH_CORE {
        eprintln!("yo3-engine was built without EDOPro's core (run engine/fetch.sh, then build again)");
        std::process::exit(2);
    }
    let vendor = engine_dir().join("vendor");
    let cards = match cards::Cards::open(&vendor) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("yo3-engine: {e}");
            std::process::exit(2);
        }
    };
    let mut session = session::Session::new(cards);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let answer = session.handle(&line);
        if writeln!(stdout, "{answer}").and_then(|_| stdout.flush()).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_identity_covers_the_overrides() {
        let engine = std::env::temp_dir().join(format!("yo3-identity-{}", std::process::id()));
        std::fs::create_dir_all(engine.join("vendor/CardScripts/official")).unwrap();
        std::fs::write(engine.join("vendor/CardScripts/official/c1.lua"), "upstream").unwrap();
        let vendor = engine.join("vendor");
        let scripts = || super::identity(&vendor)["scripts"].as_str().unwrap().to_string();
        let upstream = scripts();
        std::fs::create_dir_all(engine.join("overrides")).unwrap();
        std::fs::write(engine.join("overrides/c1.lua"), "ours").unwrap();
        let ours = scripts();
        assert_ne!(upstream, ours, "an override changes the scripts' fingerprint");
        std::fs::write(engine.join("overrides/c1.lua"), "ours, fixed").unwrap();
        assert_ne!(ours, scripts(), "so does changing one");
        // The same text moved from ProjectIgnis' folder to ours is a different engine too.
        std::fs::remove_file(engine.join("overrides/c1.lua")).unwrap();
        std::fs::write(engine.join("overrides/c1.lua"), "upstream").unwrap();
        std::fs::remove_file(engine.join("vendor/CardScripts/official/c1.lua")).unwrap();
        assert_ne!(upstream, scripts());
        std::fs::remove_dir_all(engine).unwrap();
    }
}
