//! ∞ is one value (`crates/yo3-engine/src/guards.lua`): any LP, ATK or DEF of 9,999,999 or more is exactly 19,999,998,
//! which a finite cost, damage, gain or ATK change leaves as it is, and an amount that is itself ∞ moves as is. The cards
//! of the game that set, pay, give or take LP, and those that change an ∞ ATK, through the real core (anime cards on),
//! with both seats' client views checked against the engine after every step.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const INFINITE: i64 = 19_999_998;
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf, 2000 ATK
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK
const RA: u16 = 5000; // the manga Ra: revived from the Graveyard, pay all but 1 LP and gain it as ATK
const REBORN: u16 = 4842;
const DE_FUSION: u16 = 5118; // on that Ra: its ATK back as LP
const PREMATURE_BURIAL: u16 = 4966; // pay 800
const DIAN_KETO: u16 = 4348; // gain 1000
const SOLEMN: u16 = 4861; // pay half your LP
const YU_JO: u16 = 5754; // both players' LP become half of their total
const REVERSAL_QUIZ: u16 = 5541; // swap LP
const SELF_DESTRUCT: u16 = 5916; // both LP become 0
const LIFE_EQUALIZER: u16 = 6708; // both LP become 3000
const WALL: u16 = 5990; // Wall of Revealing Light: pay a multiple of 1000
const HOPE: u16 = 9354; // Hope for Escape: draw 1 per 2000 LP behind
const RIRYOKU: u16 = 5009; // the anime one: halve the opponent's LP, a monster gains that much
const RASCA: u16 = 8461; // the anime Earthbound Immortal Wiraqocha Rasca: the opponent's LP become 1
const YAMI: u16 = 4341;
const GRANEL: u16 = 9265; // the anime Meklord Emperor Granel ∞: its original ATK is its controller's LP
const SHADOW_SPELL: u16 = 4675; // -700 ATK
const AXE_OF_DESPAIR: u16 = 4310; // +1000 ATK
const MEGAMORPH: u16 = 4663; // with more LP than the opponent: half the original ATK
// Added cards (engine/added/cards.tsv) that pay, set or compare LP:
const CYBERNETIC_FUSION_SUPPORT: u16 = 10982; // pay half your LP
const DARK_ELEMENT: u16 = 5049; // pay half your LP: a Gate Guardian from the hand
const GATE_GUARDIAN: u16 = 4380;
const SOUL_CHARGE: u16 = 11159; // lose 1000 LP per monster revived
const MULTIPLE_DESTRUCTION: u16 = 5481; // lose 300 LP per card returned
const MICHION: u16 = 13815; // Michion, the Timelord: after battling, halve the opponent's LP
const HAILON: u16 = 13816; // Hailon, the Timelord: after battling, with less LP, inflict the difference
const LIFE_STREAM: u16 = 9509; // Life Stream Dragon: when Synchro Summoned, your LP can become 4000
const POWER_TOOL: u16 = 8137;
const TURBO_SYNCHRON: u16 = 7870; // a Level 1 Tuner
const JURASSIC_IMPACT: u16 = 13156; // with less LP: destroy every monster, 1000 damage each to both players
const SABERSAURUS: u16 = 6796;

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }

/// Team 0 at ∞ LP (Mystik Wok on the anime Geh: 8000 and ∞ is ∞), with `spawns` in place; team 1 at 8000.
fn infinite(spawns: &[Spawn]) -> Option<Scenario> {
    infinite_extra(&[], spawns)
}

/// [`infinite`] with team 0's Extra Deck.
fn infinite_extra(extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(0, WOK), mon(0, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[KURIBOH; 40][..], extra), (&[KURIBOH; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &field).expect("the duel starts");
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[]);
    Some(s)
}

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:?}", s.problems);
}

