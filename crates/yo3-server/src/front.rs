//! Front server: login -> player select -> world entry -> main-screen polls -> chat.
//!
//! The request/reply catalog, with the client evidence and status of every pair, is
//! `docs/protocol.md`. Keep it, this file and the contract tests in step.
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use yo3_proto::{Message, Packet, Value};

use crate::mail::{Mail, Mailbox};
use crate::social::Social;
use crate::teams::{LEADER, MEMBER, SUBLEADER, Teams};

/// Where a session's outgoing messages go: its connection's writer task.
pub type Outbox = tokio::sync::mpsc::UnboundedSender<Message>;

pub const VERSION_VALID: u32 = 0x219F;
/// The default banlist every account logs in with (0x2201 regulation_id), which Auto Duel's and the NPC duel's deck
/// check uses (`World::login_regulation`, set by --regulation): 3, "unlimited", no banned or limited cards.
pub const LOGIN_REGULATION: u32 = 3;
/// The banlist the starter deck must pass: the client's first (the Beta list), so the starter stays legal whatever
/// --regulation a host picks.
pub const STARTER_REGULATION: u32 = 1;
pub const HEARTBEAT: u32 = 0x2724;

/// Server clock in Japan time: the client treats server timestamps as JST (UTC+9) and
/// converts them to the player's local time.
fn jst_now() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(9 * 3600).expect("valid offset"))
}

/// Main-screen polls whose replies carry nothing the client reads.
const BARE_REPLIES: [(u32, u32); 1] = [(0x271A, 0x271B)];

#[derive(Clone, Debug)]
pub struct Player {
    pub player_id: u32,
    pub name: String,
    pub index: u32,
    pub level: u32,
    /// 41 BE u16 appearance slots (character settings read 17..=40; slot meanings not mapped yet).
    pub avatar: Vec<u8>,
    /// Per-player flag blob; 4096 bits like the 4096 entries of data/extra/yo3pbininfo.bin.
    pub pbin: Vec<u8>,
    /// Character settings: the 23 speech lines, the 2 comments and the title.
    pub speeches: Vec<String>,
    pub comments: Vec<String>,
    pub title: u32,
    /// The cards the player got beyond the starter deck (bought packs): CARD_Prop row -> count.
    pub cards: BTreeMap<u16, u32>,
    /// The structure decks the player bought (preset deck ids, in order), listed after the starter deck.
    pub decks: Vec<u32>,
    /// The decks the player's clients uploaded at Select Deck (0x1F4F update_type 1), oldest first: the deck list
    /// hands them to a client that lacks a deck of that name, so decks follow the player to another machine.
    pub uploaded: Vec<Deck>,
    /// The protectors and avatar items the player owns, by pbin id (their bits in the record's pbin).
    pub items: std::collections::BTreeSet<u16>,
    /// Cards traded away from the starter deck: CARD_Prop row -> count.
    pub given: BTreeMap<u16, u32>,
    /// The manner ratings other players gave after duels (0x2CED eval_data, -10 to 10): how many, and their sum.
    pub eval_num: u32,
    pub eval_total: i32,
    /// The country flag others see (`country`, an ISO code of the client's `countrytbl.bin`): the account's setting,
    /// or the one its session derives from the client's language. Empty: none sent (the client then shows Germany).
    pub country: String,
}

/// The client language a session starts with (0x1F48 lang 1).
const ENGLISH: u32 = 1;

/// The country a client of a language gets when its account sets none: no request carries the client's country, so
/// this is a **guess** from the login's language (0 Japanese, 1 English, then German, French, Italian, Spanish,
/// Korean, Chinese; the table `countrytbl.bin` has European countries first, so English is GB, not US).
pub fn country_for_lang(lang: u32) -> &'static str {
    match lang {
        0 => "JP",
        1 => "GB",
        2 => "DE",
        3 => "FR",
        4 => "IT",
        5 => "ES",
        6 => "KR",
        7 => "CN",
        _ => "DE",
    }
}
/// The manner level (evallevel 0-4, 4 the best; 0x45F786) of a player nobody rated.
const MANNER: u32 = 2;
/// The values `dlg_manner.swf`'s five buttons send as `eval_data` (the table at `0x790004`), worst first.
const RATINGS: [i32; 5] = [-10, -5, 0, 5, 10];

/// The manner level of `eval_num` ratings adding up to `eval_total`: their average mapped onto the buttons' values
/// (-10 level 0 ... 10 level 4, rounded). A **guess**: the original server's thresholds are unknown.
pub fn manner_level(eval_num: u32, eval_total: i32) -> u32 {
    if eval_num == 0 {
        return MANNER;
    }
    let average = eval_total as f64 / eval_num as f64;
    ((average + 10.0) / 5.0).round().clamp(0.0, 4.0) as u32
}

/// 0x2BE2's error codes for an offer: ERRORCODE_TRADE_CARD_NUM_ILLEGAL, ERRORCODE_TRADE_ERROR.
const CARD_NUM_ILLEGAL: u32 = 504;
const TRADE_ERROR: u32 = 503;

/// The first user event's id: above the client's own event ids (1 for Create Duel and NPC duels, 2 for Auto Duel).
const FIRST_USER_EVENT: u32 = 100;

/// A D-Mail's mail_type: to a player, to the sender's team.
const MAIL_TO_PLAYER: u32 = 0;
const MAIL_TO_TEAM: u32 = 3;
/// 0x2B14's error_code: the player has no team.
const NO_TEAM: u32 = 404;

/// The npcbin's size: the client reads up to NPC id 197 (8 bytes each) without a check.
const NPCBIN_LEN: usize = 2048;

/// Speech lines and comments a player has (CharMake's update 0x51C960).
pub const SPEECHES: usize = 23;
pub const COMMENTS: usize = 2;

impl Player {
    pub fn new(player_id: u32, name: &str) -> Player {
        Player {
            player_id,
            name: name.into(),
            index: 0,
            level: 1,
            avatar: vec![0; 41 * 2],
            pbin: vec![0; 512],
            speeches: vec![String::new(); SPEECHES],
            comments: vec![String::new(); COMMENTS],
            title: 0,
            cards: BTreeMap::new(),
            decks: vec![],
            uploaded: vec![],
            items: Default::default(),
            given: BTreeMap::new(),
            eval_num: 0,
            eval_total: 0,
            country: String::new(),
        }
    }

    /// The pbin the client gets: the player's flags, with bits 0x250-0x253 set (pbin[74] bits 0-3), without which
    /// Seto Kaiba, Yami Marik, Alexis Rhodes and Jack Atlas refuse to duel (0x4BDAB0 state 2), and the items owned.
    fn flags(&self) -> Vec<u8> {
        let mut pbin = self.pbin.clone();
        if let Some(b) = pbin.get_mut(74) {
            *b |= 0x0F;
        }
        for &id in &self.items {
            if let Some(b) = pbin.get_mut(id as usize >> 3) {
                *b |= 1 << (id & 7);
            }
        }
        pbin
    }

    /// The record read by the player list (0x46C1E4) and the PlayerData loader (0x51C260).
    pub fn record(&self) -> Message {
        self.record_with([0; 3], None)
    }

    /// The record with the player's wins, losses and draws against players (My Profile, 0x4705D0; the VS screen) and
    /// their team's name and their authority in it (0x46C1D0: team_level 1, below 1 no team).
    pub fn record_with(&self, [wins, losses, draws]: [u32; 3], team: Option<(&str, u32)>) -> Message {
        let mut m = Message::record()
            .with("pname", Value::text(&self.name))
            .with("pindex", Value::U32(self.index))
            .with("player_id", Value::U32(self.player_id))
            .with("dlv", Value::U32(self.level))
            .with("title", Value::U32(self.title))
            .with("n_win", Value::U32(wins))
            .with("n_lose", Value::U32(losses))
            .with("n_draw", Value::U32(draws))
            .with("n_duelall", Value::U32(wins + losses + draws))
            .with("team_level", Value::U32(team.is_some() as u32))
            .with("team_name", Value::text(team.map_or("", |t| t.0)))
            .with("team_authority", Value::U32(team.map_or(0, |t| t.1)))
            .with("avatar1", Value::Bin(self.avatar.clone()))
            .with("pbin", Value::Bin(self.flags()));
        if !self.country.is_empty() {
            m = m.with("country", Value::text(&self.country));
        }
        for (i, line) in self.speeches.iter().enumerate() {
            m = m.with(&format!("speech{i:02}"), Value::text(line));
        }
        for (i, line) in self.comments.iter().enumerate() {
            m = m.with(&format!("comment{i:02}"), Value::text(line));
        }
        m
    }
}

#[derive(Clone, Debug)]
pub struct Account {
    pub user_id: u32,
    pub name: String,
    pub dpoint: u32,
    pub players: Vec<Player>,
    /// The country the host set for the account (empty: derived from the client's language at login).
    pub country: String,
}

impl Account {
    pub fn test() -> Account {
        Account { user_id: 1001, name: "test".into(), dpoint: 0, players: vec![Player::new(1001, "test")], country: String::new() }
    }
}

/// A deck as the deck list 0x22C8 carries it (read by 0x43FC60 and 0x414410). Cards are CARD_Prop rows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Deck {
    pub id: u32,
    pub name: String,
    pub main: Vec<u16>,
    pub extra: Vec<u16>,
    pub side: Vec<u16>,
}

impl Deck {
    pub fn record(&self) -> Message {
        Message::deck()
            .with("deck_id", Value::U32(self.id))
            .with("deck_name", Value::text(&self.name))
            .with("discription", Value::text(""))
            .with("card_protector", Value::U32(0))
            .with("main", Value::u16_be_array(&self.main))
            .with("extra", Value::u16_be_array(&self.extra))
            .with("side", Value::u16_be_array(&self.side))
    }

    pub fn cards(&self) -> impl Iterator<Item = u16> + '_ {
        self.main.iter().chain(&self.extra).chain(&self.side).copied()
    }

    /// A deck as the client sends it (tag 6 item of 0x1F4F).
    pub fn from_record(m: &Message) -> Deck {
        let rows = |k| m.get(k).and_then(Value::as_bytes).map(|b| b.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect());
        Deck {
            id: m.get("deck_id").and_then(Value::as_u32).unwrap_or(0),
            name: m.get("deck_name").and_then(Value::as_str).unwrap_or_default(),
            main: rows("main").unwrap_or_default(),
            extra: rows("extra").unwrap_or_default(),
            side: rows("side").unwrap_or_default(),
        }
    }
}

/// What a player brings to the duel they are entering: the deck picked at Select Deck (0x1F4F) and,
/// in an NPC duel, the NPC (the 0x17AC rule's cpu_id), in a duel between players the match's duel_id and the
/// opponent. The duel server finds it by player_id (0x200A).
#[derive(Clone, Debug, Default)]
pub struct DuelEntry {
    pub deck: Deck,
    pub npc_id: Option<u32>,
    pub duel_id: Option<u32>,
    pub opponent: Option<u32>,
    /// The duel's starting LP, when the match set it (a hosted duel's lp_start).
    pub lp: Option<u32>,
    /// Duels in the match: 3 for Match Mode, else one (0 counts as one).
    pub best_of: u32,
    /// The duel's time limits in seconds (turn, total; 0: off), when the match set them (a hosted duel's).
    pub times: Option<(u32, u32)>,
}

/// The rule's match_type (DuelMenu's table 0x790C50 by Rule Format button): Single Mode, Match Mode (best of three),
/// and Auto Duel's Both.
pub const SINGLE_MODE: u32 = 1;
pub const MATCH_MODE: u32 = 3;
const EITHER_MODE: u32 = 6;

/// A duel a player hosts (Create Duel, 0x17AC with its settings), waiting for a challenger (0x17B6).
struct Host {
    player_id: u32,
    record: Message,
    settings: Vec<(String, Value)>,
    /// The challenger whose offer the host is answering.
    offer: Option<u32>,
}

impl Host {
    fn setting(&self, k: &str) -> Option<&Value> {
        self.settings.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    }

    fn password(&self) -> String {
        self.setting("password").and_then(Value::as_str).unwrap_or_default()
    }
}

/// The player search fields of a hosted duel (read by PlayerInfoWindow 0x4B6060, 0x4B8730, 0x4B8F30): duel_status 1
/// enables the Duel button. Players in a duel between players get duel_status DUELLING (**guess**: any other value
/// shows them as duelling).
const HOSTING: u32 = 1;
const DUELLING: u32 = 3;
/// The challenge errors (0x17B8 error_code, 0x4A28F0): the host answers someone else, the duel is gone, the password,
/// and "Duel Request was refused." (the host's Opponent's Limits).
const BUSY: u32 = 3000;
const NO_DUEL: u32 = 3001;
const WRONG_PASSWORD: u32 = 4006;
const REFUSED: u32 = 201;
/// Create Duel's Opponent's Limits (the host's party): friends only, team only (PlayerInfoWindow 0x4B6510 offers Send
/// Duel Request only to a player with the host in their friend list, or on its team; 1 is off).
const FRIENDS_ONLY: u32 = 2;
const TEAM_ONLY: u32 = 3;
/// The host's answer (0x17CA).
const ACCEPT: u32 = 2;

/// A player waiting for an automatic match (0x1798): the kind of duel and the record the opponent will see.
struct Waiting {
    player_id: u32,
    match_type: u32,
    record: Message,
}

/// Two matched players (0x1978) and their duel's rule, by duel_id.
struct Match {
    players: [u32; 2],
    rule: Vec<(String, Value)>,
    /// A hosted duel's time_turn and time_total.
    times: Option<(u32, u32)>,
    /// Whether others may watch it (a hosted duel's Allow Watch Duel, watch 1).
    watchable: bool,
}

/// How long the second player's match push waits after its 0x1799, so the agent has read that first (**guess**: it
/// takes one message per step).
const MATCH_PUSH_DELAY: std::time::Duration = std::time::Duration::from_millis(1000);

/// The end of a duel (`result` 1 won, 2 lost, 3 draw), pushed on the player's front connection: duel data (tag 7)
/// "duelData" with children "result" {duel_result, duel_end} and "players" (the player's record {player_id} and its
/// "resultData"). The dispatcher (0x4F4C9E) keeps it in slot 0x15 and sets flag 0x20000, which DuelResult's first
/// step (0x512070) waits for. duel_end 1 = a normal end (**guess**: 2 and 3 show an abnormal-end dialog).
/// resultData is what the result screen shows (getResult 0x511200): `record` = the player's wins, losses and draws
/// against this opponent including this duel, and each `*_add` = this duel's change.
pub fn duel_over(player_id: u32, result: u32, record: [u32; 3]) -> Message {
    duel_over_with_log(player_id, result, record, None)
}

/// The same with the duel's log: its `dlogURL` in the result item, which DuelResult (`0x512070`, once the `0x2009` is in
/// slot 21) reads with `duel_result` and, when the text isn't empty, hands to the client's duel log downloader
/// (`0x498700`) to keep under Duel Log > Latest Duel Logs.
pub fn duel_over_with_log(player_id: u32, result: u32, record: [u32; 3], dlog_url: Option<&str>) -> Message {
    let name = |n: &str| Message::record().with("name", Value::text(n));
    let counts = ["win", "lose", "draw"].iter().zip(record).enumerate().fold(name("resultData"), |m, (i, (k, n))| {
        m.with(k, Value::U32(n)).with(&format!("{k}_add"), Value::U32((i as u32 + 1 == result) as u32))
    });
    let me = Message::record().with("player_id", Value::U32(player_id)).item(counts);
    let data = Message::duel_data()
        .with("name", Value::text("duelData"))
        .item(match dlog_url {
            Some(url) => name("result").with("duel_result", Value::U32(result)).with("duel_end", Value::U32(1)).with("dlogURL", Value::text(url)),
            None => name("result").with("duel_result", Value::U32(result)).with("duel_end", Value::U32(1)),
        })
        .item(name("players").item(me));
    Message::reply(0x2009).item(data)
}

/// A duel that ended for one player (`Hub::finish_duel`). `opponent` is the player's id, or the NPC's id for the duel records.
pub struct Ended {
    pub player_id: u32,
    pub opponent: u32,
    /// Whether the opponent is an NPC (its id is not a player's).
    pub npc: bool,
    pub duel_id: u32,
    pub match_type: u32,
    pub result: u32,
    /// The duel to replay, when the duel server kept it (a duel between players).
    pub replay: Option<crate::duellog::Replay>,
}

/// A chat room, as listed in 0x2458 (fields read by 0x434D20).
#[derive(Clone, Debug)]
pub struct Room {
    /// Must be at least 1: the 0x245A handler ignores lower ids.
    pub id: u32,
    pub name: String,
    pub capacity: u32,
    /// Meaning of region/status/type not mapped yet; 0 works.
    pub region: u32,
    pub status: u32,
    pub kind: u32,
}

impl Room {
    fn new(id: u32, name: &str) -> Room {
        Room { id, name: name.into(), capacity: 50, region: 0, status: 0, kind: 0 }
    }
}

