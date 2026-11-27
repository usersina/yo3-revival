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
/// The pseudo card code the engine's own skills (skills.lua) put their texts under: a description or hint of
/// `SKILL_STRINGS << 20 | n` is [`skill_string`] `n`, as a card's script string would be.
pub const SKILL_STRINGS: u32 = 0x7FF0_0000;

/// A skill's text (see [`SKILL_STRINGS`]): 0 Destiny Draw's question, 1 its card choice's title.
pub fn skill_string(n: u32) -> Option<&'static str> {
    match n {
        0 => Some("Destiny Draw: choose the card you draw this turn?"),
        1 => Some("Select the card to draw with Destiny Draw."),
        _ => None,
    }
}

const CARD_SCRIPT_FOLDERS: [&str; 3] = ["official", "pre-errata", "pre-release"];
/// With the anime card versions on, the unofficial scripts (ProjectIgnis' anime cards, `unofficial/c<passcode>.lua`) too,
/// after the others: an official script is never shadowed by an anime one.
const ANIME_SCRIPT_FOLDERS: [&str; 4] = ["official", "pre-errata", "pre-release", "unofficial"];

/// The reviewed map from the client's cards to their anime versions (`engine/anime-cards.json`, made by
/// `tools/engine/anime-map.py`), beside `vendor/`.
pub fn anime_map_file(vendor: &Path) -> PathBuf {
    vendor.parent().unwrap_or(vendor).join("anime-cards.json")
}

/// The cards the game never had (`engine/added-cards.json`, #75): (the id the client knows the card by, its passcode,
/// whether it is anime-only). That id is the card's Konami id, or a stand-in for a newer card whose Konami id is beyond
/// the client's 14 bits or an anime-only card's (docs/added-cards.md). None when the file isn't there.
pub fn read_added(file: &Path) -> Result<Vec<(u32, u32, bool)>, String> {
    let text = match std::fs::read(file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(format!("open '{}': {e}", file.display())),
    };
    let json: Json = serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    json["cards"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| match (c["cid"].as_u64(), c["passcode"].as_u64()) {
            (Some(cid), Some(code)) if cid > 0 && code > 0 => Ok((cid as u32, code as u32, c["anime_only"].as_bool().unwrap_or(false))),
            _ => Err(format!("{}: a card without its cid and passcode", file.display())),
        })
        .collect()
}

/// A corrected anime card record (`"records"` of `engine/anime-cards.json`, from `engine/anime/records.json`): the data row, the
/// name and the strings of an anime card whose upstream entry is a placeholder, replacing the unofficial database's row of the
/// same passcode. Its script is ours (`engine/overrides/c<code>.lua`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Record {
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub kind: i64,
    pub level: i64,
    pub atk: i64,
    pub def: i64,
    pub race: i64,
    pub attribute: i64,
    pub name: String,
    /// The card's script strings (the database's str1...).
    pub strings: Vec<String>,
}

/// The corrected records of the reviewed map file.
pub fn read_anime_records(file: &Path) -> Result<Vec<Record>, String> {
    let text = std::fs::read(file).map_err(|e| format!("open '{}': {e}", file.display()))?;
    let json: Json = serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut out = vec![];
    for r in json["records"].as_array().into_iter().flatten() {
        let num = |key: &str| r[key].as_i64().ok_or_else(|| format!("{}: record {} lacks \"{key}\"", file.display(), r["code"]));
        out.push(Record {
            code: num("code")? as u32,
            alias: num("alias")? as u32,
            setcode: num("setcode")? as u64,
            kind: num("type")?,
            level: num("level")?,
            atk: num("atk")?,
            def: num("def")?,
            race: num("race")?,
            attribute: num("attribute")?,
            name: r["name"].as_str().unwrap_or_default().to_string(),
            strings: r["strings"].as_array().into_iter().flatten().filter_map(|s| s.as_str().map(str::to_string)).collect(),
        });
    }
    Ok(out)
}

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
    /// The anime card versions are on: the unofficial scripts are searched too.
    anime: bool,
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
        // The anime cards' shared procedures (`unofficial/proc_unofficial.lua`, which utility.lua loads) are there too.
        let folders: &[&str] = match (is_card_script(name), self.anime) {
            (false, false) => &[""],
            (false, true) => &["", "unofficial"],
            (true, false) => &CARD_SCRIPT_FOLDERS,
            (true, true) => &ANIME_SCRIPT_FOLDERS,
        };
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
    /// The unofficial database (anime cards' names and strings), with the anime versions on.
    anime_db: Option<Connection>,
    /// Konami id -> passcode: the database's original among the card's artworks, or the card's anime version
    /// (`anime_cards`) when those are on.
    passcode_of: HashMap<u32, u32>,
    /// Konami id -> the database's original passcode, whatever the anime versions do: the name a card is declared by.
    official_of: HashMap<u32, u32>,
    /// The cards that play as their anime version: Konami id -> anime passcode.
    pub anime_cards: HashMap<u32, u32>,
    /// The corrected anime records (name and strings), by passcode.
    records: HashMap<u32, Record>,
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

