//! Anime-only cards, group 24 (rows 5993-6023 of engine/added/cards.tsv): Lawton's, Crow Hogan's, Bolton's and Jakob's cards
//! (the Ogres, Black Wing, the White Warriors, Chaos Life, Life Exchange, Granel Top 5, Wisel Attack 5...). These cards exist
//! only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the real
//! core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones.
//! Every Trap and every Quick-Play Spell answering the opponent is Set first (most of this batch's cards are Traps), and
//! neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3 (team 1 attacks in turn 2).
//! Covered: the Token (Ambush Token, made by Back Attack Ambush), the cards the per-card fuzz flagged (Big Cannon Ogre, Full
//! Armor Ogre, Pinpoint Shot, Black Wing, Chaos Life, Synchro Rivalry, White Warrior - Sleet the Morning Star, Attack Cannon,
//! Banishing Climb, Granel Top 5, Wisel Attack 5: each needs a board the self-play never builds (a card named in a
//! declaration, an opponent's Synchro Summon, a Meklord Emperor attacking, Gatling Ogre in play...), so each has its board
//! here), the cards `--find` noted (Black Wing's cost, Chaos Life's and Life Exchange's cases at ∞ LP, Down Burst), the
//! signature cards (Back Attack Ambush, Blaster Ogre, Big Cannon Ogre, Black Wing, Wisel Attack 5, Granel Top 5, Explosive
//! Wall, Life Exchange) and a seeded sample of the rest (Card of Burial Magic, Double Arms, Metal Coat, White Warrior - Fog the
//! Treasure Shield, White Warrior - Snow the Rope Dart), with Down Force, Level Wing, Rendering Tuning, Sacred Arrow, Hail the
//! Iron Hammer, Infinity Wall, Narrow Tunnel and Whiteout on top. Parasite Mind (it copies a Continuous Spell/Trap's effect
//! onto itself) has no scenario.
//! Overrides this batch needed: `engine/overrides/c810000039.lua` and `c810000038.lua` (Big Cannon Ogre and Full Armor Ogre
//! name Gatling Ogre by its anime code, and the game's Gatling Ogre has no anime version, so neither could ever work),
//! `c511001488.lua` (Chaos Life remembered the damage, not the player, so it reversed nothing for its controller),
//! `c511001979.lua` (Sleet's `c:getAttack()`, a Lua error every time), `c511008026.lua` (Granel Top 5's two misspelt
//! variables, which made its equip effect return at once) and `c100000444.lua` (Attack Cannon waited for EVENT_BATTLE_END,
//! which the core no longer raises).
//! Not coverable: Black Wing's "Blackwing monster from the field" (the script's cost lists the Monster Zone but its filter
//! keeps only the Graveyard, as the text says: tested as a card that cannot be paid), Banishing Climb's negation of the
//! returned monster (the client's view carries no "negated" mark to read), and Chaos Life against an ∞ amount of damage (no
//! card of the game announces damage of 9,999,999 or more in its operation info: only the finite case at ∞ LP is tested).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const OOKAZI: u16 = 4352; // 800 damage
const CANNON_SOLDIER: u16 = 4518; // Tribute a monster: 500 damage
const GALE: u16 = 8011; // Blackwing - Gale the Whirlwind
const ARMOR_MASTER: u16 = 8036; // Blackwing Armor Master, a Synchro Monster
const TURBO_SYNCHRON: u16 = 7870; // a Level 1 Tuner
const SUMMONED_SKULL: u16 = 4028; // Level 6, 2500/1200
const POWER_TOOL: u16 = 8137; // Power Tool Dragon, a Level 7 Synchro Monster, 2300/2500
const GATLING_OGRE: u16 = 5156; // an added card, with no anime version

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card.
fn duel(seed: u64, lp: u32, spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, spawns) }

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// A duel with the Extra Decks given: `extra[team]`.
fn field_extra(extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], extra[0]), (&[FILLER; 40][..], extra[1])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

