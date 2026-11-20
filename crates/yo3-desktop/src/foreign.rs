//! Other programs inside the game (#68), from the player's side: what the launcher's helper found (the connection DLL's
//! survey, judged by `yo3_launcher::survey`), what the player is told, and the one thing YO3 Revival does about a
//! foreign file: **move it aside** (renamed to end in `.yo3-disabled`, listed and undoable in Settings → Advanced).
//! It never deletes anything and never changes Windows' settings: a Winsock provider is the player's to reset.
use std::path::{Path, PathBuf};
use anyhow::{Context, ensure};
use yo3_launcher::survey::{Decision, Finding, Survey};

/// What a file moved aside ends with.
pub const SUFFIX: &str = ".yo3-disabled";
/// What a server that asks for a clean game says (#69): it is the host's rule for friends' games, and other servers are
/// not affected.
pub const CLEAN_GAME: &str = "This server asks for a clean game: no other program's files in your game folder, nothing in Windows' network path \
    that isn't Microsoft's, and nothing watching the game's connection. Remove what is listed below, then press PLAY again. Other servers and \
    your own Singleplayer don't ask for this.";
/// The command that clears a Winsock provider left by other software: the player runs it, as administrator.
pub const WINSOCK_RESET: &str = "netsh winsock reset";

/// Where the helper's report of a Play is kept: beside the ready file the Play names.
pub fn survey_file(ready: &Path) -> PathBuf { ready.with_extension("survey.json") }

/// Why a start stopped before the game ran: what was found, and whether Continue anyway is possible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held { pub findings: Vec<Finding>, pub blocked: bool,
    /// The server asks for a clean game (#69): this stop is its policy, not the game's safety, and there is no Continue anyway.
    pub clean: bool }

impl Held {
    /// The helper's report in `file`, when it stopped the game for it.
    pub fn read(file: &Path) -> Option<Self> {
        let survey = Survey::read(file)?;
        let blocked = match survey.decision? { Decision::Went => return None, Decision::Held => false, Decision::Blocked => true };
        let verdict = survey.assessment();
        let findings: Vec<Finding> = verdict.all().cloned().collect();
        (!findings.is_empty()).then_some(Self { findings, blocked, clean: survey.clean_game })
    }
    /// The other programs' files the findings name, by file name: what Move it aside can rename (it only touches
    /// `.dll` and `.asi` files directly in the game folder, whatever else a finding names).
    pub fn files(&self) -> Vec<String> {
        let mut names: Vec<String> = self.findings.iter().filter_map(|f| match f {
            Finding::GameFolderFile { name, .. } => Some(name.clone()),
            Finding::KnownBad { name, .. } => Some(name.clone()),
            _ => None,
        }).collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        names
    }
    /// Which of the files are loaded into the game (the rest are only present in its folder).
    pub fn loaded_files(&self) -> Vec<String> {
        self.findings.iter().filter_map(|f| if let Finding::GameFolderFile { name, loaded: true, .. } = f { Some(name.clone()) } else { None }).collect()
    }
    /// The Winsock providers that aren't Microsoft's.
    pub fn providers(&self) -> Vec<String> {
        self.findings.iter().filter_map(|f| if let Finding::Provider { name, .. } = f { Some(name.clone()) } else { None }).collect()
    }
    /// Security software watching the game's network functions: the products by name (or "" when unknown, once).
    pub fn security(&self) -> Vec<String> {
        let mut out: Vec<String> = self.findings.iter().filter_map(|f| match f {
            Finding::Hooked { how, product, .. } if how != "import" => Some(product.clone()),
            _ => None,
        }).collect();
        out.sort(); out.dedup(); out
    }
    /// Programs that took over the game's network functions (blocking), by their module's name.
    pub fn taken_over(&self) -> Vec<String> {
        let mut out: Vec<String> = self.findings.iter().filter_map(|f| match f {
            Finding::Hooked { how, module, product, .. } if how == "import" => Some(if !product.is_empty() { product.clone() } else if module.is_empty() { "Another program".into() } else { module.clone() }),
            _ => None,
        }).collect();
        out.sort(); out.dedup(); out
    }
    /// Files known to break the connection.
    pub fn known_bad(&self) -> Vec<String> {
        self.findings.iter().filter_map(|f| if let Finding::KnownBad { name, .. } = f { Some(name.clone()) } else { None }).collect()
    }
    /// The technical text behind Details: each finding with the file's place, size and SHA-256.
    pub fn details(&self) -> String {
        let mut text = String::new();
        for finding in &self.findings {
            let line = match finding {
                Finding::GameFolderFile { name, path, size, sha256, loaded } => format!("File in the game folder: {name} ({} bytes, {}), {path}\nSHA-256 {sha256}", size, if *loaded { "loaded into the game" } else { "not loaded" }),
                Finding::Hooked { function, how, module, path, product } => format!("{} {function}: {}{}{}",
                    if how == "import" { "Taken over:" } else { "Detour at the start of:" }, if module.is_empty() { "code outside any module".to_string() } else { module.clone() },
                    if path.is_empty() { String::new() } else { format!(", {path}") }, if product.is_empty() { String::new() } else { format!(" ({product})") }),
                Finding::Provider { name, path } => format!("Winsock provider that isn't Microsoft's: {name}{}", if path.is_empty() { String::new() } else { format!(", {path}") }),
                Finding::KnownBad { name, path, sha256, why } => format!("Known to break the connection: {name}, {path}\nSHA-256 {sha256}\n{why}"),
            };
            if !text.is_empty() { text.push_str("\n\n") }
            text.push_str(&line);
        }
        text
    }
}

