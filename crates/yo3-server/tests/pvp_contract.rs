//! A duel between two players on the duel connection: both matched players connect (0x200A), play the janken against
//! each other, and the rules engine runs one duel for both, each client seeing itself as player 0 (setup PlayerNum).
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{Message, Packet, Value};
use yo3_server::duel::{DuelSession, SERVER_DUEL};
use yo3_server::front::{Deck, DuelEntry, Hub, World};
use yo3_server::translate::CardStats;

const DUEL: u32 = 7;

/// Rows 1 (Gene-Warped Warwolf, 6960) and 2 (Vorse Raider, 5004): level 4 Normal Monsters the engine knows.
fn world(engine: bool) -> World {
    let stats = |cid, atk| CardStats { cid, atk, def: 0, level: 4, attribute: 1, race: 1 };
    let cards: HashMap<u16, CardStats> = [(1, stats(6960, 2000)), (2, stats(5004, 1900))].into_iter().collect();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    let engine_dir = (engine && dir.join("vendor/cardinfo.json").is_file()).then_some(dir);
    World { cards, engine_dir, ..World::new((1..=20).collect()) }
}

/// A seat: the session and what it was pushed.
struct Seat {
    session: DuelSession,
    inbox: UnboundedReceiver<Message>,
}

impl Seat {
    fn send(&mut self, name: u32, fields: &[(&str, u32)]) -> Vec<Message> {
        let m = Message::reply(0x200C).with("duelRequestName", Value::U32(name));
        self.session.handle(&Packet::Message(fields.iter().fold(m, |m, (k, v)| m.with(k, Value::U32(*v)))))
    }

    fn pushed(&mut self) -> Vec<Message> {
        std::iter::from_fn(|| self.inbox.try_recv().ok()).collect()
    }
}

/// Players 1001 (rows 1) and 1002 (rows 2) matched into duel 7; both seats, not entered yet.
fn table(engine: bool) -> Option<(Seat, Seat, Arc<Mutex<Hub>>)> {
    table_of(engine, 1, 8000)
}

/// The same, as a match of `best_of` duels starting with `lp` each.
fn table_of(engine: bool, best_of: u32, lp: u32) -> Option<(Seat, Seat, Arc<Mutex<Hub>>)> {
    let deck = |row| Deck { main: vec![row; 40], ..Deck::default() };
    table_with(engine, best_of, lp, [deck(1), deck(2)])
}

/// The same with these decks for 1001 and 1002.
fn table_with(engine: bool, best_of: u32, lp: u32, decks: [Deck; 2]) -> Option<(Seat, Seat, Arc<Mutex<Hub>>)> {
    let world = Arc::new(World { start_lp: lp, ..world(engine) });
    if engine && world.engine_dir.is_none() {
        eprintln!("engine not fetched; skipping");
        return None;
    }
    let hub = Arc::new(Mutex::new(Hub::default()));
    for ((me, them), deck) in [(1001, 1002), (1002, 1001)].into_iter().zip(decks) {
        let entry = DuelEntry { deck, duel_id: Some(DUEL), opponent: Some(them), best_of, ..DuelEntry::default() };
        hub.lock().unwrap().enter_duel(me, entry);
    }
    let seat = |seed| {
        let (tx, inbox) = unbounded_channel();
        Seat { session: DuelSession::new(world.clone(), hub.clone(), seed).with_outbox(tx), inbox }
    };
    Some((seat(3), seat(5), hub))
}

fn enter(seat: &mut Seat, player_id: u32) -> Vec<Message> {
    let m = Message::reply(0x200A).with("player_id", Value::U32(player_id)).with("autoSet", Value::U32(1));
    seat.session.handle(&Packet::Message(m))
}

