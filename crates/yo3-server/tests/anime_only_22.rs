//! Anime-only cards, group 22 (rows 5928-5958 of engine/added/cards.tsv): Yusei Fudo's cards, the second half (Triangle
//! Token to Wild Heart). These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Nearly every card here is a Trap, Set first (a Trap
//! cannot be activated from the hand); neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! Covered: all 31 rows. The Triangle Token (made by Triangle Warrior: two on the opponent's field, shattered when it
//! leaves), the cards the per-card fuzz flagged (Space Jump, Star Excursion, Star Shift, Synchro Barrier Force, Synchro
//! Baton, Synchro Destructor, Synchro Out, Synchro Overlimit, Synchro Prominence, Synchro Ring, Synchro Striker Unit,
//! Synchron Believer, Tuner's Explosion, Wave Force, Wave Rebound, Wave Wall), Synchro Deception (whose fuzz duel failed on a
//! Lua error), Skill Twist (against Arcana Force XII - The Hangman (VG), the batch 9 fuzz's finding), the signature cards
//! (Synchro Barrier Force, Synchro Destructor, Synchron Keeper, Synchro Blast, Trust Chain, Wave Wall) and the seeded sample
//! (Shock Reborn, Sneak Giant, Stay Force and Talisman of Reversal), with the rest (Summon Tax, Superior Protector, Trick
//! Mirror and Wild Heart) beside them. The Synchro Monsters that Synchro Summon are Junk Warrior (Junk Synchron and a
//! Level 2 non-Tuner: the anime Junk Warrior wants that Tuner) and Armory Arm (Tune Warrior and Skull Servant).
//! Two overrides: `engine/overrides/c511000855.lua` (Synchro Deception: nothing is left to send when another Synchro
//! Deception chained to it took the only Synchro Monster, and the ATK target may be missing) and `c100000106.lua` (Hangman
//! (VG)'s target check named a player nobody had set, a Lua error for Skill Twist asking it about a new target). Every
//! "Synchro Monster" spawned on the field here was not Synchro Summoned: a card that needs the Synchro Summon (Synchro Out,
//! Star Excursion's return, Wave Rebound's Monster Reborn) is played with a real one.
//! Not coverable: nothing; Monster Reborn cannot revive a Junk Warrior of the anime (a Special Summon condition), so Wave
//! Rebound's test revives Armory Arm.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder Kid: a Level 2 Thunder, 700/600, no effect
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const BLUE_EYES: u16 = 4007; // Level 8 LIGHT Dragon, 3000/2500
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const OOKAZI: u16 = 4352; // 800 damage
const POT_OF_GREED: u16 = 4844; // draw 2
const JUST_DESSERTS: u16 = 4865; // Trap: 500 damage for each monster the opponent controls
const RAIGEKI_BREAK: u16 = 5544; // Trap: discard 1 card, target 1 card on the field; destroy it
const MAN_EATER_BUG: u16 = 4507; // FLIP: target 1 monster on the field; destroy it
const SKULL_SERVANT: u16 = 4030; // Level 1 Zombie, 300/200
const PETIT_DRAGON: u16 = 4206; // Level 2 Dragon, 600/700
const GOYO: u16 = 7736; // Goyo Guardian: a Level 6 Synchro Monster, 2800/2000
const JUNK_WARRIOR: u16 = 7696; // Level 5 Synchro Monster, 2300/1300
const GAIA_KNIGHT: u16 = 7697; // Gaia Knight, the Force of Earth: a Level 6 Synchro Monster without an effect, 2600/800
const STARDUST: u16 = 7734; // Stardust Dragon: a Level 8 Synchro Monster, 2500/2000
const ARMORY_ARM: u16 = 7987; // Level 4 Synchro Monster, 1800/1200
const TUNE_WARRIOR: u16 = 7684; // a Level 3 Tuner without an effect, 1600/200
const JUNK_SYNCHRON: u16 = 7687; // a Level 3 Tuner

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
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

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

/// A run that must stop because the action never became possible.
fn never(result: Result<(), String>) {
    let err = result.expect_err("the action should never become possible");
    assert!(err.contains("never became possible") || err.contains("can't attack"), "{err}");
}

