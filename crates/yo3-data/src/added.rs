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
    /// A card never printed (#75): played only with the anime card versions on; every duel with it is refused otherwise.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub anime_only: bool,
    /// A Token a card's effect makes (TYPE_TOKEN, no Konami id: a stand-in): a row of the client's card table, so the
    /// client can show it on the field, and never a card a player holds.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub token: bool,
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
    /// The server has anime cards off: its anime-only cards are Forbidden in every banlist the launcher writes
    /// ([`extend_files`]), so Edit Deck marks them and the game's own deck check refuses a deck with one. Not in the
    /// engine's file; the server sets it in what it serves ([`Added::render_for`]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub anime_only_banned: bool,
    /// Characters the game never had as NPCs (`engine/added/npcs.json`), in the Kaiba Corporation slots past its 25.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub npcs: Vec<AddedNpc>,
}

/// A custom NPC: slot 25 on (ids `120 + 3 * slot` to `+ 2`, one per level), on a body the game has (`body`, one of its
/// 25 NPCs: the launcher copies that NPC's model, portrait and faces until the character has its own pictures).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddedNpc {
    pub slot: u32,
    pub name: String,
    /// The Kaiba Corporation screen's text about them.
    pub help: String,
    /// The tab: 0 Yu-Gi-Oh!, 1 GX, 2 5D's.
    pub series: u32,
    /// The game NPC whose skeleton and animations it moves with (0-24).
    pub body: u32,
    /// What it wears on that skeleton: "mannequin" for our clothes-store mannequin (`doll.rs`), else the game NPC's
    /// own body and skins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figure: Option<String>,
    /// Three, the easiest first: each a deck by card names (the anime deck plays while the server has anime cards on).
    pub levels: Vec<AddedNpcLevel>,
    /// What they say, by line number in their block of the UI texts (`62000 + 300 * slot + n`): per level a hundred,
    /// as the game's own NPCs (0 the duel's opening, 1-24 the duel's moments, 24 the ace monster with `@@m`, 25-36
    /// Kaiba Corporation's talk, 37-86 the win, loss and draw; `@@s` the player's name). The talk needs them: without,
    /// the scene shows no body.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    /// Keyed by the line number as text: the added cards are read flattened, where serde can't read number keys.
    pub lines: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddedNpcLevel {
    pub title: String,
    pub deck: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anime_deck: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anime_extra: Vec<String>,
}

/// The first NPC id (0x4BFE34): NPC k's levels are `FIRST_NPC_ID + 3k` to `+ 2`.
pub const FIRST_NPC_ID: u32 = 120;
/// Text ids: NPC k's name `0x5208 + 2k` and help `+ 1` (0x4BFE34); NPC id i's deck title `0x5FB4 + 2i` (0x4BFE05).
pub const NPC_NAME_TEXT: u32 = 0x5208;
pub const NPC_DECK_TITLE_TEXT: u32 = 0x5FB4;
/// NPC k's lines: texts `62000 + 300k` on (Pegasus's "My turn now!" is 63501).
pub const NPC_LINES_TEXT: u32 = 62000;

impl AddedNpc {
    /// Its three NPC ids, one per level.
    pub fn ids(&self) -> [u32; 3] {
        let first = FIRST_NPC_ID + 3 * self.slot;
        [first, first + 1, first + 2]
    }

    /// Its UI texts: name, help and each level's deck title.
    pub fn texts(&self) -> Vec<(u32, String)> {
        let mut out = vec![(NPC_NAME_TEXT + 2 * self.slot, self.name.clone()), (NPC_NAME_TEXT + 2 * self.slot + 1, self.help.clone())];
        out.extend(self.ids().iter().zip(&self.levels).map(|(id, level)| (NPC_DECK_TITLE_TEXT + 2 * id, level.title.clone())));
        out.extend(self.lines.iter().filter_map(|(n, text)| Some((NPC_LINES_TEXT + 300 * self.slot + n.parse::<u32>().ok().filter(|&n| n < 300)?, text.clone()))));
        out
    }
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
        // With the files' format: a launcher writes new kinds of files (the Anime Collection pack) into a new folder.
        let text = format!("{FILES_FORMAT}{}", serde_json::to_string(self).unwrap_or_default());
        let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
        format!("{hash:016x}")
    }

    /// The file the server serves the launcher with anime cards on or off (`anime`): off, its anime-only cards are banned.
    pub fn render_for(&self, anime: bool) -> String {
        let banned = !anime && self.cards.iter().any(|c| c.anime_only && !c.token);
        if banned == self.anime_only_banned {
            return self.render();
        }
        Added { anime_only_banned: banned, ..self.clone() }.render()
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
    out.extend(banlists(&read, added)?);
    out.extend(anime_pack_files(&read, added)?);
    out.extend(text_files(&read, added)?);
    out.extend(npc_files(&read, added)?);
    Ok(out)
}

