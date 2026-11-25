//! Group 04 of the anime's printed cards (rows 4848-5205): Dark Dragon Ritual to Double Magical Arm Bind. Each effect
//! through the real core with both seats' client models checked after every step (`duelcheck::scenario`). The printed
//! text plays with anime cards off; a card whose anime version differs gets one more test with them on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The group's cards (client ids: the Konami id, or a stand-in).
const DARK_DRAGON_RITUAL: u16 = 11897;
const DARK_ELEMENT: u16 = 5049;
const DARK_GUARDIAN: u16 = 5047;
const DARK_RENEWAL: u16 = 4822;
const DARK_SANCTUARY: u16 = 12572;
const DARK_SPELL_REGENERATION: u16 = 5638;
const DARK_SUMMONING_BEAST: u16 = 12903;
const CATASTROGUE: u16 = 5154; // Dark Tuner Catastrogue (Level 8)
const NIGHTMARE_HAND: u16 = 5155; // Dark Tuner Nightmare Hand (Level 10)
const DARK_TYRANNO: u16 = 5159;
const DMG_DRAGON_KNIGHT: u16 = 11151; // Dark Magician Girl the Dragon Knight
// The game's own.
const RED_EYES_SPELL: u16 = 11895; // Red-Eyes Transmigration, a "Red-Eyes" Spell
const POLYMERIZATION: u16 = 4837;
const DARK_MAGICIAN: u16 = 4041;
const DARK_MAGICIAN_GIRL: u16 = 4766;
const SKILLED_DARK_MAGICIAN: u16 = 5649; // Level 4 DARK Spellcaster
const URIA: u16 = 6563;
const HAMON: u16 = 6564;
const RAVIEL: u16 = 6565;
const RA: u16 = 5000;
const ULTIMATE_TYRANNO: u16 = 6777; // Level 8 Dinosaur
const DARK_DIVINER: u16 = 9217; // a Level 4 Synchro Monster
const GOYO_GUARDIAN: u16 = 7736; // a Level 6 Synchro Monster
const DESTINY_HERO_DEFENDER: u16 = 6755;
const DESTINY_HERO_DIAMOND_DUDE: u16 = 6660;
const DESTINY_HERO_MALICIOUS: u16 = 6949;
const CELESTIAL: u16 = 12707; // Destiny HERO - Celestial
const DARK_ANGEL: u16 = 12706; // Destiny HERO - Dark Angel
const DIABOUND_KERNEL: u16 = 12899;
const DICE_DUNGEON: u16 = 4697;
const DIMENSION_DICE: u16 = 4699;
const DOBLE_PASSE: u16 = 11166;
const DOCTOR_D: u16 = 4003;
const DOLL_HAMMER: u16 = 5171;
const SANGA: u16 = 4377; // Sanga of the Thunder
const KAZEJIN: u16 = 4378;
const SUIJIN: u16 = 4379;
const GATE_GUARDIAN: u16 = 4380;
const OBELISK: u16 = 4998;
const BLUE_EYES: u16 = 4007;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn duel(anime: bool, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// Forty Kuribohs and these cards after them: a Deck holding the cards a test searches for.
fn deck_of(cards: &[u16]) -> Vec<u16> {
    [KURIBOH; 30].into_iter().chain(cards.iter().copied()).collect()
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// Whether the idle prompt on offer lists this card's effect.
fn can_activate(s: &Scenario, cid: u16) -> bool {
    s.prompt()["activates"].as_array().is_some_and(|a| a.iter().any(|c| c["cid"] == cid))
}

fn deck(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap_or(-1)
}

/// Dark Element: "If you have a "Gate Guardian" monster in your GY: Pay half your LP; Special Summon 1 Level 11 or higher
/// "Gate Guardian" monster from your hand, Deck, or Extra Deck, ignoring its Summoning conditions." Dark Guardian
/// (always a "Gate Guardian", Level 12) comes from the Deck for half the LP, and the Element goes to the Graveyard.
#[test]
fn dark_element_summons_a_level_11_gate_guardian() {
    let d = deck_of(&[DARK_GUARDIAN]);
    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_ELEMENT }, Spawn { team: 0, place: Grave, cid: GATE_GUARDIAN }];
    let Some(mut s) = duel(false, [(&d, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DARK_ELEMENT)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "half the LP: {}", s.context());
    assert_eq!(s.attack_of(0, DARK_GUARDIAN), Some(3800), "Dark Guardian came: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_ELEMENT), 1, "{}", s.context());
}

/// Dark Element needs a "Gate Guardian" in the Graveyard: without one it can't be activated.
#[test]
fn dark_element_needs_a_gate_guardian_in_the_graveyard() {
    let d = deck_of(&[DARK_GUARDIAN]);
    let Some(mut s) = duel(false, [(&d, &[]), (&[KURIBOH; 40], &[])], &[Spawn { team: 0, place: Hand, cid: DARK_ELEMENT }]) else { return };
    assert!(s.run(&[Activate(DARK_ELEMENT)]).is_err(), "no Gate Guardian in the Graveyard: {}", s.context());
}

/// Dark Element: "You can banish this card from your GY; add 1 of your "Sanga of the Thunder", "Kazejin", or "Suijin" from
/// your Deck or banishment to your hand." Kazejin comes from the Deck, Suijin from the banishment.
#[test]
fn dark_element_in_the_graveyard_adds_a_guardian_from_the_deck_or_banishment() {
    let Some(mut s) = duel(false, [(&[KAZEJIN; 40], &[]), (&[KURIBOH; 40], &[])], &[Spawn { team: 0, place: Grave, cid: DARK_ELEMENT }]) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(DARK_ELEMENT)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before + 1, "Kazejin from the Deck: {}", s.context());
    assert_eq!(s.count(0, "removed", DARK_ELEMENT), 1, "the Element was banished as the cost: {}", s.context());

    let spawns = [Spawn { team: 0, place: Grave, cid: DARK_ELEMENT }, Spawn { team: 0, place: Banished, cid: SUIJIN }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DARK_ELEMENT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", SUIJIN), 1, "Suijin from the banishment: {}", s.context());
}

/// Dark Guardian: "Special Summoned (from your hand or GY) by shuffling into the Deck, 1 each of your "Sanga of the
/// Thunder", "Kazejin", and "Suijin" that is in your hand, face-up field, GY, and/or banished." One from the hand, one
/// from the Graveyard, one face-up banished.
#[test]
fn dark_guardian_is_summoned_by_shuffling_the_three_guardians() {
    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_GUARDIAN }, Spawn { team: 0, place: Hand, cid: SANGA },
        Spawn { team: 0, place: Grave, cid: KAZEJIN }, Spawn { team: 0, place: Banished, cid: SUIJIN }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    let before = deck(&s, 0);
    s.run(&[SpSummon(DARK_GUARDIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_GUARDIAN), Some(3800), "{}", s.context());
    assert_eq!(s.count(0, "hand", SANGA) + s.count(0, "grave", KAZEJIN) + s.count(0, "removed", SUIJIN), 0, "{}", s.context());
    assert_eq!(deck(&s, 0), before + 3, "the three went into the Deck: {}", s.context());
}

/// Dark Guardian is not Summoned with only two of the three.
#[test]
fn dark_guardian_needs_all_three_guardians() {
    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_GUARDIAN }, Spawn { team: 0, place: Hand, cid: SANGA },
        Spawn { team: 0, place: Grave, cid: KAZEJIN }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    assert!(s.run(&[SpSummon(DARK_GUARDIAN)]).is_err(), "two guardians are not enough: {}", s.context());
}

/// Dark Guardian: "Cannot be destroyed by battle." It attacks Obelisk the Tormentor (4000 ATK against its 3800), stays on
/// the field and its controller takes 200.
#[test]
fn dark_guardian_is_not_destroyed_by_battle() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DARK_GUARDIAN }, Spawn { team: 1, place: Monster, cid: OBELISK }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Attack(DARK_GUARDIAN)]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_GUARDIAN).is_some(), "Dark Guardian survived: {}", s.context());
    assert_eq!(s.lp(0), 7800, "the 200 difference: {}", s.context());
}