/// From a battle prompt on to the Main Phase 2 of the same turn.
fn to_main_phase_2(s: &mut Scenario) {
    if s.prompt()["type"] == msg::SELECT_BATTLECMD {
        s.respond(serde_json::json!({"type": 1, "action": battle::TO_M2, "index": null})).unwrap();
    }
    for _ in 0..8 {
        if s.prompt()["type"] == msg::SELECT_IDLECMD {
            break;
        }
        s.pass().unwrap();
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Triangle Warrior and the Triangle Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Triangle Warrior: "You can Special Summon 2 "Triangle Tokens" (Warrior-Type/LIGHT/Level 2/ATK 1200/DEF 1200) to your
/// opponent's side of the field in Defense Position to Special Summon this card from your hand." Triangle Warrior (600/1200)
/// comes to my field and two Tokens of those stats to the opponent's, in face-up Defense Position.
#[test]
fn triangle_warrior_gives_the_opponent_two_triangle_tokens_in_defense_position() {
    let [warrior, token] = ["Triangle Warrior", "Triangle Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, warrior)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(warrior)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", warrior), 1, "{}", s.context());
    assert_eq!((s.count(1, "mzone", token), s.count(0, "mzone", token)), (2, 0), "the Tokens are the opponent's: {}", s.context());
    let t = stats(&s, 1, token);
    assert_eq!((t.0, t.1, t.2), (Some(1200), Some(1200), Some(2)), "1200/1200, Level 2: {}", s.context());
    assert_eq!(s.position_of(1, token), Some(4), "face-up Defense: {}", s.context());
    let w = stats(&s, 0, warrior);
    assert_eq!((w.0, w.1, w.2), (Some(600), Some(1200), Some(2)), "{}", s.context());
    // Warrior-Type (as Dark Blade) and LIGHT (as Blue-Eyes).
    let Some(s2) = field(&[], &[mon(0, DARK_BLADE), mon(0, BLUE_EYES)]) else { return };
    assert_eq!(t.3, stats(&s2, 0, DARK_BLADE).3, "Warrior-Type");
    assert_eq!(t.4, stats(&s2, 0, BLUE_EYES).4, "LIGHT");
}

/// Triangle Warrior needs two free Monster Zones on the opponent's side: with four monsters there it is not offered, with
/// three it is.
#[test]
fn triangle_warrior_needs_two_free_zones_on_the_opponents_side() {
    let warrior = card("Triangle Warrior");
    let four = [hand(0, warrior), mon(1, WARWOLF), mon(1, DARK_BLADE), mon(1, FILLER), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &four) else { return };
    keep_hands(&mut s);
    never(s.run(&[SpSummon(warrior)]));
    assert_eq!(s.count(0, "hand", warrior), 1, "{}", s.context());
    let three = [hand(0, warrior), mon(1, WARWOLF), mon(1, DARK_BLADE), mon(1, FILLER)];
    let Some(mut s) = field(&[], &three) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(warrior)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", warrior), 1, "{}", s.context());
}

/// A Triangle Token leaves the field like any Token: Dark Hole destroys Triangle Warrior and both Tokens, and the Tokens are
/// gone (not in the Graveyard) while the Warrior is in mine. A Token in Defense Position is also destroyed by an attack:
/// Warwolf (2000) beats a Token (1200 DEF) and no Token reaches the Graveyard.
#[test]
fn a_triangle_token_is_gone_when_it_leaves_the_field() {
    let [warrior, token] = ["Triangle Warrior", "Triangle Token"].map(card);
    let Some(mut s) = field(&[], &[hand(0, warrior), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(warrior), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", token), s.count(1, "grave", token), s.count(0, "grave", token)), (0, 0, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", warrior), 1, "{}", s.context());
    // Destroyed by battle.
    let Some(mut s) = field(&[], &[hand(0, warrior), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(warrior), EndTurn, Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", token), 1, "one Token was destroyed by Warwolf: {}", s.context());
    assert_eq!(s.count(1, "grave", token), 0, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "a Defense Position monster: no battle damage: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Shock Reborn, Skill Twist, Sneak Giant
// ---------------------------------------------------------------------------------------------------------------

/// Shock Reborn: "When you would take battle damage: Halve that battle damage, then Special Summon 1 monster from your
/// Graveyard whose ATK is equal to or less than the battle damage you took." Warwolf (2000) destroys my Thunder Kid (700):
/// 1300 battle damage, halved to 650, and Petit Dragon (600) comes back from my Graveyard; Dark Blade (1800) stays there.
#[test]
fn shock_reborn_halves_the_battle_damage_and_revives_a_monster_with_less_atk() {
    let shock = card("Shock Reborn");
    let spawns = [set(0, shock), mon(0, THUNDER_KID), grave(0, PETIT_DRAGON), grave(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(shock)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 650, "1300 halved: {}", s.context());
    assert_eq!(s.count(0, "mzone", PETIT_DRAGON), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(0, "grave", THUNDER_KID), s.count(0, "grave", shock)), (1, 1, 1), "{}", s.context());
    assert_eq!(s.position_of(0, PETIT_DRAGON), Some(1), "face-up Attack: {}", s.context());
    // The control: without it the 1300 is taken whole.
    let Some(mut s) = field(&[], &[mon(0, THUNDER_KID), grave(0, PETIT_DRAGON), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}

/// Shock Reborn is not offered when no monster in the Graveyard has an ATK of at most half the battle damage: only Dark
/// Blade (1800) is there, and 650 is not enough.
#[test]
fn shock_reborn_needs_a_monster_in_the_graveyard_with_atk_at_most_the_damage() {
    let shock = card("Shock Reborn");
    let spawns = [set(0, shock), mon(0, THUNDER_KID), grave(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(shock)], &[Attack(WARWOLF)]]));
    assert_eq!(s.count(0, "grave", shock), 0, "{}", s.context());
}

/// Skill Twist: "When your opponent activates an Effect Monster's effect that targets 1 monster on the field: Change the
/// target to another appropriate monster you control." Team 1's Man-Eater Bug is flipped (FLIP: target 1 monster, destroy
/// it) against my Dark Magician; Skill Twist changes the target to my Warwolf, which is destroyed in its place.
#[test]
fn skill_twist_changes_the_target_of_a_monster_effect_to_another_monster_i_control() {
    let twist = card("Skill Twist");
    let spawns = [set(0, twist), mon(0, DARK_MAGICIAN), mon(0, WARWOLF), Spawn { team: 1, place: MonsterSet, cid: MAN_EATER_BUG }];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_MAGICIAN), (0, WARWOLF)]);
    s.run_both([&[Chain(twist)], &[EndTurn, Reposition(MAN_EATER_BUG)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", twist), 1, "Skill Twist was activated: {}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_MAGICIAN), s.count(0, "grave", WARWOLF)), (1, 1), "Warwolf, not Dark Magician: {}", s.context());
    // The control: without Skill Twist the chosen target, Dark Magician, is destroyed.
    let spawns = [mon(0, DARK_MAGICIAN), mon(0, WARWOLF), Spawn { team: 1, place: MonsterSet, cid: MAN_EATER_BUG }];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_MAGICIAN), (0, WARWOLF)]);
    s.run_both([&[], &[Reposition(MAN_EATER_BUG)]]).unwrap();
    assert_eq!((s.count(0, "grave", DARK_MAGICIAN), s.count(0, "mzone", WARWOLF)), (1, 1), "{}", s.context());
}

/// Sneak Giant: "If only your opponent controls a monster, you can Normal Summon this card without Tributing, by Special
/// Summoning 1 Level 1 monster from your Graveyard." Skull Servant (Level 1) comes back from my Graveyard and the Level 5
/// Giant is summoned with no Tribute; with no monster on the opponent's side it is not offered.
#[test]
fn sneak_giant_is_summoned_without_tribute_by_reviving_a_level_1_monster() {
    let giant = card("Sneak Giant");
    let spawns = [hand(0, giant), grave(0, SKULL_SERVANT), grave(0, PETIT_DRAGON), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Summon(giant)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", giant), s.count(0, "mzone", SKULL_SERVANT)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", PETIT_DRAGON), 1, "only the Level 1 monster: {}", s.context());
    assert_eq!(s.level_of(0, giant), Some(5), "{}", s.context());
    // No monster on the opponent's side: a Level 5 monster needs a Tribute.
    let Some(mut s) = field(&[], &[hand(0, giant), grave(0, SKULL_SERVANT)]) else { return };
    keep_hands(&mut s);
    never(s.run(&[Summon(giant)]));
    assert_eq!(s.count(0, "hand", giant), 1, "{}", s.context());
    // A Graveyard without a Level 1 monster: not offered either.
    let Some(mut s) = field(&[], &[hand(0, giant), grave(0, PETIT_DRAGON), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run(&[Summon(giant)]));
}

// ---------------------------------------------------------------------------------------------------------------
// Space Jump, Star Excursion, Star Shift
// ---------------------------------------------------------------------------------------------------------------

/// Space Jump: "When your opponent activates a card effect that would destroy or banish exactly 1 monster you control during
/// your Battle Phase: Banish that monster and inflict damage to your opponent equal to its ATK." In my turn 3 my Thunder Kid
/// (700) attacks and the opponent's Raigeki Break (discard 1, target 1 card on the field: destroy it) targets Dark Blade
/// (1800); Space Jump banishes Dark Blade and the opponent takes 1800. Raigeki Break finds its target gone.
#[test]
fn space_jump_banishes_the_monster_an_effect_would_destroy_and_burns_its_atk() {
    let jump = card("Space Jump");
    let spawns = [set(0, jump), mon(0, THUNDER_KID), mon(0, DARK_BLADE), set(1, RAIGEKI_BREAK)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE), (1, FILLER)]);
    s.run_both([&[EndTurn, Attack(THUNDER_KID), Chain(jump)], &[EndTurn, ChainOnAttack(RAIGEKI_BREAK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", DARK_BLADE), 1, "banished, not destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 0, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1800 - 700, "Space Jump's 1800 and Thunder Kid's direct attack: {}", s.context());
    assert_eq!((s.count(0, "grave", jump), s.count(1, "grave", RAIGEKI_BREAK)), (1, 1), "{}", s.context());
}

/// Space Jump answers the Battle Phase only: Raigeki Break chained to my Normal Summon in the Main Phase is not answered.
#[test]
fn space_jump_waits_for_my_battle_phase() {
    let jump = card("Space Jump");
    let spawns = [set(0, jump), hand(0, DARK_BLADE), mon(0, THUNDER_KID), set(1, RAIGEKI_BREAK)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID), (1, FILLER)]);
    never(s.run_both([&[Summon(DARK_BLADE), Chain(jump)], &[Chain(RAIGEKI_BREAK)]]));
    assert_eq!((s.count(1, "grave", RAIGEKI_BREAK), s.count(0, "grave", THUNDER_KID)), (1, 1), "Raigeki Break destroyed it: {}", s.context());
    assert_eq!((s.count(0, "removed", THUNDER_KID), s.count(0, "szone", jump)), (0, 1), "Space Jump stays Set: {}", s.context());
}

