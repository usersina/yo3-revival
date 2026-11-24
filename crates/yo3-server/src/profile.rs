//! Card profiles (issue #60): the card data a server pins for its players, as one named set the rules engine, the duel
//! check, the deck placement and the game client all read.
//!
//! The `official` profile is the game's own cards. The `anime` profile (the `anime-cards` setting) gives each card of
//! `engine/anime-cards.json` marked `mapped` the data of its anime version: the engine plays its effects (that is
//! `yo3-engine`'s side), and this module says what the client must show for it: kind (Normal, Effect, Spell, Trap, ...),
//! the Spell/Trap kind, level, attribute, race, ATK and DEF, effect text, and whether it is a Main or an Extra Deck card.
//!
//! The client keeps every card in a `CARD_Prop` row of two words, `a` and `b` (`yo3_data::Card::words`). A profile
//! rewrites the bits it owns and keeps the others (the Konami id, the artwork number): [`Profile::build`]. Its
//! [client file](Profile::render) (format `yo3-card-profile 1`, below) is what the connection DLL reads at start to
//! rewrite the client's own card table in memory (`tools/mod/`), served at `/card-profile` to the signed-in launcher and
//! written by `yo3-server card-profile FILE`. Evidence for the row layout: `docs/client-internals.md` (Card data);
//! each mapping below was checked against all 3,100 monsters and 1,700 Spells and Traps of the client's table
//! (`tests/card_profile.rs`).
//!
//! ```text
//! yo3-card-profile 1
//! profile anime
//! hash 0123456789abcdef            (FNV-1a 64 of every line after this one)
//! card <row> <cid> <a hex> <b hex> <genre hex64> <atk> <def>   (atk and def: a number, or ? )
//! text <row> <effect text as a JSON string>
//! art <cid>
//! ```

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use yo3_data::Card;

/// The format's first line.
pub const MAGIC: &str = "yo3-card-profile 1";
/// The first line of an anime card's text in the game (Edit Deck, the Card Album, a duel's card panel), so a player sees
/// which cards play as their anime versions.
pub const ANIME_TAG: &str = "[Anime]";
/// The line above a card that plays as its manga version (the map's label "Manga": the Egyptian and Wicked Gods).
pub const MANGA_TAG: &str = "[Manga]";
/// The longest card text the connection DLL takes, in UTF-16 units (`YO3_PROFILE_TEXT_CHARS` in
/// `tools/mod/card-profile-logic.h`): a longer one is skipped and the game shows its own text (anime Ra's, 2406, was
/// skipped at 2048 until v0.1.12).
pub const CLIENT_TEXT_CHARS: usize = 4096;
/// The profile of the game's own cards: nothing is rewritten.
pub const OFFICIAL: &str = "official";
/// The profile of the reviewed anime card versions (`engine/anime-cards.json`).
pub const ANIME: &str = "anime";

// EDOPro's card type flags, as the unofficial database has them.
const T_MONSTER: u32 = 0x1;
const T_SPELL: u32 = 0x2;
const T_TRAP: u32 = 0x4;
const T_EFFECT: u32 = 0x20;
const T_FUSION: u32 = 0x40;
const T_RITUAL: u32 = 0x80;
const T_SPIRIT: u32 = 0x200;
const T_UNION: u32 = 0x400;
const T_GEMINI: u32 = 0x800;
const T_TUNER: u32 = 0x1000;
const T_SYNCHRO: u32 = 0x2000;
const T_TOKEN: u32 = 0x4000;
const T_QUICKPLAY: u32 = 0x10000;
const T_CONTINUOUS: u32 = 0x20000;
const T_EQUIP: u32 = 0x40000;
const T_FIELD: u32 = 0x80000;
const T_COUNTER: u32 = 0x100000;
const T_TOON: u32 = 0x400000;
/// Xyz and Link: the client has neither, and a card of those types is never mapped.
const T_XYZ: u32 = 0x800000;
const T_LINK: u32 = 0x4000000;

/// EDOPro's races and attributes in the client's order: its number is the position here plus one (the same lists
/// `translate` uses for the duel's announcements).
const RACES: [u32; 22] = [
    0x2000, 0x10, 0x8, 0x80, 0x40000, 0x100, 0x20, 0x20000, 0x10000, 0x800, 0x4000, 0x8000, 0x400, 0x40, 0x1, 0x200, 0x4,
    0x2, 0x1000, 0x80000, 0x100000, 0x200000,
];
const ATTRIBUTES: [u32; 7] = [0x10, 0x20, 0x2, 0x4, 0x1, 0x8, 0x40];

