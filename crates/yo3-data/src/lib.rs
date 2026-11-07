//! Readers for the Yu-Gi-Oh! ONLINE 3 client's own data: `.ypk` archives, Konami's card
//! table, the UI text table, and the regulation / preset-deck / booster-pack records the server hands out.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use yo3_proto::{Message, Value};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{path}: {source}")]
    Proto { path: String, source: yo3_proto::Error },
    #[error("missing {0}")]
    Missing(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The default install location under Wine.
pub fn default_game_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3")
}

/// One file inside a `.ypk` archive.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub method: u16,
    pub size: u32,
    compressed: u32,
    offset: u64,
}

/// A `.ypk` archive: ZIP local headers with a `YO` signature and no central directory.
pub struct Ypk {
    path: PathBuf,
    entries: Vec<Entry>,
}

impl Ypk {
    pub fn open(path: impl AsRef<Path>) -> Result<Ypk> {
        let path = path.as_ref().to_path_buf();
        let mut r = BufReader::new(File::open(&path)?);
        let len = r.get_ref().metadata()?.len();
        let mut entries = Vec::new();
        let mut pos = 0u64;
        let mut head = [0u8; 30];
        while pos + 30 <= len {
            r.seek(SeekFrom::Start(pos))?;
            r.read_exact(&mut head)?;
            if &head[..4] != b"YO\x03\x04" {
                break;
            }
            let u16_at = |i: usize| u16::from_le_bytes([head[i], head[i + 1]]);
            let u32_at = |i: usize| u32::from_le_bytes(head[i..i + 4].try_into().unwrap());
            let (method, compressed, size) = (u16_at(8), u32_at(18), u32_at(22));
            let (name_len, extra_len) = (u16_at(26) as u64, u16_at(28) as u64);
            let mut name = vec![0; name_len as usize];
            r.read_exact(&mut name)?;
            let offset = pos + 30 + name_len + extra_len;
            entries.push(Entry { name: String::from_utf8_lossy(&name).into_owned(), method, size, compressed, offset });
            pos = offset + compressed as u64;
        }
        Ok(Ypk { path, entries })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Reads and inflates one entry; `None` if the archive has no such file.
    pub fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(e) = self.entries.iter().find(|e| e.name == name) else { return Ok(None) };
        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(e.offset))?;
        let mut raw = vec![0; e.compressed as usize];
        f.read_exact(&mut raw)?;
        if e.method == 0 {
            return Ok(Some(raw));
        }
        let mut out = Vec::with_capacity(e.size as usize);
        flate2::read::DeflateDecoder::new(&raw[..]).read_to_end(&mut out)?;
        Ok(Some(out))
    }

    fn read_required(&self, name: &str) -> Result<Vec<u8>> {
        self.read(name)?.ok_or_else(|| Error::Missing(name.into()))
    }

    fn records(&self, dir: &str, exts: &[&str]) -> Result<Vec<Message>> {
        let mut out = Vec::new();
        for e in self.entries.iter().filter(|e| e.name.starts_with(dir) && exts.iter().any(|x| e.name.ends_with(x))) {
            let blob = self.read_required(&e.name)?;
            out.push(yo3_proto::decode_file(&blob).map_err(|source| Error::Proto { path: e.name.clone(), source })?);
        }
        Ok(out)
    }
}

/// One row of Konami's card table.
#[derive(Clone, Debug)]
pub struct Card {
    /// Row in CARD_Prop.bin; decks and packs refer to cards by row.
    pub row: u16,
    /// Konami card ID ("CID"), e.g. 4007 = Blue-Eyes White Dragon; regulations use these.
    pub cid: u16,
    pub atk: u32,
    pub def: u32,
    /// From the second word: level (bits 12-15), attribute (8-11) and race (19-23), as the duel's card info
    /// reads them (0x453740).
    pub level: u32,
    pub attribute: u32,
    pub race: u32,
    pub name: String,
}

#[derive(Default)]
pub struct CardDb {
    cards: Vec<Card>,
    by_cid: HashMap<u16, usize>,
    by_row: HashMap<u16, usize>,
}

