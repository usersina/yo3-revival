//! Banlists the players keep on the server (#77): each account may import EDOPro `lflist.conf` lists (Desktop's
//! Banlists, through the admin API on your own server or TLS operation 8 on a friend's), up to `player-banlists`
//! (default 3), export them back and delete them. They are banlists like the game's and the host's (numbered after
//! those, never reused): anyone may pick one for their duels against the computer or a Create Duel room, so every
//! player's launcher writes them all into the game (`yo3_data::added::AddedBanlist`). Kept in `server.db` (the
//! `banlists.json` domain), so in backups, and changed while the server runs: the server reads them from here
//! (`World::regulation`, `World::banlist_names`).
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::json;
use yo3_data::added::AddedBanlist;

use crate::banlists::{self, Lflist};
use crate::lock::Locked;

/// A list of a player's: its number, name, owner (account and the name shown) and cards by limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerList {
    pub id: u32,
    pub name: String,
    pub owner: u32,
    pub owner_name: String,
    pub lists: [Vec<u16>; 4],
    pub default_limit: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    next_id: u32,
    #[serde(default)]
    lists: Vec<PlayerList>,
}

/// The players' lists and where they are kept (none: in memory, for tests).
#[derive(Debug, Default)]
pub struct Store {
    path: Option<PathBuf>,
    saved: Saved,
    /// The first number a new list may get: after the game's and the host's.
    floor: u32,
    /// A game banlist's record, the shape every list's `.y3r` and check take.
    template: Option<yo3_proto::Message>,
}

pub type Shared = Arc<Mutex<Store>>;

/// A name is 3 to 24 characters, printable, unique among every banlist (any case).
pub const NAME_CHARS: std::ops::RangeInclusive<usize> = 3..=24;

impl Store {
    /// The lists kept in `dir` (`banlists.json`, in `server.db` when the folder has one), numbered from `floor` on.
    pub fn open(dir: Option<&Path>, floor: u32, template: Option<yo3_proto::Message>) -> Store {
        let path = dir.map(|d| d.join("banlists.json"));
        let saved = path.as_deref().map(|p| crate::store::read_or_keep(p, "players' banlists")).and_then(|j| serde_json::from_value(j).ok()).unwrap_or_default();
        Store { path, saved, floor, template }
    }

    pub fn shared(self) -> Shared { Arc::new(Mutex::new(self)) }

    /// The first number no list of theirs ever took: the host's new lists start there.
    pub fn next_free(&self) -> u32 {
        self.saved.next_id.max(self.saved.lists.iter().map(|l| l.id + 1).max().unwrap_or(0))
    }

    /// New lists get at least `floor` (after the host's numbers).
    pub fn raise_floor(&mut self, floor: u32) { self.floor = self.floor.max(floor) }

    fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else { return Ok(()) };
        crate::store::write(path, &serde_json::to_value(&self.saved).unwrap_or_default()).map_err(|e| format!("the banlists could not be saved: {e}"))
    }

    pub fn all(&self) -> &[PlayerList] { &self.saved.lists }

    pub fn of(&self, owner: u32) -> Vec<&PlayerList> { self.saved.lists.iter().filter(|l| l.owner == owner).collect() }

    pub fn get(&self, id: u32) -> Option<&PlayerList> { self.saved.lists.iter().find(|l| l.id == id) }

    /// The list as the server checks decks against it.
    pub fn regulation(&self, id: u32) -> Option<yo3_data::Regulation> {
        let l = self.get(id)?;
        let added = l.added(0);
        Some(match &self.template {
            Some(t) => banlists::regulation(&added, t),
            None => yo3_data::Regulation { id: l.id, kind: 0, status: 0, name_id: 0, lists: l.lists.clone(), default_limit: l.default_limit, record: Default::default() },
        })
    }

    /// Keeps the first list of an `lflist.conf` text for `owner` as `name` (else the file's own name): at most `cap`
    /// lists an account, a name no banlist has (`taken`: the game's and the host's). The list, and how many cards were
    /// left out (passcodes of no card of the client's).
    #[allow(clippy::too_many_arguments, reason = "The owner, the file, the server's terms and the card lookup are each needed once.")]
    pub fn import(&mut self, owner: u32, owner_name: &str, name: &str, text: &str, cap: u32, taken: &[String], cid: impl Fn(u32) -> Option<u16>)
        -> Result<(PlayerList, usize), String> {
        if cap == 0 { return Err("This server doesn't keep players' banlists.".into()) }
        if self.of(owner).len() as u32 >= cap {
            return Err(format!("You already keep {cap} banlist(s) on this server: delete one first."));
        }
        let parsed = banlists::parse(text);
        let Some(first) = parsed.first() else { return Err("That file holds no banlist (an EDOPro lflist.conf starts a list with !Name).".into()) };
        let name = if name.trim().is_empty() { first.name.trim().to_string() } else { name.trim().to_string() };
        let count = name.chars().count();
        if !NAME_CHARS.contains(&count) || name.chars().any(char::is_control) {
            return Err(format!("A banlist's name is {} to {} characters (\"{}\").", NAME_CHARS.start(), NAME_CHARS.end(), name.chars().take(40).collect::<String>()));
        }
        if taken.iter().chain(self.saved.lists.iter().map(|l| &l.name)).any(|t| t.eq_ignore_ascii_case(&name)) {
            return Err(format!("A banlist named \"{name}\" already exists here: choose another name."));
        }
        let (lists, default_limit, unknown) = banlists::to_lists(&Lflist { name: name.clone(), ..first.clone() }, cid);
        let id = self.saved.next_id.max(self.floor).max(self.saved.lists.iter().map(|l| l.id + 1).max().unwrap_or(0));
        let list = PlayerList { id, name, owner, owner_name: owner_name.to_string(), lists, default_limit };
        let mut next = self.saved.clone();
        next.next_id = id + 1;
        next.lists.push(list.clone());
        let before = std::mem::replace(&mut self.saved, next);
        if let Err(e) = self.save() { self.saved = before; return Err(e) }
        Ok((list, unknown))
    }

    /// `owner`'s list named `name` (any case) as an `lflist.conf` text, its cards by passcode with their names.
    pub fn export(&self, owner: u32, name: &str, passcode: impl Fn(u16) -> Option<u32>, card_name: impl Fn(u16) -> Option<String>) -> Result<(String, String), String> {
        let l = self.saved.lists.iter().find(|l| l.owner == owner && l.name.eq_ignore_ascii_case(name.trim())).ok_or_else(|| format!("You keep no banlist named \"{name}\"."))?;
        let mut out = format!("#[{}]\n!{}\n", l.name, l.name);
        if l.default_limit == 0 { out.push_str("$whitelist\n") }
        for (count, cards) in l.lists.iter().enumerate() {
            for &cid in cards {
                let Some(code) = passcode(cid) else { continue };
                out.push_str(&format!("{code} {count} --{}\n", card_name(cid).unwrap_or_default()));
            }
        }
        Ok((l.name.clone(), out))
    }

    /// Deletes `owner`'s list named `name`: its number is never given again.
    pub fn delete(&mut self, owner: u32, name: &str) -> Result<PlayerList, String> {
        let at = self.saved.lists.iter().position(|l| l.owner == owner && l.name.eq_ignore_ascii_case(name.trim())).ok_or_else(|| format!("You keep no banlist named \"{name}\"."))?;
        let mut next = self.saved.clone();
        let gone = next.lists.remove(at);
        next.next_id = next.next_id.max(gone.id + 1);
        let before = std::mem::replace(&mut self.saved, next);
        if let Err(e) = self.save() { self.saved = before; return Err(e) }
        Ok(gone)
    }

    /// `owner`'s lists as the operations answer them.
    pub fn listing(&self, owner: u32) -> serde_json::Value {
        let lists: Vec<_> = self.of(owner).iter().map(|l| json!({ "id": l.id, "name": l.name,
            "cards": l.lists.iter().map(Vec::len).sum::<usize>(), "allowlist": l.default_limit == 0 })).collect();
        json!({ "banlists": lists })
    }
}

impl PlayerList {
    /// As the launcher writes it, with its name's text id.
    pub fn added(&self, name_id: u32) -> AddedBanlist {
        AddedBanlist { id: self.id, name: self.name.clone(), name_id, lists: self.lists.clone(), default_limit: self.default_limit }
    }
}

/// What the operations need besides the store: banlist names already taken (the game's and the host's), the cap an
/// account, and the cards (passcodes both ways, which Konami ids the client has, their names).
pub struct Context<'a> {
    pub taken: Vec<String>,
    pub cap: u32,
    pub passcodes: &'a crate::ydk::Passcodes,
    pub has_card: &'a dyn Fn(u16) -> bool,
    pub card_name: &'a dyn Fn(u16) -> Option<String>,
}

