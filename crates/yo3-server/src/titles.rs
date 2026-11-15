//! Titles a player earns (the item catalogue's kind 10, `data/extra/yo3pbininfo.bin`: 160 titles, each with a name and
//! a line saying what it is for). Avatar Settings lists the owned ones (their pbin bits, the player's `items`), and the
//! player wears one. BEGINNER DUELIST is every player's (`GameData::free_items`); this module decides the others the
//! server can measure. Their lines say what each rewards ("A token of friendship with Yami Yugi", "summoned many high
//! level monsters"), never how many: the numbers here are ours. Titles of the official events, the rankings and the
//! Championship wait for those (issue #16); the staff and promotion titles are granted by the host
//! (`yo3-server grant-title`).
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// What the server keeps of a player's duels for titles (`Player::counters`, by name).
pub const DUELS: &str = "duels";
pub const HIGH_LEVEL: &str = "high_level_summons";
pub const FUSIONS: &str = "fusion_summons";
pub const SYNCHROS: &str = "synchro_summons";
pub const SPELLS: &str = "spells_activated";
pub const TRAPS: &str = "traps_activated";
pub const COMEBACKS: &str = "comeback_wins";

/// A high-level monster: level 7 or more.
pub const HIGH: u32 = 7;
/// A comeback win: the player's LP fell to this or below during the duel.
pub const COMEBACK_LP: i64 = 1000;
/// Wins against an NPC (its three levels together) for its "lasting friendship" (FAN) title.
pub const LASTING_WINS: u32 = 10;

/// Counter titles: (counter, [(threshold, title name)]).
const COUNTED: &[(&str, &[(u32, &str)])] = &[
    (DUELS, &[(10, "BRONZE DUELIST"), (50, "SILVER DUELIST"), (100, "GOLDEN DUELIST"), (300, "PLATINUM DUELIST"), (1000, "ULTIMATE DUELIST")]),
    (HIGH_LEVEL, &[(10, "BRONZE SUMMONER"), (50, "SILVER SUMMONER"), (200, "GOLDEN SUMMONER")]),
    (FUSIONS, &[(5, "BRONZE FUSIONIST"), (25, "SILVER FUSIONIST"), (100, "GOLDEN FUSIONIST")]),
    (SYNCHROS, &[(5, "BRONZE TUNER"), (25, "SILVER TUNER"), (100, "GOLDEN TUNER")]),
    (TRAPS, &[(20, "BRONZE TRAP ARTIST"), (100, "SILVER TRAP ARTIST"), (500, "GOLDEN TRAP ARTIST")]),
    (SPELLS, &[(20, "BRONZE SPELLCASTER"), (100, "SILVER SPELLCASTER"), (500, "GOLDEN SPELLCASTER")]),
    (COMEBACKS, &[(1, "NEVER SAY DIE WARRIOR"), (10, "NEVER SAY DIE CHAMPION"), (30, "NEVER SAY DIE HERO")]),
];

/// Distinct cards a player got beyond the starter deck, for the collection titles.
const COLLECTION: &[(usize, &str)] = &[(100, "THE CARD ENTHUSIAST"), (500, "THE CARD COLLECTOR"), (1000, "THE CARD HOARDER")];

/// Wins by a card's own rule (the engine's win reasons, `WIN_REASON_*` in ProjectIgnis' constant.lua).
const SPECIAL_WINS: &[(u64, &str)] =
    &[(0x10, "SUMMONER OF EXODIA"), (0x11, "THE BRINGER OF DOOM"), (0x12, "THE REPTILIAN KING"), (0x15, "THE BIRINGER OF DEATH")];

/// The Egyptian God Cards a player summoned (by English name).
const GODS: &[(&str, &str)] =
    &[("Slifer the Sky Dragon", "MASTER OF THE SKIES"), ("Obelisk the Tormentor", "MASTER OF TORMENT"), ("The Winged Dragon of Ra", "MASTER OF THE SUN")];

