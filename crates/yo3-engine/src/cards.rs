//! The card database (`vendor/BabelCDB/cards.cdb`), the card scripts and the Konami ids (`vendor/cardinfo.json`):
//! which passcode a Konami id plays as in a duel, and back.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::Value as Json;

use crate::core::Card;

/// The folders a card's script (`c<passcode>.lua`) may be in, in order: a few cards' current scripts sit outside
/// `official/` (Red-Eyes Darkness Metal Dragon's in `pre-errata/`). Any other script is at the top.
const CARD_SCRIPT_FOLDERS: [&str; 3] = ["official", "pre-errata", "pre-release"];

/// A card for card-name filters (`declarable`).
pub struct Declarable {
    pub code: u32,
    pub alias: u32,
    pub setcodes: Vec<u16>,
    pub kind: u64,
    pub race: u64,
    pub attribute: u64,
}

pub struct Cards {
    pub vendor: PathBuf,
    db: Connection,
    /// Konami id -> passcode: the database's original among the card's artworks.
    passcode_of: HashMap<u32, u32>,
    /// Passcode (any artwork the database holds) -> Konami id.
    pub cid_of: HashMap<u32, u32>,
    declarables: Option<Vec<Declarable>>,
}

/// A database row's setcode as the 16-bit codes the core reads: low to high, while what is left is nonzero.
fn setcodes(setcode: u64) -> Vec<u16> {
    let mut out = vec![];
    let mut s = setcode;
    while s > 0 {
        out.push((s & 0xffff) as u16);
        s >>= 16;
    }
    out
}

impl Cards {
    pub fn open(vendor: &Path) -> Result<Cards, String> {
        let path = vendor.join("BabelCDB/cards.cdb");
        std::fs::metadata(&path).map_err(|e| format!("open '{}': {e}", path.display()))?;
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|e| format!("open '{}': {e}", path.display()))?;
        let mut alias: HashMap<u32, u32> = HashMap::new();
        {
            let mut q = db.prepare("select id, alias from datas").map_err(|e| e.to_string())?;
            let rows = q.query_map([], |r| Ok((r.get::<_, i64>(0)? as u32, r.get::<_, i64>(1)? as u32))).map_err(|e| e.to_string())?;
            for row in rows.flatten() {
                alias.insert(row.0, row.1);
            }
        }
        let info = vendor.join("cardinfo.json");
        let text = std::fs::read(&info).map_err(|e| format!("open '{}': {e}", info.display()))?;
        let json: Json = serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", info.display()))?;
        let mut passcode_of = HashMap::new();
        let mut cid_of = HashMap::new();
        for card in json["data"].as_array().into_iter().flatten() {
            let mut ids: Vec<u32> = vec![];
            for id in std::iter::once(&card["id"]).chain(card["card_images"].as_array().into_iter().flatten().map(|i| &i["id"])) {
                if let Some(id) = id.as_u64().map(|i| i as u32)
                    && !ids.contains(&id)
                {
                    ids.push(id);
                }
            }
            let known: Vec<u32> = ids.into_iter().filter(|id| alias.contains_key(id)).collect();
            let original = known.iter().copied().find(|id| alias[id] == 0).or(known.first().copied());
            let Some(original) = original else { continue };
            for misc in card["misc_info"].as_array().into_iter().flatten() {
                let cid = misc["konami_id"].as_u64().unwrap_or(0) as u32;
                if cid == 0 || passcode_of.contains_key(&cid) {
                    continue;
                }
                passcode_of.insert(cid, original);
                for id in &known {
                    cid_of.entry(*id).or_insert(cid);
                }
            }
        }
        Ok(Cards { vendor: vendor.to_path_buf(), db, passcode_of, cid_of, declarables: None })
    }

    pub fn passcode(&self, cid: u32) -> Option<u32> {
        self.passcode_of.get(&cid).copied()
    }

    pub fn alias(&self, code: u32) -> Option<u32> {
        self.db.query_row("select alias from datas where id = ?", params![code], |r| r.get::<_, i64>(0)).optional().ok().flatten().map(|a| a as u32)
    }

    /// The original (alias 0) whose English name is `name`, ignoring ASCII case.
    pub fn by_name(&self, name: &str) -> Option<u32> {
        self.db
            .query_row(
                "select d.id from datas d join texts t on t.id = d.id where lower(t.name) = lower(?) and d.alias = 0 order by d.id",
                params![name],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|c| c as u32)
    }

    /// The artworks of `original` the database holds (passcodes within 20 of it).
    pub fn artworks(&self, original: u32) -> Vec<u32> {
        let Ok(mut q) = self.db.prepare("select id from datas where alias = ? and id > ? - 20 and id < ? + 20 order by id") else { return vec![] };
        q.query_map(params![original, original, original], |r| r.get::<_, i64>(0)).map(|rows| rows.flatten().map(|c| c as u32).collect()).unwrap_or_default()
    }

    pub fn name(&self, code: u32) -> Option<String> {
        self.db.query_row("select name from texts where id = ?", params![code], |r| r.get::<_, Option<String>>(0)).optional().ok().flatten().flatten()
    }

    /// The card's data row: type, level column, atk, def, attribute, race, alias, setcode.
    fn row(&self, code: u32) -> Option<[i64; 8]> {
        self.db
            .query_row("select type, level, atk, def, attribute, race, alias, setcode from datas where id = ?", params![code], |r| {
                // A NULL column reads as 0, as the sidecar's Number(null) did.
                let mut row = [0i64; 8];
                for (i, v) in row.iter_mut().enumerate() {
                    *v = r.get::<_, Option<i64>>(i)?.unwrap_or(0);
                }
                Ok(row)
            })
            .optional()
            .ok()
            .flatten()
    }

    /// The core's card data for `code`.
    pub fn card(&self, code: u32) -> Option<Card> {
        let [kind, level, atk, def, attribute, race, alias, setcode] = self.row(code)?;
        Some(Card {
            code,
            alias: alias as u32,
            setcodes: setcodes(setcode as u64),
            kind: kind as u32,
            level: (level & 0xff) as u32,
            attribute: attribute as u32,
            race: race as u64,
            attack: atk as i32,
            defense: def as i32,
            lscale: ((level >> 24) & 0xff) as u32,
            rscale: ((level >> 16) & 0xff) as u32,
        })
    }

    /// The card check's view of a card's data: (type, level, attack, defense, attribute, race, alias).
    pub fn data(&self, code: u32) -> Option<[i64; 7]> {
        let [kind, level, atk, def, attribute, race, alias, _] = self.row(code)?;
        Some([kind, level & 0xff, atk, def, attribute, race, alias])
    }

    /// A script's text by file name.
    pub fn script(&self, name: &str) -> Option<Vec<u8>> {
        let card = name.strip_prefix('c').and_then(|n| n.strip_suffix(".lua")).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        let folders: &[&str] = if card { &CARD_SCRIPT_FOLDERS } else { &[""] };
        folders.iter().find_map(|f| std::fs::read(self.vendor.join("CardScripts").join(f).join(name)).ok())
    }

    /// Every original card with a Konami id, for card-name filters.
    pub fn declarables(&mut self) -> &[Declarable] {
        if self.declarables.is_none() {
            let mut out = vec![];
            if let Ok(mut q) = self.db.prepare("select id, alias, setcode, type, race, attribute from datas where alias = 0") {
                let rows = q.query_map([], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?))
                });
                for (id, alias, setcode, kind, race, attribute) in rows.into_iter().flatten().flatten() {
                    if self.cid_of.contains_key(&(id as u32)) {
                        out.push(Declarable { code: id as u32, alias: alias as u32, setcodes: setcodes(setcode as u64), kind: kind as u64, race: race as u64, attribute: attribute as u64 });
                    }
                }
            }
            self.declarables = Some(out);
        }
        self.declarables.as_deref().unwrap_or_default()
    }
}

