//! Fixes to anime-only cards found after their batches (#75), each with a scenario through the real core (both seats' client
//! models checked after every step: `Scenario::problems` empty, and no engine error, which `run_both` returns as an Err).
//! Protects:
//! - the equip-target guards of three overrides (an attack-negation or battle-protection condition that asked a missing equip
//!   target for its battle target): Cursed Ring (`engine/overrides/c511000442.lua`), Magnet Force Plus (`c170000122.lua`) and
//!   Magnet Force Minus (`c170000123.lua`). The equipped monster leaving in a battle, or to Dark Hole, with a battle after it:
//!   these pass with the guards. They also pass without them: the core removes an equip card in the same step as its target
//!   on those paths. The failure the guards stop is reached only when a monster is Released as the material of another's
//!   Summon while its Magnet Force is on the opponent's side (the fuzz's duels, seeds 63876 and 63889 of Magnet Force Minus and
//!   Plus: `YO3_FUZZ_ROWS=5685-5686 YO3_FUZZ_DUELS=40 task check:fuzz` fails with "attempt to index a nil value (local 'ec')"
//!   with the three overrides moved aside, and passes with them); no scenario of ours has reproduced it by hand;
//! - Purse with a Hole's "for the rest of this turn, your opponent cannot play cards from their hand": ProjectIgnis' own
//!   script already locks the hand (the core offers it nothing in that turn: no Summon, Set, Special Summon or Activate), batch
//!   15's note that the core still plays the hand was the next own turn's prompt; no override is needed;
//! - Flame Swordsman given as an Extra Deck card: with anime cards on it is a Main Deck card, as docs/anime-cards.md says;
//! - Load of Darkness: Infinite Fiend Mirror's Load of Darkness Tokens reach the client's field in the End Phase and leave
//!   with Load of Darkness when it is destroyed;
//! - Brilliant Shrine Art's redirect onto an Earthbound Immortal holding against a CPU attacker that re-picks its old target
//!   (`engine/overrides/c511002698.lua`).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 2, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7, 2500/2100 vanilla
const DARK_BLADE: u16 = 5581; // Level 4, 1800/1500 vanilla
const DARK_HOLE: u16 = 4342;
const FISSURE: u16 = 4835; // destroys the opponent's lowest-ATK face-up monster
const DIAN_KETO: u16 = 4348;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel, 8000 LP, both Decks of Feral Imps (team 0's `extra` is its Extra Deck).
fn duel(seed: u64, extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], extra), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, seed, 8000, decks, spawns).unwrap();
    s.prefer(&[(0, FILLER), (1, FILLER)]);
    Some(s)
}

fn field(spawns: &[Spawn]) -> Option<Scenario> { duel(7, &[], spawns) }

// ---------------------------------------------------------------------------------------------------------------
// Equip-target guards
// ---------------------------------------------------------------------------------------------------------------