/// What [`extend_files`] writes, as part of [`Added::hash`]: 2 added the Anime Collection pack's file and texts.
const FILES_FORMAT: &str = "files 2 ";

/// The Anime Collection pack (pack id [`ANIME_PACK`]): Blister's sells it while the server has anime cards on.
pub const ANIME_PACK: u32 = 300;
/// Its name and description in the UI text table: 23120-23199 is free in every language (the packs' texts are 23000-23119).
pub const ANIME_PACK_NAME: u32 = 23120;
pub const ANIME_PACK_DESC: u32 = 23121;
/// Its picture (`data/img/pack/`, 145x290; the launcher ships it).
pub const ANIME_PACK_IMAGE: &str = "YO3_ANIME_1.png";
const ANIME_PACK_TEXTS: [(u32, &str); 2] = [(ANIME_PACK_NAME, "Anime Collection"),
    (ANIME_PACK_DESC, "Cards from the anime that were never printed. On sale while the server plays the anime cards.")];

/// A card's rarity (`b` bits 24-26: 0 Common, 4 Rare, 3 Super Rare, 2 Ultra Rare) as its tier: 0 to 3.
fn tier_of(card: &AddedCard) -> usize {
    match (card.b >> 24) & 7 { 4 => 1, 3 => 2, 2 | 1 | 5 => 3, _ => 0 }
}

/// The anime-only cards (no Tokens) by rarity tier, Common first, each tier in row order; empty tiers left out.
pub fn anime_pack_tiers(added: &Added) -> Vec<Vec<u16>> {
    let mut tiers = vec![vec![]; 4];
    for card in added.cards.iter().filter(|c| c.anime_only && !c.token) {
        tiers[tier_of(card)].push(card.row);
    }
    tiers.into_iter().filter(|t: &Vec<u16>| !t.is_empty()).collect()
}

/// The pack's file (`data/pack/300.y3a`, pack 1's record with its own id, texts, picture and cards) and the UI text
/// tables with its name and description, when the server has anime-only cards. The picture is the launcher's.
pub fn anime_pack_files(read: &impl Fn(&str) -> Option<Vec<u8>>, added: &Added) -> Result<Vec<(String, Vec<u8>)>, String> {
    let tiers = anime_pack_tiers(added);
    if tiers.is_empty() {
        return Ok(vec![]);
    }
    let template = read("data/pack/1.y3a").ok_or("the game has no data/pack/1.y3a")?;
    let mut pack = yo3_proto::decode_file(&template).map_err(|e| format!("data/pack/1.y3a: {e}"))?;
    let rows: Vec<u8> = tiers.iter().flatten().flat_map(|r| r.to_be_bytes()).collect();
    for (key, value) in [("pack_id", yo3_proto::Value::U32Alt(ANIME_PACK)), ("name_id", yo3_proto::Value::text(&ANIME_PACK_NAME.to_string())),
        ("discription_id", yo3_proto::Value::text(&ANIME_PACK_DESC.to_string())), ("pack_image", yo3_proto::Value::text(ANIME_PACK_IMAGE)),
        ("pack", yo3_proto::Value::Bin(rows)), ("pbin_no", yo3_proto::Value::U32Alt(0))] {
        match pack.fields.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => pack.fields.push((key.into(), value)),
        }
    }
    Ok(vec![(format!("data/pack/{ANIME_PACK}.y3a"), yo3_proto::frame(&pack.encode()))])
}

/// The UI text tables with the server's own texts (the Anime Collection's, the custom NPCs'), all seven languages.
pub fn text_files(read: &impl Fn(&str) -> Option<Vec<u8>>, added: &Added) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut texts: Vec<(u32, String)> = vec![];
    if !anime_pack_tiers(added).is_empty() {
        texts.extend(ANIME_PACK_TEXTS.iter().map(|(id, t)| (*id, t.to_string())));
    }
    for npc in &added.npcs {
        texts.extend(npc.texts());
    }
    if texts.is_empty() {
        return Ok(vec![]);
    }
    let texts: Vec<(u32, &str)> = texts.iter().map(|(id, t)| (*id, t.as_str())).collect();
    let mut out = vec![];
    for lang in LANGUAGES {
        let name = format!("data/text/txttbl_{lang}.bin");
        let Some(table) = read(&name) else { continue };
        out.push((name.clone(), with_texts(&table, &texts).map_err(|e| format!("{name}: {e}"))?));
    }
    Ok(out)
}