/// Star Excursion: "During your opponent's Battle Phase, when a Synchro Monster you control is attacked by an opponent's
/// Synchro Monster: Banish both monsters, and if you do, at the end of your opponent's Battle Phase during their 3rd turn
/// after this effect is activated, each player Special Summons their monsters banished by this effect." My Junk Warrior (Synchro
/// Summoned from Junk Synchron and Thunder Kid) is attacked in turn 2 by Junk Warrior (Synchro Summoned from Junk Synchron and
/// Petit Dragon): both are banished, no damage. A Synchro Monster attacked by Warwolf (not a Synchro Monster) is not answered.
#[test]
fn star_excursion_banishes_both_synchro_monsters_of_a_battle() {
    let star = card("Star Excursion");
    let spawns = [set(0, star), mon(0, JUNK_SYNCHRON), mon(0, THUNDER_KID), mon(1, JUNK_SYNCHRON), mon(1, PETIT_DRAGON)];
    let Some(mut s) = field_extra([&[JUNK_WARRIOR], &[JUNK_WARRIOR]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(JUNK_WARRIOR), Chain(star)], &[SpSummon(JUNK_WARRIOR), Attack(JUNK_WARRIOR)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", JUNK_WARRIOR), s.count(1, "removed", JUNK_WARRIOR)), (1, 1), "both banished: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    assert_eq!(s.count(0, "grave", star), 1, "{}", s.context());
    // Not against a monster that is no Synchro Monster.
    let Some(mut s) = field(&[], &[set(0, star), mon(0, GOYO), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(star)], &[Attack(WARWOLF)]]));
    assert_eq!(s.count(1, "removed", WARWOLF), 0, "{}", s.context());
}

