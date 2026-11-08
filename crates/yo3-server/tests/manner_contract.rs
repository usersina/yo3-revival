//! Manner ratings. After a duel between players the server pushes `0x2CEC {eval_id}` (the global dispatcher keeps it in
//! slot 22 and sets front flag 0x40, `0x4F6024`); DuelResult (`0x512A40`) then opens `dlg_manner.swf`, and the chosen
//! button sends `0x2CED {eval_id, eval_data}` (`0x4C3F20`, `eval_data` from the table at `0x790004`: -10, -5, 0, 5, 10
//! for the buttons left to right). DuelResult waits for `0x2CF1` (`0x512A40` state 2: an id above `0x2CF0` and below
//! `0x2CF3`). The average of a player's ratings is their `evallevel` (0-4, the shop's five lines `0x48ABB6`).
mod common;

use common::{Server, one, req_with};
use yo3_proto::{Message, Value};
use yo3_server::front::{Ended, Player};

fn level_of(c: &mut common::Client, about: u32) -> Option<u32> {
    let search = [("search_flag", Value::U32(0x40000)), ("player_id", Value::U32(about)), ("search_type", Value::U32(1)), ("front_num", Value::U32(100))];
    let r = one(c.send(req_with(0x2CF0, &search)));
    let entry = r.items.iter().find(|i| i.get("player_id").and_then(Value::as_u32) == Some(about))?;
    entry.get("evallevel").and_then(Value::as_u32)
}

fn evaluation(pushed: &[Message]) -> Option<u32> {
    pushed.iter().find(|m| m.request() == Some(0x2CEC)).and_then(|m| m.get("eval_id").and_then(Value::as_u32))
}

fn finish(s: &Server, player_id: u32, opponent: u32) {
    let ended = Ended { player_id, opponent, npc: false, duel_id: 3, match_type: 1, result: 1, replay: None };
    s.hub.lock().unwrap().finish_duel(&ended);
}

#[test]
fn a_duel_between_players_asks_each_to_rate_the_other() {
    let s = Server::new();
    let (mut a, mut b) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"));
    a.pushed();
    b.pushed();
    finish(&s, 1001, 1002);
    finish(&s, 1002, 1001);
    let (ea, eb) = (evaluation(&a.pushed()), evaluation(&b.pushed()));
    assert!(ea.is_some() && eb.is_some() && ea != eb, "each player's own evaluation: {ea:?} {eb:?}");
}

#[test]
fn a_duel_against_an_npc_asks_for_no_rating() {
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    a.pushed();
    let ended = Ended { player_id: 1001, opponent: 125, npc: true, duel_id: 3, match_type: 1, result: 1, replay: None };
    s.hub.lock().unwrap().finish_duel(&ended);
    assert_eq!(evaluation(&a.pushed()), None);
}

#[test]
fn ratings_set_the_opponents_level_by_their_average() {
    let s = Server::new();
    let (mut a, _b) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"));
    assert_eq!(level_of(&mut a, 1002), Some(2), "no ratings: the middle");
    // Kaiba's ratings so far -> his level: 10 (average 10: 4), then 10 and -10 (0: 2), -3.3 (1), then -5 (1).
    for (value, level) in [(10i32, 4), (-10, 2), (-10, 1), (-10, 1)] {
        a.pushed();
        finish(&s, 1001, 1002);
        let id = evaluation(&a.pushed()).expect("an evaluation");
        let r = one(a.send(req_with(0x2CED, &[("eval_id", Value::U32(id)), ("eval_data", Value::U32(value as u32))])));
        assert_eq!(r.request(), Some(0x2CF1));
        assert_eq!(level_of(&mut a, 1002), Some(level), "after {value}");
    }
}

#[test]
fn an_evaluation_counts_once_and_only_for_its_rater() {
    let s = Server::new();
    let (mut a, mut b) = (s.join(1001, "Yugi"), s.join(1002, "Kaiba"));
    a.pushed();
    finish(&s, 1001, 1002);
    let id = evaluation(&a.pushed()).unwrap();
    let rate = |c: &mut common::Client| c.send(req_with(0x2CED, &[("eval_id", Value::U32(id)), ("eval_data", Value::U32(10))]));
    assert_eq!(one(rate(&mut b)).request(), Some(0x2CF2), "somebody else's evaluation is refused");
    assert_eq!(one(rate(&mut a)).request(), Some(0x2CF1));
    assert_eq!(one(rate(&mut a)).request(), Some(0x2CF2), "and counts once");
    assert_eq!(level_of(&mut a, 1002), Some(4));
}

#[test]
fn only_the_buttons_values_are_ratings() {
    let s = Server::new();
    let mut a = s.join(1001, "Yugi");
    let _b = s.join(1002, "Kaiba");
    a.pushed();
    finish(&s, 1001, 1002);
    let id = evaluation(&a.pushed()).unwrap();
    let r = one(a.send(req_with(0x2CED, &[("eval_id", Value::U32(id)), ("eval_data", Value::U32(7))])));
    assert_eq!(r.request(), Some(0x2CF2));
}

#[test]
fn ratings_are_kept_with_the_player() {
    let mut p = Player::new(1002, "Kaiba");
    p.eval_num = 2;
    p.eval_total = -5;
    assert_eq!((p.eval_num, p.eval_total), (2, -5));
}

#[test]
fn the_level_is_the_average_on_the_buttons_scale() {
    use yo3_server::front::manner_level;
    assert_eq!(manner_level(0, 0), 2);
    assert_eq!([(1, -10), (1, -5), (1, 0), (1, 5), (1, 10)].map(|(n, t)| manner_level(n, t)), [0, 1, 2, 3, 4]);
    assert_eq!(manner_level(4, -40), 0);
    assert_eq!(manner_level(3, 12), 3, "an average of 4 is nearer 5 than 0");
}
