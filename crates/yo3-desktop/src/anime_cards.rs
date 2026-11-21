//! The Anime cards (#60) and Destiny Draw (#71) switches of Settings → Singleplayer: your own server's live `anime-cards` setting. The value is always
//! the server's: read from the running server (its admin API), else from the saved settings in its data folder, and written
//! the same two ways, so there is no copy in the launcher that could drift. A running server applies it at the next duel and
//! its card profile then differs from the one an open game loaded, which is what makes Desktop offer RESTART THE GAME; a
//! stopped server applies it when it starts. Only your own server is touched: a joined server's settings are its host's.
use anyhow::Context;
use yo3_server::api::{self, Op};
use crate::{Backend, Settings};

/// The setting's name in the server's registry.
pub const NAME: &str = "anime-cards";

/// The Destiny Draw switch's setting (#71), kept the same way.
pub const DESTINY_DRAW: &str = "destiny-draw";

impl Backend {
    /// Whether your own server plays the anime card versions: asked of the running server, else read from its saved settings.
    pub fn anime_cards(&self, settings: &Settings) -> anyhow::Result<bool> { self.flag(settings, NAME) }

    /// Turns the anime card versions on or off (off is the default) on your own server, and returns the value the server now has.
    pub fn set_anime_cards(&self, settings: &Settings, on: bool) -> anyhow::Result<bool> { self.set_flag(settings, NAME, on) }

    /// Whether your own server offers Destiny Draw (#71) in duels against the computer: read the same two ways.
    pub fn destiny_draw(&self, settings: &Settings) -> anyhow::Result<bool> { self.flag(settings, DESTINY_DRAW) }

    /// Turns Destiny Draw on or off (off is the default) on your own server; it applies at the next duel, no restart.
    pub fn set_destiny_draw(&self, settings: &Settings, on: bool) -> anyhow::Result<bool> { self.set_flag(settings, DESTINY_DRAW, on) }

    /// An on/off setting of your own server: asked of the running server, else read from its saved settings.
    fn flag(&self, settings: &Settings, name: &str) -> anyhow::Result<bool> {
        if let Some(live) = self.live(settings) {
            let answer = api::call(&live, &Op::Settings).map_err(anyhow::Error::msg)?;
            let row = answer["settings"].as_array().and_then(|rows| rows.iter().find(|r| r["name"] == name)).context("The server doesn't know this setting.")?;
            return Ok(row["value"] == "on");
        }
        Ok(*self.saved_settings(settings)?.now().value(name) == "on")
    }

    fn set_flag(&self, settings: &Settings, name: &str, on: bool) -> anyhow::Result<bool> {
        if let Some(live) = self.live(settings) {
            let op = if on { Op::Set { name: name.into(), value: "on".into() } } else { Op::Reset { name: name.into() } };
            api::call(&live, &op).map_err(anyhow::Error::msg)?;
        } else {
            let saved = self.saved_settings(settings)?;
            let done = if on { saved.set(name, "on") } else { saved.reset(name) };
            done.map_err(anyhow::Error::msg)?;
        }
        self.flag(settings, name)
    }