/// Server-wide state shared by all sessions.
pub struct World {
    pub regulation_ids: Vec<u16>,
    /// The client's banlists by id (`regulation_ids` with their cards), which a duel's deck is checked against. Empty
    /// without the game's data: then no deck is checked.
    pub regulations: HashMap<u32, yo3_data::Regulation>,
    /// The banlist accounts log in with (0x219A, 0x2201 regulation_id): Auto Duel's and the NPC duel's deck check.
    pub login_regulation: u32,
    /// The highest CARD_Prop row: the client's per-card arrays have one byte per row (row 0 is a dummy).
    pub card_rows: usize,
    /// Every player's deck until decks are stored per player (from the client's preset decks).
    pub starter_deck: Option<Deck>,
    /// NPC opponents, every level of each (three consecutive ids per NPC, from 120).
    pub npcs: Vec<u32>,
    /// Each NPC's deck (from its preset), by npc_id.
    pub npc_decks: HashMap<u32, Deck>,
    /// Each CARD_Prop row's Konami id and stats, for the rules engine and the duel's card info.
    pub cards: HashMap<u16, crate::translate::CardStats>,
    /// The client's token cards (CARD_Prop rows with ids 3900-3999) by name, for the rules engine's tokens.
    pub token_rows: HashMap<String, Vec<u16>>,
    /// Each player's LP at the start of a duel (8000; lower only to test).
    pub start_lp: u32,
    /// Testing aid: the regulation every duel's deck is checked against, over the duel's own.
    pub test_regulation: Option<u32>,
    /// Testing aid: every player's main deck in a duel, as CARD_Prop rows repeated to 40 cards.
    pub test_deck: Option<Vec<u16>>,
    /// Testing aid: every player's extra deck in a duel, as CARD_Prop rows.
    pub test_extra: Option<Vec<u16>>,
    /// Testing aid: every duel between players' turn and total clocks in seconds (0: off), over a hosted duel's.
    pub test_times: Option<(u32, u32)>,
    /// The rules engine's directory (engine/, once fetched); without it the server deals the opening itself.
    pub engine_dir: Option<std::path::PathBuf>,
    /// Where clients connect for their duels (0x22C4 duelData host and port).
    pub duel_server: (String, u16),
    /// The patch server's port, which also serves the duel logs.
    pub http_port: u16,
    pub news: Vec<(String, String)>,
    pub rooms: Vec<Room>,
    /// The booster packs and structure decks on the shop's shelves (from the client's pack and recipe files).
    pub packs: Vec<crate::shop::ShopPack>,
    pub structure_decks: Vec<crate::shop::ShopDeck>,
    /// The card protectors and avatar items (with their name text) on sale, by pbin id.
    pub protectors: Vec<u16>,
    pub avatar_items: Vec<(u16, u16)>,
}

impl World {
    pub fn new(regulation_ids: Vec<u16>) -> World {
        World {
            regulation_ids,
            regulations: HashMap::new(),
            login_regulation: LOGIN_REGULATION,
            card_rows: 0,
            starter_deck: None,
            npcs: vec![],
            npc_decks: HashMap::new(),
            cards: HashMap::new(),
            token_rows: HashMap::new(),
            engine_dir: None,
            start_lp: 8000,
            test_regulation: None,
            test_deck: None,
            test_extra: None,
            test_times: None,
            duel_server: ("127.0.0.1".into(), 0),
            http_port: 0,
            news: vec![
                (
                    "Welcome back to Yu-Gi-Oh! ONLINE".into(),
                    "This is a private revival server. Have fun duelling with your friends!".into(),
                ),
                // The client keeps decks in its own files and only Edit Deck writes them (its Flash calls
                // SaveDeckData), so a new player needs that one visit before any duel.
                (
                    "New here?".into(),
                    "Open Main Menu > Edit Deck once: it saves your starter deck, which every duel needs.".into(),
                ),
            ],
            rooms: vec![
                Room::new(1, "Lobby"),
                Room::new(2, "Free Talk"),
                Room::new(3, "Trading"),
                Room::new(4, "Duel Recruiting"),
            ],
            packs: vec![],
            structure_decks: vec![],
            protectors: vec![],
            avatar_items: vec![],
        }
    }
}

/// Who is online, in which chat room, and how to reach them.
#[derive(Default)]
pub struct Hub {
    next_id: u64,
    online: HashMap<u64, Online>,
    /// The last join_id (0x1785) and duel_id (0x17AD) handed out.
    last_join: u32,
    last_duel: u32,
    /// Duel entries by player_id.
    duel_entries: HashMap<u32, DuelEntry>,
    /// Wins, losses and draws by (player_id, opponent id), and the file they are kept in (none: since the start).
    records: HashMap<(u32, u32), [u32; 3]>,
    records_file: Option<std::path::PathBuf>,
    /// The accounts, which each counted duel pays BP into (none: no pay).
    directory: Option<Arc<Mutex<crate::accounts::Directory>>>,
    /// Players waiting for an automatic match, in order, and the matched duels.
    waiting: Vec<Waiting>,
    matches: HashMap<u32, Match>,
    /// Hosted duels by duel_id.
    hosts: HashMap<u32, Host>,
    /// The duel server's tables of the matched duels, by duel_id.
    pub(crate) tables: HashMap<u32, Arc<Mutex<crate::table::Table>>>,
    /// Friends, ignore lists and friend requests.
    social: Social,
    /// D-Mail waiting for its recipients.
    mail: Mailbox,
    /// Teams and the invitations waiting for an answer.
    teams: Teams,
    /// Card trades under way, and the invitations waiting for an answer (the player invited -> the one inviting).
    trades: Vec<crate::trade::Trade>,
    trade_invites: HashMap<u32, u32>,
    /// Card rentals: the scenes under way, the invitations waiting for an answer (the player invited -> the lender),
    /// and the rentals made.
    rental_sessions: Vec<crate::rental::Session>,
    rental_invites: HashMap<u32, u32>,
    rentals: crate::rental::Rentals,
    /// User events, and the last event id handed out.
    events: Vec<crate::events::Event>,
    last_event: u32,
    /// Where the events are kept (none: they end with the server).
    events_file: Option<std::path::PathBuf>,
    /// How long the lobby waits between an event's rounds (the winners go through the result screens first).
    round_delay: std::time::Duration,
    /// Where the duel logs are kept, and the HTTP port clients download them from (none: no logs).
    duel_logs: Option<std::path::PathBuf>,
    http_port: u16,
    /// Evaluations asked for and not answered yet (0x2CEC eval_id -> the rater, the player rated), and the last id.
    evals: HashMap<u32, (u32, u32)>,
    last_eval: u32,
    /// Each player's manner ratings (count, sum) as far as this run knows; the accounts have the saved ones.
    ratings: HashMap<u32, (u32, i32)>,
}

/// 0x251F's error codes for a friend request (0x4A522E): "This player cannot receive another request.", " The
/// opponent has left the lobby."; 0 is "A transmission error has occurred.".
const REQUEST_WAITING: u32 = 106;
const NOT_ONLINE: u32 = 112;
/// 0x24BB's answer that accepts a friend request (else 1, declined).
const ACCEPTED: u32 = 2;

struct Online {
    player_id: u32,
    name: String,
    level: u32,
    room: Option<u32>,
    outbox: Outbox,
    /// The player's record as others get it (an opponent's vsPlayerData).
    record: Message,
    /// The client's language (0x1F48 lang: 0 Japanese, 1 English, then German, French, Italian, Spanish, Korean, Chinese).
    lang: u32,
    /// The server address this client reached: where it downloads its duel logs from.
    host: String,
}

impl Hub {
    pub fn online(&self) -> usize {
        self.online.len()
    }

    /// A hub whose duel records are kept in `file` (read now if it exists, written after each duel).
    pub fn with_records(file: &std::path::Path) -> Hub {
        let mut hub = Hub { records_file: Some(file.to_path_buf()), ..Hub::default() };
        match crate::store::read(file) {
            Ok(saved) => {
                for r in saved.iter().flat_map(|s| s["records"].as_array()).flatten() {
                    let n = |k: &str| r[k].as_u64().unwrap_or(0) as u32;
                    hub.records.insert((n("player_id"), n("opponent")), [n("win"), n("lose"), n("draw")]);
                }
            }
            Err(e) => tracing::warn!("records not read: {e}"),
        }
        hub
    }

    /// The accounts duel results pay BP into.
    pub fn set_directory(&mut self, directory: Arc<Mutex<crate::accounts::Directory>>) {
        self.directory = Some(directory);
    }

    pub fn enter_duel(&mut self, player_id: u32, entry: DuelEntry) {
        self.duel_entries.insert(player_id, entry);
    }

    pub fn duel_entry(&self, player_id: u32) -> Option<DuelEntry> {
        self.duel_entries.get(&player_id).cloned()
    }

    /// A player's manner ratings: how many and their sum.
    pub fn ratings(&self, player_id: u32) -> (u32, i32) {
        if let Some(&r) = self.ratings.get(&player_id) {
            return r;
        }
        let saved = self.directory.as_ref().and_then(|d| d.lock().expect("directory lock").player(player_id).map(|p| (p.eval_num, p.eval_total)));
        saved.unwrap_or_default()
    }

    /// A player's manner level, the `evallevel` of their records.
    pub fn manner(&self, player_id: u32) -> u32 {
        let (num, total) = self.ratings(player_id);
        manner_level(num, total)
    }

    /// Asks `rater` to rate `target` (0x2CEC {eval_id}): DuelResult opens the manner dialog once it has the push.
    fn ask_to_rate(&mut self, rater: u32, target: u32) {
        self.last_eval += 1;
        self.evals.insert(self.last_eval, (rater, target));
        self.push(rater, &Message::reply(0x2CEC).with("eval_id", Value::U32(self.last_eval)));
    }

    /// `rater`'s answer to an evaluation (0x2CED {eval_id, eval_data}): the rating counts once, for the player it asked
    /// about, and goes into the accounts. False for an unknown or already answered evaluation, somebody else's, or a
    /// value that no button sends.
    pub fn rate(&mut self, rater: u32, eval_id: u32, value: i32) -> bool {
        let Some(&(who, target)) = self.evals.get(&eval_id) else { return false };
        if who != rater || !RATINGS.contains(&value) {
            return false;
        }
        self.evals.remove(&eval_id);
        let (num, total) = self.ratings(target);
        let (num, total) = (num + 1, total + value);
        self.ratings.insert(target, (num, total));
        if let Some(d) = &self.directory {
            d.lock().expect("directory lock").update_player(target, |p| (p.eval_num, p.eval_total) = (num, total));
        }
        true
    }

    /// A player as a replay shows them (`duellog::Side`): their record as the opponent's client gets it.
    pub fn side(&self, player_id: u32, deck: &Deck, player_type: u32) -> crate::duellog::Side {
        let record = self.online.values().find(|o| o.player_id == player_id).map(|o| &o.record);
        let int = |k: &str| record.and_then(|r| r.get(k)).and_then(Value::as_u32).unwrap_or(0);
        crate::duellog::Side {
            player_id,
            level: int("dlv"),
            title: int("title"),
            main: deck.main.len() as u32,
            extra: deck.extra.len() as u32,
            player_type,
            protector: 0,
            record: self.totals(player_id),
            name: record.and_then(|r| r.get("pname")).and_then(Value::as_str).unwrap_or_default(),
            avatar: record.and_then(|r| r.get("avatar1")).and_then(Value::as_bytes).unwrap_or_default().to_vec(),
            speeches: (0..SPEECHES).map(|i| record.and_then(|r| r.get(&format!("speech{i:02}"))).and_then(Value::as_str).unwrap_or_default()).collect(),
        }
    }

    /// Keeps each duel's log in `dir`, which the HTTP server on `http_port` serves (`/duellogs/NAME`).
    pub fn keep_duel_logs(&mut self, dir: &std::path::Path, http_port: u16) {
        self.duel_logs = Some(dir.to_path_buf());
        self.http_port = http_port;
    }

    /// A duel's end for one player: counts the result, saves the duel's log and pushes the `0x2009` that makes the
    /// client download it. Returns the log's file name (none without a log directory).
    pub fn finish_duel(&mut self, ended: &Ended) -> Option<String> {
        let record = self.record_result(ended.player_id, ended.opponent, ended.result);
        let me = self.online.values().find(|o| o.player_id == ended.player_id);
        let (host, name, level) = me.map(|o| (o.host.clone(), o.name.clone(), o.level)).unwrap_or_default();
        let (opponent_name, opponent_level) = match self.online.values().find(|o| !ended.npc && o.player_id == ended.opponent) {
            Some(o) => (o.name.clone(), o.level),
            None => ("CPU".to_string(), 0),
        };
        // Only a duel against a player is kept: the client downloads no log of one against an NPC (DuelResult
        // `0x512070`: MainGamen `+0x804` is set, and the `dlogURL` step is skipped).
        let saved = self.duel_logs.as_ref().filter(|_| !ended.npc).map(|dir| {
            let end_time = chrono::Utc::now().timestamp() as u32;
            let mut log = crate::duellog::Log {
                duel_id: ended.duel_id,
                player_1: ended.player_id,
                player_2: ended.opponent,
                match_type: ended.match_type,
                dlv_1: level,
                dlv_2: opponent_level,
                end_time,
                pname_1: name,
                pname_2: opponent_name,
                comment: String::new(),
                owner: ended.player_id,
                body: vec![],
            };
            if let Some(replay) = &ended.replay {
                // The replay shows the player who went first first.
                let [a, b] = &replay.sides;
                (log.player_1, log.player_2, log.dlv_1, log.dlv_2) = (a.player_id, b.player_id, a.level, b.level);
                (log.pname_1, log.pname_2) = (a.name.clone(), b.name.clone());
                log.body = replay.body(ended.duel_id, ended.match_type, end_time);
            }
            (log.save(dir), log)
        });
        let file = match saved {
            Some((Ok(file), _)) => Some(file),
            Some((Err(e), _)) => {
                tracing::warn!("duel log not saved: {e}");
                None
            }
            None => None,
        };
        let url = file.as_ref().map(|f| format!("http://{host}:{}/duellogs/{f}", self.http_port));
        self.push(ended.player_id, &duel_over_with_log(ended.player_id, ended.result, record, url.as_deref()));
        if !ended.npc {
            self.ask_to_rate(ended.player_id, ended.opponent);
        }
        file
    }

    /// Counts a duel's `result` (1 won, 2 lost, 3 draw), pays its BP into the player's account (`shop::duel_bp`), and
    /// returns the player's wins, losses and draws against this opponent.
    pub fn record_result(&mut self, player_id: u32, opponent: u32, result: u32) -> [u32; 3] {
        if let Some(d) = &self.directory {
            d.lock().expect("directory lock").reward(player_id, crate::shop::duel_bp(result));
        }
        let record = self.records.entry((player_id, opponent)).or_default();
        if let Some(n) = record.get_mut(result.wrapping_sub(1) as usize) {
            *n += 1;
        }
        let record = *record;
        if let Some(file) = &self.records_file {
            let mut all: Vec<_> = self.records.iter().collect();
            all.sort();
            let rows: Vec<serde_json::Value> = all
                .into_iter()
                .map(|((me, them), [w, l, d])| serde_json::json!({ "player_id": me, "opponent": them, "win": w, "lose": l, "draw": d }))
                .collect();
            if let Err(e) = crate::store::write(file, &serde_json::json!({ "records": rows })) {
                tracing::warn!("records not saved to {}: {e}", file.display());
            }
        }
        record
    }

    /// The player's record against each NPC level id (the record's npcbin, read by 0x51C680): u16 little-endian at
    /// 8 * (id - 1) + 2k, k 0 wins, 1 losses, 2 draws, 3 experience (100 a win, 50 a draw, 10 a loss: a **guess**).
    pub fn npcbin(&self, player_id: u32) -> Vec<u8> {
        let mut npcbin = vec![0; NPCBIN_LEN];
        for (&(me, id), &[w, l, d]) in &self.records {
            let at = 8 * (id as usize).wrapping_sub(1);
            if me != player_id || !(1..=256).contains(&id) || at + 8 > npcbin.len() {
                continue;
            }
            let exp = 100 * w + 50 * d + 10 * l;
            for (k, n) in [w, l, d, exp].into_iter().enumerate() {
                npcbin[at + 2 * k..at + 2 * k + 2].copy_from_slice(&(n.min(u16::MAX as u32) as u16).to_le_bytes());
            }
        }
        npcbin
    }

    fn set_lang(&mut self, online: u64, lang: u32) {
        if let Some(o) = self.online.get_mut(&online) {
            o.lang = lang;
        }
    }

    /// A player's wins, losses and draws against other players (NPC ids are 256 and below: practice, not counted).
    fn totals(&self, player_id: u32) -> [u32; 3] {
        let against_players = self.records.iter().filter(|((me, them), _)| *me == player_id && *them > 256);
        against_players.fold([0; 3], |t, (_, r)| [t[0] + r[0], t[1] + r[1], t[2] + r[2]])
    }

    /// A player's own record (the player list 0x2260, the refresh 0x2CF3): with their totals, team and NPC experience.
    fn own_record(&self, player: &Player) -> Message {
        self.record(player).with("npcbin", Value::Bin(self.npcbin(player.player_id)))
    }

    /// A player's record with their totals and team.
    fn record(&self, player: &Player) -> Message {
        let team = self.teams.of(player.player_id);
        let team = team.map(|t| (t.name.as_str(), t.authority(player.player_id).unwrap_or(MEMBER)));
        let (eval_num, eval_total) = self.ratings(player.player_id);
        player
            .record_with(self.totals(player.player_id), team)
            .with("evalnum", Value::U32(eval_num))
            .with("evaltotal", Value::U32(eval_total.max(0) as u32))
            .with("evallevel", Value::U32(manner_level(eval_num, eval_total)))
    }

    fn join(&mut self, player: &Player, outbox: Outbox, lang: u32, host: String) -> u64 {
        self.next_id += 1;
        let record = self.record(player);
        let online = Online { player_id: player.player_id, name: player.name.clone(), level: player.level, room: None, outbox, record, lang, host };
        self.online.insert(self.next_id, online);
        self.social.seen(player.player_id, &player.name);
        self.tell_friends(player.player_id);
        self.next_id
    }

    /// Friends and ignore lists are kept in `file` (read now if it exists, written after each change).
    pub fn keep_social(&mut self, file: &std::path::Path) {
        self.social = Social::load(file);
    }

    /// Keeps the mail waiting for its recipients in `file`.
    pub fn keep_mail(&mut self, file: &std::path::Path) {
        self.mail = Mailbox::load(file);
    }

    /// Keeps the teams in `file`.
    pub fn keep_teams(&mut self, file: &std::path::Path) {
        self.teams = Teams::load(file);
    }

    /// How long an event waits between two rounds (none by default).
    pub fn set_round_delay(&mut self, delay: std::time::Duration) {
        self.round_delay = delay;
    }

    /// Starts the rounds whose wait is over (called every second).
    pub fn tick_events(&mut self, now: std::time::Instant) {
        let due: Vec<usize> = (0..self.events.len()).filter(|&at| self.events[at].next_at.is_some_and(|t| t <= now)).collect();
        for at in due {
            self.events[at].next_at = None;
            self.events[at].waiting = false;
            for p in self.events[at].players() {
                self.push(p, &Message::reply(0x3302));
            }
            self.start_round(at);
        }
    }

