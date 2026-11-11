//! Local-only diagnostic archives, assembled from our session inventory instead of arbitrary user directories.
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use serde::Serialize;
use anyhow::ensure;
use yo3_tunnel::diagnostics::{self as logs, Record, Metadata};
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
        ensure!(destination.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")), "Choose a .zip report file");
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o600);
        archive.start_file("report.json", options)?;
        archive.write_all(&serde_json::to_vec_pretty(&serde_json::json!({"format":2,"created":logs::now(),"preview":self.preview,"personal_data_filtered":remove_personal,
            "release":yo3_tunnel::BUILD_VERSION,"commit":yo3_tunnel::BUILD_COMMIT,"protocol":yo3_tunnel::PROTOCOL_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"note":"User-created report. Credentials, databases, profiles and game files are excluded. Personal filtering omits client/packet/engine traces and player activity, and masks common user paths and network addresses; review remaining text before sharing. Nothing is uploaded."}))?)?;
        let current = logs::records(&self.root)?;
        let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok();
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
            archive.start_file(format!("sessions/{}", record.name), options)?; archive.write_all(text.as_bytes())?;
        }
        let bytes = archive.finish()?.into_inner();
        // Complete the archive before replacing a selected report; errors cannot leave a half-written ZIP.
        let stage = destination.with_extension(format!("{}.zip.tmp", std::process::id()));
        let mut file = logs::private_create(&stage)?; file.write_all(&bytes)?; file.sync_all()?; drop(file);
        let result = std::fs::rename(&stage, destination); if result.is_err() { let _ = std::fs::remove_file(&stage); } result?;
        Ok(())
    }
}
fn sanitize(text: &str, home: Option<&str>, remove_personal: bool) -> String {
    let mut out = String::new(); let mut private_key = false;
    for line in text.lines() {
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") { private_key = true; }
        if private_key { if line.contains("-----END") && line.contains("PRIVATE KEY-----") { private_key = false; } out.push_str("[private key omitted]\n"); }
        else if line.len() > 16384 { out.push_str("[overlong diagnostic line omitted]\n"); }
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