impl CardDb {
    /// Parses CARD_Prop.bin (8 bytes per row) with names from CARD_Indx/CARD_Name.
    pub fn parse(prop: &[u8], index: &[u8], names: &[u8]) -> CardDb {
        let mut db = CardDb::default();
        for (row, p) in prop.as_chunks::<8>().0.iter().enumerate() {
            let a = u32::from_le_bytes(p[..4].try_into().unwrap());
            let b = u32::from_le_bytes(p[4..].try_into().unwrap());
            let cid = (a & 0x3FFF) as u16;
            if cid == 0 {
                continue;
            }
            let name = index
                .get(row * 8..row * 8 + 4)
                .map(|o| u32::from_le_bytes(o.try_into().unwrap()) as usize)
                .map(|off| utf16z(names.get(off..).unwrap_or_default()))
                .unwrap_or_default();
            let card = Card {
                row: row as u16,
                cid,
                atk: ((a >> 14) & 0x1FF) * 10,
                def: ((a >> 23) & 0x1FF) * 10,
                level: (b >> 12) & 0xF,
                attribute: (b >> 8) & 0xF,
                race: (b >> 19) & 0x1F,
                name,
            };
            db.by_cid.entry(cid).or_insert(db.cards.len());
            db.by_row.insert(card.row, db.cards.len());
            db.cards.push(card);
        }
        db
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn by_cid(&self, cid: u16) -> Option<&Card> {
        self.by_cid.get(&cid).map(|&i| &self.cards[i])
    }

    pub fn by_row(&self, row: u16) -> Option<&Card> {
        self.by_row.get(&row).map(|&i| &self.cards[i])
    }

    pub fn iter(&self) -> impl Iterator<Item = &Card> {
        self.cards.iter()
    }
}

fn utf16z(bytes: &[u8]) -> String {
    let units: Vec<u16> =
        bytes.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
    String::from_utf16_lossy(&units)
}

fn u16_list(msg: &Message, name: &str) -> Vec<u16> {
    msg.get(name)
        .and_then(Value::as_bytes)
        .map(|b| b.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]])).collect())
        .unwrap_or_default()
}

fn int(msg: &Message, name: &str) -> u32 {
    msg.get(name).and_then(Value::as_u32).unwrap_or(0)
}

/// A banlist ("regulation"). Its lists hold Konami card IDs.
#[derive(Clone, Debug)]
pub struct Regulation {
    pub id: u32,
    pub kind: u32,
    pub status: u32,
    pub name_id: u32,
    /// card0 = forbidden, card1 = limited, card2 = semi-limited, card3 = three copies (see `limit`).
    pub lists: [Vec<u16>; 4],
    /// Copies allowed of a card in no list ("addition_card"; 3 when absent, a guess).
    pub default_limit: u32,
    pub record: Message,
}

impl Regulation {
    /// Cards no deck may hold under any regulation (0x51A9A0; the same three the trunk zeroes).
    pub const NEVER_ALLOWED: [u16; 3] = [5280, 5434, 6276];

    pub fn forbidden(&self) -> &[u16] {
        &self.lists[0]
    }

    /// Copies of a card (Konami ID) a deck may hold, as 0x51A930 decides it: the index of the list
    /// that names the card, else the default. The list whose index is the default is skipped, so
    /// format regulations (default 0) use card3 as the cards allowed three times.
    pub fn limit(&self, cid: u16) -> u32 {
        (0..4)
            .filter(|&i| i != self.default_limit as usize)
            .find(|&i| self.lists[i].contains(&cid))
            .map_or(self.default_limit, |i| i as u32)
    }

    /// Whether a deck (its cards as Konami IDs) is legal (0x51A9A0).
    pub fn allows(&self, cids: impl IntoIterator<Item = u16>) -> bool {
        let mut counts: HashMap<u16, u32> = HashMap::new();
        for cid in cids {
            *counts.entry(cid).or_default() += 1;
        }
        counts.iter().all(|(&cid, &n)| !Self::NEVER_ALLOWED.contains(&cid) && n <= self.limit(cid))
    }
}

/// A preset deck ("recipe"). Its lists hold CARD_Prop rows.
#[derive(Clone, Debug)]
pub struct Deck {
    pub id: u32,
    pub kind: u32,
    pub main: Vec<u16>,
    pub extra: Vec<u16>,
    pub side: Vec<u16>,
    pub record: Message,
}

impl Deck {
    /// Every card of the deck (CARD_Prop rows): main, then extra, then side.
    pub fn cards(&self) -> impl Iterator<Item = u16> + '_ {
        self.main.iter().chain(&self.extra).chain(&self.side).copied()
    }
}

/// A booster pack. Its list holds CARD_Prop rows.
#[derive(Clone, Debug)]
pub struct Pack {
    pub id: u32,
    pub kind: u32,
    pub cards: Vec<u16>,
    pub record: Message,
}

/// Everything the server serves straight from the client's own files.
pub struct GameData {
    pub cards: CardDb,
    pub regulations: Vec<Regulation>,
    pub decks: Vec<Deck>,
    pub packs: Vec<Pack>,
    /// The English UI texts (data/text/txttbl_E.bin) by id: every name_id and discription_id, and the `$NNNNN` ids.
    pub texts: HashMap<u32, String>,
}