    /// Keeps the user events in `file`: the ones still open come back at the next start. A running event cannot (its
    /// duels die with the server, and its players are no longer in the lobby), so it ends there without a winner.
    pub fn keep_events(&mut self, file: &std::path::Path) {
        self.events_file = Some(file.to_path_buf());
        let saved = match crate::store::read(file) {
            Ok(saved) => saved.unwrap_or(serde_json::Value::Null),
            Err(e) => {
                tracing::warn!("events not read: {e}");
                serde_json::Value::Null
            }
        };
        self.last_event = self.last_event.max(saved["last_event"].as_u64().unwrap_or(0) as u32);
        let kept = saved["events"].as_array().into_iter().flatten().filter_map(crate::events::Event::from_json);
        self.events = kept.filter(|e| e.status == crate::events::OPEN).collect();
    }

    fn save_events(&self) {
        let Some(file) = &self.events_file else { return };
        let events: Vec<serde_json::Value> = self.events.iter().filter(|e| e.status != crate::events::FINISHED).map(|e| e.to_json()).collect();
        if let Err(e) = crate::store::write(file, &serde_json::json!({ "last_event": self.last_event, "events": events })) {
            tracing::warn!("events not saved to {}: {e}", file.display());
        }
    }

    /// A player as the accounts keep them (none without accounts).
    fn saved_player(&self, player_id: u32) -> Option<Player> {
        let d = self.directory.as_ref()?.lock().expect("directory lock");
        d.accounts().flat_map(|(_, a)| &a.players).find(|p| p.player_id == player_id).cloned()
    }

    /// The trade a player is in, and their seat in it.
    fn trade_of(&mut self, player_id: u32) -> Option<(&mut crate::trade::Trade, usize)> {
        self.trades.iter_mut().find_map(|t| t.seat(player_id).map(|s| (t, s)))
    }

    /// Ends a player's trade (they left the scene or the server): the partner gets 0x2BCD ("The trade partner has
    /// cancelled.").
    fn end_trade(&mut self, player_id: u32) {
        let Some(at) = self.trades.iter().position(|t| t.seat(player_id).is_some()) else { return };
        let trade = self.trades.remove(at);
        for p in trade.players.into_iter().filter(|&p| p != player_id) {
            self.push(p, &Message::reply(0x2BCD));
        }
    }

    /// The card trade requests (agent step 0x4A91D0 and the trade scene, handler 0x4248F0). Each reply's error twin is
    /// the id after it, with an error_code.
    fn trade_request(&mut self, (me, my_name): (u32, &str), id: u32, msg: &Message, world: &World) -> Message {
        let int = |k: &str| msg.get(k).and_then(|v| v.as_u32().or_else(|| v.as_str()?.trim().parse().ok())).unwrap_or(0);
        let fail = |reply: u32, code: u32| Message::reply(reply + 1).with("error_code", Value::U32(code));
        match id {
            // Invite {to_id} -> 0x2BD7 {pname}; the player gets 0x2BC1 {from_id, from_name} on the bell. 106 when
            // they are away, invited, or trading.
            0x2BC0 => {
                let to = int("to_id");
                let busy = self.trade_invites.contains_key(&to) || self.trades.iter().any(|t| t.seat(to).is_some());
                let Some(name) = self.online.values().find(|o| o.player_id == to).map(|o| o.name.clone()) else {
                    return fail(0x2BD7, REQUEST_WAITING);
                };
                if busy || to == me {
                    return fail(0x2BD7, REQUEST_WAITING);
                }
                self.trade_invites.insert(to, me);
                self.push(to, &Message::reply(0x2BC1).with("from_id", Value::U32(me)).with("from_name", Value::text(my_name)));
                Message::reply(0x2BD7).with("pname", Value::text(&name))
            }
            // The inviter cancels -> 0x2BDD; the player gets 0x2BC5.
            0x2BC4 => {
                let invited: Vec<u32> = self.trade_invites.iter().filter(|(_, from)| **from == me).map(|(&to, _)| to).collect();
                for to in invited {
                    self.trade_invites.remove(&to);
                    self.push(to, &Message::reply(0x2BC5));
                }
                Message::reply(0x2BDD)
            }
            // The answer {to_id: the inviter, answer: 2 yes, 1 no} -> 0x2BDB; the inviter gets 0x2BC3 {answer}, and a
            // yes opens both scenes.
            0x2BC2 => {
                let Some(from) = self.trade_invites.remove(&me) else { return fail(0x2BDB, 0) };
                let answer = int("answer");
                self.push(from, &Message::reply(0x2BC3).with("answer", Value::U32(answer)));
                if answer == ACCEPTED {
                    self.trades.push(crate::trade::Trade::new([from, me]));
                }
                Message::reply(0x2BDB)
            }
            // The scene is ready -> 0x2BED; once both are, both get 0x2BD6 (NOTICE_START).
            0x2BD5 => {
                let Some((trade, seat)) = self.trade_of(me) else { return fail(0x2BED, 0) };
                trade.ready[seat] = true;
                if trade.ready == [true; 2] {
                    let players = trade.players;
                    for p in players {
                        self.push(p, &Message::reply(0x2BD6));
                    }
                }
                Message::reply(0x2BED)
            }
            // An offer {main_list} -> 0x2BE1; the partner gets 0x2BCF {main_list}. 504 for cards the player doesn't
            // have. A new offer takes back both OKs.
            0x2BC6 => {
                let rows: Vec<u16> = msg.get("main_list").and_then(Value::as_bytes).unwrap_or_default().as_chunks::<2>().0.iter().map(|b| u16::from_be_bytes(*b)).collect();
                let player = self.saved_player(me);
                if player.is_some_and(|p| !crate::trade::owns(world, &p, &self.rentals.lent_by(me), &rows)) {
                    return fail(0x2BE1, CARD_NUM_ILLEGAL);
                }
                let Some((trade, seat)) = self.trade_of(me) else { return fail(0x2BE1, TRADE_ERROR) };
                trade.offers[seat] = rows.clone();
                trade.ok = [false; 2];
                let partner = trade.players[1 - seat];
                self.push(partner, &Message::reply(0x2BCF).with("main_list", Value::u16_be_array(&rows)));
                Message::reply(0x2BE1)
            }
            // The partner saw the offer -> 0x2BE3; the offerer gets 0x2BCE and 0x2BD0.
            0x2BC7 => {
                let Some((trade, seat)) = self.trade_of(me) else { return fail(0x2BE3, TRADE_ERROR) };
                let partner = trade.players[1 - seat];
                self.push(partner, &Message::reply(0x2BCE));
                self.push(partner, &Message::reply(0x2BD0));
                Message::reply(0x2BE3)
            }
            // OK {status: 1 on, 0 off} -> 0x2BE5; the partner gets 0x2BD1 {status}, and with both on both get 0x2BD2.
            0x2BC8 => {
                let status = int("status");
                let Some((trade, seat)) = self.trade_of(me) else { return fail(0x2BE5, TRADE_ERROR) };
                trade.ok[seat] = status == 1;
                trade.answers = [None; 2];
                let (players, both) = (trade.players, trade.ok == [true; 2]);
                self.push(players[1 - seat], &Message::reply(0x2BD1).with("status", Value::U32(status)));
                if both {
                    for p in players {
                        self.push(p, &Message::reply(0x2BD2));
                    }
                }
                Message::reply(0x2BE5)
            }
            // "Execute Trade?" {answer: 2 yes, 1 no (live: Yes sent 2)} -> 0x2BE7; once both answered, both get 0x2BCA
            // {error_code, own_status, other_status, trade_result: 1 done}, and with two yeses the cards change hands
            // and both get a new trunk.
            0x2BC9 => {
                let yes = int("answer") == ACCEPTED;
                let Some((trade, seat)) = self.trade_of(me) else { return fail(0x2BE7, TRADE_ERROR) };
                trade.answers[seat] = Some(yes);
                let [Some(first), Some(second)] = trade.answers else { return Message::reply(0x2BE7) };
                let at = self.trades.iter().position(|t| t.seat(me).is_some()).expect("the trade");
                let trade = self.trades.remove(at);
                let done = first && second && self.exchange(&trade, world);
                let answers = [first, second];
                for (s, &p) in trade.players.iter().enumerate() {
                    let result = Message::reply(0x2BCA)
                        .with("error_code", Value::U32(0))
                        .with("own_status", Value::U32(answers[s] as u32))
                        .with("other_status", Value::U32(answers[1 - s] as u32))
                        .with("trade_result", Value::U32(done as u32));
                    self.push(p, &result);
                    if done && let Some(player) = self.saved_player(p) {
                        self.push(p, &crate::trade::trunk(world, &player));
                    }
                }
                Message::reply(0x2BE7)
            }
            // Leaving the scene -> 0x2BE9; the partner gets 0x2BCD.
            0x2BCB => {
                self.end_trade(me);
                Message::reply(0x2BE9)
            }
            _ => fail(id, 0),
        }
    }

    /// Keeps the rentals in `file`: they come back at the next start, and end when their time is up.
    pub fn keep_rentals(&mut self, file: &std::path::Path) {
        self.rentals = crate::rental::Rentals::load(file);
    }

    /// Ends the rentals whose time is up at `now` (unix seconds): their players get the records again, so their
    /// collections go back to what they own.
    pub fn tick_rentals(&mut self, now: i64) {
        for (lender, borrower) in self.rentals.expire(now) {
            for p in [lender, borrower] {
                self.push(p, &self.rentals.records_for(p));
            }
        }
    }

    /// The rental records of a player (0x2D6C), and their history lists (0x2D6E for kind 0 borrowed, 1 lent).
    pub fn rental_records(&self, player_id: u32) -> Message {
        self.rentals.records_for(player_id)
    }

    pub fn rental_history(&self, player_id: u32, kind: u32) -> Message {
        self.rentals.history_for(player_id, kind)
    }

    /// The rental scene a player is in, and their seat in it (0 the lender).
    fn rental_of(&mut self, player_id: u32) -> Option<(&mut crate::rental::Session, usize)> {
        self.rental_sessions.iter_mut().find_map(|t| t.seat(player_id).map(|s| (t, s)))
    }

    /// Ends a player's rental scene (they left it or the server): the partner gets 0x2C37 ("The Borrower has
    /// cancelled." or "The Lender has cancelled.").
    fn end_rental_scene(&mut self, player_id: u32) {
        let Some(at) = self.rental_sessions.iter().position(|t| t.seat(player_id).is_some()) else { return };
        let session = self.rental_sessions.remove(at);
        for (p, _) in session.players.into_iter().filter(|(p, _)| *p != player_id) {
            self.push(p, &Message::reply(0x2C37));
        }
    }

    /// The card rental requests (agent steps 0x4A9D80 and 0x4AA430, the rental scene CardRental.cpp, handler
    /// 0x41C7E0). Each reply's error twin is the id after it, with an error_code. The player who sends the request is
    /// the lender: the scene opens with `for_rent` 1 for them (0x4AA3BB) and 0 for the one who accepts.
    fn rental_request(&mut self, (me, my_name): (u32, &str), id: u32, msg: &Message, world: &World, now: i64) -> Message {
        let int = |k: &str| msg.get(k).and_then(|v| v.as_u32().or_else(|| v.as_str()?.trim().parse().ok())).unwrap_or(0);
        let fail = |reply: u32, code: u32| Message::reply(reply + 1).with("error_code", Value::U32(code));
        let list = |k: &str| -> Vec<u16> { msg.get(k).and_then(Value::as_bytes).unwrap_or_default().as_chunks::<2>().0.iter().map(|b| u16::from_be_bytes(*b)).collect() };
        match id {
            // Invite {to_id} -> 0x2D50 {pname}; the player gets 0x2C25 {from_id, from_name} on the bell. 106 when they
            // are away, invited, or in a rental.
            0x2C24 => {
                let to = int("to_id");
                let busy = self.rental_invites.contains_key(&to) || self.rental_sessions.iter().any(|t| t.seat(to).is_some());
                let Some(name) = self.online.values().find(|o| o.player_id == to).map(|o| o.name.clone()) else {
                    return fail(0x2D50, REQUEST_WAITING);
                };
                if busy || to == me {
                    return fail(0x2D50, REQUEST_WAITING);
                }
                self.rental_invites.insert(to, me);
                self.push(to, &Message::reply(0x2C25).with("from_id", Value::U32(me)).with("from_name", Value::text(my_name)));
                Message::reply(0x2D50).with("pname", Value::text(&name))
            }
            // The inviter cancels -> 0x2D56; the player gets 0x2C29.
            0x2C28 => {
                let invited: Vec<u32> = self.rental_invites.iter().filter(|(_, from)| **from == me).map(|(&to, _)| to).collect();
                for to in invited {
                    self.rental_invites.remove(&to);
                    self.push(to, &Message::reply(0x2C29));
                }
                Message::reply(0x2D56)
            }
            // The answer {to_id: the lender, answer: 2 yes, 1 no} -> 0x2D54; the lender gets 0x2C27 {answer}, and a
            // yes opens both scenes.
            0x2C26 => {
                let Some(from) = self.rental_invites.remove(&me) else { return fail(0x2D54, 0) };
                let answer = int("answer");
                self.push(from, &Message::reply(0x2C27).with("answer", Value::U32(answer)));
                if answer == ACCEPTED {
                    let lender = self.online.values().find(|o| o.player_id == from).map(|o| o.name.clone()).unwrap_or_default();
                    self.rental_sessions.push(crate::rental::Session::new((from, lender), (me, my_name.to_string())));
                }
                Message::reply(0x2D54)
            }
            // The scene is ready -> a reply the scene ignores (meaning of its id unknown); once both are, both get
            // 0x2C3F (NOTICE_START).
            0x2C3E => {
                let Some((session, seat)) = self.rental_of(me) else { return fail(0x2D68, 0) };
                session.ready[seat] = true;
                if session.ready == [true; 2] {
                    let players = [session.players[0].0, session.players[1].0];
                    for p in players {
                        self.push(p, &Message::reply(0x2C3F));
                    }
                }
                Message::reply(0x2D68)
            }
            // The lender's cards {main_list, extra_list, side_list: big-endian u16 rows} -> 0x2D5A; the borrower gets
            // them as 0x2C38. 504 for cards the lender doesn't own (or has lent already) or too many. A new offer
            // withdraws the confirmation.
            0x2C2A => {
                let lists = [list("main_list"), list("extra_list"), list("side_list")];
                let Some((session, seat)) = self.rental_of(me) else { return fail(0x2D5A, TRADE_ERROR) };
                let limits = if session.kind == 0 { [90, 0, 0] } else { [60, 15, 15] };
                if seat != 0 || lists.iter().zip(limits).any(|(l, max)| l.len() > max) {
                    return fail(0x2D5A, CARD_NUM_ILLEGAL);
                }
                let all: Vec<u16> = lists.iter().flatten().copied().collect();
                let player = self.saved_player(me);
                if player.is_some_and(|p| !crate::trade::owns(world, &p, &self.rentals.lent_by(me), &all)) {
                    return fail(0x2D5A, CARD_NUM_ILLEGAL);
                }
                let (session, _) = self.rental_of(me).expect("the scene");
                session.lists = lists;
                session.offered = false;
                let (borrower, push) = (session.players[1].0, session.lists.clone());
                let [main, extra, side] = push;
                self.push(
                    borrower,
                    &Message::reply(0x2C38)
                        .with("main_list", Value::u16_be_array(&main))
                        .with("extra_list", Value::u16_be_array(&extra))
                        .with("side_list", Value::u16_be_array(&side)),
                );
                Message::reply(0x2D5A)
            }
            // The conditions {type 0 Free / 1 Deck, days} -> 0x2D5C; the borrower gets them as 0x2C39. A change of
            // type clears the cards (their places differ).
            0x2C2B => {
                let (kind, days) = (int("type"), int("days"));
                let Some((session, seat)) = self.rental_of(me) else { return fail(0x2D5C, TRADE_ERROR) };
                if seat != 0 || kind > 1 || !crate::rental::TERMS.contains(&days) {
                    return fail(0x2D5C, TRADE_ERROR);
                }
                if session.kind != kind {
                    session.lists = Default::default();
                }
                (session.kind, session.days, session.offered) = (kind, days, false);
                let borrower = session.players[1].0;
                self.push(borrower, &Message::reply(0x2C39).with("type", Value::U32(kind)).with("days", Value::U32(days)));
                Message::reply(0x2D5C)
            }
            // The partner's scene has the cards or conditions -> 0x2D5E; the partner gets 0x2C3A.
            0x2C2C => {
                let Some((session, seat)) = self.rental_of(me) else { return fail(0x2D5E, TRADE_ERROR) };
                let partner = session.players[1 - seat].0;
                self.push(partner, &Message::reply(0x2C3A));
                Message::reply(0x2D5E)
            }
            // The lender's Confirm (cards put up) -> 0x2D60; the borrower gets 0x2C2E and is asked "Borrow these
            // cards?". 504 with no cards.
            0x2C2D => {
                let Some((session, seat)) = self.rental_of(me) else { return fail(0x2D60, TRADE_ERROR) };
                if seat != 0 || session.cards().next().is_none() {
                    return fail(0x2D60, CARD_NUM_ILLEGAL);
                }
                session.offered = true;
                let borrower = session.players[1].0;
                self.push(borrower, &Message::reply(0x2C2E));
                Message::reply(0x2D60)
            }
            // The lender withdraws the confirmation -> 0x2D66; the borrower gets 0x2C32.
            0x2C31 => {
                let Some((session, seat)) = self.rental_of(me) else { return fail(0x2D66, TRADE_ERROR) };
                session.offered = false;
                let partner = session.players[1 - seat].0;
                self.push(partner, &Message::reply(0x2C32));
                Message::reply(0x2D66)
            }
            // The borrower's answer {answer: 2 yes, 1 no} -> 0x2D64; both get 0x2C30 {trade_result: 1 done, 0 not}.
            // With a yes the rental starts (the cards are still the lender's to lend) and both get the records.
            0x2C2F => {
                let yes = int("answer") == ACCEPTED;
                let Some((session, seat)) = self.rental_of(me) else { return fail(0x2D64, TRADE_ERROR) };
                if seat != 1 || !session.offered {
                    return fail(0x2D64, TRADE_ERROR);
                }
                session.offered = false;
                let (players, cards) = ([session.players[0].0, session.players[1].0], session.cards().collect::<Vec<_>>());
                let still = self.saved_player(players[0]).is_none_or(|p| crate::trade::owns(world, &p, &self.rentals.lent_by(players[0]), &cards));
                let done = yes && still;
                if done {
                    let at = self.rental_sessions.iter().position(|t| t.seat(me).is_some()).expect("the scene");
                    let session = self.rental_sessions.remove(at);
                    self.rentals.start(&session, now);
                }
                for p in players {
                    self.push(p, &Message::reply(0x2C30).with("trade_result", Value::U32(done as u32)));
                    if done {
                        self.push(p, &self.rentals.records_for(p));
                    }
                }
                Message::reply(0x2D64)
            }
            // Leaving the scene -> 0x2D6A; the partner gets 0x2C37.
            0x2C33 => {
                self.end_rental_scene(me);
                Message::reply(0x2D6A)
            }
            _ => fail(id, 0),
        }
    }

