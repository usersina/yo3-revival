//! Anime-only cards, group 27 (rows 6086-6118 of engine/added/cards.tsv): Breo's, Clark Smith's, Elsworth's, Lester's, Lyndon's
//! and Andre's cards (the Knights and Clock Knights, the Cups, the Reactors, the Flares and the Traps of the Different Dimension).
//! These cards exist only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes
//! through the real core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the
//! anime ones. Most of the batch is Traps (type 4): each is Set first and answers the opponent, a Set card waits for turn 3 to
//! be activated on the Spell's own turn, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! A coin's result comes from the duel's seed: each coin card runs over seeds until both faces have come up. A Synchro Monster
//! put into the Graveyard by hand was never properly summoned and no effect revives it, so Regretful Tuning's test Synchro
//! Summons its monster first. A card spawned from a Deck shuffles that Deck: team 1's Tricolor Illusion decks hold what they
//! need in the opening hand instead.
//! Covered: the Token (Ultimate Flare Token, made by Ultimate Flare, reaching the field and leaving it destroyed), every card
//! the per-card fuzz flagged (Damage Lance, Regretful Tuning, Tricolor Illusion, Decago Knight, Triangle Knight, Master and
//! Servant's Resolve, Reversal Sword, Sky A3, Different Dimension Barrier - Lost Force, Magic Charge, Level Change and Delta
//! Reactor: the random duels never held what their conditions ask (a direct attack of 2000, three Decago Knights in the
//! Graveyard, a Synchro Monster properly summoned, Skiel Attack beside a Meklord Emperor, three Reactors...); Master and Servant's
//! Resolve's damage is the one the fuzz could not see: it is a step damage ended by `RDComplete`, and
//! `master_and_servants_resolve_damages_...` shows the 1800), the two cards with a script note (Tricolor Illusion: an empty hand
//! in the End Phase leaves the choice empty, not an error, so no override; Dimension Trap: it did stop on a Lua error, so it has
//! an override, `engine/overrides/c511000952.lua`), the signature cards (Ultimate Flare, Delta Reactor, Tricolor Illusion,
//! Dimension Trap and Different Dimension Barrier - Lost Force) and the seeded sample of the rest (Clock Knight No. 12, Clock
//! Knight No. 3, Cup Udon Uno, Fair Exchange, Glory Shield, Infinite Aura), then every other card too, since each was cheap:
//! Flattery, Hexa Knight, Liberated Rear Warrior, Spirit Summit Warrior, Exus Summon, Infinity Barrage, Clockwork, Monster
//! Clock, Cup Meatball Cayenne, Cup Ramen Lard, Martyr Flag and Damage Summon.
//! Three more overrides came out of the texts: Regretful Tuning (`c511000951.lua`: its condition read the empty card group of
//! the battle damage event, so the Trap could never be chained), Triangle Knight (`c511002948.lua`: it asked for two free Monster
//! Zones, which its text does not), and Reversal Sword (`c511000946.lua`: the damage was the ATK of the monster it destroys, not
//! of the destroyed monster).
//! Not coverable: Infinity Barrage's "no attack declared yet this turn" (the Battle Phase leaves the scenario no Main Phase 2
//! prompt to look at). The fuzz's own failing duels (Flattery's and Martyr Flag's on Chaos Life, Sky A3's on Yomi Kid, Cup
//! Meatball Cayenne's on Attribute Bomb) were those other cards' errors, not this batch's.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const BATTLE_WARRIOR: u16 = 4106; // Level 3 Warrior, 700/1000, vanilla
const HERO_OF_THE_EAST: u16 = 4116; // Level 3 Warrior, 1100/1000, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MYSTICAL_SPACE_TYPHOON: u16 = 4909;
const BLUE_EYES: u16 = 4007; // Level 8 LIGHT Dragon, 3000/2500
const JUNK_SYNCHRON: u16 = 7687; // Level 3 Tuner, 1300/500

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }
fn face_down(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: team 0's has `bottom` at its very
/// bottom (out of the opening hand and the first draws), for a card an effect fetches from the Deck.
fn duel(seed: u64, lp: u32, fillers: [u16; 2], bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(fillers[0], 40 - bottom.len())).collect();
    let decks = [(&mine[..], &[][..]), (&vec![fillers[1]; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [FILLER, FILLER], bottom, spawns) }

/// A duel with both Decks given, bottom first (the opening hand is the last five cards, the next draw the sixth from the end).
fn decks(seed: u64, mine: &[u16], theirs: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(Scenario::start_anime_field(&dir, seed, 8000, [(mine, &[][..]), (theirs, &[][..])], spawns).unwrap())
}

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

// ---------------------------------------------------------------------------------------------------------------
// Ultimate Flare and the Ultimate Flare Token
// ---------------------------------------------------------------------------------------------------------------

/// Ultimate Flare (a Normal Trap, Set first and activated in turn 3): "Special Summon 1 "Ultimate Flare Token" (Machine-Type/DARK/Level 10/ATK 0/DEF 0). When an "Ultimate Flare
/// Token" is destroyed, your opponent takes 800 damage. Monsters you control have their effects negated until the End Phase
/// of the turn that this card is activated." The Token (Machine, DARK, Level 10, 0/0, face-up Attack) reaches the field and
/// Dark Magician Girl's boost for the Magicians in the Graveyard (+300 for one) is negated: 2000, not 2300, until the End Phase.
#[test]
fn ultimate_flare_makes_its_token_and_negates_my_monsters_until_the_end_phase() {
    let [flare, token, dmg] = [card("Ultimate Flare"), card("Ultimate Flare Token"), card("Dark Magician Girl")];
    let Some(mut s) = field(&[], &[set(0, flare), mon(0, dmg), grave(1, DARK_MAGICIAN)]) else { return };
    assert_eq!(s.attack_of(0, dmg), Some(2300), "the boost works first: {}", s.context());
    s.run_both([&[EndTurn, Activate(flare)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 1, "the Token is on my field: {}", s.context());
    assert_eq!(s.count(0, "grave", flare), 1, "{}", s.context());
    let t = s.monster(0, token).unwrap();
    assert_eq!((t["attack"].as_i64(), t["defense"].as_i64(), t["level"].as_i64()), (Some(0), Some(0), Some(10)), "{}", s.context());
    assert_eq!(s.position_of(0, token), Some(1), "face-up Attack: {}", s.context());
    assert_eq!(s.attack_of(0, dmg), Some(2000), "Dark Magician Girl is negated: {}", s.context());
    // Until the End Phase: back to 2300 in my next turn's Main Phase.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, dmg), Some(2300), "negated until the End Phase only: {}", s.context());
}

/// Ultimate Flare Token: "When an "Ultimate Flare Token" is destroyed, your opponent takes 800 damage." Dark Hole destroys it
/// (and the opponent's Warwolf): the Token leaves the field without a trace in the Graveyard and team 1 takes 800.
#[test]
fn a_destroyed_ultimate_flare_token_costs_the_opponent_800() {
    let [flare, token] = ["Ultimate Flare", "Ultimate Flare Token"].map(card);
    let Some(mut s) = field(&[], &[set(0, flare), hand(0, DARK_HOLE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(flare), Activate(DARK_HOLE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token) + s.count(0, "grave", token), 0, "the Token is gone: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 800), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Damage Lance
// ---------------------------------------------------------------------------------------------------------------

/// Damage Lance: "Activate only when an opponent's monster with 2000 or more ATK declares a direct attack. Inflict 1000
/// damage to your opponent. The attacking monster loses 1000 ATK during damage calculation only." Team 1's Warwolf (2000)
/// attacks me directly: team 1 takes 1000 and, at 1000 ATK, Warwolf's battle damage to me is 1000 (not 2000); it has its 2000
/// again afterwards.
#[test]
fn damage_lance_hurts_the_attacker_and_weakens_its_direct_attack() {
    let lance = card("Damage Lance");
    let Some(mut s) = field(&[], &[set(0, lance), mon(1, WARWOLF)]) else { return };
    s.run_both([&[ChainOnAttack(lance)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", lance), 1, "Damage Lance was activated: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1000, 8000 - 1000), "1000 each: {}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(2000), "the loss was for the damage calculation: {}", s.context());
}

/// Damage Lance: "with 2000 or more ATK" and "a direct attack": Thunder Kid (700) attacking directly, and Warwolf attacking
/// a monster of mine, give it no chance.
#[test]
fn damage_lance_answers_only_a_strong_direct_attack() {
    let lance = card("Damage Lance");
    let Some(mut s) = field(&[], &[set(0, lance), mon(1, THUNDER_KID)]) else { return };
    let err = s.run_both([&[ChainOnAttack(lance)], &[Attack(THUNDER_KID)]]).expect_err("700 ATK is too weak");
    assert!(err.contains("never became possible") || err.contains("ChainOnAttack"), "{err}");
    assert_eq!(s.count(0, "grave", lance), 0, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, lance), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    let _ = s.run_both([&[ChainOnAttack(lance)], &[Attack(WARWOLF)]]);
    assert_eq!(s.count(0, "grave", lance), 0, "not a direct attack: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Flattery
// ---------------------------------------------------------------------------------------------------------------

/// Flattery: "When an opponent's monster's ATK changes during the Battle Phase: Target 1 face-up monster you control; It gains
/// ATK equal to the change, until the end of the Battle Phase." Team 1's Warwolf (2000) attacks my Dark Blade (1800) and
/// team 1 chains Rush Recklessly (+700): Flattery gives Dark Blade the same 700 (2500 against 2700), so the battle costs me
/// 200 and not 900 LP.
#[test]
fn flattery_gives_a_monster_the_atk_an_opposing_monster_gained() {
    let [flattery, rush] = ["Flattery", "Rush Recklessly"].map(card);
    let spawns = [set(0, flattery), mon(0, DARK_BLADE), set(1, rush), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF), (0, DARK_BLADE)]);
    s.run_both([&[Chain(flattery)], &[Attack(WARWOLF), Chain(rush)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", flattery), s.count(1, "grave", rush)), (1, 1), "both were activated: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 200, "2700 against 2500: {}", s.context());
    // The control: without Flattery the same battle costs 900.
    let spawns = [mon(0, DARK_BLADE), set(1, rush), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[], &[Attack(WARWOLF), Chain(rush)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 900, "2700 against 1800: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Regretful Tuning
// ---------------------------------------------------------------------------------------------------------------

/// A duel with Gaia Knight, the Force of Earth in my Extra Deck and its materials on my field (Junk Synchron, Hero of the
/// East), a Set Regretful Tuning, and team 1's Warwolf (2000) and Blue-Eyes White Dragon (3000), Warwolf first. Gaia Knight
/// is Synchro Summoned, so it is a Synchro Monster that can be revived: a Synchro Monster put into the Graveyard by hand
/// has not been properly summoned, which no effect may revive.
fn tuning_duel() -> Option<Scenario> {
    let [tuning, gaia] = [card("Regretful Tuning"), card("Gaia Knight, the Force of Earth")];
    let spawns = [set(0, tuning), mon(0, JUNK_SYNCHRON), mon(0, HERO_OF_THE_EAST), mon(1, WARWOLF), mon(1, BLUE_EYES)];
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], &[gaia][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &spawns).unwrap();
    keep_hands(&mut s);
    Some(s)
}

/// Regretful Tuning: "When you take battle damage from a direct attack: Target 1 Synchro monster in your Graveyard; Special
/// Summon it, also, if it attacks, destroy it at the end of the Battle Phase." Gaia Knight (2600) is Synchro Summoned in turn
/// 1; in turn 2 Blue-Eyes destroys it (400 damage) and Warwolf attacks me directly for 2000: Gaia Knight comes back. In turn 3
/// it attacks and is destroyed at the end of the Battle Phase, whatever the attack did.
/// (ProjectIgnis' script read the event's card group, empty at battle damage, and stopped on a Lua error before this Trap
/// could ever be chained to: engine/overrides/c511000951.lua.)
#[test]
fn regretful_tuning_revives_a_synchro_monster_that_dies_if_it_attacks() {
    let [tuning, gaia] = [card("Regretful Tuning"), card("Gaia Knight, the Force of Earth")];
    let Some(mut s) = tuning_duel() else { return };
    s.run_both([&[SpSummon(gaia), Chain(tuning)], &[Attack(BLUE_EYES), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 400 - 2000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", gaia), 1, "Gaia Knight came back: {}", s.context());
    assert_eq!(s.count(0, "grave", tuning), 1, "{}", s.context());
    // Turn 3: it attacks, then it is destroyed at the end of the Battle Phase.
    s.run_both([&[EndTurn, Attack(gaia)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", gaia) + s.count(0, "grave", gaia), 1, "{}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "grave", gaia), 1, "Gaia Knight is destroyed at the end of the Battle Phase: {}", s.context());
}

/// Regretful Tuning, the other half: "if it attacks": a Synchro monster that stays home is not destroyed.
#[test]
fn a_revived_synchro_monster_that_does_not_attack_stays() {
    let [tuning, gaia] = [card("Regretful Tuning"), card("Gaia Knight, the Force of Earth")];
    let Some(mut s) = tuning_duel() else { return };
    s.run_both([&[SpSummon(gaia), Chain(tuning)], &[Attack(BLUE_EYES), Attack(WARWOLF)]]).unwrap();
    s.run_both([&[EndTurn, EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", gaia), 1, "it never attacked: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Tricolor Illusion
// ---------------------------------------------------------------------------------------------------------------

const MIRROR_FORCE: u16 = 4887;

/// Team 1's Deck, bottom first: Feral Imps, then `top` (the card a Standby Phase would reveal first, the last of the list
/// on top), then the card team 1 draws in turn 2 and its opening hand: `opening` first, Feral Imps for the rest (a card
/// spawned from the Deck would shuffle it, so team 1's Warwolf is in its opening hand). Team 0 holds a face-up Tricolor
/// Illusion.
fn tricolor_duel(top: &[u16], opening: &[u16]) -> Option<Scenario> {
    let tricolor = card("Tricolor Illusion");
    let mut theirs: Vec<u16> = vec![FILLER; 40 - top.len() - 6];
    theirs.extend_from_slice(top);
    theirs.push(FILLER);
    theirs.extend(opening.iter().copied().chain(std::iter::repeat(FILLER)).take(5));
    let mut s = decks(7, &[FILLER; 40], &theirs, &[spell(0, tricolor)])?;
    keep_hands(&mut s);
    Some(s)
}

/// Tricolor Illusion: "Monsters your opponent controls cannot attack." Team 1's Warwolf cannot attack in turn 2; once the
/// Trap is gone (the control without it) it can.
#[test]
fn tricolor_illusion_stops_the_opponents_monsters_attacking() {
    let Some(mut s) = tricolor_duel(&[FILLER], &[WARWOLF]) else { return };
    let err = s.run_both([&[], &[Summon(WARWOLF), Attack(WARWOLF)]]).expect_err("Warwolf cannot attack");
    assert!(err.contains("can't attack"), "{err}");
    clean(&s);
    let Some(mut s) = field(&[], &[mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
}

/// Tricolor Illusion: "Once during each of their Standby Phases or End Phases, whichever is applicable first, your opponent
/// must reveal the top card of their Deck and apply 1 of the following effects: Spell: Your opponent adds that card to their
/// hand." Two Dian Ketos on top of team 1's Deck: the first goes to its hand in the Standby Phase, the second is not revealed
/// in the End Phase of the same turn (a Deck of 34 after the draw and one card more, 33, and a hand of 7 less the end-of-turn discard).
#[test]
fn tricolor_illusion_adds_a_revealed_spell_to_the_hand_once_a_turn() {
    let Some(mut s) = tricolor_duel(&[DIAN_KETO, DIAN_KETO], &[]) else { return };
    s.run_both([&[EndTurn], &[Summon(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", DIAN_KETO), 1, "Dian Keto was added to the hand: {}", s.context());
    assert_eq!(s.state()[1]["deck"].as_i64(), Some(40 - 5 - 1 - 1), "drawn and added: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", DIAN_KETO), 1, "the second Dian Keto stays on top of the Deck: {}", s.context());
    assert_eq!(s.state()[1]["deck"].as_i64(), Some(40 - 5 - 1 - 1), "{}", s.context());
}

/// Tricolor Illusion: "Trap: Your opponent discards that card and 1 card from their hand." Mirror Force on top: team 1 sends
/// it and a Feral Imp of its hand to the Graveyard.
#[test]
fn tricolor_illusion_discards_a_revealed_trap_and_a_card_from_the_hand() {
    let Some(mut s) = tricolor_duel(&[MIRROR_FORCE], &[]) else { return };
    s.run_both([&[EndTurn], &[Summon(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MIRROR_FORCE), 1, "the revealed Trap was discarded: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 1, "and one card from the hand: {}", s.context());
    assert_eq!(s.count(1, "mzone", FILLER), 1, "{}", s.context());
    assert_eq!(s.state()[1]["hand"].as_array().unwrap().len(), 5 + 1 - 1 - 1, "drawn, summoned and discarded: {}", s.context());
    assert_eq!(s.state()[1]["deck"].as_i64(), Some(40 - 5 - 1 - 1), "{}", s.context());
}

/// Tricolor Illusion: "Monster: Your opponent adds that card to their hand, then returns 1 other card from their hand to the
/// top of their Deck." Dark Magician on top: it joins the hand, a Feral Imp goes back on the Deck (its size is as after the
/// draw alone).
#[test]
fn tricolor_illusion_swaps_a_revealed_monster_for_another_card_of_the_hand() {
    let Some(mut s) = tricolor_duel(&[DARK_MAGICIAN], &[]) else { return };
    s.run_both([&[EndTurn], &[SetMonster(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", DARK_MAGICIAN), 1, "Dark Magician is in the hand: {}", s.context());
    assert_eq!(s.state()[1]["deck"].as_i64(), Some(40 - 5 - 1), "one card went back: {}", s.context());
    assert_eq!(s.state()[1]["hand"].as_array().unwrap().len(), 5 + 1 + 1 - 1 - 1, "{}", s.context());
}

/// Tricolor Illusion: "Your opponent can send the top card of their Deck to the Graveyard to destroy this face-up card."
/// Team 1 does that in its Main Phase, after the Standby Phase's reveal: the top card goes to its Graveyard, the Trap to mine,
/// and Warwolf attacks me directly.
#[test]
fn the_opponent_may_mill_a_card_to_destroy_tricolor_illusion() {
    let tricolor = card("Tricolor Illusion");
    let Some(mut s) = tricolor_duel(&[FILLER, DIAN_KETO], &[WARWOLF]) else { return };
    s.run_both([&[EndTurn], &[Summon(WARWOLF), Activate(tricolor), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", tricolor), 1, "Tricolor Illusion was destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 1, "the milled card: {}", s.context());
    assert_eq!(s.count(1, "hand", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "Warwolf attacked: {}", s.context());
}

/// Tricolor Illusion, Set and activated in answer to team 1's first Spell, so that its first reveal is in the End Phase
/// (after the Standby Phase): team 1 has played its whole hand (five Dian Ketos and a Feral Imp) by then. ProjectIgnis' script
/// needed no override: the empty choice is an empty group, not an error. Whatever the revealed card
/// asks of the hand (a Trap: "discards that card and 1 card from their hand"; a Monster: "returns 1 other card from their
/// hand to the top of their Deck"), an empty hand leaves nothing to choose and the effect must not stop on an error.
#[test]
fn tricolor_illusion_in_the_end_phase_with_nothing_left_in_the_hand() {
    let tricolor = card("Tricolor Illusion");
    for (top, name) in [(MIRROR_FORCE, "Trap"), (DARK_MAGICIAN, "Monster"), (DIAN_KETO, "Spell")] {
        let mut theirs: Vec<u16> = vec![FILLER; 40 - 7];
        theirs.extend([top, FILLER]);
        theirs.extend([DIAN_KETO; 5]);
        let Some(mut s) = decks(7, &[FILLER; 40], &theirs, &[set(0, tricolor)]) else { return };
        keep_hands(&mut s);
        // The Standby Phase has passed when team 1's first Dian Keto is played; the Trap answers the second.
        s.run_both([&[EndTurn], &[Activate(DIAN_KETO)]]).unwrap();
        let plan = [Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Activate(DIAN_KETO), Summon(FILLER)];
        s.run_both([&[Chain(tricolor)], &plan]).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(s.state()[1]["hand"].as_array().unwrap().len(), 0, "{name}: nothing is left in the hand: {}", s.context());
        s.run_both([&[], &[EndTurn]]).unwrap_or_else(|e| panic!("{name}: {e}"));
        clean(&s);
        let hand = s.state()[1]["hand"].as_array().unwrap().len();
        match name {
            "Trap" => assert_eq!((s.count(1, "grave", MIRROR_FORCE), hand), (1, 0), "the Trap is discarded, no second card: {}", s.context()),
            _ => assert_eq!((s.count(1, "hand", top), hand), (1, 1), "{name} joined the hand, nothing went back: {}", s.context()),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Decago Knight, Triangle Knight (engine/overrides/c511002948.lua), Hexa Knight
// ---------------------------------------------------------------------------------------------------------------

/// Decago Knight: "If there are 3 "Decago Knight" in your Graveyard: You can shuffle all "Decago Knight" from your Graveyard
/// into the Deck." With two in the Graveyard it is not offered; with three, all of them (a fourth too, from the Graveyard
/// of a later duel) go back to the Deck (+3 cards) and the Graveyard holds none.
#[test]
fn decago_knight_shuffles_three_of_itself_from_the_graveyard_into_the_deck() {
    let decago = card("Decago Knight");
    let Some(mut s) = field(&[], &[grave(0, decago), grave(0, decago)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&decago.into()), "two are too few: {:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[grave(0, decago), grave(0, decago), grave(0, decago), grave(0, FILLER)]) else { return };
    s.run(&[]).unwrap();
    let deck = s.state()[0]["deck"].as_i64().unwrap();
    assert!(offered(&s, "activates").contains(&decago.into()), "{:?}", offered(&s, "activates"));
    s.run(&[Activate(decago)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", decago), 0, "{}", s.context());
    assert_eq!(s.state()[0]["deck"].as_i64(), Some(deck + 3), "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "the other card stays: {}", s.context());
}

/// A duel with Triangle Knight (Level 3, 900/900) and four Feral Imps on my field (every Monster Zone), Thunder Kids in my
/// Graveyard, and Warwolf (2000/100) on team 1's.
fn triangle_duel(kids: usize) -> Option<Scenario> {
    let triangle = card("Triangle Knight");
    let mut spawns = vec![mon(0, triangle), mon(0, FILLER), mon(0, FILLER), mon(0, FILLER), mon(0, FILLER), mon(1, WARWOLF)];
    spawns.extend((0..kids).map(|_| grave(0, THUNDER_KID)));
    let mut s = field(&[], &spawns)?;
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (0, triangle)]);
    Some(s)
}

/// Triangle Knight: "When this card is destroyed by battle and sent to the Graveyard: You can target 3 cards in your
/// Graveyard; shuffle those cards into the Deck." With all five Monster Zones full (one free after it is destroyed) the
/// effect still works: Triangle Knight attacks Warwolf, is destroyed, and with the two Thunder Kids in my Graveyard it
/// goes back to the Deck itself (the Deck has 3 cards more, the Graveyard none of them). ProjectIgnis' script wanted two
/// free Zones, a Special Summon's condition the text does not have.
#[test]
fn triangle_knight_shuffles_three_cards_back_with_every_zone_full() {
    let triangle = card("Triangle Knight");
    let Some(mut s) = triangle_duel(2) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let deck = s.state()[0]["deck"].as_i64().unwrap();
    s.run_both([&[Attack(triangle), Chain(triangle)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1100, "2000 against 900: {}", s.context());
    assert_eq!((s.count(0, "grave", triangle), s.count(0, "grave", THUNDER_KID)), (0, 0), "all three were shuffled away: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_i64(), Some(deck + 3), "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 4, "{}", s.context());
}

/// Triangle Knight: "target 3 cards in your Graveyard": with only one other card there (two in all) the effect is not
/// offered, and Triangle Knight stays in the Graveyard.
#[test]
fn triangle_knight_needs_three_cards_in_the_graveyard() {
    let triangle = card("Triangle Knight");
    let Some(mut s) = triangle_duel(1) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let _ = s.run_both([&[Attack(triangle), Chain(triangle)], &[]]);
    clean(&s);
    assert_eq!((s.count(0, "grave", triangle), s.count(0, "grave", THUNDER_KID)), (1, 1), "{}", s.context());
}

/// Hexa Knight: "If your opponent controls no monsters, you can Normal Summon this card without Tributing." Level 6: with
/// team 1's field empty it is Summoned without Tribute and my Feral Imp stays; with a monster of team 1's it needs a Tribute
/// (not offered with no monster of mine, offered with one).
#[test]
fn hexa_knight_is_summoned_without_tribute_while_the_opponent_has_no_monsters() {
    let hexa = card("Hexa Knight");
    let Some(mut s) = field(&[], &[hand(0, hexa), mon(0, FILLER)]) else { return };
    s.choose_options(&[1]); // "Normal Summon without Tributes", the second of the two ways
    s.run(&[Summon(hexa)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", hexa), s.count(0, "mzone", FILLER), s.count(0, "grave", FILLER)), (1, 1, 0), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, hexa), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "summons").contains(&hexa.into()), "a Level 6 needs a Tribute: {:?}", offered(&s, "summons"));
    let Some(mut s) = field(&[], &[hand(0, hexa), mon(0, FILLER), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "summons").contains(&hexa.into()), "with a Tribute it can: {:?}", offered(&s, "summons"));
}

// ---------------------------------------------------------------------------------------------------------------
// Liberated Rear Warrior
// ---------------------------------------------------------------------------------------------------------------

/// Liberated Rear Warrior: "You take no Battle Damage from battles involving Warrior-Type monsters you control." Warwolf
/// (2000) attacks one of my two monsters, Dark Blade (Warrior, 1800) or Liberated Rear Warrior (Warrior, 0 ATK): no damage
/// either way; with Feral Imps (1300 ATK) in place of the Warriors the same attack hurts.
#[test]
fn liberated_rear_warrior_spares_me_the_battle_damage_of_my_warriors() {
    let rear = card("Liberated Rear Warrior");
    let Some(mut s) = field(&[], &[mon(0, rear), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert!(s.count(0, "grave", rear) + s.count(0, "grave", DARK_BLADE) >= 1, "the battle happened: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, FILLER), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 700, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Fair Exchange
// ---------------------------------------------------------------------------------------------------------------

/// Fair Exchange: "Draw 3 cards, and if you do, send them all to the Graveyard, then if any monsters were sent to the Graveyard
/// by this effect, destroy cards from the top of your opponent's Deck equal to the combined Levels of those monsters." The
/// three cards on top of my Deck are Dark Magician (Level 7), Dian Keto and a Feral Imp (Level 4): all three go to my
/// Graveyard, the Spell's Level counts for nothing and 11 cards from the top of team 1's Deck are destroyed (35 to 24).
#[test]
fn fair_exchange_mills_the_opponent_by_the_levels_of_the_monsters_it_drew() {
    let fair = card("Fair Exchange");
    // Bottom first: the three cards, the turn's draw, then the opening hand with Fair Exchange.
    let mut mine: Vec<u16> = vec![FILLER; 40 - 3 - 1 - 5];
    mine.extend([DIAN_KETO, FILLER, DARK_MAGICIAN, FILLER, fair, FILLER, FILLER, FILLER, FILLER]);
    let Some(mut s) = decks(7, &mine, &[FILLER; 40], &[]) else { return };
    keep_hands(&mut s);
    s.run(&[]).unwrap();
    let (hand, deck, theirs) = (s.state()[0]["hand"].as_array().unwrap().len(), s.state()[0]["deck"].as_i64().unwrap(), s.state()[1]["deck"].as_i64().unwrap());
    s.run(&[Activate(fair)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", DARK_MAGICIAN), s.count(0, "grave", DIAN_KETO), s.count(0, "grave", fair)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "the Feral Imp it drew: {}", s.context());
    assert_eq!(s.state()[0]["hand"].as_array().unwrap().len(), hand - 1, "Fair Exchange left the hand, the three cards passed through it: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_i64(), Some(deck - 3), "{}", s.context());
    assert_eq!((s.state()[1]["deck"].as_i64(), s.count(1, "grave", FILLER)), (Some(theirs - 11), 11), "7 + 4 cards destroyed from the top: {}", s.context());
}

/// Fair Exchange with Spells only (three Dian Ketos on top): nothing to destroy, the opponent's Deck is as it was.
#[test]
fn fair_exchange_destroys_nothing_when_no_monster_was_drawn() {
    let fair = card("Fair Exchange");
    let mut mine: Vec<u16> = vec![FILLER; 40 - 3 - 1 - 5];
    mine.extend([DIAN_KETO, DIAN_KETO, DIAN_KETO, FILLER, fair, FILLER, FILLER, FILLER, FILLER]);
    let Some(mut s) = decks(7, &mine, &[FILLER; 40], &[]) else { return };
    keep_hands(&mut s);
    s.run(&[]).unwrap();
    let theirs = s.state()[1]["deck"].as_i64().unwrap();
    s.run(&[Activate(fair)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 3, "{}", s.context());
    assert_eq!((s.state()[1]["deck"].as_i64(), s.count(1, "grave", FILLER)), (Some(theirs), 0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Master and Servant's Resolve
// ---------------------------------------------------------------------------------------------------------------

const MAN_EATER_BUG: u16 = 4507; // FLIP: target 1 monster on the field; destroy it. 450/600

/// A duel with a face-up Master and Servant's Resolve and Dark Blade on my field, Thunder Kid too, and a Set Man-Eater Bug on
/// team 1's (its flip effect destroys the monster the planned side prefers: Dark Blade).
fn resolve_duel() -> Option<Scenario> {
    let resolve = card("Master and Servant's Resolve");
    let mut s = field(&[], &[spell(0, resolve), mon(0, DARK_BLADE), mon(0, THUNDER_KID), face_down(1, MAN_EATER_BUG)])?;
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE), (1, MAN_EATER_BUG)]);
    Some(s)
}

/// Master and Servant's Resolve: "During the Battle Phase, when a monster is destroyed by an Effect Monster's effect, the
/// destroyed monster's controller takes damage equal to its ATK. During each of your Standby Phases, take 1000 damage or destroy
/// this card." Thunder Kid attacks the Set Man-Eater Bug in turn 3; its flip destroys my Dark Blade (1800) and I take 1800;
/// the Bug is destroyed by battle, which is not an effect (no damage for it). Paying the 1000 at each of my Standby Phases
/// (turns 1 and 3, the duel's first turn has one too) keeps the Trap: 8000 - 1000 - 1000 - 1800.
#[test]
fn master_and_servants_resolve_damages_the_controller_of_a_monster_destroyed_by_an_effect_in_the_battle_phase() {
    let resolve = card("Master and Servant's Resolve");
    let Some(mut s) = resolve_duel() else { return };
    s.choose_options(&[0, 0]); // take 1000 damage, twice
    s.run_both([&[EndTurn, Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the flip effect destroyed Dark Blade: {}", s.context());
    assert_eq!(s.count(1, "grave", MAN_EATER_BUG), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1000 - 1000 - 1800, 8000), "the two Standby Phases' 1000 and Dark Blade's ATK: {}", s.context());
    assert_eq!(s.count(0, "szone", resolve), 1, "{}", s.context());
}

/// Master and Servant's Resolve: "take 1000 damage or destroy this card": choosing to destroy it costs no LP and ends the
/// damage for the destroyed monster: paying in turn 1 and destroying it in turn 3's Standby Phase leaves Dark Blade's
/// destruction (by the flip, in the Battle Phase) without damage.
#[test]
fn master_and_servants_resolve_can_be_destroyed_at_my_standby_phase_instead_of_paying() {
    let resolve = card("Master and Servant's Resolve");
    let Some(mut s) = resolve_duel() else { return };
    s.choose_options(&[0, 1]); // take 1000 damage in turn 1, destroy this card in turn 3
    s.run_both([&[EndTurn, Attack(THUNDER_KID)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", resolve), 1, "it destroyed itself: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000, "turn 1's 1000 only, no damage for Dark Blade: {}", s.context());
}

/// Master and Servant's Resolve, outside the Battle Phase: team 1 Flip Summons its Man-Eater Bug in its Main Phase and
/// destroys Dark Blade: no damage.
#[test]
fn master_and_servants_resolve_does_nothing_outside_the_battle_phase() {
    let Some(mut s) = resolve_duel() else { return };
    s.run_both([&[EndTurn], &[Reposition(MAN_EATER_BUG)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000, "turn 1's 1000 only: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Reversal Sword (engine/overrides/c511000946.lua)
// ---------------------------------------------------------------------------------------------------------------

/// Reversal Sword: "Activate only when an opponent's monster was destroyed by battle. Target 1 face-up monster on the field
/// with a lower Level than the destroyed monster's, and destroy it. Then, both players take damage equal to the ATK of the
/// destroyed monster." Dark Magician (2500) destroys Warwolf (Level 4, 2000) in battle (500 damage) and the Sword destroys my
/// Thunder Kid (Level lower than 4): both players take 2000, the destroyed monster's ATK, not Thunder Kid's 700 (ProjectIgnis'
/// script dealt the ATK of the monster it destroyed).
#[test]
fn reversal_sword_destroys_a_lower_level_monster_and_damages_both_by_the_destroyed_monsters_atk() {
    let sword = card("Reversal Sword");
    let Some(mut s) = field(&[], &[set(0, sword), mon(0, DARK_MAGICIAN), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[EndTurn, Attack(DARK_MAGICIAN), Chain(sword)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", sword), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 2000, 8000 - 500 - 2000), "2000 each: {}", s.context());
}

/// Reversal Sword: "a lower Level than the destroyed monster's": a destroyed Level 4 monster leaves only monsters of Level 3
/// or less as targets. With a Level 4 Feral Imp as the only other monster the Trap is not offered.
#[test]
fn reversal_sword_needs_a_monster_of_a_lower_level() {
    let sword = card("Reversal Sword");
    let Some(mut s) = field(&[], &[set(0, sword), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let _ = s.run(&[EndTurn, Attack(DARK_MAGICIAN), Chain(sword)]);
    assert_eq!(s.count(0, "grave", sword), 0, "no monster of a lower Level but Dark Magician (7): {}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Different Dimension Barrier - Lost Force
// ---------------------------------------------------------------------------------------------------------------

const FISSURE: u16 = 4835; // Normal Spell: destroy the face-up monster with the lowest ATK your opponent controls

/// Different Dimension Barrier - Lost Force: "When an opponent's monster declares an attack during the turn a monster leaves
/// your side of the field, except by battle: negate the attack. Then, inflict damage to your opponent equal to the ATK of the
/// monster that attacked." Team 1's Fissure destroys my Feral Imp in turn 2 and its Warwolf (2000) attacks directly: the attack
/// is negated (no damage to me) and team 1 takes 2000.
#[test]
fn lost_force_negates_an_attack_in_a_turn_a_monster_left_my_side_and_damages_by_its_atk() {
    let barrier = card("Different Dimension Barrier - Lost Force");
    let Some(mut s) = field(&[], &[set(0, barrier), mon(0, FILLER), hand(1, FISSURE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(barrier)], &[Activate(FISSURE), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FILLER), 1, "Fissure destroyed the Imp: {}", s.context());
    assert_eq!(s.count(0, "grave", barrier), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2000), "the attack was negated and 2000 came back: {}", s.context());
}

/// Lost Force: "during the turn a monster leaves your side": with no monster gone in turn 2 the Barrier is not offered and the
/// direct attack hurts; a monster that was destroyed in battle does not count either (Warwolf destroys my Imp by attacking it and
/// a second attacker, Dark Blade, finds the Barrier shut).
#[test]
fn lost_force_needs_a_monster_that_left_my_side_not_by_battle() {
    let barrier = card("Different Dimension Barrier - Lost Force");
    let Some(mut s) = field(&[], &[set(0, barrier), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let _ = s.run_both([&[ChainOnAttack(barrier)], &[Attack(WARWOLF)]]);
    assert_eq!((s.count(0, "grave", barrier), s.lp(0)), (0, 8000 - 2000), "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, barrier), mon(0, FILLER), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let _ = s.run_both([&[ChainOnAttack(barrier)], &[Attack(WARWOLF), Attack(DARK_BLADE)]]);
    assert_eq!(s.count(0, "grave", barrier), 0, "a monster destroyed in battle is no reason: {}", s.context());
    assert_eq!((s.count(0, "mzone", FILLER), s.lp(0)), (0, 8000 - 700 - 1800), "the Imp died in battle, Dark Blade's direct attack hit: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Magic Charge
// ---------------------------------------------------------------------------------------------------------------

const SWORDS_OF_REVEALING_LIGHT: u16 = 4354;

/// Magic Charge: "When a Spell you control is destroyed by an opponent's card effect: Both players take 2000 damage." Team 1's
/// Mystical Space Typhoon destroys my face-up Swords of Revealing Light: both take 2000.
#[test]
fn magic_charge_hurts_both_players_when_my_spell_is_destroyed_by_the_opponent() {
    let charge = card("Magic Charge");
    let Some(mut s) = field(&[], &[set(0, charge), spell(0, SWORDS_OF_REVEALING_LIGHT), hand(1, MYSTICAL_SPACE_TYPHOON)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, SWORDS_OF_REVEALING_LIGHT)]);
    s.run_both([&[Chain(charge)], &[Activate(MYSTICAL_SPACE_TYPHOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SWORDS_OF_REVEALING_LIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", charge), 1, "Magic Charge was activated: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 2000, 8000 - 2000), "{}", s.context());
}

/// Magic Charge: "a Spell you control ... by an opponent's card effect": a face-down Spell destroyed, or my own Spell
/// destroyed by my own Mystical Space Typhoon, does not start it.
#[test]
fn magic_charge_ignores_a_face_down_spell_and_my_own_effects() {
    let charge = card("Magic Charge");
    let Some(mut s) = field(&[], &[set(0, charge), set(0, DIAN_KETO), hand(1, MYSTICAL_SPACE_TYPHOON)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DIAN_KETO)]);
    let _ = s.run_both([&[Chain(charge)], &[Activate(MYSTICAL_SPACE_TYPHOON)]]);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "the Set Spell was destroyed: {}", s.context());
    assert_eq!((s.count(0, "grave", charge), s.lp(0), s.lp(1)), (0, 8000, 8000), "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, charge), spell(0, SWORDS_OF_REVEALING_LIGHT), hand(0, MYSTICAL_SPACE_TYPHOON)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, SWORDS_OF_REVEALING_LIGHT)]);
    let _ = s.run_both([&[Activate(MYSTICAL_SPACE_TYPHOON), Chain(charge)], &[]]);
    assert_eq!(s.count(0, "grave", SWORDS_OF_REVEALING_LIGHT), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", charge), s.lp(0), s.lp(1)), (0, 8000, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dimension Trap
// ---------------------------------------------------------------------------------------------------------------

const JUST_DESSERTS: u16 = 4865; // Trap: 500 damage for each monster the opponent controls

/// Dimension Trap: "Banish 1 monster you control and 1 Trap Card in your Graveyard. The effect of this card will be the same as
/// that Trap Card." My Feral Imp and Just Desserts (500 damage for each monster the opponent controls) are banished and
/// team 1's two monsters cost it 1000; Dimension Trap itself goes to the Graveyard.
#[test]
fn dimension_trap_banishes_a_monster_and_a_trap_and_copies_the_trap() {
    let dim = card("Dimension Trap");
    let Some(mut s) = field(&[], &[set(0, dim), mon(0, FILLER), grave(0, JUST_DESSERTS), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER), (0, JUST_DESSERTS)]);
    s.run_both([&[EndTurn, Activate(dim)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", FILLER), s.count(0, "removed", JUST_DESSERTS)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", dim), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1000), "Just Desserts' effect, 500 for each of 2 monsters: {}", s.context());
}

/// Dimension Trap: with no Trap Card in my Graveyard, or no monster of mine, it is not offered.
#[test]
fn dimension_trap_needs_a_monster_and_a_trap_in_the_graveyard() {
    let dim = card("Dimension Trap");
    let Some(mut s) = field(&[], &[set(0, dim), mon(0, FILLER), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(dim)], &[EndTurn]]).expect_err("no Trap in the Graveyard");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = field(&[], &[set(0, dim), grave(0, JUST_DESSERTS), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(dim)], &[EndTurn]]).expect_err("no monster of mine");
    assert!(err.contains("never became possible"), "{err}");
}

/// Dimension Trap with a second Trap in the Graveyard that cannot be activated now (Mirror Force answers an attack):
/// it is not one to choose, so the cost banishes Just Desserts whichever the player prefers, and the effect is not lost.
#[test]
fn dimension_trap_leaves_out_a_trap_that_could_not_be_activated_now() {
    let dim = card("Dimension Trap");
    let spawns = [set(0, dim), mon(0, FILLER), grave(0, MIRROR_FORCE), grave(0, JUST_DESSERTS), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER), (0, MIRROR_FORCE), (0, JUST_DESSERTS)]);
    s.run_both([&[EndTurn, Activate(dim)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", JUST_DESSERTS), s.count(0, "removed", MIRROR_FORCE), s.count(0, "grave", MIRROR_FORCE)), (1, 0, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Level Change
// ---------------------------------------------------------------------------------------------------------------

/// Level Change: "During your opponent's End Phase: Send 1 face-up "LV" monster you control to the Graveyard; Special Summon 1
/// monster from your Graveyard whose name is written on that card, ignoring its Summoning conditions." Armed Dragon LV3 goes
/// to the Graveyard in team 1's End Phase and Armed Dragon LV5 (whose name is on it) comes from there; until the End Phase
/// nothing happened.
#[test]
fn level_change_swaps_an_lv_monster_for_the_one_named_on_it_in_the_opponents_end_phase() {
    let [change, lv3, lv5] = [card("Level Change"), card("Armed Dragon LV3"), card("Armed Dragon LV5")];
    let Some(mut s) = field(&[], &[set(0, change), mon(0, lv3), grave(0, lv5)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[SetMonster(FILLER)]]).unwrap();
    assert_eq!((s.count(0, "mzone", lv3), s.count(0, "grave", lv5)), (1, 1), "not before the End Phase: {}", s.context());
    s.run_both([&[Chain(change)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", lv5), s.count(0, "grave", lv3), s.count(0, "grave", change)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", lv5), 0, "{}", s.context());
}

/// Level Change: "Special Summon 1 monster from your Graveyard whose name is written on that card": without Armed Dragon LV5
/// in the Graveyard (a Feral Imp there is no one named on the card) the Trap is not offered.
#[test]
fn level_change_needs_the_named_monster_in_the_graveyard() {
    let [change, lv3] = [card("Level Change"), card("Armed Dragon LV3")];
    let Some(mut s) = field(&[], &[set(0, change), mon(0, lv3), grave(0, FILLER)]) else { return };
    keep_hands(&mut s);
    let _ = s.run_both([&[Chain(change)], &[EndTurn]]);
    assert_eq!((s.count(0, "mzone", lv3), s.count(0, "grave", change)), (1, 0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Delta Reactor
// ---------------------------------------------------------------------------------------------------------------

/// Delta Reactor: "Send 1 "Trap Reactor・Y FI", "Spell Reactor・RE", and "Summon Reactor・SK" you control to the Graveyard;
/// Special Summon "Flying Fortress SKY FIRE" from your hand, Deck, or Graveyard, ignoring its Summoning conditions. Once per
/// turn, if it would be destroyed, it is not destroyed." The three Reactors go to the Graveyard and SKY FIRE (3000/2500, from the
/// hand) takes their place; Dark Hole cannot destroy it the first time in that turn (Warwolf of team 1 is destroyed and the
/// Feral Imp too), but the second Dark Hole does.
#[test]
fn delta_reactor_brings_flying_fortress_sky_fire_that_survives_one_destruction_a_turn() {
    let [delta, trap_r, spell_r, summon_r, fire] = ["Delta Reactor", "Trap Reactor ・Y FI", "Spell Reactor ・RE", "Summon Reactor ・SK", "Flying Fortress SKY FIRE"].map(card);
    let spawns = [set(0, delta), mon(0, trap_r), mon(0, spell_r), mon(0, summon_r), hand(0, fire), hand(0, DARK_HOLE), hand(0, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(delta)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", trap_r), s.count(0, "grave", spell_r), s.count(0, "grave", summon_r)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.count(0, "mzone", fire), s.count(0, "hand", fire)), (1, 0), "SKY FIRE came from the hand: {}", s.context());
    assert_eq!(s.attack_of(0, fire), Some(3000), "{}", s.context());
    s.run_both([&[EndTurn, Activate(DARK_HOLE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole resolved: {}", s.context());
    assert_eq!(s.count(0, "mzone", fire), 1, "SKY FIRE was not destroyed: {}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", fire), 1, "the second destruction in a turn works: {}", s.context());
}

/// Delta Reactor: each of the three Reactors is needed. Without Summon Reactor ・SK it is not offered, and SKY FIRE stays in
/// the hand.
#[test]
fn delta_reactor_needs_all_three_reactors() {
    let [delta, trap_r, spell_r, fire] = ["Delta Reactor", "Trap Reactor ・Y FI", "Spell Reactor ・RE", "Flying Fortress SKY FIRE"].map(card);
    let Some(mut s) = field(&[], &[set(0, delta), mon(0, trap_r), mon(0, spell_r), hand(0, fire)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(delta)], &[EndTurn]]).expect_err("only two Reactors");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "hand", fire), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Martyr Flag
// ---------------------------------------------------------------------------------------------------------------

/// Martyr Flag: "During your opponent's Battle Phase: Tribute 1 monster or send 1 monster from your hand to the Graveyard;
/// double the ATK of all Warrior-Type monsters you control until the End Phase." Dark Blade is my only Warrior, so the cost is a
/// Feral Imp of my hand: Warwolf (2000) attacks Dark Blade, which is 3600 by then and destroys it (1600 damage to team 1);
/// in my next turn Dark Blade is back at 1800.
#[test]
fn martyr_flag_doubles_my_warriors_for_the_opponents_battle_phase() {
    let flag = card("Martyr Flag");
    let Some(mut s) = field(&[], &[set(0, flag), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let imps = s.count(0, "grave", FILLER);
    s.run_both([&[Chain(flag)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", flag), 1, "Martyr Flag was activated: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), imps + 1, "a monster of my hand was the cost: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "3600 against 2000: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1600), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(1800), "until the End Phase: {}", s.context());
}

/// Martyr Flag, the other cost: with no monster in my hand (a Deck of Spells) a monster on the field is Tributed, and it may not
/// be the only Warrior (the others must stay to be doubled): with Dark Blade and a Feral Imp the Imp is the Tribute and Dark
/// Blade is doubled, 3600, whatever Warwolf attacks. With Dark Blade alone and no monster in the hand there is no cost.
#[test]
fn martyr_flag_tributes_a_monster_when_the_hand_has_none() {
    let flag = card("Martyr Flag");
    let spells = [DIAN_KETO; 40];
    let spawns = [set(0, flag), mon(0, DARK_BLADE), mon(0, FILLER), mon(1, WARWOLF)];
    let Some(mut s) = decks(7, &spells, &[FILLER; 40], &spawns) else { return };
    s.run_both([&[Chain(flag)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", flag), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 0, "the Imp was Tributed or destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade (3600) is no one's prey: {}", s.context());
    let spawns = [set(0, flag), mon(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = decks(7, &spells, &[FILLER; 40], &spawns) else { return };
    let _ = s.run_both([&[Chain(flag)], &[Attack(WARWOLF)]]);
    assert_eq!(s.count(0, "grave", flag), 0, "the only Warrior cannot be the cost: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Damage Summon
// ---------------------------------------------------------------------------------------------------------------

/// Damage Summon: "At the end of the Damage Step, when a monster you control attacks and the attack target is not destroyed by
/// battle: Pay 800 LP; Special Summon 1 Level 4 or lower monster from your hand." Thunder Kid (700) attacks Dark Blade (1800),
/// which survives: I lose 1100 in battle, 800 for the Trap, and a Feral Imp is Special Summoned from my hand.
#[test]
fn damage_summon_pays_800_for_a_monster_from_the_hand_when_the_attack_target_survives() {
    let summon = card("Damage Summon");
    let Some(mut s) = field(&[], &[set(0, summon), mon(0, THUNDER_KID), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER)]);
    s.run(&[EndTurn, Attack(THUNDER_KID), Chain(summon)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", summon), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1100 - 800, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "a Feral Imp came from the hand: {}", s.context());
    assert_eq!(s.state()[0]["hand"].as_array().unwrap().len(), 5 + 1 + 1 - 1, "5 and the draws of turns 1 and 3, less the Imp: {}", s.context());
}

/// Damage Summon: "the attack target is not destroyed by battle": Dark Magician (2500) destroys Dark Blade (1800) and the Trap
/// finds no chance; a direct attack has no target either.
#[test]
fn damage_summon_is_not_offered_when_the_attack_target_is_destroyed() {
    let summon = card("Damage Summon");
    let Some(mut s) = field(&[], &[set(0, summon), mon(0, DARK_MAGICIAN), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let _ = s.run(&[EndTurn, Attack(DARK_MAGICIAN), Chain(summon)]);
    assert_eq!((s.count(0, "grave", summon), s.lp(0), s.lp(1)), (0, 8000, 8000 - 700), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Exus Summon
// ---------------------------------------------------------------------------------------------------------------

/// Exus Summon: "When a monster you control is targeted for an attack: Return that monster to the hand, then Special Summon 1
/// monster from your hand in Attack Position, whose ATK is lower than the returned monster's ATK." Team 1's Thunder Kid
/// attacks my Dark Blade (1800): it goes back to my hand and a Feral Imp (1300) takes its place in Attack Position.
#[test]
fn exus_summon_returns_the_attacked_monster_and_puts_a_weaker_one_in_its_place() {
    let exus = card("Exus Summon");
    let Some(mut s) = field(&[], &[set(0, exus), mon(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER)]);
    s.run_both([&[ChainOnAttack(exus)], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", exus), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", DARK_BLADE), s.count(0, "mzone", DARK_BLADE)), (1, 0), "Dark Blade is back in my hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "a Feral Imp stands in its place: {}", s.context());
    assert_eq!(s.position_of(0, FILLER), Some(1), "in Attack Position: {}", s.context());
}

/// Exus Summon: "whose ATK is lower than the returned monster's": a Hand of Dark Magicians (2500) against my Thunder Kid
/// (700) brings no one, so it is not offered, and Thunder Kid dies in the battle.
#[test]
fn exus_summon_needs_a_weaker_monster_in_the_hand() {
    let exus = card("Exus Summon");
    let Some(mut s) = decks(7, &[DARK_MAGICIAN; 40], &[FILLER; 40], &[set(0, exus), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    let _ = s.run_both([&[ChainOnAttack(exus)], &[Attack(WARWOLF)]]);
    assert_eq!((s.count(0, "grave", exus), s.count(0, "mzone", THUNDER_KID)), (0, 0), "Thunder Kid (700) died in battle: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Sky A3
// ---------------------------------------------------------------------------------------------------------------

/// Sky A3: "Send 1 face-up "Skiel Attack" monster you control to the Graveyard; Special Summon 1 "Skiel Attack 3" from your
/// hand." The Skiel Attacks live only while a "Meklord Emperor" monster is on the field (the fuzz's random duels had none, so
/// every one of them was destroyed at once): with Meklord Emperor Granel on my field Skiel Attack goes to the Graveyard and
/// Skiel Attack 3 (1200 ATK) comes from my hand.
#[test]
fn sky_a3_swaps_skiel_attack_for_skiel_attack_3() {
    let [sky, attack, attack3, granel] = [card("Sky A3"), card("Skiel Attack (TF6)"), card("Skiel Attack 3 (TF6)"), card("Meklord Emperor Granel ∞")];
    let Some(mut s) = field(&[], &[set(0, sky), mon(0, granel), mon(0, attack), hand(0, attack3)]) else { return };
    keep_hands(&mut s);
    assert_eq!(s.count(0, "mzone", attack), 1, "Skiel Attack lives with an Emperor on the field: {}", s.context());
    s.run_both([&[EndTurn, Activate(sky)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", attack), s.count(0, "mzone", attack3), s.count(0, "hand", attack3)), (1, 1, 0), "{}", s.context());
    assert_eq!(s.attack_of(0, attack3), Some(1200), "{}", s.context());
    assert_eq!(s.count(0, "grave", sky), 1, "{}", s.context());
}

/// Sky A3: "1 face-up "Skiel Attack" monster": a field without one (Meklord Emperor Granel alone) leaves it unoffered.
#[test]
fn sky_a3_needs_a_skiel_attack_monster() {
    let [sky, attack3, granel] = [card("Sky A3"), card("Skiel Attack 3 (TF6)"), card("Meklord Emperor Granel ∞")];
    let Some(mut s) = field(&[], &[set(0, sky), mon(0, granel), hand(0, attack3)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(sky)], &[EndTurn]]).expect_err("no Skiel Attack");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Infinite Aura
// ---------------------------------------------------------------------------------------------------------------

const OOKAZI: u16 = 4352; // 800 damage
const AURA_COUNTER: &str = "4352"; // the Aura Counter's id (0x1100) in the engine's field

/// The Aura Counters on a card of a team's Spell & Trap Zone.
fn aura_counters(s: &Scenario, team: usize, cid: u16) -> i64 {
    s.state()[team]["szone"].as_array().unwrap().iter().find(|c| c["cid"] == cid).map_or(-1, |c| c["counters"][AURA_COUNTER].as_i64().unwrap_or(0))
}

/// Infinite Aura: "Each time you take damage, place 1 Aura Counter on this card for each multiple of 100 damage you took."
/// Warwolf's direct attack (2000) puts 20 counters on it, and team 1's Ookazi (800 damage) 8 more: 28 in all, battle or effect
/// damage alike (the counters reach the client's view: `clean`).
#[test]
fn infinite_aura_counts_each_hundred_damage_i_take() {
    let aura = card("Infinite Aura");
    let Some(mut s) = field(&[], &[spell(0, aura), mon(1, WARWOLF), hand(1, OOKAZI)]) else { return };
    keep_hands(&mut s);
    assert_eq!(aura_counters(&s, 0, aura), 0, "{}", s.context());
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), aura_counters(&s, 0, aura)), (8000 - 2000, 20), "{}", s.context());
    s.run_both([&[], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), aura_counters(&s, 0, aura)), (8000 - 2000 - 800, 28), "{}", s.context());
}

/// Infinite Aura: "each multiple of 100": 850 damage is 8 counters, and damage to the opponent places none.
#[test]
fn infinite_aura_counts_whole_hundreds_and_only_my_damage() {
    let aura = card("Infinite Aura");
    let Some(mut s) = field(&[], &[spell(0, aura), mon(0, THUNDER_KID), mon(1, WARWOLF), hand(0, OOKAZI)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(OOKAZI)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1), aura_counters(&s, 0, aura)), (8000, 8000 - 800, 0), "damage to the opponent: no counter: {}", s.context());
    // Thunder Kid (700) attacks Dark Blade... : 1800 - 700 = 1100 of battle damage is 11 counters.
    let Some(mut s) = field(&[], &[spell(0, aura), mon(0, THUNDER_KID), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Attack(THUNDER_KID)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), aura_counters(&s, 0, aura)), (8000 - 1100, 11), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Infinity Barrage
// ---------------------------------------------------------------------------------------------------------------

/// Infinity Barrage: "While your opponent controls no monsters: Inflict 400 damage to your opponent for each monster you
/// control. You cannot declare an attack the turn you activate this card." Three monsters of mine and none of team 1's: 1200
/// damage, and none of my monsters can attack that turn.
#[test]
fn infinity_barrage_inflicts_400_for_each_monster_and_forbids_my_attacks() {
    let barrage = card("Infinity Barrage");
    let Some(mut s) = field(&[], &[set(0, barrage), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(0, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(barrage)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1200), "{}", s.context());
    let err = s.run(&[Attack(DARK_BLADE)]).expect_err("no attack the turn it was activated");
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
}

/// Infinity Barrage: "While your opponent controls no monsters": it is not offered against a monster. (Its cost also wants no
/// attack declared yet this turn: the Battle Phase leaves no Main Phase 2 prompt for the scenario to look at, so that half is not tested.)
#[test]
fn infinity_barrage_needs_an_empty_opposing_field() {
    let barrage = card("Infinity Barrage");
    let Some(mut s) = field(&[], &[set(0, barrage), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(barrage)], &[EndTurn]]).expect_err("Warwolf is on the field");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Cup Meatball Cayenne, Cup Ramen Lard, Cup Udon Uno
// ---------------------------------------------------------------------------------------------------------------

const UMI: u16 = 4340; // Field Spell

/// Cup Meatball Cayenne: "When this card is destroyed and sent to the Graveyard, destroy all cards in your opponent's hand."
/// Warwolf (2000) destroys my Cup (1600) in battle: team 1's whole hand goes to its Graveyard (the hand of 6, the Spell it
/// holds too).
#[test]
fn cup_meatball_cayenne_destroys_the_opponents_hand() {
    let cup = card("Cup Meatball Cayenne");
    let Some(mut s) = field(&[], &[mon(0, cup), mon(1, WARWOLF), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cup), 1, "{}", s.context());
    assert_eq!(s.state()[1]["hand"].as_array().unwrap().len(), 0, "team 1's hand was destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 400, "{}", s.context());
    assert_eq!(s.state()[0]["hand"].as_array().unwrap().len(), 5 + 1, "my own hand stays: {}", s.context());
}

/// Cup Ramen Lard: "When this card is destroyed and sent to the Graveyard, destroy all monsters your opponent controls." Warwolf
/// destroys my Cup (600) in battle and then both of team 1's monsters, Warwolf and Dark Blade, are destroyed; my other monster stays.
#[test]
fn cup_ramen_lard_destroys_all_the_opponents_monsters() {
    let cup = card("Cup Ramen Lard");
    let Some(mut s) = field(&[], &[mon(0, cup), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cup) + s.count(0, "grave", THUNDER_KID), 1, "one monster of mine died, the one attacked: {}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
}

/// Cup Udon Uno: "When this card is destroyed and sent to the Graveyard: destroy all cards in your opponent's Spell & Trap
/// Zone." My Cup (1400) is destroyed by Warwolf; team 1's Set Mirror Force and Dian Keto go, and its Field Spell Umi (in the
/// Field Zone, no Spell & Trap Zone) stays.
#[test]
fn cup_udon_uno_destroys_the_opponents_spell_and_trap_zone() {
    let cup = card("Cup Udon Uno");
    let spawns = [mon(0, cup), mon(1, WARWOLF), set(1, MIRROR_FORCE), set(1, DIAN_KETO), spell(1, UMI)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cup), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", MIRROR_FORCE), s.count(1, "grave", DIAN_KETO)), (1, 1), "{}", s.context());
    assert_eq!(s.count(1, "szone", UMI), 1, "the Field Spell stays: {}", s.context());
}

/// Cup Udon Uno, "destroyed": a Cup that is Tributed leaves no effect behind, and neither does one banished.
#[test]
fn cup_udon_uno_needs_to_be_destroyed() {
    let cup = card("Cup Udon Uno");
    let Some(mut s) = field(&[], &[mon(0, cup), mon(0, DARK_BLADE), hand(0, DARK_MAGICIAN), set(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, cup), (0, DARK_BLADE)]);
    s.prefer_most(true);
    s.run(&[Summon(DARK_MAGICIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cup), 1, "the Cup was Tributed: {}", s.context());
    assert_eq!(s.count(1, "szone", DIAN_KETO), 1, "and the Spell & Trap Zone stands: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Spirit Summit Warrior
// ---------------------------------------------------------------------------------------------------------------

/// Spirit Summit Warrior: "When this card is Normal Summoned, you can Special Summon 1 Level 3 or lower Warrior-Type monster from
/// your hand." Battle Warrior (Level 3 Warrior) comes from my hand; Dark Blade (Level 4) and Feral Imps stay.
#[test]
fn spirit_summit_warrior_summons_a_small_warrior_from_the_hand() {
    let warrior = card("Spirit Summit Warrior");
    let Some(mut s) = field(&[], &[hand(0, warrior), hand(0, BATTLE_WARRIOR), hand(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Summon(warrior), Chain(warrior)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", warrior), s.count(0, "mzone", BATTLE_WARRIOR)), (1, 1), "{}", s.context());
    assert_eq!((s.count(0, "hand", DARK_BLADE), s.count(0, "mzone", DARK_BLADE)), (1, 0), "a Level 4 Warrior is too big: {}", s.context());
    assert_eq!(s.attack_of(0, warrior), Some(1200), "{}", s.context());
}

/// Spirit Summit Warrior: with no Level 3 or lower Warrior in the hand nothing is asked and nothing comes.
#[test]
fn spirit_summit_warrior_finds_no_one_without_a_small_warrior() {
    let warrior = card("Spirit Summit Warrior");
    let Some(mut s) = field(&[], &[hand(0, warrior), hand(0, DARK_BLADE), hand(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Summon(warrior)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", warrior) + s.count(0, "hand", DARK_BLADE) + s.count(0, "hand", THUNDER_KID), 3, "{}", s.context());
    assert_eq!(s.state()[0]["mzone"].as_array().unwrap().iter().filter(|c| !c.is_null()).count(), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Monster Clock, Clockwork and the Clock Knights
// ---------------------------------------------------------------------------------------------------------------

/// Monster Clock: "When this card is targeted for an attack: You can equip the attacking monster to this card. Gains ATK equal
/// to those equipped cards' combined ATK." Warwolf (2000) attacks my Monster Clock (0 ATK): it is equipped to the Clock, which
/// gains 2000 ATK, and the attack does no damage.
#[test]
fn monster_clock_equips_the_attacker_and_gains_its_atk() {
    let clock = card("Monster Clock");
    let Some(mut s) = field(&[], &[mon(0, clock), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(clock)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "Warwolf left its side: {}", s.context());
    assert_eq!(s.count(0, "szone", WARWOLF), 1, "it is an equip card of the Clock: {}", s.context());
    assert_eq!(s.attack_of(0, clock), Some(2000), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Clockwork: "While you control a face-up "Clock Knight" monster: Special Summon 1 "Clock Knight" monster from your hand with
/// a lower Level than a face-up "Clock Knight" monster you control." Clock Knight No. 3 (Level 1) comes from my hand next to
/// Clock Knight No. 12 (Level 4); with no Clock Knight on the field, or only the same Level in the hand, it is not offered.
#[test]
fn clockwork_summons_a_clock_knight_of_a_lower_level() {
    let [clockwork, no12, no3] = ["Clockwork", "Clock Knight No. 12", "Clock Knight No. 3"].map(card);
    let Some(mut s) = field(&[], &[hand(0, clockwork), mon(0, no12), hand(0, no3)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(clockwork)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", no3), s.count(0, "mzone", no12), s.count(0, "grave", clockwork)), (1, 1, 1), "{}", s.context());
    // Without a Clock Knight, and with a same-Level Clock Knight in the hand: no.
    let Some(mut s) = field(&[], &[hand(0, clockwork), hand(0, no3)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&clockwork.into()), "no Clock Knight on the field: {:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[hand(0, clockwork), mon(0, no3), hand(0, no3)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&clockwork.into()), "the same Level is not lower: {:?}", offered(&s, "activates"));
}

/// Clock Knight No. 12: "Once per turn: You can toss a coin. If Heads, this card gains 1200 ATK until the End Phase. If Tails,
/// destroy this card." Over eight seeds heads give 2400 ATK and tails send it to the Graveyard, both come up, and a second
/// toss in a turn is not offered.
#[test]
fn clock_knight_no_12_gains_1200_atk_on_heads_and_is_destroyed_on_tails() {
    let no12 = card("Clock Knight No. 12");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=8 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[mon(0, no12)]) else { return };
        s.run(&[Activate(no12)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        match (s.attack_of(0, no12), s.count(0, "grave", no12)) {
            (Some(2400), 0) => {
                heads += 1;
                assert!(!offered(&s, "activates").contains(&no12.into()), "seed {seed}: once per turn: {}", s.context());
            }
            (None, 1) => tails += 1,
            other => panic!("seed {seed}: 2400 ATK or destroyed, got {other:?}: {}", s.context()),
        }
    }
    assert!(heads > 0 && tails > 0, "both faces in 8 seeds: heads {heads}, tails {tails}");
}

/// Clock Knight No. 3: the same effect: Level 1, 300 ATK, 1500 on heads.
#[test]
fn clock_knight_no_3_gains_1200_atk_on_heads_and_is_destroyed_on_tails() {
    let no3 = card("Clock Knight No. 3");
    let (mut heads, mut tails) = (0, 0);
    for seed in 1..=8 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[mon(0, no3)]) else { return };
        assert_eq!(s.attack_of(0, no3), Some(300), "{}", s.context());
        s.run(&[Activate(no3)]).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        clean(&s);
        match (s.attack_of(0, no3), s.count(0, "grave", no3)) {
            (Some(1500), 0) => heads += 1,
            (None, 1) => tails += 1,
            other => panic!("seed {seed}: 1500 ATK or destroyed, got {other:?}: {}", s.context()),
        }
    }
    assert!(heads > 0 && tails > 0, "both faces in 8 seeds: heads {heads}, tails {tails}");
}

/// Clock Knight: "Once per turn, you can negate the effects of a "Clock Knight" monster requiring a coin toss and redo the coin
/// toss." Clock Knight No. 12 tosses and my No. 3 answers (asked as "use this effect?", yes): the toss is made again, and No. 12
/// ends as either 2400 ATK or destroyed, in every one of six seeds.
#[test]
fn a_clock_knight_may_redo_the_coin_toss_of_another() {
    let [no12, no3] = ["Clock Knight No. 12", "Clock Knight No. 3"].map(card);
    let mut redone = 0;
    for seed in 1..=6 {
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &[mon(0, no3), mon(0, no12)]) else { return };
        s.run(&[Activate(no12), Chain(no3)]).unwrap_or_else(|e| panic!("seed {seed}: the redo was not offered: {e}"));
        clean(&s);
        redone += 1;
        match (s.attack_of(0, no12), s.count(0, "grave", no12)) {
            (Some(2400), 0) | (None, 1) => {}
            other => panic!("seed {seed}: 2400 ATK or destroyed, got {other:?}: {}", s.context()),
        }
    }
    assert_eq!(redone, 6);
}

// ---------------------------------------------------------------------------------------------------------------
// Glory Shield
// ---------------------------------------------------------------------------------------------------------------

/// Glory Shield: "If the equipped monster attacks, your opponent cannot activate Spell/Trap Cards until the end of the Damage Step.
/// When the equipped monster inflicts battle damage to your opponent: You can target 1 Spell/Trap on the field; destroy it."
/// Dark Blade, equipped with it, attacks directly in turn 3 against a Set Mirror Force that cannot be activated: team 1 takes
/// 1800 and then Mirror Force is the Spell/Trap I destroy. Without Glory Shield (the control) Mirror Force destroys Dark Blade.
#[test]
fn glory_shield_keeps_the_opponents_traps_shut_and_destroys_one_after_battle_damage() {
    let shield = card("Glory Shield");
    let Some(mut s) = field(&[], &[hand(0, shield), mon(0, DARK_BLADE), set(1, MIRROR_FORCE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE), (1, MIRROR_FORCE)]);
    s.run(&[Activate(shield), EndTurn, Attack(DARK_BLADE), Chain(shield)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1800, "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Mirror Force could not be activated: {}", s.context());
    assert_eq!(s.count(1, "grave", MIRROR_FORCE), 1, "and was destroyed by Glory Shield: {}", s.context());
    assert_eq!(s.count(0, "szone", shield), 1, "{}", s.context());
    // The control.
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), set(1, MIRROR_FORCE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(DARK_BLADE)], &[ChainOnAttack(MIRROR_FORCE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.lp(1)), (1, 8000), "Mirror Force destroyed the attacker: {}", s.context());
}