/// `a`'s ATK and DEF fields: 9 bits each, in tens; all ones is "?".
const STAT_UNKNOWN: u32 = 0x1FF;
const MASK_STAT: u32 = 0x1FF << 14 | 0x1FF << 23;
/// `b`'s kind (bits 3-7), Spell/Trap kind (16-18), attribute (8-11), level (12-15) and race (19-23).
const MASK_KIND: u32 = 0x1F << 3;
const MASK_SUB: u32 = 0x7 << 16;
const MASK_ATTRIBUTE: u32 = 0xF << 8;
const MASK_LEVEL: u32 = 0xF << 12;
const MASK_RACE: u32 = 0x1F << 19;

/// The client's kind (`b` bits 3-7) of a card with these EDOPro type flags: 0 Normal, 1 Effect, 2-3 Fusion (Normal,
/// Effect), 4-5 Ritual, 6 Toon, 7 Spirit, 8 Union, 9 Gemini, 10 Token, 13 Spell, 14 Trap, 15-16 Tuner (Normal, Effect),
/// 17-19 Synchro (Normal, Effect, Tuner). None for a type the client has no kind for (Xyz, Link, Pendulum...).
pub fn client_kind(ty: u32) -> Option<u32> {
    if ty & T_SPELL != 0 { return Some(13) }
    if ty & T_TRAP != 0 { return Some(14) }
    if ty & (T_XYZ | T_LINK | 0x1000000) != 0 || ty & T_MONSTER == 0 { return None }
    let effect = ty & T_EFFECT != 0;
    Some(match () {
        _ if ty & T_TOKEN != 0 => 10,
        _ if ty & T_TOON != 0 => 6,
        _ if ty & T_SYNCHRO != 0 => if ty & T_TUNER != 0 { 19 } else if effect { 18 } else { 17 },
        _ if ty & T_FUSION != 0 => if effect { 3 } else { 2 },
        _ if ty & T_RITUAL != 0 => if effect { 5 } else { 4 },
        _ if ty & T_SPIRIT != 0 => 7,
        _ if ty & T_UNION != 0 => 8,
        _ if ty & T_GEMINI != 0 => 9,
        _ if ty & T_TUNER != 0 => if effect { 16 } else { 15 },
        _ => u32::from(effect),
    })
}

/// The client's Spell/Trap kind (`b` bits 16-18): 0 Normal, 1 Counter (Trap), 2 Field, 3 Equip, 4 Continuous, 5 Quick-Play,
/// 6 Ritual (Spell). None for a monster.
pub fn client_sub(ty: u32) -> Option<u32> {
    if ty & (T_SPELL | T_TRAP) == 0 { return None }
    Some(match () {
        _ if ty & T_TRAP != 0 && ty & T_COUNTER != 0 => 1,
        _ if ty & T_FIELD != 0 => 2,
        _ if ty & T_EQUIP != 0 => 3,
        _ if ty & T_CONTINUOUS != 0 => 4,
        _ if ty & T_QUICKPLAY != 0 => 5,
        _ if ty & T_SPELL != 0 && ty & T_RITUAL != 0 => 6,
        _ => 0,
    })
}

/// Whether the client's deck editor puts a card of this kind in the Extra Deck (Fusion and Synchro Monsters).
pub fn extra_kind(kind: u32) -> bool {
    matches!(kind, 2 | 3 | 17..=19)
}

/// The client's race number of EDOPro's RACE_* bits (None for a race the client has no name for).
pub fn client_race(bits: u32) -> Option<u32> {
    RACES.iter().position(|&r| bits & r != 0).map(|i| i as u32 + 1)
}

/// The client's attribute number of EDOPro's ATTRIBUTE_* bits.
pub fn client_attribute(bits: u32) -> Option<u32> {
    ATTRIBUTES.iter().position(|&a| bits & a != 0).map(|i| i as u32 + 1)
}

/// An ATK or DEF: a number, or "?".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Value(u32),
    Unknown,
}

impl Stat {
    /// The database's number: below zero is "?" (-2 in the unofficial database, -1 elsewhere).
    pub fn of(value: i64) -> Stat {
        u32::try_from(value).map_or(Stat::Unknown, Stat::Value)
    }

    /// What the row's 9-bit field holds: tens rounded down, at most 5100, all ones for "?".
    fn field(self) -> u32 {
        match self {
            Stat::Unknown => STAT_UNKNOWN,
            Stat::Value(v) => (v / 10).min(STAT_UNKNOWN - 1),
        }
    }

    /// The number the duel's card info carries: the value, or the 5110 a "?" row gives (`yo3_data::Card::atk`).
    pub fn shown(self) -> u32 {
        match self {
            Stat::Unknown => STAT_UNKNOWN * 10,
            Stat::Value(v) => v,
        }
    }

    /// Whether the row's 9-bit field cannot say this exactly (not a multiple of ten, or above 5100): the connection DLL
    /// then answers the client's ATK/DEF getters with the number itself.
    pub fn needs_exact(self) -> bool {
        matches!(self, Stat::Value(v) if v % 10 != 0 || v > 5100)
    }

