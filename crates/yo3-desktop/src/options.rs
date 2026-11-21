//! The launcher's Settings (#31) and Host for friends (#37), one change at a time: each is checked, saved in
//! `launcher.db`, and read again at the next Play or server start, so none needs a restart of the launcher.
use std::path::Path;
use anyhow::ensure;
use crate::{LauncherDb, Settings};

/// The log level's values: the server's own tracing filter (`RUST_LOG`) for your servers.
pub const DETAILED: &str = "detailed";

/// One setting, as the player changed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// The game's screen size: "off" (the game's own 1024×768), "auto" or "WIDTHxHEIGHT".
    Resolution(String),
    /// Open the game without the launcher (direct start).
    DirectStart(bool),
    /// Singleplayer's banlist: "" for the server's default, else a regulation id or name.
    Banlist(String),
    /// Linux: the Wine prefix ("" for Wine's default).
    WinePrefix(String),
    /// Linux: the Wine program ("" for `wine` on the PATH).
    WineBinary(String),
    /// Detailed logs for your servers.
    DetailedLogs(bool),
    /// Send a broken game's logs to the server it played on (problem reports).
    ProblemReports(bool),
    /// Host for friends: the port friends connect to.
    HostPort(u16),
    /// Host for friends: the address friends dial ("" for this PC's network address).
    HostShare(String),
    /// Host for friends: the server's name, as friends see it in their list.
    HostName(String),
}

impl Change {
    /// Checks the change and applies it to `settings`.
    pub fn apply(&self, settings: &mut Settings) -> anyhow::Result<()> {
        match self {
            Change::Resolution(value) => {
                settings.resolution = value.parse::<yo3_launcher::resolution::Resolution>().map_err(|_| anyhow::anyhow!("Choose one of the screen sizes."))?.to_string();
            }
            Change::DirectStart(on) => settings.direct_start = *on,
            Change::Banlist(value) => {
                let value = value.trim().to_string();
                let id = value.parse::<u32>().ok();
                ensure!(value.is_empty() || id.is_some_and(|id| (1..=255).contains(&id))
                    || (id.is_none() && value.len() <= 64 && value.starts_with(|c: char| c.is_ascii_alphanumeric())
                        && value.chars().all(|c| c.is_ascii_alphanumeric() || " /.-()".contains(c))),
                    "Choose one of the game's banlists.");
                settings.banlist = value;
            }
            Change::WinePrefix(value) => {
                let value = value.trim();
                ensure!(value.is_empty() || Path::new(value).join("drive_c").is_dir(), "That folder isn't a Wine prefix: it has no drive_c folder.");
                settings.wine_prefix = value.into();
            }
            Change::WineBinary(value) => {
                let value = value.trim();
                ensure!(value.is_empty() || Path::new(value).is_file(), "That Wine program doesn't exist.");
                settings.wine_binary = value.into();
            }
            Change::DetailedLogs(on) => settings.log_level = if *on { DETAILED.into() } else { String::new() },
            Change::ProblemReports(on) => settings.problem_reports = *on,
            Change::HostPort(port) => {
                ensure!(*port >= 1024, "Choose a port from 1024 to 65535.");
                settings.host_port = *port;
            }
            Change::HostShare(value) => {
                let value = value.trim();
                if !value.is_empty() {
                    ensure!(!value.contains(':') || value.parse::<std::net::IpAddr>().is_ok(), "Type the address without a port: the port is set below.");
                    let (_, name) = crate::server_address(value).map_err(|_| anyhow::anyhow!("Type a name or an IP address, like play.example.com or 203.0.113.7."))?;
                    let own = name.eq_ignore_ascii_case("localhost") || name.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback() || ip.is_unspecified());
                    ensure!(!own, "Friends can't reach this computer's own loopback address: type its network address.");
                }
                settings.host_share = value.into();
            }
            Change::HostName(value) => {
                let value: String = value.trim().into();
                ensure!((1..=32).contains(&value.chars().count()) && !value.chars().any(char::is_control), "A server name has 1 to 32 characters.");
                settings.host_name = value;
            }
        }
        Ok(())
    }
}

impl LauncherDb {
    /// Applies `change` to the saved settings and saves them: the settings as they are now.
    pub fn change(&self, change: &Change) -> anyhow::Result<Settings> {
        let mut settings = self.settings()?;
        change.apply(&mut settings)?;
        self.save_settings(&settings)?;
        Ok(settings)
    }
}

