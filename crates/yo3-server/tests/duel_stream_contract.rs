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

/// A deck whose main cards are rows `first..first + main`.
fn deck(first: u16, main: u16, extra: usize) -> Deck {
    Deck { id: 1, name: "Deck".into(), main: (first..first + main).collect(), extra: vec![2; extra], side: vec![] }
}

/// Player 1002 brings a 40-card deck against NPC 124, whose deck has 41 cards and 2 extra.
fn session() -> DuelSession {
    let mut world = World::new((1..=20).collect());
    world.npc_decks.insert(124, deck(1000, 41, 2));
    let mut hub = Hub::default();
    hub.enter_duel(1002, DuelEntry { deck: deck(100, 40, 0), npc_id: Some(124) });
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
        let others: Vec<_> = commands(&replies)[2..].iter().filter(|(id, _)| ![1000, 0x1A].contains(id)).copied().collect();
        let opening = [(1, [0; 4]), (0xF, [0, 8000, 0, 0]), (0xF, [1, 8000, 0, 0]), (5, [first, 0, 0, 0])];
        assert_eq!(others, opening);
    }
}

/// Where each uid is in the card table after `cmds` (command 1000), and its row.
fn table(cmds: &[(u32, [u32; 4])]) -> std::collections::HashMap<u32, (u32, u32)> {
    cmds.iter().filter(|(id, _)| *id == 1000).map(|(_, [uid, row, _, place])| (*uid, (*row, *place))).collect()
}

#[test]
fn both_decks_go_into_the_card_table_shuffled() {
    // Command 1000 (0x521DF0) sets a card in the engine's card table {uid, CARD_Prop row, 0, player |
    // zone << 8 | index << 16}, adding it if new; the scenes find cards there (0x523DF0 by place,
    // 0x523D40 by uid) and the card objects show the row's image (0x4FB090: data/card/<cid>_<n>.png).
    let mut s = session();
    enter(&mut s);
    let cmds = commands(&s.handle(&duel_msg(0xD, &[("choice", 1)])));
    let first: Vec<_> = cmds.iter().take_while(|(id, _)| *id != 1).filter(|(id, _)| *id == 1000).collect();
    assert_eq!(first.len(), 81, "every main deck card, before the duel starts");
    let mut uids: Vec<_> = first.iter().map(|(_, p)| p[0]).collect();
    uids.sort();
    uids.dedup();
    assert_eq!(uids.len(), 81);
    for (player, rows) in [(0, 100..140), (1, 1000..1041)] {
        let mut deck: Vec<_> = first.iter().filter(|(_, p)| p[3] & 0xFF == player).map(|(_, p)| (p[3] >> 16, p[1])).collect();
        assert!(first.iter().filter(|(_, p)| p[3] & 0xFF == player).all(|(_, p)| p[3] >> 8 & 0xFF == 0xD));
        deck.sort();
        assert_eq!(deck.iter().map(|(i, _)| *i).collect::<Vec<_>>(), (0..rows.len() as u32).collect::<Vec<_>>());
        let mut got: Vec<_> = deck.iter().map(|(_, r)| *r).collect();
        assert_ne!(got, rows.clone().collect::<Vec<_>>(), "shuffled");
        got.sort();
        assert_eq!(got, rows.collect::<Vec<_>>());
    }
}

