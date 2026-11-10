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
        ensure!(destination.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")), "Choose a .zip report file");
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o600);
        archive.start_file("report.json", options)?;
        archive.write_all(&serde_json::to_vec_pretty(&serde_json::json!({"format":1,"created":logs::now(),"preview":self.preview,
            "release":yo3_tunnel::BUILD_VERSION,"commit":yo3_tunnel::BUILD_COMMIT,"protocol":yo3_tunnel::PROTOCOL_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,
            "os":std::env::consts::OS,"note":"User-created report. No database, connection profiles, keys, game files or automatic uploads. Detailed traces may contain player/chat information; review before sharing."}))?)?;
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
                    if let Some(text) = field.as_str() { *field = logs::redact(text, &[], home.as_deref()).into(); }
                } }
                serde_json::to_string_pretty(&value)?
            } else { sanitize(&String::from_utf8_lossy(&bytes), home.as_deref()) };
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
fn sanitize(text: &str, home: Option<&str>) -> String {
    let mut out = String::new(); let mut private_key = false;
    for line in text.lines() {
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") { private_key = true; }
        if private_key { if line.contains("-----END") && line.contains("PRIVATE KEY-----") { private_key = false; } out.push_str("[private key omitted]\n"); }
        else if line.len() > 16384 { out.push_str("[overlong diagnostic line omitted]\n"); }
        else { out.push_str(&logs::redact(line, &[], home)); out.push('\n'); }
    }
    out
}