    fn render(self) -> String {
        match self {
            Stat::Unknown => "?".into(),
            Stat::Value(v) => v.to_string(),
        }
    }
}

/// What the engine's database says of an anime card (`datas` and `texts` of `cards-unofficial.cdb`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimeData {
    pub code: u32,
    pub ty: u32,
    pub atk: i64,
    pub def: i64,
    pub level: u32,
    pub race: u32,
    pub attribute: u32,
    pub text: String,
    /// The line the game's text panel shows above it: [`ANIME_TAG`], or [`MANGA_TAG`] for a manga version.
    pub tag: &'static str,
    /// What the anime version changes from the game's card (the map's `differs`: `level`, `atk`, `def`, `race`, `attribute`,
    /// `kind`...): what the duel check holds the engine to.
    pub differs: Vec<String>,
}

/// One card's row as the profile has it.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub row: u16,
    pub cid: u16,
    /// The anime card's passcode in the engine's database.
    pub anime: u32,
    pub words: [u32; 2],
    /// The row before the profile (to tell what changed).
    pub original: [u32; 2],
    pub genre: u64,
    pub atk: Stat,
    pub def: Stat,
    pub text: Option<String>,
    /// What the profile changes from the client's own row (see [`AnimeData::differs`]).
    pub differs: Vec<String>,
}

impl Target {
    /// The client's kind of the card as the profile has it.
    pub fn kind(&self) -> u32 {
        self.words[1] >> 3 & 0x1F
    }
    pub fn level(&self) -> u32 {
        self.words[1] >> 12 & 0xF
    }
    pub fn attribute(&self) -> u32 {
        self.words[1] >> 8 & 0xF
    }
    pub fn race(&self) -> u32 {
        self.words[1] >> 19 & 0x1F
    }
    /// Whether the card belongs in the Extra Deck as the profile has it.
    pub fn extra_deck(&self) -> bool {
        extra_kind(self.kind())
    }
    /// What the duel's card info shows: the profile's data for the Konami id.
    pub fn stats(&self) -> crate::translate::CardStats {
        crate::translate::CardStats {
            cid: self.cid,
            atk: self.atk.shown(),
            def: self.def.shown(),
            level: self.level(),
            attribute: self.attribute(),
            race: self.race(),
        }
    }
}

/// What a card of the profile must be in a duel's monster zone, as the profile has it: the duel check compares the rules
/// engine's value of every face-up monster the first time it is seen with it (issue #60).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expect {
    pub level: u32,
    pub atk: Stat,
    pub def: Stat,
    /// The client's race and attribute numbers (None: the data has no number for it).
    pub race: Option<u32>,
    pub attribute: Option<u32>,
    /// Which of them are judged.
    pub judge: Judge,
}

/// The properties of a profile card the duel check holds the engine to. Only what the profile changes from the game's card is
/// judged: the rest is the client's own data, equal to the engine's by `card_data.rs`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Judge {
    pub level: bool,
    pub atk: bool,
    pub def: bool,
    pub race: bool,
    pub attribute: bool,
}

impl Judge {
    /// Everything.
    pub fn all() -> Judge {
        Judge { level: true, atk: true, def: true, race: true, attribute: true }
    }

    /// The properties named by a profile card's `differs`: a card whose kind changes (a Trap that is a Monster) has them all new.
    pub fn of(differs: &[String]) -> Judge {
        let has = |name: &str| differs.iter().any(|d| d == name || d == "kind");
        Judge { level: has("level"), atk: has("atk"), def: has("def"), race: has("race"), attribute: has("attribute") }
    }
}

impl Expect {
    /// What the engine's database says of the anime card.
    pub fn of_anime(data: &AnimeData) -> Expect {
        let judge = if data.ty & T_MONSTER != 0 { Judge::of(&data.differs) } else { Judge::default() };
        Expect { level: data.level.min(15), atk: Stat::of(data.atk), def: Stat::of(data.def), race: client_race(data.race), attribute: client_attribute(data.attribute), judge }
    }

    /// What the profile's row says (the client's own numbers). A Spell or Trap (kind 13, 14) has no stats to judge.
    pub fn of_target(t: &Target) -> Expect {
        let judge = if matches!(t.kind(), 13 | 14) { Judge::default() } else { Judge::of(&t.differs) };
        Expect { level: t.level(), atk: t.atk, def: t.def, race: Some(t.race()), attribute: Some(t.attribute()), judge }
    }