/// The engine commands among `replies`: (id, [param1..param4]).
fn commands(replies: &[Message]) -> Vec<(u32, [u32; 4])> {
    replies
        .iter()
        .filter(|m| m.request() == Some(SERVER_DUEL) && m.get("duelRequestName") == Some(&Value::U32(0x11)))
        .map(|m| {
            let int = |k| m.get(k).and_then(Value::as_u32).unwrap();
            (int("id"), [int("param1"), int("param2"), int("param3"), int("param4")])
        })
        .collect()
}

/// Both seated and the janken won by 1001 (rock against scissors).
fn after_janken(engine: bool) -> Option<(Seat, Seat, Arc<Mutex<Hub>>)> {
    let (mut a, mut b, hub) = table(engine)?;
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    a.pushed();
    a.send(0xC, &[("selectHand", 0)]);
    b.send(0xC, &[("selectHand", 1)]);
    a.pushed();
    Some((a, b, hub))
}

#[test]
fn the_janken_starts_once_both_players_are_in() {
    let (mut a, mut b, _) = table(false).unwrap();
    assert!(enter(&mut a, 1001).is_empty());
    assert_eq!(commands(&enter(&mut b, 1002)), [(1500, [0; 4])]);
    assert_eq!(commands(&a.pushed()), [(1500, [0; 4])]);
}

#[test]
fn the_janken_is_played_between_the_two_hands() {
    // 1501 {result for this client: 0 draw, 1 won, 2 lost; the opponent's hand; this client's hand} (0 rock, 1
    // scissors, 2 paper), then 1502 {1 = this client picks the turn order} for the winner, {0} for the other.
    let (mut a, mut b, _) = table(false).unwrap();
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    a.pushed();
    assert!(a.send(0xC, &[("selectHand", 0)]).is_empty());
    assert_eq!(commands(&b.send(0xC, &[("selectHand", 1)])), [(1501, [2, 0, 1, 0]), (1502, [0; 4])]);
    assert_eq!(commands(&a.pushed()), [(1501, [1, 1, 0, 0]), (1502, [1, 0, 0, 0])]);
}

#[test]
fn a_drawn_janken_is_played_again() {
    // On a draw the scene asks for another hand by itself (0x518ED0).
    let (mut a, mut b, _) = table(false).unwrap();
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    a.pushed();
    a.send(0xC, &[("selectHand", 2)]);
    assert_eq!(commands(&b.send(0xC, &[("selectHand", 2)])), [(1501, [0, 2, 2, 0])]);
    assert_eq!(commands(&a.pushed()), [(1501, [0, 2, 2, 0])]);
    b.send(0xC, &[("selectHand", 2)]);
    assert_eq!(commands(&a.send(0xC, &[("selectHand", 1)])), [(1501, [1, 2, 1, 0]), (1502, [1, 0, 0, 0])]);
}

#[test]
fn the_winner_picks_the_turn_order_for_both() {
    // 1503 {1 first, 2 second}, from each client's side; only the janken's winner may choose.
    let (mut a, mut b, _) = after_janken(false).unwrap();
    assert!(b.send(0xD, &[("choice", 1)]).is_empty());
    assert_eq!(commands(&a.send(0xD, &[("choice", 2)]))[0], (1503, [2, 0, 0, 0]));
    assert_eq!(commands(&b.pushed())[0], (1503, [1, 0, 0, 0]));
}