/// The textures a model names (`.\\np0050.png`), by their file name.
fn skins(mesh: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for at in mesh.windows(4).enumerate().filter(|(_, w)| w.eq_ignore_ascii_case(b".png")).map(|(i, _)| i) {
        let start = mesh[..at].iter().rposition(|&b| !(b.is_ascii_alphanumeric() || b == b'_' || b == b'-')).map_or(0, |i| i + 1);
        let name = String::from_utf8_lossy(&mesh[start..at + 4]).to_string();
        if start < at && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// Each custom NPC's body, portrait and faces, copied from the game NPC it borrows them from (`data/model/np005.mdl` ->
/// `data/model/np025.mdl`, `data/npc/npcsel025.png`, `data/face/np025f0.png`...): the model names its skins relatively,
/// so it wears the borrowed NPC's.
pub fn npc_files(read: &impl Fn(&str) -> Option<Vec<u8>>, added: &Added) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = vec![];
    for npc in &added.npcs {
        if !(25..30).contains(&npc.slot) || npc.body >= 25 {
            return Err(format!("{}: slot {} (25-29) on body {} (0-24)", npc.name, npc.slot, npc.body));
        }
        let (from, to) = (npc.body, npc.slot);
        let model = format!("data/model/np{from:03}.mdl");
        let mut mesh = read(&model).ok_or_else(|| format!("the game has no {model}"))?;
        if npc.figure.as_deref() == Some("mannequin") {
            mesh = crate::doll::mannequin(&mesh).map_err(|e| format!("{}'s mannequin: {e}", npc.name))?;
            out.push((format!("data/model/np{to:03}/{}", crate::doll::TEXTURE), crate::doll::shading_png()));
        } else if let Some(figure) = &npc.figure {
            return Err(format!("{}: no figure named {figure:?} (mannequin)", npc.name));
        }
        // Its own folder, `data/model/np025/`: the model finds its skins by name beside it, so the borrowed body's
        // textures go there too (a character's own skins later replace them under the same names: the "doll").
        for skin in skins(&mesh).into_iter().filter(|s| s != crate::doll::TEXTURE) {
            if let Some(png) = read(&format!("data/model/{skin}")) {
                out.push((format!("data/model/np{to:03}/{skin}"), png));
            }
        }
        out.push((format!("data/model/np{to:03}/np{to:03}.mdl"), mesh));
        // Which game NPC's animations it plays, for the DLL's cut-in timing (tools/mod/card-profile.h).
        out.push((format!("data/model/np{to:03}/donor.txt"), from.to_string().into_bytes()));
        // Its portrait and duel faces: the mannequin's own pictures, else the borrowed NPC's.
        let mannequin = npc.figure.as_deref() == Some("mannequin");
        let portrait = if mannequin { Some(crate::doll::portrait_png()) } else { read(&format!("data/npc/npcsel{from:03}.png")) };
        if let Some(portrait) = portrait {
            out.push((format!("data/npc/npcsel{to:03}.png"), portrait));
        }
        for k in 0..5 {
            let face = if mannequin { Some(crate::doll::face_png()) } else { read(&format!("data/face/np{from:03}f{k}.png")) };
            if let Some(face) = face {
                out.push((format!("data/face/np{to:03}f{k}.png"), face));
            }
        }
    }
    Ok(out)
}

/// A UI text table (`crate::parse_texts`' format) with `texts` added after its own (an id it has already is pointed at the
/// new text; the game builds a map from the pairs at load, 0x51E6C0, so their order doesn't matter):
/// u32 count, count pairs {u32 id, u32 offset into the strings}, u32 the strings' size, then the strings (UTF-16LE, NUL).
pub fn with_texts(table: &[u8], texts: &[(u32, &str)]) -> Result<Vec<u8>, String> {
    let u32_at = |at: usize| table.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or("too short");
    let count = u32_at(0)? as usize;
    let base = 8 + 8 * count;
    if table.len() < base { return Err("too short".into()) }
    let mut pairs: Vec<(u32, u32)> = (0..count).map(|i| Ok((u32_at(4 + 8 * i)?, u32_at(8 + 8 * i)?))).collect::<Result<_, &str>>()?;
    let mut strings = table[base..].to_vec();
    for &(id, text) in texts {
        let offset = strings.len() as u32;
        strings.extend(utf16z(text));
        match pairs.iter_mut().find(|(i, _)| *i == id) {
            Some(pair) => pair.1 = offset,
            None => pairs.push((id, offset)),
        }
    }
    let mut out = (pairs.len() as u32).to_le_bytes().to_vec();
    for (id, offset) in &pairs { out.extend(id.to_le_bytes()); out.extend(offset.to_le_bytes()); }
    out.extend((strings.len() as u32).to_le_bytes());
    out.extend(strings);
    Ok(out)
}

/// The game's banlists (`data/regulation/<id>.y3r`, 1 to 20) with the anime-only cards Forbidden, when the server has
/// anime cards off ([`Added::anime_only_banned`]): each card's Konami id joins `card0`, the Forbidden list, which Edit Deck
/// marks and the game's deck check (0x51A9A0) refuses. A banlist whose unlisted cards are allowed none
/// (`addition_card` 0, the formats) already refuses them and is left alone.
fn banlists(read: &impl Fn(&str) -> Option<Vec<u8>>, added: &Added) -> Result<Vec<(String, Vec<u8>)>, String> {
    let banned: Vec<u16> = added.cards.iter().filter(|c| c.anime_only && !c.token).map(|c| c.cid).collect();
    if !added.anime_only_banned || banned.is_empty() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for id in 1..=255u32 {
        let name = format!("data/regulation/{id}.y3r");
        let Some(blob) = read(&name) else { continue };
        let mut rule = yo3_proto::decode_file(&blob).map_err(|e| format!("{name}: {e}"))?;
        if rule.get("addition_card").and_then(yo3_proto::Value::as_u32).unwrap_or(3) == 0 {
            continue;
        }
        let mut forbidden: Vec<u8> = rule.get("card0").and_then(yo3_proto::Value::as_bytes).map(<[u8]>::to_vec).unwrap_or_default();
        let listed: Vec<u16> = forbidden.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect();
        for cid in banned.iter().filter(|c| !listed.contains(c)) {
            forbidden.extend(cid.to_be_bytes());
        }
        match rule.fields.iter_mut().find(|(k, _)| k == "card0") {
            Some((_, v)) => *v = yo3_proto::Value::Bin(forbidden),
            None => rule.fields.push(("card0".into(), yo3_proto::Value::Bin(forbidden))),
        }
        out.push((name, yo3_proto::frame(&rule.encode())));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn quasar(row: u16) -> AddedCard {
        AddedCard { row, cid: 30, passcode: 35952884, name: "Bravo".into(), desc: "Text\rmore".into(),
            a: 30 | 400 << 14 | 400 << 23, b: 18 << 3 | 1 << 8 | 12 << 12 | 1 << 19 | 1 << 27, genre: 5, anime_only: false, token: false }
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
        let added = Added { first_row: 3, cards: vec![quasar(3)], ..Default::default() };
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
        let wrong = Added { first_row: 5, cards: vec![quasar(5)], ..Default::default() };
        assert!(extend_files(|n| files.get(n).cloned(), &wrong).is_err(), "a gap after the game's rows");
        assert!(extend_files(|n| files.get(n).cloned(), &Added::default()).unwrap().is_empty());
        let added = Added { first_row: 3, cards: vec![quasar(3)], ..Default::default() };
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
        // The tool's file: a stand-in card keeps its real Konami id beside the id the client knows (`cid`), and every card
        // a note; the server reads only what it needs, and an added Spell is a main-deck card.
        let file = r#"{"format":"yo3-added-cards","version":1,"first_row":3,"cards":[{"row":3,"cid":11,"passcode":12644061,"name":"Spell",
            "desc":"","a":11,"b":133224,"genre":0,"note":"","konami_id":17632}]}"#;
        let spell = Added::parse(file).unwrap();
        assert_eq!((spell.cards[0].cid, spell.cards[0].card().kind), (11, 13));
        let mut db = CardDb::parse(&files["data/bin#/CARD_Prop.bin"], &files["data/bin#/CARD_Indx_E.bin"], &files["data/bin#/CARD_Name_E.bin"], &files["data/bin#/CARD_Desc_E.bin"]);
        db.add(&spell).unwrap();
        assert_eq!(db.by_row(3).map(|c| (c.cid, c.extra_deck())), Some((11, false)));
    }
}