/// The cards the reviewed map approves (`"status": "mapped"`): Konami id -> anime passcode.
pub fn read_anime_map(file: &Path) -> Result<Vec<(u32, u32)>, String> {
    let text = std::fs::read(file).map_err(|e| format!("open '{}': {e}", file.display()))?;
    let json: Json = serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(json["cards"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["status"] == "mapped")
        .filter_map(|c| Some((c["cid"].as_u64()? as u32, c["anime"].as_u64()? as u32)))
        .collect())
}

impl Cards {
    /// The cards as the client's game has them (the anime versions off).
    #[cfg(test)]
    pub fn open(vendor: &Path) -> Result<Cards, String> {
        Cards::open_with(vendor, false)
    }

    /// `open`, with `anime`: the cards of `engine/anime-cards.json` play as their anime versions (their data from the
    /// unofficial database, their scripts from `unofficial/`), everything else as it is.
    pub fn open_with(vendor: &Path, anime: bool) -> Result<Cards, String> {
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
        // The Konami id tables are made from the official data alone: an unofficial entry (a game exclusive with a script
        // of the anime's) never gives a card the game keeps out of duels a passcode.
        let alias: HashMap<u32, u32> = rows.iter().map(|(id, r)| (*id, r[6] as u32)).collect();
        let official_rows: HashSet<u32> = rows.keys().copied().collect();
        let mut anime_db = None;
        if anime {
            let path = vendor.join("BabelCDB/cards-unofficial.cdb");
            let extra = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
                .map_err(|e| format!("open '{}': {e}", path.display()))?;
            {
                let mut q = extra.prepare("select id, type, level, atk, def, attribute, race, alias, setcode from datas").map_err(|e| format!("{}: {e}", path.display()))?;
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
                    // The official data wins a passcode both hold.
                    rows.entry(id).or_insert(row);
                }
            }
            anime_db = Some(extra);
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
        // An added card plays its passcode under the id the client knows it by: a stand-in id replaces its Konami id both
        // ways, so the engine's messages name the card as the client's rows do.
        let added = vendor.parent().unwrap_or(vendor).join("added-cards.json");
        // An anime-only card (its data and script from the unofficial database) plays only with the anime versions on;
        // the server refuses every duel with it otherwise.
        let added_cards = read_added(&added)?;
        // A stand-in may be the Konami id of a card the client can never show (an Xyz, Link or Pendulum Monster): the id is
        // the added card's, and that card's passcodes name nothing the client has.
        let owner: HashMap<u32, u32> = added_cards.iter().map(|&(cid, code, _)| (cid, code)).collect();
        cid_of.retain(|id, cid| owner.get(cid).is_none_or(|&code| *id == code || alias.get(id) == Some(&code)));
        for (cid, code, anime_only) in added_cards {
            if anime_only && !anime {
                continue;
            }
            if !rows.contains_key(&code) {
                return Err(format!("{}: card {cid} plays {code}, which the card data lacks", added.display()));
            }
            passcode_of.insert(cid, code);
            for id in rows.keys().copied().filter(|id| *id == code || alias.get(id) == Some(&code)).collect::<Vec<_>>() {
                cid_of.insert(id, cid);
            }
        }
        let official_of = passcode_of.clone();
        let mut anime_cards = HashMap::new();
        let mut records = HashMap::new();
        if anime {
            let file = anime_map_file(vendor);
            // A corrected record replaces the unofficial database's row of its passcode.
            for r in read_anime_records(&file)? {
                if official_rows.contains(&r.code) {
                    return Err(format!("{}: record {} is an official card's passcode", file.display(), r.code));
                }
                rows.insert(r.code, [r.kind, r.level, r.atk, r.def, r.attribute, r.race, r.alias as i64, r.setcode as i64]);
                records.insert(r.code, r);
            }
            for (cid, code) in read_anime_map(&file)? {
                if !rows.contains_key(&code) || !passcode_of.contains_key(&cid) {
                    return Err(format!("{}: card {cid} maps to {code}, which the card data lacks", file.display()));
                }
                passcode_of.insert(cid, code);
                cid_of.insert(code, cid);
                anime_cards.insert(cid, code);
            }
        }
        let library = Arc::new(Library { vendor: vendor.to_path_buf(), overrides: overrides_dir(vendor), rows, scripts: Mutex::default(), anime });
        Ok(Cards { vendor: vendor.to_path_buf(), library, db, anime_db, passcode_of, official_of, anime_cards, records, cid_of, declarables: None })
    }

