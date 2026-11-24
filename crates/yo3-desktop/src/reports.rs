//! Local-only diagnostic archives, assembled from our session inventory instead of arbitrary user directories.
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use serde::Serialize;
use anyhow::ensure;
use yo3_tunnel::diagnostics::{self as logs, Record, Metadata};
pub const GITHUB_ATTACHMENT_BYTES: u64 = 25_000_000;
#[derive(Clone, Serialize)]
pub struct Preview { pub files: usize, pub bytes: u64, pub oldest: u64, pub newest: u64 }
#[derive(Clone)]
pub struct Report { root: PathBuf, files: Vec<Record>, pub preview: Preview }
impl Report {
    /// None means all retained sessions; Some is the manager plus its attached server/latest game.
    pub fn prepare(root: &Path, ids: Option<&[String]>) -> anyhow::Result<Self> {
        logs::prune(root, logs::now(), logs::RETAIN_BYTES, logs::RETAIN_SECS)?;
        let files: Vec<_> = logs::records(root)?.into_iter().filter(|r| ids.is_none_or(|ids| ids.contains(&r.id))).collect();
        let preview = Preview { files: files.len(), bytes: files.iter().map(|f| f.bytes).sum(),
            oldest: files.iter().map(|f| f.modified).min().unwrap_or(0), newest: files.iter().map(|f| f.modified).max().unwrap_or(0) };
        Ok(Self { root: root.into(), files, preview })
    }
    pub fn export(&self, destination: &Path) -> anyhow::Result<()> {
        self.export_with_privacy(destination, false)
    }
    pub fn export_with_privacy(&self, destination: &Path, remove_personal: bool) -> anyhow::Result<()> {
        self.export_to(destination,remove_personal,None,None)
    }
    pub fn export_bug(&self, destination: &Path, remove_personal: bool, context: &serde_json::Value, report_markdown: &str, system: &crate::system_info::SystemInfo) -> anyhow::Result<()> {
        self.export_to(destination,remove_personal,Some((context,report_markdown,system)),Some(GITHUB_ATTACHMENT_BYTES))
    }
    /// The report file of Report a problem (format [`REPORT_VERSION`]): the logs with personal details removed,
    /// `report.json` with the report's own fields beside the build and the PC, `README.txt`, and binary files (recordings).
    #[expect(clippy::too_many_arguments, reason = "The report's parts: the file, its context, summary, PC, fields, README and recordings.")]
    pub fn export_full(&self, destination: &Path, context: &serde_json::Value, markdown: &str, system: &crate::system_info::SystemInfo,
        report: &serde_json::Value, readme: &str, binary: &[(String, Vec<u8>)]) -> anyhow::Result<()> {
        let mut files = self.text_files(true, Some((context, markdown, system)))?;
        let mut metadata: serde_json::Value = serde_json::from_str(&files["report.json"])?;
        metadata["format"] = REPORT_FORMAT.into();
        metadata["version"] = REPORT_VERSION.into();
        metadata["note"] = "Report a problem's file. Send it to the developer as it is. Credentials, databases, profiles and game files are excluded; personal details are removed from the logs.".into();
        let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok();
        metadata["report"] = clean_context(report, home.as_deref(), true);
        files.insert("report.json".into(), serde_json::to_string_pretty(&metadata)?);
        files.insert("README.txt".into(), filter_narrative(readme, home.as_deref(), true));
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o600);
        for (name, text) in files { archive.start_file(name, options)?; archive.write_all(text.as_bytes())?; }
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored).unix_permissions(0o600);
        for (name, bytes) in binary { archive.start_file(name.as_str(), stored)?; archive.write_all(bytes)?; }
        let bytes = archive.finish()?.into_inner();
        let stage = destination.with_extension(format!("{}.zip.tmp", std::process::id()));
        let mut file = logs::private_create(&stage)?; file.write_all(&bytes)?; file.sync_all()?; drop(file);
        let result = std::fs::rename(&stage, destination); if result.is_err() { let _ = std::fs::remove_file(&stage); } result?;
        Ok(())
    }
    fn text_files(&self, remove_personal: bool, bug: Option<(&serde_json::Value,&str,&crate::system_info::SystemInfo)>) -> anyhow::Result<BTreeMap<String,String>> {
        let mut files=BTreeMap::new();
        let mut add=|name:String,text:String|->anyhow::Result<()> { files.insert(name,text);Ok(()) };
        let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok();
        let mut metadata=serde_json::json!({"format":2,"created":logs::now(),"preview":self.preview,"personal_data_filtered":remove_personal,
            "release":yo3_tunnel::BUILD_VERSION,"commit":yo3_tunnel::BUILD_COMMIT,"protocol":yo3_tunnel::PROTOCOL_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"note":"User-created report. Credentials, databases, profiles and game files are excluded. Personal filtering omits client/packet/engine traces and player activity, and masks common user paths and network addresses; review remaining text before sharing. Nothing is uploaded."});
        if let Some((context,_,system))=bug {
            metadata["context"]=clean_context(context,home.as_deref(),remove_personal);
            metadata["system"]=clean_context(&serde_json::to_value(system)?,home.as_deref(),remove_personal);
        }
        add("report.json".into(),serde_json::to_string_pretty(&metadata)?)?;
        if let Some((_,markdown,_))=bug {add("bug-report.md".into(),filter_narrative(markdown,home.as_deref(),remove_personal))?;}
        let current = logs::records(&self.root)?;
        let mut remaining = logs::RETAIN_BYTES;
        for record in &self.files {
            // Re-inventory at export; a file removed/changed into a symlink since preview is skipped.
            if !current.iter().any(|r| r.name == record.name) { continue }
            let mut file = std::fs::File::open(self.root.join(&record.name))?;
            let mut bytes = Vec::new(); Read::by_ref(&mut file).take(logs::PART_BYTES + 16384).read_to_end(&mut bytes)?;
            ensure!(bytes.len() as u64 <= remaining, "Diagnostic retention changed; refresh the report preview"); remaining -= bytes.len() as u64;
            let text = if record.name.ends_with(".json") {
                let Ok(meta) = serde_json::from_slice::<Metadata>(&bytes) else { continue };
                if meta.id != record.id || !["app", "server", "game", "direct"].contains(&meta.kind.as_str()) { continue }
                let mut value = serde_json::to_value(&meta)?;
                if let Some(fields) = value.as_object_mut() { for field in fields.values_mut() {
                    if let Some(text) = field.as_str() { *field = clean(text, home.as_deref(), remove_personal).into(); }
                } }
                serde_json::to_string_pretty(&value)?
            } else { sanitize(&String::from_utf8_lossy(&bytes), home.as_deref(), remove_personal) };
            add(format!("sessions/{}", record.name),text)?;
        }
        Ok(files)
    }
    fn export_to(&self, destination: &Path, remove_personal: bool, bug: Option<(&serde_json::Value,&str,&crate::system_info::SystemInfo)>, limit: Option<u64>) -> anyhow::Result<()> {
        ensure!(destination.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")), "Choose a .zip report file");
        let files=self.text_files(remove_personal,bug)?;
        let mut archive=zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options=zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o600);
        for (name,text) in files {archive.start_file(name,options)?;archive.write_all(text.as_bytes())?;}
        let bytes = archive.finish()?.into_inner();
        if let Some(limit)=limit {ensure!(bytes.len() as u64<=limit,"The diagnostic ZIP exceeds GitHub's {limit}-byte attachment limit. Choose Current session or remove personal data, then try again. Your existing report was not replaced.");}
        // Complete the archive before replacing a selected report; errors cannot leave a half-written ZIP.
        let stage = destination.with_extension(format!("{}.zip.tmp", std::process::id()));
        let mut file = logs::private_create(&stage)?; file.write_all(&bytes)?; file.sync_all()?; drop(file);
        let result = std::fs::rename(&stage, destination); if result.is_err() { let _ = std::fs::remove_file(&stage); } result?;
        Ok(())
    }
}
pub fn filter_text(text: &str, remove_personal: bool) -> String {
    let home=std::env::var(if cfg!(windows){"USERPROFILE"}else{"HOME"}).ok();
    filter_narrative(text,home.as_deref(),remove_personal)
}
fn filter_narrative(text: &str, home: Option<&str>, remove_personal: bool) -> String {
    let mut out=String::new();let mut private_key=false;
    for line in text.lines() {
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") {private_key=true;}
        if private_key {if line.contains("-----END") && line.contains("PRIVATE KEY-----") {private_key=false;}out.push_str("[private key omitted]\n");}
        else {out.push_str(&clean(line,home,remove_personal));out.push('\n');}
    }
    out
}
fn clean_context(value: &serde_json::Value, home: Option<&str>, remove_personal: bool) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::String(text)=>sanitize(text,home,remove_personal).trim_end_matches('\n').into(),
        Value::Array(values)=>Value::Array(values.iter().map(|value|clean_context(value,home,remove_personal)).collect()),
        Value::Object(fields)=>Value::Object(fields.iter().map(|(name,value)| {
            let key=name.to_ascii_lowercase().replace(['-',' '],"_");
            let sensitive=["key","access_key","account_key","private_key","cookie","session","session_key","session_ticket","ticket","token","password","authorization","credential","credentials"].contains(&key.as_str())
                || key.contains("password") || key.contains("token");
            let value=if sensitive {Value::String("[redacted]".into())}
                else if remove_personal && personal_line(&key.replace('_'," ")) {Value::String("[personal data omitted]".into())}
                else {clean_context(value,home,remove_personal)};
            (clean(name,home,remove_personal),value)
        }).collect()),
        _=>value.clone(),
    }
}
fn sanitize(text: &str, home: Option<&str>, remove_personal: bool) -> String {
    let mut out = String::new(); let mut private_key = false;
    for line in text.lines() {
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") { private_key = true; }
        if private_key { if line.contains("-----END") && line.contains("PRIVATE KEY-----") { private_key = false; } out.push_str("[private key omitted]\n"); }
        else if line.len() > 16384 { out.push_str("[overlong diagnostic line omitted]\n"); }
        // A yo3-log/1 line: each of its texts filtered as a line would be (a JSON-escaped path wouldn't match as text),
        // the line left out when its message or a field names a person.
        else if line.starts_with('{') && let Ok(serde_json::Value::Object(mut event)) = serde_json::from_str::<serde_json::Value>(line) {
            let msg = event.get("msg").and_then(|m| m.as_str()).unwrap_or_default().to_owned();
            let personal = remove_personal && (personal_line(&msg) || event.keys().any(|k| ["user", "player", "peer"].contains(&k.as_str()))
                || event.get("topic").and_then(|t| t.as_str()) == Some("tls") || logs::is_trace(line));
            if personal { continue }
            for (key, value) in event.iter_mut() {
                if let serde_json::Value::String(text) = value && !["ts", "level", "src", "session"].contains(&key.as_str()) { *text = clean(text, home, remove_personal) }
            }
            if let Ok(json) = serde_json::to_string(&serde_json::Value::Object(event)) { out.push_str(&json); out.push('\n'); }
        }
        else if !remove_personal || !personal_line(line) { out.push_str(&clean(line, home, remove_personal)); out.push('\n'); }
    }
    out
}
fn personal_line(line: &str) -> bool {
    if logs::is_trace(line) || line.contains("[tls]") || line.contains("] <-") || line.contains("] ->") { return true }
    line.split(|c:char| !c.is_alphanumeric() && c != '_').any(|word| ["player", "player_id", "user", "user_id", "username", "account", "name", "chat", "mail", "friend", "team", "trade"].iter().any(|w| word.eq_ignore_ascii_case(w)))
}
fn clean(text: &str, home: Option<&str>, remove_personal: bool) -> String {
    if !remove_personal { return logs::redact_secrets(text, &[]) }
    let mut text = logs::redact(text, &[], home);
    for scheme in ["https://", "http://"] {
        let mut from = 0;
        while let Some(offset) = text[from..].to_ascii_lowercase().find(scheme) {
            let start = from + offset + scheme.len();
            let end = text[start..].find(|c:char| c.is_whitespace() || ['/', '?', '#', '"', '\'', '\\'].contains(&c)).map_or(text.len(), |n| start+n);
            text.replace_range(start..end, "<address>"); from = start + "<address>".len();
        }
    }
    let mut result = String::new(); let mut token = String::new();
    let append = |token: &mut String, result: &mut String| {
        let network = token.parse::<std::net::IpAddr>().is_ok() || token.parse::<std::net::SocketAddr>().is_ok()
            || token.rsplit_once(':').is_some_and(|(host,port)| port.parse::<u16>().is_ok() && host.contains('.') && !host.contains(['/', '\\']))
            || token.contains('@');
        result.push_str(if network { "<address>" } else { token }); token.clear();
    };
    for c in text.chars() {
        if c.is_alphanumeric() || ['.', ':', '-', '_', '@'].contains(&c) { token.push(c); }
        else { append(&mut token, &mut result); result.push(c); }
    }
    append(&mut token, &mut result); result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn github_archive_cap_preserves_destination_and_creates_no_partial_report() {
        let root=std::env::temp_dir().join(format!("yo3-report-cap-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);std::fs::create_dir_all(&root).unwrap();
        let report=Report::prepare(&root.join("sessions"),None).unwrap();let destination=root.join("report.zip");std::fs::write(&destination,b"existing report").unwrap();
        let system=crate::system_info::SystemInfo{os:"Windows 11".into(),arch:"x86_64".into(),wine:None,cpu:None,threads:0,memory_mb:None,gpu:None};let context=serde_json::json!({"error":"Specific failure"});
        let error=report.export_to(&destination,false,Some((&context,"# A reviewed bug report",&system)),Some(128)).unwrap_err();
        assert!(error.to_string().contains("GitHub") && error.to_string().contains("128"),"{error:#}");
        assert_eq!(std::fs::read(&destination).unwrap(),b"existing report");assert!(!destination.with_extension(format!("{}.zip.tmp",std::process::id())).exists());
        report.export(&destination).unwrap();assert!(std::fs::metadata(&destination).unwrap().len()>128,"ordinary exports do not inherit the GitHub cap");
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// How much of each log a problem report from the app carries (its newest bytes).
const REPORT_LOG_BUDGET: usize = 384 * 1024;

impl crate::Backend {
    /// What a problem report from Report a problem carries: the newest lines of this app session's log and of the
    /// latest game's (the helper's, with the game's and the connection DLL's lines).
    pub fn problem_logs(&self) -> String {
        let root = self.root.join("logs/sessions");
        let mut text = format!("== The app ==\n{}", self.diagnostics.tail(REPORT_LOG_BUDGET));
        if let Ok(id) = std::fs::read_to_string(root.join("latest-game")) && logs::valid_id(id.trim()) {
            text.push_str(&format!("\n\n== The latest game ==\n{}", logs::tail_of(&root, id.trim(), REPORT_LOG_BUDGET)));
        }
        text
    }

    /// The report IDs the helper got for games that broke in the last day (problem reports), newest first.
    pub fn recent_report_ids(&self) -> Vec<String> {
        let root = self.root.join("logs/sessions");
        let since = logs::now().saturating_sub(86_400);
        let mut games: Vec<_> = logs::records(&root).unwrap_or_default().into_iter()
            .filter(|r| r.name.ends_with(".json") && (r.id.starts_with("game-") || r.id.starts_with("direct-")) && r.modified >= since).collect();
        games.sort_by_key(|r| std::cmp::Reverse(r.modified));
        let mut ids = Vec::new();
        for game in games.iter().take(5) {
            for line in logs::tail_of(&root, &game.id, 256 * 1024).lines().rev() {
                if let Some(id) = line.split("Problem report ").nth(1).and_then(|rest| rest.split_whitespace().next()).filter(|id| is_report_id(id))
                    && !ids.iter().any(|known| known == id) { ids.push(id.to_string()) }
            }
        }
        ids.truncate(5);
        ids
    }
}

/// The report file's format (#87's follow-up): one ZIP the player keeps and can send to the developer however they
/// like, whatever server is up. `docs/report-format.md` describes it.
pub const REPORT_FORMAT: &str = "yo3-revival-report";
pub const REPORT_VERSION: u32 = 4;
/// Singleplayer's newest pinned duel recordings (the `bug` command) that go in the file, and the largest of them.
const REPORT_RECORDINGS: usize = 3;
const REPORT_RECORDING_BYTES: u64 = 8 * 1024 * 1024;

/// What the player wrote and where the report went, for [`crate::Backend::save_report`].
#[derive(Clone, Debug, Default)]
pub struct Details {
    pub what: String,
    /// How to reach the player (optional).
    pub contact: String,
    /// The error the app showed, if the report came from one.
    pub error: String,
    /// Reports the helper sent hosts by itself after recent games that broke, for reference: (who, "R-…").
    pub sent: Vec<(String, String)>,
    /// "singleplayer", or the server's name in Multiplayer.
    pub play: String,
    /// The launcher's settings that explain a problem (no account, key or path): screen size, direct start, banlist...
    pub settings: serde_json::Value,
    /// Singleplayer's server data folder, for its pinned recordings.
    pub recordings: Option<PathBuf>,
}

/// Where a report file goes: the player's Downloads folder when there is one, else the app's own `reports/`.
pub fn report_folder(root: &Path) -> PathBuf {
    downloads().filter(|d| d.is_dir()).unwrap_or_else(|| root.join("reports"))
}

fn downloads() -> Option<PathBuf> {
    #[cfg(windows)]
    { std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("Downloads")) }
    #[cfg(not(windows))]
    {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        // XDG's user-dirs.dirs names it (`XDG_DOWNLOAD_DIR="$HOME/Downloads"`), in the system's language.
        let dirs = std::fs::read_to_string(home.join(".config/user-dirs.dirs")).unwrap_or_default();
        let named = dirs.lines().find_map(|l| l.strip_prefix("XDG_DOWNLOAD_DIR=")).map(|v| v.trim().trim_matches('"').replace("$HOME", &home.to_string_lossy()));
        Some(named.map(PathBuf::from).filter(|p| p.is_absolute() && p != &home).unwrap_or_else(|| home.join("Downloads")))
    }
}

impl crate::Backend {
    /// The whole report as one ZIP in [`report_folder`] (format [`REPORT_VERSION`]): `report.json` (what happened, how
    /// to reach the player, the error, the app's build, the PC, the game's settings, where it was sent), `README.txt`
    /// (the same for a person, and where to send it), `bug-report.md`, this session's and the latest game's logs with
    /// personal details removed, and Singleplayer's newest pinned duel recordings. Its path.
    pub fn save_report(&self, details: &Details) -> anyhow::Result<PathBuf> {
        use crate::bug_report::{Context, Draft};
        let dir = report_folder(&self.root);
        std::fs::create_dir_all(&dir)?;
        let name = format!("yo3-revival-report-{}.zip", chrono::Local::now().format("%Y%m%d-%H%M%S"));
        let zip = dir.join(&name);
        let system = crate::system_info::SystemInfo::collect();
        let context = Context { error: details.error.clone(), ..Context::default() };
        let draft = Draft { description: details.what.trim().to_owned(), steps: String::new() };
        let ids: Vec<String> = details.sent.iter().map(|(_, id)| id.clone()).collect();
        let markdown = draft.markdown(&context, &system, Some(&name), &ids);
        let mut recordings = vec![];
        if let Some(data) = &details.recordings {
            let mut pinned: Vec<(std::time::SystemTime, PathBuf, u64)> = std::fs::read_dir(data.join("recordings/pinned")).into_iter().flatten().flatten()
                .filter(|e| e.file_name().to_str().is_some_and(|n| n.ends_with(".jsonl.gz")))
                .filter_map(|e| { let m = e.metadata().ok()?; if !m.is_file() || m.len() > REPORT_RECORDING_BYTES { return None } Some((m.modified().ok()?, e.path(), m.len())) }).collect();
            pinned.sort_by_key(|p| std::cmp::Reverse(p.0));
            for (_, path, _) in pinned.into_iter().take(REPORT_RECORDINGS) {
                if let (Some(file), Ok(bytes)) = (path.file_name().and_then(|n| n.to_str()), std::fs::read(&path)) { recordings.push((format!("recordings/{file}"), bytes)) }
            }
        }
        let sent: Vec<serde_json::Value> = details.sent.iter().map(|(to, id)| serde_json::json!({ "to": to, "id": id })).collect();
        let report = serde_json::json!({
            "what": details.what.trim(), "contact": details.contact.trim(), "error": details.error.trim(), "play": details.play,
            "settings": details.settings, "sent": sent,
            "recordings": recordings.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
        });
        let readme = readme(details, &system, &recordings.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>());
        self.report(false)?.export_full(&zip, &serde_json::to_value(&context)?, &markdown, &system, &report, &readme, &recordings)?;
        self.diagnostics.line(&format!("Report a problem: saved {}", zip.display()));
        Ok(zip)
    }

    /// The public report form's link for a player who wants to talk about it on GitHub too: their words, the error,
    /// the report IDs and the saved file's name.
    pub fn report_form(&self, details: &Details, zip_name: Option<&str>) -> anyhow::Result<String> {
        use crate::bug_report::{Context, Draft};
        let context = Context { error: details.error.clone(), ..Context::default() };
        let draft = Draft { description: details.what.trim().to_owned(), steps: String::new() };
        let ids: Vec<String> = details.sent.iter().map(|(_, id)| id.clone()).collect();
        draft.url(&context, &crate::system_info::SystemInfo::collect(), zip_name, &ids)
    }
}

/// `README.txt`: the report for a person, and what to do with the file.
fn readme(d: &Details, s: &crate::system_info::SystemInfo, recordings: &[&str]) -> String {
    let or = |t: &str, none: &str| if t.trim().is_empty() { none.to_owned() } else { t.trim().to_owned() };
    let mut text = format!("YO3 Revival problem report (format {REPORT_FORMAT} {REPORT_VERSION})\n\n\
        Send this whole ZIP file to the developer: on the project's Discord, privately, or attached to a GitHub issue.\n\
        Your sign-in secrets are never in it, and personal details (your user folder, addresses) are removed from the logs.\n\n\
        What happened:\n{}\n\nHow to reach you: {}\nError shown: {}\nPlaying: {}\n\n",
        or(&d.what, "(nothing written)"), or(&d.contact, "(not given)"), or(&d.error, "(none)"), or(&d.play, "(unknown)"));
    text.push_str(&format!("YO3 Revival {} ({})\nSystem: {} {}{}\nProcessor: {} ({} threads)\nMemory: {}\nGraphics: {}\n\n",
        yo3_tunnel::BUILD_VERSION, yo3_tunnel::BUILD_COMMIT, s.os, s.arch, s.wine.as_ref().map(|w| format!(", {w}")).unwrap_or_default(),
        s.cpu.as_deref().unwrap_or("unknown"), s.threads, s.memory_mb.map_or("unknown".into(), |m| format!("{m} MiB")), s.gpu.as_deref().unwrap_or("unknown")));
    text.push_str("Nothing in this file was sent anywhere: this file is the report.\n");
    if !d.sent.is_empty() {
        let ids: Vec<&str> = d.sent.iter().map(|(_, id)| id.as_str()).collect();
        text.push_str(&format!("Reports the game sent its server's host by itself after recent games that broke: {}\n", ids.join(", ")));
    }
    if !recordings.is_empty() { text.push_str(&format!("Duel recordings (replayed exactly by the developer): {}\n", recordings.join(", "))) }
    text
}

/// `R-XXXX-XXXX`, as the server makes them.
pub fn is_report_id(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 11 && text.starts_with("R-") && b[6] == b'-' && b.iter().enumerate().all(|(i, c)| i < 2 || i == 6 || c.is_ascii_uppercase() || c.is_ascii_digit())
}

#[cfg(test)]
mod report_problem_tests {
    fn backend(test: &str) -> (std::sync::Arc<crate::Backend>, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("yo3-report-{test}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        (crate::Backend::new(root.clone()).unwrap(), root)
    }

    #[test]
    fn a_report_is_always_saved_as_one_file_in_the_documented_format() {
        // The player asked that a report never depend on the project's server: it is saved first, whatever was sent.
        let (backend, root) = backend("format");
        backend.diagnostics.line("token=abcdef0123456789abcdef0123456789 something to report");
        let data = root.join("singleplayer");
        std::fs::create_dir_all(data.join("recordings/pinned")).unwrap();
        std::fs::write(data.join("recordings/pinned/R-7K3Q-M2PA.jsonl.gz"), b"\x1f\x8b recording").unwrap();
        let details = super::Details { what: "Black Brutdrago closed my game".into(), contact: "Discord: duelist".into(), error: String::new(),
            sent: vec![("the server of a game that broke".into(), "R-AAAA-BBBB".into())], play: "singleplayer".into(),
            settings: serde_json::json!({ "resolution": "auto", "direct_start": false }), recordings: Some(data) };
        let zip = backend.save_report(&details).unwrap();
        assert!(zip.is_file() && zip.extension().is_some_and(|e| e == "zip"));
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&zip).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len()).map(|i| archive.by_index(i).unwrap().name().unwrap().to_string()).collect();
        for name in ["report.json", "README.txt", "bug-report.md", "recordings/R-7K3Q-M2PA.jsonl.gz"] { assert!(names.iter().any(|n| n == name), "{name} in {names:?}") }
        assert!(names.iter().any(|n| n.starts_with("sessions/")), "the logs: {names:?}");
        let mut read = |name: &str| { let mut text = String::new(); std::io::Read::read_to_string(&mut archive.by_name(name).unwrap(), &mut text).unwrap(); text };
        let report: serde_json::Value = serde_json::from_str(&read("report.json")).unwrap();
        assert_eq!((report["format"].as_str(), report["version"].as_u64()), (Some(super::REPORT_FORMAT), Some(super::REPORT_VERSION as u64)));
        assert_eq!(report["report"]["what"], "Black Brutdrago closed my game");
        assert_eq!(report["report"]["contact"], "Discord: duelist");
        assert_eq!(report["report"]["sent"][0]["id"], "R-AAAA-BBBB");
        assert_eq!(report["report"]["recordings"][0], "recordings/R-7K3Q-M2PA.jsonl.gz");
        assert!(report["system"]["threads"].as_u64().is_some_and(|t| t > 0), "the PC: {}", report["system"]);
        assert!(report["release"].is_string() && report["commit"].is_string());
        let readme = read("README.txt");
        assert!(readme.contains("Send this whole ZIP file to the developer") && readme.contains("Nothing in this file was sent anywhere") && readme.contains("by itself after recent games that broke: R-AAAA-BBBB"), "{readme}");
        assert!(readme.contains("sign-in secrets are never in it") && !readme.contains("omitted"), "the README's own lines survive the filter: {readme}");
        let all: String = names.iter().filter(|n| !n.starts_with("recordings/")).map(|n| read(n)).collect();
        assert!(!all.contains("abcdef0123456789abcdef0123456789"), "no token in any file");
        drop(backend);
        std::fs::remove_dir_all(root).unwrap();
    }



    #[test]
    fn the_helpers_report_ids_from_recent_games_are_found() {
        let (backend, root) = backend("recent");
        let sessions = root.join("logs/sessions");
        let game = yo3_tunnel::diagnostics::Session::start(&sessions, "game", None).unwrap();
        game.line("Problem report R-7K3Q-M2PA sent to the server (network): quote it when you report this problem");
        game.line("Problem report not sent: the sign-in ended");
        assert_eq!(backend.recent_report_ids(), vec!["R-7K3Q-M2PA".to_string()]);
        assert!(backend.problem_logs().contains("== The latest game ==") && backend.problem_logs().contains("R-7K3Q-M2PA"));
        drop((backend, game));
        let _ = std::fs::remove_dir_all(root);
    }

    /// The setting reaches every helper: Play's (its flag) and direct start's, which only sees the file beside the profiles.
    #[test]
    fn turning_problem_reports_off_reaches_the_helper() {
        let (backend, root) = backend("switch");
        let off = root.join("profiles").join(yo3_launcher::problems::OFF_FILE);
        let mut settings = crate::Settings { problem_reports: false, ..crate::Settings::default() };
        backend.sync_problem_reports(&settings);
        assert!(off.is_file() && yo3_launcher::problems::off(&root.join("profiles/direct.json")));
        let cmd = backend.launch_command(&settings, &root.join("profiles/connection-0.json"), &root.join("profiles/ready.json")).unwrap();
        assert!(cmd.get_args().any(|a| a == "--no-problem-reports"));
        settings.problem_reports = true;
        backend.sync_problem_reports(&settings);
        assert!(!off.exists());
        let cmd = backend.launch_command(&settings, &root.join("profiles/connection-0.json"), &root.join("profiles/ready.json")).unwrap();
        assert!(!cmd.get_args().any(|a| a == "--no-problem-reports"));
        drop(backend);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn report_ids_are_recognised_exactly() {
        assert!(super::is_report_id("R-7K3Q-M2PA"));
        for not in ["R-7K3Q-M2P", "X-7K3Q-M2PA", "R-7k3q-m2pa", "R-7K3QM-2PA", "not"] { assert!(!super::is_report_id(not), "{not}") }
    }
}