#[test]
fn finite_costs_and_gains_leave_infinite_lp_as_it_is() {
    let Some(mut s) = infinite(&[hand(0, DIAN_KETO), hand(0, PREMATURE_BURIAL), Spawn { team: 0, place: Grave, cid: WARWOLF }]) else { return };
    s.run(&[Activate(DIAN_KETO), Activate(PREMATURE_BURIAL)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Premature Burial still revived Warwolf: {}", s.context());
    clean(&s);
}

#[test]
fn half_of_infinite_lp_is_infinite() {
    // Solemn Judgment's cost is half the LP: from ∞, ∞ is left; the summon is still negated.
    let Some(mut s) = infinite(&[set(0, SOLEMN), hand(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Chain(SOLEMN)], &[Summon(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    assert_eq!((s.count(1, "mzone", WARWOLF), s.count(1, "grave", WARWOLF)), (0, 1), "{}", s.context());
    clean(&s);
}

#[test]
fn riryoku_on_infinite_lp_gives_a_true_infinity() {
    // The anime Riryoku halves the opponent's LP and gives a monster that much: half of ∞ is ∞, so the LP stay ∞ and
    // Warwolf's ATK is ∞, not a finite 5 million shown as ∞.
    let Some(mut s) = infinite(&[set(1, RIRYOKU), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Activate(RIRYOKU)]]).unwrap();
    assert_eq!((s.lp(0), s.attack_of(1, WARWOLF)), (INFINITE, Some(INFINITE)), "{}", s.context());
    clean(&s);
}

#[test]
fn yu_jo_friendship_gives_both_players_infinity() {
    let Some(mut s) = infinite(&[hand(0, YU_JO)]) else { return };
    s.run(&[Activate(YU_JO)]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, INFINITE), "{}", s.context());
    clean(&s);
}

#[test]
fn ra_takes_infinite_lp_but_1_and_de_fusion_gives_it_back() {
    let Some(mut s) = infinite(&[Spawn { team: 0, place: Grave, cid: RA }, hand(0, REBORN), hand(0, DE_FUSION)]) else { return };
    s.prefer(&[(0, RA)]);
    s.run(&[Activate(REBORN)]).unwrap();
    assert_eq!((s.lp(0), s.attack_of(0, RA)), (1, Some(INFINITE)), "Ra pays all but 1: {}", s.context());
    s.run(&[Activate(DE_FUSION)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "De-Fusion: {}", s.context());
    clean(&s);
}

#[test]
fn an_atk_that_is_infinite_lp_is_infinite() {
    let Some(mut s) = infinite(&[mon(0, GRANEL)]) else { return };
    s.run(&[EndTurn]).unwrap();
    assert_eq!(s.attack_of(0, GRANEL), Some(INFINITE), "{}", s.context());
    clean(&s);
}

#[test]
fn finite_atk_changes_leave_an_infinite_atk_as_it_is() {
    // Granel's ATK is ∞ (its controller's ∞ LP): Axe of Despair's +1000, Megamorph's half of the original ATK and Shadow
    // Spell's -700 all leave it ∞.
    let Some(mut s) = infinite(&[mon(0, GRANEL), hand(0, AXE_OF_DESPAIR), hand(0, MEGAMORPH), set(1, SHADOW_SPELL)]) else { return };
    s.prefer(&[(0, GRANEL)]);
    s.run(&[Activate(AXE_OF_DESPAIR)]).unwrap();
    assert_eq!(s.attack_of(0, GRANEL), Some(INFINITE), "Axe of Despair: {}", s.context());
    s.run(&[Activate(MEGAMORPH)]).unwrap();
    assert_eq!(s.attack_of(0, GRANEL), Some(INFINITE), "Megamorph: {}", s.context());
    s.run_both([&[EndTurn], &[Activate(SHADOW_SPELL)]]).unwrap();
    assert_eq!(s.attack_of(0, GRANEL), Some(INFINITE), "Shadow Spell: {}", s.context());
    clean(&s);
}

#[test]
fn infinity_against_infinity_is_a_tie() {
    // Granel (∞, its controller's LP) attacks the Warwolf Riryoku made ∞: equal ATK, both destroyed, no damage.
    let Some(mut s) = infinite(&[mon(0, GRANEL), set(1, RIRYOKU), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Attack(GRANEL)], &[Activate(RIRYOKU)]]).unwrap();
    assert_eq!((s.count(0, "grave", GRANEL), s.count(1, "grave", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, 8000), "{}", s.context());
    clean(&s);
}

#[test]
fn lp_that_becomes_a_number_is_that_number() {
    let Some(mut s) = infinite(&[set(0, LIFE_EQUALIZER)]) else { return };
    s.run(&[Activate(LIFE_EQUALIZER)]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (3000, 3000), "Life Equalizer: {}", s.context());
    clean(&s);
    let Some(mut s) = infinite(&[hand(0, REVERSAL_QUIZ)]) else { return };
    s.run(&[Activate(REVERSAL_QUIZ)]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000, INFINITE), "Reversal Quiz swaps: {}", s.context());
    clean(&s);
    let Some(mut s) = infinite(&[set(1, SELF_DESTRUCT)]) else { return };
    let r = s.run_both([&[EndTurn], &[Activate(SELF_DESTRUCT)]]);
    assert!(r.is_err_and(|e| e.contains("the duel is over")), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (0, 0), "Self-Destruct Button: {}", s.context());
    clean(&s);
}

#[test]
fn rasca_takes_infinite_lp_to_1() {
    // Summoned on its owner's turn with a Field Spell up (without one, an Earthbound Immortal is destroyed in the End Phase).
    let Some(mut s) = infinite(&[hand(1, YAMI), hand(1, RASCA), mon(1, WARWOLF), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Activate(YAMI), Summon(RASCA), Activate(RASCA)]]).unwrap();
    assert_eq!(s.lp(0), 1, "{}", s.context());
    clean(&s);
}

#[test]
fn hope_for_escape_against_infinity_draws_the_deck_out() {
    // 1 card per 2000 LP behind ∞: more than the Deck holds, so its player loses at once.
    let Some(mut s) = infinite(&[set(1, HOPE)]) else { return };
    let r = s.run_both([&[EndTurn], &[Activate(HOPE)]]);
    assert!(r.is_err_and(|e| e.contains("the duel is over")), "{}", s.context());
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    clean(&s);
}

#[test]
fn a_long_number_declaration_keeps_255_choices_and_the_largest() {
    // Wall of Revealing Light at ∞ LP lists every multiple of 1000 up to the LP: 19,999 choices, which the core sends
    // with a one-byte count. 255 reach the player: the first 200 and 55 more up to the largest. The finite amount paid
    // leaves ∞ LP as it is.
    let Some(mut s) = infinite(&[set(0, WALL)]) else { return };
    s.run_to(&[Activate(WALL)], |p| p["type"] == 143).unwrap();
    let options: Vec<i64> = s.prompt()["options"].as_array().unwrap().iter().filter_map(|o| o.as_i64()).collect();
    assert_eq!((options.len(), options.first(), options.get(199), options.last()), (255, Some(&1000), Some(&200_000), Some(&19_999_000)));
    assert!(options.windows(2).all(|w| w[0] < w[1]), "in order: {options:?}");
    s.run(&[]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    clean(&s);
}

/// Both players at ∞ from the start, anime versions off: the added printed cards as printed (with anime cards on,
/// several play their anime versions, tested below where they differ).
fn both_infinite(extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[KURIBOH; 40][..], extra), (&[KURIBOH; 40][..], &[][..])];
    let s = Scenario::start_field(&dir, 7, 20_000_000, decks, spawns).expect("the duel starts");
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, INFINITE), "{}", s.context());
    Some(s)
}

