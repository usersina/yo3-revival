//! The duel connection (DuelServerConnection), opened after the 0x22C4 push. Both directions are
//! key/value messages with a duelRequestName. The client sends request 0x200C (built by 0x49F990)
//! with duelRequestName 1-0xF, and 0x200A once on connecting. The server's messages go through
//! the pump 0x5233A0, which switches on duelRequestName 0x10-0x15 (0x5201A0); 0x11 is an engine
//! command {id, param1..param4}, queued as {id, request, param1..param4} (0x523541) and popped by
//! the scenes (0x522A50). A negative request id counts as a lost connection (0x4F1860). A scene
//! pops the next command only once its queued states (animations) are done (0x4D3220, 0x517B90),
//! so several commands can be sent at once.
use std::sync::{Arc, Mutex};

use yo3_proto::{Message, Packet, Value};
use yo3_server::duel::{DuelSession, SERVER_DUEL};
use yo3_server::front::{Deck, DuelEntry, Hub, World};

fn deck(main: usize, extra: usize) -> Deck {
    Deck { id: 1, name: "Deck".into(), main: vec![1; main], extra: vec![2; extra], side: vec![] }
}

/// Player 1002 brings a 40-card deck against NPC 124, whose deck has 41 cards and 2 extra.
fn session() -> DuelSession {
    let mut world = World::new((1..=20).collect());
    world.npc_decks.insert(124, deck(41, 2));
    let mut hub = Hub::default();
    hub.enter_duel(1002, DuelEntry { deck: deck(40, 0), npc_id: Some(124) });
    DuelSession::new(Arc::new(world), Arc::new(Mutex::new(hub)), 7)
}

fn duel_msg(name: u32, fields: &[(&str, u32)]) -> Packet {
    let m = Message::reply(0x200C).with("duelRequestName", Value::U32(name));
    Packet::Message(fields.iter().fold(m, |m, (k, v)| m.with(k, Value::U32(*v))))
}

fn enter(s: &mut DuelSession) -> Vec<Message> {
    let m = Message::reply(0x200A).with("player_id", Value::U32(1002)).with("autoSet", Value::U32(1));
    s.handle(&Packet::Message(m))
}

/// The engine commands among `replies`: (id, [param1..param4]).
fn commands(replies: &[Message]) -> Vec<(u32, [u32; 4])> {
    replies
        .iter()
        .filter(|m| m.get("duelRequestName") != Some(&Value::U32(0x15)))
        .map(|m| {
            assert_eq!(m.request(), Some(SERVER_DUEL), "{m:?}");
            assert_eq!(m.get("duelRequestName"), Some(&Value::U32(0x11)), "{m:?}");
            let int = |k| m.get(k).and_then(Value::as_u32).unwrap_or_else(|| panic!("{k} in {m:?}"));
            (int("id"), [int("param1"), int("param2"), int("param3"), int("param4")])
        })
        .collect()
}

#[test]
fn entering_the_duel_starts_the_janken() {
    // JankenScene (0x518400) pops commands while the duel connection is up; 1500 (0x5DC) shows the
    // three hands (beginJanken, 0x518D80).
    let mut s = session();
    assert_eq!(commands(&enter(&mut s)), [(1500, [0; 4])]);
}