/// One request of account `owner` (shown as `owner_name`): `{op: "banlists"}` lists its lists; `"banlist_import"`
/// `{conf, name?}` keeps the first list of an EDOPro lflist.conf; `"banlist_export"` `{name}` answers it as one
/// (`{name, conf}`); `"banlist_delete"` `{name}`. The answer has a `message` for people.
pub fn operate(store: &mut Store, owner: u32, owner_name: &str, request: &serde_json::Value, cx: &Context) -> Result<serde_json::Value, String> {
    let text = |k: &str| request[k].as_str().unwrap_or_default().to_string();
    match request["op"].as_str().unwrap_or_default() {
        "banlists" => {
            let mut v = store.listing(owner);
            let n = store.of(owner).len();
            v["cap"] = json!(cx.cap);
            v["message"] = json!(format!("{n} of {} banlist(s) of yours.", cx.cap));
            Ok(v)
        }
        "banlist_import" => {
            let cid = |code: u32| cx.passcodes.cid(code).filter(|c| (cx.has_card)(*c));
            let (list, unknown) = store.import(owner, owner_name, &text("name"), &text("conf"), cx.cap, &cx.taken, cid)?;
            tracing::info!(user = owner, banlist = %list.name, id = list.id, "[banlists] a player's banlist imported");
            Ok(json!({ "id": list.id, "name": list.name, "left_out": unknown, "message": imported_sentence(&list, unknown) }))
        }
        "banlist_export" => {
            let (name, conf) = store.export(owner, &text("name"), |c| cx.passcodes.passcode(c), |c| (cx.card_name)(c))?;
            Ok(json!({ "name": name, "conf": conf, "message": format!("Exported \"{name}\".") }))
        }
        "banlist_delete" => {
            let gone = store.delete(owner, &text("name"))?;
            tracing::info!(user = owner, banlist = %gone.name, id = gone.id, "[banlists] a player's banlist deleted");
            Ok(json!({ "name": gone.name, "message": format!("Deleted \"{}\": players who picked it are back on the server's banlist.", gone.name) }))
        }
        _ => Err("bad_request".into()),
    }
}

/// The operations on a server's data folder while it is stopped (Desktop's own server): the store kept there, the
/// game's banlist names and cards from `game_dir`, the host's lists' names from `banlists/`, the cap from the saved
/// settings, the passcodes from the rules engine's folder.
pub fn offline(data_dir: &Path, game_dir: &Path, owner: u32, owner_name: &str, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let (data, _) = crate::game_data(game_dir).map_err(|e| format!("the game's files: {e:#}"))?;
    let engine = crate::engine::Engine::dir().ok_or("the rules engine's folder is missing")?;
    let passcodes = crate::ydk::Passcodes::load(&engine)?;
    let mut taken: Vec<String> = data.regulations.iter().map(|r| data.text(r.name_id).unwrap_or_default().to_string()).collect();
    if let Ok(entries) = std::fs::read_dir(data_dir.join("banlists")) {
        for path in entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("conf"))) {
            taken.extend(std::fs::read_to_string(&path).map(|t| banlists::parse(&t).into_iter().map(|l| l.name).collect::<Vec<_>>()).unwrap_or_default());
        }
    }
    let banlist_names: Vec<(u32, String)> = data.regulations.iter().map(|r| (r.id, data.text(r.name_id).unwrap_or_default().to_string())).collect();
    let cap = crate::settings::Live::open(Some(&data_dir.join("settings.json")), banlist_names).map(|l| l.now().player_banlists).unwrap_or(3);
    let cids: std::collections::HashSet<u16> = data.cards.iter().map(|c| c.cid).collect();
    let names: std::collections::HashMap<u16, String> = data.cards.iter().map(|c| (c.cid, c.name.clone())).collect();
    let mut store = Store::open(Some(data_dir), banlists::numbers_taken(data_dir), None);
    let has_card = |c: u16| cids.contains(&c);
    let card_name = |c: u16| names.get(&c).cloned();
    let cx = Context { taken, cap, passcodes: &passcodes, has_card: &has_card, card_name: &card_name };
    operate(&mut store, owner, owner_name, request, &cx)
}