    /// The differences between the profile's ATK and DEF and an engine monster on the field (`state`'s card: base_attack,
    /// base_defense), as text: what the card's script makes the base stats, since the core reports the base with the card's
    /// own set-base effects applied. The core reports "?" as a negative number or 0. Level, race and attribute are not judged
    /// here: on the field they are the effects' as much as the card's (Zombie World, Level Limit, Glow-Up Bulb); see
    /// [`card_problems`](Self::card_problems).
    pub fn problems(&self, engine: &serde_json::Value) -> Vec<String> {
        let signed = |key: &str| engine[key].as_i64().map(|v| if v > i64::from(i32::MAX) { v - (1 << 32) } else { v });
        let stat = |want: Stat, key: &str| match (want, signed(key)) {
            (_, None) => None,
            (Stat::Unknown, Some(v)) if v <= 0 => None,
            (Stat::Unknown, Some(v)) => Some(format!("{key} {v}, the profile \"?\"")),
            (Stat::Value(w), Some(v)) if i64::from(w) == v => None,
            (Stat::Value(w), Some(v)) => Some(format!("{key} {v}, the profile {w}")),
        };
        let mut out: Vec<String> = vec![];
        if self.judge.atk { out.extend(stat(self.atk, "base_attack")); }
        if self.judge.def { out.extend(stat(self.def, "base_defense")); }
        out
    }

    /// The differences between the profile's Level, race and attribute and the card as the rules engine has it (the engine's
    /// card data: `level`, `race`, `attribute`), as text: the engine plays what the profile shows.
    pub fn card_problems(&self, data: &serde_json::Value) -> Vec<String> {
        let mut out = vec![];
        if self.judge.level && let Some(level) = data["level"].as_u64().map(|l| l & 0xFF).filter(|&l| l > 0) && level.min(15) != u64::from(self.level) {
            out.push(format!("level {level}, the profile {}", self.level));
        }
        let number = |key: &str, map: fn(u32) -> Option<u32>| data[key].as_u64().and_then(|b| map(b as u32));
        for (key, judged, want, shown) in [("race", self.judge.race, self.race, number("race", client_race)), ("attribute", self.judge.attribute, self.attribute, number("attribute", client_attribute))] {
            if let (true, Some(want), Some(shown)) = (judged, want, shown) && want != shown {
                out.push(format!("{key} {shown} (the client's number), the profile {want}"));
            }
        }
        out
    }
}

/// A named set of cards the client shows differently from its own files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Profile {
    pub name: String,
    /// By CARD_Prop row.
    pub targets: BTreeMap<u16, Target>,
}

impl Profile {
    /// The game's own cards: no card is rewritten.
    pub fn official() -> Profile {
        Profile { name: OFFICIAL.into(), targets: BTreeMap::new() }
    }

    pub fn is_official(&self) -> bool {
        self.targets.is_empty()
    }

