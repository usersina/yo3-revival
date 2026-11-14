//! The card database (`vendor/BabelCDB/cards.cdb`), the card scripts and the Konami ids (`vendor/cardinfo.json`):
//! which passcode a Konami id plays as in a duel, and back.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::Value as Json;

use crate::core::Card;

/// The folders a card's script (`c<passcode>.lua`) may be in, in order: a few cards' current scripts sit outside
/// `official/` (Red-Eyes Darkness Metal Dragon's in `pre-errata/`). Any other script is at the top.
const CARD_SCRIPT_FOLDERS: [&str; 3] = ["official", "pre-errata", "pre-release"];

/// Our own scripts, beside `vendor/` (`engine/overrides/`, tracked in git): a script there is loaded instead of
/// ProjectIgnis' of the same name, for a card played by the client's 2010–2012 text or an upstream script's bug.
pub fn overrides_dir(vendor: &Path) -> PathBuf {
    vendor.parent().unwrap_or(vendor).join("overrides")
}

/// A card for card-name filters (`declarable`).
pub struct Declarable {
    pub code: u32,
    pub alias: u32,
    pub setcodes: Vec<u16>,
    pub kind: u64,
    pub race: u64,
    pub attribute: u64,
}

/// Card data and scripts, shared by the duel and the bot's look-ahead copies of it (on threads of their own).
pub struct Library {
    vendor: PathBuf,
    /// `overrides_dir`: searched before the vendor's scripts.
    overrides: PathBuf,
    /// Each card's data row: type, level column, atk, def, attribute, race, alias, setcode (a NULL column reads as 0,
    /// as the sidecar's Number(null) did).
    rows: HashMap<u32, [i64; 8]>,
    /// Scripts by file name, read once (None: not found).
    scripts: Mutex<HashMap<String, Option<Arc<Vec<u8>>>>>,
}