/// The operations on the running server: its store, banlists, settings and cards.
pub fn live(world: &crate::front::World, owner: u32, owner_name: &str, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let engine = world.engine_dir.clone().or_else(crate::engine::Engine::dir).ok_or("the rules engine's folder is missing")?;
    let passcodes = crate::ydk::Passcodes::load(&engine)?;
    let cids: std::collections::HashSet<u16> = world.cards.values().map(|c| c.cid).collect();
    let has_card = |c: u16| cids.contains(&c);
    let card_name = |c: u16| world.card_names.get(&c).cloned();
    let cx = Context { taken: world.banlists.iter().map(|(_, n)| n.clone()).collect(), cap: world.settings.now().player_banlists,
        passcodes: &passcodes, has_card: &has_card, card_name: &card_name };
    let mut store = locked(&world.player_banlists);
    operate(&mut store, owner, owner_name, request, &cx)
}

/// The sentence for an import.
pub fn imported_sentence(list: &PlayerList, unknown: usize) -> String {
    let cards: usize = list.lists.iter().map(Vec::len).sum();
    let plural = |n: usize| if n == 1 { "card" } else { "cards" };
    let left = if unknown > 0 { format!(" ({unknown} {} the game doesn't have left out)", plural(unknown)) } else { String::new() };
    format!("Imported \"{}\": {cards} {} listed{left}. Pick it in the game's Edit Deck → Change Regulations after its next start.", list.name, plural(cards))
}

/// Shared store's lock, for callers holding the `Shared`.
pub fn locked(shared: &Shared) -> std::sync::MutexGuard<'_, Store> { shared.locked() }

#[cfg(test)]
mod tests {
    use super::*;

    const GOAT: &str = "!2005.4 GOAT\n$whitelist\n55144522 0\n70368879 1\n83764718 3\n";

    #[test]
    fn a_player_imports_exports_and_deletes_their_lists() {
        let mut s = Store::open(None, 25, None);
        let cid = |code: u32| match code { 55144522 => Some(4844u16), 70368879 => Some(5000), 83764718 => Some(4343), _ => None };
        let (goat, unknown) = s.import(1001, "Yugi", "", GOAT, 3, &["Season 16".into()], cid).unwrap();
        assert_eq!((goat.id, goat.name.as_str(), goat.default_limit, unknown), (25, "2005.4 GOAT", 0, 0));
        assert_eq!(goat.lists, [vec![], vec![5000], vec![], vec![4343]]);
        // Names are unique, any case, among every banlist; the cap holds per account.
        assert!(s.import(1002, "Kaiba", "2005.4 goat", GOAT, 3, &[], cid).unwrap_err().contains("already exists"));
        assert!(s.import(1001, "Yugi", "season 16", GOAT, 3, &["Season 16".into()], cid).is_err());
        assert!(s.import(1001, "Yugi", "x", GOAT, 3, &[], cid).unwrap_err().contains("3 to 24"));
        s.import(1001, "Yugi", "Edison", GOAT, 3, &[], cid).unwrap();
        s.import(1001, "Yugi", "Tengu", GOAT, 3, &[], cid).unwrap();
        assert!(s.import(1001, "Yugi", "One more", GOAT, 3, &[], cid).unwrap_err().contains("already keep 3"));
        assert!(s.import(1001, "Yugi", "Off", GOAT, 0, &[], cid).is_err());
        // Export: an lflist.conf EDOPro reads, by passcode.
        let code = |c: u16| match c { 4844 => Some(55144522), 5000 => Some(70368879), 4343 => Some(83764718), _ => None };
        let (_, text) = s.export(1001, "2005.4 goat", code, |_| Some("X".into())).unwrap();
        assert_eq!(text, "#[2005.4 GOAT]\n!2005.4 GOAT\n$whitelist\n70368879 1 --X\n83764718 3 --X\n");
        assert_eq!(banlists::to_lists(&banlists::parse(&text)[0], cid).0, goat.lists, "round trip");
        assert!(s.export(1002, "Edison", code, |_| None).is_err(), "only the owner's");
        // Delete: the number is never given again.
        assert!(s.delete(1002, "Edison").is_err());
        s.delete(1001, "Tengu").unwrap();
        let (next, _) = s.import(1001, "Yugi", "Perfect Circle", GOAT, 3, &[], cid).unwrap();
        assert_eq!(next.id, 28);
        assert_eq!(s.regulation(25).map(|r| r.limit(5000)), Some(1));
        assert!(s.regulation(27).is_none());
    }
}
