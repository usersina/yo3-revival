//! Each form owns only its fields; saving it never applies another form's unfinished draft.
use crate::Settings;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section { Game, Hosting, Diagnostics }
impl Section {
    pub fn merge(self, saved: &Settings, draft: &Settings) -> Settings {
        let mut result = saved.clone();
        match self {
            Self::Game => { result.game_dir = draft.game_dir.clone(); result.wine_prefix = draft.wine_prefix.clone(); }
            Self::Hosting => { result.host_address = draft.host_address.clone(); result.data_dir = draft.data_dir.clone(); result.player_name = draft.player_name.clone();
                result.bind_address = draft.bind_address.clone(); result.bind_interface = draft.bind_interface.clone(); result.listen_port = draft.listen_port; }
            Self::Diagnostics => result.detailed_logs = draft.detailed_logs,
        }
        result
    }
    pub fn changed(self, saved: &Settings, draft: &Settings) -> bool { self.merge(saved, draft) != *saved }
}
impl crate::Backend {
    pub fn save_preferences(&self, section: Section, draft: &Settings) -> anyhow::Result<Settings> {
        use anyhow::ensure;
        let db = crate::LauncherDb::open(&self.root)?; let saved = db.settings()?; let mut next = section.merge(&saved, draft);
        match section {
            Section::Game => {
                if next.game_dir != saved.game_dir || next.wine_prefix != saved.wine_prefix {
                    ensure!(!self.game_running() && self.live(&saved).is_none(), "Close your game and stop your server before changing the game installation.");
                    ensure!(!self.root.join("profiles/loader.ini").exists() && !std::path::Path::new(&saved.game_dir).join("yo3-revival-loader.json").exists(), "Disable direct launch for the old game location before changing it.");
                }
                crate::validate_game(std::path::Path::new(&next.game_dir))?;
            }
            Section::Hosting => {
                ensure!(!section.changed(&saved, &next) || self.live(&saved).is_none(), "Stop your server before saving hosting changes.");
                crate::server_address(&next.host_address)?;
                ensure!(next.listen_port > 0, "Port must be between 1 and 65535.");
                crate::network::resolve_bind(&next, &crate::network::bindings()?)?;
                next.player_name = next.player_name.trim().into();
                ensure!(!next.player_name.is_empty() && next.player_name.chars().count() <= 16, "First player name must contain 1–16 characters.");
                if next.data_dir != saved.data_dir { next.local_account.clear(); }
            }
            Section::Diagnostics => {},
        }
        db.save_settings(&next)?;
        if section != Section::Diagnostics {
            let remote = next.selected_server.and_then(|id| db.servers().ok()?.into_iter().find(|s| s.id == id));
            if let Err(error) = self.sync_direct(&next, remote) { self.diagnostics.line(&format!("Direct-launch profile needs refresh: {error:#}")); }
        }
        Ok(next)
    }
}
