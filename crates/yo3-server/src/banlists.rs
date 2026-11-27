//! The host's own banlists (#77, phase 1): EDOPro's `lflist.conf` files in the data folder's `banlists/`, read at the
//! server's start. Each list (`!Name` and its `passcode count` lines; `$whitelist` for a format whose unlisted cards are
//! allowed none) becomes a banlist the game knows, numbered from 21 (after the game's 20) for good: `banlists/ids.json`
//! keeps each name's number, so a number is never given to another list. The server checks decks against it like the
//! game's own (`/banlist`, Create Duel, events, `login-regulation`), and the launcher writes it as the game's
//! `data/regulation/<id>.y3r` with its name (`yo3_data::added::AddedBanlist`), which the connection DLL serves.
use std::collections::BTreeMap;
use std::path::Path;

use yo3_data::added::{AddedBanlist, BANLIST_NAME_TEXT, MAX_BANLISTS};

/// The first number a host's banlist gets: the game ships 1 to 20.
pub const FIRST_ID: u32 = 21;

/// One list of an `lflist.conf`: its name, whether unlisted cards are allowed none (`$whitelist`), and its cards by
/// passcode with their copies (0 to 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lflist {
    pub name: String,
    pub whitelist: bool,
    pub cards: Vec<(u32, u32)>,
}

/// The lists of an EDOPro `lflist.conf` text: `#` comments, `!Name` starts a list, `$whitelist` marks it, and each
/// `passcode count` line (anything after is a comment, `--Name`) adds a card. A count above 3 counts as 3, a negative
/// one as 0; lines that are none of these are left out.
pub fn parse(text: &str) -> Vec<Lflist> {
    let mut lists: Vec<Lflist> = vec![];
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('!') {
            lists.push(Lflist { name: name.trim().to_string(), whitelist: false, cards: vec![] });
            continue;
        }
        let Some(list) = lists.last_mut() else { continue };
        if line.eq_ignore_ascii_case("$whitelist") {
            list.whitelist = true;
            continue;
        }
        let mut words = line.split_whitespace();
        let (Some(Ok(code)), Some(Ok(count))) = (words.next().map(str::parse::<u32>), words.next().map(str::parse::<i64>)) else { continue };
        list.cards.push((code, count.clamp(0, 3) as u32));
    }
    lists.retain(|l| !l.name.is_empty());
    lists
}

/// A list as the game's banlists hold cards: Forbidden, Limited, Semi-Limited and three copies by Konami id, and the
/// copies of a card in none. Passcodes with no card of the client are counted, not kept.
pub fn to_lists(list: &Lflist, cid: impl Fn(u32) -> Option<u16>) -> ([Vec<u16>; 4], u32, usize) {
    let mut lists: [Vec<u16>; 4] = Default::default();
    let mut unknown = 0;
    let default = if list.whitelist { 0 } else { 3 };
    for &(code, count) in &list.cards {
        let Some(cid) = cid(code) else { unknown += 1; continue };
        // In an allowlist a card at 0 is the default; a blocklist's 3 likewise.
        if count == default {
            continue;
        }
        let slot = &mut lists[count as usize];
        if !slot.contains(&cid) {
            slot.push(cid);
        }
    }
    (lists, default, unknown)
}

/// The host's banlists in `dir/banlists/*.conf`, numbered by `dir/banlists/ids.json` (new names get the next number,
/// saved there), at most [`MAX_BANLISTS`], none named like a game banlist (`taken`). A problem with one file is logged
/// and that file left out.
pub fn load(dir: &Path, taken: &[String], cid: impl Fn(u32) -> Option<u16>) -> Vec<AddedBanlist> {
    let folder = dir.join("banlists");
    let mut files: Vec<_> = match std::fs::read_dir(&folder) {
        Ok(entries) => entries.filter_map(Result::ok).map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("conf"))).collect(),
        Err(_) => return vec![],
    };
    files.sort();
    let ids_path = folder.join("ids.json");
    let mut ids: BTreeMap<String, u32> = std::fs::read(&ids_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let before = ids.clone();
    let mut out: Vec<AddedBanlist> = vec![];
    for file in files {
        let text = match std::fs::read(&file) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => { tracing::warn!("banlist {}: {e}", file.display()); continue }
        };
        for list in parse(&text) {
            let name: String = list.name.chars().take(40).collect();
            if taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) || out.iter().any(|b| b.name.eq_ignore_ascii_case(&name)) {
                tracing::warn!("banlist {:?} in {}: the name is taken; left out", name, file.display());
                continue;
            }
            if out.len() == MAX_BANLISTS {
                tracing::warn!("banlist {:?} in {}: the server serves {MAX_BANLISTS} at most; left out", name, file.display());
                continue;
            }
            let next = ids.values().copied().max().map_or(FIRST_ID, |m| m + 1).max(FIRST_ID);
            let id = *ids.entry(name.clone()).or_insert(next);
            let (lists, default_limit, unknown) = to_lists(&list, &cid);
            if unknown > 0 {
                tracing::info!("banlist {name:?}: {unknown} card(s) the game doesn't have left out");
            }
            let name_id = BANLIST_NAME_TEXT + out.len() as u32;
            out.push(AddedBanlist { id, name, name_id, lists, default_limit });
        }
    }
    if ids != before && let Err(e) = std::fs::write(&ids_path, serde_json::to_vec_pretty(&ids).unwrap_or_default()) {
        tracing::warn!("{}: {e}", ids_path.display());
    }
    if !out.is_empty() {
        tracing::info!("{} banlist(s) of the host's: {}", out.len(), out.iter().map(|b| format!("{} ({})", b.name, b.id)).collect::<Vec<_>>().join(", "));
    }
    out
}