#[test]
fn half_of_infinite_lp_as_a_cost_leaves_it() {
    // Cybernetic Fusion Support and Dark Element pay half the LP: half of ∞ is ∞.
    let Some(mut s) = both_infinite(&[], &[hand(0, CYBERNETIC_FUSION_SUPPORT)]) else { return };
    s.run(&[Activate(CYBERNETIC_FUSION_SUPPORT)]).unwrap();
    assert_eq!((s.lp(0), s.count(0, "grave", CYBERNETIC_FUSION_SUPPORT)), (INFINITE, 1), "{}", s.context());
    clean(&s);
    let Some(mut s) = both_infinite(&[], &[hand(0, DARK_ELEMENT), hand(0, GATE_GUARDIAN), Spawn { team: 0, place: Grave, cid: GATE_GUARDIAN }]) else { return };
    s.prefer(&[(0, GATE_GUARDIAN)]);
    s.run(&[Activate(DARK_ELEMENT)]).unwrap();
    assert_eq!((s.lp(0), s.count(0, "mzone", GATE_GUARDIAN)), (INFINITE, 1), "Gate Guardian summoned: {}", s.context());
    clean(&s);
}

#[test]
fn lp_lost_per_card_leaves_infinite_lp() {
    // Soul Charge: 1000 per monster revived; Multiple Destruction: 300 per card returned. Both finite: ∞ stays.
    let Some(mut s) = both_infinite(&[], &[hand(0, SOUL_CHARGE), Spawn { team: 0, place: Grave, cid: WARWOLF }]) else { return };
    s.run(&[Activate(SOUL_CHARGE)]).unwrap();
    assert_eq!((s.lp(0), s.count(0, "mzone", WARWOLF)), (INFINITE, 1), "{}", s.context());
    clean(&s);
    let Some(mut s) = both_infinite(&[], &[set(0, MULTIPLE_DESTRUCTION)]) else { return };
    s.run_both([&[EndTurn, Activate(MULTIPLE_DESTRUCTION)], &[EndTurn]]).unwrap();
    assert_eq!((s.lp(0), s.count(0, "grave", MULTIPLE_DESTRUCTION)), (INFINITE, 1), "{}", s.context());
    clean(&s);
}