/// Each NPC's titles (the client's NPC names): a token of friendship with it (its three levels mastered), of lasting
/// friendship (`LASTING_WINS` wins against it), and of defeat by it (a loss).
const NPCS: &[(&str, &str, &str, &str)] = &[
    ("Yami Yugi", "KING OF DOMINO CITY", "YUGI FAN", "ASPIRING DUELIST"),
    ("Seto Kaiba", "THE KC OPERATIVE", "KAIBA FAN", "THE DWEEB"),
    ("Joey Wheeler", "THE BLAZING DUELIST", "JOEY FAN", "THIRD RATE DUELIST"),
    ("Mai Valentine", "THE MAN OF DUELS", "MAI FAN", "THE LONE DUELIST"),
    ("Bandit Keith", "BANDIT BUSTER", "KEITH FAN", "BANDIT ROOKIE"),
    ("Maximillion Pegasus", "CARD DESIGNER", "PEGASUS FAN", "CAPTIVE OF THE MILLENNIUM EYE"),
    ("Yami Bakura", "THE KING OF THIEVES", "YAMI BAKURA FAN", "APPRENTICE THIEF"),
    ("Yami Marik", "THE LAST GRAVEKEEPER", "MARIK FAN", "GHOULS OPERATIVE"),
    ("Yugi Muto", "PUZZLE SOLVER", "YUGI MUTO FAN", "PART-TIMER AT GAME SHOP \"KAME\""),
    ("Jaden Yuki", "KING OF DUEL ACADEMY", "JADEN FAN", "THE DROP OUT KID"),
    ("Chazz Princeton", "THE ELITE DUELIST", "CHAZZ FAN", "FORMER ELITE DUELIST"),
    ("Alexis Rhodes", "ALEXIS GUARD", "ALEXIS FAN", "THE BIGGEST LET DOWN"),
    ("Syrus Truesdale", "TRUE BIG BRO", "SYRUS FAN", "RECKLESS DUELIST"),
    ("Hell Kaiser", "CYBER DARK DUELIST", "KAISER FAN", "CYBER STYLE NOVICE"),
    ("Vellian Crowler", "CROWLER'S PRIZE PUPIL", "CROWLER FAN", "SUPER DROPOUT"),
    ("Aster Phoenix", "THE PROFESSIONAL", "ASTER FAN", "THE AMATEUR"),
    ("Jesse Andersen", "CRYSTAL BEAST FAN", "JESSE FAN", "SPIRITED DUELIST"),
    ("Yusei Fudo", "STARDUST DUELIST", "YUSEI FAN", "SCRAP METAL SCARECROW"),
    ("Jack Atlas", "THE NEXT KING", "JACK FAN", "KING'S JESTER"),
    ("Akiza Izinski", "THE BLACKROSE KNIGHT", "AKIZA FAN", "PSYCHIC DUELIST"),
    ("Leo", "LEO'S HERO", "LEO FAN", "LEO'S PROTÉGÉ"),
    ("Luna", "GUARDIAN OF THE SPIRIT REALM", "LUNA FAN", "DELICATE DUELIST"),
    ("Crow Hogan", "BLACKWING", "CROW FAN", "SATELLITE'S FALLING STAR"),
    ("Carly Carmine", "SUPER REPORTER", "CARLY FAN", "BUMBLING REPORTER"),
    ("Kalin Kessler", "THE ENFORCERS NO.5", "KALIN FAN", "EARTHBOUND IMMORTAL OFFERING"),
];
/// Friendship with every NPC.
const EVERYBODY: &str = "EVERYBODY'S FRIEND";

/// What a player did in one duel, as their side's translator saw it (`translate::Translator::stats`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DuelStats {
    /// The Konami ids of the monsters the player summoned.
    pub summoned: Vec<u16>,
    pub high_level: u32,
    pub fusions: u32,
    pub synchros: u32,
    pub spells: u32,
    pub traps: u32,
    /// The lowest the player's LP went, as a change from the start (0 or less).
    pub lowest_change: i64,
    /// The engine's win reason, when the player won.
    pub win_reason: Option<u64>,
}

impl DuelStats {
    /// Another duel of the same match added in.
    pub fn merge(&mut self, other: &DuelStats) {
        self.summoned.extend_from_slice(&other.summoned);
        self.high_level += other.high_level;
        self.fusions += other.fusions;
        self.synchros += other.synchros;
        self.spells += other.spells;
        self.traps += other.traps;
        self.lowest_change = self.lowest_change.min(other.lowest_change);
        self.win_reason = other.win_reason.or(self.win_reason);
    }
}