#[test]
fn each_client_sees_its_own_side_and_the_first_player_gets_the_prompt() {
    // Setup 0x15: PlayerNum 0 and the client's own deck first; FirstPlayer from the client's side. Each card table
    // lists the client's deck as player 0. The first player's client gets the input wait (command 4), the other none.
    let Some((mut a, mut b, _)) = after_janken(true) else { return };
    let to_a = a.send(0xD, &[("choice", 2)]);
    let to_b = b.pushed();
    let setup = |out: &[Message]| out.iter().find(|m| m.get("duelRequestName") == Some(&Value::U32(0x15))).cloned().expect("setup");
    let int = |m: &Message, k| m.get(k).and_then(Value::as_u32).unwrap();
    assert_eq!((int(&setup(&to_a), "FirstPlayer"), int(&setup(&to_b), "FirstPlayer")), (1, 0));
    assert_eq!((int(&setup(&to_a), "PlayerNum"), int(&setup(&to_b), "PlayerNum")), (0, 0));
    let own_rows = |out: &[Message], row| commands(out).iter().filter(|(id, p)| *id == 1000 && p[3] & 0xFF == 0 && p[1] == row).count();
    assert!(own_rows(&to_a, 1) >= 35, "1001's own cards as player 0");
    assert!(own_rows(&to_b, 2) >= 35, "1002's own cards as player 0");
    let waits = |out: &[Message]| commands(out).iter().filter(|(id, p)| *id == 4 && p[1] == 0).count();
    assert_eq!((waits(&to_a), waits(&to_b)), (0, 1), "1002 goes first");
    let piles = |out: &[Message]| commands(out).into_iter().filter(|(id, _)| *id == 0x16).collect::<Vec<_>>();
    assert_eq!(piles(&to_a), [(0x16, [0, 0xD, 0, 0]), (0x16, [1, 0xD, 0, 0]), (0x16, [0, 0xC, 0, 0]), (0x16, [1, 0xC, 0, 0])]);
}

#[test]
fn the_opponents_hidden_cards_carry_no_row() {
    // A client may not learn the opponent's deck or hand from the card table (1000 {uid, row, 0, place}): the
    // opponent's deck and hand go in with row 0 (**guess**: Online 2 used placeholders too).
    let Some((mut a, mut b, _)) = after_janken(true) else { return };
    let to_a = a.send(0xD, &[("choice", 1)]);
    let to_b = b.pushed();
    for (out, theirs) in [(&to_a, 2), (&to_b, 1)] {
        let opponent: Vec<_> = commands(out).into_iter().filter(|(id, p)| *id == 1000 && p[3] & 0xFF == 1).collect();
        assert!(!opponent.is_empty());
        assert!(opponent.iter().all(|(_, p)| p[1] == 0), "{opponent:?}");
        assert!(!commands(out).iter().any(|(id, p)| *id == 1000 && p[3] & 0xFF == 1 && p[1] == theirs));
    }
}

#[test]
fn a_card_command_answers_its_seat_and_both_clients_see_the_summon() {
    // The first player Normal Summons the first monster of its hand: both clients get the move; the next prompt goes
    // to whoever the engine asks.
    let Some((mut a, mut b, _)) = after_janken(true) else { return };
    let to_a = a.send(0xD, &[("choice", 1)]);
    b.pushed();
    assert!(commands(&to_a).iter().any(|(id, _)| *id == 4), "1001 goes first");
    // Summon (bit 4) on hand card 0: {CommandID, player, position: zone, index}.
    let out = a.send(2, &[("CommandID", 4), ("player", 0), ("position", 0xB), ("index", 0)]);
    let to_b = b.pushed();
    let summoned = |out: &[Message], player| commands(out).iter().any(|(id, p)| *id == 0x1A && p[2] & 1 == player && (p[2] >> 1) & 0x1F < 5);
    assert!(summoned(&out, 0), "1001 sees its own summon");
    assert!(summoned(&to_b, 1), "1002 sees the opponent's summon");
}

#[test]
fn a_player_leaving_loses_the_duel() {
    // A player leaving the duel loses it: the other client gets the duel end (command 2 {1 won, 14 disconnected}).
    let Some((mut a, mut b, _)) = after_janken(true) else { return };
    a.send(0xD, &[("choice", 1)]);
    b.pushed();
    drop(a);
    assert_eq!(commands(&b.pushed()).last(), Some(&(2, [1, 14, 0, 0])));
}

