//! Each form owns only its fields; saving it never applies another form's unfinished draft.
use crate::Settings;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section { Game, Hosting }
impl Section {
    pub fn merge(self, saved: &Settings, draft: &Settings) -> Settings {
        let mut result = saved.clone();
        match self {
            Self::Game => { result.game_dir = draft.game_dir.clone(); result.wine_prefix = draft.wine_prefix.clone(); }
            Self::Hosting => { result.host_address = draft.host_address.clone(); result.data_dir = draft.data_dir.clone(); result.player_name = draft.player_name.clone();
                result.bind_address = draft.bind_address.clone(); result.bind_interface = draft.bind_interface.clone(); result.listen_port = draft.listen_port;
                result.server_options = draft.server_options.clone(); }
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
                    // Direct start goes with the old location; the next Play sets it up for the new one. Only direct start
                    // this launcher set up: a fresh launcher's first folder is the default install, which may hold another
                    // launcher's (live on Windows 11, removing it asked for an administrator).
                    let old_loader = std::path::Path::new(&saved.game_dir).join("yo3-revival-loader.json");
                    if (self.root.join("profiles/loader.ini").exists() || (saved.setup_complete && old_loader.exists()))
                        && let Err(error) = self.disable_direct(&saved)
                    {
                        ensure!(!self.root.join("profiles/loader.ini").exists() && !old_loader.exists(),
                            "Could not take direct start out of the old game folder ({error:#}). Use Remove from game folder, then change it.");
                        self.diagnostics.line(&format!("Direct start removed; its registry entry stays: {error:#}"));
                    }
                }
                crate::validate_game(std::path::Path::new(&next.game_dir))?;
            }
            Section::Hosting => {
                ensure!(!section.changed(&saved, &next) || self.live(&saved).is_none(), "Stop your server before saving hosting changes.");
                crate::server_address(&next.host_address)?;
                ensure!(next.listen_port > 0, "Port must be between 1 and 65535.");
                crate::network::resolve_bind(&next, &crate::network::bindings()?)?;
                crate::server_options::arguments(&next)?;
                next.player_name = next.player_name.trim().into();
                ensure!(!next.player_name.is_empty() && next.player_name.chars().count() <= 16, "First player name must contain 1–16 characters.");
                if next.data_dir != saved.data_dir { next.local_account.clear(); }
            }
        }
        db.save_settings(&next)?;
        if let Err(error) = self.sync_direct(&next) { self.diagnostics.line(&format!("Direct-launch profile needs refresh: {error:#}")); }
        Ok(next)
    }
}