    pub fn passcode(&self, cid: u32) -> Option<u32> {
        self.passcode_of.get(&cid).copied()
    }

    /// The database's original passcode for Konami id `cid`, the card's name for the core's name checks.
    pub fn official(&self, cid: u32) -> Option<u32> {
        self.official_of.get(&cid).copied()
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
        if code == SKILL_STRINGS {
            return skill_string(n).map(str::to_string);
        }
        if n > 15 {
            return None;
        }
        if let Some(r) = self.records.get(&code) {
            return r.strings.get(n as usize).filter(|t| !t.is_empty()).cloned();
        }
        let sql = format!("select str{} from texts where id = ?", n + 1);
        std::iter::once(&self.db).chain(self.anime_db.as_ref()).find_map(|db| db.query_row(&sql, params![code], |r| r.get::<_, Option<String>>(0)).optional().ok().flatten().flatten())
    }

    pub fn name(&self, code: u32) -> Option<String> {
        if let Some(r) = self.records.get(&code) {
            return Some(r.name.clone());
        }
        std::iter::once(&self.db).chain(self.anime_db.as_ref()).find_map(|db| {
            db.query_row("select name from texts where id = ?", params![code], |r| r.get::<_, Option<String>>(0)).optional().ok().flatten().flatten()
        })
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
        let code = cards.official(cid).or_else(|| self.passcodes.get(&cid).copied()).unwrap_or(0);
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
        let library = Library { vendor: vendor.clone(), overrides: overrides_dir(&vendor), rows: HashMap::new(), scripts: Mutex::default(), anime: false };
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

    #[test]
    fn an_added_card_is_known_by_the_id_the_client_has() {
        // Shooting Quasar Dragon (#75) by its own Konami id; a stand-in id is read the same way.
        let file = std::env::temp_dir().join(format!("yo3-added-{}.json", std::process::id()));
        std::fs::write(&file, r#"{"format":"yo3-added-cards","version":1,"first_row":4834,"cards":[{"cid":9709,"passcode":35952884},{"cid":3901,"passcode":18847598}]}"#).unwrap();
        assert_eq!(read_added(&file).unwrap(), vec![(9709, 35952884, false), (3901, 18847598, false)]);
        std::fs::write(&file, r#"{"cards":[{"cid":3968,"passcode":170000171,"anime_only":true}]}"#).unwrap();
        assert_eq!(read_added(&file).unwrap(), vec![(3968, 170000171, true)], "an anime-only card");
        std::fs::write(&file, r#"{"cards":[{"name":"no ids"}]}"#).unwrap();
        assert!(read_added(&file).is_err());
        std::fs::remove_file(&file).unwrap();
        assert_eq!(read_added(&file).unwrap(), vec![], "no file: no added cards");
    }
}