/// One duel of a match from its janken: `first` wins the janken and goes first, ends its turn; `second` summons and
/// attacks directly for the win (the LP are lower than the monster's ATK). What each client got.
fn second_wins(first: &mut Seat, second: &mut Seat) -> (Vec<Message>, Vec<Message>) {
    let (mut to_first, mut to_second) = (vec![], vec![]);
    first.send(0xC, &[("selectHand", 0)]);
    to_second.extend(second.send(0xC, &[("selectHand", 1)]));
    to_first.extend(first.pushed());
    to_first.extend(first.send(0xD, &[("choice", 1)]));
    to_first.extend(first.send(3, &[("phase", 5)]));
    to_second.extend(second.pushed());
    to_second.extend(second.send(2, &[("CommandID", 4), ("player", 0), ("position", 0xB), ("index", 0)]));
    to_second.extend(second.send(3, &[("phase", 3)]));
    to_second.extend(second.send(2, &[("CommandID", 0), ("player", 0), ("position", 0), ("index", 0)]));
    to_first.extend(first.pushed());
    (to_first, to_second)
}

#[test]
fn a_match_goes_on_to_its_next_duel_until_someone_wins_two() {
    // In a match, DuelResult's first step (0x512070) finds 1500 (or 1504, Card Swap) at the head of the queue after the
    // duel's end and resets the match result: the match goes on. DuelManager (0x50FC00) then opens the next janken
    // (0x50F690), on the same duel connection when it is still open (state 0 skips the 0x22C4 wait). Once someone has
    // won two duels, no 1500 follows: the front push 0x2009 ends the match.
    let Some((mut a, mut b, _)) = table_of(true, 3, 100) else { return };
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    a.pushed();
    let ends = |out: &[Message]| commands(out).into_iter().filter(|(id, _)| matches!(id, 2 | 1500)).collect::<Vec<_>>();
    let (to_a, to_b) = second_wins(&mut a, &mut b);
    assert_eq!(ends(&to_a), [(2, [2, 1, 0, 0]), (1500, [0; 4])], "duel 1: 1001 lost, the match goes on");
    assert_eq!(ends(&to_b), [(2, [1, 1, 0, 0]), (1500, [0; 4])]);
    let (to_a, to_b) = second_wins(&mut a, &mut b);
    assert_eq!(ends(&to_a), [(2, [2, 1, 0, 0])], "duel 2: 1002 has won two");
    assert_eq!(ends(&to_b), [(2, [1, 1, 0, 0])]);
}

/// The rows of the client's own deck in a duel's opening card table (commands 1000 before the duel start, command 1):
/// player 0's deck places, row 0 for the opponent's hidden cards.
fn own_deck(out: &[Message]) -> Vec<u32> {
    let opening = commands(out).into_iter().skip_while(|(id, _)| *id != 1000).take_while(|(id, _)| *id == 1000);
    opening.map(|(_, p)| p[1]).filter(|&row| row != 0).collect()
}

/// 1001 with a side deck (rows 1, side five of row 2) and 1002 without one, in a match of three at 100 LP.
fn side_decked_match() -> Option<(Seat, Seat, Arc<Mutex<Hub>>)> {
    let decks = [Deck { main: vec![1; 40], side: vec![2; 5], ..Deck::default() }, Deck { main: vec![2; 40], ..Deck::default() }];
    let (mut a, mut b, hub) = table_with(true, 3, 100, decks)?;
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    a.pushed();
    Some((a, b, hub))
}

/// 1001's deck for the match changes, as the front's 0x1F4F does (Card Swap's update_type 2 keeps the match's entry).
fn swap(hub: &Arc<Mutex<Hub>>, main: Vec<u16>, side: Vec<u16>) {
    let entry = hub.lock().unwrap().duel_entry(1001).expect("1001's entry");
    hub.lock().unwrap().enter_duel(1001, DuelEntry { deck: Deck { main, side, ..entry.deck.clone() }, ..entry });
}

