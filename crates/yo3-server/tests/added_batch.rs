//! The Orichalcos and Battle City batch of added cards (#75), and the game's cards they play with: each through the real
//! core with both seats' client models checked after every step (`duelcheck::scenario`). Card texts: the official
//! database for the printed cards, ProjectIgnis' anime database for the anime ones (`[Anime]` in the game).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// Added cards (rows 4836-4846; ids from engine/added-cards.json, a stand-in where the Konami id is too large or none).
const RAS_DISCIPLE: u16 = 11169;
const SEAL: u16 = 10327; // The Seal of Orichalcos
const GEH: u16 = 12671; // Divine Serpent Geh
const ILLUSION_GATE: u16 = 3976;
const UNDERWORLD_CIRCLE: u16 = 3978;
const ARDUOUS_DECISION: u16 = 3980;
const DEUTEROS: u16 = 3981;
const TRITOS: u16 = 3982;
const KYUTORA: u16 = 3984;
const DEXIA: u16 = 3985;
const ARISTEROS: u16 = 3986;
// The game's own.
const SHUNOROS: u16 = 8535;
const FOOLISH_BURIAL: u16 = 5236;
const BLAST: u16 = 5365; // Blast Held by a Tribute
const NULL_AND_VOID: u16 = 5362; // Muko
const OBELISK: u16 = 4998;
const BLUE_EYES: u16 = 4007;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;
const POT_OF_GREED: u16 = 4844;
const MONSTER_REBORN: u16 = 4842;
const STARDUST: u16 = 7734; // Stardust Dragon, a Synchro Monster

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn field(anime: bool, lp: u32, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }.unwrap())
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn count_all(s: &Scenario, team: usize, place: &str) -> usize {
    s.state()[team][place].as_array().map_or(0, |cards| cards.iter().filter(|c| !c.is_null()).count())
}