    fn saved_settings(&self, settings: &Settings) -> anyhow::Result<yo3_server::settings::Live> {
        let data = self.data_dir(settings);
        // The server keeps its settings in server.db and ignores a settings.json file, so the database has to exist
        // before the first write: a switch set before the server's first start would otherwise be lost.
        yo3_server::store::initialize(&data)?;
        Ok(yo3_server::settings::Live::open(Some(&data.join("settings.json")), crate::banlists(std::path::Path::new(&settings.game_dir)))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    use yo3_server::admin;

    /// A server that answers `settings`, `set` and `reset` for `anime-cards`, keeping its value; the requests it got.
    fn mock(listener: std::net::TcpListener, value: Arc<Mutex<String>>, seen: Arc<Mutex<Vec<String>>>) {
        std::thread::spawn(move || for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
            let (mut header, mut byte) = (Vec::new(), [0]);
            while !header.ends_with(b"\r\n\r\n") { if stream.read_exact(&mut byte).is_err() { break } header.push(byte[0]) }
            let header = String::from_utf8_lossy(&header).into_owned();
            let Some(length) = header.lines().find_map(|l| l.strip_prefix("Content-Length: ")).and_then(|n| n.parse::<usize>().ok()) else { continue };
            let mut body = vec![0; length];
            if stream.read_exact(&mut body).is_err() { continue }
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            seen.lock().unwrap().push(body.to_string());
            let result = match body["op"].as_str().unwrap() {
                "settings" => serde_json::json!({ "settings": [{ "name": NAME, "value": *value.lock().unwrap() }] }),
                "set" => { *value.lock().unwrap() = body["value"].as_str().unwrap().to_string(); serde_json::json!({ "message": "ok" }) }
                _ => { *value.lock().unwrap() = "off".into(); serde_json::json!({ "message": "ok" }) }
            };
            let reply = serde_json::json!({ "ok": true, "result": result }).to_string();
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.len(), reply);
        });
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("yo3-anime-switch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn a_stopped_server_keeps_the_switch_in_its_data_and_off_is_the_default() {
        let root = scratch("stopped");
        let backend = Backend::new(root.clone()).unwrap();
        let settings = Settings::default();
        assert!(!backend.anime_cards(&settings).unwrap(), "off until a player turns it on");
        assert!(backend.set_anime_cards(&settings, true).unwrap());
        // A fresh read, as the server's start would do, sees what was saved: the server keeps its settings in server.db
        // and ignores a settings.json file (live on Windows 11: the switch set before the first start was lost).
        let data = backend.data_dir(&settings);
        let file = data.join("settings.json");
        assert!(!file.exists(), "a plain settings.json is a file the server doesn't read");
        yo3_server::store::initialize(&data).unwrap();
        let as_the_server_reads_it = yo3_server::settings::Live::open(Some(&file), Vec::new()).unwrap();
        assert!(as_the_server_reads_it.now().anime_cards, "the server's start finds the switch on");
        assert!(Backend::new(root.clone()).unwrap().anime_cards(&settings).unwrap());
        assert!(!backend.set_anime_cards(&settings, false).unwrap());
        assert!(!yo3_server::settings::Live::open(Some(&file), Vec::new()).unwrap().now().anime_cards, "off is the default again");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn destiny_draw_is_kept_the_same_way_beside_anime_cards() {
        let root = scratch("destiny");
        let backend = Backend::new(root.clone()).unwrap();
        let settings = Settings::default();
        assert!(!backend.destiny_draw(&settings).unwrap(), "off until a player turns it on");
        assert!(backend.set_destiny_draw(&settings, true).unwrap());
        assert!(!backend.anime_cards(&settings).unwrap(), "the other switch is untouched");
        let data = backend.data_dir(&settings);
        assert!(yo3_server::settings::Live::open(Some(&data.join("settings.json")), Vec::new()).unwrap().now().destiny_draw, "the server's start finds it on");
        assert!(!backend.set_destiny_draw(&settings, false).unwrap());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_running_server_is_asked_and_told_and_the_saved_file_is_not_the_answer() {
        let root = scratch("running");
        let backend = Backend::new(root.clone()).unwrap();
        let settings = Settings::default();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        admin::write_live(&backend.data_dir(&settings), listener.local_addr().unwrap().port()).unwrap();
        let (value, seen) = (Arc::new(Mutex::new("off".to_string())), Arc::new(Mutex::new(Vec::new())));
        mock(listener, value.clone(), seen.clone());
        // The server says on while the saved file says nothing: the server's value is the one shown.
        *value.lock().unwrap() = "on".into();
        assert!(backend.anime_cards(&settings).unwrap());
        assert!(!backend.set_anime_cards(&settings, false).unwrap());
        assert_eq!(*value.lock().unwrap(), "off");
        assert!(backend.set_anime_cards(&settings, true).unwrap());
        assert_eq!(*value.lock().unwrap(), "on");
        let ops: Vec<String> = seen.lock().unwrap().iter().map(|b| serde_json::from_str::<serde_json::Value>(b).unwrap()["op"].as_str().unwrap().to_string()).collect();
        assert_eq!(ops, ["settings", "reset", "settings", "set", "settings"]);
        assert!(!backend.data_dir(&settings).join("settings.json").exists(), "a running server's setting isn't written behind its back");
        let _ = std::fs::remove_dir_all(root);
    }
}