    /// The profile of `anime` (by Konami id) over the client's `cards`: every row of each card (all its artworks)
    /// gets the anime version's kind, Spell/Trap kind, level, attribute, race, ATK, DEF and text; the Konami id and
    /// artwork bits stay. A card whose anime data the client's kinds cannot say (Xyz, Link, a race it has no name
    /// for) is left out, and so is named in the second list.
    pub fn build<'a>(name: &str, cards: impl IntoIterator<Item = &'a Card>, anime: &HashMap<u16, AnimeData>) -> (Profile, Vec<u16>) {
        let mut targets = BTreeMap::new();
        let mut skipped = vec![];
        for card in cards {
            let Some(data) = anime.get(&card.cid) else { continue };
            let Some(kind) = client_kind(data.ty) else {
                skipped.push(card.cid);
                continue;
            };
            let [mut a, mut b] = card.words;
            let monster = data.ty & T_MONSTER != 0;
            b = b & !MASK_KIND | kind << 3;
            let (mut atk, mut def) = (Stat::of(0), Stat::of(0));
            if monster {
                let (Some(race), Some(attribute)) = (client_race(data.race), client_attribute(data.attribute)) else {
                    skipped.push(card.cid);
                    continue;
                };
                (atk, def) = (Stat::of(data.atk), Stat::of(data.def));
                a = a & !MASK_STAT | atk.field() << 14 | def.field() << 23;
                b = b & !(MASK_ATTRIBUTE | MASK_LEVEL | MASK_RACE) | attribute << 8 | data.level.min(15) << 12 | race << 19;
            } else {
                // A Spell or Trap: no stats, and its own Spell/Trap kind.
                b = b & !MASK_SUB | client_sub(data.ty).unwrap_or(0) << 16;
                // The type line prints the attribute and race too: a Spell is attribute 8, race 23, a Trap 9 and 24.
                let class = if data.ty & T_SPELL != 0 { 0 } else { 1 };
                b = b & !(MASK_ATTRIBUTE | MASK_RACE) | (8 + class) << 8 | (23 + class) << 19;
                a &= !MASK_STAT;
            }
            // The game's text panel names an anime card as one: its own line above the anime text.
            let text = (!data.text.is_empty()).then(|| format!("{}\n{}", data.tag, data.text));
            targets.insert(card.row, Target { row: card.row, cid: card.cid, anime: data.code, words: [a, b], original: card.words, genre: card.genre, atk, def, text, differs: data.differs.clone() });
        }
        skipped.sort_unstable();
        skipped.dedup();
        (Profile { name: name.into(), targets }, skipped)
    }

    pub fn target(&self, row: u16) -> Option<&Target> {
        self.targets.get(&row)
    }

    /// The Konami ids the profile covers.
    pub fn cids(&self) -> std::collections::BTreeSet<u16> {
        self.targets.values().map(|t| t.cid).collect()
    }

    /// Whether a row belongs in the Extra Deck: the profile's say for its cards, else `default` (the game's own).
    pub fn extra_deck(&self, row: u16, default: bool) -> bool {
        self.targets.get(&row).map_or(default, Target::extra_deck)
    }

    /// The lines after the hash: what it covers.
    fn body(&self) -> String {
        let mut out = String::new();
        let mut cids = std::collections::BTreeSet::new();
        for t in self.targets.values() {
            out += &format!("card {} {} {:08x} {:08x} {:016x} {} {}\n", t.row, t.cid, t.words[0], t.words[1], t.genre, t.atk.render(), t.def.render());
            cids.insert(t.cid);
        }
        for t in self.targets.values() {
            if let Some(text) = &t.text {
                out += &format!("text {} {}\n", t.row, serde_json::to_string(text).expect("a string is JSON"));
            }
        }
        for cid in cids {
            out += &format!("art {cid}\n");
        }
        out
    }

    /// The hash of the profile's content (16 hex digits): two servers with the same data give the same one.
    pub fn hash(&self) -> String {
        fnv(&self.body())
    }

    /// The client file: empty for the official profile (nothing to change).
    pub fn render(&self) -> String {
        if self.is_official() { return String::new() }
        let body = self.body();
        format!("{MAGIC}\nprofile {}\nhash {}\n{body}", self.name, fnv(&body))
    }

    /// What a launch loaded: the profile's name and hash of a client file (`render`'s first lines).
    pub fn identify(file: &str) -> Option<(String, String)> {
        let mut lines = file.lines().filter(|l| !l.starts_with('#'));
        (lines.next()? == MAGIC).then_some(())?;
        let name = lines.next()?.strip_prefix("profile ")?.to_string();
        let hash = lines.next()?.strip_prefix("hash ")?.to_string();
        Some((name, hash))
    }

    /// The profile `name` for the client's `cards`, from the reviewed map and the anime database of the engine
    /// directory `engine_dir` (`anime-cards.json`, `vendor/BabelCDB/cards-unofficial.cdb`). `official`, or an unknown
    /// name, is the game's own cards.
    pub fn load<'a>(name: &str, cards: impl IntoIterator<Item = &'a Card>, engine_dir: &Path) -> Result<Profile, String> {
        if name != ANIME { return Ok(Profile::official()) }
        let anime = read_anime(engine_dir)?;
        let (profile, skipped) = Profile::build(ANIME, cards, &anime);
        if !skipped.is_empty() {
            tracing::warn!("anime profile: no client kind for Konami ids {skipped:?}; they keep the game's data");
        }
        Ok(profile)
    }
}

/// The card data a duel, a deck check and a deck's placement read, as one profile has it: the server's own maps with the
/// profile's cards over them.
#[derive(Clone, Debug, Default)]
pub struct CardView {
    pub profile: std::sync::Arc<Profile>,
    /// Each CARD_Prop row's stats for the duel's card info.
    pub cards: HashMap<u16, crate::translate::CardStats>,
    /// Each Konami id's client kind (13 a Spell, 14 a Trap...).
    pub card_kinds: HashMap<u16, u32>,
    /// The CARD_Prop rows that belong in the Extra Deck.
    pub extra_deck_rows: std::collections::HashSet<u16>,
    /// The Tuners' Konami ids.
    pub tuners: std::collections::HashSet<u16>,
}

impl CardView {
    /// What the duel check expects of the profile's cards, by CARD_Prop row.
    pub fn expectations(&self) -> HashMap<u16, Expect> {
        self.profile.targets.values().map(|t| (t.row, Expect::of_target(t))).collect()
    }

    /// The view of `profile` over `base` (the game's own).
    pub fn over(base: &CardView, profile: std::sync::Arc<Profile>) -> CardView {
        let mut view = CardView { profile: profile.clone(), ..base.clone() };
        for t in profile.targets.values() {
            view.cards.insert(t.row, t.stats());
            view.card_kinds.insert(t.cid, t.kind());
            if t.extra_deck() { view.extra_deck_rows.insert(t.row); } else { view.extra_deck_rows.remove(&t.row); }
            if yo3_data::tuner_kind(t.kind()) { view.tuners.insert(t.cid); } else { view.tuners.remove(&t.cid); }
        }
        view
    }
}