#[test]
fn a_player_with_a_side_deck_swaps_cards_between_a_matchs_duels() {
    // Between a match's duels DuelResult (0x512070) lets 1504 at the head of the queue through like 1500, and
    // DuelManager (0x50FC00) takes it to Card Swap (0x50FDF0: DeckReplace, cardchange.swf), which sends the swapped
    // deck to the front (0x1F4F update_type 2) and then a bare 0xE on the duel connection before the next janken
    // (0x50F690). So 1001, who has a side deck, gets 1504 and 1002 gets 1500; 1001's 0xE is answered with 1500, and
    // the next duel is played with the swapped deck.
    let Some((mut a, mut b, hub)) = side_decked_match() else { return };
    let ends = |out: &[Message]| commands(out).into_iter().filter(|(id, _)| matches!(id, 2 | 1500 | 1504)).collect::<Vec<_>>();
    let (to_a, to_b) = second_wins(&mut a, &mut b);
    assert_eq!(ends(&to_a), [(2, [2, 1, 0, 0]), (1504, [0; 4])], "1001 goes to Card Swap");
    assert_eq!(ends(&to_b), [(2, [1, 1, 0, 0]), (1500, [0; 4])], "1002 has no side deck: straight to the janken");
    assert!(own_deck(&to_a).iter().all(|&row| row == 1));
    swap(&hub, [vec![1; 35], vec![2; 5]].concat(), vec![1; 5]);
    assert_eq!(commands(&a.send(0xE, &[])), [(1500, [0; 4])]);
    let (to_a, _) = second_wins(&mut a, &mut b);
    assert_eq!(own_deck(&to_a).iter().filter(|&&row| row == 2).count(), 5, "the five swapped in");
}

#[test]
fn a_swap_that_changes_the_cards_keeps_the_deck_as_it_was() {
    // Card Swap only moves cards between the main (or extra) deck and the side deck: a deck with other cards is
    // refused and the player duels with the deck they had.
    let Some((mut a, mut b, hub)) = side_decked_match() else { return };
    second_wins(&mut a, &mut b);
    swap(&hub, vec![2; 40], vec![]);
    assert_eq!(commands(&a.send(0xE, &[])), [(1500, [0; 4])]);
    let (to_a, _) = second_wins(&mut a, &mut b);
    assert!(own_deck(&to_a).iter().all(|&row| row == 1), "still 1001's own deck");
}

#[test]
fn a_player_who_runs_out_of_time_loses() {
    // The first player is waited on: both clients get the clocks (0x14), the first player's running (timerKind 0 for
    // its client, 1 for the opponent's). Once its time is out (the server's clock check), it loses with reason 11
    // ("has ran out of time"), the other wins, and the hub counts it.
    let Some((mut a, mut b, hub)) = after_janken(true) else { return };
    let to_a = a.send(0xD, &[("choice", 1)]);
    let to_b = b.pushed();
    let timers = |out: &[Message]| -> Vec<(u32, u32)> {
        out.iter()
            .filter(|m| m.get("duelRequestName") == Some(&Value::U32(0x14)))
            .map(|m| (m.get("timerKind").and_then(Value::as_u32).unwrap(), m.get("start").and_then(Value::as_u32).unwrap()))
            .collect()
    };
    assert_eq!(timers(&to_a).last(), Some(&(0, 1)), "1001's clocks run on its own client");
    assert_eq!(timers(&to_b).last(), Some(&(1, 1)), "and as the opponent's on 1002's");
    yo3_server::duel::tick_tables(&hub, std::time::Instant::now() + std::time::Duration::from_secs(1000));
    assert_eq!(commands(&a.pushed()).last(), Some(&(2, [2, 11, 0, 0])));
    assert_eq!(commands(&b.pushed()).last(), Some(&(2, [1, 11, 0, 0])));
    assert_eq!(hub.lock().unwrap().record_result(1002, 1001, 0), [1, 0, 0], "1002's win is counted");
}

