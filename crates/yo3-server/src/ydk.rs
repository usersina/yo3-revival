//! EDOPro decks: the `.ydk` file (`#main`, `#extra`, `!side`, a passcode per line) and the `ydke://` link (each section
//! base64 of little-endian 32-bit passcodes, ended by `!`), turned into the decks the game keeps (CARD_Prop rows) and back.
//!
//! A passcode names a card through the rules engine's data, as the engine itself plays it: YGOPRODeck's card info
//! (`engine/vendor/cardinfo.json`: every artwork's passcode and the card's Konami id), the cards the game never had
//! (`engine/added-cards.json`: their passcode under the id the client knows, a stand-in for the newer ones, #75) and the
//! anime versions (`engine/anime-cards.json`: an anime passcode names the game's card it is the anime version of, #60).
use base64::Engine as _;
use serde_json::Value as Json;
use std::collections::HashMap;
use std::path::Path;

/// A deck as EDOPro writes it: passcodes by section, a copy per entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ydk {
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
    pub side: Vec<u32>,
}

impl Ydk {
    /// A `.ydk` file's text or a `ydke://` link. Lines that are no passcode (`#created by ...`, blank) are skipped; a
    /// passcode before any section counts as the main deck's, as EDOPro reads it.
    pub fn parse(text: &str) -> Result<Ydk, String> {
        let text = text.trim_start_matches('\u{feff}').trim();
        if let Some(link) = text.strip_prefix("ydke://") {
            return Ydk::parse_link(link);
        }
        let mut deck = Ydk::default();
        let mut section = 0;
        let mut any = false;
        for line in text.lines().map(str::trim) {
            match line {
                "#main" => section = 0,
                "#extra" => section = 1,
                "!side" => section = 2,
                _ => {
                    let Ok(code) = line.parse::<u32>() else { continue };
                    any = true;
                    [&mut deck.main, &mut deck.extra, &mut deck.side][section].push(code);
                }
            }
        }
        if !any {
            return Err("no cards: a .ydk file lists a passcode per line under #main, #extra and !side".into());
        }
        Ok(deck)
    }

    fn parse_link(link: &str) -> Result<Ydk, String> {
        let parts: Vec<&str> = link.split('!').collect();
        if parts.len() < 3 {
            return Err("a ydke:// link has three sections, each ended by !".into());
        }
        let section = |b64: &str| -> Result<Vec<u32>, String> {
            let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| format!("ydke:// link: {e}"))?;
            if bytes.len() % 4 != 0 {
                return Err("ydke:// link: a section is not whole passcodes".into());
            }
            Ok(bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect())
        };
        Ok(Ydk { main: section(parts[0])?, extra: section(parts[1])?, side: section(parts[2])? })
    }

    /// The `.ydk` file EDOPro reads: its sections in order, a passcode per copy.
    pub fn to_file(&self) -> String {
        let mut out = String::from("#created by yo3-revival\n#main\n");
        let lines = |out: &mut String, codes: &[u32]| codes.iter().for_each(|c| { out.push_str(&c.to_string()); out.push('\n') });
        lines(&mut out, &self.main);
        out.push_str("#extra\n");
        lines(&mut out, &self.extra);
        out.push_str("!side\n");
        lines(&mut out, &self.side);
        out
    }

    /// The `ydke://` link EDOPro's "Copy deck" makes and pastes.
    pub fn to_link(&self) -> String {
        let section = |codes: &[u32]| base64::engine::general_purpose::STANDARD.encode(codes.iter().flat_map(|c| c.to_le_bytes()).collect::<Vec<u8>>());
        format!("ydke://{}!{}!{}!", section(&self.main), section(&self.extra), section(&self.side))
    }
}

/// Passcodes and the Konami ids the client knows, both ways.
#[derive(Clone, Debug, Default)]
pub struct Passcodes {
    cid_of: HashMap<u32, u16>,
    passcode_of: HashMap<u16, u32>,
}

