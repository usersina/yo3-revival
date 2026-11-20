//! Duel server: the connection DuelManager (0x50F690) opens at the address in the 0x22C4 push.
//!
//! Both directions are key/value messages carrying a duelRequestName; see "Duel connection" in
//! `docs/protocol.md`. The server drives the duel with engine commands, which the client's scenes
//! pop one at a time. Against an NPC it plays the janken as the CPU and runs the rules engine (see
//! `translate.rs`); a duel between two players runs at a shared table (`table.rs`).
use crate::lock::Locked;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use yo3_proto::{Message, Packet, Value};

use crate::banter::{self, Banter, Said};
use crate::engine::{Answer, Engine};
use crate::front::{Deck, DuelEntry, Ended, Hub, Outbox, SINGLE_MODE, World};
use crate::table::Table;
use crate::translate::{Prompt, Repeats, Translator};
use crate::wire::{Cmd, Place, cmd, request, result, table_place, zone};

/// Request id of the server's duel messages. A guess: the pump (0x5233A0) reads only
/// duelRequestName, and treats a negative id as a lost connection.
pub const SERVER_DUEL: u32 = 0x200D;
/// The client's first message on the duel connection, {player_id, autoSet} (0x50FAD1).
const ENTER: u32 = 0x200A;
/// A spectator's first message on the duel connection (DuelManager's watching state 0x50F320), instead of ENTER.
const WATCH: u32 = 0x1838;
/// Every later client message (0x49F990), told apart by duelRequestName.
const CLIENT_DUEL: u32 = 0x200C;


/// Turn order as the janken commands give it: 1 first, 2 second.
pub(crate) const FIRST: u32 = 1;
pub(crate) const SECOND: u32 = 2;

const DRAW_PHASE: u32 = 0;
const OPENING_HAND: usize = 5;
/// Engine players: the client is player 0 (setup PlayerNum), the CPU player 1.
pub(crate) const CLIENT: u32 = 0;
pub(crate) const CPU: u32 = 1;

/// The server's clock check for every duel between players (once a second, and in tests at any `now`). A table that
/// is busy (its lock is held, say by an engine call) is skipped until the next second; one that panicked, under its
/// lock or in this very check, ends its duel as a draw and goes, so one bad table never stops the others' clocks.
pub fn tick_tables(hub: &Arc<Mutex<Hub>>, now: std::time::Instant) {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::TryLockError;
    let tables: Vec<_> = hub.locked().tables.iter().map(|(id, t)| (*id, t.clone())).collect();
    let mut broken = vec![];
    for (id, table) in tables {
        match table.try_lock() {
            Ok(mut t) => {
                if catch_unwind(AssertUnwindSafe(|| t.tick(now))).is_err() {
                    broken.push((id, table.clone()));
                }
            }
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Poisoned(_)) => broken.push((id, table.clone())),
        }
    }
    for (id, table) in broken {
        tracing::error!("duel {id}: its table panicked; the duel ends as a draw");
        let mut t = table.locked();
        let _ = catch_unwind(AssertUnwindSafe(|| t.abort()));
        drop(t);
        hub.locked().end_match(id, None);
    }
}

/// The duel prompt ids that name the server's texts: `TEXT_IDS + slot`, slots used in turn. The client's table
/// (data manager +8, vector at +0x84) is a few hundred texts long; the DLL answers only ids past its end, and the
/// Select dialog keeps 14 bits of a label's id (0x4C592E), so they sit at the top of that range.
pub const TEXT_IDS: u32 = 0x3F00;
pub const TEXT_SLOTS: u32 = 0x100;
/// The most UTF-16 units of one text: a message is built in a 1024-unit buffer (0x519870).
const TEXT_MAX: usize = 400;

/// A text for the duel prompt id `TEXT_IDS + slot`, sent before the dialog that names it. The client's dialogs show
/// HTML (Flash htmlText), so markup characters are escaped; `@n…@0` colours as in the client's own texts.
pub fn text_message(slot: u32, text: &str) -> Message {
    let (mut escaped, mut units) = (String::new(), 0);
    for c in text.chars().filter(|&c| c != '\0') {
        let mut one = [0; 4];
        let piece = match c {
            '<' => "&lt;",
            '>' => "&gt;",
            '&' => "&amp;",
            c => &*c.encode_utf8(&mut one),
        };
        units += piece.encode_utf16().count();
        if units > TEXT_MAX {
            break;
        }
        escaped.push_str(piece);
    }
    Message::reply(SERVER_DUEL)
        .with("duelRequestName", Value::U32(request::REVIVAL_TEXT))
        .with("slot", Value::U32(slot))
        .with("text", Value::text(&escaped))
}

/// An engine command, as pump case 0x11 reads it.
pub fn command(id: u32, params: [u32; 4]) -> Message {
    let m = Message::reply(SERVER_DUEL).with("duelRequestName", Value::U32(request::ENGINE_COMMAND)).with("id", Value::U32(id));
    params.iter().zip(["param1", "param2", "param3", "param4"]).fold(m, |m, (v, k)| m.with(k, Value::U32(*v)))
}

/// The commands each card offers, as {place, command bits} (pump case 0x12), then the other flags as {key,
/// value} bytes (the phase panel reads key = button 0 DP .. 5 EP, value != 0 = enabled; 0x501C00). Places are
/// `player | zone << 8 | index << 16`; bit n is the menu entry n (see [`CardCommand`]).
pub fn command_data(cards: &[(u32, u32)], flags: &[(u8, u8)]) -> Message {
    let mut data: Vec<u8> = cards.iter().flat_map(|(place, bits)| place.to_be_bytes().into_iter().chain(bits.to_be_bytes())).collect();
    data.extend(flags.iter().flat_map(|(k, v)| [*k, *v]));
    Message::reply(SERVER_DUEL)
        .with("duelRequestName", Value::U32(request::COMMAND_DATA))
        .with("comIDsize", Value::U32(cards.len() as u32))
        .with("helpIDsize", Value::U32(0))
        .with("otherFlagSize", Value::U32(flags.len() as u32))
        .with("COMIDDATA", Value::Bin(data))
}