fn extension_ok(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".dll") || lower.ends_with(".asi")
}

/// Whether `name` in `game` is a file YO3 Revival may move aside: a `.dll` or `.asi` directly in the game folder, not a
/// link, and not YO3 Revival's own loader (`winmm.dll` it put there itself).
pub fn movable(game: &Path, name: &str) -> anyhow::Result<PathBuf> {
    ensure!(extension_ok(name) && !name.contains(['/', '\\']) && name != "." && name != "..", "{name} is not a file YO3 Revival moves aside");
    let file = game.join(name);
    let meta = std::fs::symlink_metadata(&file).with_context(|| format!("{name} is not in the game folder any more"))?;
    ensure!(meta.is_file() && !meta.file_type().is_symlink(), "{name} is not an ordinary file");
    if name.eq_ignore_ascii_case("winmm.dll") { ensure!(crate::direct::foreign_winmm(game).is_some(), "{name} is YO3 Revival's own file: turn off Open the game without the launcher in Settings instead") }
    Ok(file)
}

/// Renames `name` in `game` to end in `.yo3-disabled`, so Windows no longer loads it. Never overwrites a file.
pub fn move_aside(game: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let file = movable(game, name)?;
    let target = game.join(format!("{name}{SUFFIX}"));
    ensure!(std::fs::symlink_metadata(&target).is_err(), "There is already a file called {name}{SUFFIX} in the game folder");
    std::fs::rename(&file, &target).with_context(|| format!("Windows would not let YO3 Revival rename {name}"))?;
    Ok(target)
}

/// The player said no to Windows' permission question: the file is where it was.
#[derive(Debug)]
pub struct Declined;
impl std::fmt::Display for Declined { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("Permission to change the game folder was declined") } }
impl std::error::Error for Declined {}

fn denied(error: &anyhow::Error) -> bool { error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied) }

