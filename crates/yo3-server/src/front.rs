//! Front server: login -> player select -> world entry -> main-screen polls -> chat.
//!
//! The request/reply catalog, with the client evidence and status of every pair, is
//! `docs/protocol.md`. Keep it, this file and the contract tests in step.
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use yo3_proto::{Message, Packet, Value};

/// Where a session's outgoing messages go: its connection's writer task.
pub type Outbox = tokio::sync::mpsc::UnboundedSender<Message>;

pub const VERSION_VALID: u32 = 0x219F;
/// The default banlist every account logs in with (0x2201 regulation_id), which Auto Duel's and the NPC duel's deck
/// check uses (`World::login_regulation`, set by --regulation); also the one the starter deck must pass.
pub const LOGIN_REGULATION: u32 = 1;
pub const HEARTBEAT: u32 = 0x2724;

/// Server clock in Japan time: the client treats server timestamps as JST (UTC+9) and
/// converts them to the player's local time.
fn jst_now() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(9 * 3600).expect("valid offset"))
}

/// Main-screen polls whose replies carry nothing the client reads.
const BARE_REPLIES: [(u32, u32); 3] = [(0x271A, 0x271B), (0x2AFB, 0x2B13), (0x2AFD, 0x2B17)];

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
}

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
        }
    }

    /// The record read by the player list (0x46C1E4) and the PlayerData loader (0x51C260).
    pub fn record(&self) -> Message {
        let mut m = Message::record()
            .with("pname", Value::text(&self.name))
            .with("pindex", Value::U32(self.index))
            .with("player_id", Value::U32(self.player_id))
            .with("dlv", Value::U32(self.level))
            .with("title", Value::U32(self.title))
            .with("n_win", Value::U32(0))
            .with("n_lose", Value::U32(0))
            .with("n_draw", Value::U32(0))
            .with("team_level", Value::U32(0))
            .with("team_name", Value::text(""))
            .with("team_authority", Value::U32(0))
            .with("avatar1", Value::Bin(self.avatar.clone()))
            .with("pbin", Value::Bin(self.pbin.clone()));
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
}

