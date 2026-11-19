//! The server's TOML file (#24): what only a restart changes. Listener addresses and ports, the TLS identity, the
//! folders and the rules engine's program. Live settings are not here: they are in the database (`settings.rs`), and
//! `yo3-server set` changes them while the server runs.
//!
//! ```toml
//! bind = "0.0.0.0"
//! auth_port = 7443
//! http_port = 8080
//! advertise = "play.example"
//! game_dir = "/games/Yu-Gi-Oh! ONLINE 3"
//! data_dir = "data"          # relative paths are from this file's folder
//! log_dir = "logs"
//! auth_cert = "data/server.crt"
//! auth_key = "data/server.key"
//! engine = "yo3-engine"
//! packet_log = false        # no packet files or traces (a hosted server's privacy)
//! ```
//!
//! A command-line flag wins over the file, and the file over the defaults.
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The file the server reads when `--config` names none, from the current folder, if it is there.
pub const DEFAULT_FILE: &str = "server.toml";

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub bind: Option<IpAddr>,
    pub auth_port: Option<u16>,
    pub http_port: Option<u16>,
    /// Opt-in staff JSON API port (always loopback, separate from host-token management). Omit to disable.
    pub dashboard_port: Option<u16>,
    /// Staff API socket shared with nginx (Unix only); never mounts host-token routes.
    pub dashboard_socket: Option<PathBuf>,
    /// Exact browser origin for the socket proxy (HTTPS, or HTTP localhost for local checks).
    pub dashboard_origin: Option<String>,
    /// `required` (default) or `optional`: whether staff must use an authenticator (docs/dashboard.md).
    pub staff_mfa: Option<crate::staff_mfa::StaffMfaPolicy>,
    /// Allow backup downloads through the public dashboard transport (off by default).
    pub dashboard_public_backups: Option<bool>,
    pub advertise: Option<String>,
    pub game_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub log_dir: Option<PathBuf>,
    pub auth_cert: Option<PathBuf>,
    pub auth_key: Option<PathBuf>,
    /// The rules engine's program (else `YO3_ENGINE`, or the one beside the server).
    pub engine: Option<PathBuf>,
    /// `false`: no raw packet files and no packet traces in the session logs (`--no-packet-log --no-packet-trace`).
    pub packet_log: Option<bool>,
}

impl FileConfig {
    /// The file at `path`, its relative paths made relative to its folder.
    pub fn read(path: &Path) -> anyhow::Result<FileConfig> {
        let text = std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        let mut file: FileConfig = toml::from_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        let base = path.parent().unwrap_or(Path::new("."));
        for p in [&mut file.game_dir, &mut file.data_dir, &mut file.log_dir, &mut file.auth_cert, &mut file.auth_key, &mut file.engine, &mut file.dashboard_socket].into_iter().flatten() {
            if p.is_relative() { *p = base.join(&*p) }
        }
        Ok(file)
    }

    /// The file `--config` names (which must exist), else `server.toml` in the current folder if there is one.
    pub fn find(named: Option<&Path>) -> anyhow::Result<Option<(PathBuf, FileConfig)>> {
        match named {
            Some(path) => Ok(Some((path.to_path_buf(), FileConfig::read(path)?))),
            None if Path::new(DEFAULT_FILE).is_file() => Ok(Some((DEFAULT_FILE.into(), FileConfig::read(Path::new(DEFAULT_FILE))?))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_read_with_paths_from_its_folder_and_unknown_keys_refused() {
        let dir = std::env::temp_dir().join(format!("yo3-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("server.toml");
        std::fs::write(&file, "bind = \"0.0.0.0\"\nauth_port = 17443\ndata_dir = \"state\"\nlog_dir = \"/var/log/yo3\"\n").unwrap();
        let read = FileConfig::read(&file).unwrap();
        assert_eq!(read.bind, Some("0.0.0.0".parse().unwrap()));
        assert_eq!((read.auth_port, read.http_port), (Some(17443), None));
        assert_eq!(read.data_dir, Some(dir.join("state")));
        assert_eq!(read.log_dir, Some(PathBuf::from("/var/log/yo3")));
        // A live setting in the file is a mistake worth saying: it belongs to `yo3-server set`.
        std::fs::write(&file, "registration = \"open\"\n").unwrap();
        assert!(FileConfig::read(&file).unwrap_err().to_string().contains("unknown field"));
        assert!(FileConfig::find(Some(&dir.join("missing.toml"))).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_server_bundles_example_file_is_read_with_packet_logs_off() {
        // tools/host/server.toml.example, the hosting guide's /etc/yo3/server.toml (#37).
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/host/server.toml.example");
        let read = FileConfig::read(&example).unwrap();
        assert_eq!((read.auth_port, read.http_port, read.packet_log), (Some(7443), Some(8080), Some(false)));
        assert_eq!(read.data_dir, Some(PathBuf::from("/var/lib/yo3")));
        assert_eq!(read.engine, Some(PathBuf::from("/opt/yo3-revival/yo3-engine")));
    }
}