    /// Hands a trade's offers over, both still owned; whether it happened.
    fn exchange(&mut self, trade: &crate::trade::Trade, world: &World) -> bool {
        let Some(d) = self.directory.clone() else { return false };
        let mut d = d.lock().expect("directory lock");
        let saved = |d: &crate::accounts::Directory, id: u32| d.accounts().flat_map(|(_, a)| &a.players).find(|p| p.player_id == id).cloned();
        let owned = trade.players.iter().zip(&trade.offers).all(|(&p, offer)| saved(&d, p).is_some_and(|s| crate::trade::owns(world, &s, &self.rentals.lent_by(p), offer)));
        if !owned {
            return false;
        }
        for s in 0..2 {
            let (giver, offer, taker) = (trade.players[s], trade.offers[s].clone(), trade.players[1 - s]);
            d.update_player(giver, |p| crate::trade::give(p, &offer));
            d.update_player(taker, |p| crate::trade::receive(p, &offer));
        }
        true
    }

    /// Pushes to every member of a team who is online, but `except`.
    fn tell_team(&self, members: &[u32], except: Option<u32>, msg: &Message) {
        for &id in members.iter().filter(|&&id| Some(id) != except) {
            self.push(id, msg);
        }
    }

    /// 0x2AFB: the player's team (0x2B13), or 0x2B14 {error_code 404}, which clears slot 17 (0x4F5CCA): no team.
    fn team_info(&self, me: u32) -> Message {
        match self.teams.of(me) {
            Some(t) => t.info(),
            None => Message::reply(0x2B14).with("error_code", Value::U32(NO_TEAM)),
        }
    }

    /// 0x2AFD: the team's members (0x2B17), or 0x2B18 (which changes nothing) without a team.
    fn team_members(&self, me: u32) -> Message {
        match self.teams.of(me) {
            Some(t) => t.member_list(|id| self.is_online(id)),
            None => Message::reply(0x2B18).with("error_code", Value::U32(NO_TEAM)),
        }
    }

    /// The team requests (the community agent's modes 5-11, 0x4A6490 on). Each reply's error twin is the id after it,
    /// with an error_code. Members' clients ask for the team again after each push (0x2B07-0x2B0B).
    fn team_request(&mut self, (me, my_name): (u32, &str), id: u32, msg: &Message) -> Message {
        // Some ids come as text (live: 0x2B00's to_id and team_id).
        let int = |k: &str| msg.get(k).and_then(|v| v.as_u32().or_else(|| v.as_str()?.trim().parse().ok())).unwrap_or(0);
        let fail = |reply: u32, code: u32| Message::reply(reply + 1).with("error_code", Value::U32(code));
        let mine = self.teams.of(me).cloned();
        let my_authority = mine.as_ref().and_then(|t| t.authority(me)).unwrap_or(MEMBER);
        match id {
            // Form Team {team_name} -> 0x2B0F {team_name}; 409 the name is taken, 808 bad characters.
            0x2AF9 => {
                let name = msg.get("team_name").and_then(Value::as_str).unwrap_or_default();
                if mine.is_some() {
                    return fail(0x2B0F, 0);
                }
                let created = jst_now().format("%Y-%m-%d %H:%M:%S").to_string();
                match self.teams.form((me, my_name), &name, created) {
                    Ok(_) => Message::reply(0x2B0F).with("team_name", Value::text(name.trim())),
                    Err(code) => fail(0x2B0F, code),
                }
            }
            // Invite {to_id} -> 0x2B19; the player gets 0x2AFF on the bell. 106 "This player cannot receive another
            // request." when they are in a team or invited already; any other code "A transmission error has occurred.".
            0x2AFE => {
                let to = int("to_id");
                let Some(team) = mine.filter(|_| my_authority >= SUBLEADER) else { return fail(0x2B19, 0) };
                if self.teams.of(to).is_some() || self.teams.invites.contains_key(&to) {
                    return fail(0x2B19, REQUEST_WAITING);
                }
                if !self.is_online(to) {
                    return fail(0x2B19, NOT_ONLINE);
                }
                self.teams.invites.insert(to, (me, team.id));
                let invite = Message::reply(0x2AFF)
                    .with("from_id", Value::U32(me))
                    .with("from_name", Value::text(my_name))
                    .with("team_id", Value::U32(team.id))
                    .with("team_name", Value::text(&team.name));
                self.push(to, &invite);
                Message::reply(0x2B19)
            }
            // The inviter cancels -> 0x2B1F; the invited player gets 0x2B03.
            0x2B02 => {
                let invited: Vec<u32> = self.teams.invites.iter().filter(|(_, (from, _))| *from == me).map(|(&to, _)| to).collect();
                for to in invited {
                    self.teams.invites.remove(&to);
                    self.push(to, &Message::reply(0x2B03));
                }
                Message::reply(0x2B1F)
            }
            // The answer {to_id: the inviter, team_id, answer: 2 yes, 1 no} -> 0x2B1D; the inviter gets 0x2B01
            // {answer}, and with a yes the members 0x2B08 {member_name}.
            0x2B00 => {
                let Some((from, team_id)) = self.teams.invites.remove(&me) else { return fail(0x2B1D, 0) };
                let answer = int("answer");
                let joined = answer == ACCEPTED && mine.is_none() && self.teams.get(team_id).is_some();
                self.push(from, &Message::reply(0x2B01).with("answer", Value::U32(if joined { ACCEPTED } else { answer })));
                if joined {
                    self.teams.join(team_id, (me, my_name));
                    let members = self.teams.get(team_id).map(|t| t.ids()).unwrap_or_default();
                    self.tell_team(&members, Some(me), &Message::reply(0x2B08).with("member_name", Value::text(my_name)));
                }
                Message::reply(0x2B1D)
            }
            // Disband (the leader) -> 0x2B11; the others get 0x2B07 {team_name}.
            0x2AFA => {
                let Some(team) = mine.filter(|_| my_authority == LEADER) else { return fail(0x2B11, 0) };
                self.teams.disband(team.id);
                self.tell_team(&team.ids(), Some(me), &Message::reply(0x2B07).with("team_name", Value::text(&team.name)));
                Message::reply(0x2B11)
            }
            // Kick {to_id} (someone of lower authority) -> 0x2B25; everyone, the one kicked too, gets 0x2B0A.
            0x2B05 => {
                let to = int("to_id");
                let Some(team) = mine.filter(|t| t.authority(to).is_some_and(|a| a < my_authority)) else { return fail(0x2B25, 0) };
                let kicked = team.member(to).map(|m| m.name.clone()).unwrap_or_default();
                self.teams.remove(to);
                let push = Message::reply(0x2B0A).with("admin_name", Value::text(my_name)).with("member_name", Value::text(&kicked));
                self.tell_team(&team.ids(), Some(me), &push);
                Message::reply(0x2B25)
            }
            // Leave (not the leader) -> 0x2B23; the others get 0x2B09 {member_name}.
            0x2B04 => {
                let Some(team) = mine.filter(|_| my_authority != LEADER) else { return fail(0x2B23, 0) };
                self.teams.remove(me);
                self.tell_team(&team.ids(), Some(me), &Message::reply(0x2B09).with("member_name", Value::text(my_name)));
                Message::reply(0x2B23)
            }
            // Authority {to_id, authority: 1 subleader, 0 member, 2 the leadership} (the leader) -> 0x2B27; everyone
            // gets 0x2B0B {authority, to_name}. Handing over the leadership makes the old leader a member (**guess**).
            0x2B06 => {
                let (to, authority) = (int("to_id"), int("authority"));
                let Some(team) = mine.filter(|t| my_authority == LEADER && to != me && t.member(to).is_some()) else {
                    return fail(0x2B27, 0);
                };
                if authority > LEADER {
                    return fail(0x2B27, 0);
                }
                self.teams.set_authority(team.id, to, authority);
                if authority == LEADER {
                    self.teams.set_authority(team.id, me, MEMBER);
                }
                let name = team.member(to).map(|m| m.name.clone()).unwrap_or_default();
                let push = Message::reply(0x2B0B).with("authority", Value::U32(authority)).with("to_name", Value::text(&name));
                self.tell_team(&team.ids(), Some(me), &push);
                Message::reply(0x2B27)
            }
            _ => fail(id, 0),
        }
    }

    /// 0x2580 {mail_type, title, body, to_id} (0x4AE8E0): 0x2584 {mail_id}, the mail kept for its recipient, who gets
    /// 0x2581 if online (flag 0x1000, the bell's mail icon); or 0x2585 {error_code}: 301 "There are no Players with this
    /// name.", 700 "A transmission error has occurred.". Team mail (mail_type 3, no to_id) goes to every other member.
    fn send_mail(&mut self, from: (u32, String), kind: u32, to_id: u32, title: &str, body: &str) -> Message {
        let refuse = |code: u32| Message::reply(0x2585).with("error_code", Value::U32(code));
        let recipients: Vec<(u32, String)> = match kind {
            MAIL_TO_PLAYER => vec![(to_id, self.social.name(to_id))],
            MAIL_TO_TEAM => match self.teams.of(from.0) {
                Some(t) => t.members.iter().filter(|m| m.id != from.0).map(|m| (m.id, m.name.clone())).collect(),
                None => return refuse(700),
            },
            _ => return refuse(700),
        };
        if recipients.iter().any(|(_, name)| name.is_empty()) {
            return refuse(301);
        }
        let team = self.teams.of(from.0).filter(|_| kind == MAIL_TO_TEAM).map(|t| (t.id, t.name.clone())).unwrap_or_default();
        let sent = jst_now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut mail_id = 0;
        for to in recipients {
            let to_id = to.0;
            let mail = Mail { kind, from: from.clone(), to, title: title.into(), body: body.into(), sent: sent.clone(), team: team.clone(), ..Mail::default() };
            let id = self.mail.post(mail);
            mail_id = if mail_id == 0 { id } else { mail_id };
            self.push(to_id, &Message::reply(0x2581));
        }
        Message::reply(0x2584).with("mail_id", Value::U32(mail_id))
    }

    /// Makes two players friends (as an accepted request does).
    pub fn befriend(&mut self, (a, a_name): (u32, &str), (b, b_name): (u32, &str)) {
        self.social.seen(a, a_name);
        self.social.seen(b, b_name);
        self.social.befriend(a, b);
    }

    /// A player's friends, with their names.
    pub fn friends_of(&self, player_id: u32) -> Vec<(u32, String)> {
        self.social.friends(player_id).into_iter().map(|id| (id, self.social.name(id))).collect()
    }

    fn is_online(&self, player_id: u32) -> bool {
        self.online.values().any(|o| o.player_id == player_id)
    }

    /// The friend list 0x251C and ignore list 0x252A a player logs in with.
    fn social_lists(&self, player_id: u32) -> [Message; 2] {
        let online = |id| self.is_online(id);
        [self.social.friend_list(player_id, online), self.social.ignore_list(player_id, online)]
    }

    /// A player came or went: their online friends get their friend list again, with the new login_flag.
    fn tell_friends(&self, player_id: u32) {
        for friend in self.social.friends(player_id) {
            self.push(friend, &self.social.friend_list(friend, |id| self.is_online(id)));
        }
    }

    /// 0x24B9 {to_id}: a friend request, pushed to that player as 0x24BA {from_id, from_name} (their bell's
    /// notification) -> 0x251E; 0x251F {error_code} when they are not online or a request already waits for them.
    fn ask_friend(&mut self, me: u32, to: u32) -> Message {
        let error = |code| Message::reply(0x251F).with("error_code", Value::U32(code));
        if !self.is_online(to) {
            return error(NOT_ONLINE);
        }
        if self.social.requests.contains_key(&to) || self.social.are_friends(me, to) || to == me {
            return error(REQUEST_WAITING);
        }
        self.social.requests.insert(to, me);
        let ask = Message::reply(0x24BA).with("from_id", Value::U32(me)).with("from_name", Value::text(&self.social.name(me)));
        self.push(to, &ask);
        Message::reply(0x251E)
    }

    /// 0x24BB {to_id, answer}: the answer to `from`'s request -> 0x2522 {answer, friend_id, friend_name}, and `from`'s
    /// waiting dialog gets 0x24BC with this player as the friend; answer 2 makes them friends. 0x2523 when no request
    /// waits (it was withdrawn).
    fn answer_friend(&mut self, me: u32, from: u32, answer: u32) -> Message {
        if self.social.requests.get(&me) != Some(&from) {
            return Message::reply(0x2523).with("error_code", Value::U32(0));
        }
        self.social.requests.remove(&me);
        if answer == ACCEPTED {
            self.social.befriend(me, from);
        }
        let told = |reply, friend: u32| {
            Message::reply(reply)
                .with("answer", Value::U32(answer))
                .with("friend_id", Value::U32(friend))
                .with("friend_name", Value::text(&self.social.name(friend)))
        };
        self.push(from, &told(0x24BC, me));
        told(0x2522, from)
    }

    /// 0x24BD: the player's waiting request is withdrawn -> 0x2524; the player asked gets 0x24BE (their dialog says so).
    fn withdraw_friend_request(&mut self, me: u32) -> Message {
        let asked: Vec<u32> = self.social.requests.iter().filter(|(_, from)| **from == me).map(|(to, _)| *to).collect();
        for to in asked {
            self.social.requests.remove(&to);
            self.push(to, &Message::reply(0x24BE).with("from_id", Value::U32(me)));
        }
        Message::reply(0x2524)
    }

    /// 0x24BF {to_id}: the friendship ends -> 0x2528 {to_id}, and the other player gets 0x2528 {to_id: this player}.
    fn unfriend(&mut self, me: u32, other: u32) -> Message {
        self.social.unfriend(me, other);
        self.push(other, &Message::reply(0x2528).with("to_id", Value::U32(me)));
        Message::reply(0x2528).with("to_id", Value::U32(other))
    }

    /// 0x24C2 / 0x24C3 {to_id}: onto the ignore list -> 0x252C {to_id, to_name}, or off it -> 0x252E {to_id}.
    fn ignore(&mut self, me: u32, other: u32, add: bool) -> Message {
        self.social.ignore(me, other, add);
        let reply = if add { 0x252C } else { 0x252E };
        let m = Message::reply(reply).with("to_id", Value::U32(other));
        if add { m.with("to_name", Value::text(&self.social.name(other))) } else { m }
    }

    /// A duel between players is over (with its winner, if it had one): its match and table go, and an event's round
    /// goes on.
    pub fn end_match(&mut self, duel_id: u32, winner: Option<u32>) {
        self.matches.remove(&duel_id);
        self.tables.remove(&duel_id);
        let Some(at) = self.events.iter().position(|e| e.round.iter().any(|(d, _)| *d == duel_id)) else { return };
        let event = &mut self.events[at];
        let Some(i) = event.round.iter().position(|(d, _)| *d == duel_id) else { return };
        let (_, players) = event.round.remove(i);
        // The loser is out; without a winner (nobody started) both are.
        event.alive.retain(|p| !players.contains(p) || Some(*p) == winner);
        if let Some(w) = winner {
            event.advance(w);
            event.second = players.into_iter().find(|p| *p != w);
        }
        if event.round.is_empty() {
            self.next_round(at, false);
        } else {
            self.save_events();
        }
    }

    /// The duels of an event's round under way (tests).
    pub fn event_duels(&self, event_id: u32) -> Vec<u32> {
        self.events.iter().find(|e| e.id == event_id).map(|e| e.round.iter().map(|(d, _)| *d).collect()).unwrap_or_default()
    }

    /// A duel of an event's round has begun (a player sat at its table): the lobby's status goes to "Duel in Progress".
    pub fn duel_began(&mut self, duel_id: u32) {
        let Some(at) = self.events.iter().position(|e| e.round.iter().any(|(d, _)| *d == duel_id)) else { return };
        if !self.events[at].dueling {
            self.events[at].dueling = true;
            self.refresh_lobby(at, &[6]);
        }
    }

    /// Tells an event's players to ask the lobby again (`0x32D4 {flag}`, `0x4CC5B0`): 2 the entries and phase blocks,
    /// 3 the bracket, 4 the timer, 5 the counts, 6 the state.
    fn refresh_lobby(&self, at: usize, flags: &[u32]) {
        for p in self.events[at].players() {
            for flag in flags {
                self.push(p, &Message::reply(0x32D4).with("flag", Value::U32(*flag)));
            }
        }
    }

    /// An entry or a leaving while the event is open: the others' lobbies ask for the entries, the bracket and the counts
    /// again.
    fn refresh_others(&self, at: usize, me: u32) {
        for p in self.events[at].players().into_iter().filter(|p| *p != me) {
            for flag in [2, 3, 5] {
                self.push(p, &Message::reply(0x32D4).with("flag", Value::U32(flag)));
            }
        }
    }

    /// The round banner (`0x32D3 {round, total_round, status}`: 1 "Starting ...", 0 "... has now finished"; the lobby
    /// reads `total_round` as the round's name, see `Event::round_label`).
    fn round_banner(&self, at: usize, starting: bool) {
        let event = &self.events[at];
        let label = Value::U32(event.round_label());
        let banner = Message::reply(0x32D3)
            .with("round", label.clone())
            .with("total_round", label)
            .with("status", Value::U32(starting as u32));
        for p in event.players() {
            self.push(p, &banner);
        }
    }