/// A command the player chose from a card's menu (client duelRequestName 2). `command` is the bit
/// number: 0 Attack, 1 See Details, 2 Special Summon, 3 Activate Effect, 4 Summon, 5 Flip Summon,
/// 6 and 7 Set, 8 to Attack Position, 9 to Defence Position, 10 Surrender, 11 Confirm, 12 Draw,
/// 13 Shuffle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardCommand {
    pub command: u32,
    pub player: u32,
    pub zone: u32,
    pub index: u32,
}

impl CardCommand {
    pub fn from_message(m: &Message) -> Option<CardCommand> {
        let int = |k| m.get(k).and_then(Value::as_u32);
        if m.request() != Some(CLIENT_DUEL) || int("duelRequestName") != Some(request::CARD_COMMAND) {
            return None;
        }
        Some(CardCommand { command: int("CommandID")?, player: int("player")?, zone: int("position")?, index: int("index")? })
    }
}

/// A client message on the duel connection (request 0x200C), by its duelRequestName ([`request`]). Read here once;
/// NPC duels (`Duel`) and duels between players (`Table`) act on it, and [`Translator::request`] answers the ones
/// that are answers to the engine's prompts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClientRequest {
    /// A card menu entry (2).
    Command(CardCommand),
    /// A phase button {phase} (3): 0 DP .. 5 EP.
    Phase(u32),
    /// A closing dialog's result {DLGResult} (4): its button, or a card list's number of picks.
    DialogResult(u32),
    /// The field's right-click (5), while the command map's other flag 6 is set.
    RightClick,
    /// A card list's pick {index: its list position} (7); the first pick's own message (6) is not read.
    Pick(u32),
    /// The janken hand {selectHand} (0xC): 0 rock, 1 scissors, 2 paper.
    Hand(u32),
    /// First or second {choice} (0xD): [`FIRST`] or [`SECOND`].
    Order(u32),
    /// Card Swap is done (0xE).
    Swapped,
    /// The duel options {autoSet} (0xF): auto card placing.
    Options { auto_place: bool },
    /// The connection DLL's settle answer {seq} (`clock::SETTLED`).
    Settled(u32),
}

impl ClientRequest {
    pub fn of(m: &Message) -> Option<ClientRequest> {
        let int = |k| m.get(k).and_then(Value::as_u32);
        if m.request() != Some(CLIENT_DUEL) {
            return None;
        }
        Some(match int("duelRequestName")? {
            request::CARD_COMMAND => ClientRequest::Command(CardCommand::from_message(m)?),
            request::PHASE_BUTTON => ClientRequest::Phase(int("phase")?),
            request::DIALOG_RESULT => ClientRequest::DialogResult(int("DLGResult")?),
            request::RIGHT_CLICK => ClientRequest::RightClick,
            request::PICK => ClientRequest::Pick(int("index")?),
            request::SELECT_HAND => ClientRequest::Hand(int("selectHand").unwrap_or(0) % 3),
            request::SELECT_FIRST => ClientRequest::Order(if int("choice") == Some(SECOND) { SECOND } else { FIRST }),
            request::SWAPPED => ClientRequest::Swapped,
            request::DUEL_OPTIONS => ClientRequest::Options { auto_place: int("autoSet") != Some(0) },
            crate::clock::SETTLED => ClientRequest::Settled(int("seq").unwrap_or(0)),
            _ => return None,
        })
    }
}

/// The setup message: who goes first, and both decks' sizes.
/// The translator of an NPC duel: `decks` are the player's and the NPC's main decks (CARD_Prop rows), and the NPC's
/// hidden cards (deck, hand, extra deck, face-down) reach the client without their rows, as a player opponent's do.
/// The client shows any card it has a row for, face-down or not (the attack-target window, the card panel).
pub fn npc_translator(first: u32, decks: [&[u16]; 2], cards: HashMap<u16, crate::translate::CardStats>) -> (Translator, Vec<Message>) {
    Translator::against_player(first, decks, cards)
}

pub(crate) fn setup(first: u32, mine: &Deck, theirs: &Deck) -> Message {
    let len = |cards: &Vec<u16>| cards.len() as u32;
    // PlayerType: 0 for the client, 1 for the CPU (meaning unknown, a guess).
    [
        ("FirstPlayer", first),
        ("PlayerNum", CLIENT),
        ("PlayerType001", 0),
        ("PlayerType002", 1),
        ("MainDeckNum001", len(&mine.main)),
        ("MainDeckNum002", len(&theirs.main)),
        ("ExtraDeckNum001", len(&mine.extra)),
        ("ExtraDeckNum002", len(&theirs.extra)),
    ]
    .into_iter()
    .fold(Message::reply(SERVER_DUEL).with("duelRequestName", Value::U32(request::DUEL_SETUP)), |m, (k, v)| m.with(k, Value::U32(v)))
}

/// The deck and extra deck piles of both players, from the setup's counts: after the setup and before any draw.
pub(crate) fn piles() -> [Message; 4] {
    [(CLIENT, zone::DECK), (CPU, zone::DECK), (CLIENT, zone::EXTRA), (CPU, zone::EXTRA)].map(|(player, zone)| command(cmd::DECK_RESET, [player, zone, 0, 0]))
}