/// The client's title data: each title's pbin id by its name, the hair parts the look titles want, and the Konami ids
/// of the Egyptian God Cards.
#[derive(Clone, Debug, Default)]
pub struct TitleData {
    pub by_name: HashMap<String, u16>,
    /// Avatar hair parts (pbin ids) named "Afro…" and "Quiff…".
    pub afro: BTreeSet<u16>,
    pub quiff: BTreeSet<u16>,
    /// Konami id -> English name, for the God Cards.
    pub card_names: HashMap<u16, String>,
    /// Each NPC level id's NPC name (three ids per NPC), for the NPC titles.
    pub npcs: HashMap<u32, String>,
    /// CARD_Prop row -> Konami id, for the collection titles.
    pub row_cids: HashMap<u16, u16>,
}

impl TitleData {
    /// From the client's files: the titles (catalogue kind 10, name text `b`), the hair parts named Afro and Quiff
    /// (kinds 1 and 2, category 1 and 7: hair front and back), the card names and rows, and the NPCs' level ids.
    pub fn from_game(data: &yo3_data::GameData) -> TitleData {
        let name = |e: &yo3_data::CatalogueEntry| data.text(e.b as u32).unwrap_or_default().trim().to_string();
        let hair = |word: &str| -> BTreeSet<u16> {
            data.catalogue.iter().filter(|e| matches!(e.kind, 1 | 2) && name(e).contains(word)).map(|e| e.id).collect()
        };
        TitleData {
            by_name: data.catalogue.iter().filter(|e| e.kind == 10).map(|e| (name(e), e.id)).filter(|(n, _)| !n.is_empty()).collect(),
            afro: hair("Afro"),
            quiff: hair("Quiff"),
            card_names: data.cards.iter().map(|c| (c.cid, c.name.clone())).collect(),
            npcs: data.npc_ids().into_iter().filter_map(|id| Some((id, data.npc_name(id)?.to_string()))).collect(),
            row_cids: data.cards.iter().map(|c| (c.row, c.cid)).collect(),
        }
    }

    fn id(&self, name: &str) -> Option<u16> {
        self.by_name.get(name).copied()
    }

    /// A title's name by its pbin id.
    pub fn name(&self, id: u16) -> Option<&str> {
        self.by_name.iter().find(|(_, i)| **i == id).map(|(n, _)| n.as_str())
    }

    /// Each NPC's wins, losses and draws (its three level ids together) and whether all three levels are mastered, from
    /// a player's records by opponent id (NPC level ids are 256 and below). A level is mastered once its experience
    /// (100 a win, 50 a draw, 10 a loss, as the record's npcbin counts it) reaches its `need_ex` (100, 200, 300).
    pub fn npc_progress(&self, records: &HashMap<u32, [u32; 3]>) -> HashMap<String, ([u32; 3], bool)> {
        let mut out: HashMap<String, ([u32; 3], [bool; 3])> = HashMap::new();
        for (&id, name) in &self.npcs {
            let [w, l, d] = records.get(&id).copied().unwrap_or_default();
            let entry = out.entry(name.clone()).or_default();
            for (sum, n) in entry.0.iter_mut().zip([w, l, d]) {
                *sum += n;
            }
            entry.1[(id % 3) as usize] = 100 * w + 50 * d + 10 * l >= 100 * (id % 3 + 1);
        }
        out.into_iter().map(|(name, (record, levels))| (name, (record, levels.iter().all(|&m| m)))).collect()
    }
}

/// A title's announcement in the player's chat window (a 0x2455 push, as the duel's notes: chat type 0, from no one).
pub fn note(title: &str) -> yo3_proto::Message {
    use yo3_proto::{Message, Value};
    let line = Message::record()
        .with("from_id", Value::U32(0))
        .with("pname", Value::text("Title"))
        .with("chat_type", Value::U32(crate::banter::NPC_CHAT))
        .with("chat_str", Value::text(&format!("You earned the title {title}! Wear it in Avatar Settings.")));
    Message::reply(0x2455).item(line)
}