/// The profiles a server can pin, and what each launch loaded.
#[derive(Default)]
pub struct Profiles {
    /// The anime profile over the game's cards (None: its files are missing, or the game's data is).
    pub anime: Option<std::sync::Arc<CardView>>,
    /// The profile each signed-in launch's game fetched at start (name, hash), by session: the client reads its card table
    /// once, so a game that loaded another profile than the pinned one, or none, is refused every duel (`current_for`).
    launches: std::sync::Mutex<BTreeMap<crate::auth::Identity, (String, String)>>,
}

impl Profiles {
    /// The anime profile as given (a test's, or a host's built elsewhere).
    pub fn with_anime(view: CardView) -> Profiles {
        Profiles { anime: Some(std::sync::Arc::new(view)), launches: Default::default() }
    }

    /// The anime profile from the engine folder and the game's cards; failures are logged and leave the game's own.
    pub fn load<'a>(base: &CardView, cards: impl IntoIterator<Item = &'a Card>, engine_dir: Option<&Path>) -> Profiles {
        let anime = engine_dir.and_then(|dir| match crate::engine::anime_problem(dir) {
            Some(problem) => { tracing::info!("anime profile not built: {problem}"); None }
            None => match Profile::load(ANIME, cards, dir) {
                Ok(profile) if !profile.is_official() => Some(std::sync::Arc::new(CardView::over(base, std::sync::Arc::new(profile)))),
                Ok(_) => None,
                Err(e) => { tracing::warn!("anime profile not built: {e}"); None }
            },
        });
        Profiles { anime, launches: Default::default() }
    }

    /// The profile named by the `anime-cards` setting, if it can be had: its view. None is the game's own.
    pub fn selected(&self, anime_cards: bool) -> Option<&std::sync::Arc<CardView>> {
        self.anime.as_ref().filter(|_| anime_cards)
    }

    /// The hash of the selected profile, empty for the game's own: what discovery tells the launchers.
    pub fn current_hash(&self, anime_cards: bool) -> String {
        self.selected(anime_cards).map(|v| v.profile.hash()).unwrap_or_default()
    }

    /// The client file of the selected profile (empty for the game's own).
    pub fn file(&self, anime_cards: bool) -> String {
        self.selected(anime_cards).map_or_else(String::new, |v| v.profile.render())
    }

    /// What the game of `who` (a signed-in session) fetched: the profile's name and hash (the game's own is "official" and
    /// no hash). Only the game's own fetch counts, never the launcher's (`web/card-profile?for=decks`): a game whose
    /// connection DLL has no profile support fetches nothing and so proves nothing.
    pub fn note_launch(&self, who: crate::auth::Identity, loaded: (String, String)) {
        let mut launches = self.launches.lock().unwrap_or_else(|e| e.into_inner());
        launches.insert(who, loaded);
        // Sessions are numbered upward and end with their game: the oldest records go first.
        while launches.len() > MAX_LAUNCHES { launches.pop_first(); }
    }

    /// Whether the game of `who` runs the profile now selected, the only state in which it may duel: a game that loaded
    /// another profile (the host changed the setting, or the server restarted with another one) does not, and neither does
    /// one that fetched nothing while a profile is pinned (it started before the setting was turned on, or without profile
    /// support). With the game's own cards pinned every game is current, except one that loaded a profile. So standard mode
    /// works for every client, and a profile only ever shuts out games that cannot show it.
    pub fn current_for(&self, who: crate::auth::Identity, anime_cards: bool) -> bool {
        let want = self.selected(anime_cards).map(|v| v.profile.hash()).unwrap_or_default();
        let has = self.launches.lock().unwrap_or_else(|e| e.into_inner()).get(&who).map(|(_, h)| h.clone()).unwrap_or_default();
        want == has
    }
}

/// How many launches `Profiles` remembers.
const MAX_LAUNCHES: usize = 4096;