/// Which passcode each Konami id plays as in one duel, and back.
#[derive(Default, Clone)]
pub struct Prints {
    pub passcodes: HashMap<u32, u32>,
    pub cids: HashMap<u32, u32>,
}

impl Prints {
    /// The passcode for Konami id `cid` (its client name `name` for another print), claiming it for this duel. A print
    /// the Konami id table doesn't know (the client's second Dark Magician, 5165) is found by its English name and
    /// given an artwork passcode of its own, so the messages still tell the prints apart.
    pub fn resolve(&mut self, cards: &Cards, cid: u32, name: Option<&str>) -> Option<u32> {
        if let Some(known) = self.passcodes.get(&cid).copied().or_else(|| cards.passcode(cid)) {
            self.passcodes.insert(cid, known);
            self.cids.entry(known).or_insert(cid);
            return Some(known);
        }
        let original = cards.by_name(name?)?;
        // An artwork no Konami id claims, so the result doesn't depend on which print the deck lists first.
        let taken = |code: u32| self.cids.get(&code).is_some_and(|c| *c != cid);
        let artworks = cards.artworks(original);
        let code = artworks
            .iter()
            .copied()
            .find(|c| !cards.cid_of.contains_key(c) && !taken(*c))
            .or_else(|| artworks.iter().copied().chain([original]).find(|c| !taken(*c)))
            .unwrap_or(original);
        self.passcodes.insert(cid, code);
        self.cids.insert(code, cid);
        Some(code)
    }

    /// A passcode's Konami id: this duel's print first.
    pub fn cid(&self, cards: &Cards, code: u32) -> Option<u32> {
        self.cids.get(&code).or_else(|| cards.cid_of.get(&code)).copied()
    }

    /// The passcode the core wants for a declared card name (Konami id `cid`): the original's, not an artwork's.
    pub fn declared(&self, cards: &Cards, cid: u32) -> u32 {
        let code = cards.passcode(cid).or_else(|| self.passcodes.get(&cid).copied()).unwrap_or(0);
        cards.alias(code).filter(|a| *a != 0).unwrap_or(code)
    }

    pub fn own(&self) -> HashSet<u32> {
        self.cids.keys().copied().collect()
    }
}
