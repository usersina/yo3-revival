//! What else runs inside the game (#68): the connection DLL's survey (`tools/mod/survey.h`) read and judged. The DLL lists
//! the modules loaded from outside Windows' folders (and the game folder's other `.dll` and `.asi` files), whether another
//! program had already redirected the game's network functions, and the 32-bit Winsock providers; it writes them as JSON
//! into the file the launcher names (`YO3_SURVEY_FILE`) before the game runs. This judges that report:
//!
//! - **blocked**, the only reasons PLAY stops: another module has *redirected* one of the game's network imports (it
//!   sits between the game and the connection YO3 Revival guards), or a module's SHA-256 is on [`KNOWN_BAD`];
//! - **attention**, which the player is told about and may go past with *Continue anyway*: another program's `.dll` or
//!   `.asi` in the game folder, a detour at the start of a Windows network function, a Winsock provider that isn't Microsoft's;
//! - everything else (an overlay's DLL, a driver's) is evidence in the report only.
//!
//! The launcher writes its decision back into the file (`decision`), which is how the Desktop knows why a start stopped.
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Files known to break the game's connection, by SHA-256 (lowercase hex) and why. Empty until a file is known to be
/// the cause of a report (#65's `winmm.dll` is our own loader, so it is not one).
pub const KNOWN_BAD: &[(&str, &str)] = &[];

/// Programs that read, change or pause another program's memory, by a lowercase part of their process's file name and
/// the name shown. They are not blocked and not held for: nothing says they break the game. They are named in the
/// helper's log, at the start and again when the game's connection fails, so a report says whether one was running (#65:
/// a player's shop connection was cut on their PC and they later mentioned using Cheat Engine).
pub const MEMORY_TOOLS: &[(&str, &str)] = &[("cheatengine", "Cheat Engine"), ("cheat engine", "Cheat Engine"), ("artmoney", "ArtMoney"),
    ("x64dbg", "x64dbg"), ("x32dbg", "x32dbg"), ("ollydbg", "OllyDbg"), ("processhacker", "Process Hacker"), ("systeminformer", "System Informer"),
    ("scanmem", "scanmem"), ("gameconqueror", "GameConqueror")];

/// The memory tools among the process file names `names`, each once, in the order found.
pub fn tools_among<I: IntoIterator<Item = S>, S: AsRef<str>>(names: I) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    for name in names {
        let name = name.as_ref().to_lowercase();
        if let Some((_, product)) = MEMORY_TOOLS.iter().find(|(part, _)| name.contains(part)) && !found.contains(product) { found.push(product) }
    }
    found
}

/// The memory tools running on this PC now (Windows: the process list; elsewhere nothing is looked at).
pub fn running_tools() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{Foundation::{CloseHandle, INVALID_HANDLE_VALUE}, System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS}};
        let mut names = Vec::new();
        // SAFETY: a snapshot handle used only here and closed; PROCESSENTRY32W is plain data with dwSize set as the API asks.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE { return Vec::new() }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snapshot, &mut entry) != 0;
            while more {
                let length = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
                names.push(String::from_utf16_lossy(&entry.szExeFile[..length]));
                more = Process32NextW(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
        }
        tools_among(names)
    }
    #[cfg(not(windows))]
    { Vec::new() }
}

