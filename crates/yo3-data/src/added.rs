//! Cards the game never had (#75): `engine/added-cards.json` (made by `tools/engine/added-cards.py` from
//! `engine/added/cards.tsv`), each a complete row of the client's card table after the game's own rows.
//!
//! Two consumers read the same list. The server adds the rows to the [`CardDb`] it uses for everything ([`CardDb::add`]),
//! so decks, collections, deck checks and duels know the cards by row like any other. The launcher builds the extended
//! copies of the game's card files ([`extend_files`]), which the connection DLL hands the game in place of its own when
//! it loads them (the loader `0x40DEE0` reads every file to its end into growable arrays, and finds a Konami id's row
//! through `CARD_IntID`, whose bounds it computes from the rows: no count is fixed in the game). The game then has the
//! new rows as if they had always been there.
//!
//! The files, by what a row needs (docs/client-internals.md, Card data):
//! - `CARD_Prop` (8 bytes a row) and `CARD_Genre` (8 bytes a row): appended.
//! - `CARD_IntID`: a row number (u16) per Konami id from the lowest id of the table; grown to the new ids.
//! - `CARD_Indx_<lang>`: a name and a text offset per row into `CARD_Name_<lang>` / `CARD_Desc_<lang>` (UTF-16, NUL
//!   ended), then one more entry, the files' ends. `_R` (the readings) has no text file.
//! - `CARD_Sort_<lang>` and `CARD_Sort2_<lang>`: the rows in two display orders (`Sort2` alphabetical, `Sort` the game's
//!   own; alternate artworks are left out of `Sort`). A new row goes in alphabetical place in `Sort2` and at the end of
//!   `Sort`. A row missing from them would not appear where the game walks these lists.
//!
//! The added cards' names and texts are English in every language's files (the database has no other).
use serde::{Deserialize, Serialize};

use crate::{Card, CardDb};

/// The file's `format`.
pub const FORMAT: &str = "yo3-added-cards";
/// The languages whose card files the game ships (`E` English first), and `R`, the readings, which has names only.
pub const LANGUAGES: [&str; 7] = ["E", "F", "G", "I", "J", "K", "S"];

/// One added card: its permanent row and Konami id, the EDOPro passcode the engine plays, and the row's words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddedCard {
    pub row: u16,
    pub cid: u16,
    pub passcode: u32,
    pub name: String,
    /// The card's text, with the game's line break (`\r`).
    pub desc: String,
    /// `CARD_Prop`'s two words: `a` the Konami id, ATK/10 and DEF/10; `b` the kind, attribute, level, race and artwork.
    pub a: u32,
    pub b: u32,
    #[serde(default)]
    pub genre: u64,
}

impl AddedCard {
    /// The row as [`CardDb`] holds the game's own.
    pub fn card(&self) -> Card {
        let (a, b) = (self.a, self.b);
        Card {
            row: self.row,
            cid: self.cid,
            atk: ((a >> 14) & 0x1FF) * 10,
            def: ((a >> 23) & 0x1FF) * 10,
            level: (b >> 12) & 0xF,
            attribute: (b >> 8) & 0xF,
            race: (b >> 19) & 0x1F,
            kind: (b >> 3) & 0x1F,
            art: (b >> 27) & 7,
            words: [a, b],
            genre: self.genre,
            name: self.name.clone(),
            desc: self.desc.clone(),
        }
    }
}

/// The added cards of a server: none, or rows `first_row`, `first_row + 1`, ... in order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Added {
    pub first_row: u16,
    pub cards: Vec<AddedCard>,
}

#[derive(Deserialize)]
struct File {
    format: String,
    version: u32,
    #[serde(flatten)]
    added: Added,
}

impl Added {
    /// Reads `engine/added-cards.json`'s text: its format, and rows that follow the game's without a gap.
    pub fn parse(text: &str) -> Result<Added, String> {
        let file: File = serde_json::from_str(text).map_err(|e| format!("added cards: {e}"))?;
        if file.format != FORMAT || file.version != 1 {
            return Err(format!("added cards: not a {FORMAT} 1 file ({} {})", file.format, file.version));
        }
        let added = file.added;
        for (i, card) in added.cards.iter().enumerate() {
            if usize::from(card.row) != usize::from(added.first_row) + i {
                return Err(format!("added cards: {} is row {}, not {}", card.name, card.row, usize::from(added.first_row) + i));
            }
            if card.a & 0x3FFF != u32::from(card.cid) || card.cid == 0 {
                return Err(format!("added cards: {}'s row doesn't hold its Konami id {}", card.name, card.cid));
            }
        }
        Ok(added)
    }