/// What to do with a rename that was refused for permission: ask the elevated helper (the loader's Allow path) to do
/// it, once; [`Declined`] when the player says no.
fn elevated(result: anyhow::Result<PathBuf>, ask: impl FnOnce() -> anyhow::Result<crate::privileged::PermissionResult>, after: impl FnOnce() -> PathBuf) -> anyhow::Result<PathBuf> {
    match result {
        Err(error) if denied(&error) => match ask()? {
            crate::privileged::PermissionResult::Applied => Ok(after()),
            crate::privileged::PermissionResult::Cancelled => Err(Declined.into()),
        },
        other => other,
    }
}
/// [`move_aside`], asking for permission when the game folder is protected.
pub fn move_aside_allowed(game: &Path, name: &str) -> anyhow::Result<PathBuf> {
    elevated(move_aside(game, name), || crate::privileged::request_file(game, name, false), || game.join(format!("{name}{SUFFIX}")))
}
/// [`put_back`], asking for permission when the game folder is protected.
pub fn put_back_allowed(game: &Path, name: &str) -> anyhow::Result<PathBuf> {
    elevated(put_back(game, name), || crate::privileged::request_file(game, name, true), || game.join(name))
}

/// The sentence for a declined permission question: what is still true and what to do next.
pub const DECLINED: &str = "Windows wasn't asked, or you chose not to allow it, so the file is still where it was. Choose Continue anyway, or move it out of the game folder yourself.";

/// [`DECLINED`] for Put back: the file stays moved aside.
pub const DECLINED_PUT_BACK: &str = "Windows wasn't asked, or you chose not to allow it, so the file stays moved aside. Press Put back again when you're ready.";

/// The files moved aside in `game`, by their original names (what Settings → Advanced lists).
pub fn moved_aside(game: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(game).into_iter().flatten().flatten().filter_map(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        let original = name.strip_suffix(SUFFIX)?;
        (extension_ok(original) && entry.file_type().is_ok_and(|t| t.is_file())).then(|| original.to_string())
    }).collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// Undoes [`move_aside`]. Refuses when a file of the original name is there now.