/// The counters after one more duel: `stats` from it, its `result` (1 won, 2 lost, 3 draw) and the LP it started with.
pub fn count(counters: &mut BTreeMap<String, u32>, stats: &DuelStats, result: u32, start_lp: i64) {
    let mut add = |k: &str, n: u32| {
        if n > 0 {
            *counters.entry(k.to_string()).or_default() += n;
        }
    };
    add(DUELS, 1);
    add(HIGH_LEVEL, stats.high_level);
    add(FUSIONS, stats.fusions);
    add(SYNCHROS, stats.synchros);
    add(SPELLS, stats.spells);
    add(TRAPS, stats.traps);
    add(COMEBACKS, (result == 1 && start_lp + stats.lowest_change <= COMEBACK_LP) as u32);
}

/// What a player has to show for titles.
pub struct Progress<'a> {
    pub counters: &'a BTreeMap<String, u32>,
    /// Distinct Konami ids among the cards got beyond the starter deck.
    pub collection: usize,
    /// The avatar parts worn (`avatar1`'s slots).
    pub wearing: Vec<u16>,
    /// Wins, losses and draws against each NPC, by its name (its three levels together), and whether all three of its
    /// levels are mastered.
    pub npcs: HashMap<String, ([u32; 3], bool)>,
}

/// The titles (pbin ids) `progress` earns, and those of one duel's `stats`.
pub fn earned(data: &TitleData, progress: &Progress, stats: Option<&DuelStats>) -> BTreeSet<u16> {
    let mut names: Vec<&str> = vec![];
    for (counter, steps) in COUNTED {
        let n = progress.counters.get(*counter).copied().unwrap_or(0);
        names.extend(steps.iter().filter(|(at, _)| n >= *at).map(|(_, t)| *t));
    }
    names.extend(COLLECTION.iter().filter(|(at, _)| progress.collection >= *at).map(|(_, t)| *t));
    if progress.wearing.iter().any(|p| data.afro.contains(p)) {
        names.push("THE AFRO KAISER");
    }
    if progress.wearing.iter().any(|p| data.quiff.contains(p)) {
        names.push("THE KING OF QUIFFS");
    }
    let mut friends = 0;
    for (npc, friendship, lasting, defeat) in NPCS {
        let Some(&([wins, losses, _], mastered)) = progress.npcs.get(*npc) else { continue };
        if mastered {
            names.push(friendship);
            friends += 1;
        }
        if wins >= LASTING_WINS {
            names.push(lasting);
        }
        if losses > 0 {
            names.push(defeat);
        }
    }
    if friends == NPCS.len() {
        names.push(EVERYBODY);
    }
    if let Some(stats) = stats {
        names.extend(SPECIAL_WINS.iter().filter(|(r, _)| stats.win_reason == Some(*r)).map(|(_, t)| *t));
        for (god, title) in GODS {
            if stats.summoned.iter().any(|c| data.card_names.get(c).is_some_and(|n| n == god)) {
                names.push(title);
            }
        }
    }
    names.into_iter().filter_map(|n| data.id(n)).collect()
}