impl GameData {
    pub fn load(game_dir: &Path) -> Result<GameData> {
        let ypk = Ypk::open(game_dir.join("package/data002.ypk"))?;
        let cards = CardDb::parse(
            &ypk.read_required("data/bin#/CARD_Prop.bin")?,
            &ypk.read_required("data/bin#/CARD_Indx_E.bin")?,
            &ypk.read_required("data/bin#/CARD_Name_E.bin")?,
        );
        let mut regulations: Vec<Regulation> = ypk
            .records("data/regulation/", &[".y3r"])?
            .into_iter()
            .map(|m| Regulation {
                id: int(&m, "regulation_id"),
                kind: int(&m, "regulation_type"),
                status: int(&m, "status"),
                name_id: int(&m, "name_id"),
                lists: ["card0", "card1", "card2", "card3"].map(|k| u16_list(&m, k)),
                default_limit: m.get("addition_card").and_then(Value::as_u32).unwrap_or(3),
                record: m,
            })
            .collect();
        regulations.sort_by_key(|r| r.id);
        let mut decks: Vec<Deck> = ypk
            // One preset deck ships as data/recipe/232.y3a.
            .records("data/recipe/", &[".y3d", ".y3a"])?
            .into_iter()
            .map(|m| Deck {
                id: int(&m, "deck_id"),
                kind: int(&m, "deck_type"),
                main: u16_list(&m, "main"),
                extra: u16_list(&m, "extra"),
                side: u16_list(&m, "side"),
                record: m,
            })
            .collect();
        decks.sort_by_key(|d| d.id);
        let mut packs: Vec<Pack> = ypk
            .records("data/pack/", &[".y3a"])?
            .into_iter()
            .map(|m| Pack { id: int(&m, "pack_id"), kind: int(&m, "pack_type"), cards: u16_list(&m, "pack"), record: m })
            .collect();
        packs.sort_by_key(|p| p.id);
        let texts = parse_texts(&ypk.read_required("data/text/txttbl_E.bin")?);
        Ok(GameData { cards, regulations, decks, packs, texts })
    }

    /// A UI text by its id.
    pub fn text(&self, id: u32) -> Option<&str> {
        self.texts.get(&id).map(String::as_str)
    }

    /// The first starter preset (deck_type 1) that a regulation allows.
    pub fn starter_deck(&self, regulation_id: u32) -> Option<&Deck> {
        let regulation = self.regulations.iter().find(|r| r.id == regulation_id)?;
        let cids = |d: &Deck| d.cards().filter_map(|row| self.cards.by_row(row)).map(|c| c.cid).collect::<Vec<_>>();
        self.decks.iter().filter(|d| d.kind == 1).find(|d| regulation.allows(cids(d)))
    }

    /// The NPC opponents the NPC duel lists, three levels each (ids 120 + 3k .. 120 + 3k + 2, the
    /// k-th NPC's name being text 0x5208 + 2k, 0x4BFE34). Each has a preset deck (deck_type 3)
    /// titled text 0x5FB4 + 2 * npc_id (the deck title the NPC screen shows, 0x4BFE05); NPCs
    /// without all three decks or without a name (a blank row on the NPC screen) are left out.
    pub fn npc_ids(&self) -> Vec<u32> {
        let has = |id: u32| self.npc_deck(id).is_some();
        let named = |id: u32| self.text(NPC_NAME + 2 * ((id - FIRST_NPC) / 3)).is_some_and(|n| !n.is_empty());
        let firsts = (FIRST_NPC..).step_by(3).take_while(|&id| has(id));
        firsts.filter(|&id| has(id + 1) && has(id + 2) && named(id)).flat_map(|id| id..id + 3).collect()
    }

    /// The preset deck an NPC plays.
    pub fn npc_deck(&self, npc_id: u32) -> Option<&Deck> {
        let name_id = NPC_DECK_TITLE + 2 * npc_id;
        self.decks.iter().filter(|d| d.kind == 3).find(|d| text_int(&d.record, "name_id") == Some(name_id))
    }
}

/// The lowest selectable NPC id (0x4BFE34, DuelManager 0x50F4F0).
const FIRST_NPC: u32 = 120;
/// Text id of NPC 0's name (0x4BFE34).
const NPC_NAME: u32 = 0x5208;
/// Text id of NPC 0's deck title (0x4BFE05).
const NPC_DECK_TITLE: u32 = 0x5FB4;

/// A UI text table (data/text/txttbl_<lang>.bin): u32 count, count pairs {u32 id, u32 byte offset}, 4 bytes, then
/// UTF-16LE NUL-terminated strings from byte 8 + 8 * count. Entries pointing outside the table are left out.
pub fn parse_texts(table: &[u8]) -> HashMap<u32, String> {
    let u32_at = |at: usize| table.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let count = u32_at(0).unwrap_or(0) as usize;
    let base = 8 + 8 * count;
    let mut texts = HashMap::new();
    for i in 0..count {
        let (Some(id), Some(offset)) = (u32_at(4 + 8 * i), u32_at(8 + 8 * i)) else { break };
        let Some(rest) = table.get(base + offset as usize..) else { continue };
        let units: Vec<u16> = rest.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).take_while(|&u| u != 0).collect();
        texts.insert(id, String::from_utf16_lossy(&units));
    }
    texts
}

impl Deck {
    /// The id of its name in the UI text table (stored as text in the recipe).
    pub fn name_id(&self) -> Option<u32> {
        text_int(&self.record, "name_id")
    }
}

/// A number the client's data files store as text (preset decks' name_id).
fn text_int(msg: &Message, name: &str) -> Option<u32> {
    msg.get(name).and_then(Value::as_str).and_then(|s| s.parse().ok())
}