#[test]
fn michion_halves_infinite_lp_to_infinity() {
    // The printed Michion halves the opponent's LP: half of ∞ is ∞.
    let Some(mut s) = both_infinite(&[], &[hand(1, MICHION), mon(0, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Summon(MICHION), Attack(MICHION), EndTurn]]).unwrap();
    assert_eq!((s.lp(0), s.count(1, "mzone", MICHION)), (INFINITE, 1), "{}", s.context());
    clean(&s);
}

#[test]
fn the_anime_timelords_against_infinite_lp_inflict_infinity() {
    // The anime Michion inflicts damage equal to half the opponent's LP, the anime Hailon the difference between the LP
    // (8000 against ∞): each amount is itself ∞, which takes every LP.
    for timelord in [MICHION, HAILON] {
        // Summoned with two Tributes (the anime ones have no procedure of their own; a spawned one shuffles itself back
        // into the Deck at its controller's first Standby Phase).
        let Some(mut s) = infinite(&[hand(1, timelord), mon(1, WARWOLF), mon(1, WARWOLF)]) else { return };
        let r = s.run_both([&[EndTurn], &[Summon(timelord), Attack(timelord), EndTurn]]);
        assert!(r.is_err_and(|e| e.contains("the duel is over")), "{timelord}: {}", s.context());
        assert_eq!(s.lp(0), 0, "{timelord}: {}", s.context());
        clean(&s);
    }
}

#[test]
fn hailon_against_infinite_lp_inflicts_infinity() {
    // The printed Hailon: with less LP than the opponent, the difference as damage. Both at ∞: no difference, nothing.
    let Some(mut s) = both_infinite(&[], &[hand(1, HAILON), mon(0, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Summon(HAILON), Attack(HAILON), EndTurn]]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, INFINITE), "{}", s.context());
    clean(&s);
}

#[test]
fn life_stream_dragon_makes_infinite_lp_4000() {
    // "You can make your LP become 4000": LP that becomes a number is that number, from ∞ too.
    let Some(mut s) = both_infinite(&[LIFE_STREAM, POWER_TOOL], &[mon(0, POWER_TOOL), mon(0, TURBO_SYNCHRON)]) else { return };
    s.run(&[SpSummon(LIFE_STREAM), Chain(LIFE_STREAM)]).unwrap();
    assert_eq!((s.lp(0), s.count(0, "mzone", LIFE_STREAM)), (4000, 1), "{}", s.context());
    clean(&s);
}

#[test]
fn jurassic_impact_against_infinite_lp_leaves_it() {
    // Team 1 (8000, less than ∞) destroys every monster and each player takes 1000 per monster it had: finite, so ∞
    // stays (anime cards on: the anime version).
    let Some(mut s) = infinite(&[set(1, JURASSIC_IMPACT), mon(1, SABERSAURUS), mon(1, SABERSAURUS)]) else { return };
    assert_eq!(s.count(1, "szone", JURASSIC_IMPACT), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[Activate(JURASSIC_IMPACT)]]).unwrap();
    assert_eq!((s.lp(0), s.lp(1), s.count(1, "grave", SABERSAURUS)), (INFINITE, 6000, 2), "{}", s.context());
    clean(&s);
}