/// A title the host grants by hand (GAME MASTER, JUMP FESTA, …): its pbin id by name, ignoring case.
pub fn by_name(data: &TitleData, name: &str) -> Option<u16> {
    data.by_name.iter().find(|(n, _)| n.trim().eq_ignore_ascii_case(name.trim())).map(|(_, &id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> TitleData {
        let names = [
            "BRONZE DUELIST", "SILVER DUELIST", "BRONZE SUMMONER", "BRONZE TUNER", "NEVER SAY DIE WARRIOR", "THE CARD ENTHUSIAST",
            "THE AFRO KAISER", "KING OF DOMINO CITY", "YUGI FAN", "ASPIRING DUELIST", EVERYBODY, "SUMMONER OF EXODIA",
            "MASTER OF THE SUN", "GAME MASTER",
        ];
        TitleData {
            by_name: names.iter().enumerate().map(|(i, n)| (n.to_string(), 300 + i as u16)).collect(),
            afro: [261].into(),
            quiff: [7].into(),
            card_names: [(4002, "The Winged Dragon of Ra".to_string())].into(),
            ..TitleData::default()
        }
    }

    fn named(data: &TitleData, ids: &BTreeSet<u16>) -> Vec<String> {
        let mut v: Vec<String> = data.by_name.iter().filter(|(_, id)| ids.contains(id)).map(|(n, _)| n.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn duels_count_towards_the_counted_titles() {
        let mut counters = BTreeMap::new();
        let stats = DuelStats { high_level: 2, synchros: 1, lowest_change: -7200, ..DuelStats::default() };
        for _ in 0..10 {
            count(&mut counters, &stats, 2, 8000);
        }
        assert_eq!((counters[DUELS], counters[HIGH_LEVEL], counters[SYNCHROS]), (10, 20, 10));
        assert!(!counters.contains_key(COMEBACKS), "a loss is no comeback");
        count(&mut counters, &stats, 1, 8000);
        assert_eq!(counters[COMEBACKS], 1, "won after falling to 800");
        count(&mut counters, &DuelStats { lowest_change: -6000, ..DuelStats::default() }, 1, 8000);
        assert_eq!(counters[COMEBACKS], 1, "2000 LP left is no comeback");
        let d = data();
        let progress = Progress { counters: &counters, collection: 0, wearing: vec![], npcs: HashMap::new() };
        assert_eq!(named(&d, &earned(&d, &progress, None)), ["BRONZE DUELIST", "BRONZE SUMMONER", "BRONZE TUNER", "NEVER SAY DIE WARRIOR"]);
    }

    #[test]
    fn npcs_give_friendship_lasting_friendship_and_defeat() {
        let d = data();
        let counters = BTreeMap::new();
        let mut npcs = HashMap::new();
        npcs.insert("Yami Yugi".to_string(), ([3, 1, 0], true));
        let progress = Progress { counters: &counters, collection: 0, wearing: vec![], npcs: npcs.clone() };
        assert_eq!(named(&d, &earned(&d, &progress, None)), ["ASPIRING DUELIST", "KING OF DOMINO CITY"]);
        npcs.insert("Yami Yugi".to_string(), ([LASTING_WINS, 0, 0], true));
        let progress = Progress { counters: &counters, collection: 0, wearing: vec![], npcs };
        assert!(named(&d, &earned(&d, &progress, None)).contains(&"YUGI FAN".to_string()));
        let all: HashMap<String, ([u32; 3], bool)> = NPCS.iter().map(|(n, ..)| (n.to_string(), ([3, 0, 0], true))).collect();
        let progress = Progress { counters: &counters, collection: 0, wearing: vec![], npcs: all };
        assert!(named(&d, &earned(&d, &progress, None)).contains(&EVERYBODY.to_string()));
    }

    #[test]
    fn an_npc_is_a_friend_once_its_three_levels_are_mastered() {
        let d = TitleData { npcs: [(120, "Yami Yugi".to_string()), (121, "Yami Yugi".to_string()), (122, "Yami Yugi".to_string())].into(), ..data() };
        let mut records = HashMap::from([(120, [1, 0, 0]), (121, [2, 0, 0]), (122, [2, 1, 0])]);
        assert_eq!(d.npc_progress(&records)["Yami Yugi"], ([5, 1, 0], false), "level 3 needs 300: 2 wins and a loss is 210");
        records.insert(122, [3, 1, 0]);
        assert_eq!(d.npc_progress(&records)["Yami Yugi"], ([6, 1, 0], true));
    }

    #[test]
    fn a_duels_own_feats_collection_and_looks() {
        let d = data();
        let counters = BTreeMap::new();
        let stats = DuelStats { summoned: vec![4002], win_reason: Some(0x10), ..DuelStats::default() };
        let progress = Progress { counters: &counters, collection: 120, wearing: vec![0, 261], npcs: HashMap::new() };
        assert_eq!(
            named(&d, &earned(&d, &progress, Some(&stats))),
            ["MASTER OF THE SUN", "SUMMONER OF EXODIA", "THE AFRO KAISER", "THE CARD ENTHUSIAST"]
        );
        assert_eq!(by_name(&d, "game master"), d.by_name.get("GAME MASTER").copied());
        assert_eq!(by_name(&d, "NOT A TITLE"), None);
    }
}