/// A spectator's duel connection on `hub`: it opens with 0x1838 {duel_id} (DuelManager's watching state 0x50F320)
/// instead of 0x200A.
fn spectator(hub: &Arc<Mutex<Hub>>) -> (Seat, Vec<Message>) {
    let (tx, inbox) = unbounded_channel();
    let mut s = Seat { session: DuelSession::new(Arc::new(world(true)), hub.clone(), 9).with_outbox(tx), inbox };
    let watch = Message::reply(0x1838).with("duel_id", Value::U32(DUEL));
    let caught_up = s.session.handle(&Packet::Message(watch));
    (s, caught_up)
}

#[test]
fn a_spectator_catches_up_and_then_sees_the_duel_with_both_hands_hidden() {
    // The spectator gets seat 0's view (1001's): the setup (0x15, PlayerNum 0), both decks in the card table with
    // every hidden card at row 0 (both hands and decks), the duel so far replayed (the opening draws), the clocks;
    // then each event as it happens, and never a prompt (no input wait 4, no command data 0x12).
    let Some((mut a, mut b, hub)) = after_janken(true) else { return };
    a.send(0xD, &[("choice", 1)]);
    b.pushed();
    let (mut s, caught_up) = spectator(&hub);
    // The pre-duel screen first plays seat 0's janken: its result with both hands (1001 rock, 1002 scissors), the turn
    // order without a choice of its own (1502 {0}), then 1001 going first (1503 {1}).
    let janken: Vec<_> = commands(&caught_up).into_iter().filter(|(id, _)| (1500..=1503).contains(id)).collect();
    assert_eq!(janken, [(1500, [0; 4]), (1501, [1, 1, 0, 0]), (1502, [0; 4]), (1503, [1, 0, 0, 0])]);
    let kind = |m: &Message| m.get("duelRequestName").and_then(Value::as_u32);
    let setup = caught_up.iter().find(|m| kind(m) == Some(0x15)).expect("the setup");
    assert_eq!(setup.get("PlayerNum").and_then(Value::as_u32), Some(0));
    let table: Vec<_> = commands(&caught_up).into_iter().filter(|(id, _)| *id == 1000).collect();
    assert!(table.len() >= 80, "both decks: {}", table.len());
    assert!(table.iter().all(|(_, p)| p[1] == 0), "no hidden card shows its row");
    let draws = commands(&caught_up).iter().filter(|(id, p)| *id == 0x1A && (p[2] >> 1) & 0x1F == 0xB).count();
    assert!(draws >= 10, "both opening hands replayed: {draws}");
    assert!(caught_up.iter().any(|m| kind(m) == Some(0x14)), "the clocks");
    assert!(!caught_up.iter().any(|m| kind(m) == Some(0x12)) && !commands(&caught_up).iter().any(|(id, _)| *id == 4));
    // 1001 summons: the spectator sees it, face up with its row, and gets no prompt.
    a.send(2, &[("CommandID", 4), ("player", 0), ("position", 0xB), ("index", 0)]);
    let seen = s.pushed();
    let summoned = commands(&seen).into_iter().find(|(id, p)| *id == 0x1A && (p[2] >> 1) & 0x1F < 5).expect("the summon");
    assert_eq!(summoned.1[2] & 1, 0, "on seat 0's side");
    assert!(commands(&seen).iter().any(|(id, p)| *id == 1000 && p[1] == 1), "the summoned card shows its row");
    assert!(!seen.iter().any(|m| kind(m) == Some(0x12)));
}