/// Whether your servers log in detail.
pub fn detailed(settings: &Settings) -> bool { settings.log_level == DETAILED }

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        let at = std::env::temp_dir().join(format!("yo3-options-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&at).unwrap();
        at
    }

    /// Saves `change` and reads the settings back as a fresh launcher would.
    fn saved(change: Change) -> anyhow::Result<Settings> {
        let root = root();
        LauncherDb::open(&root)?.change(&change)?;
        let settings = LauncherDb::open(&root)?.settings();
        std::fs::remove_dir_all(root).unwrap();
        settings
    }

    #[test]
    fn the_screen_size_is_saved_in_its_normal_form_and_a_bad_one_refused() {
        assert_eq!(saved(Change::Resolution("1920X1080".into())).unwrap().resolution, "1920x1080");
        assert_eq!(saved(Change::Resolution("".into())).unwrap().resolution, "off");
        assert_eq!(saved(Change::Resolution("auto".into())).unwrap().resolution, "auto");
        assert!(saved(Change::Resolution("huge".into())).is_err());
    }

    #[test]
    fn direct_start_is_saved() {
        assert!(!saved(Change::DirectStart(false)).unwrap().direct_start);
        assert!(saved(Change::DirectStart(true)).unwrap().direct_start);
    }

    #[test]
    fn the_banlist_takes_an_id_a_name_or_the_default() {
        assert_eq!(saved(Change::Banlist(" 4 ".into())).unwrap().banlist, "4");
        assert_eq!(saved(Change::Banlist("Season 13".into())).unwrap().banlist, "Season 13");
        assert_eq!(saved(Change::Banlist("".into())).unwrap().banlist, "");
        for bad in ["0", "256", "--regulation", "a\"b"] { assert!(saved(Change::Banlist(bad.into())).is_err(), "{bad}") }
    }

    #[test]
    fn the_wine_prefix_must_be_a_prefix_and_the_wine_program_must_exist() {
        let dir = root();
        assert!(saved(Change::WinePrefix(dir.to_string_lossy().into())).is_err(), "no drive_c");
        std::fs::create_dir(dir.join("drive_c")).unwrap();
        assert_eq!(saved(Change::WinePrefix(dir.to_string_lossy().into())).unwrap().wine_prefix, dir.to_string_lossy());
        assert_eq!(saved(Change::WinePrefix(" ".into())).unwrap().wine_prefix, "");
        assert!(saved(Change::WineBinary(dir.join("wine").to_string_lossy().into())).is_err());
        std::fs::write(dir.join("wine"), b"#!/bin/sh\n").unwrap();
        assert_eq!(saved(Change::WineBinary(dir.join("wine").to_string_lossy().into())).unwrap().wine_binary, dir.join("wine").to_string_lossy());
        assert_eq!(saved(Change::WineBinary("".into())).unwrap().wine_binary, "");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn detailed_logs_are_a_level() {
        let on = saved(Change::DetailedLogs(true)).unwrap();
        assert!(detailed(&on));
        assert!(!detailed(&saved(Change::DetailedLogs(false)).unwrap()));
    }

    #[test]
    fn hosting_takes_a_port_an_address_friends_can_reach_and_a_name() {
        assert_eq!(saved(Change::HostPort(7444)).unwrap().host_port, 7444);
        assert!(saved(Change::HostPort(80)).is_err(), "a privileged port");
        assert_eq!(saved(Change::HostShare(" play.example.com ".into())).unwrap().host_share, "play.example.com");
        assert_eq!(saved(Change::HostShare("203.0.113.7".into())).unwrap().host_share, "203.0.113.7");
        assert_eq!(saved(Change::HostShare("".into())).unwrap().host_share, "");
        for bad in ["localhost", "127.0.0.1", "0.0.0.0", "a/b", "play.example.com:7443"] { assert!(saved(Change::HostShare(bad.into())).is_err(), "{bad}") }
        assert_eq!(saved(Change::HostName(" Friday duels ".into())).unwrap().host_name, "Friday duels");
        assert!(saved(Change::HostName("".into())).is_err());
        assert!(saved(Change::HostName("x".repeat(33))).is_err());
    }
}