    /// A round is over, or the event has just started: the winner if one player is left, else the next round after
    /// the wait (the lobby shows "Match-ups coming up" and the timer `0x3301` meanwhile; `immediate` skips it).
    fn next_round(&mut self, at: usize, immediate: bool) {
        let alive = self.events[at].alive.clone();
        if self.events[at].round_number > 0 {
            self.round_banner(at, false);
        }
        if alive.len() <= 1 {
            self.finish_event(at);
            return;
        }
        if self.events[at].round_number > 0 && !immediate && !self.round_delay.is_zero() {
            let event = &mut self.events[at];
            event.waiting = true;
            event.dueling = false;
            event.next_at = Some(std::time::Instant::now() + self.round_delay);
            for p in self.events[at].players() {
                self.push(p, &Message::reply(0x3301).with("timer", Value::U32(self.round_delay.as_secs() as u32)));
            }
            self.refresh_lobby(at, &[6, 5, 3]);
            return;
        }
        self.start_round(at);
    }

    /// Pairs an event's players still in for the next round, each pair getting the match push 0x1978 as the matching
    /// agent does (TournamentScene's 0x4CC60C reads it the same way), the last one without a partner going through.
    /// The lobby is told: the round's banner, the bracket, the state.
    fn start_round(&mut self, at: usize) {
        let alive = self.events[at].alive.clone();
        self.events[at].round_number += 1;
        self.events[at].dueling = false;
        let event = self.events[at].clone();
        let rule: Vec<(String, Value)> = [
            ("match_type", "match_type"),
            ("regulation_id", "regulation_id"),
            ("lp_start", "duel_lp_start"),
            ("surrender", "duel_surrender"),
            ("time_turn", "duel_time_turn"),
            ("time_total", "duel_time_total"),
        ]
        .into_iter()
        .map(|(k, from)| (k.to_string(), Value::U32(event.int(from))))
        .collect();
        let times = Some((event.int("duel_time_turn"), event.int("duel_time_total"))).filter(|(turn, _)| *turn > 0);
        let mut round = vec![];
        for pair in alive.chunks(2) {
            let [a, b] = pair else {
                // The one without a partner goes through.
                self.events[at].advance(pair[0]);
                continue;
            };
            let records = [*a, *b].map(|p| self.online.values().find(|o| o.player_id == p).map(|o| o.record.clone()));
            let [Some(ra), Some(rb)] = records else {
                // A player gone forfeits; the other goes through.
                let (gone, stays) = if records[0].is_none() { (*a, *b) } else { (*b, *a) };
                self.events[at].alive.retain(|p| *p != gone);
                self.events[at].advance(stays);
                continue;
            };
            self.last_duel += 1;
            let duel_id = self.last_duel;
            let push = |vs: &Message| {
                let data = rule.iter().fold(Message::record().with("name", Value::text("duelData")), |m, (k, v)| m.with(k, v.clone()));
                Message::reply(0x1978).item(vs.clone().with("name", Value::text("vsPlayerData"))).item(data.with("duel_id", Value::U32(duel_id)))
            };
            self.push(*a, &push(&rb));
            self.push(*b, &push(&ra));
            self.matches.insert(duel_id, Match { players: [*a, *b], rule: rule.clone(), times, watchable: event.int("duel_watch") != 0 });
            round.push((duel_id, [*a, *b]));
        }
        self.events[at].round = round;
        if self.events[at].round.is_empty() {
            self.next_round(at, true);
            return;
        }
        self.round_banner(at, true);
        self.refresh_lobby(at, &[6, 5, 2, 3]);
        self.save_events();
    }

    /// The last player is left: the winner and the runner-up (`0x3307`), and the lobby's final state.
    fn finish_event(&mut self, at: usize) {
        self.events[at].status = crate::events::FINISHED;
        self.events[at].round = vec![];
        let event = &self.events[at];
        let (winner, second) = (event.alive.first().copied().unwrap_or(0), event.second.unwrap_or(0));
        let push = Message::reply(0x3307)
            .with("winner", Value::U32(winner))
            .with("winner_name", Value::text(&event.name(winner)))
            .with("second", Value::U32(second))
            .with("second_name", Value::text(&event.name(second)));
        for p in event.players() {
            self.push(p, &push);
        }
        self.refresh_lobby(at, &[6, 5, 3]);
        self.save_events();
    }

    /// The user event requests (UserEventScene 0x4D9BB0, the lobby 0x4CBD70). Each reply's error twin is the id after
    /// it, with an error_code.
    fn event_request(&mut self, (me, my_name): (u32, &str), id: u32, msg: &Message) -> Message {
        use crate::events::{Event, OPEN, RUNNING};
        let int = |k: &str| msg.get(k).and_then(|v| v.as_u32().or_else(|| v.as_str()?.trim().parse().ok())).unwrap_or(0);
        let fail = |reply: u32, code: u32| Message::reply(reply + 1).with("error_code", Value::U32(code));
        let event_id = int("event_id");
        let hosted = self.events.iter().find(|e| e.host.0 == me && e.status != crate::events::FINISHED).map(|e| e.id);
        match id {
            // The player's own event: remain 1 keeps its id (and turns Create off).
            0x339A => Message::reply(0x33FE).with("remain", Value::U32(hosted.is_some() as u32)).with("event_id", Value::U32(hosted.unwrap_or(0))),
            // May the player create one: 0, or bit 8 "already in another event" (0x3E88).
            0x339B => {
                let busy = self.events.iter().any(|e| e.status != crate::events::FINISHED && e.entrants.iter().any(|(p, _)| *p == me));
                Message::reply(0x3400).with("create_status", Value::U32(if busy { 8 } else { 0 }))
            }
            // The Create form -> 0x33F4 {event_id}; the client then enters it (0x1784, 0x3397).
            0x3390 => {
                let fields: Vec<(String, Value)> = msg.fields.iter().chain(msg.items.first().map(|i| &i.fields).into_iter().flatten()).filter(|(k, _)| k != "request").cloned().collect();
                // User event ids start above the built-in ones (Create Duel and NPC duels 1, Auto Duel 2).
                self.last_event = self.last_event.max(FIRST_USER_EVENT - 1) + 1;
                self.events.push(Event::new(self.last_event, (me, my_name.into()), fields));
                self.save_events();
                Message::reply(0x33F4).with("event_id", Value::U32(self.last_event))
            }
            // An entry -> 0x33FA; the others get 0x3399 {entry_name, entry_type 1}.
            0x3397 => {
                let Some(event) = self.events.iter_mut().find(|e| e.id == event_id && e.status == OPEN) else { return fail(0x33FA, 0) };
                if event.entrants.iter().any(|(p, _)| *p == me) {
                    return Message::reply(0x33FA);
                }
                if event.entrants.len() >= event.capacity() {
                    return fail(0x33FA, 0);
                }
                event.entrants.push((me, my_name.into()));
                let others: Vec<u32> = event.entrants.iter().map(|(p, _)| *p).filter(|&p| p != me).collect();
                let push = Message::reply(0x3399).with("entry_name", Value::text(my_name)).with("entry_type", Value::U32(1));
                for p in others {
                    self.push(p, &push);
                }
                if let Some(at) = self.events.iter().position(|e| e.id == event_id) {
                    self.refresh_others(at, me);
                }
                self.save_events();
                Message::reply(0x33FA)
            }
            // Leaving (not the host) -> 0x178F; the others get 0x3399 {entry_type 2}.
            0x178E => {
                let Some(event) = self.events.iter_mut().find(|e| e.id == event_id) else { return Message::reply(0x178F) };
                if event.host.0 == me {
                    return fail(0x178F, 0);
                }
                event.entrants.retain(|(p, _)| *p != me);
                event.alive.retain(|p| *p != me);
                let others: Vec<u32> = event.entrants.iter().map(|(p, _)| *p).collect();
                let push = Message::reply(0x3399).with("entry_name", Value::text(my_name)).with("entry_type", Value::U32(2));
                for p in others {
                    self.push(p, &push);
                }
                if let Some(at) = self.events.iter().position(|e| e.id == event_id) {
                    self.refresh_others(at, me);
                }
                self.save_events();
                Message::reply(0x178F)
            }
            // The host cancels -> 0x33F6; the others get 0x3394 and leave the lobby.
            0x3393 => {
                let Some(at) = self.events.iter().position(|e| e.id == event_id && e.host.0 == me) else { return fail(0x33F6, 0) };
                let event = self.events.remove(at);
                for (p, _) in event.entrants.iter().filter(|(p, _)| *p != me) {
                    self.push(*p, &Message::reply(0x3394));
                }
                self.save_events();
                Message::reply(0x33F6)
            }
            // The host starts it (two players at least) -> 0x33F8; everyone gets 0x3396, then the first round's pairs
            // their match pushes.
            0x3395 => {
                let Some(at) = self.events.iter().position(|e| e.id == event_id && e.host.0 == me && e.status == OPEN) else { return fail(0x33F8, 0) };
                if self.events[at].entrants.len() < 2 {
                    return fail(0x33F8, 0);
                }
                let event = &mut self.events[at];
                event.status = RUNNING;
                event.alive = event.entrants.iter().map(|(p, _)| *p).collect();
                event.slots = event.entrants.clone();
                event.wins.clear();
                for p in event.alive.clone() {
                    self.push(p, &Message::reply(0x3396));
                }
                self.next_round(at, true);
                Message::reply(0x33F8)
            }
            // The lobby: the event (official 0, its name, the host), the entries, and the official tournament's data,
            // which a user event doesn't use.
            // The lobby's event (0x4CBD70: official 0, the host's player_id), which its Event Menu (the menu bar's duel
            // button) also shows: the list row's fields.
            0x177F => match self.events.iter().find(|e| e.id == event_id) {
                Some(e) => Message::reply(0x1780).item(e.row().with("official", Value::U32(0))),
                None => fail(0x1780, 0),
            },
            // The entries (0x4D13F0): the seats of the bracket, each with what 0x4CEBE0 shows of them (name, face).
            0x32CA => {
                let Some(event) = self.events.iter().find(|e| e.id == event_id) else { return Message::reply(0x32FD).with("total_num", Value::U32(0)) };
                let seats = event.seats();
                let list = Message::reply(0x32FD).with("total_num", Value::U32(seats.len() as u32));
                seats.iter().skip(int("start_index") as usize).fold(list, |m, (p, name)| {
                    let online = self.online.values().find(|o| o.player_id == *p);
                    let field = |k: &str| online.and_then(|o| o.record.get(k).cloned());
                    let item = Message::record().with("player_id", Value::U32(*p)).with("player_name", Value::text(name)).with("remain", Value::U32(0));
                    let item = field("avatar1").into_iter().fold(item, |i, a| i.with("avatar1", a));
                    m.item(field("country").into_iter().fold(item, |i, c| i.with("country", c)))
                })
            }
            // The lobby's state (0x4CC5B0): the tournament (0x32FB), its timer (0x3301), the counts (0x3303) and the
            // blocks (0x3305). A user event has no official tournament data.
            0x32C9 => {
                let event = self.events.iter().find(|e| e.id == event_id);
                let winner = event.and_then(|e| e.winner());
                let second = event.filter(|_| winner.is_some()).and_then(|e| e.second);
                let name = |p: Option<u32>| p.and_then(|p| event.map(|e| e.name(p))).unwrap_or_default();
                // status is the lobby's own state (+0x2F8): above 1 the event has begun and the host's Start/Cancel go
                // (0x4CE670); 0 open, 6 "Prepare to Duel", 7 "Duel in Progress", 8 "Event Completed". `total_round` is the
                // round's name (0x3BCD + n, see Event::round_label), shown beside the status.
                let label = event.map_or(0, |e| e.round_label());
                Message::reply(0x32FB)
                    .with("event_id", Value::U32(event_id))
                    .with("status", Value::U32(event.map_or(0, |e| e.lobby_status())))
                    .with("phase", Value::U32(0))
                    .with("round", Value::U32(label))
                    .with("total_round", Value::U32(label))
                    .with("winner", Value::U32(winner.unwrap_or(0)))
                    .with("winner_name", Value::text(&name(winner)))
                    .with("second", Value::U32(second.unwrap_or(0)))
                    .with("second_name", Value::text(&name(second)))
            }
            // The time to the next round (0x3301, shown as the lobby's TIME by 0x4CEAD0).
            0x32CC => {
                let left = self.events.iter().find(|e| e.id == event_id).and_then(|e| e.next_at).map_or(0, |t| t.saturating_duration_since(std::time::Instant::now()).as_secs());
                Message::reply(0x3301).with("timer", Value::U32(left as u32))
            }
            0x32CD => {
                let event = self.events.iter().find(|e| e.id == event_id);
                Message::reply(0x3303)
                    .with("alive_num", Value::U32(event.map_or(0, |e| if e.alive.is_empty() { e.entrants.len() } else { e.alive.len() }) as u32))
                    .with("entry_num", Value::U32(event.map_or(0, |e| e.seats().len()) as u32))
                    .with("watch_num", Value::U32(0))
            }
            // The blocks the round plays in (0x3305, lit on the bracket map by 0x4CF4A0; -1 for none).
            0x32CE => {
                let blocks = self.events.iter().find(|e| e.id == event_id).map_or([u32::MAX; 3], |e| e.phase_blocks());
                let reply = Message::reply(0x3305).with("remain", Value::U32(0));
                ["phase_block1", "phase_block2", "phase_block3"].into_iter().zip(blocks).fold(reply, |m, (k, b)| m.with(k, Value::U32(b)))
            }
            // The bracket map: a flag byte per block (0x49 of them, 0x4D0F40 copies them unchecked and 0x4CF4A0 lights the
            // non-zero ones; the lobby shows block 0 first when its flag is set); without it the lobby crashed (live,
            // NULL at 0x4D10A9).
            0x32D5 => Message::reply(0x3309).with("map_data", Value::Bin(self.events.iter().find(|e| e.id == event_id).map_or(vec![0; crate::events::MAP_BLOCKS], |e| e.map_data()))),
            // A block of the bracket (0x4D10F0 asks for the one at its index, 0x4CEBE0 draws it): item `block_number`.
            0x32CB => {
                let block = self.events.iter().find(|e| e.id == event_id).and_then(|e| e.block(int("block_index")));
                block.into_iter().fold(Message::reply(0x32FF), |m, b| m.item(b))
            }
            // The event list (search_flag 1 all; the other criteria aren't kept): open and running events.
            0x1789 => {
                let shown: Vec<&Event> = self.events.iter().filter(|e| e.status != crate::events::FINISHED).collect();
                let page = shown.iter().skip(int("base_point") as usize).take((int("front_num") as usize).max(1));
                page.fold(Message::reply(0x177B).with("result_number", Value::U32(shown.len() as u32)), |m, e| m.item(e.row()))
            }
            _ => fail(id, 0),
        }
    }

    fn leave(&mut self, id: u64) {
        if let Some(o) = self.online.remove(&id) {
            // Friend requests end with them: one they made is withdrawn, one waiting for them fails (they left).
            if !self.is_online(o.player_id) {
                self.withdraw_friend_request(o.player_id);
                if let Some(from) = self.social.requests.remove(&o.player_id) {
                    self.push(from, &Message::reply(0x24C0).with("error_code", Value::U32(NOT_ONLINE)));
                }
                self.tell_friends(o.player_id);
            }
            self.waiting.retain(|w| w.player_id != o.player_id);
            self.hosts.retain(|_, h| h.player_id != o.player_id);
            self.trade_invites.retain(|to, from| *to != o.player_id && *from != o.player_id);
            self.end_trade(o.player_id);
            self.rental_invites.retain(|to, from| *to != o.player_id && *from != o.player_id);
            self.end_rental_scene(o.player_id);
            let withdrawn: Vec<(u32, u32)> = self.hosts.iter().filter(|(_, h)| h.offer == Some(o.player_id)).map(|(&d, h)| (d, h.player_id)).collect();
            for (duel_id, host) in withdrawn {
                self.withdraw(duel_id, host);
            }
        }
    }

    /// A player is about to be deleted: they leave their team (a team's leader can't be deleted: false).
    fn forget_player(&mut self, player_id: u32) -> bool {
        if self.teams.of(player_id).and_then(|t| t.authority(player_id)).is_some_and(|a| a == LEADER) {
            return false;
        }
        if let Some(team) = self.teams.remove(player_id) {
            let name = self.social.name(player_id);
            self.tell_team(&team.ids(), Some(player_id), &Message::reply(0x2B09).with("member_name", Value::text(&name)));
        }
        true
    }

    /// A challenger's offer is gone: the host's offer dialog closes (0x17C0) and it waits again.
    fn withdraw(&mut self, duel_id: u32, host: u32) {
        if let Some(h) = self.hosts.get_mut(&duel_id) {
            h.offer = None;
        }
        self.push(host, &Message::reply(0x17C0).with("duel_id", Value::U32(duel_id)));
    }

    /// A player search entry with the player's duel: a hosted duel's settings, or the duel they play.
    fn duel_fields(&self, player_id: u32, entry: Message) -> Message {
        if let Some((&duel_id, host)) = self.hosts.iter().find(|(_, h)| h.player_id == player_id) {
            let setting = |k: &str| host.setting(k).and_then(Value::as_u32).unwrap_or(0);
            let entry = ["match_type", "regulation_id", "watch", "party", "level", "lang_mask", "surrender", "lp_start", "time_turn", "time_total"]
                .into_iter()
                .fold(entry, |m, k| m.with(&format!("duel_{k}"), Value::U32(setting(k))));
            return entry
                .with("duel_status", Value::U32(HOSTING))
                .with("duel_id", Value::U32(duel_id))
                .with("duel_password_flag", Value::U32(!host.password().is_empty() as u32));
        }
        // Duelling: PlayerInfoWindow offers Watch when duel_watch isn't 0 (0x4B6510), and shows the rule format.
        match self.in_match(player_id).and_then(|duel_id| Some((duel_id, self.matches.get(&duel_id)?))) {
            Some((duel_id, m)) => {
                let match_type = m.rule.iter().find(|(k, _)| k == "match_type").and_then(|(_, v)| v.as_u32()).unwrap_or(SINGLE_MODE);
                entry
                    .with("duel_status", Value::U32(DUELLING))
                    .with("duel_id", Value::U32(duel_id))
                    .with("duel_watch", Value::U32(m.watchable as u32))
                    .with("duel_match_type", Value::U32(match_type))
            }
            None => entry,
        }
    }