/// The log line for the tools found, or None when there are none.
pub fn tools_line(tools: &[&str]) -> Option<String> {
    (!tools.is_empty()).then(|| format!("{} is running on this PC. Programs like it can pause the game, change its memory or hook its time functions, which \
        can cut the game's connection. Nothing is blocked: close it and play again if the game disconnects", tools.join(", ")))
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Hook { pub function: String, pub how: String, pub module: String, pub path: String, #[serde(default)] pub product: String }
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Module {
    pub name: String, pub path: String, #[serde(default)] pub size: u64, #[serde(default)] pub sha256: String,
    #[serde(default)] pub game_folder: bool, #[serde(default)] pub loaded: bool, #[serde(default)] pub product: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Provider { pub name: String, #[serde(default)] pub path: String, #[serde(default)] pub microsoft: bool }

/// The DLL's report, as it wrote it.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Survey {
    #[serde(default)] pub hooked: Vec<Hook>,
    #[serde(default)] pub modules: Vec<Module>,
    #[serde(default)] pub providers: Vec<Provider>,
    #[serde(default)] pub providers_known: bool,
    /// What the launcher decided, written back into the file: [`Decision`].
    #[serde(default)] pub decision: Option<Decision>,
    /// The server asked for a clean game (#69) when the launcher decided: with it, what the player could have gone
    /// past stops the start too.
    #[serde(default)] pub clean_game: bool,
}

/// What the launcher did with the report.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Nothing to tell, or the player chose Continue anyway: the game was released.
    Went,
    /// Something to tell the player first: the game was not released (Continue anyway starts it again with `--accept-foreign`).
    Held,
    /// Something certain to break the game: not released, and no Continue.
    Blocked,
}

/// One thing found, for the player and for the report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Finding {
    /// Another program's `.dll` or `.asi` in the game folder (`loaded`: Windows loaded it into the game).
    GameFolderFile { name: String, path: String, size: u64, sha256: String, loaded: bool },
    /// Another module has redirected a network function the game imports (`import`, blocks), or detoured the start of
    /// Windows' own (`detour`, only a warning, usually security software: `product` names it when it is known).
    Hooked { function: String, how: String, module: String, path: String, product: String },
    /// A Winsock provider the game's connection goes through that is not Microsoft's.
    Provider { name: String, path: String },
    /// A file whose SHA-256 is known to break the game's connection.
    KnownBad { name: String, path: String, sha256: String, why: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Assessment { pub blocked: Vec<Finding>, pub attention: Vec<Finding> }

impl Assessment {
    /// Nothing to tell the player.
    pub fn is_clear(&self) -> bool { self.blocked.is_empty() && self.attention.is_empty() }
    /// What the launcher does before releasing the game: stop for certain breakage, hold for what the player should hear
    /// first (unless they accepted), else go.
    pub fn decision(&self, accepted: bool) -> Decision {
        if !self.blocked.is_empty() { Decision::Blocked } else if !self.attention.is_empty() && !accepted { Decision::Held } else { Decision::Went }
    }
    /// [`Assessment::decision`] on a server that asks for a clean game (#69, the host's `clean-game` setting): anything
    /// found stops the start, and there is no Continue anyway. A tidiness policy the launcher enforces by reporting,
    /// not anti-cheat: nothing on the player's PC can't be bypassed.
    pub fn decision_on(&self, accepted: bool, clean_game: bool) -> Decision {
        if clean_game && !self.is_clear() { Decision::Blocked } else { self.decision(accepted) }
    }
    /// Every finding, the certain ones first.
    pub fn all(&self) -> impl Iterator<Item = &Finding> { self.blocked.iter().chain(self.attention.iter()) }
}

impl Survey {
    pub fn parse(bytes: &[u8]) -> anyhow::Result<Self> { Ok(serde_json::from_slice(bytes)?) }
    /// The report in `file`, if the DLL wrote one (and it is small and a plain file: the game's process wrote it).
    pub fn read(file: &Path) -> Option<Self> {
        let meta = std::fs::symlink_metadata(file).ok()?;
        if !meta.is_file() || meta.len() > 256 * 1024 { return None }
        Self::parse(&std::fs::read(file).ok()?).ok()
    }
    /// The judgement, with `known_bad` (SHA-256, reason) as the list of files known to break the connection.
    pub fn assess(&self, known_bad: &[(&str, &str)]) -> Assessment {
        let mut out = Assessment::default();
        for hook in &self.hooked {
            let finding = Finding::Hooked { function: hook.function.clone(), how: hook.how.clone(), module: hook.module.clone(), path: hook.path.clone(), product: hook.product.clone() };
            if hook.how == "import" { out.blocked.push(finding) } else { out.attention.push(finding) }
        }
        for module in &self.modules {
            let hash = module.sha256.to_ascii_lowercase();
            if let Some((_, why)) = known_bad.iter().find(|(known, _)| !hash.is_empty() && known.eq_ignore_ascii_case(&hash)) {
                out.blocked.push(Finding::KnownBad { name: module.name.clone(), path: module.path.clone(), sha256: hash, why: (*why).into() });
            } else if module.game_folder {
                out.attention.push(Finding::GameFolderFile { name: module.name.clone(), path: module.path.clone(), size: module.size, sha256: hash, loaded: module.loaded });
            }
        }
        if self.providers_known {
            for provider in self.providers.iter().filter(|p| !p.microsoft) {
                out.attention.push(Finding::Provider { name: provider.name.clone(), path: provider.path.clone() });
            }
        }
        out
    }
    /// [`Survey::assess`] against the list this build ships.
    pub fn assessment(&self) -> Assessment { self.assess(KNOWN_BAD) }
}

/// Writes the launcher's decision into the report, so whoever started the launcher can read why a start stopped.
pub fn record_decision(file: &Path, decision: Decision) -> anyhow::Result<()> { record_decision_on(file, decision, false) }

/// [`record_decision`], saying whether the server asked for a clean game (#69).
pub fn record_decision_on(file: &Path, decision: Decision, clean_game: bool) -> anyhow::Result<()> {
    let mut report = Survey::read(file).unwrap_or_default();
    report.decision = Some(decision);
    report.clean_game = clean_game;
    let part = file.with_extension("part");
    std::fs::write(&part, serde_json::to_vec(&report)?)?;
    std::fs::rename(&part, file)?;
    Ok(())
}

/// A sentence for the command line and the log: what was found and what to do.
pub fn describe(finding: &Finding) -> String {
    match finding {
        Finding::GameFolderFile { name, loaded, .. } => format!("{name} in the game folder is another program's file{}", if *loaded { ", and Windows loads it into the game" } else { "" }),
        Finding::Hooked { function, how, module, product, .. } => {
            let by = if !product.is_empty() { format!("{product} ({module})") } else if module.is_empty() { "unknown code".into() } else { module.clone() };
            if how == "import" { format!("{by} has taken over the game's {function} function") } else { format!("{by} watches the game's {function} function") }
        }
        Finding::Provider { name, .. } => format!("the Winsock provider {name} is not Microsoft's"),
        Finding::KnownBad { name, why, .. } => format!("{name} is known to break the game's connection: {why}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheat_engine_and_other_memory_tools_are_named_by_their_process_and_never_held_for() {
        assert_eq!(tools_among(["explorer.exe", "cheatengine-x86_64-SSE4-AVX2.exe", "CHEATENGINE-I386.EXE", "Cheat Engine.exe", "x64dbg.exe"]),
            ["Cheat Engine", "x64dbg"], "each once, in the order found, whatever the case or build");
        assert!(tools_among(["yo3.exe", "chrome.exe", "cheater.exe"]).is_empty());
        assert!(tools_line(&[]).is_none());
        let line = tools_line(&["Cheat Engine"]).unwrap();
        assert!(line.starts_with("Cheat Engine is running") && line.contains("Nothing is blocked"), "{line}");
        // Findings, decisions and Continue anyway are untouched: a tool is evidence in the log only.
        assert!(Survey::default().assessment().is_clear());
    }

    const CLEAN: &str = r#"{"version":1,"hooked":[],"modules":[{"name":"overlay.dll","path":"C:\\Program Files\\Overlay\\overlay.dll","size":10,"sha256":"aa","game_folder":false,"loaded":true,"product":""}],"providers":[{"name":"MSAFD Tcpip [TCP/IP]","path":"","microsoft":true}],"providers_known":true}"#;

    #[test]
    fn a_report_with_only_outside_modules_and_windows_providers_is_clear() {
        let survey = Survey::parse(CLEAN.as_bytes()).unwrap();
        let verdict = survey.assessment();
        assert!(verdict.is_clear(), "an overlay's DLL is evidence in the report, not something to tell the player");
        assert_eq!(verdict.decision(false), Decision::Went);
    }

    #[test]
    fn another_programs_file_in_the_game_folder_holds_the_game_until_the_player_accepts() {
        let survey = Survey { modules: vec![Module { name: "winmm.dll".into(), path: "C:\\Games\\yo3\\winmm.dll".into(), game_folder: true, loaded: true, size: 9, sha256: "AB".into(), ..Module::default() }],
            ..Survey::default() };
        let verdict = survey.assessment();
        assert_eq!(verdict.attention, [Finding::GameFolderFile { name: "winmm.dll".into(), path: "C:\\Games\\yo3\\winmm.dll".into(), size: 9, sha256: "ab".into(), loaded: true }]);
        assert!(verdict.blocked.is_empty());
        assert_eq!((verdict.decision(false), verdict.decision(true)), (Decision::Held, Decision::Went), "Continue anyway releases it");
    }

    #[test]
    fn a_clean_game_server_refuses_whatever_was_found_and_a_clean_game_goes() {
        let file = Module { name: "dinput8.dll".into(), path: "C:\\g\\dinput8.dll".into(), game_folder: true, ..Module::default() };
        let found = Survey { modules: vec![file], ..Survey::default() }.assessment();
        assert_eq!((found.decision_on(false, false), found.decision_on(true, false)), (Decision::Held, Decision::Went), "an ordinary server: Continue anyway");
        assert_eq!((found.decision_on(false, true), found.decision_on(true, true)), (Decision::Blocked, Decision::Blocked), "a clean-game server: no Continue anyway");
        let provider = Survey { providers: vec![Provider { name: "Old VPN".into(), path: String::new(), microsoft: false }], providers_known: true, ..Survey::default() }.assessment();
        assert_eq!(provider.decision_on(true, true), Decision::Blocked, "a Winsock provider counts");
        let detour = Survey { hooked: vec![Hook { function: "connect".into(), how: "detour".into(), module: "aswhook.dll".into(), path: String::new(), product: "Avast".into() }], ..Survey::default() }.assessment();
        assert_eq!(detour.decision_on(false, true), Decision::Blocked, "a detour counts");
        let clean = Survey::default().assessment();
        assert_eq!(clean.decision_on(false, true), Decision::Went, "nothing found: the game starts");
    }

    #[test]
    fn a_redirected_network_function_blocks_and_continue_anyway_does_not_override_it() {
        let survey = Survey { hooked: vec![Hook { function: "send".into(), how: "import".into(), module: "loader.dll".into(), path: "C:\\x\\loader.dll".into(), product: String::new() }], ..Survey::default() };
        let verdict = survey.assessment();
        assert_eq!(verdict.blocked.len(), 1);
        assert_eq!((verdict.decision(false), verdict.decision(true)), (Decision::Blocked, Decision::Blocked));
        assert!(describe(&verdict.blocked[0]).contains("taken over the game's send"));
    }

    #[test]
    fn a_detour_in_a_windows_function_is_a_warning_naming_the_product_when_known() {
        let survey = Survey { hooked: vec![Hook { function: "connect".into(), how: "detour".into(), module: "aswhook.dll".into(), path: "C:\\Av\\aswhook.dll".into(), product: "Avast".into() }], ..Survey::default() };
        let verdict = survey.assessment();
        assert!(verdict.blocked.is_empty() && verdict.attention.len() == 1);
        assert_eq!(verdict.decision(false), Decision::Held);
        assert!(describe(&verdict.attention[0]).starts_with("Avast (aswhook.dll) watches"));
    }

    #[test]
    fn a_provider_that_is_not_microsofts_is_a_warning_only_when_the_catalog_was_read() {
        let providers = vec![Provider { name: "Old VPN Layer".into(), path: "C:\\Old\\lsp.dll".into(), microsoft: false }];
        let read = Survey { providers: providers.clone(), providers_known: true, ..Survey::default() };
        assert_eq!(read.assessment().attention, [Finding::Provider { name: "Old VPN Layer".into(), path: "C:\\Old\\lsp.dll".into() }]);
        let unread = Survey { providers, providers_known: false, ..Survey::default() };
        assert!(unread.assessment().is_clear(), "a catalog that couldn't be read says nothing");
    }

    /// The shipped list of known-bad files is empty for now; a file on a list blocks wherever it sits, game folder or not.
    #[test]
    fn a_file_on_the_known_bad_list_blocks_and_the_shipped_list_is_empty() {
        assert!(KNOWN_BAD.is_empty());
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let survey = Survey { modules: vec![Module { name: "bad.dll".into(), path: "D:\\elsewhere\\bad.dll".into(), sha256: hash.to_ascii_uppercase(), loaded: true, ..Module::default() },
            Module { name: "other.dll".into(), path: "D:\\elsewhere\\other.dll".into(), sha256: "ff".into(), loaded: true, ..Module::default() }], ..Survey::default() };
        assert!(survey.assessment().is_clear(), "nothing is known bad by default");
        let verdict = survey.assess(&[(hash, "it replaces the network functions")]);
        assert!(matches!(&verdict.blocked[..], [Finding::KnownBad { name, why, .. }] if name == "bad.dll" && why.contains("network")));
        assert_eq!(verdict.decision(true), Decision::Blocked);
        let unhashed = Survey { modules: vec![Module { name: "x.dll".into(), ..Module::default() }], ..Survey::default() };
        assert!(unhashed.assess(&[("", "empty")]).is_clear(), "a file that couldn't be hashed matches nothing");
    }

    #[test]
    fn the_launchers_decision_is_written_back_for_whoever_started_it() {
        let dir = std::env::temp_dir().join(format!("yo3-survey-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("survey.json");
        std::fs::write(&file, CLEAN).unwrap();
        record_decision(&file, Decision::Held).unwrap();
        let read = Survey::read(&file).unwrap();
        assert_eq!(read.decision, Some(Decision::Held));
        assert_eq!(read.modules.len(), 1, "the report stays");
        assert!(Survey::read(&dir.join("missing.json")).is_none());
        std::fs::write(&file, "not json").unwrap();
        assert!(Survey::read(&file).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