/// Star Excursion: the monsters come back at the end of the opponent's Battle Phase of their 3rd turn after the activation:
/// the opponent takes a Battle Phase in every turn here (Warwolf attacks directly: 2000 each), the activation's is the
/// first, and in the Battle Phase of turn 8 (the 4th attack) Junk Warrior returns to me and to them; not in turns 4 or 6.
#[test]
fn star_excursion_returns_the_monsters_in_the_third_turn_of_the_opponent() {
    let star = card("Star Excursion");
    let spawns = [set(0, star), mon(0, JUNK_SYNCHRON), mon(0, THUNDER_KID), mon(1, JUNK_SYNCHRON), mon(1, PETIT_DRAGON), mon(1, WARWOLF)];
    let Some(dir) = engine_dir() else { return };
    let decks = [(&[FILLER; 40][..], &[JUNK_WARRIOR][..]), (&[FILLER; 40][..], &[JUNK_WARRIOR][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 50_000, decks, &spawns).unwrap();
    keep_hands(&mut s);
    s.run_both([&[SpSummon(JUNK_WARRIOR), Chain(star)], &[SpSummon(JUNK_WARRIOR), Attack(JUNK_WARRIOR)]]).unwrap();
    assert_eq!((s.count(0, "removed", JUNK_WARRIOR), s.count(1, "removed", JUNK_WARRIOR)), (1, 1), "{}", s.context());
    // Turn 2's own Battle Phase went on with Warwolf's direct attack (the first of four); then turns 4 and 6 follow.
    for lp in [48_000, 46_000, 44_000] {
        s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
        to_main_phase_2(&mut s);
        clean(&s);
        assert_eq!(s.lp(0), lp, "{}", s.context());
        assert_eq!((s.count(0, "removed", JUNK_WARRIOR), s.count(1, "removed", JUNK_WARRIOR)), (1, 1), "still banished at {lp} LP: {}", s.context());
    }
    // The 4th Battle Phase after the activation (turn 8): both come back as it ends.
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    to_main_phase_2(&mut s);
    clean(&s);
    assert_eq!(s.lp(0), 42_000, "{}", s.context());
    assert_eq!((s.count(0, "mzone", JUNK_WARRIOR), s.count(1, "mzone", JUNK_WARRIOR)), (1, 1), "back in turn 8: {}", s.context());
    assert_eq!((s.count(0, "removed", JUNK_WARRIOR), s.count(1, "removed", JUNK_WARRIOR)), (0, 0), "{}", s.context());
}

/// Star Shift: "Target 1 Synchro Monster you control: Return it to the Extra Deck, then Special Summon 1 Synchro Monster with
/// the same Level, but with a different name, from your Extra Deck, but its effects are negated." Gaia Knight (Level 6) goes
/// back to the Extra Deck and Goyo Guardian (Level 6) comes from it. Goyo's effect (taking the monster it destroys by battle)
/// is negated: in turn 3 it destroys Warwolf, which stays in the Graveyard; a Goyo that was not shifted in takes it.
#[test]
fn star_shift_swaps_a_synchro_monster_for_another_of_its_level_with_effects_negated() {
    let shift = card("Star Shift");
    let Some(mut s) = field_extra([&[GOYO], &[]], &[set(0, shift), mon(0, GAIA_KNIGHT), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(shift)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "extra", GAIA_KNIGHT), s.count(0, "mzone", GAIA_KNIGHT)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "mzone", GOYO), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", shift), 1, "{}", s.context());
    // Turn 3: the negated Goyo Guardian destroys Warwolf and takes nothing.
    let Some(mut s) = field_extra([&[GOYO], &[]], &[set(0, shift), mon(0, GAIA_KNIGHT), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(shift), EndTurn, Attack(GOYO)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(0, "mzone", WARWOLF)), (1, 0), "Goyo's effect is negated: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
    // The control: a Goyo Guardian on the field takes Warwolf (the optional trigger is answered yes).
    let Some(mut s) = field(&[], &[mon(0, GOYO), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Attack(GOYO), Chain(GOYO)]).unwrap();
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Star Shift needs another Synchro Monster of the same Level with another name in the Extra Deck: a second Gaia Knight (the
/// same name) and Junk Warrior (Level 5) are not enough.
#[test]
fn star_shift_needs_a_synchro_monster_of_the_same_level_and_another_name() {
    let shift = card("Star Shift");
    let Some(mut s) = field_extra([&[GAIA_KNIGHT, JUNK_WARRIOR], &[]], &[set(0, shift), mon(0, GAIA_KNIGHT)]) else { return };
    keep_hands(&mut s);
    never(s.run(&[Activate(shift)]));
    assert_eq!(s.count(0, "mzone", GAIA_KNIGHT), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Stay Force, Summon Tax, Superior Protector
// ---------------------------------------------------------------------------------------------------------------

/// Stay Force: "If a monster you control would make itself leave the field during the End Phase: You can pay 1000 LP; its
/// effects are negated, until the end of this turn." Yata-Garasu, Normal Summoned, would return to my hand in the End Phase:
/// with Stay Force I pay 1000 and it stays on the field. Without it Yata-Garasu returns to the hand.
#[test]
fn stay_force_pays_1000_lp_to_keep_a_monster_from_leaving_the_field_in_the_end_phase() {
    let [stay, yata] = ["Stay Force", "Yata-Garasu"].map(card);
    let Some(mut s) = field(&[], &[set(0, stay), hand(0, yata)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(stay), Summon(yata), Chain(stay)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1000, "{}", s.context());
    assert_eq!((s.count(0, "mzone", yata), s.count(0, "hand", yata)), (1, 0), "Yata-Garasu stayed: {}", s.context());
    // The control: no Stay Force.
    let Some(mut s) = field(&[], &[hand(0, yata)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Summon(yata), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", yata), s.count(0, "hand", yata)), (0, 1), "returned to the hand: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Summon Tax: "When a monster(s) is Summoned: The Summoning player takes damage equal to half the combined ATK of those
/// Summoned monster(s). Destroy this card during your 2nd Standby Phase after activation." I Normal Summon Dark Blade (1800)
/// and take 900; the opponent Normal Summons Warwolf (2000) and takes 1000. Summon Tax is still there after my turn 3's
/// Standby Phase and destroyed in the Standby Phase of my turn 5.
#[test]
fn summon_tax_hurts_the_summoning_player_and_is_destroyed_in_my_2nd_standby_phase() {
    let tax = card("Summon Tax");
    let spawns = [set(0, tax), hand(0, DARK_BLADE), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(tax), Summon(DARK_BLADE), EndTurn], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 900, 8000 - 1000), "half of each ATK: {}", s.context());
    assert_eq!(s.count(0, "szone", tax), 1, "{}", s.context());
    // Turns 3 and 4: my 1st Standby Phase passes, Summon Tax stays.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.count(0, "szone", tax), s.count(0, "grave", tax)), (1, 0), "still there after my 1st Standby Phase: {}", s.context());
    // Turns 5 and 6: destroyed in my 2nd Standby Phase.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", tax), s.count(0, "grave", tax)), (0, 1), "destroyed in the 2nd Standby Phase: {}", s.context());
}

/// Superior Protector: "Equip this card to a monster you control. You take no battle damage from battles involving monsters
/// you control, except the equipped monster." Equipped to Dark Blade; the opponent's Warwolf (2000) destroys my Thunder Kid
/// (700) and I take no damage (the 1300 would have been taken without it).
#[test]
fn superior_protector_stops_the_battle_damage_of_the_other_monsters() {
    let protector = card("Superior Protector");
    let Some(mut s) = field(&[], &[set(0, protector), mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run(&[Activate(protector)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", protector), 1, "equipped, so it stays: {}", s.context());
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Thunder Kid was destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    // The control: no Protector.
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}

/// Superior Protector: "If an opponent's monster attacks the equipped Defense Position monster, take piercing battle
/// damage. If the equipped monster would be destroyed by battle, you can destroy this card instead." Dark Blade (DEF 1500)
/// in Defense Position is attacked by Warwolf (2000): I take 500 of piercing damage and, answering yes, the Protector is
/// destroyed in its place and Dark Blade stays.
#[test]
fn superior_protector_pierces_the_equipped_defense_monster_and_is_destroyed_in_its_place() {
    let protector = card("Superior Protector");
    let Some(mut s) = field(&[], &[set(0, protector), mon_def(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(protector)]).unwrap();
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 500, "2000 - 1500, piercing: {}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(0, "grave", protector)), (1, 1), "the Protector took the hit: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Synchro Barrier Force, Synchro Baton, Synchro Blast
// ---------------------------------------------------------------------------------------------------------------

/// Synchro Barrier Force: "When a card or effect is activated that would destroy a card(s) on the field: Negate that effect,
/// then inflict 500 damage to your opponent for each Synchro Monster you control." The opponent's Dark Hole is negated; with
/// Goyo Guardian and Junk Warrior on my field they take 1000, and my monsters (Warwolf too) are not destroyed.
#[test]
fn synchro_barrier_force_negates_a_destroying_effect_and_burns_500_per_synchro_monster() {
    let barrier = card("Synchro Barrier Force");
    let spawns = [set(0, barrier), mon(0, GOYO), mon(0, JUNK_WARRIOR), mon(0, WARWOLF), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(barrier)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1000), "2 Synchro Monsters, 500 each: {}", s.context());
    assert_eq!((s.count(0, "mzone", GOYO), s.count(0, "mzone", JUNK_WARRIOR), s.count(0, "mzone", WARWOLF)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.count(0, "grave", barrier), s.count(1, "grave", DARK_HOLE)), (1, 1), "{}", s.context());
    // Not against an effect that destroys nothing: Dian Keto (a heal) does not start it.
    let Some(mut s) = field(&[], &[set(0, barrier), mon(0, GOYO), hand(1, 4348)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(barrier)], &[Activate(4348)]]));
    assert_eq!(s.lp(1), 9000, "{}", s.context());
}

/// Synchro Baton: "Target 1 face-up Synchro Monster you control. It gains 600 ATK for each Synchro Monster in your
/// Graveyard." Two Synchro Monsters are in my Graveyard: Goyo Guardian gains 1200 (2800 to 4000) and keeps it past the End
/// Phase. With none in the Graveyard Synchro Baton is not offered.
#[test]
fn synchro_baton_gives_600_atk_for_each_synchro_monster_in_the_graveyard() {
    let baton = card("Synchro Baton");
    let spawns = [set(0, baton), mon(0, GOYO), grave(0, JUNK_WARRIOR), grave(0, GAIA_KNIGHT), grave(0, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(baton)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GOYO), Some(2800 + 1200), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, GOYO), Some(2800 + 1200), "the gain stays: {}", s.context());
    let Some(mut s) = field(&[], &[set(0, baton), mon(0, GOYO), grave(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run(&[Activate(baton)]));
    assert_eq!(s.attack_of(0, GOYO), Some(2800), "{}", s.context());
}

/// Synchro Blast: "Once per turn, if a Synchro Monster you control attacks an opponent's monster: Inflict 500 damage to your
/// opponent." Junk Warrior (2300) attacks one monster and Gaia Knight (2600) another: 1600 of battle damage between them
/// and only one 500 (2100 in all). Dark Blade, no Synchro Monster, attacking Warwolf starts nothing.
#[test]
fn synchro_blast_inflicts_500_once_per_turn_when_a_synchro_monster_attacks_a_monster() {
    let blast = card("Synchro Blast");
    let spawns = [set(0, blast), mon(0, JUNK_WARRIOR), mon(0, GAIA_KNIGHT), mon(1, WARWOLF), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(blast), EndTurn, Attack(JUNK_WARRIOR), Chain(blast), Attack(GAIA_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1600 - 500, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF) + s.count(1, "grave", FILLER), 2, "both defenders died: {}", s.context());
    // A monster that is no Synchro Monster: the battle damage only (1800 - 1300).
    let Some(mut s) = field(&[], &[set(0, blast), mon(0, DARK_BLADE), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(blast), EndTurn, Attack(DARK_BLADE)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Synchro Deception (engine/overrides/c511000855.lua), Synchro Destructor, Synchro Out
// ---------------------------------------------------------------------------------------------------------------

/// Synchro Deception: "Send 1 Synchro Monster from your Extra Deck to the Graveyard. If you control face-up monsters that could
/// be used to Synchro Summon that Synchro Monster, 1 other face-up monster you control gains ATK equal to half the ATK of the
/// Synchro Monster, until the End Phase. Monsters you control cannot attack this turn, except the monster that gained ATK by
/// this effect." Junk Warrior goes from the Extra Deck to the Graveyard (Junk Synchron and Petit Dragon are its materials):
/// Warwolf gains 1150 (3150) and attacks for it, and Petit Dragon cannot attack.
#[test]
fn synchro_deception_pumps_one_monster_and_stops_the_others_from_attacking() {
    let deception = card("Synchro Deception");
    let spawns = [set(0, deception), mon(0, JUNK_SYNCHRON), mon(0, PETIT_DRAGON), mon(0, WARWOLF)];
    let Some(mut s) = field_extra([&[JUNK_WARRIOR], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[EndTurn, EndTurn, Activate(deception)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", JUNK_WARRIOR), s.count(0, "extra", JUNK_WARRIOR)), (1, 0), "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000 + 1150), "{}", s.context());
    assert_eq!(s.attack_of(0, PETIT_DRAGON), Some(600), "{}", s.context());
    s.run(&[Attack(WARWOLF)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 3150, "{}", s.context());
    // The others cannot attack: Petit Dragon is not offered.
    let Some(mut s) = field_extra([&[JUNK_WARRIOR], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    let err = s.run(&[EndTurn, EndTurn, Activate(deception), Attack(PETIT_DRAGON)]).expect_err("only Warwolf can attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{WARWOLF}")) && !err.contains(&format!("\"cid\":{PETIT_DRAGON}")), "{err}");
}

/// Synchro Deception without monsters that could Synchro Summon the monster: only the Extra Deck's Junk Warrior is sent, nothing
/// gains ATK and every monster can attack (Junk Warrior needs Junk Synchron and Warwolf (Level 4) is no match).
#[test]
fn synchro_deception_without_materials_only_sends_the_synchro_monster() {
    let deception = card("Synchro Deception");
    let Some(mut s) = field_extra([&[JUNK_WARRIOR], &[]], &[set(0, deception), mon(0, WARWOLF), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, EndTurn, Activate(deception)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", JUNK_WARRIOR), 1, "{}", s.context());
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(0, DARK_BLADE)), (Some(2000), Some(1800)), "{}", s.context());
    s.run(&[Attack(WARWOLF), Attack(DARK_BLADE)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 2000 - 1800, "both attacked: {}", s.context());
}

/// Synchro Deception chained to another Synchro Deception: the second sends the only Synchro Monster and the first, resolving
/// last, finds the Extra Deck without one (the fuzz's Lua error, indexing an empty selection): it does nothing and nothing
/// breaks.
#[test]
fn synchro_deception_chained_to_itself_finds_nothing_left_to_send() {
    let deception = card("Synchro Deception");
    let spawns = [set(0, deception), set(0, deception), mon(0, WARWOLF)];
    let Some(mut s) = field_extra([&[JUNK_WARRIOR], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(deception), Chain(deception)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", deception), s.count(0, "grave", JUNK_WARRIOR)), (2, 1), "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "{}", s.context());
}

/// Synchro Destructor: "When a Synchro Monster you control destroys an opponent's monster by battle: Inflict damage to your
/// opponent equal to half the original ATK of the destroyed monster, then if the destroyed monster is a Synchro Monster,
/// inflict damage to your opponent equal to half the original ATK of the destroyed monster." Gaia Knight (2600) destroys
/// Junk Warrior (2300, a Synchro Monster): 300 of battle damage and 1150 twice. Against Warwolf (2000) it is 600 and 1000 once.
#[test]
fn synchro_destructor_burns_half_the_atk_of_the_destroyed_monster_twice_for_a_synchro_monster() {
    let destructor = card("Synchro Destructor");
    let Some(mut s) = field(&[], &[set(0, destructor), mon(0, GAIA_KNIGHT), mon(1, JUNK_WARRIOR)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Attack(GAIA_KNIGHT), Chain(destructor)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", JUNK_WARRIOR), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 300 - 2 * 1150, "{}", s.context());
    assert_eq!(s.count(0, "grave", destructor), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, destructor), mon(0, GAIA_KNIGHT), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Attack(GAIA_KNIGHT), Chain(destructor)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 600 - 1000, "{}", s.context());
    // A monster that is no Synchro Monster destroying one does not start it.
    let Some(mut s) = field(&[], &[set(0, destructor), mon(0, DARK_BLADE), mon(1, FILLER)]) else { return };
    keep_hands(&mut s);
    never(s.run(&[EndTurn, Attack(DARK_BLADE), Chain(destructor)]));
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

/// Synchro Out: "Target 1 Synchro Monster you control; return it to the Extra Deck, then, if all the Synchro Materials that
/// were used for its Synchro Summon of that monster are in your Graveyard, Special Summon them, but destroy them during the
/// End Phase." Junk Warrior, Synchro Summoned from Junk Synchron and Petit Dragon, goes back to the Extra Deck, both
/// materials come back to the field, and they are destroyed in the End Phase.
#[test]
fn synchro_out_returns_a_synchro_monster_and_revives_its_materials_until_the_end_phase() {
    let out = card("Synchro Out");
    let spawns = [set(0, out), mon(0, JUNK_SYNCHRON), mon(0, PETIT_DRAGON)];
    let Some(mut s) = field_extra([&[JUNK_WARRIOR], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(JUNK_WARRIOR), Activate(out)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "extra", JUNK_WARRIOR), s.count(0, "mzone", JUNK_WARRIOR)), (1, 0), "{}", s.context());
    assert_eq!((s.count(0, "mzone", JUNK_SYNCHRON), s.count(0, "mzone", PETIT_DRAGON)), (1, 1), "the materials are back: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", JUNK_SYNCHRON), s.count(0, "grave", PETIT_DRAGON)), (1, 1), "destroyed in the End Phase: {}", s.context());
}

/// Synchro Out on a Synchro Monster that was not Synchro Summoned (put on the field by an effect): it returns to the Extra Deck
/// and no monster comes back from the Graveyard, whatever lies there.
#[test]
fn synchro_out_revives_nothing_for_a_synchro_monster_that_was_not_synchro_summoned() {
    let out = card("Synchro Out");
    let spawns = [set(0, out), mon(0, JUNK_WARRIOR), grave(0, JUNK_SYNCHRON), grave(0, PETIT_DRAGON)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(out)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "extra", JUNK_WARRIOR), s.count(0, "mzone", JUNK_WARRIOR)), (1, 0), "{}", s.context());
    assert_eq!((s.count(0, "mzone", JUNK_SYNCHRON), s.count(0, "mzone", PETIT_DRAGON)), (0, 0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Synchro Overlimit, Synchro Prominence, Synchro Ring, Synchro Striker Unit
// ---------------------------------------------------------------------------------------------------------------

/// Synchro Overlimit: "When a Synchro Monster you control destroys an opponent's monster by battle: It can attack once again
/// this turn, but if it does, destroy it after damage calculation." Gaia Knight (2600) destroys Thunder Kid (700): 1900
/// damage; it attacks again (Feral Imp, 1300: 1300 damage) and is destroyed after that damage calculation.
#[test]
fn synchro_overlimit_gives_an_extra_attack_and_destroys_the_attacker_after_it() {
    let overlimit = card("Synchro Overlimit");
    let spawns = [set(0, overlimit), mon(0, GAIA_KNIGHT), mon(1, THUNDER_KID), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, THUNDER_KID), (1, FILLER)]);
    s.run(&[EndTurn, Attack(GAIA_KNIGHT), Chain(overlimit), Attack(GAIA_KNIGHT)]).unwrap();
    to_main_phase_2(&mut s);
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1900 - 1300, "two attacks: {}", s.context());
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "grave", FILLER)), (1, 1), "{}", s.context());
    assert_eq!((s.count(0, "grave", GAIA_KNIGHT), s.count(0, "grave", overlimit)), (1, 1), "destroyed after the 2nd attack: {}", s.context());
}

/// Synchro Overlimit: the attacker is destroyed only if it attacks again. Gaia Knight destroys Thunder Kid, I answer with
/// Overlimit and then decline the second attack (the Battle Phase is left): Gaia Knight is still on the field.
#[test]
fn synchro_overlimit_destroys_the_attacker_only_if_it_attacks_again() {
    let overlimit = card("Synchro Overlimit");
    let spawns = [set(0, overlimit), mon(0, GAIA_KNIGHT), mon(1, THUNDER_KID), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, THUNDER_KID)]);
    s.run(&[EndTurn, Attack(GAIA_KNIGHT), Chain(overlimit)]).unwrap();
    to_main_phase_2(&mut s);
    clean(&s);
    assert_eq!(s.count(0, "mzone", GAIA_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", overlimit), 1, "{}", s.context());
}

/// Synchro Prominence: "Each player who controls a Synchro Monster(s) can inflict 1000 damage to their opponent." Both
/// players control one and say yes: each loses 1000. When only I control one, only the opponent loses LP.
#[test]
fn synchro_prominence_lets_each_player_with_a_synchro_monster_inflict_1000() {
    let prominence = card("Synchro Prominence");
    let Some(mut s) = field(&[], &[set(0, prominence), mon(0, GOYO), mon(1, JUNK_WARRIOR)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(prominence)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (7000, 7000), "{}", s.context());
    assert_eq!(s.count(0, "grave", prominence), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, prominence), mon(0, GOYO), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(prominence)]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7000), "{}", s.context());
    // Nobody controls a Synchro Monster: it cannot be activated.
    let Some(mut s) = field(&[], &[set(0, prominence), mon(0, WARWOLF), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run(&[Activate(prominence)]));
    assert_eq!((s.lp(0), s.lp(1), s.count(0, "szone", prominence)), (8000, 8000, 1), "{}", s.context());
}

/// Synchro Ring: "During your Battle Phase: Target 1 Synchro Monster you control that attacked an opponent's monster this turn,
/// but did not destroy it by battle; Double its ATK until the End Phase and it can attack once again during the same Battle
/// Phase." Armory Arm (1800) attacks Dark Magician in Defense Position (2100 DEF): it destroys nothing and I lose 300. Doubled
/// to 3600 it attacks again and destroys Dark Magician.
#[test]
fn synchro_ring_doubles_the_atk_of_a_synchro_monster_that_did_not_destroy_its_target_and_lets_it_attack_again() {
    let ring = card("Synchro Ring");
    let spawns = [set(0, ring), mon(0, ARMORY_ARM), Spawn { team: 1, place: MonsterDefense, cid: DARK_MAGICIAN }];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Attack(ARMORY_ARM)]).unwrap();
    assert_eq!((s.lp(0), s.count(1, "mzone", DARK_MAGICIAN)), (8000 - 300, 1), "{}", s.context());
    // Back at the Battle Phase's menu: the Ring is activated there.
    let list = s.prompt()["chains"].as_array().cloned().unwrap_or_default();
    let at = list.iter().position(|c| c["cid"] == ring).unwrap_or_else(|| panic!("the Ring is offered in the Battle Phase: {}", s.prompt()));
    s.respond(serde_json::json!({"type": 1, "action": battle::CHAIN, "index": at})).unwrap();
    s.run(&[Attack(ARMORY_ARM)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, ARMORY_ARM), Some(3600), "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "the 2nd attack destroyed it: {}", s.context());
    assert_eq!(s.lp(1), 8000, "a Defense Position monster: no damage to them: {}", s.context());
}

/// Synchro Striker Unit: "Target 1 Synchro monster you control; equip this card to that target. It gains 1000 ATK. Once per
/// turn, during your End Phase: It loses 800 ATK." Goyo Guardian (2800) gains 1000 (3800), loses 800 at the end of my turn 1
/// (3000) and 800 more at the end of turn 3 (2200).
#[test]
fn synchro_striker_unit_gives_1000_atk_and_takes_800_each_of_my_end_phases() {
    let striker = card("Synchro Striker Unit");
    let Some(mut s) = field(&[], &[set(0, striker), mon(0, GOYO)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(striker)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GOYO), Some(3800), "{}", s.context());
    assert_eq!(s.count(0, "szone", striker), 1, "equipped: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GOYO), Some(3000), "after my turn 1's End Phase: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, GOYO), Some(2200), "after turn 3's: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Synchron Believer, Synchron Keeper, Talisman of Reversal
// ---------------------------------------------------------------------------------------------------------------

/// Synchron Believer: "When a face-up Synchro Monster you control is targeted for an attack: you can Special Summon this card
/// from your hand and negate the attack." Warwolf (2000) attacks my Goyo Guardian: Synchron Believer (100/100) comes to the
/// field and the attack is negated (no damage, nothing destroyed). Warwolf attacking a monster that is no Synchro Monster
/// does not offer it.
#[test]
fn synchron_believer_comes_from_the_hand_and_negates_an_attack_on_a_synchro_monster() {
    let believer = card("Synchron Believer");
    let Some(mut s) = field(&[], &[hand(0, believer), mon(0, GOYO), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(believer)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", believer), s.count(0, "hand", believer)), (1, 0), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!((s.count(0, "mzone", GOYO), s.count(1, "mzone", WARWOLF)), (1, 1), "{}", s.context());
    let b = stats(&s, 0, believer);
    assert_eq!((b.0, b.1, b.2), (Some(100), Some(100), Some(1)), "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, believer), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(believer)], &[Attack(WARWOLF)]]));
    assert_eq!((s.count(0, "hand", believer), s.lp(0)), (1, 8000 - 200), "{}", s.context());
}

/// Synchron Keeper: "When you would take damage by a card effect: you can banish 1 Tuner monster from your Graveyard and this
/// card from your hand; Negate that effect, and Special Summon 1 Synchro Monster from your Extra Deck whose Level is equal to
/// the combined Levels of the banished monsters. Negate the effect(s) of that Synchro Monster." The opponent's Ookazi (800
/// damage) is answered with Synchron Keeper (Level 5) and Tune Warrior (Level 3) from my Graveyard: no damage, and Stardust
/// Dragon (Level 8) comes from the Extra Deck.
#[test]
fn synchron_keeper_negates_effect_damage_and_summons_a_synchro_monster_of_the_combined_level() {
    let keeper = card("Synchron Keeper");
    let spawns = [hand(0, keeper), grave(0, TUNE_WARRIOR), hand(1, OOKAZI)];
    let Some(mut s) = field_extra([&[STARDUST, GOYO], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(keeper)], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "Ookazi was negated: {}", s.context());
    assert_eq!(s.count(0, "mzone", STARDUST), 1, "Level 5 + Level 3 = 8: {}", s.context());
    assert_eq!((s.count(0, "removed", keeper), s.count(0, "removed", TUNE_WARRIOR)), (1, 1), "both banished: {}", s.context());
    assert_eq!(s.count(0, "extra", GOYO), 1, "Goyo (Level 6) stays: {}", s.context());
    assert_eq!(s.count(1, "grave", OOKAZI), 1, "{}", s.context());
    // Without a Tuner in the Graveyard it is not offered.
    let Some(mut s) = field_extra([&[STARDUST, GOYO], &[]], &[hand(0, keeper), hand(1, OOKAZI)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(keeper)], &[Activate(OOKAZI)]]));
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
}

/// Talisman of Reversal: "When a card(s) is added to your opponent's hand, except during their Draw Phase: Draw cards until you
/// have the same number of cards in your hand as your opponent." The opponent's Pot of Greed (draw 2) leaves them with more
/// cards than I have; I draw until the hands are equal.
#[test]
fn talisman_of_reversal_draws_until_the_hands_are_equal() {
    let talisman = card("Talisman of Reversal");
    let Some(mut s) = field(&[], &[set(0, talisman), hand(1, POT_OF_GREED)]) else { return };
    keep_hands(&mut s);
    let before = hand_size(&s, 0);
    s.run_both([&[Chain(talisman)], &[EndTurn, Activate(POT_OF_GREED)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", talisman), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), hand_size(&s, 1), "equal hands: {}", s.context());
    assert!(hand_size(&s, 0) > before, "I drew: {}", s.context());
    // The control: no Talisman.
    let Some(mut s) = field(&[], &[hand(1, POT_OF_GREED)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[EndTurn, Activate(POT_OF_GREED)]]).unwrap();
    assert!(hand_size(&s, 1) > hand_size(&s, 0), "{}", s.context());
}

/// Talisman of Reversal does not answer the opponent's Draw Phase draw (the card added to their hand is the normal draw).
#[test]
fn talisman_of_reversal_ignores_the_draw_phase() {
    let talisman = card("Talisman of Reversal");
    let Some(mut s) = field(&[], &[set(0, talisman)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(talisman)], &[EndTurn]]));
    assert_eq!(s.count(0, "szone", talisman), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Trick Mirror, Trust Chain, Tuner's Explosion
// ---------------------------------------------------------------------------------------------------------------

/// Trick Mirror: "Activate only when your opponent activates a Trap Card: this card's effect is activated as that card's effect
/// instead." The opponent's Just Desserts (500 damage for each monster the other player controls) is answered with Trick
/// Mirror: it runs as my effect, and the opponent takes 500 for each of their own 2 monsters, where the Just Desserts alone
/// would have hit me for 500 for each of mine.
#[test]
fn trick_mirror_runs_the_opponents_trap_as_my_own_effect() {
    let mirror = card("Trick Mirror");
    let spawns = [set(0, mirror), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(0, WARWOLF), mon(1, FILLER), mon(1, PETIT_DRAGON), set(1, JUST_DESSERTS)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(mirror)], &[Activate(JUST_DESSERTS)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1000), "500 for each of their 2 monsters: {}", s.context());
    assert_eq!((s.count(0, "grave", mirror), s.count(1, "grave", JUST_DESSERTS)), (1, 1), "{}", s.context());
    // The control: Just Desserts alone burns me for my 3 monsters.
    let spawns = [mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(0, WARWOLF), mon(1, FILLER), mon(1, PETIT_DRAGON), set(1, JUST_DESSERTS)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(JUST_DESSERTS)]]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1500, 8000), "{}", s.context());
}

/// Trick Mirror answers a Trap Card only: the opponent's Ookazi, a Spell, does not offer it.
#[test]
fn trick_mirror_does_not_answer_a_spell() {
    let mirror = card("Trick Mirror");
    let Some(mut s) = field(&[], &[set(0, mirror), hand(1, OOKAZI)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(mirror)], &[Activate(OOKAZI)]]));
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
}

/// Trust Chain: "When you activate this card, target 1 face-up monster you control. It gains ATK equal to the combined ATK of
/// all face-up Synchro Monsters you control. While this card is face-up on the field, Synchro Monsters you control cannot
/// attack." Warwolf gains 2300 + 2600 (Junk Warrior and Gaia Knight): 6900. In turn 3 Warwolf attacks directly for it; the
/// Synchro Monsters are not offered an attack.
#[test]
fn trust_chain_gives_a_monster_the_atk_of_all_synchro_monsters_and_forbids_their_attacks() {
    let chain = card("Trust Chain");
    let spawns = [set(0, chain), mon(0, WARWOLF), mon(0, JUNK_WARRIOR), mon(0, GAIA_KNIGHT)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(chain)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000 + 2300 + 2600), "{}", s.context());
    assert_eq!(s.count(0, "szone", chain), 1, "{}", s.context());
    let err = s.run(&[EndTurn, EndTurn, Attack(JUNK_WARRIOR)]).expect_err("a Synchro Monster cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{WARWOLF}")) && !err.contains(&format!("\"cid\":{JUNK_WARRIOR}")), "{err}");
    let Some(mut s) = field(&[], &[set(0, chain), mon(0, WARWOLF), mon(0, JUNK_WARRIOR), mon(0, GAIA_KNIGHT)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(chain), EndTurn, EndTurn, Attack(WARWOLF)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 6900, "{}", s.context());
}

/// Trust Chain: "During the Battle Phase, you can destroy this face-up card to end the Battle Phase." In my turn 3, from the
/// Battle Phase's menu, it destroys itself and the Battle Phase is over: no attack was made, and the opponent keeps their LP and
/// their Feral Imp.
#[test]
fn trust_chain_destroys_itself_to_end_the_battle_phase() {
    let chain = card("Trust Chain");
    let spawns = [set(0, chain), mon(0, WARWOLF), mon(0, DARK_BLADE), mon(0, JUNK_WARRIOR), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(chain)]).unwrap();
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
    for _ in 0..6 {
        if s.prompt()["type"] == msg::SELECT_BATTLECMD {
            break;
        }
        s.pass().unwrap();
    }
    let list = s.prompt()["chains"].as_array().cloned().unwrap_or_default();
    let at = list.iter().position(|c| c["cid"] == chain).unwrap_or_else(|| panic!("Trust Chain is offered in the Battle Phase: {}", s.context()));
    s.respond(serde_json::json!({"type": 1, "action": battle::CHAIN, "index": at})).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", chain), 1, "{}", s.context());
    assert_ne!(s.prompt()["type"], msg::SELECT_BATTLECMD, "the Battle Phase is over: {}", s.context());
    assert_eq!((s.lp(1), s.count(1, "mzone", FILLER)), (8000, 1), "nothing attacked: {}", s.context());
}

/// Tuner's Explosion: "Tribute Tuner monsters you control equal to the number of monsters your opponent controls. Destroy all
/// monsters your opponent controls, then inflict 1000 damage to your opponent for each monster destroyed." The opponent has
/// two monsters: I Tribute Tune Warrior and Junk Synchron (Tuners), both are destroyed and they take 2000.
#[test]
fn tuners_explosion_tributes_tuners_and_destroys_all_the_opponents_monsters_for_1000_each() {
    let explosion = card("Tuner's Explosion");
    let spawns = [set(0, explosion), mon(0, TUNE_WARRIOR), mon(0, JUNK_SYNCHRON), mon(0, WARWOLF), mon(1, FILLER), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(explosion)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", TUNE_WARRIOR), s.count(0, "grave", JUNK_SYNCHRON), s.count(0, "mzone", WARWOLF)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.count(1, "grave", FILLER), s.count(1, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
    // Two monsters on their side and one Tuner on mine: it cannot be activated.
    let Some(mut s) = field(&[], &[set(0, explosion), mon(0, TUNE_WARRIOR), mon(1, FILLER), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    never(s.run(&[Activate(explosion)]));
    assert_eq!((s.count(0, "mzone", TUNE_WARRIOR), s.count(1, "mzone", FILLER), s.lp(1)), (1, 1, 8000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Wave Force, Wave Rebound, Wave Wall, Wild Heart
// ---------------------------------------------------------------------------------------------------------------

/// Wave Force: "When a Synchro Monster you control is targeted for an attack: Destroy all Attack Position monsters your
/// opponent controls." Warwolf attacks Goyo Guardian: Warwolf and Dark Blade (Attack Position) are destroyed, Feral Imp (in
/// Defense Position) stays, and no damage is taken.
#[test]
fn wave_force_destroys_every_attack_position_monster_of_the_opponent() {
    let force = card("Wave Force");
    let spawns = [set(0, force), mon(0, GOYO), mon(1, WARWOLF), mon(1, DARK_BLADE), mon_def(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(force)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_BLADE), s.count(1, "mzone", FILLER)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.count(0, "mzone", GOYO), s.count(0, "grave", force)), (8000, 1, 1), "{}", s.context());
    // A monster that is no Synchro Monster being attacked does not start it.
    let Some(mut s) = field(&[], &[set(0, force), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[Chain(force)], &[Attack(WARWOLF)]]));
    assert_eq!((s.lp(0), s.count(1, "mzone", WARWOLF)), (8000 - 200, 1), "{}", s.context());
}

/// Wave Rebound: "When an effect that Special Summons Synchro Monster(s) from the Graveyard is activated: Negate its activation
/// and inflict damage to your opponent equal to the ATK of the Synchro Monster(s)." The opponent Synchro Summons Armory Arm
/// (1800) in turn 2 and my Dark Magician (2500) destroys it in turn 3 (700 damage); their Monster Reborn on it in turn 4 is
/// negated and they take 1800. A Monster Reborn on Warwolf (no Synchro Monster) does not start it.
#[test]
fn wave_rebound_negates_a_revival_of_a_synchro_monster_and_burns_its_atk() {
    let rebound = card("Wave Rebound");
    let spawns = [set(0, rebound), mon(0, DARK_MAGICIAN), mon(1, TUNE_WARRIOR), mon(1, SKULL_SERVANT), hand(1, MONSTER_REBORN)];
    let Some(mut s) = field_extra([&[], &[ARMORY_ARM]], &spawns) else { return };
    s.prefer(&[(1, ARMORY_ARM), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Attack(DARK_MAGICIAN), Chain(rebound)], &[SpSummon(ARMORY_ARM), EndTurn, Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 700 - 1800), "{}", s.context());
    assert_eq!((s.count(1, "mzone", ARMORY_ARM), s.count(1, "grave", ARMORY_ARM)), (0, 1), "{}", s.context());
    assert_eq!((s.count(0, "grave", rebound), s.count(1, "grave", MONSTER_REBORN)), (1, 1), "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, rebound), hand(1, MONSTER_REBORN), grave(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    never(s.run_both([&[Chain(rebound)], &[Activate(MONSTER_REBORN)]]));
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Wave Wall: "During each of your opponent's Battle Phases, you can negate attacks from monsters your opponent controls, up to
/// the number of face-up Synchro Monsters you control." With one Synchro Monster (Goyo Guardian) the first of the opponent's two
/// attacks is negated and the second is not: Warwolf's attack is negated, and Dark Blade's goes through and is destroyed by
/// Junk Warrior (2300 against 1800: 500 damage).
#[test]
fn wave_wall_negates_as_many_attacks_as_i_have_synchro_monsters() {
    let wall = card("Wave Wall");
    let spawns = [mon_def(0, wall), mon(0, JUNK_WARRIOR), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, JUNK_WARRIOR)]);
    s.run_both([&[ChainOnAttack(wall)], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    // Warwolf's attack was negated (it lives); Dark Blade's was not (it died against Goyo Guardian).
    assert_eq!((s.count(1, "mzone", WARWOLF), s.count(1, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "Junk Warrior (2300) beat Dark Blade (1800): {}", s.context());
    // No Synchro Monster: Wave Wall is not offered.
    let Some(mut s) = field(&[], &[mon_def(0, wall), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never(s.run_both([&[ChainOnAttack(wall)], &[Attack(WARWOLF)]]));
}

/// Wild Heart: "When a monster is sent from the hand to the Graveyard: You can Special Summon this card from your hand."
/// My Hand Destruction sends 2 Feral Imps from my hand to the Graveyard (and 2 from the opponent's): Wild Heart (1000/1000)
/// is Special Summoned from the hand in face-up Attack Position.
#[test]
fn wild_heart_is_special_summoned_when_a_monster_is_sent_from_the_hand_to_the_graveyard() {
    let wild = card("Wild Heart");
    let Some(mut s) = field(&[], &[hand(0, wild), hand(0, card("Hand Destruction"))]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(card("Hand Destruction")), Chain(wild)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", wild), s.count(0, "hand", wild)), (1, 0), "{}", s.context());
    assert_eq!(s.position_of(0, wild), Some(1), "face-up Attack: {}", s.context());
    let w = stats(&s, 0, wild);
    assert_eq!((w.0, w.1, w.2), (Some(1000), Some(1000), Some(4)), "{}", s.context());
}

/// Wild Heart: "This card can attack directly. If it does with this effect, destroy it at the end of the Damage Step." In turn 3
/// Wild Heart attacks the opponent's LP directly although they control Warwolf: 1000 damage, and it is destroyed after it.
#[test]
fn wild_heart_attacks_directly_and_is_destroyed_at_the_end_of_the_damage_step() {
    let wild = card("Wild Heart");
    let Some(mut s) = field(&[], &[mon(0, wild), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Attack(wild)]).unwrap();
    to_main_phase_2(&mut s);
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "a direct attack, though Warwolf stands: {}", s.context());
    assert_eq!((s.count(0, "grave", wild), s.count(0, "mzone", wild), s.count(1, "mzone", WARWOLF)), (1, 0, 1), "{}", s.context());
}

/// Wild Heart: "When this card on the field is destroyed by a card effect, except its own: You can Special Summon it from the
/// Graveyard. If this card is Special Summoned this way, banish it during the End Phase." The opponent's Dark Hole destroys it;
/// it is Special Summoned from the Graveyard and banished in the End Phase of that turn.
#[test]
fn wild_heart_comes_back_from_the_graveyard_and_is_banished_in_the_end_phase() {
    let wild = card("Wild Heart");
    let Some(mut s) = field(&[], &[mon(0, wild), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(wild)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", wild), s.count(0, "grave", wild)), (1, 0), "Special Summoned from the Graveyard: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", wild), s.count(0, "removed", wild), s.count(0, "grave", wild)), (0, 1, 0), "banished in the End Phase: {}", s.context());
}

/// Skill Twist against Arcana Force XII - The Hangman (VG) (engine/overrides/c100000106.lua): Hangman's End Phase effect
/// targets a monster of the player its coin names; Skill Twist asks the effect about each other monster of mine as a new
/// target, and Hangman's check named a player nobody had set (a Lua error, the fuzz's). On a tails seed Hangman targets my Thunder
/// Kid, Skill Twist changes the target to my Dark Blade and that is the monster destroyed. On a heads seed Hangman targets
/// its controller's monsters and Skill Twist (my monsters are no legal target) is not offered.
#[test]
fn skill_twist_changes_the_target_of_the_hangman_vg_without_a_lua_error() {
    let [twist, hangman] = ["Skill Twist", "Arcana Force XII - The Hangman (VG)"].map(card);
    let (mut redirected, mut heads) = (0, 0);
    for seed in 1..=6 {
        let spawns = [set(0, twist), mon(0, THUNDER_KID), mon(0, DARK_BLADE), hand(1, hangman), mon(1, FILLER), mon(1, FILLER)];
        let Some(mut s) = duel(seed, 8000, [FILLER, FILLER], &[], &spawns) else { return };
        s.prefer(&[(0, THUNDER_KID), (0, DARK_BLADE), (1, FILLER)]);
        match s.run_both([&[Chain(twist)], &[Summon(hangman)]]) {
            Ok(()) => {
                // Run on into the End Phase of team 1's turn 2, where Hangman's effect works.
                clean(&s);
                if s.count(0, "grave", twist) == 1 {
                    redirected += 1;
                    assert_eq!((s.count(0, "mzone", THUNDER_KID), s.count(0, "grave", DARK_BLADE)), (1, 1), "seed {seed}: {}", s.context());
                } else {
                    heads += 1;
                }
            }
            Err(e) => {
                assert!(e.contains("never became possible"), "seed {seed}: {e}");
                heads += 1;
            }
        }
    }
    assert!(redirected > 0 && heads > 0, "both coin faces in 6 seeds: tails {redirected}, heads {heads}");
}