    /// Whether a player is in a duel between players.
    fn in_match(&self, player_id: u32) -> Option<u32> {
        self.matches.iter().find(|(_, m)| m.players.contains(&player_id)).map(|(&d, _)| d)
    }

    /// Queues a player for an automatic match, or matches them with the first player waiting for the same kind of
    /// duel: then the new duel's id, the opponent, and both match pushes (the opponent's first).
    fn wait_for_match(&mut self, me: Waiting) -> Option<(u32, [(u32, Message); 2])> {
        self.waiting.retain(|w| w.player_id != me.player_id);
        // The same Rule Format matches, and Both matches either (two Both players get a Single duel, **guess**).
        let fits = |a: u32, b: u32| a == b || a == EITHER_MODE || b == EITHER_MODE;
        let Some(at) = self.waiting.iter().position(|w| fits(w.match_type, me.match_type)) else {
            self.waiting.push(me);
            return None;
        };
        let other = self.waiting.remove(at);
        self.last_duel += 1;
        let duel_id = self.last_duel;
        let match_type = match (other.match_type, me.match_type) {
            (EITHER_MODE, EITHER_MODE) => SINGLE_MODE,
            (EITHER_MODE, kind) | (kind, _) => kind,
        };
        let rule = vec![("match_type".to_string(), Value::U32(match_type))];
        let push = |vs: &Message| {
            let data = rule.iter().fold(Message::record().with("name", Value::text("duelData")), |m, (k, v)| m.with(k, v.clone()));
            Message::reply(0x1978).item(vs.clone().with("name", Value::text("vsPlayerData"))).item(data.with("duel_id", Value::U32(duel_id)))
        };
        let pushes = [(other.player_id, push(&me.record)), (me.player_id, push(&other.record))];
        self.matches.insert(duel_id, Match { players: [other.player_id, me.player_id], rule, times: None, watchable: true });
        Some((duel_id, pushes))
    }

    fn set_room(&mut self, id: u64, room: Option<u32>) {
        if let Some(o) = self.online.get_mut(&id) {
            o.room = room;
        }
    }

    fn room_of(&self, id: u64) -> Option<u32> {
        self.online.get(&id).and_then(|o| o.room)
    }

    fn occupancy(&self, room: u32) -> u32 {
        self.online.values().filter(|o| o.room == Some(room)).count() as u32
    }

    /// Sends to everyone in `room` except `except`.
    fn send_room(&self, room: u32, msg: &Message, except: u64) {
        for (_, o) in self.online.iter().filter(|(id, o)| **id != except && o.room == Some(room)) {
            let _ = o.outbox.send(msg.clone());
        }
    }

    /// Pushes to a player's front connection; false if they are not online.
    pub fn push(&self, player_id: u32, msg: &Message) -> bool {
        self.send_player(Some(player_id), None, msg)
    }

    /// Sends to one player by id or name; false if they are not online.
    fn send_player(&self, to_id: Option<u32>, to_name: Option<&str>, msg: &Message) -> bool {
        let target = self.online.values().find(|o| Some(o.player_id) == to_id || Some(o.name.as_str()) == to_name);
        target.is_some_and(|o| o.outbox.send(msg.clone()).is_ok())
    }
}

pub struct FrontSession {
    account: Account,
    world: Arc<World>,
    hub: Arc<Mutex<Hub>>,
    outbox: Outbox,
    opened: bool,
    selected: Option<usize>,
    /// Hub registration once a player is selected.
    online: Option<u64>,
    /// The rule (0x17AC item fields) of each duel this session asked for, by duel_id.
    duels: HashMap<u32, Vec<(String, Value)>>,
    /// The server address this client reached, announced as the duel server's when none is advertised.
    reached_at: Option<std::net::IpAddr>,
    /// The accounts, where a player's saved settings are kept (none: this session's copy only).
    directory: Option<Arc<Mutex<crate::accounts::Directory>>>,
    /// Bought packs and decks not shown yet, and the last present id given.
    presents: VecDeque<crate::shop::Present>,
    last_present: u32,
    rng: u64,
    /// The client's language, from its login (0x1F48 lang).
    lang: u32,
}

impl Drop for FrontSession {
    fn drop(&mut self) {
        if let Some(id) = self.online
            && let Ok(mut hub) = self.hub.lock()
        {
            hub.leave(id);
        }
    }
}

impl FrontSession {
    pub fn new(account: Account, world: Arc<World>, hub: Arc<Mutex<Hub>>, outbox: Outbox) -> FrontSession {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
        let mut session = FrontSession {
            account,
            world,
            hub,
            outbox,
            opened: false,
            selected: None,
            online: None,
            duels: HashMap::new(),
            reached_at: None,
            directory: None,
            presents: VecDeque::new(),
            last_present: 0,
            rng: seed | 1,
            lang: ENGLISH,
        };
        session.apply_country();
        session
    }

    /// Gives the account's players their country: the account's own setting, else the one the language implies.
    fn apply_country(&mut self) {
        let country = if self.account.country.is_empty() { country_for_lang(self.lang).to_string() } else { self.account.country.clone() };
        for p in &mut self.account.players {
            p.country.clone_from(&country);
        }
    }

    /// The server address this client connected to.
    pub fn set_reached_at(&mut self, ip: std::net::IpAddr) {
        self.reached_at = Some(ip);
    }

    /// The accounts the player's saved settings go to.
    pub fn set_directory(&mut self, directory: Arc<Mutex<crate::accounts::Directory>>) {
        self.directory = Some(directory);
    }

    /// Takes the account's players as the accounts keep them now (a player another session made or deleted), the
    /// selected one still selected if it exists.
    fn refresh_players(&mut self) {
        let Some(fresh) = self.directory.as_ref().and_then(|d| d.lock().expect("directory lock").account_of(self.account.user_id)) else {
            return;
        };
        let chosen = self.selected.map(|at| self.account.players[at].player_id);
        self.account.players = fresh.players;
        self.apply_country();
        self.selected = chosen.and_then(|id| self.account.players.iter().position(|p| p.player_id == id));
        if self.selected.is_none()
            && let Some(old) = self.online.take()
        {
            self.hub().leave(old);
        }
    }

    /// The regulation a duel is played under (its rule's `regulation_id`; 0 means the login one).
    fn duel_regulation(&self, duel_id: Option<u32>) -> u32 {
        let find = |rule: &[(String, Value)]| rule.iter().find(|(k, _)| k == "regulation_id").and_then(|(_, v)| v.as_u32());
        if let Some(id) = self.world.test_regulation {
            return id;
        }
        let Some(id) = duel_id else { return 0 };
        match self.duels.get(&id) {
            Some(rule) => find(rule),
            None => self.hub().matches.get(&id).and_then(|m| find(&m.rule)),
        }
        .unwrap_or(0)
    }

    /// Whether the selected player may bring `deck` to the duel: sizes, cards owned (with the ones they borrowed) and
    /// the duel's regulation, as the client checks them itself.
    fn check_deck(&self, duel_id: Option<u32>, deck: &Deck) -> Result<(), crate::deckcheck::Refusal> {
        let me = self.player();
        let saved = self.directory.as_ref().and_then(|d| d.lock().expect("directory lock").player(me.player_id).cloned());
        let mut owned = crate::trade::collection(&self.world, saved.as_ref().unwrap_or(me));
        for row in self.hub().rentals.borrowed_by(me.player_id) {
            *owned.entry(row).or_default() += 1;
        }
        crate::deckcheck::check(&self.world, self.duel_regulation(duel_id), &owned, deck)
    }

    /// The decks the selected player's clients uploaded, as the accounts keep them now.
    fn uploaded_decks(&self) -> Vec<Deck> {
        let Some(at) = self.selected else { return vec![] };
        let me = &self.account.players[at];
        let saved = self.directory.as_ref().and_then(|d| d.lock().expect("directory lock").player(me.player_id).map(|p| p.uploaded.clone()));
        saved.unwrap_or_else(|| me.uploaded.clone())
    }

    /// Changes the selected player's uploaded decks, here and in the accounts.
    fn change_uploaded(&mut self, change: impl Fn(&mut Vec<Deck>)) {
        let Some(at) = self.selected else { return };
        let mut decks = self.uploaded_decks();
        change(&mut decks);
        self.account.players[at].uploaded.clone_from(&decks);
        if let Some(d) = &self.directory {
            d.lock().expect("directory lock").update_player(self.account.players[at].player_id, |p| p.uploaded = decks);
        }
    }

    /// A deck the player picked at Select Deck: kept by name (a newer upload replaces it; the oldest goes when ten are
    /// kept), unless the starter deck's name or an empty deck.
    fn keep_deck(&mut self, deck: Deck) {
        if deck.name.is_empty() || deck.main.is_empty() || self.world.starter_deck.as_ref().is_some_and(|s| s.name == deck.name) {
            return;
        }
        self.change_uploaded(|decks| {
            decks.retain(|d| d.name != deck.name);
            decks.push(deck.clone());
            let extra = decks.len().saturating_sub(10);
            decks.drain(..extra);
        });
    }

    /// The editor deleted a deck: the server forgets its copy.
    fn forget_deck(&mut self, name: &str) {
        self.change_uploaded(|decks| decks.retain(|d| d.name != name));
    }

    /// Select Player's create 0x1F4A {pindex, pname} (builder 0x4F2D50) -> 0x2262, after which the client lists its
    /// players again; 0x2263 for a slot taken, a name taken, empty or too long, or a full account.
    fn create_player(&mut self, msg: &Message) -> Message {
        let index = msg.get("pindex").and_then(Value::as_u32).unwrap_or(u32::MAX);
        let name = msg.get("pname").and_then(Value::as_str).unwrap_or_default().to_string();
        let made = self.directory.as_ref().map(|d| d.lock().expect("directory lock").create_player(self.account.user_id, index, &name));
        match made {
            Some(Ok(())) => {
                self.refresh_players();
                Message::reply(0x2262)
            }
            _ => Message::reply(0x2263),
        }
    }

    /// Select Player's delete 0x1F4B {player_id} (builder 0x4F2EB0) -> 0x2264, after which the client lists its players
    /// again; 0x2265 for a player the account doesn't have, its last player, or the leader of a team (who disbands it
    /// first: **guess**, the original's rules are unknown). A member leaves their team.
    fn delete_player(&mut self, msg: &Message) -> Message {
        let Some(id) = msg.get("player_id").and_then(Value::as_u32) else { return Message::reply(0x2265) };
        let Some(d) = self.directory.clone() else { return Message::reply(0x2265) };
        if !self.account.players.iter().any(|p| p.player_id == id) || !self.hub().forget_player(id) {
            return Message::reply(0x2265);
        }
        let deleted = d.lock().expect("directory lock").delete_player(self.account.user_id, id);
        match deleted {
            Ok(()) => {
                self.refresh_players();
                Message::reply(0x2264)
            }
            Err(_) => Message::reply(0x2265),
        }
    }

    /// The player update 0x2CEF (CharMake's Confirm, 0x51C960): {avatar1 (binary), speech00-22, comment00-01,
    /// title} -> 0x2CF5, or 0x2CF6 without a binary avatar1 (as text it would crash the PlayerData loader).
    fn update_player(&mut self, msg: &Message) -> Message {
        let body = msg.items.first().filter(|i| i.get("avatar1").is_some()).unwrap_or(msg);
        let Some(Value::Bin(avatar)) = body.get("avatar1").cloned() else { return Message::reply(0x2CF6) };
        let text = |k: String| body.get(&k).and_then(Value::as_str).unwrap_or_default();
        let speeches: Vec<String> = (0..SPEECHES).map(|i| text(format!("speech{i:02}"))).collect();
        let comments: Vec<String> = (0..COMMENTS).map(|i| text(format!("comment{i:02}"))).collect();
        let title = body.get("title").and_then(Value::as_u32).unwrap_or(0);
        let apply = move |p: &mut Player| {
            p.avatar = avatar.clone();
            p.speeches = speeches.clone();
            p.comments = comments.clone();
            p.title = title;
        };
        let Some(at) = self.selected else { return Message::reply(0x2CF6) };
        apply(&mut self.account.players[at]);
        if let Some(d) = &self.directory {
            d.lock().expect("directory lock").update_player(self.account.players[at].player_id, apply);
        }
        Message::reply(0x2CF5)
    }

    /// A purchase 0x1B68 {sale_id, item_num, end_time} -> 0x1C24 {dpoint}: the BP left (shown as user_bp), or 0x1C25
    /// (shop buy error) for goods not on sale, a count of 0 or beyond item_max, or too few BP. Each pack's or deck's
    /// cards join the player's collection at once and wait as a present (0x2775) for the client to show.
    fn buy(&mut self, msg: &Message) -> Message {
        let int = |k| msg.get(k).and_then(Value::as_u32).unwrap_or(0);
        let (sale_id, count) = (int("sale_id"), int("item_num"));
        let world = self.world.clone();
        let Some(goods) = crate::shop::Goods::find(&world, sale_id) else { return Message::reply(0x1C25) };
        let (price, max) = goods.price();
        let cost = price * count;
        let Some(at) = self.selected else { return Message::reply(0x1C25) };
        if count == 0 || count > max || cost > self.account.dpoint {
            return Message::reply(0x1C25);
        }
        let item = goods.item();
        if item.is_some_and(|id| self.account.players[at].items.contains(&id)) {
            return Message::reply(0x1C25);
        }
        let bought: Vec<crate::shop::Present> = (0..count).map(|_| goods.take(&mut self.rng)).collect();
        let add = |p: &mut Player| {
            if let Some(id) = item {
                p.items.insert(id);
            }
            for &row in bought.iter().flat_map(crate::shop::Present::cards) {
                *p.cards.entry(row).or_default() += 1;
            }
            for present in &bought {
                if let crate::shop::Present::Deck { id, .. } = present
                    && !p.decks.contains(id)
                {
                    p.decks.push(*id);
                }
            }
        };
        self.account.dpoint -= cost;
        add(&mut self.account.players[at]);
        if let Some(d) = &self.directory {
            let mut d = d.lock().expect("directory lock");
            let dpoint = self.account.dpoint;
            d.update_account(self.account.user_id, |a| a.dpoint = dpoint);
            d.update_player(self.account.players[at].player_id, add);
        }
        self.presents.extend(bought);
        Message::reply(0x1C24).with("dpoint", Value::U32(self.account.dpoint))
    }