/// Cursed Ring: team 1's Warwolf (2000) attacks team 0's Dark Magician (2500) and the Ring is chained: it equips the
/// attacker, and Warwolf is destroyed in the battle. The Ring's protection (`indtg`) is asked of every monster in the battle
/// and in the adjusts after it, with no equip target left to look at: no Lua error, the Ring goes with its monster.
#[test]
fn cursed_ring_survives_its_equipped_monster_leaving_in_the_battle() {
    let ring = card("Cursed Ring");
    let Some(mut s) = field(&[set(0, ring), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Chain(ring)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ring), 1, "the Ring left with its monster: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

/// Cursed Ring: the equipped monster is removed by an effect (team 0's own Dark Hole in its next turn) and a battle follows
/// from the monsters that come after it: the Ring is in the Graveyard and nothing is asked of a missing target.
#[test]
fn cursed_ring_survives_its_equipped_monster_destroyed_by_an_effect() {
    let ring = card("Cursed Ring");
    // Warwolf (2000) attacks Dark Blade (1800): the Ring equips it, so Dark Blade (the monster it battles) cannot be destroyed.
    let spawns = [set(0, ring), mon(0, DARK_BLADE), hand(0, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    s.run_both([&[Chain(ring), EndTurn, Activate(DARK_HOLE)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole took the equipped Warwolf: {}", s.context());
    assert_eq!(s.count(0, "grave", ring), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 200, "Warwolf 2000 against Dark Blade 1800 in attack: {}", s.context());
    // A battle follows with new monsters: no error, no drift.
    s.run_both([&[Summon(THUNDER_KID), EndTurn], &[Summon(WARWOLF), EndTurn]]).unwrap_or_default();
    clean(&s);
}

/// Magnet Force Plus (a Trap): it is equipped to Warwolf in turn 3; Warwolf attacks Dark Magician (2500) and is destroyed,
/// with the Trap still in its zone for the adjusts of that battle (its attack-negation condition asks the equip target for
/// its battle target).
#[test]
fn magnet_force_plus_survives_its_equipped_monster_leaving_in_the_battle() {
    let magnet = card("Magnet Force Plus");
    let Some(mut s) = field(&[set(0, magnet), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[EndTurn, Activate(magnet), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", magnet), 1, "the Trap left with its monster: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
}

/// Magnet Force Plus: team 1's Dark Hole (turn 2) is answered by the Trap (free chain, Set since turn 1): it is equipped to
/// Warwolf, and Dark Hole then destroys Warwolf (and Dark Magician) with the Trap still in its zone for the adjust.
#[test]
fn magnet_force_plus_survives_its_equipped_monster_destroyed_by_an_effect() {
    let magnet = card("Magnet Force Plus");
    let spawns = [set(0, magnet), mon(0, WARWOLF), hand(1, DARK_HOLE), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(magnet)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", magnet), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// Magnet Force Minus (a Trap): the same two cases as the Plus (a battle death, then a Dark Hole): the card's script is the
/// Plus script with the poles swapped.
#[test]
fn magnet_force_minus_survives_its_equipped_monster_leaving() {
    let magnet = card("Magnet Force Minus");
    let Some(mut s) = field(&[set(0, magnet), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[EndTurn, Activate(magnet), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", magnet)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
    let spawns = [set(0, magnet), mon(0, WARWOLF), hand(1, DARK_HOLE), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(magnet)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", magnet), s.count(0, "grave", WARWOLF)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Purse with a Hole
// ---------------------------------------------------------------------------------------------------------------

/// What team 0's first idle prompt offers from its hand: summons, Sets, Special Summons and activations.
fn playable(s: &Scenario) -> usize {
    let p = s.prompt();
    ["summons", "monster_sets", "special_summons", "spell_sets", "activates"]
        .iter()
        .map(|l| p[l].as_array().map_or(0, Vec::len))
        .sum()
}

/// Purse with a Hole: "send random cards from their hand until they have 5 cards. For the rest of this turn, your opponent
/// cannot play cards from their hand." Team 1's Purse is chained at the first chance (the Draw Phase of turn 1); the
/// Main Phase's prompt then offers team 0 nothing from its hand (a Thunder Kid, a Dian Keto and Feral Imps), where the same
/// duel without the Purse offers them all. The lock ends with the turn: turn 3 offers the hand again.
#[test]
fn purse_with_a_hole_locks_the_opponents_hand_for_the_turn() {
    let purse = card("Purse with a Hole");
    let spawns = [set(1, purse), hand(0, THUNDER_KID), hand(0, DIAN_KETO)];
    // Without the Purse chained: the Main Phase offers the hand.
    let Some(mut s) = field(&spawns) else { return };
    s.run_to(&[], |p| p["type"] == 11 && p["player"] == 0).unwrap();
    assert!(playable(&s) > 0, "{:?}", s.prompt());
    // With it: nothing is offered in turn 1, and team 0 can only go to its End Phase.
    let Some(mut s) = field(&spawns) else { return };
    s.run_both([&[], &[Chain(purse)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", purse), 1, "{}", s.context());
    assert_eq!(playable(&s), 0, "the hand is locked: {:?}", s.prompt());
    assert_eq!(s.prompt()["to_ep"], true);
    // The hand holds 5 cards after the Purse; Dian Keto, if it is still there, is not played this turn.
    assert_eq!(s.state()[0]["hand"].as_array().unwrap().len(), 5, "{}", s.context());
    // Turn 3: the lock is over. (Dian Keto can have been among the discarded cards: only Feral Imps are counted.)
    s.run_to(&[EndTurn], |p| p["type"] == 11 && p["player"] == 0 && p["summons"].as_array().is_some_and(|v| !v.is_empty())).unwrap();
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Flame Swordsman
// ---------------------------------------------------------------------------------------------------------------

/// Flame Swordsman: a Fusion Monster in the game, a Normal Monster of the Main Deck in the anime (docs/anime-cards.md). A
/// scenario that lists it as an Extra Deck card gets it in the Main Deck with anime cards on, as the profile places it: the
/// engine's own Deck holds it (and its Extra Deck does not), so this is not a harness bug. With anime cards off it is in
/// the Extra Deck, as the game has it.
#[test]
fn flame_swordsman_is_a_main_deck_card_with_anime_cards_on() {
    let flame = card("Flame Swordsman");
    let Some(dir) = engine_dir() else { return };
    // The Decks after the opening hand and turn 1's draw: 41 cards (the Flame Swordsman among them) less 6, or 40 less 6.
    for (anime, deck, extra) in [(true, 35, 0), (false, 34, 1)] {
        let decks = [(&[FILLER; 40][..], &[flame][..]), (&[FILLER; 40][..], &[][..])];
        let s = if anime { Scenario::start_anime(&dir, 7, 8000, decks) } else { Scenario::start(&dir, 7, 8000, decks) }.unwrap();
        let count = |pile: &str| s.state()[0][pile].as_i64().or_else(|| s.state()[0][pile].as_array().map(|a| a.len() as i64)).unwrap_or(-1);
        assert_eq!((count("deck"), count("extra")), (deck, extra), "anime {anime}");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Load of Darkness
// ---------------------------------------------------------------------------------------------------------------

/// Infinite Fiend Mirror (a Field Spell): "Once per turn, during the End Phase, if the turn player controls "Load of
/// Darkness": they must Special Summon "Load of Darkness Tokens" equal to their empty Monster Zones. If a "Load of Darkness"
/// on the field is destroyed, destroy all "Load of Darkness Tokens"." Team 0 controls Load of Darkness (one of five zones)
/// in turn 1: its End Phase brings four Tokens to the client's field (the client models agree with the engine). Team 1's
/// Fissure (turn 2: it destroys the lowest ATK of team 0's face-up monsters, Load of Darkness) then takes every Token with it.
#[test]
fn infinite_fiend_mirror_makes_load_of_darkness_tokens_that_leave_with_it() {
    let [mirror, load, token] = ["Infinite Fiend Mirror", "Load of Darkness", "Load of Darkness Token"].map(card);
    let Some(mut s) = field(&[spell(0, mirror), mon(0, load), hand(1, FISSURE)]) else { return };
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 4, "a Token for each empty Monster Zone: {}", s.context());
    assert_eq!(s.count(0, "mzone", load), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, token), Some(3000), "{}", s.context());
    // Tokens leave the field when Load of Darkness is destroyed.
    s.run_both([&[], &[Activate(FISSURE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", load), 1, "Fissure took the lowest ATK, Load of Darkness: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 0, "the Tokens are gone: {}", s.context());
    assert_eq!(s.count(1, "grave", FISSURE), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Brilliant Shrine Art
// ---------------------------------------------------------------------------------------------------------------

const YAMI: u16 = 4341; // Field Spell: Fiend and Spellcaster monsters gain 200 ATK and DEF (what Ccapac Apu needs to stay)

/// Brilliant Shrine Art: "When an opponent's monster declares an attack: you can change the attack target to an "Earthbound
/// Immortal" monster you control." An Earthbound Immortal cannot be chosen as an attack target, so with ProjectIgnis' script
/// (`ChangeAttackTarget`) the core asked the attacker whether to attack again, and a computer attacker picked its old target
/// again: the redirect was undone. The override battles the attacker with the Immortal at once (the attack "becomes an
/// attack on that monster"), so the attacker is never asked: Team 1's Warwolf (2000) attacks team 0's Dark Blade (1800); the
/// Art sends the attack to Ccapac Apu, Dark Blade is not attacked (it is neither destroyed nor hurts team 0), and the
/// attacker meets the Immortal. The step's answers are the harness's computer answers, as for any CPU attacker.
#[test]
fn brilliant_shrine_art_sends_the_attack_to_the_immortal_and_a_cpu_attacker_cannot_undo_it() {
    let [art, ccapac] = ["Brilliant Shrine Art", "Earthbound Immortal Ccapac Apu"].map(card);
    let spawns = [set(0, art), mon(0, ccapac), mon(0, DARK_BLADE), spell(0, YAMI), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    s.run_both([&[Activate(art), Chain(art)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade was not attacked: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert!(!s.prompts.iter().any(|&(team, kind)| team == 1 && kind == msg::SELECT_YESNO), "the attacker was asked to attack again: {:?}", s.prompts);
    assert_eq!(s.count(0, "szone", art), 1, "{}", s.context());
    // The attack was a battle with Ccapac Apu (3200 with the Field Spell against Warwolf's 2000): the attacker is destroyed and
    // its controller takes the damage, where an attack that went nowhere would cost it nothing.
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert!(s.lp(1) < 8000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", ccapac), 1, "{}", s.context());
}