/// The janken {result, the opponent's hand, own hand} as the player who went first saw it, given as seat 0's (or the
/// player's) was: `lead_saw_it` when that player is the one who went first.
pub fn lead_janken([result, theirs, mine]: [u32; 3], lead_saw_it: bool) -> [u32; 3] {
    if lead_saw_it { [result, theirs, mine] } else { [[0, 2, 1][result as usize % 3], mine, theirs] }
}

/// The commands of a duel's replay (`duellog.rs`): the duel from the side of the player who went first (the first
/// deck), from the janken through the card table and the duel's events to the match's end, every card with its row
/// (Konami's replays hide no card: no card table entry with row 0 in 227 original logs, so both hands show). `history`
/// is every engine message of the duel.
pub fn replay_commands(world: &World, decks: [&Deck; 2], janken: [u32; 3], lp: u32, history: &[serde_json::Value]) -> Vec<[u32; 6]> {
    let view = world.card_view();
    let (mut translator, table) = Translator::new(CLIENT, [&decks[0].main, &decks[1].main], view.cards.clone());
    translator.tokens(world.token_rows.clone());
    translator.tuners(view.tuners.clone());
    let [result, theirs, mine] = janken;
    let mut messages = vec![
        command(cmd::JANKEN_BEGIN, [0; 4]),
        command(cmd::JANKEN_RESOLVE, [result, theirs, mine, 0]),
        command(cmd::FIRST_BEGIN, [0; 4]),
        command(cmd::FIRST_RESOLVE, [FIRST, 0, 0, 0]),
    ];
    messages.extend(table);
    messages.extend(translator.extra_decks([&decks[0].extra, &decks[1].extra]));
    messages.extend([command(cmd::DUEL_START, [0; 4]), command(cmd::SET_LP, [CLIENT, lp, 0, 0]), command(cmd::SET_LP, [CPU, lp, 0, 0])]);
    messages.extend(piles());
    messages.extend(translator.events(history));
    // The match's end for the leading player, which takes a watching client's DuelResult out of the duel.
    messages.push(command(cmd::MATCH_END, [translator.result().unwrap_or(3), 0, 0, 0]));
    crate::duellog::commands(&messages)
}

/// Fisher-Yates.
pub(crate) fn shuffle(rng: &mut u64, cards: &mut [u16]) {
    for i in (1..cards.len()).rev() {
        let j = (next(rng) % (i as u64 + 1)) as usize;
        cards.swap(i, j);
    }
}