impl Passcodes {
    /// From the engine folder (`vendor/cardinfo.json`, `added-cards.json`, `anime-cards.json`).
    pub fn load(engine_dir: &Path) -> Result<Passcodes, String> {
        let read = |name: &str| -> Result<Option<Json>, String> {
            match std::fs::read(engine_dir.join(name)) {
                Ok(text) => serde_json::from_slice(&text).map(Some).map_err(|e| format!("{name}: {e}")),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(format!("{name}: {e}")),
            }
        };
        let info = read("vendor/cardinfo.json")?.ok_or("vendor/cardinfo.json: missing (engine/fetch.sh)")?;
        let mut map = Passcodes::default();
        for card in info["data"].as_array().into_iter().flatten() {
            let Some(main) = card["id"].as_u64().map(|i| i as u32) else { continue };
            let arts: Vec<u32> = card["card_images"].as_array().into_iter().flatten().filter_map(|i| i["id"].as_u64()).map(|i| i as u32).collect();
            for misc in card["misc_info"].as_array().into_iter().flatten() {
                let Some(cid) = misc["konami_id"].as_u64().and_then(|c| u16::try_from(c).ok()).filter(|&c| c > 0) else { continue };
                map.passcode_of.entry(cid).or_insert(main);
                for code in std::iter::once(main).chain(arts.iter().copied()) {
                    map.cid_of.entry(code).or_insert(cid);
                }
            }
        }
        // An added card's passcode names it by the id the client knows (a stand-in for a newer card).
        if let Some(added) = read("added-cards.json")? {
            for c in added["cards"].as_array().into_iter().flatten() {
                if let (Some(cid), Some(code)) = (c["cid"].as_u64().and_then(|c| u16::try_from(c).ok()), c["passcode"].as_u64().map(|c| c as u32)) {
                    map.cid_of.insert(code, cid);
                    map.passcode_of.insert(cid, code);
                }
            }
        }
        // An anime version's passcode names the game's card; a deck written back keeps the printed passcode.
        if let Some(anime) = read("anime-cards.json")? {
            for c in anime["cards"].as_array().into_iter().flatten() {
                if let (Some(cid), Some(code)) = (c["cid"].as_u64().and_then(|c| u16::try_from(c).ok()), c["anime"].as_u64().map(|c| c as u32)) {
                    map.cid_of.entry(code).or_insert(cid);
                }
            }
        }
        Ok(map)
    }

    /// The Konami id (or stand-in) a passcode names.
    pub fn cid(&self, passcode: u32) -> Option<u16> {
        self.cid_of.get(&passcode).copied()
    }

    /// The passcode EDOPro knows a card by.
    pub fn passcode(&self, cid: u16) -> Option<u32> {
        self.passcode_of.get(&cid).copied()
    }

    #[cfg(test)]
    fn with(pairs: &[(u32, u16)]) -> Passcodes {
        let mut map = Passcodes::default();
        for &(code, cid) in pairs {
            map.cid_of.insert(code, cid);
            map.passcode_of.entry(cid).or_insert(code);
        }
        map
    }
}

/// An imported deck as the game's rows, with what could not come.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub main: Vec<u16>,
    pub extra: Vec<u16>,
    pub side: Vec<u16>,
    /// Passcodes no card of the game (or the server's added cards) has, once each with how many copies.
    pub unknown: Vec<(u32, usize)>,
}

/// The rows of `ydk`: each passcode's card by `passcodes`, then its row by `row_of` (the artwork the player owns first;
/// `None` for a card the client has no row for, which counts as unknown).
pub fn import(ydk: &Ydk, passcodes: &Passcodes, row_of: impl Fn(u16) -> Option<u16>) -> Imported {
    let mut out = Imported::default();
    let mut unknown: Vec<(u32, usize)> = vec![];
    let mut put = |codes: &[u32], rows: &mut Vec<u16>| {
        for &code in codes {
            match passcodes.cid(code).and_then(&row_of) {
                Some(row) => rows.push(row),
                None => match unknown.iter_mut().find(|(c, _)| *c == code) {
                    Some((_, n)) => *n += 1,
                    None => unknown.push((code, 1)),
                },
            }
        }
    };
    put(&ydk.main, &mut out.main);
    put(&ydk.extra, &mut out.extra);
    put(&ydk.side, &mut out.side);
    out.unknown = unknown;
    out
}

/// A deck the game keeps as EDOPro's: each row's card (`cid_of`) by its passcode. Rows with no passcode (a card EDOPro
/// lacks) are left out and returned.
pub fn export(main: &[u16], extra: &[u16], side: &[u16], passcodes: &Passcodes, cid_of: impl Fn(u16) -> Option<u16>) -> (Ydk, Vec<u16>) {
    let mut missing = vec![];
    let mut codes = |rows: &[u16]| -> Vec<u32> {
        rows.iter().filter_map(|&row| {
            let code = cid_of(row).and_then(|cid| passcodes.passcode(cid));
            if code.is_none() {
                missing.push(row);
            }
            code
        }).collect()
    };
    let ydk = Ydk { main: codes(main), extra: codes(extra), side: codes(side) };
    (ydk, missing)
}

/// The decks the server keeps for a player (as `keep_deck`: Load Deck reads at most ten).
pub const MAX_DECKS: usize = 10;
/// The most rows of a list the client reads (`0x1F4F`, `0x414410`).
const MAX_ROWS: usize = 100;

/// What an import needs to know of the server: every card (row, Konami id, name; `gifts::Gifts::cards`), the rows a
/// player starts with, the starter deck's name when the player still has it, and the passcodes.
pub struct Cards<'a> {
    pub cards: &'a [crate::gifts::Card],
    pub starting: &'a [u16],
    pub passcodes: &'a Passcodes,
}

