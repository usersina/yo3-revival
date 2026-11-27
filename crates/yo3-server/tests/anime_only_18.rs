//! Anime-only cards, group 18 (rows 5797-5828 of engine/added/cards.tsv): The D's, Trueman's, Zane Truesdale's, the
//! Emotionless Mask's, the Laughter Mask's and Lucien Grimley's cards. These cards exist only in the anime, so every duel
//! plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real core with both seats'
//! client models checked after every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every
//! Quick-Play Spell answering the opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's
//! attacks wait for turn 3.
//! Covered: the Tokens (Plasma Token, made by Claret Note; Enigma Token, made by Enigma the Creator), the cards the per-card
//! fuzz flagged (D - Boost, Unfair Judge, Card from a Different Dimension, Time Fusion, Silent Pain, Silent Torment, Rough
//! Exploder, Rough Fight, Insect Pheromone, Charge, Debris Station), the signature cards (Claret Note, Enigma the Creator,
//! Fool Clown with the Silent trio and the Rough pair, Debris Station with Satellite Cannon and Satellite Base, Insect
//! Garden), and a seeded sample of the rest (Explosion Wing, Footsteps of the Goddess, Glory Level Talisman, Magical
//! Longicorn, Thousand Punisher and Zero Gazer), with Silent Space, Level Tax and the other two Goddess Spells on the way.
//! Three cards have an override: Unfair Judge (`engine/overrides/c511001347.lua`: its target is chosen only while its controller
//! has a monster; upstream read the strongest monster first and stopped on a Lua error when Mirror Force had destroyed them all
//! in the Battle Phase of a negated attack, `unfair_judge_is_not_offered_to_a_player_without_monsters`), Thousand Punisher
//! (`c511002890.lua`: a Tribute of under 1000 ATK leaves no attack, `thousand_punisher_with_less_than_1000_atk_gives_no_attack`)
//! and Glory Level Talisman (`c511000723.lua`: the forced trigger that destroys the attacker was never chained by the core,
//! `glory_level_talisman_destroys_a_lower_level_monster_attacking_the_equipped_monster`).
//! Not coverable: D - Boost. It needs "D - Force" face-up on top of the Deck, which only the anime D - Force can do
//! ("ReverseInDeck" in its anime script); the card plays as printed here (`docs/anime-cards.md`: D - Force's anime effect
//! works from the Deck, which the client's menus have not been shown to offer), so the fuzz never reaches D - Boost and
//! no scenario can put "D - Force" there. `d_boost_waits_for_a_face_up_d_force_on_top_of_the_deck` shows it is never
//! offered with "D - Force" face-down on top.
use serde_json::json;
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn banished(team: u32, cid: u16) -> Spawn { Spawn { team, place: Banished, cid } }
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

/// A duel with the Extra Decks given: `extra[team]`.
fn field_extra(extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], extra[0]), (&[FILLER; 40][..], extra[1])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

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

/// A monster's (ATK, DEF, Level, race, attribute) as the engine holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

const WARRIOR: i64 = 0x1;
const FAIRY: i64 = 0x4;
const DARK: i64 = 0x20;
const LIGHT: i64 = 0x10;

// ---------------------------------------------------------------------------------------------------------------
// Claret Note and the Plasma Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Claret Note: "Activate this card by targeting 1 face-up monster your opponent controls. Special Summon 1 "Plasma Token"
/// (Warrior-Type/DARK/Level 1/ATK 0/DEF 0) for every 4 Levels the targeted monster has. The Token(s) Special Summoned by
/// this effect are destroyed at the end of this turn." Blue-Eyes White Dragon (Level 8) gives two Tokens, Dark Magician (7)
/// and Dark Blade (4) one; the Tokens are in face-up Attack Position and are gone at the End Phase (a Token leaves no card
/// in the Graveyard); the opponent's monster is left alone.
#[test]
fn claret_note_makes_a_plasma_token_for_every_four_levels_and_they_end_with_the_turn() {
    let [note, token] = ["Claret Note", "Plasma Token"].map(card);
    let blue_eyes = card("Blue-Eyes White Dragon");
    for (target, tokens) in [(blue_eyes, 2), (DARK_MAGICIAN, 1), (DARK_BLADE, 1)] {
        let Some(mut s) = field(&[], &[hand(0, note), mon(1, target)]) else { return };
        keep_hands(&mut s);
        s.run(&[Activate(note)]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "mzone", token), tokens, "{tokens} Token(s) for {target}: {}", s.context());
        assert_eq!(s.count(0, "grave", note), 1, "{}", s.context());
        let t = stats(&s, 0, token);
        assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(0), Some(0), Some(1), WARRIOR, DARK), "Warrior, DARK, Level 1, 0/0: {}", s.context());
        assert_eq!(s.position_of(0, token), Some(1), "face-up Attack: {}", s.context());
        assert_eq!(s.count(1, "mzone", target), 1, "the target stays: {}", s.context());
        // The End Phase destroys them, and a Token is not a card in the Graveyard.
        s.run(&[EndTurn]).unwrap();
        clean(&s);
        assert_eq!((s.count(0, "mzone", token), s.count(0, "grave", token)), (0, 0), "the Tokens are gone: {}", s.context());
        assert_eq!(s.count(1, "mzone", target), 1, "{}", s.context());
    }
}

/// Claret Note: "every 4 Levels": a Level 3 monster (Thunder Kid) has none, so the Spell has no target and is not offered; a
/// face-down monster is no target either.
#[test]
fn claret_note_needs_a_target_of_level_4_or_more() {
    let note = card("Claret Note");
    let Some(mut s) = field(&[], &[hand(0, note), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(note)]).expect_err("no Level 4 monster");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "hand", note), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Enigma the Creator and the Enigma Token
// ---------------------------------------------------------------------------------------------------------------