/// A host's banlist as the server checks decks against it, with the game's record shape (`template`: one of the game's).
pub fn regulation(b: &AddedBanlist, template: &yo3_proto::Message) -> yo3_data::Regulation {
    yo3_data::Regulation { id: b.id, kind: 0, status: 0, name_id: b.name_id, lists: b.lists.clone(), default_limit: b.default_limit, record: b.record(template) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOAT: &str = "#[2005.4 GOAT]\n!2005.4 GOAT\n$whitelist\n55144522 0 --Pot of Greed\n70368879 1 --Upstart Goblin\n\
        83764718 3 --Monster Reborn\n12345 3\n!Edison\n#comment\n55144522 0 --Pot of Greed\n83764718 1\n44095762 2 extra words\n\
        83764718 1\nnot a line\n";

    #[test]
    fn an_edopro_lflist_reads_as_lists() {
        let lists = parse(GOAT);
        assert_eq!(lists.len(), 2);
        assert_eq!((lists[0].name.as_str(), lists[0].whitelist, lists[0].cards.len()), ("2005.4 GOAT", true, 4));
        assert_eq!(lists[1].cards, [(55144522, 0), (83764718, 1), (44095762, 2), (83764718, 1)]);
    }

    #[test]
    fn a_list_holds_the_games_ids_by_limit() {
        let ids = |code: u32| match code { 55144522 => Some(4844u16), 70368879 => Some(5000), 83764718 => Some(4343), 44095762 => Some(4887), _ => None };
        let lists = parse(GOAT);
        // An allowlist: unlisted cards none, its 0s left out (the default), 3s in the three-copies list.
        let (goat, default, unknown) = to_lists(&lists[0], ids);
        assert_eq!((default, unknown), (0, 1));
        assert_eq!(goat, [vec![], vec![5000], vec![], vec![4343]]);
        let rule = yo3_data::Regulation { id: 21, kind: 0, status: 0, name_id: 0, lists: goat, default_limit: 0, record: Default::default() };
        assert_eq!((rule.limit(4343), rule.limit(5000), rule.limit(4844), rule.limit(9999)), (3, 1, 0, 0));
        // A blocklist: once each.
        let (edison, default, _) = to_lists(&lists[1], ids);
        assert_eq!((edison, default), ([vec![4844], vec![4343], vec![4887], vec![]], 3));
    }

    #[test]
    fn the_folders_lists_keep_their_numbers_for_good() {
        let dir = std::env::temp_dir().join(format!("yo3-banlists-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("banlists")).unwrap();
        std::fs::write(dir.join("banlists/a.conf"), GOAT).unwrap();
        let cid = |_| Some(1u16);
        let first = load(&dir, &["Season 16".into()], cid);
        assert_eq!(first.iter().map(|b| (b.id, b.name.as_str(), b.name_id)).collect::<Vec<_>>(),
            [(21, "2005.4 GOAT", BANLIST_NAME_TEXT), (22, "Edison", BANLIST_NAME_TEXT + 1)]);
        // Goat removed, a new list added: Edison keeps 22, the new one gets 23 (21 is never given again).
        std::fs::write(dir.join("banlists/a.conf"), "!Edison\n!Tengu Plant\n!season 16\n").unwrap();
        let second = load(&dir, &["Season 16".into()], cid);
        assert_eq!(second.iter().map(|b| (b.id, b.name.as_str())).collect::<Vec<_>>(), [(22, "Edison"), (23, "Tengu Plant")]);
        assert!(load(&dir.join("none"), &[], cid).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