/// xorshift64*
pub(crate) fn next(rng: &mut u64) -> u64 {
    *rng ^= *rng >> 12;
    *rng ^= *rng << 25;
    *rng ^= *rng >> 27;
    rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

/// One client's duel connection: against the server as the CPU, or at a table against another player.
pub struct DuelSession {
    identity: Option<crate::auth::Identity>,
    /// The LP this NPC duel started with (the `start-lp` setting at its start).
    start_lp: u32,
    world: Arc<World>,
    hub: Arc<Mutex<Hub>>,
    /// What the player brought (found by the player_id of 0x200A).
    entry: Option<DuelEntry>,
    /// The player, from 0x200A.
    player_id: Option<u32>,
    /// The rules engine running this duel, once it started.
    engine: Option<(Engine, Translator)>,
    /// The result went to the player's front connection.
    reported: bool,
    rng: u64,
    /// This connection's outbox, and in a duel against another player the shared table and this client's seat.
    outbox: Option<Outbox>,
    table: Option<(Arc<Mutex<Table>>, usize)>,
    /// Watching a duel between players: nothing it sends matters after 0x1838.
    watching: bool,
    /// The turn order was taken (against the CPU, once per duel): no second engine for a repeated 0xD or janken.
    ordered: bool,
    /// The address this connection comes from, which must be the one of the front connection that brought the deck.
    peer: Option<std::net::IpAddr>,
    /// The duel this connection came for is over (its match ended before it sat): nothing it sends matters.
    closed: bool,
    /// The duel check (`--duel-check`): the client's view of the NPC duel, compared with the engine after each step.
    check: Option<crate::duelcheck::ClientModel>,
    /// The NPC's lines in the chat window (crate::banter), its npc_id and name, and when the next line may go.
    banter: Option<(Banter, u32, String)>,
    /// The NPC's duel bot style for this duel (crate::npc::against: Pegasus can't see a Millennium Eye wearer).
    npc_style: serde_json::Value,
    chat_free: Instant,
    /// For the NPC duel's log: the janken as the player saw it {result, the NPC's hand, the player's}, who went first,
    /// and every engine message of the duel.
    janken_seen: [u32; 3],
    first: Option<u32>,
    history: Vec<serde_json::Value>,
}

impl Drop for DuelSession {
    fn drop(&mut self) {
        if let Some((table, seat)) = self.table.take() {
            table.locked().leave(seat);
        }
    }
}

impl DuelSession {
    /// A session whose CPU choices follow `seed`.
    pub fn new(world: Arc<World>, hub: Arc<Mutex<Hub>>, seed: u64) -> DuelSession {
        DuelSession {
            identity: None,
            start_lp: world.test_lp.unwrap_or_else(|| world.settings.now().start_lp),
            world,
            hub,
            entry: None,
            player_id: None,
            engine: None,
            reported: false,
            rng: seed | 1,
            outbox: None,
            table: None,
            watching: false,
            ordered: false,
            peer: None,
            closed: false,
            check: None,
            banter: None,
            npc_style: serde_json::Value::Null,
            chat_free: Instant::now(),
            janken_seen: [0; 3],
            first: None,
            history: vec![],
        }
    }

    /// The address this connection comes from: a duel entry brought by a front connection from another address is not
    /// claimed by it (0x200A names the player, and anyone can name anyone).
    pub fn with_peer(mut self, peer: std::net::IpAddr) -> DuelSession {
        self.peer = Some(peer);
        self
    }

    pub fn with_identity(mut self, identity: crate::auth::Identity) -> DuelSession {
        self.identity = Some(identity);
        self
    }

    /// The connection's outbox, through which the other player's commands reach this client.
    pub fn with_outbox(mut self, outbox: Outbox) -> DuelSession {
        self.outbox = Some(outbox);
        self
    }

    /// Sits the player at the table of a duel against another player (made by whoever connects first).
    fn sit(&mut self, duel_id: u32, entry: DuelEntry) -> Vec<Message> {
        let (Some(outbox), Some(player_id)) = (self.outbox.clone(), self.player_id) else { return vec![] };
        let seed = self.next();
        let table = {
            let mut hub = self.hub.locked();
            // The match is over already (the other player left before this one connected): no table to make for a
            // partner who isn't coming; this player, still before its janken, is told and sent back.
            if hub.ended(duel_id) {
                hub.cancel_for(player_id);
                self.closed = true;
                return vec![command(cmd::DUEL_END, [result::WON, 14, 0, 0])];
            }
            let make = || Arc::new(Mutex::new(Table::new(self.world.clone(), self.hub.clone(), duel_id, seed)));
            let table = hub.tables.entry(duel_id).or_insert_with(make).clone();
            hub.duel_began(duel_id);
            table
        };
        let mut t = table.locked();
        let Some((seat, out)) = t.sit(player_id, entry, outbox) else { return vec![] };
        let mine = t.deliver(seat, out);
        drop(t);
        self.table = Some((table, seat));
        mine
    }

    /// A message at a table: routed to this client's seat, the other client's part sent on.
    fn at_table(&mut self, msg: &Message) -> Vec<Message> {
        let Some((table, seat)) = self.table.clone() else { return vec![] };
        let mut t = table.locked();
        let out = match ClientRequest::of(msg) {
            Some(ClientRequest::Hand(hand)) => t.hand(seat, hand),
            Some(ClientRequest::Order(order)) => t.choose(seat, order),
            Some(ClientRequest::Swapped) => t.swapped(seat),
            Some(ClientRequest::Options { auto_place }) => {
                t.auto_place(seat, auto_place);
                Default::default()
            }
            Some(ClientRequest::Settled(seq)) => t.settled(seat, seq, Instant::now()),
            Some(answer) => t.request(seat, answer),
            None => Default::default(),
        };
        t.deliver(seat, out)
    }

    /// Handles one client packet and returns the messages to send, in order. With the duel check on, the client's view
    /// of an NPC duel follows them and is compared with the engine: an ERROR names where they differ.
    pub fn handle(&mut self, packet: &Packet) -> Vec<Message> {
        let mut out = self.respond_to(packet);
        if let (Some(model), Some((engine, _))) = (self.check.as_mut(), self.engine.as_mut()) {
            model.apply_all(&out);
            model.checkpoint_into(&mut out);
            match engine.state() {
                Ok(state) => {
                    let problems = model.check(&state);
                    if !problems.is_empty() {
                        // Once per duel: every later step would repeat the same difference.
                        tracing::error!("duel check (NPC duel, player {:?}): {}", self.player_id, problems.join(" | "));
                        self.check = None;
                    }
                }
                Err(e) => tracing::error!("duel check: no engine state: {e}"),
            }
        }
        out
    }

    fn respond_to(&mut self, packet: &Packet) -> Vec<Message> {
        let Packet::Message(msg) = packet else { return vec![] };
        let int = |k| msg.get(k).and_then(Value::as_u32);
        if self.watching || self.closed {
            return vec![];
        }
        match (msg.request(), int("duelRequestName")) {
            // A spectator: the table's catch-up, then (through the outbox) every event of the duel.
            (Some(WATCH), _) => {
                // Only a duel its host opened to watchers (the front's 0x1838 asks the same).
                let table = int("duel_id").and_then(|id| self.hub.locked().watchable_table(id));
                let (Some(table), Some(outbox)) = (table, self.outbox.clone()) else { return vec![] };
                self.watching = true;
                table.locked().watch(outbox)
            }
            (Some(ENTER), _) => {
                // Once per duel connection (0x50F690): a second ENTER would sit the same player again.
                if self.player_id.is_some() || self.table.is_some() {
                    tracing::warn!("a second duel entry on one duel connection (player {:?}); ignored", int("player_id"));
                    return vec![];
                }
                let mut entry = int("player_id").and_then(|id| self.hub.locked().duel_entry(id));
                if entry.as_ref().is_some_and(|e| e.identity != self.identity)
                    || (self.identity.is_some() && entry.is_none())
                {
                    tracing::warn!("duel entry does not belong to the authenticated launcher session; closed");
                    self.closed = true;
                    return vec![];
                }
                if let (Some(e), Some(peer)) = (&entry, self.peer)
                    && e.peer.is_some_and(|brought_by| brought_by != peer)
                {
                    tracing::warn!("duel connection from {peer} named player {:?}, whose deck came from {:?}; closed", int("player_id"), e.peer);
                    self.closed = true;
                    return vec![];
                }
                self.player_id = int("player_id");
                if let Some(entry) = entry.as_mut() {
                    entry.manual_place = int("autoSet") == Some(0);
                }
                if let (Some(entry), Some(rows)) = (entry.as_mut(), &self.world.test_deck) {
                    entry.deck.main = rows.iter().copied().cycle().take(40).collect();
                }
                if let (Some(entry), Some(rows)) = (entry.as_mut(), &self.world.test_extra) {
                    entry.deck.extra = rows.clone();
                }
                if let Some(entry) = entry.clone()
                    && let Some(duel_id) = entry.duel_id
                {
                    return self.sit(duel_id, entry);
                }
                self.entry = entry;
                vec![command(cmd::JANKEN_BEGIN, [0; 4])]
            }
            _ if self.table.is_some() => self.at_table(msg),
            (Some(CLIENT_DUEL), _) => match ClientRequest::of(msg) {
                Some(ClientRequest::Options { auto_place }) => {
                    if let Some(entry) = self.entry.as_mut() {
                        entry.manual_place = !auto_place;
                    }
                    if let Some((_, t)) = self.engine.as_mut() {
                        t.auto_place(auto_place);
                    }
                    vec![]
                }
                Some(ClientRequest::Hand(hand)) if !self.ordered => self.janken(hand),
                Some(ClientRequest::Order(order)) if !self.ordered => self.turn_order(order),
                Some(answer) => {
                    let prompt = self.engine.as_mut().and_then(|(_, t)| t.request(answer));
                    let mut out = self.prompted(prompt);
                    if !self.reported && self.engine.as_ref().is_some_and(|(_, t)| t.conceded()) {
                        out.extend(self.surrender());
                    }
                    out
                }
                None => vec![],
            },
            _ => vec![],
        }
    }

    /// The CPU's hand against the player's (0 rock, 1 scissors, 2 paper), and what follows: nothing
    /// on a draw (the scene asks for another hand by itself, 0x518ED0), the player's choice of turn
    /// order on a win, the CPU going first on a loss.
    fn janken(&mut self, mine: u32) -> Vec<Message> {
        let theirs = (self.next() % 3) as u32;
        // Each hand beats the next one: rock > scissors > paper > rock.
        let result = if theirs == mine { 0 } else if theirs == (mine + 1) % 3 { 1 } else { 2 };
        let mut out = vec![command(cmd::JANKEN_RESOLVE, [result, theirs, mine, 0])];
        self.janken_seen = [result, theirs, mine];
        match result {
            0 => {}
            1 => out.push(command(cmd::FIRST_BEGIN, [1, 0, 0, 0])),
            _ => {
                out.push(command(cmd::FIRST_BEGIN, [0; 4]));
                out.extend(self.turn_order(SECOND));
            }
        }
        out
    }

    /// The player's turn order, then the duel's opening.
    fn turn_order(&mut self, order: u32) -> Vec<Message> {
        self.ordered = true;
        let mut out = vec![command(cmd::FIRST_RESOLVE, [order, 0, 0, 0])];
        let first = if order == FIRST { CLIENT } else { CPU };
        let opening = match self.world.engine_dir.clone() {
            Some(dir) => self.engine_opening(first, &dir),
            None => None,
        };
        match opening {
            Some(opening) => out.extend(opening),
            // Without the rules engine (not fetched, or its program missing) nothing can play the duel: the scripted opening
            // would stop at the first Draw Phase forever (issue #2), so the duel ends at once as a draw, as a table's
            // does and as the startup error says.
            // (No opening at all without a duel entry or the NPC's deck.)
            None => {
                let scripted = self.opening(first);
                if !scripted.is_empty() {
                    tracing::warn!("NPC duel: the rules engine can't run, so the duel ends as a draw");
                    out.extend(scripted);
                    out.push(command(cmd::DUEL_END, [result::DRAW, 0, 0, 0]));
                    self.report(result::DRAW);
                }
            }
        }
        out
    }

    /// The opening with the rules engine: setup, both decks in the card table, start and LP, then whatever the
    /// engine does up to the client's first choice (its draws, turns and phases). None if the engine won't start.
    fn engine_opening(&mut self, first: u32, dir: &std::path::Path) -> Option<Vec<Message>> {
        let entry = self.entry.clone()?;
        let npc = entry.npc_id.and_then(|id| self.world.npc_decks.get(&id)).map(|d| self.world.place_deck(d))?;
        let decks = [&entry.deck, &npc];
        let cids = |rows: &[u16]| rows.iter().filter_map(|r| self.world.cards.get(r).map(|c| c.cid)).collect::<Vec<u16>>();
        // Engine team 0 goes first.
        let teams = if first == CLIENT { [decks[0], decks[1]] } else { [decks[1], decks[0]] };
        let (mut m0, e0, mut m1, e1) = (cids(&teams[0].main), cids(&teams[0].extra), cids(&teams[1].main), cids(&teams[1].extra));
        // The core deals in the order the cards were added (each new card goes on top; live, every duel opened
        // with the same hand), so the decks are shuffled here.
        self.shuffle(&mut m0);
        self.shuffle(&mut m1);
        let anime = self.world.settings.now().anime_cards;
        let mut engine = Engine::start_with(dir, anime).map_err(|e| tracing::warn!("rules engine not started: {e}")).ok()?;
        if self.world.duel_check { engine.record(); }
        let seed = self.next();
        // The LP at this duel's start (the `start-lp` setting now, #24), kept for the whole duel.
        let lp = self.world.test_lp.unwrap_or_else(|| self.world.settings.now().start_lp);
        self.start_lp = lp;
        // The NPC's hidden powers (crate::npc), for its engine team.
        let npc_team = if first == CLIENT { 1 } else { 0 };
        let avatar = self.player_id.and_then(|id| self.hub.locked().avatar_of(id)).unwrap_or_default();
        let base = entry.npc_id.and_then(|id| self.world.npc_styles.get(&id)).cloned().unwrap_or_default();
        self.npc_style = crate::npc::against(&base, &avatar);
        let blind = base["sees"] == true && self.npc_style["sees"] == false;
        let style = Some(&self.npc_style);
        let skills: Vec<(u32, String)> = style.and_then(|s| s["skills"].as_array()).into_iter().flatten()
            .filter_map(|s| s.as_str()).map(|s| (npc_team, s.to_string())).collect();
        let answer = engine.new_duel_with(seed, lp, [(&m0, &e0), (&m1, &e1)], &self.world.card_names, &skills);
        let view = self.world.card_view();
        let (mut translator, mut table) = npc_translator(first, [&entry.deck.main, &npc.main], view.cards.clone());
        if self.world.duel_check {
            // The NPC's hidden cards' rows, from a translator that hides nothing (the same uids in the same order).
            let mut model = crate::duelcheck::ClientModel::new(first, view.cards.clone());
            model.expect_profile(view.expectations());
            let (mut full, mut known) = Translator::new(first, [&entry.deck.main, &npc.main], view.cards.clone());
            known.extend(full.extra_decks([&entry.deck.extra, &npc.extra]));
            for m in &known {
                let int = |k: &str| m.get(k).and_then(Value::as_u32).unwrap_or(0);
                model.know(int("param1") & 0xFF_FFFF, int("param2") as u16);
            }
            self.check = Some(model);
        }
        translator.tokens(self.world.token_rows.clone());
        if engine.plays_anime() { translator.note(crate::engine::ANIME_NOTE); }
        if let Some(id) = self.identity.filter(|i| !self.world.profiles.current_for(*i, anime)) {
            tracing::info!("user {}'s game runs another card profile than the server's now", id.user_id);
            translator.note(crate::engine::RESTART_NOTE);
        }
        translator.tuners(view.tuners.clone());
        translator.auto_place(!entry.manual_place);
        translator.allow_surrender(!entry.no_surrender);
        translator.card_kinds(view.card_kinds.clone());
        table.extend(translator.extra_decks([&entry.deck.extra, &npc.extra]));
        self.engine = Some((engine, translator));
        self.first = Some(first);
        self.history.clear();
        let player = self.player_id.and_then(|id| self.hub.locked().front_of(id));
        let you = player.map(|(name, _)| name).unwrap_or_default();
        self.banter = entry.npc_id.and_then(|id| {
            let name = self.world.npc_names.get(&id)?;
            let banter = Banter::new(name, npc_team as u64, &you, seed.rotate_left(17))?;
            Some((if blind { banter.blind() } else { banter }, id, name.clone()))
        });
        let greet = self.banter.as_mut().map(|(b, ..)| b.greet()).unwrap_or_default();
        self.speak(greet);
        let mut out = vec![setup(first, &entry.deck, &npc)];
        out.extend(table);
        out.extend([
            command(cmd::DUEL_START, [0; 4]),
            command(cmd::SET_LP, [CLIENT, lp, 0, 0]),
            command(cmd::SET_LP, [CPU, lp, 0, 0]),
        ]);
        out.extend(piles());
        out.extend(self.advance(answer));
        Some(out)
    }

    /// Runs the engine from `answer`: its events become client commands, the CPU's prompts (and those the client
    /// has no window for yet) are answered, until the client must choose or the duel ends.
    /// A translator's answer to the client: the engine onwards, or the next window of the same choice.
    fn prompted(&mut self, prompt: Option<Prompt>) -> Vec<Message> {
        match prompt {
            Some(Prompt::Answer(r)) => match self.engine.as_mut().map(|(e, _)| e.respond(r)) {
                Some(answer) => self.advance(answer),
                None => vec![],
            },
            Some(Prompt::Client(out)) => {
                self.notes();
                out
            }
            None => vec![],
        }
    }

    /// What the duel windows can't say (an option list's texts), to the player's chat window now.
    fn notes(&mut self) {
        let Some((_, translator)) = self.engine.as_mut() else { return };
        let notes = translator.take_notes();
        let Some(player_id) = self.player_id.filter(|_| !notes.is_empty()) else { return };
        if let Some((_, outbox)) = self.hub.locked().front_of(player_id) {
            for note in notes {
                let _ = outbox.send(banter::note_line(&note));
            }
        }
    }

    fn advance(&mut self, mut answer: std::io::Result<Answer>) -> Vec<Message> {
        let style = self.npc_style.clone();
        let Some((engine, translator)) = self.engine.as_mut() else { return vec![] };
        let mut out = vec![];
        let mut said: Vec<Said> = vec![];
        if let Some((b, ..)) = self.banter.as_mut() { b.begin(); }
        let mut failed = false;
        let mut repeats = Repeats::default();
        // Each round answers one prompt; a duel needs far fewer between two of the client's choices.
        for _ in 0..10_000 {
            let a = match answer {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!("rules engine: {e}");
                    failed = true;
                    break;
                }
            };
            for e in &a.errors {
                tracing::warn!("rules engine: {e}");
            }
            if let Some(model) = self.check.as_mut() { model.observe_engine(&a.messages); }
            self.history.extend(a.messages.iter().cloned());
            out.extend(translator.events(&a.messages));
            if let Some((b, ..)) = self.banter.as_mut() {
                said.extend(b.observe(&a.messages, &mut banter::Live { engine: &mut *engine, world: &self.world }));
            }
            let last = a.messages.last();
            let (true, Some(last)) = (a.status == "await", last) else { break };
            match translator.prompt(last) {
                Prompt::Answer(response) => {
                    if repeats.stuck(last) {
                        tracing::warn!("rules engine: the same prompt again and again ({last}); giving the duel up");
                        failed = true;
                        break;
                    }
                    // The NPC's own choices come from the duel bot (crates/yo3-engine/src/bot); the translator's
                    // answer stays for the prompts the client has no window for, and if the engine can't think.
                    let response = if translator.prompted(last) {
                        response
                    } else {
                        engine.think(style.clone()).unwrap_or_else(|e| {
                            tracing::warn!("duel bot: {e}; the NPC plays by the simple rules");
                            response
                        })
                    };
                    answer = engine.respond(response)
                }
                Prompt::Client(messages) => {
                    out.extend(messages);
                    break;
                }
            }
        }
        crate::translate::settle_positions(&mut out);
        // An engine that died or hung can't be asked again: the duel ends as a draw (a no-result end the client shows).
        let result = if failed {
            out.push(command(cmd::DUEL_END, [result::DRAW, 0, 0, 0]));
            Some(result::DRAW)
        } else {
            translator.result()
        };
        if failed {
            self.engine = None;
        }
        if let Some(result) = result {
            self.report(result);
        }
        self.speak(said);
        self.notes();
        out
    }

    /// Sends the NPC's lines to the player's chat window, each when the client should be showing its moment (and
    /// some time after the one before).
    fn speak(&mut self, said: Vec<Said>) {
        let (Some(player_id), Some((_, npc_id, name)), false) = (self.player_id, self.banter.as_ref(), said.is_empty()) else { return };
        let Some((_, outbox)) = self.hub.locked().front_of(player_id) else { return };
        let now = Instant::now();
        let mut lines = vec![];
        for s in said {
            let at = (now + Duration::from_secs_f64(s.after * banter::PACE)).max(self.chat_free);
            self.chat_free = at + banter::GAP;
            lines.push((at, banter::chat_line(*npc_id, name, &s.text)));
        }
        std::thread::spawn(move || {
            for (at, line) in lines {
                std::thread::sleep(at.saturating_duration_since(Instant::now()));
                if outbox.send(line).is_err() {
                    break;
                }
            }
        });
    }

    /// The player surrendered (the deck menu's Surrender, confirmed): the duel ends for them as a loss, the NPC's win
    /// counted as after any duel (see `Table::surrender`); the engine is left where it was.
    fn surrender(&mut self) -> Vec<Message> {
        let Some((_, t)) = self.engine.as_mut() else { return vec![] };
        let out = t.surrender_end(true);
        self.report(result::LOST);
        out
    }

    /// Tells the hub the NPC duel ended with `result`, once.
    fn report(&mut self, result: u32) {
        if let (Some(player_id), false) = (self.player_id, self.reported) {
            self.reported = true;
            let opponent = self.entry.as_ref().and_then(|e| e.npc_id).unwrap_or(0);
            let duel_id = self.entry.as_ref().and_then(|e| e.duel_id).unwrap_or(0);
            let mut hub = self.hub.locked();
            let replay = self.replay(&hub);
            let stats = self.engine.as_ref().map(|(_, t)| t.stats().clone());
            let start_lp = self.start_lp;
            let ended = Ended { player_id, opponent, npc: true, duel_id, match_type: SINGLE_MODE, result, replay, stats, start_lp };
            hub.finish_duel(&ended);
        }
    }

    /// The NPC duel as a replay (`duellog::Replay`, issue #12), as a duel between players is kept (`Table::replay`): from
    /// the side of whoever went first, the player or the NPC, with that side first. None for a duel the engine didn't
    /// play (no history). The client fetches it through the connection DLL, which lets DuelResult (0x512070) take its
    /// dlogURL step for NPC duels too (`tools/mod/npc-duel-log.h`).
    fn replay(&self, hub: &Hub) -> Option<crate::duellog::Replay> {
        let (entry, first, player_id) = (self.entry.as_ref()?, self.first?, self.player_id?);
        let npc_id = entry.npc_id?;
        let npc = self.world.npc_decks.get(&npc_id)?;
        if self.history.is_empty() {
            return None;
        }
        let mut theirs = hub.side(player_id, npc, 1);
        theirs.player_id = npc_id;
        theirs.name = self.world.npc_names.get(&npc_id).cloned().unwrap_or_default();
        (theirs.level, theirs.title, theirs.record, theirs.avatar, theirs.speeches) = (0, 0, [0; 3], vec![], vec![]);
        let mine = hub.side(player_id, &entry.deck, 0);
        let janken = lead_janken(self.janken_seen, first == CLIENT);
        let (sides, decks) = if first == CLIENT { ([mine, theirs], [&entry.deck, npc]) } else { ([theirs, mine], [npc, &entry.deck]) };
        let commands = replay_commands(&self.world, decks, janken, self.start_lp, &self.history);
        Some(crate::duellog::Replay { sides, commands })
    }

    /// Setup (both decks' sizes), both shuffled decks in the card table, the duel start, both players'
    /// LP, five cards each (the first player first), then the first player's Draw Phase. Nothing
    /// without the player's deck and an NPC to play.
    fn opening(&mut self, first: u32) -> Vec<Message> {
        let Some(entry) = self.entry.clone() else { return vec![] };
        let Some(npc) = entry.npc_id.and_then(|id| self.world.npc_decks.get(&id)).map(|d| self.world.place_deck(d)) else { return vec![] };
        let mut out = vec![setup(first, &entry.deck, &npc)];
        // uids 1.. in table order: the client's deck, then the CPU's.
        let mut decks = [entry.deck.main.clone(), npc.main.clone()];
        let mut uids: [Vec<u32>; 2] = [vec![], vec![]];
        for (player, deck) in decks.iter_mut().enumerate() {
            self.shuffle(deck);
            for (i, &row) in deck.iter().enumerate() {
                let uid = uids[0].len() as u32 + uids[1].len() as u32 + 1;
                uids[player].push(uid);
                out.push(Cmd::Card { uid, open: false, row: row as u32, place: table_place(player as u32, zone::DECK, i) }.message());
            }
        }
        out.extend([
            command(cmd::DUEL_START, [0; 4]),
            command(cmd::SET_LP, [CLIENT, self.start_lp, 0, 0]),
            command(cmd::SET_LP, [CPU, self.start_lp, 0, 0]),
        ]);
        out.extend(piles());
        for player in [first, 1 - first] {
            let p = player as usize;
            for i in 0..OPENING_HAND.min(decks[p].len()) {
                let (uid, row) = (uids[p][i], decks[p][i] as u32);
                out.push(Cmd::Card { uid, open: false, row, place: table_place(player, zone::HAND, i) }.message());
                out.push(Cmd::Move { uid, kind: row, from: Place { player, zone: zone::DECK, ..Place::default() }, to: Place { player, zone: zone::HAND, index: i, ..Place::default() } }.message());
            }
        }
        out.push(command(cmd::PHASE, [first, DRAW_PHASE, 0, 0]));
        out
    }

    fn shuffle(&mut self, cards: &mut [u16]) {
        shuffle(&mut self.rng, cards)
    }

    fn next(&mut self) -> u64 {
        next(&mut self.rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;

    fn message(id: u32, fields: &[(&str, u32)]) -> Packet {
        Packet::Message(fields.iter().fold(Message::reply(id), |m, (k, v)| m.with(k, Value::U32(*v))))
    }

    /// 1001 and 1002 seated at the table of duel 7, with what each was pushed.
    type Seat = (DuelSession, tokio::sync::mpsc::UnboundedReceiver<Message>);

    fn seated() -> (Arc<Mutex<Hub>>, [Seat; 2]) {
        let world = Arc::new(World::new((1..=20).collect()));
        let hub = Arc::new(Mutex::new(Hub::default()));
        let seats = [(1001, 1002), (1002, 1001)].map(|(me, them)| {
            let entry = DuelEntry { duel_id: Some(7), opponent: Some(them), best_of: 1, ..DuelEntry::default() };
            hub.locked().enter_duel(me, entry);
            let (tx, rx) = unbounded_channel();
            let mut session = DuelSession::new(world.clone(), hub.clone(), 3).with_outbox(tx);
            session.handle(&message(ENTER, &[("player_id", me), ("autoSet", 1)]));
            (session, rx)
        });
        (hub, seats)
    }

    fn ended_as_draw(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Message>) -> bool {
        std::iter::from_fn(|| rx.try_recv().ok())
            .any(|m| m.get("id") == Some(&Value::U32(cmd::DUEL_END)) && m.get("param1") == Some(&Value::U32(result::DRAW)))
    }

    #[test]
    fn a_seats_auto_card_placing_follows_its_duel_options() {
        // ENTER's autoSet (seated() sends 1: on), then the duel options 0xF {player, autoSet} as the player changes it.
        let (hub, [(mut a, _ra), (_b, _rb)]) = seated();
        let table = hub.locked().tables.get(&7).cloned().expect("the table");
        assert_eq!(table.locked().manual_place(0), Some(false));
        let options = |on| message(CLIENT_DUEL, &[("duelRequestName", request::DUEL_OPTIONS), ("player", 0), ("autoSet", on)]);
        a.handle(&options(0));
        assert_eq!(table.locked().manual_place(0), Some(true));
        a.handle(&options(1));
        assert_eq!(table.locked().manual_place(0), Some(false));
    }

    #[test]
    fn a_poisoned_hub_does_not_stop_the_clock() {
        let (hub, _seats) = seated();
        let h = hub.clone();
        let _ = std::thread::spawn(move || {
            let _held = h.lock().unwrap();
            panic!("a bug under the hub lock");
        })
        .join();
        assert!(hub.is_poisoned());
        tick_tables(&hub, std::time::Instant::now());
        tick_tables(&hub, std::time::Instant::now());
    }

    #[test]
    fn a_table_that_panicked_ends_its_duel_and_the_others_keep_running() {
        let (hub, [(_a, mut rx_a), (_b, mut rx_b)]) = seated();
        let table = hub.locked().tables.get(&7).cloned().expect("the table");
        let t = table.clone();
        let _ = std::thread::spawn(move || {
            let _held = t.lock().unwrap();
            panic!("a bug under the table lock");
        })
        .join();
        assert!(table.is_poisoned());
        tick_tables(&hub, std::time::Instant::now());
        assert!(hub.locked().tables.is_empty(), "the broken table is dropped");
        assert!(ended_as_draw(&mut rx_a) && ended_as_draw(&mut rx_b), "both seats are told");
        // The clock task goes on.
        tick_tables(&hub, std::time::Instant::now());
    }

    #[test]
    fn a_table_that_is_busy_is_skipped_not_waited_for() {
        let (hub, _seats) = seated();
        let table = hub.locked().tables.get(&7).cloned().expect("the table");
        let _busy = table.lock().unwrap();
        // Would block forever on a plain lock (this thread holds it).
        tick_tables(&hub, std::time::Instant::now());
        assert!(hub.locked().tables.contains_key(&7));
    }
}