    /// The added cards of an engine directory (`added-cards.json` in it): none when the file isn't there.
    pub fn load(engine_dir: &std::path::Path) -> Result<Added, String> {
        match std::fs::read_to_string(engine_dir.join("added-cards.json")) {
            Ok(text) => Added::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Added::default()),
            Err(e) => Err(format!("added cards: {e}")),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// A short name for this list (FNV-1a of its JSON), for a launcher's folder of extended files.
    pub fn hash(&self) -> String {
        let text = serde_json::to_string(self).unwrap_or_default();
        let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
        format!("{hash:016x}")
    }

    /// The file the server serves the launcher (and the launcher reads back).
    pub fn render(&self) -> String {
        let mut value = serde_json::to_value(self).unwrap_or_default();
        if let Some(map) = value.as_object_mut() {
            map.insert("format".into(), FORMAT.into());
            map.insert("version".into(), 1.into());
        }
        value.to_string()
    }
}

impl CardDb {
    /// Whether `row` is an added card's (after the game's own table).
    pub fn added_row(&self, row: u16) -> bool {
        self.first_added.is_some_and(|first| row >= first)
    }

    /// Adds `added`'s rows after the game's own: `first_row` must be the row after the table's last.
    pub fn add(&mut self, added: &Added) -> Result<(), String> {
        if added.is_empty() {
            return Ok(());
        }
        let next = self.cards.iter().map(|c| c.row).max().map_or(0, |r| r + 1);
        if added.first_row != next {
            return Err(format!("added cards start at row {}, but the game's table ends at row {}", added.first_row, next));
        }
        self.first_added = Some(added.first_row);
        for card in &added.cards {
            if self.by_cid.contains_key(&card.cid) {
                return Err(format!("added cards: Konami id {} ({}) is a card of the game", card.cid, card.name));
            }
            let c = card.card();
            self.by_cid.insert(c.cid, self.cards.len());
            self.by_row.insert(c.row, self.cards.len());
            self.cards.push(c);
        }
        Ok(())
    }
}

fn u16s(bytes: &[u8]) -> Vec<u16> {
    bytes.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect()
}

fn u32s(bytes: &[u8]) -> Vec<u32> {
    bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect()
}

fn bytes16(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn bytes32(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn utf16z(text: &str) -> Vec<u8> {
    text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
}

/// The name at a byte offset of a `CARD_Name` file, lower-cased, for `Sort2`'s alphabetical place.
fn name_at(names: &[u8], offset: u32) -> String {
    crate::utf16z(names.get(offset as usize..).unwrap_or_default()).to_lowercase()
}

/// The game's card files with `added`'s rows, from `read` (a file of the game's archives by its name, as
/// `data/bin#/CARD_Prop.bin`): every file that changes, by that name. Empty when nothing is added.
pub fn extend_files(read: impl Fn(&str) -> Option<Vec<u8>>, added: &Added) -> Result<Vec<(String, Vec<u8>)>, String> {
    if added.is_empty() {
        return Ok(vec![]);
    }
    let path = |name: &str| format!("data/bin#/{name}.bin");
    let need = |name: &str| read(&path(name)).ok_or_else(|| format!("the game has no {}", path(name)));
    let mut out = Vec::new();
    let rows = usize::from(added.first_row);

    let mut prop = need("CARD_Prop")?;
    if prop.len() != rows * 8 {
        return Err(format!("CARD_Prop has {} rows, the added cards start at {rows}", prop.len() / 8));
    }
    let row_words: Vec<(u32, u32)> = prop.as_chunks::<8>().0.iter().map(|r| (u32::from_le_bytes(r[..4].try_into().unwrap()), u32::from_le_bytes(r[4..].try_into().unwrap()))).collect();
    for card in &added.cards {
        prop.extend(bytes32(&[card.a, card.b]));
    }
    out.push((path("CARD_Prop"), prop));

    if let Some(mut genre) = read(&path("CARD_Genre")) {
        genre.resize(rows * 8, 0);
        for card in &added.cards {
            genre.extend(card.genre.to_le_bytes());
        }
        out.push((path("CARD_Genre"), genre));
    }

    // CARD_IntID[i] is the row of Konami id `base + i`, `base` the table's lowest id (0x536FF0 computes it the same way).
    let base = row_words.iter().map(|(a, _)| (a & 0x3FFF) as u16).filter(|&c| c != 0).min().ok_or("CARD_Prop has no card")?;
    let mut intid = u16s(&need("CARD_IntID")?);
    for card in &added.cards {
        let at = usize::from(card.cid.checked_sub(base).ok_or_else(|| format!("{}'s Konami id is below the table's lowest", card.name))?);
        if intid.len() <= at {
            intid.resize(at + 1, 0);
        }
        intid[at] = card.row;
    }
    out.push((path("CARD_IntID"), bytes16(&intid)));

    for lang in LANGUAGES.iter().copied().chain(["R"]) {
        let (Some(index), Some(mut names)) = (read(&path(&format!("CARD_Indx_{lang}"))), read(&path(&format!("CARD_Name_{lang}")))) else { continue };
        let mut index = u32s(&index);
        if index.len() != (rows + 1) * 2 {
            return Err(format!("CARD_Indx_{lang} has {} entries for {rows} rows", index.len() / 2));
        }
        let mut descs = if lang == "R" { None } else { read(&path(&format!("CARD_Desc_{lang}"))) };
        index.truncate(rows * 2);
        let desc_end = |descs: &Option<Vec<u8>>, index: &[u32]| descs.as_ref().map_or(index.last().copied().unwrap_or(0), |d| d.len() as u32);
        for card in &added.cards {
            index.push(names.len() as u32);
            index.push(desc_end(&descs, &index));
            names.extend(utf16z(&card.name));
            if let Some(d) = &mut descs {
                d.extend(utf16z(&card.desc));
            }
        }
        index.push(names.len() as u32);
        index.push(desc_end(&descs, &index));
        let names_of = names.clone();
        out.push((path(&format!("CARD_Indx_{lang}")), bytes32(&index)));
        out.push((path(&format!("CARD_Name_{lang}")), names));
        if let Some(d) = descs {
            out.push((path(&format!("CARD_Desc_{lang}")), d));
        }
        if lang == "R" {
            continue;
        }
        if let Some(sort) = read(&path(&format!("CARD_Sort_{lang}"))) {
            let mut sort = u16s(&sort);
            sort.extend(added.cards.iter().map(|c| c.row));
            out.push((path(&format!("CARD_Sort_{lang}")), bytes16(&sort)));
        }
        if let Some(sort) = read(&path(&format!("CARD_Sort2_{lang}"))) {
            let mut sort = u16s(&sort);
            let name_of_row = |row: u16| index.get(usize::from(row) * 2).map(|&o| name_at(&names_of, o)).unwrap_or_default();
            for card in &added.cards {
                let name = card.name.to_lowercase();
                let at = sort.iter().position(|&r| r != 0 && name_of_row(r) > name).unwrap_or(sort.len());
                sort.insert(at, card.row);
            }
            out.push((path(&format!("CARD_Sort2_{lang}")), bytes16(&sort)));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn quasar(row: u16) -> AddedCard {
        AddedCard { row, cid: 30, passcode: 35952884, name: "Bravo".into(), desc: "Text\rmore".into(),
            a: 30 | 400 << 14 | 400 << 23, b: 18 << 3 | 1 << 8 | 12 << 12 | 1 << 19 | 1 << 27, genre: 5 }
    }

    /// A game of three rows (0 empty, "Alpha" id 10, "Charlie" id 12) in English and the readings.
    fn game() -> HashMap<String, Vec<u8>> {
        let mut files = HashMap::new();
        let mut put = |name: &str, bytes: Vec<u8>| { files.insert(format!("data/bin#/{name}.bin"), bytes); };
        put("CARD_Prop", bytes32(&[0, 0, 10, 0, 12, 0]));
        put("CARD_Genre", vec![0; 24]);
        put("CARD_IntID", bytes16(&[1, 0, 2]));
        let mut names = utf16z(""); let mut descs = utf16z("");
        let mut index = vec![0, 0];
        for (n, d) in [("Alpha", "a"), ("Charlie", "c")] {
            index.extend([names.len() as u32, descs.len() as u32]);
            names.extend(utf16z(n)); descs.extend(utf16z(d));
        }
        index.extend([names.len() as u32, descs.len() as u32]);
        put("CARD_Indx_E", bytes32(&index)); put("CARD_Name_E", names.clone()); put("CARD_Desc_E", descs);
        put("CARD_Indx_R", bytes32(&index)); put("CARD_Name_R", names);
        put("CARD_Sort_E", bytes16(&[0, 2, 1]));
        put("CARD_Sort2_E", bytes16(&[0, 1, 2]));
        files
    }

    #[test]
    fn the_files_gain_the_row_in_every_table() {
        let files = game();
        let added = Added { first_row: 3, cards: vec![quasar(3)] };
        let out: HashMap<String, Vec<u8>> = extend_files(|n| files.get(n).cloned(), &added).unwrap().into_iter().collect();
        let file = |n: &str| out[&format!("data/bin#/{n}.bin")].clone();
        assert_eq!(u32s(&file("CARD_Prop"))[6..], [added.cards[0].a, added.cards[0].b]);
        assert_eq!(file("CARD_Genre").len(), 32);
        // Id 30 is base 10 + 20: the lookup grows to it, the ids in between stay 0.
        let intid = u16s(&file("CARD_IntID"));
        assert_eq!((intid.len(), intid[20], intid[0], intid[2]), (21, 3, 1, 2));
        // The index gains the row and keeps its end entry; the names and texts follow.
        let index = u32s(&file("CARD_Indx_E"));
        assert_eq!(index.len(), 10);
        assert_eq!(name_at(&file("CARD_Name_E"), index[6]), "bravo");
        assert_eq!(crate::utf16z(&file("CARD_Desc_E")[index[7] as usize..]), "Text\rmore");
        assert_eq!(index[8] as usize, file("CARD_Name_E").len());
        assert_eq!(index[9] as usize, file("CARD_Desc_E").len());
        let parsed = CardDb::parse(&file("CARD_Prop"), &file("CARD_Indx_E"), &file("CARD_Name_E"), &file("CARD_Desc_E"));
        assert_eq!(parsed.by_cid(30).map(|c| (c.row, c.name.as_str(), c.atk, c.kind)), Some((3, "Bravo", 4000, 18)));
        assert_eq!(parsed.by_cid(12).map(|c| c.name.as_str()), Some("Charlie"), "the game's rows are untouched");
        // The readings have names only.
        assert_eq!(u32s(&file("CARD_Indx_R")).len(), 10);
        assert!(!out.contains_key("data/bin#/CARD_Desc_R.bin"));
        // Sort2 is alphabetical: Bravo between Alpha and Charlie; Sort gets it at the end.
        assert_eq!(u16s(&file("CARD_Sort2_E")), [0, 1, 3, 2]);
        assert_eq!(u16s(&file("CARD_Sort_E")), [0, 2, 1, 3]);
    }

    #[test]
    fn a_list_must_follow_the_game_and_parse_its_own_rendering() {
        let files = game();
        let wrong = Added { first_row: 5, cards: vec![quasar(5)] };
        assert!(extend_files(|n| files.get(n).cloned(), &wrong).is_err(), "a gap after the game's rows");
        assert!(extend_files(|n| files.get(n).cloned(), &Added::default()).unwrap().is_empty());
        let added = Added { first_row: 3, cards: vec![quasar(3)] };
        assert_eq!(Added::parse(&added.render()).unwrap(), added);
        assert_eq!(added.hash(), Added::parse(&added.render()).unwrap().hash());
        let mut skipped = added.clone();
        skipped.cards[0].row = 4;
        assert!(Added::parse(&skipped.render()).is_err());
        let mut db = CardDb::parse(&files["data/bin#/CARD_Prop.bin"], &files["data/bin#/CARD_Indx_E.bin"], &files["data/bin#/CARD_Name_E.bin"], &files["data/bin#/CARD_Desc_E.bin"]);
        db.add(&added).unwrap();
        assert_eq!(db.by_row(3).map(|c| (c.cid, c.extra_deck())), Some((30, true)), "a Synchro belongs in the Extra Deck");
        let mut fresh = CardDb::parse(&files["data/bin#/CARD_Prop.bin"], &files["data/bin#/CARD_Indx_E.bin"], &files["data/bin#/CARD_Name_E.bin"], &files["data/bin#/CARD_Desc_E.bin"]);
        assert!(fresh.add(&wrong).is_err(), "rows must start right after the game's");
        assert!(db.add(&added).is_err(), "a Konami id the table has already");
    }
}