// ---------------------------------------------------------------------------------------------------------------
// Back Attack Ambush and the Ambush Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Back Attack Ambush: "When an opponent's monster declares an attack: End the Battle Phase, and Special Summon 1 "Ambush
/// Token" (Warrior-Type/EARTH/Level 1/ATK 100/DEF 100) for each face-up Attack Position monster your opponent controls.
/// Then, you can Tribute any number of "Ambush Tokens" and inflict 500 damage to your opponent for each." Team 1 has two
/// face-up Attack Position monsters and attacks with one: two Tokens come in face-up Attack Position, with the stats above,
/// the attack does no damage and the Battle Phase is over (the other monster cannot attack). The default answers Tribute
/// the fewest Tokens: one is Tributed (500 damage, team 1 at 7500) and one stays.
#[test]
fn back_attack_ambush_makes_a_token_for_each_attacking_side_monster() {
    let [ambush, token] = ["Back Attack Ambush", "Ambush Token"].map(card);
    let Some(mut s) = field(&[set(0, ambush), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.prefer(&[(1, FILLER)]);
    s.prefer_most(false);
    s.run_both([&[ChainOnAttack(ambush)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ambush), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack did no damage: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 1, "two Tokens came, one was Tributed: {}", s.context());
    assert_eq!(s.lp(1), 7500, "500 damage for the Tributed Token: {}", s.context());
    let t = stats(&s, 0, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(100), Some(100), Some(1), 0x1, 1), "Warrior, EARTH, Level 1, 100/100: {}", s.context());
    assert_eq!(s.position_of(0, token), Some(1), "face-up Attack: {}", s.context());
    // The Battle Phase ended: team 1's other monster cannot attack.
    assert!(s.prompt()["type"] == msg::SELECT_IDLECMD && s.prompt()["to_bp"] == false, "no Battle Phase left: {}\n{}", s.prompt(), s.context());
}

/// Back Attack Ambush: the Tokens may be Tributed for 500 damage each; Tributed Tokens are gone (a Token is not sent to the
/// Graveyard). Both Tokens Tributed: 1000 damage to team 1 (7000), no Token left.
#[test]
fn back_attack_ambush_tributes_its_tokens_for_damage() {
    let [ambush, token] = ["Back Attack Ambush", "Ambush Token"].map(card);
    let Some(mut s) = field(&[set(0, ambush), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.prefer(&[(0, token)]);
    s.prefer_most(true);
    s.run_both([&[ChainOnAttack(ambush)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 0, "both Tokens were Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", token), 0, "{}", s.context());
    assert_eq!(s.lp(1), 7000, "500 damage for each Tributed Token: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Ogres: Big Cannon Ogre, Full Armor Ogre, Blaster Ogre (engine/overrides/c810000039.lua, c810000038.lua)
// ---------------------------------------------------------------------------------------------------------------

/// Big Cannon Ogre: "Must be Special Summoned (from your hand) by Tributing "Gatling Ogre" while there is a "Full Armor
/// Ogre" in your Graveyard." With both in place (Gatling Ogre on the field, Full Armor Ogre in the Graveyard) it is
/// Special Summoned from the hand, Gatling Ogre is Tributed to the Graveyard; without a Full Armor Ogre in the Graveyard
/// it is not offered (engine/overrides/c810000039.lua: the script wanted the anime code of Gatling Ogre, which the game's
/// own card does not have, so the procedure was never possible; batch 24's fuzz).
#[test]
fn big_cannon_ogre_is_summoned_by_tributing_gatling_ogre_with_full_armor_ogre_in_the_graveyard() {
    let [cannon, armor] = ["Big Cannon Ogre", "Full Armor Ogre"].map(card);
    let Some(mut s) = field(&[hand(0, cannon), mon(0, GATLING_OGRE), grave(0, armor)]) else { return };
    s.run(&[SpSummon(cannon)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", cannon), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", GATLING_OGRE), 1, "Gatling Ogre was Tributed: {}", s.context());
    let c = stats(&s, 0, cannon);
    assert_eq!((c.0, c.1, c.2), (Some(2400), Some(2400), Some(7)), "{}", s.context());
    // Without Full Armor Ogre in the Graveyard: not offered.
    let Some(mut s) = field(&[hand(0, cannon), mon(0, GATLING_OGRE)]) else { return };
    s.run(&[]).unwrap();
    let offered: Vec<_> = s.prompt()["special_summons"].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect());
    assert!(!offered.contains(&cannon.into()), "{offered:?}");
}

/// Big Cannon Ogre: "Double all battle damage this card inflicts to your opponent." A direct attack of its 2400 ATK in turn
/// 3 takes 4800 (3200 left).
#[test]
fn big_cannon_ogre_doubles_its_battle_damage() {
    let [cannon, armor] = ["Big Cannon Ogre", "Full Armor Ogre"].map(card);
    let Some(mut s) = field(&[hand(0, cannon), mon(0, GATLING_OGRE), grave(0, armor)]) else { return };
    s.run_both([&[SpSummon(cannon), EndTurn, Attack(cannon)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2 * 2400, "{}", s.context());
}

/// Big Cannon Ogre: "When this card is destroyed: Special Summon 1 "Full Armor Ogre" from your Graveyard." Dark Hole destroys
/// it and Full Armor Ogre comes from the Graveyard.
#[test]
fn big_cannon_ogre_revives_full_armor_ogre_when_destroyed() {
    let [cannon, armor] = ["Big Cannon Ogre", "Full Armor Ogre"].map(card);
    let Some(mut s) = field(&[mon(0, cannon), grave(0, armor), hand(0, DARK_HOLE)]) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cannon), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", armor), 1, "Full Armor Ogre came back: {}", s.context());
    assert_eq!(s.count(0, "grave", armor), 0, "{}", s.context());
}

/// Full Armor Ogre: "When this card destroys an opponent's monster by battle: inflict 1000 damage to your opponent." In
/// turn 3 its 1600 ATK destroys a Thunder (700): 900 battle damage and 1000 more, 8000 - 1900.
#[test]
fn full_armor_ogre_inflicts_1000_when_it_wins_a_battle() {
    let armor = card("Full Armor Ogre");
    let Some(mut s) = field(&[mon(0, armor), mon(1, THUNDER_KID)]) else { return };
    s.run_both([&[EndTurn, Attack(armor)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 900 - 1000, "{}", s.context());
}

/// Full Armor Ogre: "When this card is sent from the field to the Graveyard: you can Special Summon 1 "Gatling Ogre" from
/// your Graveyard." Dark Hole destroys it and, answering yes, Gatling Ogre comes from the Graveyard (engine/overrides/
/// c810000038.lua; batch 24's fuzz never saw it).
#[test]
fn full_armor_ogre_revives_gatling_ogre_when_it_leaves_the_field() {
    let armor = card("Full Armor Ogre");
    let Some(mut s) = field(&[mon(0, armor), grave(0, GATLING_OGRE), hand(0, DARK_HOLE)]) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(armor)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", armor), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", GATLING_OGRE), 1, "Gatling Ogre came back: {}", s.context());
}

/// Blaster Ogre: "Twice per turn: you can destroy 1 face-up Attack Position monster your opponent controls with the highest
/// ATK and inflict damage to your opponent equal to half of the destroyed monster's ATK. (If it's a tie, you get to choose.)"
/// With Warwolf (2000) and Dark Blade (1800) it destroys Warwolf for 1000, then, the second time, Dark Blade for 900, and
/// a third try is not offered.
#[test]
fn blaster_ogre_destroys_the_strongest_attack_position_monster_twice_a_turn() {
    let blaster = card("Blaster Ogre");
    let Some(mut s) = field(&[mon(0, blaster), mon(1, DARK_BLADE), mon(1, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(blaster)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the highest ATK first: {}", s.context());
    assert_eq!(s.lp(1), 7000, "half of 2000: {}", s.context());
    s.run(&[Activate(blaster)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 6100, "half of 1800: {}", s.context());
    // Twice per turn only: the Thunder is safe until next turn.
    let offered: Vec<_> = s.prompt()["activates"].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect());
    assert!(!offered.contains(&blaster.into()), "{offered:?}");
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
}

/// Blaster Ogre: "(If it's a tie, you get to choose.)" Two Warwolves tie: the one picked is destroyed, the other stays.
/// A Defense Position monster is no target at all.
#[test]
fn blaster_ogre_lets_its_controller_choose_between_tied_monsters() {
    let blaster = card("Blaster Ogre");
    let Some(mut s) = field(&[mon(0, blaster), mon(1, WARWOLF), mon(1, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    // Dark Magician is turned to Defense: it is not a face-up Attack Position monster.
    s.run_both([&[EndTurn], &[Reposition(DARK_MAGICIAN)]]).unwrap();
    s.run_both([&[Activate(blaster)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "one of the tied Warwolves: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "a Defense Position monster is untouched: {}", s.context());
    assert_eq!(s.lp(1), 7000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Black Wing
// ---------------------------------------------------------------------------------------------------------------

/// Black Wing: "When an opponent's monster declares an attack: Banish 1 "Blackwing" monster from your Graveyard; negate the
/// attacks of opponent's monsters with 2000 or more ATK this turn." Team 1 attacks first with Dark Blade (1800: not negated,
/// 1800 damage), Black Wing is chained, and then Warwolf (2000) is negated: team 0 loses 1800 only, Gale is banished and
/// Black Wing is in the Graveyard.
#[test]
fn black_wing_negates_attacks_of_2000_or_more_atk_for_the_turn() {
    let wing = card("Black Wing");
    let Some(mut s) = field(&[set(0, wing), grave(0, GALE), mon(1, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.run_both([&[ChainOnAttack(wing)], &[Attack(DARK_BLADE), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", GALE), 1, "the cost: {}", s.context());
    assert_eq!(s.count(0, "grave", wing), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1800, "Dark Blade's attack was not negated, Warwolf's was: {}", s.context());
}

/// Black Wing: the attack that starts it is negated when the attacker has 2000 or more ATK: Warwolf's attack does nothing
/// (team 0 keeps its 8000 LP, Gale is banished). The cost is a "Blackwing" monster in the Graveyard, as the card says: with
/// Gale only on the field (or none at all) it cannot be paid, Black Wing is not offered and the attack goes through.
#[test]
fn black_wing_negates_the_attack_that_started_it_and_needs_a_blackwing_monster_in_the_graveyard() {
    let wing = card("Black Wing");
    let Some(mut s) = field(&[set(0, wing), grave(0, GALE), mon(1, WARWOLF)]) else { return };
    s.run_both([&[ChainOnAttack(wing)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", GALE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", wing), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "Warwolf's attack was negated: {}", s.context());
    // Gale on the field is not a cost; nor is an empty Graveyard.
    for spawns in [[set(0, wing), mon(0, GALE), mon(1, WARWOLF)], [set(0, wing), mon(0, THUNDER_KID), mon(1, WARWOLF)]] {
        let Some(mut s) = field(&spawns) else { return };
        s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
        assert_eq!(s.count(0, "szone", wing), 1, "Black Wing is still Set: {}", s.context());
        assert_eq!(s.count(0, "removed", GALE), 0, "{}", s.context());
        assert!(s.lp(0) < 8000 || s.count(0, "mzone", GALE) + s.count(0, "mzone", THUNDER_KID) > 0, "{}", s.context());
    }
}

/// Black Wing: "If you have exactly 2 "Black Wing" in your Graveyard including this card: You can banish this card and 1 other
/// "Black Wing" from your Graveyard; destroy 1 Synchro Monster you control and 1 monster your opponent controls, and if you
/// do, inflict damage to your opponent equal to the ATK of the opponent's destroyed monster had on the field." Armor Master
/// (Synchro) and Warwolf (2000) are destroyed, both Black Wings are banished and team 1 takes 2000.
#[test]
fn black_wing_in_the_graveyard_destroys_a_synchro_monster_and_a_monster_for_damage() {
    let wing = card("Black Wing");
    let Some(mut s) = field(&[grave(0, wing), grave(0, wing), mon(0, ARMOR_MASTER), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.prefer(&[(0, ARMOR_MASTER), (1, WARWOLF)]);
    s.run(&[Activate(wing)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", wing), 2, "both Black Wings were banished: {}", s.context());
    assert_eq!(s.count(0, "grave", ARMOR_MASTER), 1, "the Synchro Monster: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 6000, "the ATK Warwolf had: {}", s.context());
}

/// Black Wing: "exactly 2": with three in the Graveyard the effect is not offered; with a Synchro Monster missing neither.
#[test]
fn black_wing_in_the_graveyard_needs_exactly_two_and_a_synchro_monster() {
    let wing = card("Black Wing");
    let offers = |s: &Scenario| s.prompt()["activates"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == wing));
    let Some(mut s) = field(&[grave(0, wing), grave(0, wing), grave(0, wing), mon(0, ARMOR_MASTER), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offers(&s), "three Black Wings: {}", s.prompt());
    let Some(mut s) = field(&[grave(0, wing), grave(0, wing), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offers(&s), "no Synchro Monster: {}", s.prompt());
    let Some(mut s) = field(&[grave(0, wing), grave(0, wing), mon(0, ARMOR_MASTER), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(offers(&s), "two Black Wings, a Synchro Monster and a target: {}", s.prompt());
}

// ---------------------------------------------------------------------------------------------------------------
// Pinpoint Shot
// ---------------------------------------------------------------------------------------------------------------

/// Pinpoint Shot: "Declare 1 card name; If that card is in your opponent's hand, send 1 of those card(s) to the Graveyard."
/// Naming Dark Magician with one in team 1's hand sends it to the Graveyard (the fuzz can never name a card); naming a
/// card that is not there does nothing.
#[test]
fn pinpoint_shot_sends_the_named_card_from_the_opponents_hand() {
    let shot = card("Pinpoint Shot");
    let Some(mut s) = field(&[set(0, shot), hand(1, DARK_MAGICIAN)]) else { return };
    let before = hand_size(&s, 1);
    s.declare_card(Some(DARK_MAGICIAN));
    s.run(&[Activate(shot)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 1), before - 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", shot), 1, "{}", s.context());
    // A name that is not in the hand.
    let Some(mut s) = field(&[set(0, shot), hand(1, DARK_MAGICIAN)]) else { return };
    s.declare_card(Some(WARWOLF));
    s.run(&[Activate(shot)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "hand", DARK_MAGICIAN), s.count(1, "grave", WARWOLF)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", shot), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Chaos Life
// ---------------------------------------------------------------------------------------------------------------

/// Chaos Life: "When a player's LP would become 0 by a card effect that would inflict damage: That player gains LP equal to
/// the amount of damage that they would have taken, instead." At 800 LP, team 1's Ookazi (800) would end the duel; Chaos Life
/// turns it into a gain: 1600 LP. At 1000 LP the same Ookazi would not take it to 0, and Chaos Life is not even offered:
/// 200 LP and it is still Set.
#[test]
fn chaos_life_turns_lethal_effect_damage_into_a_gain() {
    let chaos = card("Chaos Life");
    let Some(mut s) = duel(7, 800, &[set(0, chaos), hand(1, OOKAZI)]) else { return };
    s.run_both([&[Chain(chaos)], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 1600, "800 gained instead of 800 lost: {}", s.context());
    assert_eq!(s.count(0, "grave", chaos), 1, "{}", s.context());
    let Some(mut s) = duel(7, 1000, &[set(0, chaos), hand(1, OOKAZI)]) else { return };
    s.run_both([&[], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 200, "{}", s.context());
    assert_eq!(s.count(0, "szone", chaos), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The sample: Card of Burial Magic, Double Arms, Metal Coat
// ---------------------------------------------------------------------------------------------------------------

/// Card of Burial Magic: "Banish 3 Spells in your Graveyard; draw 2 cards." With four Spells in the Graveyard three are
/// banished and one stays (with Burial Magic itself, two Spells there), and two cards are drawn: the hand grows by one
/// (the card played, two drawn). With two Spells in the Graveyard it is not offered.
#[test]
fn card_of_burial_magic_banishes_three_spells_and_draws_two() {
    let burial = card("Card of Burial Magic");
    let Some(mut s) = field(&[hand(0, burial), grave(0, DIAN_KETO), grave(0, DIAN_KETO), grave(0, DARK_HOLE), grave(0, OOKAZI)]) else { return };
    let before = hand_size(&s, 0);
    s.run(&[Activate(burial)]).unwrap();
    clean(&s);
    let spells = |s: &Scenario, place: &str| s.count(0, place, DIAN_KETO) + s.count(0, place, DARK_HOLE) + s.count(0, place, OOKAZI);
    assert_eq!(spells(&s, "removed"), 3, "{}", s.context());
    assert_eq!(spells(&s, "grave"), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", burial), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "{}", s.context());
    let Some(mut s) = field(&[hand(0, burial), grave(0, DIAN_KETO), grave(0, DARK_HOLE)]) else { return };
    s.run(&[]).unwrap();
    assert!(!s.prompt()["activates"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == burial)), "{}", s.prompt());
}

/// Double Arms: "Double any effect damage inflicted by the equipped monster's effect." Cannon Soldier (Tribute a monster:
/// 500 damage) equipped with it inflicts 1000; the same effect without it, 500.
#[test]
fn double_arms_doubles_the_equipped_monsters_effect_damage() {
    let arms = card("Double Arms");
    let Some(mut s) = field(&[hand(0, arms), mon(0, CANNON_SOLDIER), mon(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, CANNON_SOLDIER)]);
    s.run(&[Activate(arms)]).unwrap();
    s.prefer(&[(0, THUNDER_KID)]);
    s.prefer_most(true); // the Tribute is chosen one card at a time (SELECT_UNSELECT_CARD)
    s.run(&[Activate(CANNON_SOLDIER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the Tribute: {}", s.context());
    assert_eq!(s.lp(1), 7000, "twice 500: {}", s.context());
    let Some(mut s) = field(&[mon(0, CANNON_SOLDIER), mon(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.prefer_most(true); // the Tribute is chosen one card at a time (SELECT_UNSELECT_CARD)
    s.run(&[Activate(CANNON_SOLDIER)]).unwrap();
    assert_eq!(s.lp(1), 7500, "{}", s.context());
}

/// Double Arms on another monster does not double a Cannon Soldier's damage: Thunder carries it, Cannon Soldier 500.
#[test]
fn double_arms_only_doubles_the_equipped_monsters_damage() {
    let arms = card("Double Arms");
    let Some(mut s) = field(&[hand(0, arms), mon(0, CANNON_SOLDIER), mon(0, THUNDER_KID), mon(0, DARK_BLADE)]) else { return };
    s.prefer(&[(0, DARK_BLADE)]);
    s.run(&[Activate(arms)]).unwrap();
    s.prefer(&[(0, THUNDER_KID)]);
    s.prefer_most(true); // the Tribute is chosen one card at a time (SELECT_UNSELECT_CARD)
    s.run(&[Activate(CANNON_SOLDIER)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7500, "{}", s.context());
}

/// Metal Coat (a Trap Card, Set first): "Equip this card to a monster you control. The equipped monster cannot be destroyed by card effects. When the
/// equipped monster would be destroyed by battle, you can send this face-up card to the Graveyard instead." Thunder with
/// the Coat survives Team 1's Dark Hole (Dark Blade, Warwolf and the Coat's monster: Thunder stays), and then, in turn 2,
/// Warwolf's attack: Thunder is not destroyed and the Coat goes to the Graveyard.
#[test]
fn metal_coat_protects_its_monster_from_effects_and_once_from_battle() {
    let coat = card("Metal Coat");
    let Some(mut s) = field(&[set(0, coat), mon(0, THUNDER_KID), mon(1, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[Activate(coat), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "not destroyed by Dark Hole: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", coat), 1, "{}", s.context());
    // Battle: a fresh duel, Thunder with the Coat against Warwolf's attack.
    let Some(mut s) = field(&[set(0, coat), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[Activate(coat)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "saved by the Coat: {}", s.context());
    assert_eq!(s.count(0, "grave", coat), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", coat), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Explosive Wall, Down Burst, Down Force, Level Wing, Rendering Tuning, Sacred Arrow
// ---------------------------------------------------------------------------------------------------------------

/// Explosive Wall: "When an opponent's monster declares an attack: Destroy the attacking monster and 1 monster you control."
/// Warwolf attacks; the Wall destroys it and the Thunder picked (Dark Blade stays); the Wall goes to the Graveyard.
#[test]
fn explosive_wall_destroys_the_attacker_and_a_monster_of_its_controller() {
    let wall = card("Explosive Wall");
    let Some(mut s) = field(&[set(0, wall), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[ChainOnAttack(wall)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the attacker: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the monster picked: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", wall), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Explosive Wall: "When an opponent's monster declares an attack, while this card is in your Graveyard: you can banish this
/// card; banish the attacking monster and all cards you control." With the Wall in the Graveyard, a Dark Blade and a Set
/// Dark Hole of team 0 and an attack by Warwolf: the Wall, Warwolf, Dark Blade and the Set card are banished, and team 1's
/// Thunder stays.
#[test]
fn explosive_wall_banishes_itself_the_attacker_and_all_of_its_controllers_cards_from_the_graveyard() {
    let wall = card("Explosive Wall");
    let Some(mut s) = field(&[grave(0, wall), mon(0, DARK_BLADE), set(0, DARK_HOLE), mon(1, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    s.run_both([&[Chain(wall)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", wall), 1, "{}", s.context());
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "the attacker: {}", s.context());
    assert_eq!(s.count(0, "removed", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Down Burst: "Set all face-up Spells/Traps your opponent controls." Team 1's face-up Gravity Bind and Messenger of Peace
/// are turned face-down (and neither is offered to team 1 as a Set card to activate the same turn: the card says Set, not
/// "treated as Set this turn": `STATUS_SET_TURN` is cleared). Team 0's own Spell stays face-up.
#[test]
fn down_burst_sets_every_face_up_spell_and_trap_of_the_opponent() {
    let burst = card("Down Burst");
    let [bind, peace] = ["Gravity Bind", "Messenger of Peace"].map(card);
    let Some(mut s) = field(&[set(0, burst), Spawn { team: 1, place: Spell, cid: bind }, Spawn { team: 1, place: Spell, cid: peace }, Spawn { team: 0, place: Spell, cid: peace }]) else { return };
    let face_up = |s: &Scenario, team: usize| -> usize {
        s.state()[team]["szone"].as_array().map_or(0, |v| v.iter().filter(|c| c["position"].as_u64().is_some_and(|p| p & 1 != 0)).count())
    };
    assert_eq!((face_up(&s, 0), face_up(&s, 1)), (1, 2), "{}", s.state()[1]["szone"]);
    s.run(&[Activate(burst)]).unwrap();
    clean(&s);
    assert_eq!(face_up(&s, 1), 0, "both were Set: {}", s.state()[1]["szone"]);
    assert_eq!(s.state()[1]["szone"].as_array().map_or(0, |v| v.iter().filter(|c| !c.is_null()).count()), 2, "{}", s.context());
    assert_eq!(face_up(&s, 0), 1, "team 0's own Messenger of Peace stays face-up: {}", s.state()[0]["szone"]);
}

/// Down Force: "When your opponent activates a card effect that would make a monster you control leave the field: Negate the
/// effect(s) of that card, and if you do, destroy it." Team 1's Raigeki would destroy team 0's Warwolf: Down Force negates
/// it and destroys it (Raigeki is in the Graveyard) and both sides keep their monsters.
#[test]
fn down_force_negates_and_destroys_a_card_that_would_remove_its_controllers_monster() {
    let force = card("Down Force");
    let raigeki = 4343;
    let Some(mut s) = field(&[set(0, force), mon(0, WARWOLF), mon(1, DARK_BLADE), hand(1, raigeki)]) else { return };
    s.run_both([&[Chain(force)], &[Activate(raigeki)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", raigeki), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "the effect was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", force), 1, "{}", s.context());
}

/// Level Wing: "Target 1 face-up monster you control; Increase its Level by 3." Warwolf is Level 4, then Level 7.
#[test]
fn level_wing_raises_a_level_by_three() {
    let wing = card("Level Wing");
    let Some(mut s) = field(&[hand(0, wing), mon(0, WARWOLF)]) else { return };
    assert_eq!(s.level_of(0, WARWOLF), Some(4));
    s.run(&[Activate(wing)]).unwrap();
    clean(&s);
    assert_eq!(s.level_of(0, WARWOLF), Some(7), "{}", s.context());
    assert_eq!(s.count(0, "grave", wing), 1, "{}", s.context());
}

/// Rendering Tuning: "Special Summon 1 Tuner monster from your hand." Turbo Synchron comes face-up from the hand; a hand
/// with no Tuner does not offer it.
#[test]
fn rendering_tuning_summons_a_tuner_from_the_hand() {
    let tuning = card("Rendering Tuning");
    let Some(mut s) = field(&[hand(0, tuning), hand(0, TURBO_SYNCHRON)]) else { return };
    s.run(&[Activate(tuning)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TURBO_SYNCHRON), 1, "{}", s.context());
    assert_eq!(s.position_of(0, TURBO_SYNCHRON), Some(1), "{}", s.context());
    let Some(mut s) = field(&[hand(0, tuning)]) else { return };
    s.run(&[]).unwrap();
    assert!(!s.prompt()["activates"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == tuning)), "{}", s.prompt());
}

/// Sacred Arrow: "During each player's End Phase: Each player takes 400 damage for each of their monsters that were destroyed
/// and sent to the Graveyard this turn." Dark Hole destroys two of team 0's monsters and one of team 1's, with the Arrow
/// face-up: at the End Phase team 0 takes 800 and team 1 400.
#[test]
fn sacred_arrow_burns_each_player_for_their_destroyed_monsters() {
    let arrow = card("Sacred Arrow");
    let Some(mut s) = field(&[Spawn { team: 0, place: Spell, cid: arrow }, hand(0, DARK_HOLE), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.run(&[Activate(DARK_HOLE), EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 800, 8000 - 400), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Synchro Rivalry and Banishing Climb: the Synchro Summons of the opponent
// ---------------------------------------------------------------------------------------------------------------

/// Team 1's board for a Synchro Summon of Power Tool Dragon (Level 7: Turbo Synchron, Level 1, and Summoned Skull, Level 6).
fn synchro_board(mine: &[Spawn]) -> Option<Scenario> {
    let mut spawns = vec![mon(1, TURBO_SYNCHRON), mon(1, SUMMONED_SKULL)];
    spawns.extend_from_slice(mine);
    field_extra([&[], &[POWER_TOOL]], &spawns)
}

/// Synchro Rivalry: "When your opponent Summons a Synchro monster: Monsters cannot be destroyed by battle this turn." Team 1
/// Synchro Summons Power Tool Dragon (2300) and attacks team 0's Thunder (700): with Rivalry chained to the summon Thunder
/// survives (battle damage is still dealt: 1600); without it Thunder is destroyed.
#[test]
fn synchro_rivalry_stops_battle_destruction_after_the_opponents_synchro_summon() {
    let rivalry = card("Synchro Rivalry");
    let Some(mut s) = synchro_board(&[set(0, rivalry), mon(0, THUNDER_KID)]) else { return };
    s.run_both([&[Chain(rivalry)], &[SpSummon(POWER_TOOL), Attack(POWER_TOOL)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", rivalry), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1600, "{}", s.context());
    // The control.
    let Some(mut s) = synchro_board(&[mon(0, THUNDER_KID)]) else { return };
    s.run_both([&[], &[SpSummon(POWER_TOOL), Attack(POWER_TOOL)]]).unwrap();
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
}

/// Banishing Climb: "Activate only during the Battle Phase. Banish all Synchro Monsters on the field that were Special Summoned
/// during this turn. Monsters banished by this effect are returned to the field at the end of the Battle Phase, and their
/// effects are negated." Team 1 Synchro Summons Power Tool Dragon and attacks: Banishing Climb, chained to the attack, banishes
/// it before it hits (team 0 keeps its 8000), and it comes back to team 1's field after the Battle Phase.
#[test]
fn banishing_climb_banishes_the_synchro_monster_special_summoned_this_turn_until_the_battle_phase_ends() {
    let climb = card("Banishing Climb");
    let Some(mut s) = synchro_board(&[set(0, climb)]) else { return };
    s.run_both([&[ChainOnAttack(climb)], &[SpSummon(POWER_TOOL), Attack(POWER_TOOL)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", climb), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack never landed: {}", s.context());
    assert_eq!(s.count(1, "removed", POWER_TOOL), 1, "banished while the Battle Phase goes on: {}", s.context());
    for _ in 0..8 {
        // team 1 leaves the Battle Phase: its Main Phase 2 is the next idle prompt
        if s.prompt()["type"] == msg::SELECT_IDLECMD {
            break;
        }
        s.pass().unwrap();
    }
    assert_eq!(s.count(1, "mzone", POWER_TOOL), 1, "back at the end of the Battle Phase: {}", s.context());
    assert_eq!(s.count(1, "removed", POWER_TOOL), 0, "{}", s.context());
    // Not offered outside the Battle Phase (it is a Trap Set in turn 1, and the Main Phase has no timing for it).
    let Some(mut s) = synchro_board(&[set(0, climb)]) else { return };
    s.run_both([&[], &[SpSummon(POWER_TOOL)]]).unwrap();
    assert_eq!(s.count(0, "szone", climb), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", POWER_TOOL), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The White Warriors (engine/overrides/c511001979.lua)
// ---------------------------------------------------------------------------------------------------------------

/// White Warrior - Sleet the Morning Star: "You can Tribute this card to target 1 face-up monster you control; it gains this
/// effect: ● If this card destroys an opponent's monster by battle: Inflict damage to your opponent equal to half of this
/// card's ATK." Sleet is Tributed for Warwolf (2000), which in turn 3 destroys a Thunder (700): battle damage 1300 and half of
/// 2000 more: 8000 - 2300 (engine/overrides/c511001979.lua: upstream called `c:getAttack()`, a Lua error every time, so
/// the damage never came: batch 19's fuzz, seeds 65294 and 65515).
#[test]
fn sleet_the_morning_star_gives_half_the_atk_as_damage_when_a_monster_wins_a_battle() {
    let sleet = card("White Warrior - Sleet the Morning Star");
    let Some(mut s) = field(&[mon(0, sleet), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(sleet)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", sleet), 1, "the Tribute: {}", s.context());
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1300 - 1000, "battle damage and half of Warwolf's ATK: {}", s.context());
}

/// White Warrior - Hail the Iron Hammer: "If this card destroys an opponent's monster by battle: Target 1 monster on the
/// field; destroy it." Hail (1800) destroys a Thunder in turn 3 and a Dark Blade picked goes too (it is team 0's own: any
/// monster on the field may be the target).
#[test]
fn hail_the_iron_hammer_destroys_another_monster_when_it_wins_a_battle() {
    let hail = card("White Warrior - Hail the Iron Hammer");
    let Some(mut s) = field(&[mon(0, hail), mon(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[EndTurn, Attack(hail)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "the battle: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the target (any monster on the field, its own side's too): {}", s.context());
    assert_eq!(s.count(0, "mzone", hail), 1, "{}", s.context());
}

/// White Warrior - Fog the Treasure Shield: "When a monster you control inflicts damage to your opponent by a direct attack:
/// You can target 1 Spell/Trap on the field; destroy it." Warwolf attacks directly in turn 3 (2000 damage) and, answering
/// yes, destroys team 1's Set Dark Hole; the Fog is on the field.
#[test]
fn fog_the_treasure_shield_destroys_a_spell_or_trap_after_a_direct_attack() {
    let fog = card("White Warrior - Fog the Treasure Shield");
    let Some(mut s) = field(&[mon(0, fog), mon(0, WARWOLF), set(1, DARK_HOLE)]) else { return };
    s.prefer(&[(1, DARK_HOLE)]);
    s.run_both([&[EndTurn, Attack(WARWOLF), Chain(fog)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6000, "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
}

/// White Warrior - Snow the Rope Dart: "This card can attack directly. This card gains 100 ATK for each card on the field.
/// When this card is targeted for an attack: Return it to the hand." With four cards on the field (Snow, two Team 1
/// monsters and a Set card) Snow attacks directly with 400 ATK in turn 3 though team 1 has monsters: 7600.
#[test]
fn snow_the_rope_dart_attacks_directly_with_100_atk_per_card_on_the_field() {
    let snow = card("White Warrior - Snow the Rope Dart");
    let Some(mut s) = field(&[mon(0, snow), mon(1, THUNDER_KID), mon(1, DARK_BLADE), set(1, DARK_HOLE)]) else { return };
    assert_eq!(s.attack_of(0, snow), Some(400), "{}", s.context());
    s.run_both([&[EndTurn, Attack(snow)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7600, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID) + s.count(1, "mzone", DARK_BLADE), 2, "{}", s.context());
}

/// White Warrior - Snow the Rope Dart: when team 1's Warwolf attacks it, it returns to team 0's hand, and its attack on it is
/// over (no battle with Snow).
#[test]
fn snow_the_rope_dart_returns_to_the_hand_when_targeted_for_an_attack() {
    let snow = card("White Warrior - Snow the Rope Dart");
    let Some(mut s) = field(&[mon(0, snow), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, snow)]);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", snow), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", snow), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Life Exchange and Chaos Life at ∞ LP (the duel built as infinite_lp.rs builds it)
// ---------------------------------------------------------------------------------------------------------------

const INFINITE: i64 = 19_999_998;
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK

/// Team 0 at ∞ LP (Mystik Wok on the anime Geh: 8000 and ∞ is ∞), with `spawns` in place; team 1 at 8000 (as in
/// `infinite_lp.rs`).
fn infinite(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(0, WOK), mon(0, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[FILLER; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &field).expect("the duel starts");
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[]);
    Some(s)
}

/// Life Exchange: "When your opponent's LP is changed by a card effect: That change is applied to your LP instead, then, apply
/// an appropriate effect. ● If you gained LP, inflict 1000 damage to your opponent." Team 1's Dian Keto (+1000) is applied to
/// team 0: 9000 for team 0, and team 1 stays at 8000 and takes the 1000: 7000.
#[test]
fn life_exchange_gives_the_opponents_gain_to_its_controller_and_burns_the_opponent() {
    let exchange = card("Life Exchange");
    let Some(mut s) = field(&[set(0, exchange), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(exchange)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", exchange), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (9000, 7000), "{}", s.context());
}

/// Life Exchange: "● If you lost LP, destroy 1 face-up monster your opponent controls, if any." Team 0's own Ookazi would take
/// 800 from team 1: the loss falls on team 0 instead (7200), team 1 stays at 8000 and its Thunder is destroyed.
#[test]
fn life_exchange_takes_the_opponents_loss_on_its_controller_and_destroys_a_monster() {
    let exchange = card("Life Exchange");
    let Some(mut s) = field(&[set(0, exchange), hand(0, OOKAZI), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(OOKAZI), Chain(exchange)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", exchange), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (7200, 8000), "{}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
}

/// Life Exchange at ∞ LP (c511005073.lua's `Duel.SetLP`, and the gain and damage worked out from LP): team 0 at ∞ LP takes
/// team 1's Dian Keto gain: a finite gain leaves ∞ as it is, team 1 is back at 8000 and burned for 1000.
#[test]
fn life_exchange_at_infinite_lp_leaves_infinity_and_still_burns_the_opponent() {
    let exchange = card("Life Exchange");
    let Some(mut s) = infinite(&[set(0, exchange), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(exchange)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", exchange), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, 7000), "{}", s.context());
}

/// Life Exchange against a team 1 at ∞ LP: team 1's Dian Keto changes nothing on ∞ LP and the exchange gives team 0 nothing
/// either: both stay where they were (team 0 8000, team 1 ∞), with no error from the script's `Duel.SetLP`.
#[test]
fn life_exchange_against_infinite_lp_changes_nothing() {
    let exchange = card("Life Exchange");
    let Some(mut s) = infinite(&[set(1, exchange), hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    // Team 0 (∞) gains with Dian Keto: team 1's Life Exchange moves a gain of team 0's LP (its opponent's) to team 1.
    s.run_both([&[Activate(DIAN_KETO)], &[Chain(exchange)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    assert_eq!(s.count(1, "grave", exchange) + s.count(1, "szone", exchange), 1, "{}", s.context());
}

/// Chaos Life at ∞ LP (c511001488.lua works the amount out from the LP): 800 damage is not what takes ∞ LP to 0, so the
/// Trap is not even offered and the LP stay ∞.
#[test]
fn chaos_life_is_not_offered_against_damage_that_cannot_take_infinite_lp() {
    let chaos = card("Chaos Life");
    let Some(mut s) = infinite(&[set(0, chaos), hand(1, OOKAZI)]) else { return };
    s.run_both([&[], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    assert_eq!(s.count(0, "szone", chaos), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", OOKAZI), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Meklord cards: Attack Cannon, Granel Top 5 (engine/overrides/c511008026.lua) and Wisel Attack 5
// ---------------------------------------------------------------------------------------------------------------

const WISEL_EMPEROR: u16 = 9264; // the anime Meklord Emperor Wisel ∞: 0 ATK of its own, the ATK of the "Attack" monsters beside it; the only monster of its side that may attack
const GRANEL_EMPEROR: u16 = 9265; // the anime Meklord Emperor Granel ∞
const GOLEM: u16 = 6315; // Ancient Gear Golem: 3000 DEF
const MAGIC_CYLINDER: u16 = 5124;

/// Attack Cannon: "At the end of the Damage Step, if a "Meklord Emperor" monster you control attacks your opponent's monster,
/// and that monster is still on the field: Send 1 face-up "Attack" monster you control to the Graveyard to target 1 monster
/// your opponent controls; destroy it." Wisel (1600 ATK: it takes the ATK of the "Attack" monsters beside it) attacks a Defense
/// Position Golem (3000 DEF), which survives, and the Cannon sends Wisel Attack 3 to the Graveyard and destroys the Golem
/// (engine/overrides/c100000444.lua: upstream waited for EVENT_BATTLE_END, a timing the core no longer raises, so the card
/// could never be activated).
#[test]
fn attack_cannon_destroys_the_monster_a_meklord_emperor_failed_to_destroy() {
    let [cannon, attack3] = ["Attack Cannon", "Wisel Attack 3 (TF6)"].map(card);
    let Some(mut s) = field(&[set(0, cannon), mon(0, WISEL_EMPEROR), mon(0, attack3), Spawn { team: 1, place: MonsterDefense, cid: GOLEM }]) else { return };
    s.run_both([&[EndTurn, Attack(WISEL_EMPEROR), Chain(cannon)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cannon), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", attack3), 1, "the cost: {}", s.context());
    assert_eq!(s.count(1, "grave", GOLEM), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WISEL_EMPEROR), 1, "{}", s.context());
}

/// Granel Top 5: "If you do not control a face-up "∞" monster, destroy this card." Alone on the field it is destroyed; beside
/// Granel ∞ it stays.
#[test]
fn granel_top_5_is_destroyed_without_an_infinity_monster() {
    let top = card("Granel Top 5");
    let Some(mut s) = field(&[mon(0, top)]) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", top), s.count(0, "grave", top)), (0, 1), "{}", s.context());
    let Some(mut s) = field(&[mon(0, GRANEL_EMPEROR), mon(0, top)]) else { return };
    s.run(&[]).unwrap();
    assert_eq!((s.count(0, "mzone", top), s.count(0, "mzone", GRANEL_EMPEROR)), (1, 1), "{}", s.context());
}

/// Granel Top 5: "Once per turn, you can equip 1 face-up Synchro Monster your opponent controls to a face-up "∞" monster you
/// control." Team 1's Power Tool Dragon is equipped to Wisel ∞ (it leaves team 1's field and is in team 0's Spell & Trap Zone)
/// (engine/overrides/c511008026.lua: upstream tested two misspelt variables and returned before equipping anything).
#[test]
fn granel_top_5_equips_an_opponents_synchro_monster_to_an_infinity_monster() {
    let top = card("Granel Top 5");
    let Some(mut s) = field_extra([&[], &[]], &[mon(0, WISEL_EMPEROR), mon(0, top), mon(1, POWER_TOOL), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(top)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", POWER_TOOL), 0, "{}", s.context());
    assert_eq!(s.count(0, "szone", POWER_TOOL), 1, "equipped to team 0's Wisel: {}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "only a Synchro Monster: {}", s.context());
    // Once per turn, and not offered without a Synchro Monster to take.
    assert!(!s.prompt()["activates"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == top)), "{}", s.prompt());
}

/// Wisel Attack 5: "Must first be Special Summoned by Tributing 1 "Wisel Attack 3"." With Wisel Attack 3 and an ∞ monster on
/// the field it is Special Summoned from the hand and Wisel Attack 3 is Tributed (2000/0, Level 5).
#[test]
fn wisel_attack_5_is_special_summoned_by_tributing_wisel_attack_3() {
    let [attack5, attack3] = ["Wisel Attack 5", "Wisel Attack 3 (TF6)"].map(card);
    let Some(mut s) = field(&[mon(0, WISEL_EMPEROR), hand(0, attack5), mon(0, attack3)]) else { return };
    s.run(&[SpSummon(attack5)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", attack5), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", attack3), 1, "the Tribute: {}", s.context());
    let a = stats(&s, 0, attack5);
    assert_eq!((a.0, a.1, a.2), (Some(2000), Some(0), Some(5)), "{}", s.context());
    // Without a Wisel Attack 3 it is not offered.
    let Some(mut s) = field(&[hand(0, attack5), mon(0, WISEL_EMPEROR)]) else { return };
    s.run(&[]).unwrap();
    assert!(!s.prompt()["special_summons"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == attack5)), "{}", s.prompt());
}

/// Wisel Attack 5: "If you do not control an "∞" monster, destroy this card."
#[test]
fn wisel_attack_5_is_destroyed_without_an_infinity_monster() {
    let attack5 = card("Wisel Attack 5");
    let Some(mut s) = field(&[mon(0, attack5)]) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", attack5), s.count(0, "grave", attack5)), (0, 1), "{}", s.context());
}

/// Wisel Attack 5: "If an "∞" monster you control attacks a Defense Position monster, inflict double piercing battle damage to
/// your opponent." Wisel ∞ (2000 ATK beside Wisel Attack 5) attacks a Defense Position Thunder (600 DEF) in turn 3: 1400
/// piercing damage, doubled: 2800.
#[test]
fn wisel_attack_5_doubles_the_piercing_damage_of_an_infinity_monster() {
    let attack5 = card("Wisel Attack 5");
    let Some(mut s) = field(&[mon(0, WISEL_EMPEROR), mon(0, attack5), Spawn { team: 1, place: MonsterDefense, cid: THUNDER_KID }]) else { return };
    // Wisel's ATK is the ATK of the "Attack" monsters beside it: Wisel Attack 5's 2000.
    let atk = s.attack_of(0, WISEL_EMPEROR).unwrap();
    assert_eq!(atk, 2000, "{}", s.context());
    s.run_both([&[EndTurn, Attack(WISEL_EMPEROR)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2 * (atk - 600), "double piercing damage: {}", s.context());
}

/// Wisel Attack 5: "Once per turn, during your turn, when an "∞" monster is attacking and your opponent activates a Trap Card
/// (Quick Effect): You can negate the activation, and if you do, destroy it." Wisel ∞ attacks, team 1's Set Magic Cylinder is
/// chained and negated and destroyed: team 0 takes no damage and team 1 takes the 2500.
#[test]
fn wisel_attack_5_negates_and_destroys_a_trap_activated_against_an_infinity_attack() {
    let attack5 = card("Wisel Attack 5");
    let Some(mut s) = field(&[mon(0, WISEL_EMPEROR), mon(0, attack5), set(1, MAGIC_CYLINDER)]) else { return };
    s.run_both([&[EndTurn, Attack(WISEL_EMPEROR), Chain(attack5)], &[ChainOnAttack(MAGIC_CYLINDER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MAGIC_CYLINDER), 1, "negated and destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "the attack went on with Wisel's 2000 ATK: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Infinity Wall, Narrow Tunnel, Whiteout
// ---------------------------------------------------------------------------------------------------------------

/// Infinity Wall: "Activate only if you control a face-up "Meklord Emperor" monster. Negate any card effects that would
/// destroy a card(s) you control during this turn." With Wisel ∞ on the field and the Wall activated, team 0's own Dark Hole
/// (which would destroy Wisel) is negated: every monster stays. Without a Meklord Emperor the Wall is not offered.
#[test]
fn infinity_wall_negates_effects_that_would_destroy_its_controllers_cards() {
    let wall = card("Infinity Wall");
    let Some(mut s) = field(&[mon(0, WISEL_EMPEROR), set(0, wall), hand(0, DARK_HOLE), mon(1, THUNDER_KID)]) else { return };
    s.run(&[Activate(wall), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WISEL_EMPEROR), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", THUNDER_KID), 1, "the whole effect was negated: {}", s.context());
    let Some(mut s) = field(&[mon(0, THUNDER_KID), set(0, wall)]) else { return };
    s.run(&[]).unwrap();
    assert!(!s.prompt()["activates"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == wall)), "{}", s.prompt());
}

/// Narrow Tunnel: "Each player can only Summon 1 monster to their side of the field per turn. If a player controls 2 or more
/// monsters, they must destroy monsters they control until they only control 1." Activated with two monsters on each side, it
/// destroys down to one monster each (the ones named stay).
#[test]
fn narrow_tunnel_destroys_each_player_down_to_one_monster() {
    let tunnel = card("Narrow Tunnel");
    let Some(mut s) = field(&[set(0, tunnel), mon(0, WARWOLF), mon(0, THUNDER_KID), mon(1, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    s.prefer(&[(0, THUNDER_KID), (1, DARK_BLADE)]);
    s.run(&[Activate(tunnel)]).unwrap();
    clean(&s);
    let monsters = |s: &Scenario, team: usize| s.state()[team]["mzone"].as_array().map_or(0, |v| v.iter().filter(|c| !c.is_null()).count());
    assert_eq!((monsters(&s, 0), monsters(&s, 1)), (1, 1), "{}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", DARK_MAGICIAN)), (1, 1), "the chosen monsters were destroyed: {}", s.context());
}

/// Whiteout: "Negate the effects of monsters your opponent controls that target a card(s) in their hand, their Graveyard, or
/// their side of the field." Team 1's Magician of Faith, flipped, would take a Spell from its Graveyard: with Whiteout face-up
/// the effect is negated and the Spell stays in the Graveyard; without it the Spell comes to the hand.
#[test]
fn whiteout_negates_a_monster_effect_that_targets_its_controllers_graveyard() {
    let whiteout = card("Whiteout");
    let faith = card("Magician of Faith");
    let flip = |spawns: &[Spawn]| -> Option<Scenario> {
        let mut s = field(spawns)?;
        s.prefer(&[(1, DIAN_KETO)]);
        s.run_both([&[], &[Reposition(faith)]]).unwrap();
        clean(&s);
        Some(s)
    };
    let Some(s) = flip(&[Spawn { team: 0, place: Spell, cid: whiteout }, Spawn { team: 1, place: MonsterSet, cid: faith }, grave(1, DIAN_KETO)]) else { return };
    assert_eq!((s.count(1, "grave", DIAN_KETO), s.count(1, "hand", DIAN_KETO)), (1, 0), "negated: {}", s.context());
    assert_eq!(s.count(1, "mzone", faith), 1, "{}", s.context());
    let Some(s) = flip(&[Spawn { team: 1, place: MonsterSet, cid: faith }, grave(1, DIAN_KETO)]) else { return };
    assert_eq!((s.count(1, "grave", DIAN_KETO), s.count(1, "hand", DIAN_KETO)), (0, 1), "the control: {}", s.context());
}
