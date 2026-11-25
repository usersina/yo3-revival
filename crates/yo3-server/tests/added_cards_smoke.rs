//! A smoke test for every card the game never had (#75, `engine/added-cards.json`), new ones included without a line of
//! test code: each starts on the field (a monster face-up, a Spell or Trap Set) with a copy in the hand, and two turns are
//! played each way through the real core, both seats' client models checked after every step. A script that errors in
//! the engine, or a card the client's view can't follow, fails here before anyone writes its effect scenarios
//! (`added_cards.rs`, `added_batch.rs`). Anime-only cards play with the anime versions on, as they only ever do; the
//! printed ones play both ways (with anime cards on, the ones with an anime version play that).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace};

const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: an opponent's monster to battle and target

#[test]
fn every_added_card_starts_on_the_field_and_plays_two_turns_without_an_error() {
    let Some(dir) = engine_dir() else { return };
    let added = yo3_data::added::Added::load(&dir).expect("engine/added-cards.json");
    assert!(!added.cards.is_empty());
    let mut failures = vec![];
    // Each card as printed (anime cards off), and each printed card again with anime cards on, when it may play its anime
    // version (engine/anime-cards.json); an anime-only card only ever plays with them on.
    let runs = added.cards.iter().map(|card| (card, card.anime_only)).chain(added.cards.iter().filter(|c| !c.anime_only).map(|card| (card, true)));
    for (card, anime) in runs {
        let c = card.card();
        let place = match c.kind {
            13 | 14 => SpawnPlace::SpellSet, // a Spell or Trap: Set, as a player would put it down
            _ => SpawnPlace::Monster,
        };
        let extra = c.extra_deck();
        let field = [Spawn { team: 0, place, cid: card.cid }, Spawn { team: 0, place: SpawnPlace::Hand, cid: card.cid },
            Spawn { team: 1, place: SpawnPlace::Monster, cid: WARWOLF }];
        let started = if anime { Scenario::start_anime_field(&dir, 3, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field) }
            else { Scenario::start_field(&dir, 3, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field) };
        let what = format!("{} (row {}, id {}{}{})", card.name, card.row, card.cid, if extra { ", Extra Deck" } else { "" },
            if anime && !card.anime_only { ", anime cards on" } else { "" });
        let mut s = match started {
            Ok(s) => s,
            Err(e) => { failures.push(format!("{what}: the duel didn't start: {e}")); continue }
        };
        if let Err(e) = s.run_both([&[EndTurn, EndTurn], &[EndTurn, EndTurn]]) {
            // A duel an added card ends early (an LP or deck effect) is no error of its own.
            if !e.contains("the duel is over") {
                failures.push(format!("{what}: {e}"));
                continue;
            }
        }
        if !s.problems.is_empty() {
            failures.push(format!("{what}: the client's view left the engine's: {:?}", s.problems));
        }
    }
    assert!(failures.is_empty(), "{} added card(s) failed:\n{}", failures.len(), failures.join("\n\n"));
}
