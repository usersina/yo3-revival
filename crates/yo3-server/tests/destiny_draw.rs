//! Destiny Draw (#71), the host's `destiny-draw` option for casual duels against the computer: once per duel, at the start
//! of the player's Draw Phase with LP at or below 1000, the player may pick any card left in the Deck as the card drawn.
//! Played through the real core (the engine's `destiny_draw` skill, skills.lua) with the client's model checked after every
//! step (`duelcheck::scenario`); the dialogs are the translator's, through the server's texts (the connection DLL).
use serde_json::Value as Json;
use yo3_proto::{Message, Value};
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::ocg::msg;

const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: no effect
const SPIRIT_REAPER: u16 = 5526;

/// The player's Deck: five Warwolf for the hand, five more drawn first, then Spirit Reapers (drawn from the end).
fn deck() -> Vec<u16> {
    let mut d = vec![SPIRIT_REAPER; 30];
    d.extend([WARWOLF; 10]);
    d
}

fn start(lp: u32, destiny: bool) -> Option<Scenario> {
    let dir = engine_dir()?;
    let skills: Vec<(u32, String)> = if destiny { vec![(0, "destiny_draw".into())] } else { vec![] };
    Some(Scenario::start_skills(&dir, 7, lp, [(&deck(), &[]), (&[WARWOLF; 40], &[])], &skills).expect("the duel starts"))
}

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn asked(s: &Scenario, kind: u64) -> usize {
    s.prompts.iter().filter(|&&(team, k)| team == 0 && k == kind).count()
}

/// Passes until the player is asked `kind`, at most `limit` steps; false if the duel got there without.
fn until(s: &mut Scenario, kind: u64, limit: usize) -> bool {
    for _ in 0..limit {
        if s.prompt()["type"] == kind && s.prompt()["player"] == 0 {
            return true;
        }
        if s.pass().is_err() {
            return false;
        }
    }
    false
}

/// The server's own texts among `out` (the connection DLL shows them in the dialogs).
fn texts(out: &[Message]) -> Vec<String> {
    out.iter().filter(|m| m.get("duelRequestName") == Some(&Value::U32(0x7A12))).filter_map(|m| m.get("text")?.as_str()).collect()
}

fn hand(s: &Scenario, cid: u16) -> usize {
    s.count(0, "hand", cid)
}

#[test]
fn it_is_offered_at_the_draw_when_lp_is_1000_or_less_and_the_chosen_card_is_drawn() {
    let Some(mut s) = start(1000, true) else { return };
    assert!(until(&mut s, msg::SELECT_YESNO, 80), "never asked: {}", s.context());
    let (reapers, wolves) = (hand(&s, SPIRIT_REAPER), hand(&s, WARWOLF));
    let question = s.windows(0).expect("the question's window");
    assert!(texts(&question).iter().any(|t| t.contains("Destiny Draw")), "the question's text: {question:?}");
    // No: nothing drawn from the Deck's choice, and it is offered again at the next draw.
    s.answer_in_windows(0, &[], 0).unwrap();
    assert_eq!(asked(&s, msg::SELECT_YESNO), 1);
    assert!(until(&mut s, msg::SELECT_YESNO, 80), "not offered again: {}", s.context());
    assert_eq!(hand(&s, SPIRIT_REAPER), reapers, "declined, the draw was the Deck's top (Warwolf)");
    assert!(hand(&s, WARWOLF) > wolves);
    // Yes: any card left in the Deck, one of each name, titled by the server's text.
    let reapers = hand(&s, SPIRIT_REAPER);
    s.windows(0).expect("the question again");
    s.answer_in_windows(0, &[], 1).unwrap();
    let list = s.windows(0).expect("the card list");
    assert!(texts(&list).iter().any(|t| t.contains("Destiny Draw")), "the choice's title: {list:?}");
    let selects = s.prompt()["selects"].as_array().expect("a card choice").clone();
    assert_eq!(selects.len(), 2, "one of each name left in the Deck: {selects:?}");
    let reaper = selects.iter().position(|c| c["cid"] == SPIRIT_REAPER).expect("Spirit Reaper is on offer");
    s.answer_in_windows(0, &[reaper as u32], 1).unwrap();
    // The Draw Phase goes on and the player draws the chosen card.
    s.run(&[EndTurn]).unwrap();
    assert_eq!(hand(&s, SPIRIT_REAPER), reapers + 1, "the chosen card is in hand: {}", s.context());
    // Once per duel: never asked again, however many draws follow.
    for _ in 0..3 {
        s.run(&[EndTurn]).unwrap();
    }
    assert_eq!(asked(&s, msg::SELECT_YESNO), 2, "the decline and the use, none after");
    clean(&s);
}

#[test]
fn it_is_not_offered_above_1000_lp() {
    let Some(mut s) = start(1001, true) else { return };
    for _ in 0..4 {
        s.run(&[EndTurn]).unwrap();
    }
    assert_eq!(asked(&s, msg::SELECT_YESNO), 0, "asked at 1001 LP");
    clean(&s);
}

#[test]
fn it_is_not_offered_without_the_setting() {
    // The setting off gives the player no skill (duel.rs `destiny_draw_applies`): the same duel at 1000 LP never asks.
    let Some(mut s) = start(1000, false) else { return };
    for _ in 0..4 {
        s.run(&[EndTurn]).unwrap();
    }
    assert_eq!(asked(&s, msg::SELECT_YESNO), 0);
    clean(&s);
}

#[test]
fn the_texts_are_the_engines_own_strings() {
    // A skill's pseudo card code: its strings reach the dialogs as `description_text` and `hint_text`.
    let Some(mut s) = start(1000, true) else { return };
    assert!(until(&mut s, msg::SELECT_YESNO, 80));
    let p: &Json = s.prompt();
    assert_eq!(p["description_text"], "Destiny Draw: choose the card you draw this turn?");
}