/// A deck kept from an EDOPro deck, and what didn't come or isn't held.
#[derive(Clone, Debug)]
pub struct Kept {
    pub deck: crate::front::Deck,
    /// Passcodes no card here has, with how many copies.
    pub unknown: Vec<(u32, usize)>,
    /// Cards beyond what the player holds, by name, with how many copies.
    pub unowned: Vec<(String, i64)>,
}

impl Kept {
    pub fn json(&self, player: &str) -> serde_json::Value {
        let d = &self.deck;
        let mut message = format!("Imported \"{}\" for {player}: {} + {} + {} cards. It is in Load Deck the next time the game lists the decks.",
            d.name, d.main.len(), d.extra.len(), d.side.len());
        if !self.unknown.is_empty() {
            let codes: Vec<String> = self.unknown.iter().map(|(c, n)| if *n > 1 { format!("{c} ×{n}") } else { c.to_string() }).collect();
            message.push_str(&format!(" Left out, no card of this server: {}.", codes.join(", ")));
        }
        if !self.unowned.is_empty() {
            let names: Vec<String> = self.unowned.iter().map(|(n, k)| if *k > 1 { format!("{n} ×{k}") } else { n.clone() }).collect();
            message.push_str(&format!(" Not held: {}.", names.join(", ")));
        }
        serde_json::json!({ "player": player, "deck": d.name, "main": d.main.len(), "extra": d.extra.len(), "side": d.side.len(),
            "unknown": self.unknown.iter().map(|(c, n)| serde_json::json!({ "passcode": c, "copies": n })).collect::<Vec<_>>(),
            "unowned": self.unowned.iter().map(|(n, k)| serde_json::json!({ "name": n, "copies": k })).collect::<Vec<_>>(), "message": message })
    }
}

/// The player's decks as Load Deck lists them: the starter deck (`starter`, unless the player deleted it), then the
/// uploaded ones.
pub fn decks_of(p: &crate::front::Player, starter: Option<&crate::front::Deck>) -> Vec<crate::front::Deck> {
    starter.filter(|_| !p.starter_deleted).cloned().into_iter().chain(p.uploaded.iter().cloned()).collect()
}

/// `decks`: each deck's name and its main, Extra and side deck sizes.
pub fn list(player: &str, decks: &[crate::front::Deck]) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = decks.iter().map(|k| serde_json::json!({ "name": k.name, "main": k.main.len(), "extra": k.extra.len(), "side": k.side.len() })).collect();
    let lines: Vec<String> = decks.iter().map(|k| format!("{}\t{} + {} + {}", k.name, k.main.len(), k.extra.len(), k.side.len())).collect();
    serde_json::json!({ "player": player, "decks": rows, "message": if lines.is_empty() { format!("{player} has no decks.") } else { lines.join("\n") } })
}

/// The `.ydk` file and `ydke://` link of the deck named `name`; cards EDOPro has no passcode for are left out and named.
pub fn export_named(player: &str, decks: &[crate::front::Deck], name: &str, cards: &Cards) -> Result<serde_json::Value, String> {
    let k = decks.iter().find(|k| k.name == name).ok_or_else(|| format!("{player} has no deck named \"{name}\"."))?;
    let cid_of: HashMap<u16, u16> = cards.cards.iter().map(|c| (c.row, c.cid)).collect();
    let (out, left) = export(&k.main, &k.extra, &k.side, cards.passcodes, |row| cid_of.get(&row).copied());
    let names: Vec<String> = left.iter().map(|row| cards.cards.iter().find(|c| c.row == *row).map_or(format!("row {row}"), |c| c.name.clone())).collect();
    let message = if names.is_empty() { format!("Exported \"{}\".", k.name) } else { format!("Exported \"{}\" without {} (no EDOPro passcode).", k.name, names.join(", ")) };
    Ok(serde_json::json!({ "player": player, "deck": k.name, "ydk": out.to_file(), "link": out.to_link(), "left_out": names, "message": message }))
}