fn fnv(text: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The mapped cards of `engine/anime-cards.json` with their anime data from the unofficial database: Konami id -> data.
pub fn read_anime(engine_dir: &Path) -> Result<HashMap<u16, AnimeData>, String> {
    let file = engine_dir.join("anime-cards.json");
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&file).map_err(|e| format!("open '{}': {e}", file.display()))?)
        .map_err(|e| format!("{}: {e}", file.display()))?;
    let cdb = engine_dir.join("vendor/BabelCDB/cards-unofficial.cdb");
    let db = rusqlite::Connection::open_with_flags(&cdb, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| format!("open '{}': {e}", cdb.display()))?;
    let mut out = HashMap::new();
    for card in json["cards"].as_array().into_iter().flatten().filter(|c| c["status"] == "mapped") {
        let (Some(cid), Some(code)) = (card["cid"].as_u64(), card["anime"].as_u64()) else { continue };
        let tag = if card["label"] == "Manga" { MANGA_TAG } else { ANIME_TAG };
        // A corrected record (an anime card whose upstream entry is a placeholder) is the card's data, not the database's row.
        if let Some(r) = json["records"].as_array().into_iter().flatten().find(|r| r["code"].as_u64() == Some(code)) {
            let num = |key: &str| r[key].as_i64().ok_or_else(|| format!("{}: record {code} lacks \"{key}\"", file.display()));
            out.insert(cid as u16, AnimeData {
                code: code as u32, ty: num("type")? as u32, atk: num("atk")?, def: num("def")?, level: (num("level")? & 255) as u32,
                race: num("race")? as u32, attribute: num("attribute")? as u32, text: r["text"].as_str().unwrap_or_default().to_string(), tag,
                differs: card["differs"].as_array().into_iter().flatten().filter_map(|d| d.as_str().map(String::from)).collect(),
            });
            continue;
        }
        let row = db.query_row(
            "select d.type, d.atk, d.def, d.level & 255, d.race, d.attribute, t.desc from datas d join texts t on t.id = d.id where d.id = ?",
            [code as i64],
            |r| Ok(AnimeData {
                code: code as u32, ty: r.get::<_, i64>(0)? as u32, atk: r.get(1)?, def: r.get(2)?, level: r.get::<_, i64>(3)? as u32,
                race: r.get::<_, i64>(4)? as u32, attribute: r.get::<_, i64>(5)? as u32, text: r.get::<_, Option<String>>(6)?.unwrap_or_default(), tag,
                differs: card["differs"].as_array().into_iter().flatten().filter_map(|d| d.as_str().map(String::from)).collect(),
            }),
        );
        match row {
            Ok(data) => { out.insert(cid as u16, data); }
            Err(e) => return Err(format!("anime card {code} ({}): {e}", card["name"].as_str().unwrap_or("?"))),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(row: u16, cid: u16, [atk, def]: [u32; 2], [kind, level, attribute, race]: [u32; 4], art: u32) -> Card {
        let a = u32::from(cid) | (atk / 10) << 14 | (def / 10) << 23;
        let b = kind << 3 | attribute << 8 | level << 12 | race << 19 | art << 27;
        Card { row, cid, atk, def, level, attribute, race, kind, art, words: [a, b], genre: 0xAB, name: String::new(), desc: String::new() }
    }

    fn anime(ty: u32, atk: i64, def: i64, level: u32, race: u32, attribute: u32) -> AnimeData {
        AnimeData { code: 511000000, ty, atk, def, level, race, attribute, text: "Anime text.".into(), tag: ANIME_TAG, differs: vec![] }
    }

    #[test]
    fn the_expectation_judges_the_engine_monster_against_the_profile() {
        use serde_json::json;
        // Wiraqocha Rasca: Level 6, 1/1, Earth Fiend... as the profile has it (race 0x8 Fiend = the client's 3, attribute 0x10 DARK = 1).
        let want = Expect { level: 6, atk: Stat::Value(1), def: Stat::Value(1), race: Some(3), attribute: Some(1), judge: Judge::all() };
        let ok = json!({"level": 6, "base_attack": 1, "base_defense": 1, "race": 8, "attribute": 16});
        assert!(want.problems(&ok).is_empty() && want.card_problems(&ok).is_empty(), "{:?}", want.problems(&ok));
        let wrong = json!({"level": 5, "base_attack": 100, "base_defense": 1, "race": 1, "attribute": 32});
        assert_eq!(want.problems(&wrong).len(), 1, "the field is judged by ATK and DEF: {:?}", want.problems(&wrong));
        assert_eq!(want.card_problems(&wrong).len(), 3, "the engine's card by Level, race and attribute: {:?}", want.card_problems(&wrong));
        // Only what the profile changes is judged: a card whose profile changes the ATK alone is judged by its ATK.
        let atk_only = Expect { judge: Judge::of(&["atk".to_string()]), ..want };
        assert_eq!((atk_only.problems(&wrong).len(), atk_only.card_problems(&wrong).len()), (1, 0));
        assert_eq!(Expect { judge: Judge::of(&["kind".to_string()]), ..want }.card_problems(&wrong).len(), 3, "a card whose kind changes is new in everything");
        assert!(Expect { judge: Judge::default(), ..want }.problems(&wrong).is_empty());
        // "?" is a negative number or 0 in the core; a number is not "?".
        let unknown = Expect { atk: Stat::Unknown, ..want };
        assert!(unknown.problems(&json!({"base_attack": 4294967294u64})).is_empty());
        assert_eq!(unknown.problems(&json!({"base_attack": 300})).len(), 1);
        // Missing fields and Level 0 are not judged.
        assert!(want.problems(&json!({"level": 0})).is_empty() && want.card_problems(&json!({"level": 0})).is_empty());
    }

    #[test]
    fn a_fusion_becomes_a_main_deck_normal_monster_and_keeps_its_id_art_and_genre() {
        // Flame Swordsman: the game's Fusion (kind 3, Level 5, 1800/1600, Fire, Warrior); the anime card is a Normal Monster.
        let game = [card(100, 4021, [1800, 1600], [3, 5, 2, 2], 1), card(101, 4021, [1800, 1600], [3, 5, 2, 2], 2)];
        let data = HashMap::from([(4021, anime(0x11, 1800, 1600, 5, 0x1, 0x4))]);
        let (profile, skipped) = Profile::build("anime", &game, &data);
        assert!(skipped.is_empty());
        let t = profile.target(101).unwrap();
        assert_eq!((t.kind(), t.level(), t.attribute(), t.race()), (0, 5, 4, 15), "Normal, Level 5, Fire, Warrior");
        assert!(!t.extra_deck() && !profile.extra_deck(101, true) && !profile.extra_deck(7, false) && profile.extra_deck(7, true), "the profile decides for its cards, the default for the others");
        assert_eq!(t.words[0] & 0x3FFF, 4021, "the Konami id stays");
        assert_eq!(t.words[1] >> 27 & 7, 2, "the artwork stays");
        assert_eq!(t.genre, 0xAB);
        assert_eq!((t.atk, t.def), (Stat::Value(1800), Stat::Value(1600)));
    }

    #[test]
    fn a_trap_becomes_a_spell_with_its_kind() {
        let game = [card(5, 4849, [0, 0], [14, 0, 0, 0], 0)];
        let data = HashMap::from([(4849, anime(0x10002, 0, 0, 0, 0, 0))]);
        let (profile, _) = Profile::build("anime", &game, &data);
        let t = profile.target(5).unwrap();
        assert_eq!(t.kind(), 13, "a Spell");
        assert_eq!(t.words[1] >> 16 & 7, 5, "a Quick-Play Spell");
        assert_eq!((t.words[1] >> 8 & 15, t.words[1] >> 19 & 31), (8, 23), "the type line reads a Spell's attribute and race, not the Trap's");
    }

    #[test]
    fn atk_that_the_row_cannot_hold_is_kept_exactly_and_a_question_mark_is_all_ones() {
        let game = [card(1, 4000, [100, 100], [1, 4, 1, 1], 0)];
        let data = HashMap::from([(4000, anime(0x21, 1, -2, 4, 0x1, 0x1))]);
        let (profile, _) = Profile::build("anime", &game, &data);
        let t = profile.target(1).unwrap();
        assert_eq!((t.atk, t.def), (Stat::Value(1), Stat::Unknown));
        assert!(t.atk.needs_exact() && !t.def.needs_exact());
        assert_eq!(t.words[0] >> 14 & 0x1FF, 0, "the 9-bit field rounds down");
        assert_eq!(t.words[0] >> 23 & 0x1FF, 0x1FF);
        assert_eq!((t.stats().atk, t.stats().def), (1, 5110));
        assert!(profile.render().contains("card 1 4000 "), "{}", profile.render());
        assert!(profile.render().contains(" 1 ?\n"));
    }

    #[test]
    fn the_client_file_is_stable_and_identifies_itself() {
        let game = [card(1, 4000, [100, 100], [1, 4, 1, 1], 0)];
        let data = HashMap::from([(4000, anime(0x21, 100, 100, 4, 0x1, 0x1))]);
        let (profile, _) = Profile::build("anime", &game, &data);
        let file = profile.render();
        assert!(file.starts_with("yo3-card-profile 1\nprofile anime\nhash "), "{file}");
        assert!(file.contains("text 1 \"[Anime]\\nAnime text.\"\nart 4000\n"), "the anime line first: {file}");
        assert_eq!(Profile::identify(&file), Some(("anime".into(), profile.hash())));
        assert_eq!(Profile::build("anime", &game, &data).0.render(), file);
        assert_eq!(Profile::official().render(), "", "the official profile is an empty file");
    }

    #[test]
    fn kinds_follow_the_clients_own_table() {
        assert_eq!([0x11, 0x21, 0x41, 0x61, 0x81, 0xA1, 0x221, 0x421, 0x821, 0x1011, 0x1021, 0x2001, 0x2021, 0x3021, 0x4011, 0x400021, 0x2, 0x4, 0x82].map(|t| client_kind(t).unwrap()),
            [0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 18, 19, 10, 6, 13, 14, 13]);
        assert_eq!(client_kind(0x800021), None, "Xyz has no kind");
        assert_eq!([0x2, 0x4, 0x82, 0x10002, 0x20002, 0x20004, 0x40002, 0x80002, 0x100004].map(|t| client_sub(t).unwrap()), [0, 0, 6, 5, 4, 4, 3, 2, 1]);
        assert_eq!((client_race(0x1), client_race(0x2), client_attribute(0x1), client_attribute(0x10)), (Some(15), Some(18), Some(5), Some(1)));
    }
}