#[test]
fn each_player_draws_five_before_the_first_draw_phase() {
    // A draw: the card moves to the hand in the card table (1000, zone 0xB), then command 0x1A moves
    // it on screen {uid | row << 16, from, to} with places as player | zone << 1 | index << 6
    // (DuelCardManager 0x4FF6A2; deck to hand plays the draw sound and counts it). Live: five cards
    // in the player's hand, five face down in the CPU's.
    for (choice, first) in [(1, 0), (2, 1)] {
        let mut s = session();
        enter(&mut s);
        let cmds = commands(&s.handle(&duel_msg(0xD, &[("choice", choice)])));
        let start = cmds.iter().position(|(id, _)| *id == 1).unwrap();
        let decks = table(&cmds[..start]);
        let moves: Vec<_> = cmds.iter().filter(|(id, _)| *id == 0x1A).collect();
        assert_eq!(moves.len(), 10);
        for (n, (_, [card, from, to, _])) in moves.iter().enumerate() {
            let player = if n < 5 { first } else { 1 - first };
            let (uid, row) = (card & 0xFFFF, card >> 16);
            let (deck_row, deck_place) = decks[&uid];
            assert_eq!((row, deck_place & 0xFF, deck_place >> 8 & 0xFF), (deck_row, player, 0xD));
            assert_eq!((*from & 1, *from >> 1 & 0x1F), (player, 0xD));
            assert_eq!((*to & 1, *to >> 1 & 0x1F, *to >> 6), (player, 0xB, n as u32 % 5));
        }
        let hands = table(&cmds[start..]);
        assert_eq!(hands.len(), 10);
        assert!(hands.values().all(|(_, place)| place >> 8 & 0xFF == 0xB));
        assert_eq!(cmds.last().unwrap().0, 5, "the Draw Phase comes last");
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

#[test]
fn command_data_gives_each_card_its_command_bits() {
    // Pump case 0x12 (0x5237CE): comIDsize pairs of u32 BE (card place, command bits) fill the command
    // map (0x520330), which the field queries by place `player | zone << 8 | index << 16` (0x5204E0;
    // DuelFieldManager highlights cards whose bits, less 0x2000, are set); then helpIDsize u32 pairs
    // and otherFlagSize byte pairs. Live: a hand card with bits 0-13 opens a 14-entry menu.
    let m = yo3_server::duel::command_data(&[(0x0001_0B00, 1 << 4 | 1 << 6), (0x0000_0B00, 1 << 1)], &[]);
    assert_eq!(m.request(), Some(SERVER_DUEL));
    assert_eq!(m.get("duelRequestName"), Some(&Value::U32(0x12)));
    assert_eq!(m.get("comIDsize"), Some(&Value::U32(2)));
    assert_eq!(m.get("helpIDsize"), Some(&Value::U32(0)));
    assert_eq!(m.get("otherFlagSize"), Some(&Value::U32(0)));
    let data = [0x00, 0x01, 0x0B, 0x00, 0, 0, 0, 0x50, 0x00, 0x00, 0x0B, 0x00, 0, 0, 0, 0x02];
    assert_eq!(m.get("COMIDDATA"), Some(&Value::Bin(data.to_vec())));
}

#[test]
fn a_card_command_names_the_command_and_the_card() {
    // Client duelRequestName 2 (0x520FD0), sent when a menu entry is chosen ("decide"): CommandID is
    // the bit number (0 Attack, 1 See Details, 2 Special Summon, 3 Activate Effect, 4 Summon, 5 Flip
    // Summon, 6/7 Set, 8 to Attack, 9 to Defence Position, 10 Surrender, 11 Confirm, 12 Draw, 13
    // Shuffle: the menu 0x4F9320 and duel_cardcmd.swf), position the zone and index the card in it.
    // Live: Summon on hand card 1 sent {CommandID 4, player 0, position 11, index 1}.
    let fields = [("CommandID", 4), ("player", 0), ("position", 11), ("index", 1)];
    let Packet::Message(m) = duel_msg(2, &fields) else { unreachable!() };
    let cmd = yo3_server::duel::CardCommand::from_message(&m).expect("a card command");
    assert_eq!((cmd.command, cmd.player, cmd.zone, cmd.index), (4, 0, 0xB, 1));
    let Packet::Message(other) = duel_msg(4, &[("DLGResult", 1)]) else { unreachable!() };
    assert!(yo3_server::duel::CardCommand::from_message(&other).is_none());
}