/// The `.ydk` file or `ydke://` link `text` as a new deck of the player `p` (whose decks are `decks`), named `name`
/// (a free one: "Deck (2)" when taken). Passcodes with no card here are left out; cards the player doesn't hold are kept
/// and named (Edit Deck shows them, a duel's deck check refuses them).
pub fn import_for(p: &crate::front::Player, decks: &[crate::front::Deck], name: &str, text: &str, cards: &Cards) -> Result<Kept, String> {
    let parsed = Ydk::parse(text)?;
    // How many of each row the player holds, starting cards included (as `collection`).
    let mut have: HashMap<u16, i64> = HashMap::new();
    for &row in cards.starting { *have.entry(row).or_default() += 1 }
    for (&row, &n) in &p.cards { *have.entry(row).or_default() += i64::from(n) }
    for (&row, &n) in &p.given { *have.entry(row).or_default() -= i64::from(n) }
    // A card's artwork the player holds, else its first.
    let mut rows_of: HashMap<u16, Vec<u16>> = HashMap::new();
    for c in cards.cards { rows_of.entry(c.cid).or_default().push(c.row) }
    let row_of = |cid: u16| -> Option<u16> {
        let rows = rows_of.get(&cid)?;
        rows.iter().copied().find(|r| have.get(r).is_some_and(|&n| n > 0)).or(rows.first().copied())
    };
    let got = import(&parsed, cards.passcodes, row_of);
    if got.main.is_empty() && got.extra.is_empty() {
        return Err("None of the deck's cards are cards of this server.".into());
    }
    if [&got.main, &got.extra, &got.side].iter().any(|l| l.len() > MAX_ROWS) {
        return Err(format!("The game reads at most {MAX_ROWS} cards per list."));
    }
    if p.uploaded.len() >= MAX_DECKS {
        return Err(format!("{} already has {MAX_DECKS} decks: delete one in Edit Deck first.", p.name));
    }
    let base = { let n = name.trim(); if n.is_empty() { "Imported deck" } else { n } };
    let free = (1..).map(|i| if i == 1 { base.to_string() } else { format!("{base} ({i})") })
        .map(|n| crate::front::clip(&n, crate::front::MAX_DECK_NAME)).find(|n| !decks.iter().any(|k| &k.name == n)).unwrap_or_default();
    let mut wanted: HashMap<u16, i64> = HashMap::new();
    for &row in got.main.iter().chain(&got.extra).chain(&got.side) { *wanted.entry(row).or_default() += 1 }
    let mut unowned: Vec<(String, i64)> = wanted.iter().filter_map(|(row, &n)| {
        let short = n - have.get(row).copied().unwrap_or(0).max(0);
        (short > 0).then(|| (cards.cards.iter().find(|c| c.row == *row).map_or(format!("row {row}"), |c| c.name.clone()), short))
    }).collect();
    unowned.sort();
    let id = p.uploaded.iter().map(|k| k.id).max().unwrap_or(1).max(1000) + 1;
    let deck = crate::front::Deck { id, name: free, main: got.main, extra: got.extra, side: got.side };
    Ok(Kept { deck, unknown: got.unknown, unowned })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "#created by Player\n#main\n89631139\n89631139\n46986414\n#extra\n44508094\n!side\n5318639\n";

    #[test]
    fn a_ydk_file_reads_by_section_and_writes_back() {
        let deck = Ydk::parse(FILE).unwrap();
        assert_eq!(deck, Ydk { main: vec![89631139, 89631139, 46986414], extra: vec![44508094], side: vec![5318639] });
        assert_eq!(Ydk::parse(&deck.to_file()).unwrap(), deck);
        assert!(Ydk::parse("#main\n#extra\n!side\n").is_err());
        // Windows line ends and a byte-order mark, as Notepad saves it.
        assert_eq!(Ydk::parse(&format!("\u{feff}{}", FILE.replace('\n', "\r\n"))).unwrap(), deck);
    }

    #[test]
    fn a_ydke_link_reads_and_writes_back() {
        let deck = Ydk::parse(FILE).unwrap();
        let link = deck.to_link();
        assert!(link.starts_with("ydke://") && link.ends_with('!'));
        assert_eq!(Ydk::parse(&link).unwrap(), deck);
        assert!(Ydk::parse("ydke://abc").is_err());
    }

    #[test]
    fn import_maps_known_passcodes_and_counts_the_unknown_ones() {
        let map = Passcodes::with(&[(89631139, 4007), (46986414, 4041), (44508094, 7734)]);
        let row = |cid: u16| (cid != 7734).then_some(cid + 1);
        let got = import(&Ydk::parse(FILE).unwrap(), &map, row);
        assert_eq!((got.main, got.extra, got.side), (vec![4008, 4008, 4042], vec![], vec![]));
        // Stardust Dragon has no row here, Ash Blossom no card: both unknown.
        assert_eq!(got.unknown, vec![(44508094, 1), (5318639, 1)]);
    }

    #[test]
    fn export_writes_each_rows_passcode() {
        let map = Passcodes::with(&[(89631139, 4007), (46986414, 4041)]);
        let (ydk, missing) = export(&[4008, 4042, 9], &[], &[4008], &map, |row| (row != 9).then(|| row - 1));
        assert_eq!(ydk, Ydk { main: vec![89631139, 46986414], extra: vec![], side: vec![89631139] });
        assert_eq!(missing, vec![9]);
    }
}