    fn hub(&self) -> std::sync::MutexGuard<'_, Hub> {
        self.hub.lock().expect("hub lock")
    }

    /// Handles one client packet and returns the replies to send, in order.
    pub fn handle(&mut self, packet: &Packet) -> Vec<Message> {
        if !self.opened {
            self.opened = true;
            return vec![Message::reply(VERSION_VALID)];
        }
        let Packet::Message(msg) = packet else { return vec![] };
        let Some(id) = msg.request() else { return vec![] };
        if let Some(&(_, reply)) = BARE_REPLIES.iter().find(|(req, _)| *req == id) {
            return vec![Message::reply(reply)];
        }
        // The login gives the client's language (player search's language criterion).
        if id == 0x1F48
            && let Some(lang) = msg.get("lang").and_then(Value::as_u32)
        {
            self.lang = lang;
            self.apply_country();
            if let Some(online) = self.online {
                self.hub().set_lang(online, lang);
            }
        }
        // The login replies and purchases read the BP, which may have grown since this session began (duels pay
        // into the account).
        if matches!(id, 0x1F47 | 0x1F48 | 0x1B68)
            && let Some(bp) = self.directory.as_ref().and_then(|d| d.lock().expect("directory lock").dpoint(self.account.user_id))
        {
            self.account.dpoint = bp;
        }
        let a = &self.account;
        let reply = match id {
            0x1F47 => Message::reply(0x219A)
                .with("regulation_id", Value::U32(self.world.login_regulation))
                .with("user_id", Value::U32(a.user_id))
                .with("uname", Value::text(&a.name))
                .with("dpoint", Value::U32(a.dpoint)),
            // premium (UserData+4, 0x4BF520): every NPC, Form Team and Create Event open.
            0x1F48 => Message::reply(0x2201).with("regulation_id", Value::U32(self.world.login_regulation)).item(
                Message::record()
                    .with("user_id", Value::U32(a.user_id))
                    .with("uname", Value::text(&a.name))
                    .with("dpoint", Value::U32(a.dpoint))
                    .with("premium", Value::U32(1))
                    .with("country", Value::text(a.players.first().map_or("", |p| p.country.as_str()))),
            ),
            0x1F49 => {
                self.refresh_players();
                let hub = self.hub.lock().expect("hub lock");
                self.account.players.iter().fold(Message::reply(0x2260), |m, p| m.item(hub.own_record(p)))
            }
            0x1F4A => self.create_player(msg),
            0x1F4B => self.delete_player(msg),
            0x1F4C => {
                self.selected = self.find_player(msg);
                if self.selected.is_none() {
                    return vec![];
                }
                let mut hub = self.hub.lock().expect("hub lock");
                if let Some(old) = self.online {
                    hub.leave(old);
                }
                let host = self.duel_server().0;
                let id = hub.join(self.player(), self.outbox.clone(), self.lang, host);
                drop(hub);
                self.online = Some(id);
                Message::reply(0x2266)
            }
            0x1FA4 => {
                if self.find_player(msg).or(self.selected).is_none() {
                    return vec![];
                }
                // Players own their starter deck's cards and the cards they bought or got in trades, less the ones
                // traded away. A trade changes the partner's collection in the accounts, so take it from there.
                if let (Some(at), Some(d)) = (self.selected, &self.directory) {
                    let id = self.account.players[at].player_id;
                    let saved = d.lock().expect("directory lock").accounts().flat_map(|(_, a)| &a.players).find(|p| p.player_id == id).cloned();
                    if let Some(saved) = saved {
                        let me = &mut self.account.players[at];
                        (me.cards, me.given) = (saved.cards, saved.given);
                    }
                }
                let trunk = crate::trade::trunk(&self.world, self.player());
                // The client never asks for its friend and black lists; the global dispatcher stores
                // 0x251C in dataset slot 6 and 0x252A in slot 7, and listing any player reads both
                // without a NULL check (0x45E6D0, 0x45E810). Likewise the rental records 0x2D6C (slot 8),
                // which opening the shop reads (0x48F080).
                let [friends, ignored] = self.hub().social_lists(self.player().player_id);
                let rentals = self.hub().rental_records(self.player().player_id);
                return vec![trunk, friends, ignored, rentals];
            }
            // Shop (handler 0x48AA60): its text, then a chain where each reply asks for the next:
            // items (the booster packs), exchange items, protectors, event items, perks, rental terms, avatar
            // items (the last clears isLoading).
            0x1B66 => Message::reply(0x1C20).with("text", Value::text("Welcome! Take a look at the packs.")),
            0x1B67 => {
                let packs = self.world.packs.iter().map(crate::shop::shelf_item);
                let decks = self.world.structure_decks.iter().map(crate::shop::shelf_deck);
                packs.chain(decks).fold(Message::reply(0x1C22), Message::item)
            }
            0x1B68 => self.buy(msg),
            0x1B6F => Message::reply(0x1C26),
            0x1B6B => self.world.protectors.iter().map(|&id| crate::shop::shelf_protector(id)).fold(Message::reply(0x1C32), Message::item),
            0x1B6D => Message::reply(0x1C2E),
            // perk_num is compared with 500 to show _root.expack_del; meaning unknown.
            0x1B72 => Message::reply(0x1C36).with("perk_num", Value::U32(0)),
            0x2C40 => ["rental_cardnum", "rental_point", "rental_term"]
                .into_iter()
                .fold(Message::reply(0x2D75), |m, k| m.with(k, Value::U32(0))),
            0x1B69 => self.world.avatar_items.iter().map(|&(id, _)| crate::shop::shelf_avatar_item(id)).fold(Message::reply(0x1C2A), Message::item),
            // Event list {event_type, status, event_id} (TournamentScene 0x4D0BB0) and event search
            // (UserEventScene 0x4D9E37): no events are held.
            0x177A => Message::reply(0x177B).with("result_number", Value::U32(0)),
            // User events (Satellite Plaza's list and Create, the lobby).
            0x339A | 0x339B | 0x3390 | 0x3397 | 0x178E | 0x3393 | 0x3395 | 0x177F | 0x32C9 | 0x32CA | 0x32CC | 0x32CD | 0x32CE | 0x32D5
            | 0x32CB | 0x1789 => {
                let p = self.player();
                let me = (p.player_id, p.name.clone());
                self.hub().event_request((me.0, &me.1), id, msg)
            }
            // Duel Log (0x499750): the championship list it asks for on opening (its server log tab waits for it), and
            // Server Duel Logs, which would list replays to download; the server keeps none.
            0x1793 => Message::reply(0x177B),
            0x19C8 => Message::reply(0x19C9),
            // Rental Card List (0x481A30) opens once all three come: the rental records, then the borrowed (rental_kind
            // 0) and lent (1) lists. Nobody rents yet.
            0x2C34 => self.hub().rental_records(self.player().player_id),
            0x2C35 => {
                let kind = msg.get("rental_kind").and_then(Value::as_u32).unwrap_or(0);
                self.hub().rental_history(self.player().player_id, kind)
            }
            // A BOOSTERPASS code (BpInputWindow, 0x40F7B0): Konami's codes are gone, so every one is refused (the "ng"
            // frame). The global dispatcher keeps premium from it (0x4F5349), so it stays 1.
            0x20D0 => Message::reply(0x23F0)
                .with("dpauth_result", Value::U32(1))
                .with("dpPoints", Value::U32(a.dpoint))
                .with("dpGetPoints", Value::U32(0))
                .with("premium", Value::U32(1)),
            // The player's own user event (UserEventScene 0x4D9BB0): none.
            // Deck list (deck editor 0x43FC60): it saves each deck it lacks as a local file.
            // The starter deck, then the structure decks the player bought; the client uses at most 10.
            0x1F4E => {
                let owned = self.selected.map(|at| self.account.players[at].decks.clone()).unwrap_or_default();
                let bought = owned.iter().filter_map(|&id| self.world.structure_decks.iter().find(|s| s.deck.id == id)).map(|s| &s.deck);
                let listed: Vec<&Deck> = self.world.starter_deck.iter().chain(bought).collect();
                // Then the decks this player's clients uploaded, unless a listed deck has the name (the client would
                // keep its own copy anyway). The newest account state: another machine may have uploaded since.
                let uploaded = self.uploaded_decks();
                let kept: Vec<&Deck> = uploaded.iter().filter(|u| !listed.iter().any(|l| l.name == u.name)).collect();
                listed.into_iter().chain(kept).take(10).fold(Message::reply(0x22C8), |m, d| m.item(d.record()))
            }
            0x17D4 => return self.duel_ready(msg),
            // Deck updated: the deck editor's save, and the deck a player picks for a duel (DuelManager
            // 0x50ECF0, {duel_id, event_id, update_type 1, item: the deck}), which is kept for the duel.
            0x1F4F => {
                let duel_id = msg.get("duel_id").and_then(Value::as_u32);
                let player_id = self.player().player_id;
                // Select Deck (update_type 1): the deck must pass the check the client makes itself. 0x22CB shows
                // text 0x2919 and leaves the duel scene (0x50EFD5).
                if msg.get("update_type").and_then(Value::as_u32) == Some(1)
                    && let Some(item) = msg.items.first()
                    && let Err(why) = self.check_deck(duel_id, &Deck::from_record(item))
                {
                    tracing::info!("player {player_id}'s deck refused for duel {duel_id:?}: {why:?}");
                    return vec![Message::reply(0x22CB)];
                }
                // Select Deck (update_type 1) sends the whole deck: keep it for the player's other machines.
                if msg.get("update_type").and_then(Value::as_u32) == Some(1)
                    && let Some(item) = msg.items.first()
                {
                    self.keep_deck(Deck::from_record(item));
                }
                if let (Some(id), Some(item)) = (duel_id, msg.items.first()) {
                    let deck = Deck::from_record(item);
                    let mut hub = self.hub();
                    if let Some(rule) = self.duels.get(&id) {
                        let npc_id = rule.iter().find(|(k, _)| k == "cpu_id").and_then(|(_, v)| v.as_u32()).filter(|&id| id > 0);
                        hub.enter_duel(player_id, DuelEntry { deck, npc_id, ..DuelEntry::default() });
                    } else if let Some(m) = hub.matches.get(&id) {
                        let opponent = m.players.into_iter().find(|&p| p != player_id);
                        let rule = |k: &str| m.rule.iter().find(|(n, _)| n == k).and_then(|(_, v)| v.as_u32());
                        let lp = rule("lp_start").filter(|&lp| lp > 0);
                        let best_of = if rule("match_type") == Some(MATCH_MODE) { 3 } else { 1 };
                        let times = m.times;
                        hub.enter_duel(player_id, DuelEntry { deck, duel_id: Some(id), opponent, lp, best_of, times, ..DuelEntry::default() });
                    }
                }
                Message::reply(0x22CA)
            }
            // Deck deleted (its local file is already gone); the starter deck comes back on the next list.
            0x1F50 => {
                let name = msg.get("deck_name").and_then(Value::as_str).unwrap_or_default();
                self.forget_deck(&name);
                Message::reply(0x22CC)
            }
            // NPC roster (kept in slot 4; SelectNpcScene 0x4BEB40 reads it three items per NPC, ids 120 + 3k on).
            // The picked item becomes the opponent's PlayerData. Level j is mastered once the experience with its id
            // (the record's npcbin) reaches its need_ex (0: mastered from the start; 0x4BFDC4 divides by the three
            // summed). npc_type is the series tab: NPCs 0-8 Yu-Gi-Oh!, 9-16 GX, 17-25 5D's. need_ex 100/200/300 and
            // the tabs are guesses.
            0x2710 => self.world.npcs.iter().fold(Message::reply(0x2711), |m, &id| {
                let tab = match id.saturating_sub(120) / 3 {
                    0..=8 => 0,
                    9..=16 => 1,
                    _ => 2,
                };
                m.item(
                    Message::record()
                        .with("npc_type", Value::U32(tab))
                        .with("npc_id", Value::U32(id))
                        .with("need_ex", Value::U32(100 * (id % 3 + 1))),
                )
            }),
            // Duel entry (DuelMatchingAgent step 0x4A1090): join_id comes back in 0x17AC.
            // The global dispatcher (0x4F4AE7) keeps the reply's event_id, which a user event's lobby then asks with. Only
            // a user event's goes back: with Auto Duel's (2) the client queued itself again after every match (live).
            0x1784 => {
                let mut hub = self.hub();
                hub.last_join += 1;
                let reply = Message::reply(0x1785).with("join_id", Value::U32(hub.last_join));
                match msg.get("event_id").and_then(Value::as_u32).filter(|id| hub.events.iter().any(|e| e.id == *id)) {
                    Some(event_id) => reply.with("event_id", Value::U32(event_id)),
                    None => reply,
                }
            }
            // DuelMenu (0x49D310) opens with the number of players duelling, hosting a duel and waiting for an
            // automatic match (menu +0x58, +0x5C, +0x60).
            0x17FC => {
                let hub = self.hub();
                Message::reply(0x17FD)
                    .with("duel_playing", Value::U32(2 * hub.matches.len() as u32))
                    .with("duel_ready", Value::U32(hub.hosts.len() as u32))
                    .with("duel_auto", Value::U32(hub.waiting.len() as u32))
            }
            // Create Duel's Cancel (0x4A2500): {event_id, duel_id}.
            0x17F2 => {
                let player_id = self.player().player_id;
                let duel_id = msg.get("duel_id").and_then(Value::as_u32).unwrap_or(0);
                self.hub().hosts.retain(|&d, h| !(d == duel_id && h.player_id == player_id));
                Message::reply(0x17F3)
            }
            0x17B6 => return self.challenge(msg),
            // The challenger's Cancel while the host decides (0x4A28F0): {event_id, join_id, duel_id}.
            0x17C0 => {
                let player_id = self.player().player_id;
                let duel_id = msg.get("duel_id").and_then(Value::as_u32).unwrap_or(0);
                let mut hub = self.hub();
                if let Some(host) = hub.hosts.get(&duel_id).filter(|h| h.offer == Some(player_id)).map(|h| h.player_id) {
                    hub.withdraw(duel_id, host);
                }
                Message::reply(0x17C1)
            }
            0x17CA => return self.answer_challenge(msg),
            // Automatic matching (DuelMenu's decideAuto, agent mode 2, step 0x4A1380): 0x1799 reads nothing, then the
            // agent waits (0x4A1D10) for the match push 0x1978 (0x4A2D30).
            0x1798 => {
                let player = self.player();
                let match_type = msg.get("match_type").and_then(Value::as_u32).unwrap_or(0);
                let me = Waiting { player_id: player.player_id, match_type, record: player.record() };
                let matched = self.hub().wait_for_match(me);
                if let Some((_, [(other, to_other), (_, to_me)])) = matched {
                    self.hub().push(other, &to_other);
                    let outbox = self.outbox.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(MATCH_PUSH_DELAY);
                        let _ = outbox.send(to_me);
                    });
                }
                Message::reply(0x1799)
            }
            // The wait cancelled (0x4A15D0).
            0x17A2 => {
                let player_id = self.player().player_id;
                self.hub().waiting.retain(|w| w.player_id != player_id);
                Message::reply(0x17A3)
            }
            // Duel (step 0x4A1790): for an NPC duel the client then opens DuelManager. The item holds
            // the duel's rule (cpu_id, match_type, regulation_id, surrender, time_turn, time_total...).
            0x17AC => {
                let duel_id = {
                    let mut hub = self.hub();
                    hub.last_duel += 1;
                    hub.last_duel
                };
                let rule = msg.items.first().map(|i| i.fields.clone()).unwrap_or_default();
                // Create Duel (agent mode 0) sends its settings with a status and no cpu_id; it then waits for
                // challengers (0x4A1D10).
                if rule.iter().any(|(k, _)| k == "status") && !rule.iter().any(|(k, _)| k == "cpu_id") {
                    let player = self.player();
                    let host = Host { player_id: player.player_id, record: player.record(), settings: rule, offer: None };
                    self.hub().hosts.insert(duel_id, host);
                } else {
                    self.duels.insert(duel_id, rule);
                }
                Message::reply(0x17AD).with("duel_id", Value::U32(duel_id))
            }
            0x2072 => Message::reply(0x2398).with("regulations", Value::u16_be_array(&self.world.regulation_ids)),
            0x27D8 => Message::reply(0x27D9).item(Message::record()),
            0x2CEE => {
                let record = self.hub().own_record(self.player());
                Message::reply(0x2CF3).item(record)
            }
            0x2CEF => self.update_player(msg),
            // The manner dialog's answer (see "Manner ratings" in docs/protocol.md): 0x2CF1, or 0x2CF2 {error_code}.
            0x2CED => {
                let int = |k: &str| msg.get(k).and_then(Value::as_u32).unwrap_or(0);
                let me = self.player().player_id;
                if self.hub().rate(me, int("eval_id"), int("eval_data") as i32) {
                    Message::reply(0x2CF1)
                } else {
                    Message::reply(0x2CF2).with("error_code", Value::U32(0))
                }
            }
            // D-Mail (MailWindow 0x4AC6B0): the count waiting (also the startup step 0x4730B0's, which lights the bell),
            // the fetch (the client files each mail by its id), and a send.
            0x2583 => Message::reply(0x2588).with("count", Value::U32(self.hub().mail.count(self.player().player_id))),
            0x2582 => self.hub().mail.take(self.player().player_id),
            0x2580 => {
                let p = self.player();
                let from = (p.player_id, p.name.clone());
                let text = |k: &str| msg.get(k).and_then(Value::as_str).unwrap_or_default();
                let int = |k: &str| msg.get(k).and_then(Value::as_u32).unwrap_or(0);
                let (title, body) = (text("title"), text("body"));
                self.hub().send_mail(from, int("mail_type"), int("to_id"), &title, &body)
            }
            0x1F41 => {
                let now = jst_now().format("%Y-%m-%d %H:%M:%S").to_string();
                let entries = self.world.news.iter().map(|(title, body)| {
                    Message::record()
                        .with("starttime", Value::text(&now))
                        .with("title", Value::text(title))
                        .with("body", Value::text(body))
                });
                let lang = entries.fold(Message::record().with("lang", Value::text("1")), Message::item);
                Message::reply(0x21FC).item(lang)
            }
            0x2CF0 => {
                let found = self.player_search(msg);
                // The left panel's friend and ignore list views (tabs 9 and 10, 0x45B920 / 0x45BA70) search with no
                // paging and mark a reply pending (+0xD7), and their rows are drawn only once none is (0x459BA0): the
                // search reply clears it, then the list fills the tab and draws it (0x458410).
                let view = if msg.get("base_point").is_none() { msg.get("search_flag").and_then(Value::as_u32) } else { None };
                let [friends, ignored] = self.hub().social_lists(self.player().player_id);
                return match view {
                    Some(0x40) => vec![found, friends],
                    Some(0x80) => vec![found, ignored],
                    _ => vec![found],
                };
            }
            // The left panel's friend and ignore list views (0x4F2FB0, 0x4F3090) ask for their lists.
            0x24B8 | 0x24C1 => {
                let [friends, ignored] = self.hub().social_lists(self.player().player_id);
                if id == 0x24B8 { friends } else { ignored }
            }
            // Friends and the ignore list (the community agent 0x4A4EE0).
            0x24B9 | 0x24BB | 0x24BD | 0x24BF | 0x24C2 | 0x24C3 => {
                let me = self.player().player_id;
                let to = msg.get("to_id").and_then(Value::as_u32).unwrap_or(0);
                let mut hub = self.hub();
                match id {
                    0x24B9 => hub.ask_friend(me, to),
                    0x24BB => hub.answer_friend(me, to, msg.get("answer").and_then(Value::as_u32).unwrap_or(0)),
                    0x24BD => hub.withdraw_friend_request(me),
                    0x24BF => hub.unfriend(me, to),
                    _ => hub.ignore(me, to, id == 0x24C2),
                }
            }
            0x1838 => self.watch(msg),
            // Card trades (Player Information's Send Trade Request, then the trade scene).
            0x2BC0 | 0x2BC4 | 0x2BC2 | 0x2BD5 | 0x2BC6 | 0x2BC7 | 0x2BC8 | 0x2BC9 | 0x2BCB => {
                let p = self.player();
                let me = (p.player_id, p.name.clone());
                let world = self.world.clone();
                self.hub().trade_request((me.0, &me.1), id, msg, &world)
            }
            // Card rentals (Player Information's Send Rental Request, then the rental scene).
            0x2C24 | 0x2C28 | 0x2C26 | 0x2C3E | 0x2C2A | 0x2C2B | 0x2C2C | 0x2C2D | 0x2C31 | 0x2C2F | 0x2C33 => {
                let p = self.player();
                let me = (p.player_id, p.name.clone());
                let world = self.world.clone();
                self.hub().rental_request((me.0, &me.1), id, msg, &world, chrono::Utc::now().timestamp())
            }
            // Teams (the Team window 0x4BBEC0 asks 0x2AFB then 0x2AFD; so do the main screen's startup steps).
            0x2AFB => self.hub().team_info(self.player().player_id),
            0x2AFD => self.hub().team_members(self.player().player_id),
            0x2AF9 | 0x2AFE | 0x2B02 | 0x2B00 | 0x2AFA | 0x2B05 | 0x2B04 | 0x2B06 => {
                let p = self.player();
                let me = (p.player_id, p.name.clone());
                self.hub().team_request((me.0, &me.1), id, msg)
            }
            // Next present: 0x2777 would carry one; 0x2778 = none, which closes PresentWindow
            // (0x4783C9) and lets the main screen unlock its facilities and menu (0x473B87).
            0x2775 => match self.presents.pop_front() {
                Some(present) => {
                    self.last_present += 1;
                    present.message(self.last_present)
                }
                None => Message::reply(0x2778),
            },
            0x2135 => self.room_list(),
            0x2136 => self.enter_room(msg),
            0x2137 => {
                if let Some(id) = self.online {
                    self.hub().set_room(id, None);
                }
                Message::reply(0x245C)
            }
            0x2134 => return self.chat(msg),
            _ => return vec![],
        };
        vec![reply]
    }

    /// 0x17D4 {event_id, join_id, duel_id}, sent by DuelManager (0x50F030) once its duel object is
    /// ready (state 2) or failed (-1) -> 0x17D5, which lets it go on, then the 0x22C4 push its next
    /// state (0x50F690) waits for: item "duelData" {host, port} sends it to the duel server,
    /// and its sub-item "rule" (the rule asked for in 0x17AC) becomes the duel's rule.
    fn duel_ready(&self, msg: &Message) -> Vec<Message> {
        let ack = Message::reply(0x17D5);
        let duel_id = msg.get("duel_id").and_then(Value::as_u32);
        let matched = duel_id.and_then(|id| self.hub().matches.get(&id).map(|m| m.rule.clone()));
        let Some(asked) = duel_id.and_then(|id| self.duels.get(&id).cloned()).or(matched) else { return vec![ack] };
        let rule = asked.iter().fold(Message::record().with("name", Value::text("rule")), |m, (k, v)| m.with(k, v.clone()));
        let (host, port) = self.duel_server();
        let data = Message::record()
            .with("name", Value::text("duelData"))
            .with("host", Value::text(&host))
            .with("port", Value::U32(port))
            .item(rule);
        vec![ack, Message::reply(0x22C4).item(data)]
    }

    /// The duel server's address for this client: the advertised one, else the address it reached.
    fn duel_server(&self) -> (String, u32) {
        let (advertised, port) = &self.world.duel_server;
        let reached = self.reached_at.map(|ip| ip.to_string()).unwrap_or_else(|| "127.0.0.1".into());
        (if advertised.is_empty() { reached } else { advertised.clone() }, *port as u32)
    }

    /// Watching a duel (agent mode 4, 0x4A3340): 0x1838 {duel_id} -> 0x1839 {host, port, card_protector_1,
    /// card_protector_2} with the items avatar_1 and avatar_2 (the players' records by seat, loaded as the duel's two
    /// players) and duelData {event_type, official, duel_id, match_type} (0x4A3650, the duel's rule); 0x183A when the
    /// duel isn't being played by two seated players, or its host didn't allow watching.
    fn watch(&self, msg: &Message) -> Message {
        let refused = Message::reply(0x183A);
        let Some(duel_id) = msg.get("duel_id").and_then(Value::as_u32) else { return refused };
        let (table, allowed, match_type) = {
            let hub = self.hub();
            let m = hub.matches.get(&duel_id);
            let rule = m.and_then(|m| m.rule.iter().find(|(k, _)| k == "match_type").and_then(|(_, v)| v.as_u32()));
            (hub.tables.get(&duel_id).cloned(), m.is_none_or(|m| m.watchable), rule.unwrap_or(SINGLE_MODE))
        };
        let (Some(table), true) = (table, allowed) else { return refused };
        // The hub is free while the table is locked (the table locks the hub when a duel ends).
        let Some(players) = table.lock().expect("table lock").players() else { return refused };
        let hub = self.hub();
        let record = |id: u32| hub.online.values().find(|o| o.player_id == id).map(|o| o.record.clone());
        let (Some(first), Some(second)) = (record(players[0]), record(players[1])) else { return refused };
        drop(hub);
        let (host, port) = self.duel_server();
        let data = Message::record()
            .with("name", Value::text("duelData"))
            .with("event_type", Value::U32(0))
            .with("official", Value::U32(0))
            .with("duel_id", Value::U32(duel_id))
            .with("match_type", Value::U32(match_type));
        Message::reply(0x1839)
            .with("host", Value::text(&host))
            .with("port", Value::U32(port))
            .with("card_protector_1", Value::U32(0))
            .with("card_protector_2", Value::U32(0))
            .item(first.with("name", Value::text("avatar_1")))
            .item(second.with("name", Value::text("avatar_2")))
            .item(data)
    }

    /// 0x2135 -> 0x2458: top-level lang, one item per room (ChatWindow 0x435950, items via 0x434D20).
    fn room_list(&self) -> Message {
        let hub = self.hub();
        self.world.rooms.iter().fold(Message::reply(0x2458).with("lang", Value::U32(1)), |m, r| {
            m.item(
                Message::record()
                    .with("id", Value::U32(r.id))
                    .with("name", Value::text(&r.name))
                    .with("capacity", Value::U32(r.capacity))
                    .with("num", Value::U32(hub.occupancy(r.id)))
                    .with("region", Value::U32(r.region))
                    .with("status", Value::U32(r.status))
                    .with("type", Value::U32(r.kind)),
            )
        })
    }

    /// 0x2136 {room_id} -> 0x245A {room_id}, or 0x245B {error_code} if unknown or full.
    fn enter_room(&self, msg: &Message) -> Message {
        let wanted = msg.get("room_id").and_then(Value::as_u32);
        let room = self.world.rooms.iter().find(|r| Some(r.id) == wanted);
        let (Some(room), Some(id)) = (room, self.online) else {
            return Message::reply(0x245B).with("error_code", Value::U32(1));
        };
        let mut hub = self.hub();
        if hub.room_of(id) != Some(room.id) && hub.occupancy(room.id) >= room.capacity {
            return Message::reply(0x245B).with("error_code", Value::U32(2));
        }
        hub.set_room(id, Some(room.id));
        Message::reply(0x245A).with("room_id", Value::U32(room.id))
    }

    /// 0x2CF0 -> 0x2CF7: the online players matching the search, one page of them. InfoWindow
    /// 0x458410 reads result_number (the total; it shows pages of 100) and each item through
    /// 0x475A20 in player mode. It skips its own player_id, so the searcher is listed too.
    /// search_flag bits come from the builder 0x45F390; 0x4B6AF0 looks one player up by id.
    fn player_search(&self, msg: &Message) -> Message {
        const BY_NAME: u32 = 0x2;
        const BY_LANGUAGE: u32 = 0x4;
        const BY_COUNTRY: u32 = 0x8;
        const BY_LEVEL: u32 = 0x10;
        const BY_MANNER: u32 = 0x100;
        // The status bits (Detailed Search 0x45F84A): waiting for a Single Mode duel, a Match Mode one, any, and a duel
        // in progress (a guess from the UI texts); and the event bits (official, championship, user event).
        const WAITING_SINGLE: u32 = 0x200;
        const WAITING_MATCH: u32 = 0x400;
        const WAITING: u32 = 0x800;
        const DUELLING_NOW: u32 = 0x1000;
        const IN_EVENT: u32 = 0xE000;
        const IN_MY_ROOM: u32 = 0x20000;
        const BY_ID: u32 = 0x40000;
        // The community bits (the preset tabs): friends ("Display online friends."), the ignore list, the team.
        const FRIENDS: u32 = 0x40;
        const IGNORED: u32 = 0x80;
        const TEAM: u32 = 0x20;
        let uint = |k| msg.get(k).and_then(Value::as_u32);
        let flags = uint("search_flag").unwrap_or(0);
        // Guess: case-insensitive part of the name. Nobody is in an event.
        let name = msg.get("pname").and_then(Value::as_str).unwrap_or_default().to_lowercase();
        let levels = uint("min_level").unwrap_or(0)..=uint("max_level").unwrap_or(u32::MAX);
        let me = self.player().player_id;
        let hub = self.hub();
        let mut found: Vec<&Online> = hub
            .online
            .values()
            .filter(|o| flags & BY_NAME == 0 || o.name.to_lowercase().contains(&name))
            .filter(|o| flags & FRIENDS == 0 || o.player_id == me || hub.social.are_friends(me, o.player_id))
            .filter(|o| flags & IGNORED == 0 || o.player_id == me || hub.social.ignores(me, o.player_id))
            .filter(|o| flags & TEAM == 0 || hub.teams.of(me).is_some_and(|t| t.member(o.player_id).is_some()))
            .filter(|o| flags & BY_LEVEL == 0 || levels.contains(&o.level))
            .filter(|o| flags & BY_COUNTRY == 0 || o.record.get("country").and_then(Value::as_str) == msg.get("country").and_then(Value::as_str))
            .filter(|o| flags & BY_LANGUAGE == 0 || uint("lang").unwrap_or(0) >> o.lang & 1 != 0)
            .filter(|o| flags & BY_MANNER == 0 || uint("eval_level").unwrap_or(0) >> hub.manner(o.player_id) & 1 != 0)
            .filter(|o| {
                let status = flags & (WAITING_SINGLE | WAITING_MATCH | WAITING | DUELLING_NOW);
                let hosted = hub.hosts.values().find(|h| h.player_id == o.player_id).map(|h| h.setting("match_type").and_then(Value::as_u32).unwrap_or(SINGLE_MODE));
                status == 0
                    || (status & DUELLING_NOW != 0 && hub.in_match(o.player_id).is_some())
                    || (status & WAITING != 0 && hosted.is_some())
                    || (status & WAITING_SINGLE != 0 && hosted.is_some_and(|t| t != MATCH_MODE))
                    || (status & WAITING_MATCH != 0 && hosted.is_some_and(|t| t != SINGLE_MODE))
            })
            .filter(|_| flags & IN_EVENT == 0)
            .filter(|o| flags & IN_MY_ROOM == 0 || (o.room.is_some() && o.room == self.online.and_then(|id| hub.room_of(id))))
            .filter(|o| flags & BY_ID == 0 || Some(o.player_id) == uint("player_id"))
            .collect();
        found.sort_by_key(|o| o.player_id);
        // Guess: a page starts at base_point and holds at most front_num players.
        let skip = uint("base_point").unwrap_or(0) as usize;
        let page = found.iter().skip(skip).take(uint("front_num").unwrap_or(100) as usize);
        page.fold(Message::reply(0x2CF7).with("result_number", Value::U32(found.len() as u32)), |m, o| {
            let [wins, losses, draws] = hub.totals(o.player_id);
            let team = hub.teams.of(o.player_id);
            let entry = Message::record()
                .with("player_id", Value::U32(o.player_id))
                .with("pname", Value::text(&o.name))
                .with("dlv", Value::U32(o.level))
                .with("n_win", Value::U32(wins))
                .with("n_lose", Value::U32(losses))
                .with("n_draw", Value::U32(draws))
                .with("team_id", Value::U32(team.map_or(0, |t| t.id)))
                .with("team_name", Value::text(team.map_or("", |t| t.name.as_str())))
                .with("team_authority", Value::U32(team.and_then(|t| t.authority(o.player_id)).unwrap_or(0)))
                .with("evallevel", Value::U32(hub.manner(o.player_id)));
            let entry = match o.record.get("country") {
                Some(country) => entry.with("country", country.clone()),
                None => entry,
            };
            m.item(hub.duel_fields(o.player_id, entry))
        })
    }

    /// 0x17B6 {event_id, join_id, duel_id, password}: a challenger's offer for a hosted duel (agent mode 1,
    /// 0x4A28F0). 0x17B7 tells it the offer went ("Requesting Duel"); the host gets the offer as 0x17B6 with item
    /// "playerInfo" (the challenger's record, 0x4A1D10). 0x17B8 {error_code} when it can't go through.
    fn challenge(&mut self, msg: &Message) -> Vec<Message> {
        let me = self.player().clone();
        let duel_id = msg.get("duel_id").and_then(Value::as_u32).unwrap_or(0);
        let password = msg.get("password").and_then(Value::as_str).unwrap_or_default();
        let refuse = |code: u32| vec![Message::reply(0x17B8).with("error_code", Value::U32(code))];
        let mut hub = self.hub();
        let Some(host) = hub.hosts.get(&duel_id).filter(|h| h.player_id != me.player_id) else { return refuse(NO_DUEL) };
        let teammates = hub.teams.of(host.player_id).is_some_and(|t| t.member(me.player_id).is_some());
        match host.setting("party").and_then(Value::as_u32) {
            Some(FRIENDS_ONLY) if !hub.social.are_friends(host.player_id, me.player_id) => return refuse(REFUSED),
            Some(TEAM_ONLY) if !teammates => return refuse(REFUSED),
            _ => {}
        }
        let Some(host) = hub.hosts.get_mut(&duel_id) else { return refuse(NO_DUEL) };
        if host.password() != password {
            return refuse(WRONG_PASSWORD);
        }
        if host.offer.is_some_and(|p| p != me.player_id) {
            return refuse(BUSY);
        }
        host.offer = Some(me.player_id);
        let host_id = host.player_id;
        let offer = Message::reply(0x17B6).with("duel_id", Value::U32(duel_id)).item(me.record().with("name", Value::text("playerInfo")));
        hub.push(host_id, &offer);
        vec![Message::reply(0x17B7)]
    }

    /// 0x17CA {event_id, join_id, duel_id, answer}: the host's answer (0x4A1D10) -> 0x17CB. The challenger gets it as
    /// 0x17CA {answer} (0x4A28F0); on ACCEPT both then get the match 0x1978, a second later each (**guess**, as for an
    /// automatic match: the agents read one message per step, and the challenger's must take the answer first).
    fn answer_challenge(&mut self, msg: &Message) -> Vec<Message> {
        let player_id = self.player().player_id;
        let duel_id = msg.get("duel_id").and_then(Value::as_u32).unwrap_or(0);
        let accepted = msg.get("answer").and_then(Value::as_u32) == Some(ACCEPT);
        let mut hub = self.hub();
        let Some(host) = hub.hosts.get_mut(&duel_id).filter(|h| h.player_id == player_id) else { return vec![Message::reply(0x17CB)] };
        let Some(challenger) = host.offer.take() else { return vec![Message::reply(0x17CB)] };
        let answer = Message::reply(0x17CA).with("duel_id", Value::U32(duel_id)).with("answer", Value::U32(if accepted { ACCEPT } else { 1 }));
        hub.push(challenger, &answer);
        if accepted {
            let host = hub.hosts.remove(&duel_id).expect("the host");
            let challenger_record = hub.online.values().find(|o| o.player_id == challenger).map(|o| o.record.clone());
            let match_type = host.setting("match_type").and_then(Value::as_u32).unwrap_or(SINGLE_MODE);
            let mut rule: Vec<(String, Value)> = host.settings.iter().filter(|(k, _)| k == "lp_start").cloned().collect();
            rule.push(("match_type".to_string(), Value::U32(match_type)));
            let push = |vs: &Message| {
                let data = rule.iter().fold(Message::record().with("name", Value::text("duelData")), |m, (k, v)| m.with(k, v.clone()));
                Message::reply(0x1978).item(vs.clone().with("name", Value::text("vsPlayerData"))).item(data.with("duel_id", Value::U32(duel_id)))
            };
            let mut pushes = vec![(challenger, push(&host.record))];
            if let Some(record) = challenger_record {
                pushes.push((player_id, push(&record)));
            }
            let setting = |k| host.setting(k).and_then(Value::as_u32).unwrap_or(0);
            let times = Some((setting("time_turn"), setting("time_total")));
            let watchable = host.setting("watch").and_then(Value::as_u32).is_none_or(|w| w == 1);
            hub.matches.insert(duel_id, Match { players: [player_id, challenger], rule, times, watchable });
            let outboxes: Vec<(Outbox, Message)> = pushes
                .into_iter()
                .filter_map(|(to, m)| hub.online.values().find(|o| o.player_id == to).map(|o| (o.outbox.clone(), m)))
                .collect();
            std::thread::spawn(move || {
                std::thread::sleep(MATCH_PUSH_DELAY);
                for (outbox, m) in outboxes {
                    let _ = outbox.send(m);
                }
            });
        }
        vec![Message::reply(0x17CB)]
    }

    /// 0x2134 -> 0x2456 to the sender, and the line as a 0x2455 push to the room (or the whisper
    /// target). The client sends its fields in one item (tag 10): chat_str, chat_type, and to_id
    /// (room chat: chat_type 1, to_id = the room) or to_name (whisper). The client shows nothing
    /// on 0x2456, so the sender gets the 0x2455 echo too. 0x2455 is read by ChatWindow 0x435950.
    fn chat(&self, msg: &Message) -> Vec<Message> {
        const ROOM_CHAT: u32 = 1;
        const TEAM_CHAT: u32 = 3;
        let Some(id) = self.online else { return vec![Message::reply(0x2457)] };
        let body = msg.items.first().unwrap_or(msg);
        let chat_type = body.get("chat_type").and_then(Value::as_u32).unwrap_or(0);
        let player = self.player();
        let line = Message::record()
            .with("from_id", Value::U32(player.player_id))
            .with("pname", Value::text(&player.name))
            .with("chat_type", Value::U32(chat_type))
            .with("chat_str", Value::text(&body.get("chat_str").and_then(Value::as_str).unwrap_or_default()));
        let push = Message::reply(0x2455).item(line);
        let to_id = body.get("to_id").and_then(Value::as_u32);
        let to_name = body.get("to_name").and_then(Value::as_str);
        let hub = self.hub();
        let delivered = match (chat_type, to_id, to_name) {
            (_, _, Some(name)) => hub.send_player(None, Some(&name), &push),
            (TEAM_CHAT, Some(team), None) => match hub.teams.of(player.player_id).filter(|t| t.id == team) {
                Some(t) => {
                    hub.tell_team(&t.ids(), Some(player.player_id), &push);
                    true
                }
                None => false,
            },
            (ROOM_CHAT, Some(room), None) => {
                let here = hub.room_of(id) == Some(room);
                if here {
                    hub.send_room(room, &push, id);
                }
                here
            }
            (_, Some(player_id), None) => hub.send_player(Some(player_id), None, &push),
            _ => false,
        };
        if !delivered {
            return vec![Message::reply(0x2457)];
        }
        vec![Message::reply(0x2456), push]
    }

    fn find_player(&self, msg: &Message) -> Option<usize> {
        let id = msg.get("player_id").and_then(Value::as_u32)?;
        self.account.players.iter().position(|p| p.player_id == id)
    }

    fn player(&self) -> &Player {
        &self.account.players[self.selected.unwrap_or(0)]
    }
}