/// Ra's Disciple: "When this card is Summoned: You can Special Summon up to 2 "Ra's Disciple" from your hand and/or Deck."
#[test]
fn ras_disciple_brings_two_more() {
    let deck: Vec<u16> = [KURIBOH; 37].into_iter().chain([RAS_DISCIPLE, RAS_DISCIPLE]).collect();
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &[Spawn { team: 0, place: Hand, cid: RAS_DISCIPLE }]) else { return };
    s.run(&[Summon(RAS_DISCIPLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", RAS_DISCIPLE), 3, "{}", s.state());
}

/// The Seal of Orichalcos: "All monsters you control gain 500 ATK." Gene-Warped Warwolf, Normal Summoned, goes to 2500.
#[test]
fn the_seal_of_orichalcos_gives_500_atk() {
    let spawns = [Spawn { team: 0, place: Hand, cid: SEAL }, Spawn { team: 0, place: Hand, cid: WARWOLF }];
    let Some(mut s) = field(false, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Summon(WARWOLF), Activate(SEAL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2500), "{}", s.state());
}

/// Orichalcos Deuteros (anime-only): "Activate only if The Seal of Orichalcos is in your Field Zone. This card gains the
/// effects of The Seal of Orichalcos. Once per turn: You can gain 500 LP for each monster you control." Then Orichalcos
/// Tritos over it ("Activate only if Orichalcos Deuteros is in your Field Zone"), which keeps the Seal's 500 ATK.
#[test]
fn the_three_seals_of_orichalcos_follow_each_other() {
    let spawns = [Spawn { team: 0, place: Hand, cid: SEAL }, Spawn { team: 0, place: Hand, cid: DEUTEROS },
        Spawn { team: 0, place: Hand, cid: TRITOS }, Spawn { team: 0, place: Hand, cid: WARWOLF }];
    let Some(mut s) = field(true, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[Summon(WARWOLF), Activate(SEAL), Activate(DEUTEROS), Activate(DEUTEROS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", DEUTEROS), 1, "Deuteros took the Field Zone: {}", s.state());
    assert_eq!(s.lp(0), 8500, "500 LP for the one monster");
    assert_eq!(s.attack_of(0, WARWOLF), Some(2500), "the Seal's 500 ATK");
    s.run(&[Activate(TRITOS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", TRITOS), 1, "Tritos over Deuteros: {}", s.state());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2500), "still the Seal's 500 ATK");
}

/// Divine Serpent Geh (printed): "If a monster(s) you control is destroyed by an opponent's attack or card effect: You can
/// pay half your LP; Special Summon this card from your hand." Blue-Eyes destroys Gene-Warped Warwolf (1000 damage), Geh
/// comes for half of the 7000 left.
#[test]
fn divine_serpent_geh_comes_when_a_monster_is_destroyed() {
    let spawns = [Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Hand, cid: GEH },
        Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = field(false, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard (seven cards) keeps Geh
    s.run_both([&[EndTurn, Chain(GEH)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, GEH).is_some(), "Geh came: {}", s.state());
    assert_eq!(s.lp(0), 3500, "1000 battle damage, then half of 7000");
}

/// Illusion Gate (printed): "Pay half your LP; destroy as many monsters your opponent controls as possible, then you can
/// Special Summon 1 monster from your opponent's GY to your field, ignoring its Summoning conditions."
#[test]
fn illusion_gate_destroys_and_takes_a_monster_from_the_opponents_graveyard() {
    let spawns = [Spawn { team: 0, place: Hand, cid: ILLUSION_GATE }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Grave, cid: BLUE_EYES }];
    let Some(mut s) = field(false, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(1, BLUE_EYES)]);
    s.run(&[Activate(ILLUSION_GATE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "half the LP");
    assert!(s.monster(1, WARWOLF).is_none(), "the opponent's monster was destroyed: {}", s.state());
    assert_eq!(s.attack_of(0, BLUE_EYES), Some(3000), "Blue-Eyes came from their Graveyard: {}", s.state());
}

/// The anime combo, by the server's house rule (crates/yo3-engine/src/house_rules.lua): the anime Foolish Burial ("Send
/// 1 card from your Deck to your opponent's Graveyard") puts Divine Serpent Geh in the opponent's Graveyard, and Illusion
/// Gate takes it from there "ignoring its Summoning conditions", which here also means "must first be properly Summoned"
/// (the official ruling keeps that limit). Summoned, anime Geh sends its controller's hand to the Graveyard and takes all
/// their LP, and they "cannot lose the Duel".
#[test]
fn foolish_burial_and_illusion_gate_summon_geh_from_the_opponents_graveyard() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([GEH]).collect();
    let spawns = [Spawn { team: 0, place: Hand, cid: FOOLISH_BURIAL }, Spawn { team: 0, place: Hand, cid: ILLUSION_GATE },
        Spawn { team: 1, place: Monster, cid: WARWOLF }];
    let Some(mut s) = field(true, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, GEH), (1, GEH)]);
    s.run(&[Activate(FOOLISH_BURIAL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", GEH), 1, "Geh is in the opponent's Graveyard: {}", s.state());
    s.run(&[Activate(ILLUSION_GATE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, GEH).is_some(), "Geh is ours: {}", s.state());
    assert_eq!((s.lp(0), hand(&s, 0)), (0, 0), "its summon took the hand and every LP: {}", s.state());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(s.monster(0, GEH).is_some(), "at 0 LP the duel goes on: {}", s.state());
}

/// The house rule's boundary: Illusion Gate ("ignoring its Summoning conditions") takes a Stardust Dragon the opponent
/// never Synchro Summoned; Monster Reborn, which ignores nothing, still can't revive one that never was.
#[test]
fn ignoring_summoning_conditions_lifts_proper_summoning_and_monster_reborn_does_not() {
    let spawns = [Spawn { team: 0, place: Hand, cid: ILLUSION_GATE }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Grave, cid: STARDUST }];
    let Some(mut s) = field(false, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[STARDUST])], &spawns) else { return };
    s.prefer(&[(1, STARDUST)]);
    s.run(&[Activate(ILLUSION_GATE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, STARDUST), Some(2500), "Illusion Gate took the never-Summoned Stardust Dragon: {}", s.state());

    let spawns = [Spawn { team: 0, place: Hand, cid: MONSTER_REBORN }, Spawn { team: 0, place: Grave, cid: STARDUST }];
    let Some(mut s) = field(false, 8000, [(&[KURIBOH; 40], &[STARDUST]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    assert!(s.run(&[Activate(MONSTER_REBORN)]).is_err(), "Monster Reborn has no target: {}", s.state());
    assert_eq!(s.count(0, "grave", STARDUST), 1);
}

/// Arduous Decision (anime) Special Summons the card the opponent picks "ignoring its Summoning conditions", from the hand
/// it was just drawn to: Divine Serpent Geh comes that way. Anime Geh, Summoned, sends its controller's hand to the
/// Graveyard and takes all their LP; they "cannot lose the Duel", and it gains infinite ATK.
#[test]
fn arduous_decision_summons_geh_ignoring_its_conditions() {
    let Some(mut s) = field(true, 8000, [(&[GEH; 40], &[]), (&[KURIBOH; 40], &[])], &[Spawn { team: 0, place: Hand, cid: ARDUOUS_DECISION }]) else { return };
    s.run(&[Activate(ARDUOUS_DECISION)]).unwrap();
    clean(&s);
    assert!(s.monster(0, GEH).is_some(), "Geh is on the field: {}", s.state());
    assert_eq!((s.lp(0), hand(&s, 0)), (0, 0), "its summon took the hand and every LP: {}", s.state());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(s.monster(0, GEH).is_some(), "at 0 LP the duel goes on: {}", s.state());
}

/// Anime Underworld Circle: "Each player can activate this effect (Quick Effect): the player who activated this effect
/// Special Summons 1 monster from their Graveyard ... Monsters can be Summoned ignoring the Summoning conditions." After
/// clearing the field, it brings back Gene-Warped Warwolf, and, by the house rule, a Divine Serpent Geh never Summoned.
#[test]
fn underworld_circle_brings_a_monster_back_from_the_graveyard() {
    let spawns = [Spawn { team: 0, place: Hand, cid: UNDERWORLD_CIRCLE }, Spawn { team: 0, place: Grave, cid: WARWOLF },
        Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    // Spell decks: the Circle banishes every monster in both Decks, and a Deck of monsters would leave no draw.
    let Some(mut s) = field(true, 8000, [(&[DARK_HOLE; 40], &[]), (&[POT_OF_GREED; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[SetSpell(UNDERWORLD_CIRCLE), EndTurn, Activate(UNDERWORLD_CIRCLE), Activate(UNDERWORLD_CIRCLE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "Warwolf came back: {}", s.state());
}

#[test]
fn underworld_circle_brings_back_a_geh_never_summoned() {
    let spawns = [Spawn { team: 0, place: Hand, cid: UNDERWORLD_CIRCLE }, Spawn { team: 0, place: Grave, cid: GEH },
        Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = field(true, 8000, [(&[DARK_HOLE; 40], &[]), (&[POT_OF_GREED; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, GEH)]);
    s.run(&[SetSpell(UNDERWORLD_CIRCLE), EndTurn, Activate(UNDERWORLD_CIRCLE), Activate(UNDERWORLD_CIRCLE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, GEH).is_some(), "Geh came back: {}", s.state());
}

/// Orichalcos Kyutora (anime-only): "Must first be Special Summoned (from your hand) by paying 500 LP. ... all battle
/// damage you would take from battles involving monsters you control becomes 0. When this card is destroyed: You can
/// Special Summon 1 Orichalcos Shunoros from your hand or Deck." Anime Shunoros: "When this card is Summoned: You can
/// Special Summon 1 Orichalcos Aristeros and 1 Orichalcos Dexia from your hand and/or Deck. This card's ATK becomes equal
/// to the total amount of battle damage reduced to 0 by the effect of the Orichalcos Kyutora that Summoned this card."
/// Blue-Eyes destroys Kyutora: 2500 damage reduced to 0, and Shunoros comes with 2500 ATK and both arms.
#[test]
fn orichalcos_kyutora_brings_shunoros_and_its_arms() {
    let deck: Vec<u16> = [KURIBOH; 37].into_iter().chain([SHUNOROS, DEXIA, ARISTEROS]).collect();
    let spawns = [Spawn { team: 0, place: Hand, cid: KYUTORA }, Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = field(true, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[SpSummon(KYUTORA)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, KYUTORA), s.lp(0)), (Some(500), 7500), "summoned for 500 LP: {}", s.state());
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "no battle damage");
    assert_eq!(s.attack_of(0, SHUNOROS), Some(2500), "Shunoros has the 2500 reduced: {}", s.state());
    assert!(s.monster(0, DEXIA).is_some() && s.monster(0, ARISTEROS).is_some(), "and both arms: {}", s.state());
}

/// Anime Shunoros: "If this card on the field is destroyed while it has 0 ATK, and you have 10,000 or more LP: Special
/// Summon 1 Divine Serpent Geh from your hand or Deck." Placed at 0 ATK (no Kyutora), 10000 LP, destroyed by Dark Hole.
#[test]
fn shunoros_destroyed_at_0_atk_with_10000_lp_brings_geh() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([GEH]).collect();
    let spawns = [Spawn { team: 0, place: Monster, cid: SHUNOROS }, Spawn { team: 1, place: Hand, cid: DARK_HOLE }];
    let Some(mut s) = field(true, 10000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    assert_eq!(s.attack_of(0, SHUNOROS), Some(0), "{}", s.state());
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, GEH).is_some(), "Geh came from the Deck: {}", s.state());
}

/// Arduous Decision (anime-only): "Draw 2 cards from the top of your Deck, then have your opponent select one of those
/// cards. If the selected card is a monster, Special Summon it, ignoring its Summoning conditions."
#[test]
fn arduous_decision_summons_the_monster_the_opponent_picks() {
    let Some(mut s) = field(true, 8000, [(&[WARWOLF; 40], &[]), (&[KURIBOH; 40], &[])], &[Spawn { team: 0, place: Hand, cid: ARDUOUS_DECISION }]) else { return };
    s.run(&[Activate(ARDUOUS_DECISION)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "a Warwolf was Special Summoned: {}", s.state());
}

/// Underworld Circle (anime): "Destroy all monsters on the field and banish all monsters in both players' Decks."
#[test]
fn underworld_circle_clears_the_field_and_the_decks_monsters() {
    let deck0: Vec<u16> = [WARWOLF, DARK_HOLE].into_iter().cycle().take(40).collect();
    let spawns = [Spawn { team: 0, place: Hand, cid: UNDERWORLD_CIRCLE }, Spawn { team: 0, place: Monster, cid: KURIBOH },
        Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let Some(mut s) = field(true, 8000, [(&deck0, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.run(&[SetSpell(UNDERWORLD_CIRCLE), EndTurn, Activate(UNDERWORLD_CIRCLE)]).unwrap();
    clean(&s);
    assert_eq!((count_all(&s, 0, "mzone"), count_all(&s, 1, "mzone")), (0, 0), "every monster destroyed: {}", s.state());
    assert!(s.count(0, "removed", WARWOLF) > 0 && s.count(1, "removed", KURIBOH) > 0, "the Decks' monsters banished: {}", s.state());
}

/// Battle City, Ishizu against Kaiba: Blast Held by a Tribute (anime: "When this card is Set: Choose 1 monster on the
/// field. When your opponent's Tribute Summoned monster that used the chosen card as Tribute declares an attack: Destroy it
/// as if destroyed by its own effect, and if you do, inflict damage to its controller equal to its ATK"). Kaiba Tributes
/// three monsters, the chosen one among them, for Obelisk the Tormentor; Obelisk attacks and is destroyed, Kaiba takes 4000.
#[test]
fn blast_held_by_a_tribute_destroys_obelisk() {
    let spawns = [Spawn { team: 0, place: Hand, cid: BLAST }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Monster, cid: WARWOLF }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Hand, cid: OBELISK }];
    let Some(mut s) = field(true, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[SetSpell(BLAST), ChainOnAttack(BLAST)], &[Summon(OBELISK), Attack(OBELISK)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, OBELISK).is_none(), "Obelisk was destroyed: {}", s.state());
    assert_eq!(s.lp(1), 8000 - 4000, "Kaiba took Obelisk's ATK");
}

/// Muko, the game's Null and Void (anime: "When your opponent draws a card(s): Send those card(s) to the Graveyard"):
/// Set, it sends the opponent's draw of their turn to the Graveyard.
#[test]
fn null_and_void_sends_the_opponents_draw_to_the_graveyard() {
    let Some(mut s) = field(true, 8000, [(&[KURIBOH; 40], &[]), (&[POT_OF_GREED; 40], &[])], &[Spawn { team: 0, place: Hand, cid: NULL_AND_VOID }]) else { return };
    let before = hand(&s, 1);
    s.run_both([&[SetSpell(NULL_AND_VOID), Chain(NULL_AND_VOID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", POT_OF_GREED), 1, "their draw went to the Graveyard: {}", s.state());
    assert_eq!(hand(&s, 1), before, "their hand didn't grow");
}