pub fn put_back(game: &Path, name: &str) -> anyhow::Result<PathBuf> {
    ensure!(extension_ok(name) && !name.contains(['/', '\\']), "{name} was not moved aside");
    let from = game.join(format!("{name}{SUFFIX}")); let to = game.join(name);
    ensure!(std::fs::symlink_metadata(&from).is_ok_and(|m| m.is_file()), "{name} is not moved aside");
    ensure!(std::fs::symlink_metadata(&to).is_err(), "There is a file called {name} in the game folder already");
    std::fs::rename(&from, &to).with_context(|| format!("Windows would not let YO3 Revival rename {name} back"))?;
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yo3_launcher::survey::{Hook, Module};

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yo3-foreign-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_held_start_is_read_from_the_helpers_decision() {
        let dir = folder("held");
        let file = dir.join("survey.json");
        let survey = Survey { modules: vec![Module { name: "dinput8.dll".into(), path: "C:\\g\\dinput8.dll".into(), game_folder: true, loaded: true, ..Module::default() }], ..Survey::default() };
        std::fs::write(&file, serde_json::to_vec(&survey).unwrap()).unwrap();
        assert_eq!(Held::read(&file), None, "no decision yet: the helper hasn't judged it");
        yo3_launcher::survey::record_decision(&file, Decision::Went).unwrap();
        assert_eq!(Held::read(&file), None, "the game went");
        yo3_launcher::survey::record_decision(&file, Decision::Held).unwrap();
        let held = Held::read(&file).unwrap();
        assert!(!held.blocked);
        assert_eq!(held.files(), ["dinput8.dll"]);
        assert!(held.details().contains("dinput8.dll"));
        std::fs::write(&file, serde_json::to_vec(&Survey::default()).unwrap()).unwrap();
        yo3_launcher::survey::record_decision(&file, Decision::Blocked).unwrap();
        assert!(Held::read(&file).is_none(), "blocked with nothing found is not a thing to say");
        let hooked = Survey { hooked: vec![Hook { function: "send".into(), how: "import".into(), module: "asi.dll".into(), path: "C:\\g\\asi.dll".into(), product: String::new() }], ..survey };
        std::fs::write(&file, serde_json::to_vec(&hooked).unwrap()).unwrap();
        yo3_launcher::survey::record_decision(&file, Decision::Blocked).unwrap();
        let held = Held::read(&file).unwrap();
        assert!(held.blocked);
        assert_eq!((held.taken_over(), held.files()), (vec!["asi.dll".to_string()], vec!["dinput8.dll".to_string()]));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_clean_game_servers_refusal_is_read_with_its_policy_and_says_it_in_plain_words() {
        let dir = folder("clean");
        let file = dir.join("survey.json");
        let survey = Survey { modules: vec![Module { name: "dinput8.dll".into(), path: "C:\\g\\dinput8.dll".into(), game_folder: true, ..Module::default() }], ..Survey::default() };
        std::fs::write(&file, serde_json::to_vec(&survey).unwrap()).unwrap();
        yo3_launcher::survey::record_decision_on(&file, Decision::Blocked, true).unwrap();
        let held = Held::read(&file).unwrap();
        assert!(held.blocked && held.clean, "no Continue anyway on that server");
        assert_eq!(held.files(), ["dinput8.dll"], "Move it aside is still there");
        yo3_launcher::survey::record_decision_on(&file, Decision::Held, false).unwrap();
        assert!(!Held::read(&file).unwrap().clean, "another server: the usual dialog");
        assert!(crate::words::technical(CLEAN_GAME).is_none(), "plain words");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_findings_are_sorted_into_what_the_player_can_act_on() {
        let held = Held { blocked: false, clean: false, findings: vec![
            Finding::Provider { name: "Old VPN".into(), path: String::new() },
            Finding::Hooked { function: "send".into(), how: "detour".into(), module: "aswhook.dll".into(), path: "C:\\a".into(), product: "Avast".into() },
            Finding::Hooked { function: "recv".into(), how: "detour".into(), module: "x.dll".into(), path: "C:\\x".into(), product: "Avast".into() },
            Finding::Hooked { function: "connect".into(), how: "detour".into(), module: "y.dll".into(), path: "C:\\y".into(), product: String::new() },
        ] };
        assert_eq!(held.providers(), ["Old VPN"]);
        assert_eq!(held.security(), ["", "Avast"], "each product once, an unknown one as an empty name");
        assert!(held.files().is_empty() && held.taken_over().is_empty() && held.known_bad().is_empty());
    }

    #[test]
    fn a_file_only_present_in_the_game_folder_is_told_apart_from_one_the_game_loaded() {
        let file = |name: &str, loaded| Finding::GameFolderFile { name: name.into(), path: String::new(), size: 1, sha256: String::new(), loaded };
        let held = Held { blocked: false, clean: false, findings: vec![file("loaded.dll", true), file("idle.dll", false)] };
        assert_eq!((held.files(), held.loaded_files()), (vec!["idle.dll".to_string(), "loaded.dll".to_string()], vec!["loaded.dll".to_string()]));
    }

    #[test]
    fn a_file_is_moved_aside_and_put_back_and_nothing_else_is_touched() {
        let game = folder("move");
        std::fs::write(game.join("yo3.exe"), b"game").unwrap();
        std::fs::write(game.join("dinput8.dll"), b"plugin loader").unwrap();
        std::fs::write(game.join("mod.ASI"), b"plugin").unwrap();
        std::fs::create_dir(game.join("scripts")).unwrap();
        let target = move_aside(&game, "dinput8.dll").unwrap();
        assert_eq!(target.file_name().unwrap(), "dinput8.dll.yo3-disabled");
        assert!(!game.join("dinput8.dll").exists() && std::fs::read(&target).unwrap() == b"plugin loader", "renamed, never deleted or changed");
        assert_eq!(moved_aside(&game), ["dinput8.dll"]);
        // Only plugin files directly in the game folder: not the game, not a path, not a folder.
        for name in ["yo3.exe", "../outside.dll", "scripts", "sub\\x.dll", "missing.dll"] { assert!(move_aside(&game, name).is_err(), "{name}") }
        assert!(game.join("yo3.exe").exists());
        move_aside(&game, "mod.ASI").unwrap();
        assert_eq!(moved_aside(&game), ["dinput8.dll", "mod.ASI"]);
        // Never over another file, either way.
        std::fs::write(game.join("dinput8.dll"), b"a new one").unwrap();
        assert!(put_back(&game, "dinput8.dll").is_err(), "a file of that name is there now");
        std::fs::write(game.join("mod.ASI"), b"again").unwrap();
        assert!(move_aside(&game, "mod.ASI").is_err(), "a file already moved aside under that name");
        std::fs::remove_file(game.join("dinput8.dll")).unwrap();
        let back = put_back(&game, "dinput8.dll").unwrap();
        assert_eq!(std::fs::read(back).unwrap(), b"plugin loader");
        assert_eq!(moved_aside(&game), ["mod.ASI"]);
        std::fs::remove_dir_all(game).unwrap();
    }

    #[test]
    fn a_refused_rename_asks_for_permission_once_and_a_no_leaves_the_file_alone() {
        use crate::privileged::PermissionResult::{Applied, Cancelled};
        let refused = || -> anyhow::Result<PathBuf> { Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)).context("Windows would not let YO3 Revival rename x.dll") };
        let allowed = elevated(refused(), || Ok(Applied), || PathBuf::from("done")).unwrap();
        assert_eq!(allowed, PathBuf::from("done"));
        let no = elevated(refused(), || Ok(Cancelled), || PathBuf::from("done")).unwrap_err();
        assert!(no.downcast_ref::<Declined>().is_some());
        // Other failures are not a permission question, and success never asks.
        let other = elevated(Err(anyhow::anyhow!("x.dll is not an ordinary file")), || panic!("asked"), PathBuf::new).unwrap_err();
        assert!(other.downcast_ref::<Declined>().is_none());
        assert_eq!(elevated(Ok(PathBuf::from("ok")), || panic!("asked"), PathBuf::new).unwrap(), PathBuf::from("ok"));
        assert!(DECLINED.contains("Continue anyway") && crate::words::technical(DECLINED).is_none());
        assert!(DECLINED_PUT_BACK.contains("Put back") && crate::words::technical(DECLINED_PUT_BACK).is_none());
    }

    #[test]
    fn yo3_revivals_own_loader_is_never_moved_aside() {
        let game = folder("own");
        std::fs::write(game.join("yo3.exe"), b"game").unwrap();
        std::fs::write(game.join("winmm.dll"), b"ours").unwrap();
        let hash = yo3_launcher::components::hash(b"ours");
        std::fs::write(game.join("yo3-revival-loader.json"), serde_json::json!({ "format": 1, "executable": yo3_launcher::components::hash(b"game"), "loader": hash }).to_string()).unwrap();
        assert!(move_aside(&game, "winmm.dll").is_err(), "the loader of Open the game without the launcher");
        std::fs::write(game.join("winmm.dll"), b"someone else's").unwrap();
        assert!(move_aside(&game, "winmm.dll").is_ok(), "a changed one isn't ours");
        std::fs::remove_dir_all(game).unwrap();
    }

    #[test]
    fn our_own_loader_is_never_offered_to_move_aside_even_without_its_receipt() {
        // #65: the player's winmm.dll was our v0.1.3 loader, and was called another program's.
        let game = folder("own-no-receipt");
        std::fs::write(game.join("yo3.exe"), b"game").unwrap();
        std::fs::write(game.join("winmm.dll"), yo3_launcher::components::crafted_loader_for_tests("v0.1.3")).unwrap();
        assert!(movable(&game, "winmm.dll").is_err() && move_aside(&game, "winmm.dll").is_err());
        assert!(game.join("winmm.dll").exists(), "left where it was");
        std::fs::remove_dir_all(game).unwrap();
    }
}