#[test]
fn a_hand_is_answered_with_both_hands_and_the_result() {
    // applyMyHand (0x518C00) sends duelRequestName 0xC {selectHand: 0 rock, 1 scissors, 2 paper}
    // (0x4F1330). 1501 (0x5DD) calls resolveJanken(myHand = param3, yourHand = param2, result =
    // param1: 0 draw, 1 win, 2 lose; tables 0x7F529C, 0x7F52A8).
    for hand in 0..3 {
        let mut s = session();
        enter(&mut s);
        let replies = s.handle(&duel_msg(0xC, &[("selectHand", hand)]));
        let cmds = commands(&replies);
        let (id, [result, theirs, mine, _]) = cmds[0];
        assert_eq!((id, mine), (1501, hand));
        assert!(theirs < 3);
        // Rock beats scissors, scissors beat paper, paper beats rock: hand + 1 loses to hand.
        let expected = if theirs == hand { 0 } else if theirs == (hand + 1) % 3 { 1 } else { 2 };
        assert_eq!(result, expected, "{cmds:?}");
        match result {
            // A draw: after the reveal the scene goes back to choosing by itself (0x518F50 ->
            // selectJanken 0x518ED0), so nothing follows.
            0 => assert!(cmds[1..].is_empty(), "{cmds:?}"),
            // Won: 1502 with param1 != 0 lets this player choose (selectFirst, 0x5191A0).
            1 => assert_eq!(cmds[1..], [(1502, [1, 0, 0, 0])]),
            // Lost: the CPU goes first. 1502 with 0 (beginFirst: wait), 1503 resolveFirst(isFirst =
            // param1 == 1, 0x519230), then 1505 ends the janken (0x5185A3): param1 is this player's
            // turn order (MainGamen+0x820, the other one at +0x84C), and param2 100 would take
            // another path.
            _ => assert_eq!(cmds[1..4], [(1502, [0; 4]), (1503, [2, 0, 0, 0]), (1505, [2, 0, 0, 0])]),
        }
    }
}

#[test]
fn the_winner_picks_first_or_second() {
    // applySelectFirst (0x518C60) sends duelRequestName 0xD {choice: 1 first, 2 second} (0x4F1430).
    for choice in [1, 2] {
        let mut s = session();
        enter(&mut s);
        let replies = s.handle(&duel_msg(0xD, &[("choice", choice)]));
        assert_eq!(commands(&replies)[..2], [(1503, [choice, 0, 0, 0]), (1505, [choice, 0, 0, 0])]);
    }
}

#[test]
fn the_duel_opens_after_the_janken() {
    // Pump case 0x15 stores FirstPlayer (engine+0x1AC), PlayerNum (+0x1B0, this client's player; the
    // scenes compare players with it), and per player PlayerType, MainDeckNum and ExtraDeckNum
    // (+0x1C4.., +0x1DC.., which command 0x1A keeps counting). Then the commands: 1 starts the duel
    // (DuelScene 0x514290), 0xF sets a player's LP {player, LP} (DuelCockpit 0x502EB0), 5 a phase
    // {player, phase: 0 draw} (DuelScene, DuelCockpit). Live: LP 8000 on both sides, DP lit. The
    // first turn needs no 6 (turn change), which would count it as turn 2.
    for (choice, first) in [(1, 0), (2, 1)] {
        let mut s = session();
        enter(&mut s);
        let replies = s.handle(&duel_msg(0xD, &[("choice", choice)]));
        let setup = replies.iter().find(|m| m.get("duelRequestName") == Some(&Value::U32(0x15))).expect("setup");
        assert_eq!(setup.request(), Some(SERVER_DUEL));
        let expected = [("FirstPlayer", first), ("PlayerNum", 0), ("MainDeckNum001", 40), ("MainDeckNum002", 41)]
            .into_iter()
            .chain([("ExtraDeckNum001", 0), ("ExtraDeckNum002", 2)]);
        for (k, v) in expected {
            assert_eq!(setup.get(k), Some(&Value::U32(v)), "{k}");
        }
        assert!(setup.get("PlayerType001").is_some() && setup.get("PlayerType002").is_some());
        let opening = [(1, [0; 4]), (0xF, [0, 8000, 0, 0]), (0xF, [1, 8000, 0, 0]), (5, [first, 0, 0, 0])];
        assert_eq!(commands(&replies)[2..], opening);
    }
}

#[test]
fn a_player_without_a_duel_entry_gets_no_opening() {
    let mut s = DuelSession::new(Arc::new(World::new(vec![])), Arc::new(Mutex::new(Hub::default())), 7);
    enter(&mut s);
    let replies = s.handle(&duel_msg(0xD, &[("choice", 1)]));
    assert_eq!(commands(&replies), [(1503, [1, 0, 0, 0]), (1505, [1, 0, 0, 0])]);
}

#[test]
fn other_duel_messages_get_no_answer_yet() {
    let mut s = session();
    enter(&mut s);
    assert!(s.handle(&duel_msg(4, &[("DLGResult", 1)])).is_empty());
    assert!(s.handle(&Packet::Message(Message::reply(0x2724))).is_empty());
}