/// Dark Guardian: "If this card is Special Summoned by the effect of "Dark Element", it is unaffected by other monsters'
/// effects or your opponent's activated Spell Cards or effects." Dark Hole destroys the one summoned by its own
/// procedure and not the one Dark Element brought.
#[test]
fn dark_guardian_from_dark_element_is_unaffected_by_spells() {
    let d = deck_of(&[DARK_GUARDIAN]);
    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_ELEMENT }, Spawn { team: 0, place: Grave, cid: GATE_GUARDIAN },
        Spawn { team: 1, place: Hand, cid: DARK_HOLE }];
    let Some(mut s) = duel(false, [(&d, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run_both([&[Activate(DARK_ELEMENT), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_GUARDIAN).is_some(), "the one Dark Element brought is unaffected: {}", s.context());

    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_GUARDIAN }, Spawn { team: 0, place: Hand, cid: SANGA },
        Spawn { team: 0, place: Hand, cid: KAZEJIN }, Spawn { team: 0, place: Hand, cid: SUIJIN },
        Spawn { team: 1, place: Hand, cid: DARK_HOLE }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run_both([&[SpSummon(DARK_GUARDIAN), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_GUARDIAN).is_none(), "the one summoned by shuffling is destroyed: {}", s.context());
}

/// Anime Dark Element: "Activate only if Gate Guardian is in your Graveyard (and no Summon this turn): pay half your LP;
/// Special Summon Dark Guardian from your Deck." Anime Dark Guardian is Special Summoned by Dark Element only.
#[test]
fn anime_dark_element_summons_dark_guardian_from_the_deck() {
    let d = deck_of(&[DARK_GUARDIAN]);
    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_ELEMENT }, Spawn { team: 0, place: Grave, cid: GATE_GUARDIAN }];
    let Some(mut s) = duel(true, [(&d, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DARK_ELEMENT)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "half the LP: {}", s.context());
    assert!(s.monster(0, DARK_GUARDIAN).is_some(), "Dark Guardian came from the Deck: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------

/// Dark Dragon Ritual: "During your Main Phase, except the turn this card was sent to the GY: You can banish this card
/// from your GY; add 1 "Red-Eyes" Spell/Trap from your Deck to your hand."
#[test]
fn dark_dragon_ritual_in_the_graveyard_adds_a_red_eyes_spell() {
    let d = [RED_EYES_SPELL; 40];
    let Some(mut s) = duel(false, [(&d, &[]), (&[KURIBOH; 40], &[])], &[Spawn { team: 0, place: Grave, cid: DARK_DRAGON_RITUAL }]) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(DARK_DRAGON_RITUAL)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before + 1, "a Red-Eyes Spell came to the hand: {}", s.context());
    assert_eq!(s.count(0, "removed", DARK_DRAGON_RITUAL), 1, "banished as the cost: {}", s.context());
}

/// Dark Dragon Ritual's second effect: "except the turn this card was sent to the GY". Discarded as the cost of Dark
/// Magician Girl the Dragon Knight's Quick Effect it can't be used that turn, and can on the next.
#[test]
fn dark_dragon_ritual_in_the_graveyard_waits_a_turn() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DMG_DRAGON_KNIGHT }, Spawn { team: 0, place: Hand, cid: DARK_DRAGON_RITUAL }];
    let d = [RED_EYES_SPELL; 40];
    let Some(mut s) = duel(false, [(&d, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, DARK_DRAGON_RITUAL)]);
    s.run(&[Activate(DMG_DRAGON_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_DRAGON_RITUAL), 1, "discarded this turn: {}", s.context());
    assert!(!can_activate(&s, DARK_DRAGON_RITUAL), "not the turn it was sent: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(can_activate(&s, DARK_DRAGON_RITUAL), "the next turn: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------

/// Dark Magician Girl the Dragon Knight: ""Dark Magician Girl" + 1 Dragon monster. Must be Fusion Summoned with the above
/// Fusion Materials". Polymerization with Dark Magician Girl and Blue-Eyes White Dragon in the hand.
#[test]
fn dmg_the_dragon_knight_is_fusion_summoned_from_dmg_and_a_dragon() {
    let spawns = [Spawn { team: 0, place: Hand, cid: POLYMERIZATION }, Spawn { team: 0, place: Hand, cid: DARK_MAGICIAN_GIRL },
        Spawn { team: 0, place: Hand, cid: BLUE_EYES }];
    let Some(mut s) = duel(false, [(&[DARK_HOLE; 40], &[DMG_DRAGON_KNIGHT]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.run(&[Activate(POLYMERIZATION)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DMG_DRAGON_KNIGHT), Some(2600), "{}", s.context());
    assert_eq!((s.count(0, "grave", DARK_MAGICIAN_GIRL), s.count(0, "grave", BLUE_EYES)), (1, 1), "the materials: {}", s.context());
}

/// Dark Magician Girl the Dragon Knight: "Once per turn (Quick Effect): You can send 1 card from your hand to the GY, then
/// target 1 face-up card on the field; destroy that target." Warwolf is destroyed, one card leaves the hand.
#[test]
fn dmg_the_dragon_knight_discards_a_card_to_destroy_a_target() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DMG_DRAGON_KNIGHT }, Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[DARK_HOLE; 40], &[]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    let before = hand(&s, 0);
    s.run(&[Activate(DMG_DRAGON_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(hand(&s, 0), before - 1, "one card was sent from the hand: {}", s.context());
    assert!(s.monster(0, DMG_DRAGON_KNIGHT).is_some(), "{}", s.context());
}

/// Anime Dark Magician Girl the Dragon Knight: "When this card battles: discard 1 card; destroy the monster it battles."
/// (an optional trigger at the start of the battle): it attacks Warwolf (2000 against 2600) and destroys it before damage.
#[test]
fn anime_dmg_the_dragon_knight_destroys_the_monster_it_battles() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DMG_DRAGON_KNIGHT }, Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(true, [(&[DARK_HOLE; 40], &[]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    // (the first player can't battle on the first turn: the attack comes on the third)
    s.run(&[Attack(DMG_DRAGON_KNIGHT), Chain(DMG_DRAGON_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed at the start of the battle: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "the discard: {}", s.context());
}

/// Dark Renewal: "When your opponent Normal or Special Summons a monster(s): Target 1 of those monsters and 1 Spellcaster
/// monster you control; send both monsters to the GY, then you can Special Summon 1 DARK Spellcaster monster from your Deck
/// or GY." Warwolf and Dark Magician go to the Graveyard, Skilled Dark Magician comes from the Deck.
#[test]
fn dark_renewal_sends_both_and_summons_a_dark_spellcaster() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DARK_RENEWAL }, Spawn { team: 0, place: Monster, cid: DARK_MAGICIAN },
        Spawn { team: 1, place: Hand, cid: WARWOLF }];
    let d = deck_of(&[SKILLED_DARK_MAGICIAN]);
    let Some(mut s) = duel(false, [(&d, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, SKILLED_DARK_MAGICIAN)]);
    s.run_both([&[EndTurn, Chain(DARK_RENEWAL)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(0, "grave", DARK_MAGICIAN)), (1, 1), "both sent: {}", s.context());
    assert_eq!(s.attack_of(0, SKILLED_DARK_MAGICIAN), Some(1900), "the DARK Spellcaster came: {}", s.context());
}

/// Anime Dark Renewal: "When the opponent Summons a monster: Send it and 1 monster you control to the Graveyard; Special
/// Summon 1 Spellcaster from your Graveyard." Dark Magician, sent with Warwolf, is the Spellcaster that comes back.
#[test]
fn anime_dark_renewal_brings_a_spellcaster_back_from_the_graveyard() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DARK_RENEWAL }, Spawn { team: 0, place: Monster, cid: DARK_MAGICIAN },
        Spawn { team: 0, place: Grave, cid: DARK_MAGICIAN_GIRL }, Spawn { team: 1, place: Hand, cid: WARWOLF }];
    let Some(mut s) = duel(true, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, DARK_MAGICIAN_GIRL)]);
    s.run_both([&[EndTurn, Chain(DARK_RENEWAL)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the summoned monster went to the Graveyard: {}", s.context());
    assert!(s.monster(0, DARK_MAGICIAN_GIRL).is_some(), "a Spellcaster came from the Graveyard: {}", s.context());
}

/// Dark Sanctuary: "When an opponent's monster declares an attack: Toss a coin and if the result is heads, negate the
/// attack, and if you do, inflict damage to your opponent equal to half the current ATK of that opponent's monster." The
/// coin is the duel's seed's: over several seeds both sides turn up, heads negating Warwolf's attack and dealing 1000 to
/// its controller, tails letting the 2000 through.
#[test]
fn dark_sanctuary_tosses_a_coin_to_negate_an_attack() {
    let dir = engine_dir();
    let Some(dir) = dir else { return };
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=24 {
        let spawns = [Spawn { team: 0, place: Hand, cid: DARK_SANCTUARY }, Spawn { team: 1, place: Monster, cid: WARWOLF }];
        let mut s = Scenario::start_field(&dir, seed, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns).unwrap();
        s.run_both([&[Activate(DARK_SANCTUARY), EndTurn], &[Attack(WARWOLF)]]).unwrap();
        clean(&s);
        match (s.lp(0), s.lp(1)) {
            (8000, 7000) => heads += 1,
            (6000, 8000) => tails += 1,
            other => panic!("seed {seed}: neither heads (8000, 7000) nor tails (6000, 8000): {other:?}\n{}", s.context()),
        }
    }
    assert!(heads > 0 && tails > 0, "both sides of the coin turned up over 24 seeds: {heads} heads, {tails} tails");
}

/// Dark Spell Regeneration: "When an opponent's monster declares an attack: Target 1 Spell in your opponent's GY; Set it to
/// your field." Dark Hole in the opponent's Graveyard is Set to our field.
#[test]
fn dark_spell_regeneration_sets_a_spell_from_the_opponents_graveyard() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DARK_SPELL_REGENERATION }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Grave, cid: DARK_HOLE }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(DARK_SPELL_REGENERATION)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", DARK_HOLE), 1, "Dark Hole was Set to our field: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 0, "{}", s.context());
}

/// Dark Spell Regeneration: "You can banish this card from your GY and send 1 "Monster Reborn" from your hand or Set on
/// your field to the GY; Special Summon 1 "The Winged Dragon of Ra" from your GY ignoring its Summoning conditions, then
/// you can send 1 monster your opponent controls to the GY, also send that Special Summoned monster to the GY during the
/// End Phase."
#[test]
fn dark_spell_regeneration_summons_ra_from_the_graveyard() {
    let spawns = [Spawn { team: 0, place: Grave, cid: DARK_SPELL_REGENERATION }, Spawn { team: 0, place: Hand, cid: MONSTER_REBORN },
        Spawn { team: 0, place: Grave, cid: RA }, Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DARK_SPELL_REGENERATION)]).unwrap();
    clean(&s);
    assert!(s.monster(0, RA).is_some(), "Ra came from the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "grave", MONSTER_REBORN), 1, "Monster Reborn was the cost: {}", s.context());
    assert_eq!(s.count(0, "removed", DARK_SPELL_REGENERATION), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the opponent's monster went to the Graveyard: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert!(s.monster(0, RA).is_none() && s.count(0, "grave", RA) == 1, "Ra went back in the End Phase: {}", s.context());
}

/// Anime Dark Spell Regeneration: "When the opponent's monster declares an attack: Target 1 Spell Card in either
/// Graveyard; activate its effect, then send it to its owner's Graveyard." Dark Hole in the opponent's Graveyard destroys
/// the attacker.
#[test]
fn anime_dark_spell_regeneration_activates_a_spell_from_a_graveyard() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DARK_SPELL_REGENERATION }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Grave, cid: DARK_HOLE }];
    let Some(mut s) = duel(true, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(DARK_SPELL_REGENERATION)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole's effect destroyed the attacker: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "and it went back to its owner's Graveyard: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Dark Summoning Beast: "You can Tribute this card; Special Summon 1 "Uria, Lord of Searing Flames", "Hamon, Lord of
/// Striking Thunder", or "Raviel, Lord of Phantasms" from your hand or Deck, ignoring its Summoning conditions, also
/// monsters you control cannot attack for the rest of this turn." In the second player's turn, so that the Battle Phase
/// is open: Raviel comes from the Deck, and Warwolf can't attack.
#[test]
fn dark_summoning_beast_tributes_itself_for_a_sacred_beast() {
    let spawns = [Spawn { team: 1, place: Monster, cid: DARK_SUMMONING_BEAST }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: Monster, cid: KURIBOH }];
    let d = deck_of(&[RAVIEL]);
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&d, &[])], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_SUMMONING_BEAST)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, RAVIEL), Some(4000), "Raviel came from the Deck: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_SUMMONING_BEAST), 1, "the Beast was the Tribute: {}", s.context());
    let err = s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap_err();
    assert!(err.contains("can't attack"), "monsters you control cannot attack this turn: {err}");
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Dark Summoning Beast: "You can banish this card from your GY; add 1 "Uria", "Hamon", or "Raviel" from your Deck to your
/// hand."
#[test]
fn dark_summoning_beast_in_the_graveyard_adds_a_sacred_beast() {
    let Some(mut s) = duel(false, [(&[HAMON; 40], &[]), (&[KURIBOH; 40], &[])], &[Spawn { team: 0, place: Grave, cid: DARK_SUMMONING_BEAST }]) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(DARK_SUMMONING_BEAST)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before + 1, "Hamon came to the hand: {}", s.context());
    assert_eq!(s.count(0, "removed", DARK_SUMMONING_BEAST), 1, "{}", s.context());
}