/// Enigma the Creator: "When this card is destroyed by battle: Special Summon 1 "Enigma Token" (Fairy-Type/LIGHT/Level 1/ATK
/// 0/DEF 0)." Team 1's Warwolf (2000) attacks Enigma (Level 4, 1200/1200, Warrior, DARK): Enigma goes to the Graveyard, the
/// Token comes to its controller's side and 800 damage is taken. Destroyed by Dark Hole it leaves no Token.
#[test]
fn enigma_the_creator_leaves_an_enigma_token_when_destroyed_in_battle() {
    let [enigma, token] = ["Enigma the Creator", "Enigma Token"].map(card);
    let Some(mut s) = field(&[], &[mon(0, enigma), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let e = stats(&s, 0, enigma);
    assert_eq!((e.0, e.1, e.2, e.3, e.4), (Some(1200), Some(1200), Some(4), WARRIOR, DARK), "{}", s.context());
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", enigma), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", token), 1, "the Token came: {}", s.context());
    let t = stats(&s, 0, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(0), Some(0), Some(1), FAIRY, LIGHT), "Fairy, LIGHT, Level 1, 0/0: {}", s.context());
    assert!(matches!(s.position_of(0, token), Some(1 | 4)), "face-up: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
    // The control: destroyed by a card effect there is no Token.
    let Some(mut s) = field(&[], &[mon(0, enigma), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", enigma), s.count(0, "mzone", token)), (1, 0), "{}", s.context());
}

/// The Enigma Token leaves the field like any Token: attacked by Warwolf in turn 4 it is destroyed and is not a card in the
/// Graveyard, and its 0 ATK costs no more than the battle's own damage.
#[test]
fn an_enigma_token_destroyed_in_battle_is_gone() {
    let [enigma, token] = ["Enigma the Creator", "Enigma Token"].map(card);
    let Some(mut s) = field(&[], &[mon(0, enigma), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "mzone", token), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", token), s.count(0, "grave", token)), (0, 0), "the Token is gone: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// D - Boost (not coverable)
// ---------------------------------------------------------------------------------------------------------------

/// D - Boost: "While "D - Force" is face-up on top of your Deck: Draw 2 cards from underneath "D - Force"." With "D - Force"
/// on top of the Deck face-down (as drawn from a shuffled Deck: nothing here turns it face-up) the Trap is never
/// offered, in any of my turns, and nothing is drawn through it.
#[test]
fn d_boost_waits_for_a_face_up_d_force_on_top_of_the_deck() {
    let [boost, force] = ["D - Boost", "D - Force"].map(card);
    let Some(mut s) = field(&[force, force, force], &[set(0, boost)]) else { return };
    keep_hands(&mut s);
    // The three D - Forces are the Deck's bottom, and the Deck has more than the turns draw: never on top.
    let err = s.run_both([&[EndTurn, Activate(boost)], &[EndTurn]]).expect_err("D - Force is never face-up on top");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", boost), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Unfair Judge (engine/overrides/c511001347.lua)
// ---------------------------------------------------------------------------------------------------------------

/// The index of a card in the battle prompt's `chains` (the effects that can be activated in the Battle Phase).
fn battle_chain(s: &Scenario, cid: u16) -> Option<usize> {
    s.prompt()["chains"].as_array()?.iter().position(|c| c["cid"] == cid)
}

/// Unfair Judge: "Activate only at the end of a Battle Phase in which an attack from a face-up monster you control was negated:
/// Pay 800 LP, then target 1 monster you control whose ATK is less than both the total ATK of all Attack Position monsters
/// your opponent controls and the total DEF of all Defense Position monsters your opponent controls; you may conduct a
/// second Battle Phase this turn. During that Battle Phase, only that monster can attack and can attack all monsters your
/// opponent controls once each." Warwolf's attack is negated by Magic Cylinder (2000 damage, 6000 left). Before the Spell,
/// the Battle Phase offers no Judge; after the negation it does. With it: 800 LP paid, then the second Battle Phase, where
/// Dark Blade (1800 against Thunder Kid and Feral Imp, 2000 in all) attacks both monsters and destroys them (500 and 1100
/// damage), and Thunder Kid, who never attacked, cannot.
#[test]
fn unfair_judge_gives_a_second_battle_phase_to_one_monster_that_attacks_all() {
    let [judge, cylinder] = ["Unfair Judge", "Magic Cylinder"].map(card);
    let spawns = [mon(0, WARWOLF), mon(0, DARK_BLADE), mon(0, THUNDER_KID), hand(0, judge), set(1, cylinder), mon(1, THUNDER_KID), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER), (0, DARK_BLADE)]);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[ChainOnAttack(cylinder)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000, "Magic Cylinder hit the attacker's controller: {}", s.context());
    let at = battle_chain(&s, judge).unwrap_or_else(|| panic!("Unfair Judge is offered after the negation: {}", s.prompt()));
    s.respond(json!({"type": 1, "action": battle::CHAIN, "index": at})).unwrap();
    // The Battle Phase ends and the second one starts at once.
    for _ in 0..12 {
        if s.prompt()["type"] == msg::SELECT_BATTLECMD {
            break;
        }
        s.pass().unwrap();
    }
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 2000 - 800, "800 LP paid: {}", s.context());
    assert_eq!(s.count(0, "grave", judge), 1, "{}", s.context());
    // The second Battle Phase: only Dark Blade, once against each monster.
    let attackers = offered_attackers(&s);
    assert_eq!(attackers, vec![DARK_BLADE as u64], "only Dark Blade may attack: {}", s.context());
    s.run(&[Attack(DARK_BLADE), Attack(DARK_BLADE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "grave", FILLER)), (1, 1), "both were destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1100 - 500, "{}", s.context());
}

/// The cids the battle prompt lets attack.
fn offered_attackers(s: &Scenario) -> Vec<u64> {
    s.prompt()["attacks"].as_array().map_or(vec![], |a| a.iter().filter_map(|c| c["cid"].as_u64()).collect())
}

/// Unfair Judge: "Activate only at the end of a Battle Phase in which an attack ... was negated": a Battle Phase whose attack
/// was not negated (Warwolf destroys Thunder Kid) has no Judge on offer, and neither does one before any attack.
#[test]
fn unfair_judge_needs_a_negated_attack() {
    let judge = card("Unfair Judge");
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(0, DARK_BLADE), hand(0, judge), mon(1, THUNDER_KID)]) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "the attack went through: {}", s.context());
    assert_eq!(battle_chain(&s, judge), None, "no negated attack, no Judge: {}", s.prompt());
}

/// Unfair Judge, the script note: its target reads the strongest monster of the controller's field first, and that field can
/// be empty in the Battle Phase of a negated attack. Dark Blade's attack is negated by Magic Cylinder, then Warwolf's is
/// answered by Mirror Force, which destroys both: with no monster left the Judge is simply not offered (upstream's script
/// stopped on a Lua error each time the core looked at it).
#[test]
fn unfair_judge_is_not_offered_to_a_player_without_monsters() {
    let [judge, cylinder, mirror] = ["Unfair Judge", "Magic Cylinder", "Mirror Force"].map(card);
    let spawns = [mon(0, WARWOLF), mon(0, DARK_BLADE), hand(0, judge), set(1, cylinder), set(1, mirror), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(DARK_BLADE)], &[ChainOnAttack(cylinder)]]).unwrap();
    assert!(battle_chain(&s, judge).is_some(), "with monsters the Judge is offered: {}", s.prompt());
    s.run_both([&[Attack(WARWOLF)], &[ChainOnAttack(mirror)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "mzone", DARK_BLADE)), (0, 0), "Mirror Force destroyed both: {}", s.context());
    assert_eq!(battle_chain(&s, judge), None, "no monster, no target: {}", s.prompt());
}

// ---------------------------------------------------------------------------------------------------------------
// Thousand Punisher
// ---------------------------------------------------------------------------------------------------------------