impl Library {
    pub fn row(&self, code: u32) -> Option<[i64; 8]> {
        self.rows.get(&code).copied()
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

    /// A script's text by file name: our override of it if there is one, else a card's (`c<passcode>.lua`) from its
    /// folders, any other from the top.
    pub fn script(&self, name: &str) -> Option<Arc<Vec<u8>>> {
        if let Some(known) = self.scripts.lock().unwrap_or_else(|e| e.into_inner()).get(name) {
            return known.clone();
        }
        let folders: &[&str] = if is_card_script(name) { &CARD_SCRIPT_FOLDERS } else { &[""] };
        let text = std::fs::read(self.overrides.join(name))
            .ok()
            .or_else(|| folders.iter().find_map(|f| std::fs::read(self.vendor.join("CardScripts").join(f).join(name)).ok()))
            .map(Arc::new);
        self.scripts.lock().unwrap_or_else(|e| e.into_inner()).insert(name.to_string(), text.clone());
        text
    }
}

/// Whether `name` is a card's script, `c<passcode>.lua`.
pub fn is_card_script(name: &str) -> bool {
    name.strip_prefix('c').and_then(|n| n.strip_suffix(".lua")).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

pub struct Cards {
    pub vendor: PathBuf,
    pub library: Arc<Library>,
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
        let mut rows: HashMap<u32, [i64; 8]> = HashMap::new();
        {
            let mut q = db.prepare("select id, type, level, atk, def, attribute, race, alias, setcode from datas").map_err(|e| e.to_string())?;
            let found = q
                .query_map([], |r| {
                    let mut row = [0i64; 8];
                    for (i, v) in row.iter_mut().enumerate() {
                        *v = r.get::<_, Option<i64>>(i + 1)?.unwrap_or(0);
                    }
                    Ok((r.get::<_, i64>(0)? as u32, row))
                })
                .map_err(|e| e.to_string())?;
            for (id, row) in found.flatten() {
                rows.insert(id, row);
            }
        }
        let alias: HashMap<u32, u32> = rows.iter().map(|(id, r)| (*id, r[6] as u32)).collect();
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
        let library = Arc::new(Library { vendor: vendor.to_path_buf(), overrides: overrides_dir(vendor), rows, scripts: Mutex::default() });
        Ok(Cards { vendor: vendor.to_path_buf(), library, db, passcode_of, cid_of, declarables: None })
    }

    pub fn passcode(&self, cid: u32) -> Option<u32> {
        self.passcode_of.get(&cid).copied()
    }

    pub fn alias(&self, code: u32) -> Option<u32> {
        self.library.row(code).map(|r| r[6] as u32)
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

    /// A card's script string `n` (0-15: the database's str1-str16), as its option descriptions (code << 20 | n) name.
    pub fn string(&self, code: u32, n: u32) -> Option<String> {
        if n > 15 {
            return None;
        }
        let sql = format!("select str{} from texts where id = ?", n + 1);
        self.db.query_row(&sql, params![code], |r| r.get::<_, Option<String>>(0)).optional().ok().flatten().flatten()
    }

    pub fn name(&self, code: u32) -> Option<String> {
        self.db.query_row("select name from texts where id = ?", params![code], |r| r.get::<_, Option<String>>(0)).optional().ok().flatten().flatten()
    }

    /// The card check's view of a card's data: (type, level, attack, defense, attribute, race, alias).
    pub fn data(&self, code: u32) -> Option<[i64; 7]> {
        let [kind, level, atk, def, attribute, race, alias, _] = self.library.row(code)?;
        Some([kind, level & 0xff, atk, def, attribute, race, alias])
    }

    /// A script's text by file name.
    pub fn script(&self, name: &str) -> Option<Arc<Vec<u8>>> {
        self.library.script(name)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_override_is_loaded_before_the_vendors_script() {
        let engine = std::env::temp_dir().join(format!("yo3-overrides-{}", std::process::id()));
        let official = engine.join("vendor/CardScripts/official");
        std::fs::create_dir_all(&official).unwrap();
        std::fs::create_dir_all(engine.join("overrides")).unwrap();
        for (dir, name, text) in [
            (&official, "c1.lua", "upstream 1"),
            (&official, "c2.lua", "upstream 2"),
            (&engine.join("overrides"), "c1.lua", "ours 1"),
            (&engine.join("vendor/CardScripts"), "utility.lua", "upstream utility"),
        ] {
            std::fs::write(dir.join(name), text).unwrap();
        }
        let vendor = engine.join("vendor");
        let library = Library { vendor: vendor.clone(), overrides: overrides_dir(&vendor), rows: HashMap::new(), scripts: Mutex::default() };
        let text = |name: &str| library.script(name).map(|t| String::from_utf8(t.to_vec()).unwrap());
        assert_eq!(text("c1.lua").as_deref(), Some("ours 1"));
        assert_eq!(text("c2.lua").as_deref(), Some("upstream 2"), "a card without an override plays ProjectIgnis' script");
        assert_eq!(text("utility.lua").as_deref(), Some("upstream utility"));
        assert_eq!(text("c3.lua"), None);
        std::fs::remove_dir_all(engine).unwrap();
    }

    #[test]
    fn a_cards_strings_are_its_option_texts() {
        // An option's description is code << 20 | n, n naming the database's str(n + 1): Allure Queen LV3's two effects.
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine/vendor");
        if !vendor.join("BabelCDB/cards.cdb").is_file() {
            return; // engine/fetch.sh not run
        }
        let cards = Cards::open(&vendor).expect("the card data");
        assert_eq!(cards.string(87257460, 0).as_deref(), Some("Equip 1 opponent's Level 3 or lower monster to this card"));
        assert_eq!(cards.string(87257460, 1).as_deref(), Some("Special Summon 1 \"Allure Queen LV5\""));
        assert_eq!(cards.string(87257460, 16), None);
    }
}
