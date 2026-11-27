//! Anime-only cards, group 19 (rows 5829-5858 of engine/added/cards.tsv): Thunder (character), Vellian Crowler, Jaden Yuki,
//! Zane Truesdale, Anger Mask and Bob Banter's cards (with those of Chumley, Dox, Frost, Baou, Makoto, Mathmatica, Princess
//! Rose and Scarr that share the batch). These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play Spell answering the
//! opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! The batch makes no Token.
//! Covered: the cards the per-card fuzz flagged (Rampage Condenser, Ojamandala, Anchor Knight, Animal Trail, Flute of
//! Hamelin, and Double it!!, see below), the two script notes (Frog Resurrection's discard cost leaving the discarded card
//! itself to be the target, Hurricane Nest's cost from the Deck or the hand with no "Special Hurricane" to pay it: both
//! stop on no Lua error, so neither needs an override), the signature cards (Lightning Talisman, Thunder Knight, Crazy Summon
//! Gear, Unfinished Time Box, Shield Attack, Lost Pride, Trap Booster, Violent Salvage, Right or Left Quiz!) and a seeded
//! sample of the rest (Desperate Battle, Fake Friendship Treaty, Gravity Gain, Illegal Summon, One-on-One Fight), with Koala
//! March, whose script's target check names a card it does not have, on the way.
//! Not coverable: Double it!! needs a Level 4 or higher "Quiz" monster, and the game has none (Quiz Sphinx and Quiz Monkey
//! are not in the client's card list), so the fuzz never reached it either; the test here is the control, that a Level 4+
//! monster of another kind winning against a Defense Position monster is not offered it.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg, resp};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 2, 700/600 vanilla Thunder
const BOLT_ESCARGOT: u16 = 4466; // Level 5 Thunder, 1400/1500 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const KOJIKOCY: u16 = 4072; // Level 4 Warrior, 1500/1200, vanilla
const MECHANICALCHASER: u16 = 4421; // Level 4 Machine, 1850/800, vanilla
const HYOSUBE: u16 = 4653; // Level 4 WATER Aqua, 1500/900, vanilla
const WATER_OMOTICS: u16 = 4255; // Level 4 WATER Aqua, 1400/1200, vanilla
const GREAT_ANGUS: u16 = 5641; // Level 4 Beast, 1800/600, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

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

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

// ---------------------------------------------------------------------------------------------------------------
// Thunder: Lightning Talisman, Rampage Condenser, Thunder Knight
// ---------------------------------------------------------------------------------------------------------------