/// Anime Dark Summoning Beast: "Tribute this card; Special Summon "Uria", "Hamon" and "Raviel" from your Graveyard,
/// ignoring their Summoning conditions. You cannot enter the Battle Phase this turn."
#[test]
fn anime_dark_summoning_beast_brings_all_three_sacred_beasts() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DARK_SUMMONING_BEAST }, Spawn { team: 0, place: Grave, cid: URIA },
        Spawn { team: 0, place: Grave, cid: HAMON }, Spawn { team: 0, place: Grave, cid: RAVIEL }];
    let Some(mut s) = duel(true, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DARK_SUMMONING_BEAST)]).unwrap();
    clean(&s);
    assert!(s.monster(0, URIA).is_some() && s.monster(0, HAMON).is_some() && s.monster(0, RAVIEL).is_some(), "all three came: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_SUMMONING_BEAST), 1, "{}", s.context());
}

/// Dark Tuner Catastrogue: "You can reveal this card in your hand; immediately after this effect resolves, Normal Summon 1
/// "Dark Tuner" monster." Dark Tuner Nightmare Hand (Level 10), Tributing two Warwolves.
#[test]
fn catastrogue_in_the_hand_normal_summons_a_dark_tuner() {
    let spawns = [Spawn { team: 0, place: Hand, cid: CATASTROGUE }, Spawn { team: 0, place: Hand, cid: NIGHTMARE_HAND },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[DARK_HOLE; 40], &[]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, NIGHTMARE_HAND)]);
    s.run(&[Activate(CATASTROGUE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, NIGHTMARE_HAND).is_some(), "a Dark Tuner was Normal Summoned: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 2, "by Tributing the two Warwolves: {}", s.context());
}

/// Dark Tuner Catastrogue: "Once per turn, if you control this Normal Summoned/Set card: You can Tribute both this
/// face-up card and 1 face-up non-Tuner with a lower Level, and if you do, Special Summon 1 Synchro Monster from your
/// Extra Deck with a Level equal to the difference in Levels of those monsters, then you can destroy 1 card your
/// opponent controls." Level 8 and Warwolf (Level 4): Dark Diviner (Level 4); the opponent's Warwolf is destroyed.
#[test]
fn catastrogue_tributes_for_a_synchro_monster() {
    let spawns = [Spawn { team: 0, place: Hand, cid: CATASTROGUE }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = duel(false, [(&[DARK_HOLE; 40], &[DARK_DIVINER]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.prefer(&[(1, BLUE_EYES)]);
    s.run(&[Summon(CATASTROGUE), Activate(CATASTROGUE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_DIVINER), Some(2000), "the Level 4 Synchro Monster came: {}", s.context());
    assert_eq!(s.count(0, "grave", CATASTROGUE), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "and the opponent's card was destroyed: {}", s.context());
}

/// Dark Tuner Nightmare Hand: "If this card is Normal Summoned: You can Special Summon 1 Level 2 or lower monster from
/// your hand." Kuriboh (Level 1) comes from the hand.
#[test]
fn nightmare_hand_normal_summoned_brings_a_low_level_monster() {
    let spawns = [Spawn { team: 0, place: Hand, cid: NIGHTMARE_HAND }, Spawn { team: 0, place: Hand, cid: KURIBOH },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[DARK_HOLE; 40], &[]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.run(&[Summon(NIGHTMARE_HAND), Chain(NIGHTMARE_HAND)]).unwrap();
    clean(&s);
    assert!(s.monster(0, NIGHTMARE_HAND).is_some(), "{}", s.context());
    assert!(s.monster(0, KURIBOH).is_some(), "Kuriboh was Special Summoned from the hand: {}", s.context());
}

/// Dark Tuner Nightmare Hand: "Once per turn, if you control this Normal Summoned/Set card: You can Tribute both this
/// face-up card and 1 face-up non-Tuner with a lower Level, and if you do, Special Summon 1 Synchro Monster from your
/// Extra Deck with a Level equal to the difference in Levels." Level 10 and Warwolf (Level 4): Goyo Guardian (Level 6).
#[test]
fn nightmare_hand_tributes_for_a_synchro_monster() {
    let spawns = [Spawn { team: 0, place: Hand, cid: NIGHTMARE_HAND }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[DARK_HOLE; 40], &[GOYO_GUARDIAN]), (&[DARK_HOLE; 40], &[])], &spawns) else { return };
    s.run(&[Summon(NIGHTMARE_HAND), Activate(NIGHTMARE_HAND)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GOYO_GUARDIAN), Some(2800), "the Level 6 Synchro Monster came: {}", s.context());
    assert_eq!(s.count(0, "grave", NIGHTMARE_HAND), 1, "{}", s.context());
}

/// Dark Tyranno: "Can attack directly if all monsters your opponent controls are in Defense Position."
#[test]
fn dark_tyranno_attacks_directly_when_every_opposing_monster_is_defending() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DARK_TYRANNO }, Spawn { team: 1, place: MonsterDefense, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Attack(DARK_TYRANNO)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2600, "the 2600 went to the player, not the Defense Position monster: {}", s.context());
}

/// Dark Tyranno: "During your Main Phase, if a Spell Card or effect was activated this turn: You can Special Summon this
/// card from your hand." After Dark Hole.
#[test]
fn dark_tyranno_comes_from_the_hand_after_a_spell() {
    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_TYRANNO }, Spawn { team: 0, place: Hand, cid: DARK_HOLE },
        Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    assert!(!can_activate(&s, DARK_TYRANNO), "no Spell yet this turn: {}", s.context());
    s.run(&[Activate(DARK_HOLE), Activate(DARK_TYRANNO)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_TYRANNO), Some(2600), "Special Summoned from the hand: {}", s.context());
}

/// Dark Tyranno: "If this card is sent to the GY: You can add 1 Level 8 or higher Dinosaur monster from your Deck to your
/// hand, then you can Special Summon it if your opponent controls a monster." Blue-Eyes destroys it in battle.
#[test]
fn dark_tyranno_in_the_graveyard_adds_a_big_dinosaur() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DARK_TYRANNO }, Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = duel(false, [(&[ULTIMATE_TYRANNO; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    let before = hand(&s, 0);
    s.run_both([&[EndTurn, Chain(DARK_TYRANNO)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_TYRANNO), 1, "{}", s.context());
    let found = hand(&s, 0) > before || s.monster(0, ULTIMATE_TYRANNO).is_some();
    assert!(found, "Ultimate Tyranno was added (and perhaps Summoned): {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------

/// Destiny HERO - Celestial: "When this card declares an attack: You can target 1 face-up Spell your opponent controls;
/// destroy it, and if you do, inflict 500 damage to your opponent." Dark Hole face-up on their side is destroyed, and their
/// 1600 LP... the attack itself is direct: 500 plus Celestial's 1600 ATK.
#[test]
fn celestial_destroys_a_spell_when_it_attacks() {
    let spawns = [Spawn { team: 0, place: Monster, cid: CELESTIAL }, Spawn { team: 1, place: Spell, cid: DARK_HOLE }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Attack(CELESTIAL), Chain(CELESTIAL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "the Spell was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 500 - 1600, "500 effect damage and the direct attack: {}", s.context());
}

/// Destiny HERO - Celestial: "While you have no cards in your hand, except the turn this card was sent to the GY: You can
/// banish this card and 1 "Destiny HERO" monster from your GY; draw 2 cards." The hand is emptied (Set Spells and one
/// Dark Hole) first: with cards in it the effect is not offered.
#[test]
fn celestial_in_the_graveyard_draws_two_with_an_empty_hand() {
    let spawns = [Spawn { team: 0, place: Grave, cid: CELESTIAL }, Spawn { team: 0, place: Grave, cid: DESTINY_HERO_DEFENDER },
        Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[DARK_HOLE; 8], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    assert!(!can_activate(&s, CELESTIAL), "cards in the hand: {}", s.context());
    s.run(&[Activate(DARK_HOLE), SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), SetSpell(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), 0, "{}", s.context());
    assert!(can_activate(&s, CELESTIAL), "the hand is empty: {}", s.context());
    s.run(&[Activate(CELESTIAL)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), 2, "drew 2: {}", s.context());
    assert_eq!((s.count(0, "removed", CELESTIAL), s.count(0, "removed", DESTINY_HERO_DEFENDER)), (1, 1), "{}", s.context());
}

/// Destiny HERO - Dark Angel: "If you have 3 or more "Destiny HERO" monsters in your GY: You can discard this card, then
/// target 1 "Destiny HERO" monster in your GY; Special Summon it to your opponent's field in Defense Position."
#[test]
fn dark_angel_gives_a_destiny_hero_to_the_opponent() {
    let spawns = [Spawn { team: 0, place: Hand, cid: DARK_ANGEL }, Spawn { team: 0, place: Grave, cid: DESTINY_HERO_DEFENDER },
        Spawn { team: 0, place: Grave, cid: DESTINY_HERO_DIAMOND_DUDE }, Spawn { team: 0, place: Grave, cid: DESTINY_HERO_MALICIOUS }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, DESTINY_HERO_DIAMOND_DUDE)]);
    s.run(&[Activate(DARK_ANGEL)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, DESTINY_HERO_DIAMOND_DUDE), Some(4), "on the opponent's field in Defense: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_ANGEL), 1, "discarded: {}", s.context());
}

/// Destiny HERO - Dark Angel: "When a Spell Card or effect you activate resolves, negate that effect, and if you do, or if
/// it did not have an effect, destroy that card." Dark Hole does nothing and is destroyed.
#[test]
fn dark_angel_negates_its_controllers_spells() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DARK_ANGEL }, Spawn { team: 0, place: Hand, cid: DARK_HOLE },
        Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert!(s.monster(1, WARWOLF).is_some() && s.monster(0, DARK_ANGEL).is_some(), "Dark Hole was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "{}", s.context());
}

/// Diabound Kernel: "When this card declares an attack: It gains 600 ATK." 1800 becomes 2400.
#[test]
fn diabound_kernel_gains_600_atk_when_it_attacks() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DIABOUND_KERNEL }, Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    assert_eq!(s.attack_of(0, DIABOUND_KERNEL), Some(1800));
    s.run(&[Attack(DIABOUND_KERNEL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DIABOUND_KERNEL), Some(2400), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 400, "2400 against Warwolf's 2000: {}", s.context());
}

/// Diabound Kernel: "(Quick Effect): You can target 1 face-up monster your opponent controls; it loses ATK equal to this
/// card's current ATK until the end of this turn, then banish this card until the Standby Phase of the next turn."
#[test]
fn diabound_kernel_weakens_a_monster_and_banishes_itself() {
    let spawns = [Spawn { team: 0, place: Monster, cid: DIABOUND_KERNEL }, Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DIABOUND_KERNEL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, BLUE_EYES), Some(1200), "3000 - 1800: {}", s.context());
    assert_eq!(s.count(0, "removed", DIABOUND_KERNEL), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DIABOUND_KERNEL).is_some(), "back at the Standby Phase: {}", s.context());
}

/// Dice Dungeon: "When this card is activated: You can add 1 "Dimension Dice" from your Deck to your hand."
#[test]
fn dice_dungeon_adds_dimension_dice() {
    let spawns = [Spawn { team: 0, place: Hand, cid: DICE_DUNGEON }];
    let Some(mut s) = duel(false, [(&[DIMENSION_DICE; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(DICE_DUNGEON)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before, "one card left the hand, one came: {}", s.context());
}

/// Doble Passe: "When an opponent's monster declares an attack on a face-up Attack Position monster you control: Make your
/// opponent's attack a direct attack instead and inflict damage to your opponent equal to the ATK of the attack target you
/// control." Blue-Eyes attacks Warwolf: its controller takes 2000, we take the direct 3000.
#[test]
fn doble_passe_makes_the_attack_direct_and_deals_the_targets_atk() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DOBLE_PASSE }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(DOBLE_PASSE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 3000, "{}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "{}", s.context());
}

/// Doctor D: "Banish 1 "Destiny HERO" monster from your GY; add to your hand or Special Summon 1 "Destiny HERO" monster
/// from your GY."
#[test]
fn doctor_d_recovers_a_destiny_hero() {
    let spawns = [Spawn { team: 0, place: Hand, cid: DOCTOR_D }, Spawn { team: 0, place: Grave, cid: DESTINY_HERO_DEFENDER },
        Spawn { team: 0, place: Grave, cid: DESTINY_HERO_DIAMOND_DUDE }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, DESTINY_HERO_DEFENDER), (0, DESTINY_HERO_DIAMOND_DUDE)]);
    s.run(&[Activate(DOCTOR_D)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", DESTINY_HERO_DEFENDER), 1, "the cost: {}", s.context());
    let back = s.count(0, "hand", DESTINY_HERO_DIAMOND_DUDE) + s.count(0, "mzone", DESTINY_HERO_DIAMOND_DUDE);
    assert_eq!(back, 1, "Diamond Dude came back to the hand or the field: {}", s.context());
}

/// Doctor D: "You can banish this card from your GY, then target 2 "Destiny HERO" monsters you control; choose 1 of those
/// monsters, and its ATK becomes the other monster's ATK." Defender (100) and Diamond Dude (1400) end with the same ATK.
#[test]
fn doctor_d_in_the_graveyard_copies_an_atk() {
    let spawns = [Spawn { team: 0, place: Grave, cid: DOCTOR_D }, Spawn { team: 0, place: Monster, cid: DESTINY_HERO_DEFENDER },
        Spawn { team: 0, place: Monster, cid: DESTINY_HERO_DIAMOND_DUDE }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Activate(DOCTOR_D)]).unwrap();
    clean(&s);
    let (a, b) = (s.attack_of(0, DESTINY_HERO_DEFENDER), s.attack_of(0, DESTINY_HERO_DIAMOND_DUDE));
    assert_eq!(a, b, "both have the same ATK: {}", s.context());
    assert_eq!(s.count(0, "removed", DOCTOR_D), 1, "{}", s.context());
}

/// Doll Hammer: "Target 1 monster you control; destroy it, and if you do, draw 2 cards, then you can change the battle
/// position of 1 monster your opponent controls."
#[test]
fn doll_hammer_destroys_our_monster_and_draws_two() {
    let spawns = [Spawn { team: 0, place: Hand, cid: DOLL_HAMMER }, Spawn { team: 0, place: Monster, cid: WARWOLF }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(DOLL_HAMMER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before - 1 + 2, "the Hammer left, two cards drawn: {}", s.context());
}

/// Doll Hammer: "If a monster(s) is Special Summoned from your GY while this card is in your GY: You can add this card to
/// your hand." Monster Reborn revives Warwolf.
#[test]
fn doll_hammer_returns_when_a_monster_comes_from_the_graveyard() {
    let spawns = [Spawn { team: 0, place: Grave, cid: DOLL_HAMMER }, Spawn { team: 0, place: Grave, cid: WARWOLF },
        Spawn { team: 0, place: Hand, cid: MONSTER_REBORN }];
    let Some(mut s) = duel(false, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(MONSTER_REBORN), Chain(DOLL_HAMMER)]).unwrap();
    clean(&s);
    assert!(s.monster(0, WARWOLF).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "hand", DOLL_HAMMER), 1, "{}", s.context());
}