/// Thousand Punisher: "Tribute 1 monster to target 1 face-up monster you control; Until the End Phase, its ATK becomes 1000
/// and can attack once for each 1000 ATK the Tributed monster had." Warwolf (2000) is Tributed for Thunder Kid: 1000 ATK,
/// two attacks, which are two direct attacks for 1000 each, and a third is not possible; the ATK is 700 again afterwards.
#[test]
fn thousand_punisher_gives_an_attack_for_each_1000_atk_of_the_tributed_monster() {
    let punisher = card("Thousand Punisher");
    let Some(mut s) = field(&[], &[hand(0, punisher), mon(0, WARWOLF), mon(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER), (0, WARWOLF), (0, THUNDER_KID)]);
    s.run_both([&[EndTurn, Activate(punisher)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was Tributed: {}", s.context());
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(1000), "{}", s.context());
    s.run(&[Attack(THUNDER_KID), Attack(THUNDER_KID)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "two direct attacks of 1000: {}", s.context());
    let err = s.run(&[Attack(THUNDER_KID)]).expect_err("no third attack");
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(700), "until the End Phase: {}", s.context());
}

/// Thousand Punisher: "can attack once for each 1000 ATK the Tributed monster had": a Tribute of 700 ATK (Thunder Kid, for
/// Warwolf) is no 1000 ATK, so the monster's ATK still becomes 1000 but it makes no attack at all (`engine/overrides/
/// c511002890.lua`: upstream left it its one normal attack).
#[test]
fn thousand_punisher_with_less_than_1000_atk_gives_no_attack() {
    let punisher = card("Thousand Punisher");
    let Some(mut s) = field(&[], &[hand(0, punisher), mon(0, THUNDER_KID), mon(0, WARWOLF)]) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER), (0, THUNDER_KID), (0, WARWOLF)]);
    s.run_both([&[EndTurn, Activate(punisher)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was Tributed: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "{}", s.context());
    let err = s.run(&[Attack(WARWOLF)]).expect_err("no attack for under 1000 ATK");
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Zero Gazer
// ---------------------------------------------------------------------------------------------------------------

/// Zero Gazer: "Activate only when a face-up Attack Position monster you control is selected as an attack target. That
/// monster's ATK becomes 0. Draw 1 card." Team 1's Thunder Kid (700) attacks my Dark Blade (1800): with Zero Gazer chained
/// Dark Blade's ATK is 0, so it is destroyed instead of Thunder Kid, 700 damage is taken, and one card is drawn. Without the
/// Trap, Thunder Kid dies.
#[test]
fn zero_gazer_makes_the_attacked_monster_0_atk_and_draws_a_card() {
    let gazer = card("Zero Gazer");
    let Some(mut s) = field(&[], &[set(0, gazer), mon(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let before = hand_size(&s, 0);
    s.run_both([&[Chain(gazer)], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", gazer), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "0 ATK loses to 700: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "one card drawn by the Trap (turn 2's draw is team 1's): {}", s.context());
    // The control: no Zero Gazer, the attacker dies.
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(THUNDER_KID)]]).unwrap();
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(1, "grave", THUNDER_KID)), (1, 1), "{}", s.context());
}

/// Zero Gazer: "face-up Attack Position": a monster in Defense Position that is attacked is no case for it, so the Trap is
/// not chained and the monster is not changed.
#[test]
fn zero_gazer_ignores_a_defense_position_target() {
    let gazer = card("Zero Gazer");
    let Some(mut s) = field(&[], &[set(0, gazer), mon_def(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn], &[Attack(THUNDER_KID), Chain(gazer)]]).expect_err("Defense Position");
    assert!(err.contains("never became possible") || err.contains("can't attack"), "{err}");
    assert_eq!(s.count(0, "szone", gazer), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Card from a Different Dimension
// ---------------------------------------------------------------------------------------------------------------

/// Cards a team has drawn so far, as every Feral Imp in its hand and Graveyard (the hand limit discards some).
fn imps_seen(s: &Scenario, team: usize) -> usize {
    s.count(team, "hand", FILLER) + s.count(team, "grave", FILLER)
}

/// Card from a Different Dimension: "If this card is banished: Activate this effect; during your next Standby Phase, add it
/// to your hand. If this card is returned to the hand this way: Both players draw 2 cards." Banished as the duel starts (so
/// the next Standby Phase is turn 1's: a card banished in the Main Phase waits for turn 3, in
/// `time_fusion_banishes_a_card_from_the_hand_and_summons_the_fusion_monster_at_the_next_standby_phase`), it comes back to
/// the hand and both players draw 2 cards, 2 more Feral Imps each than in a duel of the same turns without it.
#[test]
fn card_from_a_different_dimension_comes_back_at_the_next_standby_phase_and_both_draw_2() {
    let dimension = card("Card from a Different Dimension");
    let Some(mut s) = field(&[], &[banished(0, dimension)]) else { return };
    keep_hands(&mut s);
    assert_eq!(s.count(0, "removed", dimension), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", dimension), 1, "it came back to the hand: {}", s.context());
    assert_eq!(s.count(0, "removed", dimension), 0, "{}", s.context());
    let Some(mut control) = field(&[], &[]) else { return };
    keep_hands(&mut control);
    control.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(imps_seen(&s, 0), imps_seen(&control, 0) + 2, "team 0 drew 2: {}", s.context());
    assert_eq!(imps_seen(&s, 1), imps_seen(&control, 1) + 2, "team 1 drew 2: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Time Fusion
// ---------------------------------------------------------------------------------------------------------------

const POLYMERIZATION: u16 = 4837;

/// A duel where team 0 holds what it takes to Fusion Summon Gaia the Dragon Champion (Polymerization, Gaia The Fierce Knight
/// and Curse of Dragon in the hand, Gaia in the Extra Deck) besides `more`. A Fusion Monster put on the field by the
/// scenario's spawn was never summoned by its procedure, and the core never lets such a card be Special Summoned from the
/// Graveyard again (`EFFECT_REVIVE_LIMIT`), as it does not for a card in a real duel, so Gaia comes by Polymerization.
fn fusion_field(more: &[Spawn]) -> Option<Scenario> {
    let [gaia_knight, curse] = ["Gaia The Fierce Knight", "Curse of Dragon"].map(card);
    let mut spawns = vec![hand(0, POLYMERIZATION), hand(0, gaia_knight), hand(0, curse)];
    spawns.extend_from_slice(more);
    field_extra([&[card("Gaia the Dragon Champion")], &[]], &spawns)
}

/// Time Fusion: "Banish 1 card from your hand; choose 1 Fusion Monster in the Graveyard, Extra Deck, or that is banished, and
/// that was destroyed on your field this turn, Special Summon it during your next Standby Phase, ignoring its Summoning
/// conditions, but it cannot attack that turn." Gaia the Dragon Champion is Fusion Summoned and destroyed by Dark Hole in
/// turn 1 and Time Fusion banishes a Feral Imp from the hand: Gaia waits in the Graveyard through turn 2 and is Special
/// Summoned in turn 3's Standby Phase (2600/2100, face-up Attack Position), where it is not allowed to attack.
#[test]
fn time_fusion_banishes_a_card_from_the_hand_and_summons_the_fusion_monster_at_the_next_standby_phase() {
    let [fusion, gaia] = ["Time Fusion", "Gaia the Dragon Champion"].map(card);
    let Some(mut s) = fusion_field(&[hand(0, DARK_HOLE), hand(0, fusion)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(POLYMERIZATION), Activate(DARK_HOLE), Activate(fusion)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", gaia), 1, "Dark Hole destroyed Gaia: {}", s.context());
    assert_eq!(s.count(0, "removed", FILLER), 1, "a card of the hand was banished as the cost: {}", s.context());
    assert_eq!(s.count(0, "grave", fusion), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", gaia), 0, "not before the Standby Phase: {}", s.context());
    // Turn 3's Standby Phase.
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", gaia), 1, "Gaia is back: {}", s.context());
    assert_eq!(s.count(0, "grave", gaia), 0, "{}", s.context());
    let g = stats(&s, 0, gaia);
    assert_eq!((g.0, g.1, g.2), (Some(2600), Some(2100), Some(7)), "{}", s.context());
    assert_eq!(s.position_of(0, gaia), Some(1), "face-up Attack: {}", s.context());
    let err = s.run(&[Attack(gaia)]).expect_err("it cannot attack that turn");
    assert!(err.contains("can't attack"), "{err}");
}

/// Time Fusion: "choose 1 Fusion Monster ... destroyed on your field this turn": with Gaia alive on the field there is none,
/// so the Spell is not offered; and a Fusion Monster destroyed in turn 1 is no choice in turn 3.
#[test]
fn time_fusion_needs_a_fusion_monster_destroyed_this_turn() {
    let [fusion, gaia] = ["Time Fusion", "Gaia the Dragon Champion"].map(card);
    let Some(mut s) = fusion_field(&[hand(0, fusion)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(POLYMERIZATION), Activate(fusion)]).expect_err("nothing was destroyed");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", gaia), 1, "{}", s.context());
    let Some(mut s) = fusion_field(&[hand(0, DARK_HOLE), hand(0, fusion)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Activate(POLYMERIZATION), Activate(DARK_HOLE), EndTurn, Activate(fusion)], &[EndTurn]]).expect_err("not destroyed this turn");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", gaia), 1, "{}", s.context());
}

/// Time Fusion's cost banishes Card from a Different Dimension: banished in the Main Phase of turn 1 it waits for "your next
/// Standby Phase", turn 3's (not the opponent's turn 2), where it returns to the hand and both players draw 2 cards, while
/// Gaia comes back in the same Standby Phase.
#[test]
fn a_card_from_a_different_dimension_banished_by_time_fusion_returns_in_turn_3() {
    let [fusion, gaia, dimension] = ["Time Fusion", "Gaia the Dragon Champion", "Card from a Different Dimension"].map(card);
    let Some(mut s) = fusion_field(&[hand(0, DARK_HOLE), hand(0, fusion), hand(0, dimension)]) else { return };
    s.prefer(&[(0, dimension), (0, FILLER), (1, FILLER)]);
    s.run_both([&[Activate(POLYMERIZATION), Activate(DARK_HOLE), Activate(fusion)], &[]]).unwrap();
    assert_eq!(s.count(0, "removed", dimension), 1, "banished as the cost: {}", s.context());
    clean(&s);
    let drawn = imps_seen(&s, 1);
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", dimension), 1, "back in the hand at turn 3's Standby Phase: {}", s.context());
    assert_eq!(s.count(0, "mzone", gaia), 1, "{}", s.context());
    // Team 1 drew its turn-2 card before and 2 now.
    assert_eq!(imps_seen(&s, 1), drawn + 1 + 2, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Silent Pain, Silent Space and Silent Torment
// ---------------------------------------------------------------------------------------------------------------

/// Silent Pain: "Once per turn, during your opponent's Standby Phase, if this is the only card in your Graveyard: Destroy all
/// monsters your opponent controls, also, inflict 1000 damage to your opponent." Alone in team 0's Graveyard it destroys
/// Warwolf and Dark Blade at turn 2's Standby Phase (team 1 takes 1000); with another card in the Graveyard it does
/// nothing, and neither does it in the turn of its controller.
#[test]
fn silent_pain_destroys_the_opponents_monsters_in_their_standby_phase_when_alone_in_the_graveyard() {
    let pain = card("Silent Pain");
    let Some(mut s) = field(&[], &[grave(0, pain), mon(1, WARWOLF), mon(1, DARK_BLADE), mon(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    assert_eq!((s.count(1, "mzone", WARWOLF), s.lp(1)), (1, 8000), "nothing in my own turn 1: {}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "both were destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    assert_eq!((s.count(0, "mzone", THUNDER_KID), s.lp(0)), (1, 8000), "my side is untouched: {}", s.context());
    assert_eq!(s.count(0, "grave", pain), 1, "it stays in the Graveyard: {}", s.context());
    // Not the only card in the Graveyard.
    let Some(mut s) = field(&[], &[grave(0, pain), grave(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", WARWOLF), s.lp(1)), (1, 8000), "{}", s.context());
}

/// Silent Torment: "During your opponent's turn, while "Silent Pain" is in your Graveyard: you can activate this card from
/// your hand by banishing this card; inflict 1000 damage to your opponent." Team 1 Normal Summons a monster in turn 2 and
/// team 0 answers from its hand: Silent Torment is banished and team 1 takes 1000. Without "Silent Pain" in the Graveyard it
/// is not offered.
#[test]
fn silent_torment_inflicts_1000_damage_in_the_opponents_turn_while_silent_pain_is_in_the_graveyard() {
    let [pain, torment] = ["Silent Pain", "Silent Torment"].map(card);
    let Some(mut s) = field(&[], &[grave(0, pain), grave(0, THUNDER_KID), hand(0, torment)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(torment)], &[Summon(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", torment), 1, "banished as its cost: {}", s.context());
    assert_eq!((s.lp(1), s.lp(0)), (8000 - 1000, 8000), "{}", s.context());
    let Some(mut s) = field(&[], &[grave(0, THUNDER_KID), hand(0, torment)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(torment)], &[Summon(FILLER)]]).expect_err("no Silent Pain in the Graveyard");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Silent Space: "During your opponent's turn, while "Silent Pain" is in your Graveyard: You can activate this card from your
/// hand by banishing this card to target 1 monster your opponent controls; destroy it." Team 0 answers team 1's Summon by
/// destroying its Warwolf, not the Feral Imp just summoned; Silent Space is banished.
#[test]
fn silent_space_destroys_a_monster_in_the_opponents_turn_while_silent_pain_is_in_the_graveyard() {
    let [pain, space] = ["Silent Pain", "Silent Space"].map(card);
    let Some(mut s) = field(&[], &[grave(0, pain), grave(0, THUNDER_KID), hand(0, space), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, FILLER), (1, FILLER), (1, WARWOLF)]);
    s.run_both([&[Chain(space)], &[Summon(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", space), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", FILLER)), (1, 1), "only the chosen monster: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    let Some(mut s) = field(&[], &[grave(0, THUNDER_KID), hand(0, space), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(space)], &[Summon(FILLER)]]).expect_err("no Silent Pain in the Graveyard");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Fool Clown, Rough Exploder and Rough Fight
// ---------------------------------------------------------------------------------------------------------------

/// Fool Clown: "When this card is Summoned, send all cards in your hand to the Graveyard." Normal Summoned, Flip Summoned
/// (Set in turn 1, flipped in turn 3) and Special Summoned (Monster Reborn from the Graveyard) it empties the hand, and
/// the opponent's hand is left alone.
#[test]
fn fool_clown_sends_the_whole_hand_to_the_graveyard_when_summoned() {
    let clown = card("Fool Clown");
    let Some(mut s) = field(&[], &[hand(0, clown)]) else { return };
    keep_hands(&mut s);
    let (cards, theirs) = (hand_size(&s, 0) - 1, hand_size(&s, 1));
    s.run(&[Summon(clown)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), 0, "the hand is gone: {}", s.context());
    assert_eq!((s.count(0, "grave", FILLER), s.count(0, "mzone", clown)), (cards, 1), "{}", s.context());
    assert_eq!(hand_size(&s, 1), theirs, "{}", s.context());
    // Flip Summoned.
    let Some(mut s) = field(&[], &[hand(0, clown)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[SetMonster(clown), EndTurn], &[EndTurn]]).unwrap();
    assert!(hand_size(&s, 0) > 0, "the Set did not send the hand away: {}", s.context());
    s.run(&[Reposition(clown)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", clown), 1, "{}", s.context());
    // Special Summoned.
    let Some(mut s) = field(&[], &[grave(0, clown), hand(0, MONSTER_REBORN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, clown)]);
    s.run(&[Activate(MONSTER_REBORN)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", clown), hand_size(&s, 0)), (1, 0), "{}", s.context());
}

/// Rough Exploder: "If you control an Attack Position "Fool Clown": You can banish this card from your Graveyard; inflict 500
/// damage to your opponent." With Fool Clown in Attack Position the opponent takes 500 and the card is banished; with
/// Fool Clown in Defense Position, or none, it is not offered.
#[test]
fn rough_exploder_inflicts_500_damage_while_an_attack_position_fool_clown_is_controlled() {
    let [exploder, clown] = ["Rough Exploder", "Fool Clown"].map(card);
    let Some(mut s) = field(&[], &[grave(0, exploder), mon(0, clown)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(exploder)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
    assert_eq!((s.count(0, "removed", exploder), s.count(0, "grave", exploder)), (1, 0), "banished as its cost: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    for spawns in [vec![grave(0, exploder), mon_def(0, clown)], vec![grave(0, exploder)]] {
        let Some(mut s) = field(&[], &spawns) else { return };
        keep_hands(&mut s);
        let err = s.run(&[Activate(exploder)]).expect_err("no Attack Position Fool Clown");
        assert!(err.contains("never became possible"), "{err}");
        assert_eq!(s.lp(1), 8000, "{}", s.context());
    }
}

/// Rough Exploder is itself a monster (Level 1 Pyro, FIRE) of the Fool Clown archetype that works from the Graveyard.
#[test]
fn rough_exploder_is_a_level_1_pyro_monster() {
    let exploder = card("Rough Exploder");
    let Some(s) = field(&[], &[mon(0, exploder)]) else { return };
    let e = stats(&s, 0, exploder);
    assert_eq!((e.2, e.3, e.4), (Some(1), 0x80, 0x4), "Level 1, Pyro, FIRE: {}", s.context());
}

/// Rough Fight: "If you take damage while you control a face-up Attack Position "Fool Clown": you can banish this card from
/// your Graveyard; inflict damage to your opponent equal to the damage you took." Team 1's Just Desserts (500 for each monster I
/// control: Fool Clown and Thunder Kid, so 1000) hurts me, and Rough Fight gives the 1000 back; with Fool Clown in
/// Defense Position it does not.
#[test]
fn rough_fight_inflicts_the_damage_taken_while_an_attack_position_fool_clown_is_controlled() {
    let [fight, clown, desserts] = ["Rough Fight", "Fool Clown", "Just Desserts"].map(card);
    let spawns = [grave(0, fight), mon(0, clown), mon(0, THUNDER_KID), set(1, desserts)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(fight)], &[Activate(desserts)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000, "Just Desserts: 500 for each of 2 monsters: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1000, "Rough Fight gave back the damage taken: {}", s.context());
    assert_eq!((s.count(0, "removed", fight), s.count(0, "grave", fight)), (1, 0), "{}", s.context());
    let spawns = [grave(0, fight), mon_def(0, clown), mon(0, THUNDER_KID), set(1, desserts)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(fight)], &[Activate(desserts)]]).expect_err("Defense Position");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1000, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Goddess Spells: Footsteps, Mischief and Temptation
// ---------------------------------------------------------------------------------------------------------------

/// Footsteps of the Goddess: "Look at your opponent's hand, choose 1 monster in it with 1500 or less ATK, and Special Summon it
/// to your opponent's side of the field in face-up Attack Position. Then, You can Special Summon 1 monster from your hand with
/// 1500 or less ATK in face-up Attack Position. The monsters Special Summoned by this effect cannot attack this turn." In turn 3
/// a Feral Imp (1300) comes from team 1's hand to team 1's field, and one from mine to mine: of the monsters I control only
/// Warwolf may attack, the Imp may not.
#[test]
fn footsteps_of_the_goddess_summons_a_monster_from_the_opponents_hand_and_one_of_mine_that_cannot_attack() {
    let footsteps = card("Footsteps of the Goddess");
    let Some(mut s) = field(&[], &[hand(0, footsteps), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(footsteps)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", FILLER), 1, "a Feral Imp of team 1's hand is on team 1's field: {}", s.context());
    assert_eq!(s.position_of(1, FILLER), Some(1), "face-up Attack: {}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 1, "and one of mine on mine: {}", s.context());
    assert_eq!(s.position_of(0, FILLER), Some(1), "{}", s.context());
    assert_eq!(s.count(0, "grave", footsteps), 1, "{}", s.context());
    // The Special Summoned monsters cannot attack: the Battle Phase offers Warwolf only.
    let err = s.run(&[Attack(FILLER)]).expect_err("the Special Summoned Feral Imp cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{WARWOLF}")) && !err.contains(&format!("\"cid\":{FILLER}")), "{err}");
}

/// Footsteps of the Goddess: "1 monster in it with 1500 or less ATK": team 1's hand of Dark Magicians (2500) holds none, so
/// nothing comes to its field.
#[test]
fn footsteps_of_the_goddess_needs_a_monster_of_1500_atk_or_less() {
    let footsteps = card("Footsteps of the Goddess");
    let Some(mut s) = duel(7, 8000, [FILLER, DARK_MAGICIAN], &[], &[hand(0, footsteps)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(footsteps)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", footsteps), 1, "{}", s.context());
}

/// Mischief of the Goddess: "Look at your opponent's hand. If a Spell is in their hand, Set 1 of those cards to your opponent's
/// side of the field." Dian Keto in team 1's hand is Set in its Spell & Trap Zone, and team 1 holds no other Spell.
#[test]
fn mischief_of_the_goddess_sets_a_spell_of_the_opponents_hand_on_their_side() {
    let mischief = card("Mischief of the Goddess");
    let Some(mut s) = field(&[], &[hand(0, mischief), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    let theirs = hand_size(&s, 1);
    s.run(&[Activate(mischief)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", DIAN_KETO), 1, "Set on team 1's side: {}", s.context());
    assert_eq!(hand_size(&s, 1), theirs - 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", mischief), 1, "{}", s.context());
}

/// Temptation of the Goddess: "Look at your opponent's hand. If a Level 4 or lower monster is in their hand, you can Special
/// Summon that monster to their side of the field in face-up Attack Position." A Feral Imp comes to team 1's field; Dark
/// Magicians (Level 7) stay in the hand.
#[test]
fn temptation_of_the_goddess_summons_a_level_4_or_lower_monster_of_the_opponents_hand_to_their_side() {
    let temptation = card("Temptation of the Goddess");
    let Some(mut s) = field(&[], &[hand(0, temptation)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(temptation)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", FILLER), 1, "{}", s.context());
    assert_eq!(s.position_of(1, FILLER), Some(1), "face-up Attack: {}", s.context());
    let Some(mut s) = duel(7, 8000, [FILLER, DARK_MAGICIAN], &[], &[hand(0, temptation)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(temptation)]).unwrap();
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Insect Garden, Insect Pheromone and Magical Longicorn
// ---------------------------------------------------------------------------------------------------------------

/// Insect Garden: "While this card is face-up on the field, you can give control of any Level 4 or lower Insect-Type monster on
/// your field to your opponent." With Killer Needle (Level 4 Insect) and Warwolf on my side, the Garden's Field Spell gives
/// the Needle to the opponent (asked when the Garden is activated) and leaves Warwolf, which is no Insect.
#[test]
fn insect_garden_gives_a_level_4_or_lower_insect_to_the_opponent() {
    let [garden, needle] = ["Insect Garden", "Killer Needle"].map(card);
    let Some(mut s) = field(&[], &[hand(0, garden), mon(0, needle), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(garden)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", needle), s.count(1, "mzone", needle)), (0, 1), "the Needle changed sides: {}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", WARWOLF)), (1, 0), "Warwolf is no Insect: {}", s.context());
}

/// Insect Garden: "Level 4 or lower": Insect Princess (Level 6) stays.
#[test]
fn insect_garden_leaves_an_insect_above_level_4() {
    let [garden, princess] = ["Insect Garden", "Insect Princess"].map(card);
    let Some(mut s) = field(&[], &[hand(0, garden), mon(0, princess)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(garden)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", princess), s.count(1, "mzone", princess)), (1, 0), "{}", s.context());
}

/// Insect Pheromone: "Equip only to "Insect Princess". Once per turn, if the equipped monster destroys a monster by battle: You
/// can activate this effect; 1 face-up Insect-Type monster your opponent controls must battle the equipped monster this Battle
/// Phase." Insect Princess (Level 6, 1900/1200) with the Pheromone attacks Thunder Kid in turn 3 (1200 damage) and then Killer
/// Needle (1200 ATK) must battle it: it is destroyed and 700 more damage is dealt, 6100 left. An Equip Spell for any other
/// monster is not offered (Warwolf).
#[test]
fn insect_pheromone_makes_an_insect_battle_the_equipped_insect_princess() {
    let [pheromone, princess, needle] = ["Insect Pheromone", "Insect Princess", "Killer Needle"].map(card);
    let spawns = [hand(0, pheromone), mon(0, princess), mon(1, THUNDER_KID), mon(1, needle)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(pheromone), Attack(princess), Chain(pheromone)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", pheromone), 1, "{}", s.context());
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "grave", needle)), (1, 1), "both were destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1200 - 700, "the forced battle dealt 1900 - 1200: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    // Equip only to Insect Princess.
    let Some(mut s) = field(&[], &[hand(0, pheromone), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(pheromone)]).expect_err("only Insect Princess");
    assert!(err.contains("never became possible"), "{err}");
}

/// Insect Pheromone: the forced battle needs "1 face-up Insect-Type monster your opponent controls": with only Thunder Kid
/// there, the Princess' victory does nothing more.
#[test]
fn insect_pheromone_needs_an_insect_to_force_into_battle() {
    let [pheromone, princess] = ["Insect Pheromone", "Insect Princess"].map(card);
    let Some(mut s) = field(&[], &[hand(0, pheromone), mon(0, princess), mon(1, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, THUNDER_KID)]);
    let err = s.run_both([&[EndTurn, Activate(pheromone), Attack(princess), Chain(pheromone)], &[EndTurn]]).expect_err("no Insect");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Magical Longicorn: "While this card is in face-up Attack Position, negate the activations of your Spell Cards." My Dian
/// Keto is negated with the Longicorn (Level 4, 600/1600) in Attack Position (8000 LP, the Spell in the Graveyard), and works
/// with it in Defense Position (9000); the opponent's Dian Keto is not negated by it.
#[test]
fn magical_longicorn_negates_its_controllers_spells_while_in_attack_position() {
    let longicorn = card("Magical Longicorn");
    let Some(mut s) = field(&[], &[mon(0, longicorn), hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the Spell was negated: {}", s.context());
    let Some(mut s) = field(&[], &[mon_def(0, longicorn), hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DIAN_KETO)]).unwrap();
    assert_eq!(s.lp(0), 9000, "Defense Position: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, longicorn), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 9000), "its controller's Spells only: {}", s.context());
    let l = stats(&s, 0, longicorn);
    assert_eq!((l.0, l.1, l.2), (Some(600), Some(1600), Some(4)), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Satellite Cannon's cards: Charge, Debris Station and Satellite Base
// ---------------------------------------------------------------------------------------------------------------

/// Charge: "All face-up "Satellite Cannons" you control gain 2000 ATK." The anime Satellite Cannon is a 0 ATK monster, so two
/// of them are 2000 each afterwards while Warwolf stays 2000 and the opponent's Satellite Cannon keeps 0.
#[test]
fn charge_gives_every_satellite_cannon_2000_atk() {
    let [charge, cannon] = ["Charge", "Satellite Cannon"].map(card);
    let spawns = [hand(0, charge), mon(0, cannon), mon(0, cannon), mon(0, THUNDER_KID), mon(1, cannon)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let c = stats(&s, 0, cannon);
    assert_eq!((c.0, c.2, c.3, c.4), (Some(0), Some(5), 0x20, LIGHT), "anime Satellite Cannon: Level 5 Machine, 0 ATK: {}", s.context());
    s.run(&[Activate(charge)]).unwrap();
    clean(&s);
    let mine: Vec<_> = s.state()[0]["mzone"].as_array().unwrap().iter().filter(|c| c["cid"] == cannon).map(|c| c["attack"].as_i64()).collect();
    assert_eq!(mine, vec![Some(2000), Some(2000)], "{}", s.context());
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(700), "{}", s.context());
    assert_eq!(s.attack_of(1, cannon), Some(0), "the opponent's Cannon is not mine: {}", s.context());
    assert_eq!(s.count(0, "grave", charge), 1, "{}", s.context());
}

/// Charge: with no Satellite Cannon on my side the Spell is not offered.
#[test]
fn charge_needs_a_satellite_cannon() {
    let charge = card("Charge");
    let Some(mut s) = field(&[], &[hand(0, charge), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(charge)]).expect_err("no Satellite Cannon");
    assert!(err.contains("never became possible"), "{err}");
}

/// Debris Station: "When a "Satellite Cannon" you control is destroyed by battle and sent to the Graveyard: Send 2 "Satellite
/// Cannons" from your side of the field or hand to the Graveyard; Special Summon 1 "Satellite Laser Balsam" from your Extra
/// Deck (This Special Summon is treated as a Fusion Summon.), and if you do, it gains 3000 ATK." Team 1's Blue-Eyes White
/// Dragon (Level 8, so the Cannon cannot negate its attack) destroys my 0 ATK Cannon; the Trap sends the two Cannons of my hand
/// to the Graveyard and Satellite Laser Balsam (0/2000, Level 8) comes in face-up Attack Position with 3000 ATK.
#[test]
fn debris_station_fusion_summons_satellite_laser_balsam_from_the_extra_deck() {
    let [station, cannon, balsam, blue_eyes] = ["Debris Station", "Satellite Cannon", "Satellite Laser Balsam", "Blue-Eyes White Dragon"].map(card);
    let spawns = [set(0, station), mon(0, cannon), hand(0, cannon), hand(0, cannon), mon(1, blue_eyes)];
    let Some(mut s) = field_extra([&[balsam], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(station)], &[Attack(blue_eyes)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cannon), 3, "the destroyed Cannon and the two sent by the Trap: {}", s.context());
    assert_eq!(s.count(0, "hand", cannon), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", balsam), 1, "Balsam is on the field: {}", s.context());
    let b = stats(&s, 0, balsam);
    assert_eq!((b.0, b.1, b.2), (Some(3000), Some(2000), Some(8)), "0 ATK and 3000 gained: {}", s.context());
    assert_eq!(s.position_of(0, balsam), Some(1), "face-up Attack: {}", s.context());
    assert_eq!(s.count(0, "grave", station), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 3000, "the battle damage was still dealt: {}", s.context());
}

/// Debris Station: "Send 2 "Satellite Cannons" from your side of the field or hand": with only one Cannon besides the
/// destroyed one the Trap is not chained, and Satellite Laser Balsam stays in the Extra Deck.
#[test]
fn debris_station_needs_two_more_satellite_cannons() {
    let [station, cannon, balsam, blue_eyes] = ["Debris Station", "Satellite Cannon", "Satellite Laser Balsam", "Blue-Eyes White Dragon"].map(card);
    let spawns = [set(0, station), mon(0, cannon), hand(0, cannon), mon(1, blue_eyes)];
    let Some(mut s) = field_extra([&[balsam], &[]], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(station)], &[Attack(blue_eyes)]]).expect_err("only one Cannon is left");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!((s.count(0, "mzone", balsam), s.count(0, "szone", station)), (0, 1), "{}", s.context());
}

/// Satellite Cannon (the anime card): "Negate attacks that target "Satellite Cannon"" from monsters of Level 7 or lower:
/// Warwolf (Level 4) attacks my Cannon in turn 2 and the attack is negated, no damage and the Cannon stays; that is why
/// Debris Station's case needs a stronger attacker.
#[test]
fn satellite_cannon_negates_the_attack_of_a_monster_of_level_7_or_lower() {
    let cannon = card("Satellite Cannon");
    let Some(mut s) = field(&[], &[mon(0, cannon), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", cannon), s.lp(0)), (1, 8000), "{}", s.context());
}

/// Satellite Base: "Cannot be destroyed by battle with a Level 5 or lower monster. Once per turn, during your End Phase: This
/// card gains 1000 DEF. After damage calculation, if an opponent's monster attacks this card, this card's DEF becomes 0."
/// My Base (0/0, Level 4 Machine) is 1000 DEF after turn 1's End Phase; Warwolf (Level 4) attacks it in turn 2, it survives and
/// its DEF is 0 again; Blue-Eyes White Dragon (Level 8) destroys it.
#[test]
fn satellite_base_survives_low_level_monsters_and_loses_its_defense_when_attacked() {
    let [base, blue_eyes] = ["Satellite Base", "Blue-Eyes White Dragon"].map(card);
    let Some(mut s) = field(&[], &[mon_def(0, base), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[]]).unwrap();
    assert_eq!(stats(&s, 0, base).1, Some(1000), "+1000 DEF in the End Phase: {}", s.context());
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", base), 1, "Level 4 cannot destroy it: {}", s.context());
    assert_eq!(stats(&s, 0, base).1, Some(0), "its DEF became 0 after the attack: {}", s.context());
    let Some(mut s) = field(&[], &[mon_def(0, base), mon(1, blue_eyes)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(blue_eyes)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", base), s.count(0, "mzone", base)), (1, 0), "Level 8 destroys it: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Explosion Wing, Glory Level Talisman and Level Tax
// ---------------------------------------------------------------------------------------------------------------

/// Explosion Wing: "Inflict 500 damage to your opponent for each card destroyed by a card effect this turn." In turn 3 Dark
/// Hole destroys three monsters (mine and two of the opponent's): the Trap deals 1500. A monster destroyed by battle is no
/// count, and with nothing destroyed by an effect the Trap is not offered.
#[test]
fn explosion_wing_inflicts_500_damage_for_each_card_destroyed_by_an_effect_this_turn() {
    let wing = card("Explosion Wing");
    let spawns = [set(0, wing), hand(0, DARK_HOLE), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(DARK_HOLE), Activate(wing)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 3 * 500, "3 cards destroyed by Dark Hole: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(0, "grave", wing), 1, "{}", s.context());
    // A battle destruction is not counted: Warwolf destroys Thunder Kid.
    let spawns = [set(0, wing), mon(0, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(WARWOLF), Activate(wing)], &[EndTurn]]).expect_err("only a battle destruction");
    assert!(err.contains("never became possible") || err.contains("can't attack"), "{err}");
    assert_eq!(s.lp(1), 8000 - 1300, "{}", s.context());
}

/// Glory Level Talisman (a Continuous Trap, Set first): "Equip this card to a Level 5 or higher monster. While your opponent controls no monsters with a Level
/// higher than the equipped monster's, the equipped monster cannot be targeted by cards or effects. When the equipped monster is
/// targeted for an attack target by a monster whose Level is lower than the equipped monster's, destroy the attacking monster."
/// The Talisman is activated in turn 3, equipped to Dark Magician (Level 7); Warwolf (Level 4) attacks it in turn 4 and is
/// destroyed, with no damage.
#[test]
fn glory_level_talisman_destroys_a_lower_level_monster_attacking_the_equipped_monster() {
    let talisman = card("Glory Level Talisman");
    let Some(mut s) = field(&[], &[set(0, talisman), mon(0, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(talisman)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", talisman), 1, "equipped to Dark Magician: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the attacker was destroyed: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
}

/// Glory Level Talisman: "Level 5 or higher": Dark Blade (Level 4) is no target for it, so with only Dark Blade it is not
/// offered. And "a Level lower than the equipped monster's": Gaia the Dragon Champion, also Level 7 (2600 ATK), attacks Dark
/// Magician (2500) and is not destroyed by the Talisman: Dark Magician is the one destroyed.
#[test]
fn glory_level_talisman_needs_a_level_5_monster_and_a_lower_level_attacker() {
    let [talisman, gaia] = ["Glory Level Talisman", "Gaia the Dragon Champion"].map(card);
    let Some(mut s) = field(&[], &[set(0, talisman), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(talisman)], &[EndTurn]]).expect_err("Level 4");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = field(&[], &[set(0, talisman), mon(0, DARK_MAGICIAN), mon(1, gaia)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(talisman)], &[EndTurn, Attack(gaia)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", talisman) + s.count(0, "grave", talisman), 1, "{}", s.context());
    assert_eq!((s.count(1, "mzone", gaia), s.count(0, "grave", DARK_MAGICIAN)), (1, 1), "an equal Level is not destroyed by the Talisman: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 100, "{}", s.context());
}

/// Glory Level Talisman: "While your opponent controls no monsters with a Level higher than the equipped monster's, the equipped
/// monster cannot be targeted by cards or effects." Team 1's Offerings to the Doomed (target 1 face-up monster) has no target
/// while Dark Magician (Level 7) is the only monster; with Blue-Eyes White Dragon (Level 8) on the opposite side it can
/// target Dark Magician and destroys it.
#[test]
fn glory_level_talisman_protects_from_targeting_unless_the_opponent_has_a_higher_level_monster() {
    let [talisman, offerings, blue_eyes] = ["Glory Level Talisman", "Offerings to the Doomed", "Blue-Eyes White Dragon"].map(card);
    let Some(mut s) = field(&[], &[set(0, talisman), mon(0, DARK_MAGICIAN), hand(1, offerings)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(talisman)], &[EndTurn, Activate(offerings)]]).expect_err("untargetable");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, talisman), mon(0, DARK_MAGICIAN), hand(1, offerings), mon(1, blue_eyes)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER), (1, FILLER), (0, DARK_MAGICIAN)]);
    s.run_both([&[EndTurn, Activate(talisman)], &[EndTurn, Activate(offerings)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "a Level 8 monster lifts the protection: {}", s.context());
}

/// Level Tax: "Players cannot Summon a Level 5 or higher monster unless they pay LP equal to its ATK." Summoned Skull (Level 6,
/// 2500 ATK) is Tribute Summoned over Warwolf and costs 2500 LP (5500 left); team 1's Skull costs team 1 the same in turn 2;
/// with 2000 LP left no Summoned Skull can be Summoned, while a Level 4 monster still can.
#[test]
fn level_tax_makes_both_players_pay_the_atk_of_a_level_5_or_higher_monster_they_summon() {
    let [tax, skull] = ["Level Tax", "Summoned Skull"].map(card);
    let Some(mut s) = field(&[], &[hand(0, tax), hand(0, skull), mon(0, WARWOLF)]) else { return };
    s.prefer(&[(0, WARWOLF), (1, FILLER), (0, FILLER)]);
    s.run(&[Activate(tax), Summon(skull)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", skull), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2500, "paid the Skull's ATK: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, tax), hand(1, skull), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(1, WARWOLF), (1, FILLER), (0, FILLER)]);
    s.run_both([&[Activate(tax)], &[Summon(skull)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", skull), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2500), "the opponent pays too: {}", s.context());
    // At 2000 LP the Skull cannot be Summoned, a Level 4 monster can.
    let Some(mut s) = duel(7, 2000, [FILLER, FILLER], &[], &[hand(0, tax), hand(0, skull), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(tax)]).unwrap();
    assert!(!offered(&s, "summons").contains(&skull.into()), "{:?}", offered(&s, "summons"));
    assert!(offered(&s, "summons").contains(&FILLER.into()), "{:?}", offered(&s, "summons"));
}

// ---------------------------------------------------------------------------------------------------------------
// Power Gift, Volcanic Curse, Warp Beam and Power Load
// ---------------------------------------------------------------------------------------------------------------

/// Power Gift: "Send 1 monster from your hand to the Graveyard to target 1 face-up monster your opponent controls; It gains ATK
/// equal to the ATK of the sent monster. If that monster battles, destroy it at the end of the Damage Step." Warwolf (2000) is
/// sent: the opponent's Thunder Kid is 2700; it destroys my attacking Dark Blade (1800) and is itself destroyed at the end of
/// the Damage Step.
#[test]
fn power_gift_gives_the_atk_of_the_sent_monster_and_destroys_the_monster_that_battles() {
    let gift = card("Power Gift");
    let spawns = [hand(0, gift), hand(0, WARWOLF), mon(0, DARK_BLADE), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run(&[Activate(gift)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "sent as the cost: {}", s.context());
    assert_eq!(s.attack_of(1, THUNDER_KID), Some(2700), "{}", s.context());
    s.run_both([&[EndTurn, Attack(DARK_BLADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "2700 beat 1800: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "the monster that battled is destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 900, "{}", s.context());
}

/// Volcanic Curse: "When an opponent's monster declares an attack targeting a face-up monster you control: Target that monster
/// you control; that target gains 500 ATK for each Pyro-Type monster in the Graveyard." Two Rough Exploders (Pyro) lie in the
/// Graveyards, so Thunder Kid (700) is 1700 against the attacking Feral Imp (1300): the Imp is destroyed and 400 damage is dealt.
#[test]
fn volcanic_curse_gives_500_atk_for_each_pyro_monster_in_the_graveyards() {
    let [curse, exploder] = ["Volcanic Curse", "Rough Exploder"].map(card);
    let spawns = [set(0, curse), mon(0, THUNDER_KID), grave(0, exploder), grave(1, exploder), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(curse)], &[Attack(FILLER)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, THUNDER_KID), Some(700 + 1000), "{}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 1, "the attacker was destroyed: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 400), "{}", s.context());
}

/// Warp Beam: "Target any number of monsters you control, then send an equal number of cards you control to the Graveyard,
/// except the targeted monster(s); This turn, the ATK of those monsters becomes 600, and can attack directly." Dark Blade is
/// the target and Warwolf is sent: Dark Blade is 600 and may attack directly though the opponent has a monster.
#[test]
fn warp_beam_makes_the_targeted_monster_600_atk_and_able_to_attack_directly() {
    let beam = card("Warp Beam");
    let spawns = [hand(0, beam), mon(0, DARK_BLADE), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DARK_BLADE), (0, WARWOLF), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(beam)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "an equal number of cards was sent: {}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(600), "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), None, "{}", s.context());
    s.respond(json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
    for _ in 0..6 {
        if s.prompt()["type"] == msg::SELECT_BATTLECMD {
            break;
        }
        s.pass().unwrap();
    }
    let direct = s.prompt()["attacks"].as_array().unwrap().iter().any(|a| a["cid"] == DARK_BLADE && a["can_direct"] == true);
    assert!(direct, "Dark Blade can attack directly: {}", s.prompt());
}

/// Power Load: "Equip only to a Machine monster. It gains 400 ATK for each attack your other Machine monsters can make. All
/// your monsters cannot attack, except the equipped monster." Mechanicalchaser (1850) is equipped in turn 3 with another
/// Machine (Overdrive) and Warwolf (no Machine) on my side: 2250 ATK, and it alone may attack (a direct attack for 2250).
/// Warwolf is no Machine, so it cannot be equipped.
#[test]
fn power_load_gives_400_atk_for_each_other_machine_and_stops_the_others_attacking() {
    let [load, chaser, overdrive] = ["Power Load", "Mechanicalchaser", "Overdrive"].map(card);
    let spawns = [hand(0, load), mon(0, chaser), mon(0, overdrive), mon(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, chaser), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(load)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", load), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, chaser), Some(1850 + 400), "{}", s.context());
    let err = s.run(&[Attack(overdrive)]).expect_err("only the equipped monster attacks");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{chaser}")) && !err.contains(&format!("\"cid\":{overdrive}")), "{err}");
    s.run(&[Attack(chaser)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 2250, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, load), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(load)]).expect_err("Warwolf is no Machine");
    assert!(err.contains("never became possible"), "{err}");
}