impl Account {
    pub fn test() -> Account {
        Account { user_id: 1001, name: "test".into(), dpoint: 0, players: vec![Player::new(1001, "test")] }
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
/// The challenge errors (0x17B8 error_code, 0x4A28F0): the host answers someone else, the duel is gone, the password.
const BUSY: u32 = 3000;
const NO_DUEL: u32 = 3001;
const WRONG_PASSWORD: u32 = 4006;
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
    let name = |n: &str| Message::record().with("name", Value::text(n));
    let counts = ["win", "lose", "draw"].iter().zip(record).enumerate().fold(name("resultData"), |m, (i, (k, n))| {
        m.with(k, Value::U32(n)).with(&format!("{k}_add"), Value::U32((i as u32 + 1 == result) as u32))
    });
    let me = Message::record().with("player_id", Value::U32(player_id)).item(counts);
    let data = Message::duel_data()
        .with("name", Value::text("duelData"))
        .item(name("result").with("duel_result", Value::U32(result)).with("duel_end", Value::U32(1)))
        .item(name("players").item(me));
    Message::reply(0x2009).item(data)
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
    /// Each player's LP at the start of a duel (8000; lower only to test).
    pub start_lp: u32,
    /// Testing aid: every player's main deck in a duel, as CARD_Prop rows repeated to 40 cards.
    pub test_deck: Option<Vec<u16>>,
    /// The rules engine's directory (engine/, once fetched); without it the server deals the opening itself.
    pub engine_dir: Option<std::path::PathBuf>,
    /// Where clients connect for their duels (0x22C4 duelData host and port).
    pub duel_server: (String, u16),
    pub news: Vec<(String, String)>,
    pub rooms: Vec<Room>,
    /// The booster packs and structure decks on the shop's shelves (from the client's pack and recipe files).
    pub packs: Vec<crate::shop::ShopPack>,
    pub structure_decks: Vec<crate::shop::ShopDeck>,
}

impl World {
    pub fn new(regulation_ids: Vec<u16>) -> World {
        World {
            regulation_ids,
            login_regulation: LOGIN_REGULATION,
            card_rows: 0,
            starter_deck: None,
            npcs: vec![],
            npc_decks: HashMap::new(),
            cards: HashMap::new(),
            engine_dir: None,
            start_lp: 8000,
            test_deck: None,
            duel_server: ("127.0.0.1".into(), 0),
            news: vec![(
                "Welcome back to Yu-Gi-Oh! ONLINE".into(),
                "This is a private revival server. Have fun duelling with your friends!".into(),
            )],
            rooms: vec![
                Room::new(1, "Lobby"),
                Room::new(2, "Free Talk"),
                Room::new(3, "Trading"),
                Room::new(4, "Duel Recruiting"),
            ],
            packs: vec![],
            structure_decks: vec![],
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
}

struct Online {
    player_id: u32,
    name: String,
    level: u32,
    room: Option<u32>,
    outbox: Outbox,
    /// The player's record as others get it (an opponent's vsPlayerData).
    record: Message,
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

    fn join(&mut self, player: &Player, outbox: Outbox) -> u64 {
        self.next_id += 1;
        let online = Online { player_id: player.player_id, name: player.name.clone(), level: player.level, room: None, outbox, record: player.record() };
        self.online.insert(self.next_id, online);
        self.next_id
    }

    /// A matched duel is over: its match and table go.
    pub fn end_match(&mut self, duel_id: u32) {
        self.matches.remove(&duel_id);
        self.tables.remove(&duel_id);
    }

    fn leave(&mut self, id: u64) {
        if let Some(o) = self.online.remove(&id) {
            self.waiting.retain(|w| w.player_id != o.player_id);
            self.hosts.retain(|_, h| h.player_id != o.player_id);
            let withdrawn: Vec<(u32, u32)> = self.hosts.iter().filter(|(_, h)| h.offer == Some(o.player_id)).map(|(&d, h)| (d, h.player_id)).collect();
            for (duel_id, host) in withdrawn {
                self.withdraw(duel_id, host);
            }
        }
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
        FrontSession {
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
        let Some(goods) = crate::shop::Goods::find(&world.packs, &world.structure_decks, sale_id) else { return Message::reply(0x1C25) };
        let (price, max) = goods.price();
        let cost = price * count;
        let Some(at) = self.selected else { return Message::reply(0x1C25) };
        if count == 0 || count > max || cost > self.account.dpoint {
            return Message::reply(0x1C25);
        }
        let bought: Vec<crate::shop::Present> = (0..count).map(|_| goods.take(&mut self.rng)).collect();
        let add = |p: &mut Player| {
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
            0x1F48 => Message::reply(0x2201).with("regulation_id", Value::U32(self.world.login_regulation)).item(
                Message::record()
                    .with("user_id", Value::U32(a.user_id))
                    .with("uname", Value::text(&a.name))
                    .with("dpoint", Value::U32(a.dpoint)),
            ),
            0x1F49 => a.players.iter().fold(Message::reply(0x2260), |m, p| m.item(p.record())),
            0x1F4C => {
                self.selected = self.find_player(msg);
                if self.selected.is_none() {
                    return vec![];
                }
                let mut hub = self.hub.lock().expect("hub lock");
                if let Some(old) = self.online {
                    hub.leave(old);
                }
                let id = hub.join(self.player(), self.outbox.clone());
                drop(hub);
                self.online = Some(id);
                Message::reply(0x2266)
            }
            0x1FA4 => {
                if self.find_player(msg).or(self.selected).is_none() {
                    return vec![];
                }
                // Copied wholesale into the trunk dataset (and kept in slot 18). cards / noTrade: owned
                // and untradeable count per card at row - 1 (0x410DB0); the NPC duel writes into both
                // unchecked (0x4F6A82). Players own their starter deck's cards and the cards they bought.
                let mut cards = vec![0u8; self.world.card_rows];
                let bought = self.selected.map(|at| self.account.players[at].cards.clone()).unwrap_or_default();
                let starter = self.world.starter_deck.iter().flat_map(Deck::cards).map(|row| (row, 1));
                for (row, count) in starter.chain(bought) {
                    if let Some(n) = (row as usize).checked_sub(1).and_then(|i| cards.get_mut(i)) {
                        *n = n.saturating_add(count.min(255) as u8);
                    }
                }
                let no_trade = Value::Bin(vec![0; self.world.card_rows]);
                let trunk = Message::reply(9000)
                    .item(Message::record().with("cards", Value::Bin(cards)).with("noTrade", no_trade));
                // The client never asks for its friend and black lists; the global dispatcher stores
                // 0x251C in dataset slot 6 and 0x252A in slot 7, and listing any player reads both
                // without a NULL check (0x45E6D0, 0x45E810). Items would be {friend_id, friend_name,
                // login_flag} and {black_id, black_name, login_flag} (0x475A20 modes 2 and 3).
                // Likewise the rental records 0x2D6C (slot 8), which opening the shop reads (0x48F080).
                return vec![trunk, Message::reply(0x251C), Message::reply(0x252A), Message::reply(0x2D6C)];
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
            0x1B6B => Message::reply(0x1C32),
            0x1B6D => Message::reply(0x1C2E),
            // perk_num is compared with 500 to show _root.expack_del; meaning unknown.
            0x1B72 => Message::reply(0x1C36).with("perk_num", Value::U32(0)),
            0x2C40 => ["rental_cardnum", "rental_point", "rental_term"]
                .into_iter()
                .fold(Message::reply(0x2D75), |m, k| m.with(k, Value::U32(0))),
            0x1B69 => Message::reply(0x1C2A),
            // Event list {event_type, status, event_id} (TournamentScene 0x4D0BB0) and event search
            // (UserEventScene 0x4D9E37): no events are held.
            0x177A | 0x1789 => Message::reply(0x177B).with("result_number", Value::U32(0)),
            // The player's own user event (UserEventScene 0x4D9BB0): none.
            0x339A => Message::reply(0x33FE).with("remain", Value::U32(0)).with("event_id", Value::U32(0)),
            // Deck list (deck editor 0x43FC60): it saves each deck it lacks as a local file.
            // The starter deck, then the structure decks the player bought; the client uses at most 10.
            0x1F4E => {
                let owned = self.selected.map(|at| self.account.players[at].decks.clone()).unwrap_or_default();
                let bought = owned.iter().filter_map(|&id| self.world.structure_decks.iter().find(|s| s.deck.id == id)).map(|s| &s.deck);
                self.world.starter_deck.iter().chain(bought).take(10).fold(Message::reply(0x22C8), |m, d| m.item(d.record()))
            }
            0x17D4 => return self.duel_ready(msg),
            // Deck updated: the deck editor's save, and the deck a player picks for a duel (DuelManager
            // 0x50ECF0, {duel_id, event_id, update_type 1, item: the deck}), which is kept for the duel.
            0x1F4F => {
                let duel_id = msg.get("duel_id").and_then(Value::as_u32);
                let player_id = self.player().player_id;
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
            0x1F50 => Message::reply(0x22CC),
            // NPC roster (kept in slot 4; SelectNpcScene 0x4BEB40 reads it three items per NPC). The
            // picked item becomes the opponent's PlayerData. A level opens when the player's
            // experience with the NPC (npcbin, not sent yet) reaches its need_ex; the three must not
            // all be 0 (0x4BFDC4 divides by their sum). need_ex = the level (0, 1, 2) and every NPC
            // on the first tab (npc_type 0) are guesses.
            0x2710 => self.world.npcs.iter().fold(Message::reply(0x2711), |m, &id| {
                m.item(
                    Message::record()
                        .with("npc_type", Value::U32(0))
                        .with("npc_id", Value::U32(id))
                        .with("need_ex", Value::U32(id % 3)),
                )
            }),
            // Duel entry (DuelMatchingAgent step 0x4A1090): join_id comes back in 0x17AC.
            0x1784 => {
                let mut hub = self.hub();
                hub.last_join += 1;
                Message::reply(0x1785).with("join_id", Value::U32(hub.last_join))
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
            0x2CEE => Message::reply(0x2CF3).item(self.player().record()),
            0x2CEF => self.update_player(msg),
            0x2583 => Message::reply(0x2588).with("count", Value::U32(0)),
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
            0x2CF0 => self.player_search(msg),
            0x1838 => self.watch(msg),
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
        const BY_LEVEL: u32 = 0x10;
        const BY_ID: u32 = 0x40000;
        let uint = |k| msg.get(k).and_then(Value::as_u32);
        let flags = uint("search_flag").unwrap_or(0);
        // Guess: case-insensitive part of the name. Other criteria (lang, country, eval_level,
        // the preset tabs) are not stored yet, so they match everyone.
        let name = msg.get("pname").and_then(Value::as_str).unwrap_or_default().to_lowercase();
        let levels = uint("min_level").unwrap_or(0)..=uint("max_level").unwrap_or(u32::MAX);
        let hub = self.hub();
        let mut found: Vec<&Online> = hub
            .online
            .values()
            .filter(|o| flags & BY_NAME == 0 || o.name.to_lowercase().contains(&name))
            .filter(|o| flags & BY_LEVEL == 0 || levels.contains(&o.level))
            .filter(|o| flags & BY_ID == 0 || Some(o.player_id) == uint("player_id"))
            .collect();
        found.sort_by_key(|o| o.player_id);
        // Guess: a page starts at base_point and holds at most front_num players.
        let skip = uint("base_point").unwrap_or(0) as usize;
        let page = found.iter().skip(skip).take(uint("front_num").unwrap_or(100) as usize);
        page.fold(Message::reply(0x2CF7).with("result_number", Value::U32(found.len() as u32)), |m, o| {
            let entry = Message::record()
                .with("player_id", Value::U32(o.player_id))
                .with("pname", Value::text(&o.name))
                .with("dlv", Value::U32(o.level));
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
        let Some(host) = hub.hosts.get_mut(&duel_id).filter(|h| h.player_id != me.player_id) else { return refuse(NO_DUEL) };
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