/// Lightning Talisman: "When an opponent's monster declares an attack: Negate the attack, and if you do, your opponent draws
/// 1 card, also, your opponent takes 400 damage for each card in their hand." Team 1's Warwolf attacks Thunder Kid in turn 2:
/// the attack is negated (Thunder Kid lives, team 0 takes nothing), team 1 draws one card and takes 400 for each card it then
/// holds. Without the Talisman the same attack destroys Thunder Kid.
#[test]
fn lightning_talisman_negates_the_attack_and_hurts_for_each_card_in_hand() {
    let talisman = card("Lightning Talisman");
    let Some(mut s) = field(&[], &[set(0, talisman), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(talisman)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", talisman), 1, "the Talisman was activated: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "the attack was negated: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    let cards = hand_size(&s, 1) as i64;
    assert_eq!(cards, 5 + 1 + 1, "five opening cards, the turn's draw and the Talisman's: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 400 * cards, "400 for each of the {cards} cards in hand: {}", s.context());
    // The control.
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1300, 8000), "{}", s.context());
}

/// Rampage Condenser: "Activate only if you control a face-up Thunder-Type monster. Each time an opponent's Machine-Type
/// monster attacks, inflict 500 damage to your opponent." With Thunder Kid on the field it is activated in turn 1; in turn 2
/// team 1's Mechanicalchaser (1850) attacks Thunder Kid: team 1 takes 500 for the attack, team 0 takes 1150 from the battle.
/// A Warwolf (not a Machine) attacking costs team 1 nothing. Without a Thunder-Type monster the Spell cannot be activated.
#[test]
fn rampage_condenser_hurts_the_opponent_for_each_machine_attack() {
    let condenser = card("Rampage Condenser");
    let Some(mut s) = field(&[], &[hand(0, condenser), mon(0, THUNDER_KID), mon(1, MECHANICALCHASER)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(condenser)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", condenser), 1, "it stays on the field: {}", s.context());
    s.run_both([&[EndTurn], &[Attack(MECHANICALCHASER)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500, "500 for the Machine's attack: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1150, "and the battle: {}", s.context());
    // A monster that is not a Machine.
    let Some(mut s) = field(&[], &[hand(0, condenser), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(condenser), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1300, 8000), "{}", s.context());
    // No Thunder-Type monster to activate it with.
    let Some(mut s) = field(&[], &[hand(0, condenser), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(condenser)]).expect_err("no Thunder-Type monster");
    assert!(err.contains("never became possible"), "{err}");
}

/// Thunder Knight: "This card gains 400 ATK for each face-up Thunder-Type monster on the field." Alone it counts itself
/// (1300 + 400); with Thunder Kid on its side and another on the opponent's it counts three (1300 + 1200).
#[test]
fn thunder_knight_gains_400_for_each_thunder_monster_on_the_field() {
    let knight = card("Thunder Knight");
    let Some(s) = field(&[], &[mon(0, knight)]) else { return };
    assert_eq!(s.attack_of(0, knight), Some(1700), "{}", s.context());
    let Some(s) = field(&[], &[mon(0, knight), mon(0, THUNDER_KID), mon(1, THUNDER_KID)]) else { return };
    assert_eq!(s.attack_of(0, knight), Some(2500), "{}", s.context());
    // The opponent's own Thunder Knight counts mine too.
    let Some(s) = field(&[], &[mon(0, knight), mon(1, knight)]) else { return };
    assert_eq!((s.attack_of(0, knight), s.attack_of(1, knight)), (Some(2100), Some(2100)), "{}", s.context());
    // And it follows the field: Dark Hole takes the Thunder Kid away, and the Knight with it.
    let Some(mut s) = field(&[], &[hand(0, DARK_HOLE), mon(0, THUNDER_KID), mon(1, knight)]) else { return };
    assert_eq!(s.attack_of(1, knight), Some(2100), "{}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    assert_eq!((s.count(1, "grave", knight), s.count(0, "grave", THUNDER_KID)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Vellian Crowler: Crazy Summon Gear, Desperate Battle, Unfinished Time Box
// ---------------------------------------------------------------------------------------------------------------

/// Crazy Summon Gear: "Target 1 face-up monster your opponent controls and 1 monster in your Graveyard with 1500 or less
/// ATK; Your opponent must Special Summon up to 2 monsters from their Deck with the same Type and Level as their targeted
/// monster. You must Special Summon your targeted monster and all other monsters with the same name from your hand, Deck,
/// or Graveyard." Team 1's Dark Blade (Warrior, Level 4) is the opponent target and its Deck is all Kojikocy (Warrior,
/// Level 4): two come to its field. Thunder Kid in my Graveyard is the other target, and the Thunder Kid in my hand comes
/// with it, but not Warwolf (2000 ATK, not a target) nor the Feral Imps.
#[test]
fn crazy_summon_gear_summons_for_both_players() {
    let gear = card("Crazy Summon Gear");
    let spawns = [hand(0, gear), grave(0, THUNDER_KID), hand(0, THUNDER_KID), grave(0, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = duel(7, 8000, [FILLER, KOJIKOCY], &[], &spawns) else { return };
    s.prefer(&[(1, DARK_BLADE), (0, THUNDER_KID), (1, KOJIKOCY)]);
    s.prefer_most(true);
    s.run(&[Activate(gear)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", gear), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 2, "the targeted Thunder Kid and the one from my hand: {}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "mzone", WARWOLF)), (1, 0), "Warwolf is no target: {}", s.context());
    assert_eq!(s.count(1, "mzone", KOJIKOCY), 2, "two Warriors of Level 4 from the Deck for the opponent: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "{}", s.context());
}

/// Crazy Summon Gear: the opponent's Deck holds no monster of the target's Type and Level (Feral Imps are Fiends): the
/// opponent summons nothing, and I still summon mine.
#[test]
fn crazy_summon_gear_summons_for_me_when_the_opponent_has_no_match() {
    let gear = card("Crazy Summon Gear");
    let Some(mut s) = field(&[], &[hand(0, gear), grave(0, THUNDER_KID), mon(1, DARK_BLADE)]) else { return };
    s.prefer(&[(1, DARK_BLADE), (0, THUNDER_KID)]);
    s.run(&[Activate(gear)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.count(1, "mzone", DARK_BLADE), s.count(1, "mzone", FILLER)), (1, 0), "{}", s.context());
}

/// Desperate Battle: "During your Battle Phase: Target 1 face-up monster you control; It loses 1000 ATK until the end of the
/// Battle Phase, and can attack twice during this turn." In turn 3 Warwolf (2000) attacks Thunder Kid (700), and the Trap is
/// chained: Warwolf has 1000 and a second attack, which destroys the other Thunder Kid (300 damage each time). Its ATK is
/// back in Main Phase 2. Without the Trap the second attack is not possible.
#[test]
fn desperate_battle_loses_1000_atk_for_a_second_attack() {
    let battle = card("Desperate Battle");
    let spawns = [set(0, battle), mon(0, WARWOLF), mon(1, THUNDER_KID), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF), Chain(battle), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 2, "two attacks, two Thunder Kids: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * 300, "1000 against 700, twice: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "still in the Battle Phase: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "until the end of the Battle Phase: {}", s.context());
    // The control.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(1, THUNDER_KID), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(WARWOLF), Attack(WARWOLF)], &[EndTurn]]).expect_err("one attack only");
    assert!(err.contains("can't attack"), "{err}");
}

/// Unfinished Time Box: "Activate only if a monster you control is destroyed by battle. Banish the monster that battled it,
/// and draw 1 card. When this card leaves the field, Special Summon the monster banished by this effect to your opponent's
/// side of the field." Team 1's Warwolf destroys Thunder Kid in turn 2: the Time Box banishes it and team 0 draws; team 1's
/// Mystical Space Typhoon then destroys the Time Box, and Warwolf returns to team 1's field.
#[test]
fn unfinished_time_box_banishes_the_attacker_and_brings_it_back_when_it_leaves() {
    let timebox = card("Unfinished Time Box");
    let mst = card("Mystical Space Typhoon");
    let spawns = [set(0, timebox), mon(0, THUNDER_KID), mon(1, WARWOLF), hand(1, mst)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, timebox)]);
    let before = hand_size(&s, 0);
    s.run_both([&[EndTurn, Chain(timebox)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "Warwolf was banished: {}", s.context());
    assert_eq!(s.count(0, "szone", timebox), 1, "the Time Box stays: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "team 0 drew a card: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    s.run_both([&[], &[Activate(mst)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", timebox), 1, "Mystical Space Typhoon destroyed it: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "and Warwolf is back: {}", s.context());
    assert_eq!(s.count(1, "removed", WARWOLF), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Jaden Yuki: Ojamandala, Shield Attack
// ---------------------------------------------------------------------------------------------------------------

/// Ojamandala: "Pay 1000 LP, then target 1 each of "Ojama Green", "Ojama Yellow", and "Ojama Black" in your Graveyard;
/// Special Summon them." All three come back (Ojama Green 0/1000, Yellow 0/1000, Black 0/1000 in the game's text: read from
/// the field), 1000 LP are paid, and Ojamandala is in the Graveyard. With one of them missing, or with only two free Monster
/// Zones, it cannot be activated (the fuzz's duels never had all three in the Graveyard).
#[test]
fn ojamandala_summons_the_three_ojamas_for_1000_lp() {
    let [ojamandala, green, yellow, black] = ["Ojamandala", "Ojama Green", "Ojama Yellow", "Ojama Black"].map(card);
    let spawns = [hand(0, ojamandala), grave(0, green), grave(0, yellow), grave(0, black), grave(0, green)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(ojamandala)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", green), s.count(0, "mzone", yellow), s.count(0, "mzone", black)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", green), 1, "the second Ojama Green stays: {}", s.context());
    assert_eq!(s.lp(0), 7000, "{}", s.context());
    assert_eq!(s.count(0, "grave", ojamandala), 1, "{}", s.context());
    // One Ojama missing.
    let Some(mut s) = field(&[], &[hand(0, ojamandala), grave(0, green), grave(0, yellow)]) else { return };
    let err = s.run(&[Activate(ojamandala)]).expect_err("no Ojama Black");
    assert!(err.contains("never became possible"), "{err}");
    // Three Ojamas but two free Monster Zones.
    let spawns = [hand(0, ojamandala), grave(0, green), grave(0, yellow), grave(0, black), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(0, KOJIKOCY)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run(&[Activate(ojamandala)]).expect_err("two free zones");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000, "nothing was paid: {}", s.context());
}

/// Shield Attack: "Switch the ATK/DEF of the equipped monster." Warwolf (2000/100) equipped with it is 100/2000, and it
/// stays so when the equip card is the only change; when Shield Attack leaves the field Warwolf is 2000/100 again.
#[test]
fn shield_attack_switches_atk_and_def() {
    let shield = card("Shield Attack");
    let Some(mut s) = field(&[], &[hand(0, shield), mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(shield)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", shield), 1, "{}", s.context());
    let m = s.monster(0, WARWOLF).unwrap();
    assert_eq!((m["attack"].as_i64(), m["defense"].as_i64()), (Some(100), Some(2000)), "{}", s.context());
    // The equip card leaves with the monster: Dark Hole destroys Warwolf and Shield Attack goes to the Graveyard.
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "grave", shield)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Zane Truesdale: Lost Pride, Trap Booster
// ---------------------------------------------------------------------------------------------------------------

/// Lost Pride: "Send 1 Spell Card from your hand to the Graveyard. Add 1 Spell Card from your opponent's Graveyard to your
/// hand. If you activate that card, you take 1000 damage." Monster Reborn is discarded, Dark Hole (not Dian Keto) is taken
/// from team 1's Graveyard, and activating it costs 1000 LP.
#[test]
fn lost_pride_takes_a_spell_from_the_opponents_graveyard_at_a_price() {
    let pride = card("Lost Pride");
    let spawns = [set(0, pride), hand(0, MONSTER_REBORN), grave(1, DIAN_KETO), grave(1, DARK_HOLE), mon(1, KOJIKOCY)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, DARK_HOLE), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(pride)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", MONSTER_REBORN), 1, "the cost: {}", s.context());
    assert_eq!((s.count(0, "hand", DARK_HOLE), s.count(1, "grave", DARK_HOLE)), (1, 0), "{}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "nothing paid yet: {}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "1000 for activating the card taken: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "it goes back to its owner's Graveyard: {}", s.context());
    assert_eq!(s.count(1, "grave", KOJIKOCY), 1, "Dark Hole did its own work: {}", s.context());
}

/// Lost Pride: without a Spell in the hand to send, or without one in the opponent's Graveyard, it is not offered.
#[test]
fn lost_pride_needs_a_spell_to_send_and_one_to_take() {
    let pride = card("Lost Pride");
    let Some(mut s) = field(&[], &[set(0, pride), grave(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(pride)], &[EndTurn]]).expect_err("no Spell in my hand");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = field(&[], &[set(0, pride), hand(0, MONSTER_REBORN), grave(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(pride)], &[EndTurn]]).expect_err("no Spell in the opponent's Graveyard");
    assert!(err.contains("never became possible"), "{err}");
}

/// Trap Booster: "Discard 1 card; This turn, you can activate 1 Trap Card from your hand." With two Just Desserts in hand
/// (500 damage for each monster the opponent controls) one is activated after the Booster, 1000 damage for two monsters, and
/// the second is not offered ("1 Trap Card"). Without the Booster a Trap in the hand is not offered at all.
#[test]
fn trap_booster_lets_one_trap_be_activated_from_the_hand() {
    let [booster, desserts] = ["Trap Booster", "Just Desserts"].map(card);
    let spawns = [hand(0, booster), hand(0, desserts), hand(0, desserts), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(booster)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", booster), 1, "{}", s.context());
    assert!(offered(&s, "activates").contains(&desserts.into()), "{:?}", offered(&s, "activates"));
    s.run(&[Activate(desserts)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "500 for each of the two monsters: {}", s.context());
    assert_eq!(s.count(0, "grave", desserts), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", desserts), 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&desserts.into()), "only 1 Trap Card: {:?}", offered(&s, "activates"));
    // The control: the same hand with no Booster.
    let Some(s) = field(&[], &[hand(0, desserts), mon(1, WARWOLF)]) else { return };
    assert!(!offered(&s, "activates").contains(&desserts.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Anger Mask: Anchor Knight, Violent Salvage
// ---------------------------------------------------------------------------------------------------------------

/// Anchor Knight: "Up to thrice per turn: You can discard 1 card; inflict 600 damage to your opponent." Three discards for
/// 1800 damage, and the fourth is not offered.
#[test]
fn anchor_knight_burns_for_600_up_to_three_times_a_turn() {
    let knight = card("Anchor Knight");
    let Some(mut s) = field(&[], &[mon(0, knight)]) else { return };
    keep_hands(&mut s);
    let graves = s.count(0, "grave", FILLER);
    s.run(&[Activate(knight), Activate(knight), Activate(knight)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1800, "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), graves + 3, "three discards: {}", s.context());
    assert!(!offered(&s, "activates").contains(&knight.into()), "up to thrice: {:?}", offered(&s, "activates"));
}

/// Anchor Knight: "When this card is destroyed by battle and sent to the Graveyard: You can discard 1 card; Special Summon
/// this card in face-up Defense Position." Team 1's Mechanicalchaser (1850) destroys it (1500/600) in turn 2: it asks, one
/// Feral Imp is discarded, and the Knight is back in face-up Defense (350 damage from the battle).
#[test]
fn anchor_knight_comes_back_in_defense_when_destroyed_by_battle() {
    let knight = card("Anchor Knight");
    let Some(mut s) = field(&[], &[mon(0, knight), mon(1, MECHANICALCHASER)]) else { return };
    keep_hands(&mut s);
    let graves = s.count(0, "grave", FILLER);
    s.run_both([&[EndTurn, Chain(knight)], &[Attack(MECHANICALCHASER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", knight), 1, "back on the field: {}", s.context());
    assert_eq!(s.position_of(0, knight), Some(4), "face-up Defense: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), graves + 1, "one card discarded: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 350, "{}", s.context());
}

/// Anchor Knight: the revival is a "can": answering no to the question leaves the Knight in the Graveyard and the hand whole.
#[test]
fn anchor_knight_stays_in_the_graveyard_when_not_used() {
    let knight = card("Anchor Knight");
    let Some(mut s) = field(&[], &[mon(0, knight), mon(1, MECHANICALCHASER)]) else { return };
    keep_hands(&mut s);
    let graves = s.count(0, "grave", FILLER);
    // Turn 3: the Knight itself attacks the stronger Mechanicalchaser.
    let asked = |p: &serde_json::Value| p["player"] == 0 && (p["type"] == msg::SELECT_EFFECTYN || p["type"] == msg::SELECT_CHAIN);
    let plan = [EndTurn, Attack(knight)];
    s.run_to(&plan, |p| asked(p) && (p["type"] == msg::SELECT_EFFECTYN || p["selects"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == knight)))).unwrap();
    let no = if s.prompt()["type"] == msg::SELECT_EFFECTYN { Response::yes(resp::EFFECTYN, false) } else { Response::Chain(None) };
    s.respond(no.json()).unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", knight), s.count(0, "mzone", knight)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), graves, "{}", s.context());
}

/// Violent Salvage (a Trap, Set first): "Activate only if you control a face-up "Anchor Knight": Add the top 3 cards of your Graveyard to your
/// hand. Those cards cannot be Summoned or activated." The Graveyard holds Warwolf, Dark Blade, Thunder Kid and Dian Keto
/// (in that order): the top three come to the hand, Warwolf stays; Dark Blade and Thunder Kid cannot be Summoned and Dian
/// Keto cannot be activated. Without an Anchor Knight it is not offered.
#[test]
fn violent_salvage_adds_the_top_three_cards_of_the_graveyard_and_locks_them() {
    let [salvage, knight] = ["Violent Salvage", "Anchor Knight"].map(card);
    let spawns = [set(0, salvage), mon(0, knight), grave(0, WARWOLF), grave(0, DARK_BLADE), grave(0, THUNDER_KID), grave(0, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(salvage)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", DARK_BLADE), s.count(0, "hand", THUNDER_KID), s.count(0, "hand", DIAN_KETO)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.count(0, "grave", WARWOLF), s.count(0, "hand", WARWOLF)), (1, 0), "only the top three: {}", s.context());
    assert_eq!(s.count(0, "grave", salvage), 1, "{}", s.context());
    let summons = offered(&s, "summons");
    assert!(!summons.contains(&DARK_BLADE.into()) && !summons.contains(&THUNDER_KID.into()), "cannot be Summoned: {summons:?}");
    assert!(!offered(&s, "activates").contains(&DIAN_KETO.into()), "cannot be activated: {:?}", offered(&s, "activates"));
    // The control: no Anchor Knight.
    let spawns = [set(0, salvage), mon(0, WARWOLF), grave(0, DARK_BLADE), grave(0, THUNDER_KID), grave(0, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(salvage)], &[EndTurn]]).expect_err("no Anchor Knight");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Bob Banter: Double it!!, Right or Left Quiz!
// ---------------------------------------------------------------------------------------------------------------

/// Double it!!: "If a Level 4 or higher "Quiz" monster you control destroys an opponent's Defense Position monster by
/// battle: it can attack again, and if it does, its ATK is doubled during the Damage Step only." The game has no Quiz
/// monster (see the header), so this is the control: Dark Blade (Level 4) destroys a Defense Position Warwolf in turn 3, and
/// the Set Double it!! is not offered at that chance; nothing else changes (no second attack).
#[test]
fn double_it_is_not_for_a_monster_that_is_not_a_quiz_monster() {
    let double = card("Double it!!");
    let spawns = [set(0, double), mon(0, DARK_BLADE), Spawn { team: 1, place: MonsterDefense, cid: WARWOLF }];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(DARK_BLADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Blade destroyed it: {}", s.context());
    assert_eq!(s.count(0, "szone", double), 1, "not activated: {}", s.context());
    let attacks = s.prompt()["attacks"].as_array().map_or(0, Vec::len);
    assert_eq!(attacks, 0, "no second attack: {}", s.context());
}

/// Answers the engine's next yes/no question with `yes`.
fn answer(s: &mut Scenario, yes: bool) {
    assert_eq!(s.prompt()["type"], msg::SELECT_YESNO, "a yes/no question: {}", s.context());
    s.respond(Response::yes(resp::YESNO, yes).json()).unwrap();
}

/// Right or Left Quiz!: "Hold this card behind your back in either your right or left hand. Your opponent must guess which
/// hand is holding this card. If they are correct, they gain 500 LP. Otherwise, inflict 500 damage to your opponent." The
/// holder answers first and the guesser second: the same answers mean a correct guess (the opponent gains 500), different
/// ones a wrong guess (500 damage). The activator's own LP never change.
#[test]
fn right_or_left_quiz_pays_500_to_the_opponent_or_takes_500_from_it() {
    let quiz = card("Right or Left Quiz!");
    for (hold, guess, lp) in [(true, true, 8500), (false, false, 8500), (true, false, 7500), (false, true, 7500)] {
        let Some(mut s) = field(&[], &[hand(0, quiz)]) else { return };
        s.run_to(&[Activate(quiz)], |p| p["type"] == msg::SELECT_YESNO).unwrap();
        answer(&mut s, hold);
        assert_eq!(s.prompt()["player"], 1, "the opponent guesses: {}", s.context());
        answer(&mut s, guess);
        s.run(&[]).unwrap();
        clean(&s);
        assert_eq!((s.lp(0), s.lp(1)), (8000, lp), "hold {hold}, guess {guess}: {}", s.context());
        assert_eq!(s.count(0, "grave", quiz), 1, "{}", s.context());
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Chumley: Animal Trail, Koala March
// ---------------------------------------------------------------------------------------------------------------

/// Animal Trail: "Activate only if a monster you control is destroyed by battle: Add 1 Beast-Type monster from your Deck to
/// your hand." Team 1's Mechanicalchaser destroys Thunder Kid in turn 2; the Set Animal Trail is chained and the Great Angus
/// at the bottom of the Deck comes to the hand (no Beast in the Deck leaves it unoffered).
#[test]
fn animal_trail_searches_a_beast_after_a_monster_is_destroyed_in_battle() {
    let trail = card("Animal Trail");
    let spawns = [set(0, trail), mon(0, THUNDER_KID), mon(1, MECHANICALCHASER)];
    let Some(mut s) = field(&[GREAT_ANGUS], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(trail)], &[Attack(MECHANICALCHASER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", GREAT_ANGUS), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", trail), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    // No Beast in the Deck: the Trap is not offered.
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(MECHANICALCHASER)]]).unwrap();
    assert_eq!(s.count(0, "szone", trail), 1, "{}", s.context());
    assert!(!offered(&s, "selects").contains(&trail.into()));
}

/// Koala March: "Target 1 Level 4 or lower "Koala" monster in your Graveyard; Special Summon it. Then, you can Special
/// Summon 1 monster with the same name from your hand." The script's check of a target (`chkc`) names a card it doesn't
/// have (`c83764718.filter`), but the activation never reached it: Des Koala comes from the Graveyard and, by the yes the
/// scenario gives, the second Des Koala from the hand.
#[test]
fn koala_march_summons_a_koala_and_one_with_the_same_name_from_the_hand() {
    let [march, koala] = ["Koala March", "Des Koala"].map(card);
    let Some(mut s) = field(&[], &[hand(0, march), grave(0, koala), hand(0, koala)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(march)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", koala), 2, "the one from the Graveyard and the one from the hand: {}", s.context());
    assert_eq!(s.count(0, "grave", march), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dox: Defense Wall, One-on-One Fight
// ---------------------------------------------------------------------------------------------------------------

/// One-on-One Fight: "Each player selects 1 monster they control with the highest ATK and conducts battle between the
/// selected monsters." In turn 3 Warwolf (2000) and Dark Blade (1800) are the two highest: Dark Blade is destroyed (200
/// damage) and no other monster is touched.
#[test]
fn one_on_one_fight_battles_the_two_strongest_monsters() {
    let fight = card("One-on-One Fight");
    let spawns = [set(0, fight), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, DARK_BLADE), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(fight)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 200, "{}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "mzone", THUNDER_KID), s.count(1, "mzone", THUNDER_KID)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", fight), 1, "{}", s.context());
}

/// Defense Wall: "While this card is in face-up Defense Position, your opponent cannot attack other monsters you control."
/// With the Wall (0/2100) in Defense and Thunder Kid beside it, team 1's Warwolf can only attack the Wall.
#[test]
fn defense_wall_is_the_only_monster_that_can_be_attacked() {
    let wall = card("Defense Wall");
    let spawns = [Spawn { team: 0, place: MonsterDefense, cid: wall }, mon(0, THUNDER_KID), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "Thunder Kid was not attacked: {}", s.context());
    assert_eq!(s.count(0, "mzone", wall), 1, "2000 against 2100: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Princess Rose: Flute of Hamelin, Frog Resurrection
// ---------------------------------------------------------------------------------------------------------------

/// Flute of Hamelin: "Target 1 monster in your Graveyard that was sent to the Graveyard this turn; Both players must send
/// all cards from their hands and Decks with the same name as it to the Graveyard." Dark Hole sends Warwolf to the
/// Graveyard, and the Flute then sends the Warwolves of my hand and Deck (one and two) and of team 1's hand and Deck (whose
/// Deck is all Warwolf) after it; the Feral Imps of my hand stay. The fuzz never had a copy of the target in a Deck.
#[test]
fn flute_of_hamelin_sends_every_copy_in_hands_and_decks_to_the_graveyard() {
    let flute = card("Flute of Hamelin");
    let spawns = [hand(0, flute), hand(0, DARK_HOLE), mon(0, WARWOLF), hand(0, WARWOLF)];
    let Some(mut s) = duel(7, 8000, [FILLER, WARWOLF], &[WARWOLF, WARWOLF], &spawns) else { return };
    let (imps, deck0, deck1) = (s.count(0, "hand", FILLER), s.state()[0]["deck"].as_u64().unwrap(), s.state()[1]["deck"].as_u64().unwrap());
    assert_eq!(s.count(0, "hand", WARWOLF), 1, "{}", s.context());
    assert!(s.count(1, "hand", WARWOLF) >= 5, "team 1's hand is all Warwolf: {}", s.context());
    let theirs = s.count(1, "hand", WARWOLF) as u64 + deck1;
    s.run(&[Activate(DARK_HOLE), Activate(flute)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", WARWOLF), s.count(1, "hand", WARWOLF)), (0, 0), "hands emptied of Warwolf: {}", s.context());
    assert_eq!((s.state()[0]["deck"].as_u64(), s.state()[1]["deck"].as_u64()), (Some(deck0 - 2), Some(0)), "Decks too: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 4, "the one from the field, one from the hand, two from the Deck: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF) as u64, theirs, "all of team 1's: {}", s.context());
    assert_eq!(s.count(0, "hand", FILLER), imps, "other names stay: {}", s.context());
    assert_eq!(s.count(0, "grave", flute), 1, "{}", s.context());
}

/// Flute of Hamelin: it needs a monster that was sent to the Graveyard this turn: with one sent the turn before (a Warwolf
/// the scenario spawned there counts as sent the previous turn) it is not offered.
#[test]
fn flute_of_hamelin_needs_a_monster_sent_this_turn() {
    let flute = card("Flute of Hamelin");
    let Some(mut s) = field(&[], &[hand(0, flute), grave(0, WARWOLF)]) else { return };
    let err = s.run(&[Activate(flute)]).expect_err("nothing sent this turn");
    assert!(err.contains("never became possible"), "{err}");
}

/// Frog Resurrection: "Discard 1 WATER monster to target 1 WATER monster in your Graveyard; Special Summon that monster."
/// The discard is paid first and the target chosen after it, so the monster just discarded is a target too: the script note's
/// worry, a cost that leaves nothing to choose, cannot happen (a discard only adds a WATER monster to the Graveyard, and the
/// Spell is not offered at all without one there before). Water Omotics is revived with Hyosube discarded, and Hyosube,
/// the card just discarded, revived with Water Omotics in the Graveyard. Without a WATER monster to discard or to revive it
/// is not offered.
#[test]
fn frog_resurrection_discards_a_water_monster_for_another() {
    let frog = card("Frog Resurrection");
    let Some(mut s) = field(&[], &[hand(0, frog), hand(0, HYOSUBE), grave(0, WATER_OMOTICS)]) else { return };
    s.prefer(&[(0, WATER_OMOTICS)]);
    s.run(&[Activate(frog)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WATER_OMOTICS), s.count(0, "grave", HYOSUBE)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", frog), 1, "{}", s.context());
    // The discarded card as the target.
    let Some(mut s) = field(&[], &[hand(0, frog), hand(0, HYOSUBE), grave(0, WATER_OMOTICS)]) else { return };
    s.prefer(&[(0, HYOSUBE)]);
    s.run(&[Activate(frog)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", HYOSUBE), s.count(0, "grave", WATER_OMOTICS)), (1, 1), "{}", s.context());
    // Nothing to discard, or nothing to revive.
    let Some(mut s) = field(&[], &[hand(0, frog), grave(0, WATER_OMOTICS)]) else { return };
    assert!(s.run(&[Activate(frog)]).expect_err("no WATER monster to discard").contains("never became possible"));
    let Some(mut s) = field(&[], &[hand(0, frog), hand(0, HYOSUBE)]) else { return };
    assert!(s.run(&[Activate(frog)]).expect_err("no WATER monster to revive").contains("never became possible"));
}

// ---------------------------------------------------------------------------------------------------------------
// Frost, Baou, Makoto, Mathmatica and Scarr: the sample's rest
// ---------------------------------------------------------------------------------------------------------------

/// Gravity Gain: "During each your Standby Phases, Tribute 1 monster or destroy this card. Level 4 or lower monsters cannot
/// declare an attack." Activated in turn 3: Warwolf (Level 4) cannot attack, Dark Magician (Level 7) can. At the next
/// Standby Phase a yes Tributes a monster and keeps the Gain, a no destroys it.
#[test]
fn gravity_gain_stops_small_monsters_and_asks_for_a_tribute_each_standby_phase() {
    let gain = card("Gravity Gain");
    let spawns = [set(0, gain), mon(0, WARWOLF), mon(0, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(gain)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", gain), 1, "{}", s.context());
    let err = s.run(&[Attack(WARWOLF)]).expect_err("Level 4 cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{DARK_MAGICIAN}")) && !err.contains(&format!("\"cid\":{WARWOLF}")), "{err}");
    // The next Standby Phase of team 0 (turn 5): tribute, then destroy.
    for (yes, kept) in [(true, 1), (false, 0)] {
        let Some(mut s) = field(&[], &spawns) else { return };
        keep_hands(&mut s);
        s.prefer(&[(0, WARWOLF)]);
        s.run_to(&[EndTurn, Activate(gain), EndTurn], |p| p["type"] == msg::SELECT_YESNO).unwrap();
        for _ in 0..40 {
            if s.prompt()["type"] == msg::SELECT_YESNO {
                break;
            }
            s.pass().unwrap();
        }
        assert_eq!(s.count(0, "szone", gain), 1, "asked for a Tribute with the Gain on the field: {}", s.context());
        answer(&mut s, yes);
        s.run(&[]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "szone", gain), kept, "yes {yes}: {}", s.context());
        assert_eq!(s.count(0, "grave", WARWOLF), kept, "the Tribute: {}", s.context());
        assert_eq!(s.count(0, "grave", gain), 1 - kept, "{}", s.context());
    }
}

/// Fake Friendship Treaty: "Your opponent cannot Summon Level 4 or lower monsters. If a monster you control attacks, or your
/// opponent takes effect damage from 1 of your cards, destroy this card." In turn 4 team 1 cannot Normal Summon Feral Imp
/// (Level 4) but can Tribute Summon Bolt Escargot (Level 5); in turn 5 team 0's Warwolf attacks and the Treaty is destroyed.
#[test]
fn fake_friendship_treaty_stops_small_summons_until_its_controller_attacks() {
    let treaty = card("Fake Friendship Treaty");
    let spawns = [set(0, treaty), mon(0, WARWOLF), hand(1, BOLT_ESCARGOT), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(treaty), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", treaty), 1, "{}", s.context());
    let summons = offered(&s, "summons");
    assert!(!summons.contains(&FILLER.into()), "no Level 4 monster: {summons:?}");
    assert!(summons.contains(&BOLT_ESCARGOT.into()), "Level 5 is allowed: {summons:?}");
    s.run_both([&[Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", treaty), 1, "attacking destroyed it: {}", s.context());
}

/// Illegal Summon: "Each player chooses 1 Level 4 or lower monster from their Deck and Special Summons it to their opponent's
/// side of the field." Warwolf, at the bottom of team 0's Deck, goes to team 1's field and team 1's Kojikocy to team 0's.
#[test]
fn illegal_summon_swaps_a_monster_from_each_deck_to_the_other_side() {
    let illegal = card("Illegal Summon");
    let Some(mut s) = duel(7, 8000, [FILLER, KOJIKOCY], &[WARWOLF], &[hand(0, illegal)]) else { return };
    s.prefer(&[(0, WARWOLF), (1, KOJIKOCY)]);
    s.run(&[Activate(illegal)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", WARWOLF), s.count(0, "mzone", WARWOLF)), (1, 0), "{}", s.context());
    assert_eq!((s.count(0, "mzone", KOJIKOCY), s.count(1, "mzone", KOJIKOCY)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "szone", illegal), 1, "a Continuous Spell: {}", s.context());
}

/// Hurricane Nest: "If your opponent Special Summons a monster: you can send 1 "Special Hurricane" from your hand or Deck to
/// the Graveyard; destroy that monster." Team 1's Monster Reborn brings back Warwolf in turn 2 and the Nest destroys it,
/// with Special Hurricane from the hand, and from the Deck. With no Special Hurricane in either place nothing is asked.
#[test]
fn hurricane_nest_destroys_a_special_summoned_monster() {
    let [nest, hurricane] = ["Hurricane Nest", "Special Hurricane"].map(card);
    for (in_hand, bottom) in [(true, &[][..]), (false, &[hurricane][..])] {
        let mut spawns = vec![spell(0, nest), hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
        if in_hand {
            spawns.push(hand(0, hurricane));
        }
        let Some(mut s) = field(bottom, &spawns) else { return };
        keep_hands(&mut s);
        s.run_both([&[EndTurn, Chain(nest)], &[Activate(MONSTER_REBORN)]]).unwrap();
        clean(&s);
        assert_eq!(s.count(1, "grave", WARWOLF), 1, "destroyed again: {}", s.context());
        assert_eq!(s.count(0, "grave", hurricane), 1, "the cost: {}", s.context());
        assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    }
    let Some(mut s) = field(&[], &[spell(0, nest), hand(1, MONSTER_REBORN), grave(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "no Special Hurricane to send: {}", s.context());
}