#[test]
fn the_front_sends_a_spectator_to_the_duel_server() {
    // 0x1838 {duel_id} (agent mode 4, 0x4A3340) -> 0x1839 {host, port, card_protector_1, card_protector_2} with the
    // items avatar_1 and avatar_2 (the players' records, by seat) and duelData {event_type, official, duel_id}
    // (0x4A3650); 0x183A when no such duel is being played.
    use yo3_server::front::{Account, FrontSession, Player};
    let Some((mut a, mut b, hub)) = after_janken(true) else { return };
    a.send(0xD, &[("choice", 1)]);
    b.pushed();
    let world = Arc::new(world(true));
    let front = |id: u32, name: &str| {
        let (tx, inbox) = unbounded_channel();
        let account = Account { players: vec![Player::new(id, name)], ..Account::test() };
        let mut f = FrontSession::new(account, world.clone(), hub.clone(), tx);
        f.handle(&Packet::Version("2010.03.23 skelton".into()));
        f.handle(&Packet::Message(Message::reply(0x1F4C).with("player_id", Value::U32(id))));
        (f, inbox)
    };
    let ((_yugi, _), (_kaiba, _), (mut watcher, _)) = (front(1001, "Yugi"), front(1002, "Kaiba"), front(1003, "Joey"));
    let watch = |duel_id| Packet::Message(Message::reply(0x1838).with("duel_id", Value::U32(duel_id)));
    let r = watcher.handle(&watch(DUEL)).remove(0);
    assert_eq!(r.request(), Some(0x1839));
    assert!(r.get("host").and_then(Value::as_str).is_some() && r.get("port").and_then(Value::as_u32).is_some());
    assert_eq!((r.get("card_protector_1"), r.get("card_protector_2")), (Some(&Value::U32(0)), Some(&Value::U32(0))));
    let item = |name: &str| r.items.iter().find(|i| i.get("name").and_then(Value::as_str).as_deref() == Some(name)).expect(name).clone();
    assert_eq!(item("avatar_1").get("player_id"), Some(&Value::U32(1001)));
    assert_eq!(item("avatar_2").get("player_id"), Some(&Value::U32(1002)));
    assert!(matches!(item("avatar_1").get("avatar1"), Some(Value::Bin(_))), "a full player record");
    let data = item("duelData");
    let int = |k| data.get(k).and_then(Value::as_u32);
    assert_eq!(["duel_id", "event_type", "official"].map(int), [Some(DUEL), Some(0), Some(0)]);
    assert_eq!(watcher.handle(&watch(99)).remove(0).request(), Some(0x183A));
}

#[test]
fn a_spectator_learns_the_matchs_result_once_the_match_is_over() {
    // A watching client's DuelResult (0x512070) takes the match result from command 1505 {result} at the head of its
    // queue (MainGamen +0x820; the other's by 0x504D00); without it the match goes on and the client waits on the
    // pre-duel screen. So a spectator gets seat 0's duel ends (1001's), and after the last one 1505 {1001's match
    // result}; between a match's duels, no 1505.
    let Some((mut a, mut b, hub)) = table_of(true, 3, 100) else { return };
    enter(&mut a, 1001);
    enter(&mut b, 1002);
    a.pushed();
    let (mut s, _) = spectator(&hub);
    let ends = |out: &[Message]| commands(out).into_iter().filter(|(id, _)| matches!(id, 2 | 1505)).collect::<Vec<_>>();
    second_wins(&mut a, &mut b);
    assert_eq!(ends(&s.pushed()), [(2, [2, 1, 0, 0])], "duel 1: 1001 lost, the match goes on");
    second_wins(&mut a, &mut b);
    assert_eq!(ends(&s.pushed()), [(2, [2, 1, 0, 0]), (1505, [2, 0, 0, 0])], "duel 2: 1002 won the match");
}

#[test]
fn a_spectator_learns_who_won_when_a_player_leaves() {
    let Some((mut a, b, hub)) = after_janken(true) else { return };
    a.send(0xD, &[("choice", 1)]);
    let (mut s, _) = spectator(&hub);
    drop(b);
    let ends: Vec<_> = commands(&s.pushed()).into_iter().filter(|(id, _)| matches!(id, 2 | 1505)).collect();
    assert_eq!(ends, [(2, [1, 14, 0, 0]), (1505, [1, 0, 0, 0])], "1001 won, by disconnection");
}
