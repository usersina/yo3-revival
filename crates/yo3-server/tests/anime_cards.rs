//! Anime card versions (#60, the `anime-cards` setting): the reviewed cards of `engine/anime-cards.json` play by their
//! anime effects through the real core, both seats' client models checked after every step (`duelcheck::scenario`); with
//! the setting off, or for a card the map holds back, the game's own card plays. Skipped before `engine/fetch.sh`.
use serde_json::json;
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::ocg::{Response, idle, msg, pos, resp};

const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const HEDGEHOG: u16 = 7701; // Quillbolt Hedgehog: Level 2, 800 ATK
const JUNK_SYNCHRON: u16 = 7687;
const JUNK_WARRIOR: u16 = 7696;
const CARD_OF_SANCTITY: u16 = 5479;
const MIRROR_FORCE: u16 = 4887;
const FLAME_SWORDSMAN: u16 = 4021;
const THOUSAND_DRAGON: u16 = 4075;
const POWER_BOND: u16 = 6398;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn start(anime: bool, decks: [(&[u16], &[u16]); 2]) -> Option<Scenario> {
    let dir = engine_dir()?;
    if !dir.join("vendor/BabelCDB/cards-unofficial.cdb").is_file() || !dir.join("anime-cards.json").is_file() {
        eprintln!("anime data not fetched; skipping");
        return None;
    }
    Some(if anime { Scenario::start_anime(&dir, 7, 8000, decks) } else { Scenario::start(&dir, 7, 8000, decks) }.expect("the duel starts"))
}

/// Junk Warrior (7696): the game's text gains the ATK once, when it is Synchro Summoned; the anime card's ATK is
/// continuous, so a Level 2 monster that arrives later adds to it. Summoned from Junk Synchron and a Hedgehog, then a
/// second Hedgehog is Normal Summoned the next turn.
fn junk_warrior_with_a_later_hedgehog(anime: bool) -> Option<i64> {
    let deck: Vec<u16> = [HEDGEHOG, HEDGEHOG, JUNK_SYNCHRON].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[JUNK_WARRIOR]), (&[WARWOLF; 40], &[])])?;
    s.run(&[Summon(HEDGEHOG), Summon(JUNK_SYNCHRON), SpSummon(JUNK_WARRIOR), Summon(HEDGEHOG)]).unwrap();
    clean(&s);
    Some(s.attack_of(0, JUNK_WARRIOR).expect("Junk Warrior is on the field"))
}

#[test]
fn anime_junk_warrior_counts_level_2_monsters_that_arrive_after_it() {
    let (Some(anime), Some(game)) = (junk_warrior_with_a_later_hedgehog(true), junk_warrior_with_a_later_hedgehog(false)) else { return };
    assert_eq!(game, 2300, "the game's Junk Warrior took the ATK once, when it was summoned");
    assert_eq!(anime, 2300 + 800, "the anime one counts the Hedgehog Normal Summoned after it");
}

/// Card of Sanctity (5479): the anime card has each player draw until they hold six cards; the game's current script
/// is another card's effect (cost: banish your hand and field).
fn hands_after_card_of_sanctity(anime: bool) -> Option<(usize, usize)> {
    let deck: Vec<u16> = [WARWOLF; 35].iter().copied().chain([CARD_OF_SANCTITY, WARWOLF, WARWOLF, WARWOLF, WARWOLF]).collect();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    // The first player holds six cards after the draw; the card needs fewer than six to draw any.
    s.run(&[Summon(WARWOLF), Activate(CARD_OF_SANCTITY)]).unwrap();
    clean(&s);
    let hand = |team: usize| s.state()[team]["hand"].as_array().map_or(0, Vec::len);
    Some((hand(0), hand(1)))
}

#[test]
fn anime_card_of_sanctity_draws_both_players_to_six() {
    let Some(hands) = hands_after_card_of_sanctity(true) else { return };
    assert_eq!(hands, (6, 6), "the first player played two cards from six, the second held five");
    assert_ne!(hands_after_card_of_sanctity(false), Some(hands), "with the setting off it is the game's script");
}

/// Mirror Force (4887): the anime card also damages the attacker's controller by the ATK difference to each of their
/// other monsters it destroys (here Gene-Warped Warwolf, 2000, attacks with a Hedgehog, 800, beside it).
fn lp_after_mirror_force(anime: bool) -> Option<(i64, i64)> {
    let deck0: Vec<u16> = [WARWOLF, MIRROR_FORCE].iter().copied().cycle().take(40).collect();
    let deck1: Vec<u16> = [WARWOLF, HEDGEHOG].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck0, &[]), (&deck1, &[])])?;
    s.run_both([&[SetSpell(MIRROR_FORCE), Chain(MIRROR_FORCE)], &[Summon(HEDGEHOG), Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    assert!(s.monster(1, WARWOLF).is_none() && s.monster(1, HEDGEHOG).is_none(), "both were destroyed: {}", s.state());
    clean(&s);
    Some((s.lp(0), s.lp(1)))
}

#[test]
fn anime_mirror_force_damages_by_the_atk_difference() {
    let Some(anime) = lp_after_mirror_force(true) else { return };
    assert_eq!(anime, (8000, 8000 - (2000 - 800)));
    assert_eq!(lp_after_mirror_force(false), Some((8000, 8000)), "the game's Mirror Force does no damage");
}

/// Cards the map holds back play as they do without the setting: Thousand Dragon is a Fusion Monster in the client (a
/// Normal-turned-Effect Main Deck monster in the anime, whose effect the Deck triggers: held), so it stays in the Extra Deck.
#[test]
fn a_held_card_plays_as_the_game_has_it() {
    let deck: Vec<u16> = [WARWOLF; 40].to_vec();
    let Some(s) = start(true, [(&deck, &[THOUSAND_DRAGON]), (&[WARWOLF; 40], &[])]) else { return };
    assert_eq!(s.count(0, "extra", THOUSAND_DRAGON), 1, "{}", s.state());
    clean(&s);
}

/// Flame Swordsman (4021), a proof card of the card profile: the game's Fusion Monster is the anime's Normal Monster, so
/// with the setting on it is a Main Deck card that is drawn (with it off, it is an Extra Deck card).
#[test]
fn flame_swordsman_is_a_main_deck_card_with_the_anime_versions_on() {
    let deck: Vec<u16> = [WARWOLF; 10].iter().copied().chain([FLAME_SWORDSMAN; 30]).collect();
    let Some(s) = start(true, [(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    assert!(s.count(0, "hand", FLAME_SWORDSMAN) > 0, "a Main Deck card: the opening hand drew some: {}", s.state());
    assert_eq!(s.count(0, "extra", FLAME_SWORDSMAN), 0);
    clean(&s);
}

const ANDRO_SPHINX: u16 = 6087;
const SPHINX_TELEIA: u16 = 6088;
const PYRAMID_OF_LIGHT: u16 = 6079;

/// Andro Sphinx (6087) and Sphinx Teleia (6088), released with their procedure from the Deck: with Pyramid of Light face-up
/// (the anime card, whose alias is the card the script names, 53569894) the engine lists them as a Special Summon from the Deck
/// (location 1), which the translator offers on the Deck pile (`translate_contract.rs`
/// `a_summon_from_the_deck_is_offered_on_the_deck_pile`), and the client's view stays the engine's through the summon.
#[test]
fn the_sphinxes_special_summon_from_the_deck() {
    for sphinx in [ANDRO_SPHINX, SPHINX_TELEIA] {
        let deck: Vec<u16> = [sphinx; 20].iter().copied().chain([PYRAMID_OF_LIGHT; 20]).collect();
        let Some(mut s) = start(true, [(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
        s.run(&[SetSpell(PYRAMID_OF_LIGHT), EndTurn, Activate(PYRAMID_OF_LIGHT), SpSummon(sphinx)]).unwrap();
        assert!(s.monster(0, sphinx).is_some(), "{sphinx} was Special Summoned from the Deck: {}", s.state());
        clean(&s);
    }
}

const CYBER_DRAGON: u16 = 6390;
const CYBER_TWIN_DRAGON: u16 = 6396;

/// Power Bond (6398): the anime card is the Fusion Spell that doubles the summoned Machine's ATK and costs its original ATK at
/// the end of the turn; its upstream database row aliases Polymerization ("treated as Polymerization"), which must not make it
/// Polymerization to the game (that would show another card). The engine reports it by its own Konami id, and it Fusion Summons
/// Cyber Twin Dragon with double the ATK, which Polymerization would not.
#[test]
fn power_bond_keeps_its_identity_and_doubles_the_machine() {
    let deck: Vec<u16> = [WARWOLF; 33].iter().copied().chain([POWER_BOND]).chain([CYBER_DRAGON; 6]).collect();
    let Some(mut s) = start(true, [(&deck, &[CYBER_TWIN_DRAGON]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Activate(POWER_BOND)]).unwrap();
    assert_eq!(s.attack_of(0, CYBER_TWIN_DRAGON), Some(2 * 2800), "Cyber Twin Dragon's ATK is doubled: {}", s.state());
    assert_eq!(s.count(0, "grave", POWER_BOND), 1, "Power Bond is its own card, by its own Konami id: {}", s.state());
    clean(&s);
}

const MULTIPLY: u16 = 4805;
const CLONING: u16 = 5574;
const KURIBOH: u16 = 4064;

/// Multiply (4805) makes a token with the code of a monster of 500 ATK or less: to the engine it is that card, by its Konami id,
/// and the client's view (the translator shows a token with a Konami id as the client's card of that id) stays the engine's.
#[test]
fn multiply_copies_a_monster_as_the_monster_itself() {
    let deck: Vec<u16> = [WARWOLF; 33].iter().copied().chain([MULTIPLY, KURIBOH, MULTIPLY, KURIBOH, MULTIPLY, KURIBOH, KURIBOH]).collect();
    let Some(mut s) = start(true, [(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(KURIBOH), Activate(MULTIPLY)]).unwrap();
    let kuribohs = s.state()[0]["mzone"].as_array().map_or(0, |z| z.iter().filter(|c| c["cid"] == KURIBOH).count());
    assert_eq!(kuribohs, 2, "the Kuriboh and its copy: {}", s.state());
    clean(&s);
}

/// Cloning (5574), activated when the opponent summons: a token with the summoned monster's code, shown as that card.
#[test]
fn cloning_copies_the_summoned_monster_as_that_monster() {
    let deck0: Vec<u16> = [WARWOLF, CLONING].iter().copied().cycle().take(40).collect();
    let Some(mut s) = start(true, [(&deck0, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run_both([&[SetSpell(CLONING), Chain(CLONING)], &[Summon(WARWOLF)]]).unwrap();
    assert!(s.monster(0, WARWOLF).is_some(), "the copy of the opponent's Warwolf is on Cloning's controller's side: {}", s.state());
    assert_eq!(s.count(0, "grave", CLONING), 1, "{}", s.state());
    clean(&s);
}

// Card profile categories (#60, `PROFILE_COVERS` in tools/engine/anime-map.py): one card of each released category, played
// through the real core with the profile's changed property mattering. `tests/card_profile.rs` holds each category's profile
// row and deck placement; the self-play duel check (`tests/duel_check.rs`) holds the field to the profile.

const SANGAN: u16 = 4054;
const ZOMBIE_WARRIOR: u16 = 4036;
const WALL_OF_IVY: u16 = 7872;
const FIENDS_SANCTUARY: u16 = 5632;
const BOOST_WARRIOR: u16 = 9234;
const ARMORY_ARM: u16 = 7987;
const RAPID_WARRIOR: u16 = 8861;
const GAIA_KNIGHT: u16 = 7697;
const GRANADORA: u16 = 5695;
const UMIIRUKA: u16 = 4933;
const CHIRON: u16 = 5810;
const SOGEN: u16 = 4339;
const PUMPKING: u16 = 4105;
const POLYMERIZATION: u16 = 4837;
const PERFECT_MACHINE_KING: u16 = 5804;
const MACHINE_KING: u16 = 4413;
const ROBOTIC_KNIGHT: u16 = 5027;
const CHACU_CHALLHUA: u16 = 8308;
const MIND_CONTROL: u16 = 4817;

/// Both versions of a scenario: the anime one and the game's, None before the data is fetched.
fn both<T>(play: impl Fn(bool) -> Option<T>) -> Option<(T, T)> {
    Some((play(true)?, play(false)?))
}

/// flags: Wall of Ivy (7872) is a Flip Effect Monster in the game, whose token comes when it is flipped; the anime card has
/// no Flip type, its token comes whenever it is attacked, face-up too. The client shows no Flip type anywhere, so its row
/// is the same in both versions (card_profile.rs); what matters is the engine's card.
fn monsters_after_an_attack_on_wall_of_ivy(anime: bool) -> Option<usize> {
    let deck: Vec<u16> = [WALL_OF_IVY; 40].to_vec();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    s.run_both([&[Summon(WALL_OF_IVY)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    Some(s.state()[1]["mzone"].as_array().map_or(0, |z| z.iter().filter(|c| c["cid"].as_u64().is_some()).count()))
}

#[test]
fn flags_wall_of_ivy_makes_its_token_when_attacked_face_up() {
    let Some((anime, game)) = both(monsters_after_an_attack_on_wall_of_ivy) else { return };
    assert_eq!(anime, 2, "the attacker and Wall of Ivy's token");
    assert_eq!(game, 1, "the game's Wall of Ivy is a Flip effect: nothing to flip");
}

/// subtype: Fiend's Sanctuary (5632) is a Normal Spell in the game and a Quick-Play Spell in the anime: Set, it is
/// activated on the opponent's turn.
fn fiends_sanctuary_on_the_opponents_turn(anime: bool) -> Option<bool> {
    let deck: Vec<u16> = [WARWOLF, FIENDS_SANCTUARY].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    // The Normal Spell never gets the chance, and the plan runs out of turns.
    let played = s.run_both([&[SetSpell(FIENDS_SANCTUARY), Chain(FIENDS_SANCTUARY)], &[Summon(WARWOLF)]]);
    if played.is_ok() {
        clean(&s);
    }
    Some(played.is_ok() && s.count(0, "grave", FIENDS_SANCTUARY) == 1)
}

#[test]
fn subtype_fiends_sanctuary_is_activated_on_the_opponents_turn() {
    let Some((anime, game)) = both(fiends_sanctuary_on_the_opponents_turn) else { return };
    assert!(anime, "a Quick-Play Spell: activated, so in the Graveyard");
    assert!(!game, "the game's Normal Spell cannot be activated on the opponent's turn");
}

/// type: Boost Warrior (9234) is a plain Effect Monster in the game and a Tuner in the anime: with Sangan (Level 3) it
/// Synchro Summons Armory Arm (Level 4, a Tuner and one other monster).
fn armory_arm_from_boost_warrior(anime: bool) -> Option<bool> {
    let deck: Vec<u16> = [BOOST_WARRIOR, SANGAN].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[ARMORY_ARM]), (&[WARWOLF; 40], &[])])?;
    let played = s.run(&[Summon(BOOST_WARRIOR), EndTurn, Summon(SANGAN), SpSummon(ARMORY_ARM)]);
    if played.is_ok() {
        clean(&s);
    }
    Some(played.is_ok() && s.monster(0, ARMORY_ARM).is_some())
}

#[test]
fn type_boost_warrior_is_a_tuner() {
    let Some((anime, game)) = both(armory_arm_from_boost_warrior) else { return };
    assert!(anime, "Boost Warrior (Level 1 Tuner) and Sangan (Level 3) make Armory Arm (Level 4)");
    assert!(!game, "the game's Boost Warrior is not a Tuner");
}

/// deck: Perfect Machine King (5804) is a Main Deck monster in the game and a Fusion Monster in the anime: Fusion Summoned
/// from the Extra Deck with Polymerization (its DEF is 2200 in the anime, 1500 in the game: the def category).
fn fusion_with_polymerization(anime: bool, hand: &[u16], fusion: u16) -> Option<Scenario> {
    let deck: Vec<u16> = [WARWOLF; 35].iter().copied().chain(hand.iter().copied()).chain([POLYMERIZATION]).collect();
    let mut s = start(anime, [(&deck, &[fusion]), (&[WARWOLF; 40], &[])])?;
    s.run(&[Activate(POLYMERIZATION)]).unwrap();
    Some(s)
}

#[test]
fn deck_perfect_machine_king_is_an_extra_deck_fusion() {
    let Some(s) = fusion_with_polymerization(true, &[MACHINE_KING, ROBOTIC_KNIGHT, WARWOLF, WARWOLF], PERFECT_MACHINE_KING) else { return };
    let king = s.monster(0, PERFECT_MACHINE_KING).unwrap_or_else(|| panic!("Fusion Summoned: {}", s.state()));
    assert_eq!(king["base_defense"], 2200, "the anime card's DEF");
    clean(&s);
}

/// def: Earthbound Immortal Chacu Challhua (8308) has 1600 DEF in the anime (2400 in the game); Tribute Summoned over two
/// Warwolves.
#[test]
fn def_chacu_challhua_has_the_anime_versions_defense() {
    let deck: Vec<u16> = [WARWOLF, CHACU_CHALLHUA].iter().copied().cycle().take(40).collect();
    let Some(mut s) = start(true, [(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(WARWOLF), EndTurn, Summon(WARWOLF), EndTurn, Summon(CHACU_CHALLHUA)]).unwrap();
    let chacu = s.monster(0, CHACU_CHALLHUA).unwrap_or_else(|| panic!("Tribute Summoned: {}", s.state()));
    assert_eq!((chacu["base_attack"].as_i64(), chacu["base_defense"].as_i64()), (Some(2900), Some(1600)));
    clean(&s);
}

/// level: Rapid Warrior (8861) is Level 3 in the anime (Level 4 in the game): with Junk Synchron (Level 3) it makes the
/// Level 6 Gaia Knight, the Force of Earth; at Level 4 the sum is 7.
fn gaia_knight_from_rapid_warrior(anime: bool) -> Option<bool> {
    let deck: Vec<u16> = [RAPID_WARRIOR, JUNK_SYNCHRON].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[GAIA_KNIGHT]), (&[WARWOLF; 40], &[])])?;
    let played = s.run(&[Summon(RAPID_WARRIOR), EndTurn, Summon(JUNK_SYNCHRON), SpSummon(GAIA_KNIGHT)]);
    if played.is_ok() {
        clean(&s);
    }
    Some(played.is_ok() && s.monster(0, GAIA_KNIGHT).is_some())
}

#[test]
fn level_rapid_warrior_makes_the_synchro_the_anime_level_allows() {
    let Some((anime, game)) = both(gaia_knight_from_rapid_warrior) else { return };
    assert!(anime, "Level 3 + Level 3 = Level 6");
    assert!(!game, "Level 4 + Level 3 = 7: no Synchro Monster of that Level");
}

/// attribute: Granadora (5695) is DARK in the anime (WATER in the game), so Umiiruka's +500 ATK for WATER monsters is
/// the game's only.
fn granadora_under_umiiruka(anime: bool) -> Option<Option<i64>> {
    let deck: Vec<u16> = [GRANADORA, UMIIRUKA].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    s.run(&[Summon(GRANADORA), Activate(UMIIRUKA)]).unwrap();
    clean(&s);
    Some(s.attack_of(0, GRANADORA))
}

#[test]
fn attribute_granadora_is_dark_so_umiiruka_does_not_raise_it() {
    let Some((anime, game)) = both(granadora_under_umiiruka) else { return };
    assert_eq!((anime, game), (Some(1900), Some(1900 + 500)));
}

/// race: Chiron the Mage (5810) is a Beast in the anime (a Beast-Warrior in the game): Sogen's +200 for Warrior and
/// Beast-Warrior monsters is the game's only.
fn chiron_under_sogen(anime: bool) -> Option<Option<i64>> {
    let deck: Vec<u16> = [CHIRON, SOGEN].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    s.run(&[Summon(CHIRON), Activate(SOGEN)]).unwrap();
    clean(&s);
    Some(s.attack_of(0, CHIRON))
}

#[test]
fn race_chiron_the_mage_is_a_beast_so_sogen_does_not_raise_it() {
    let Some((anime, game)) = both(chiron_under_sogen) else { return };
    assert_eq!((anime, game), (Some(1800), Some(1800 + 200)));
}

/// atk: Pumpking the King of Ghosts (4105) has 1500 ATK in the anime (1800 in the game): Tribute Summoned over a Warwolf,
/// and then the opponent's Warwolf (2000) attacks it, which costs the difference.
fn pumpking_and_the_damage_it_takes(anime: bool) -> Option<(Option<i64>, i64)> {
    let deck: Vec<u16> = [WARWOLF, PUMPKING].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    s.run(&[Summon(WARWOLF), EndTurn, Summon(PUMPKING)]).unwrap();
    let base = s.monster(0, PUMPKING).and_then(|m| m["base_attack"].as_i64());
    s.run_both([&[], &[Summon(WARWOLF), EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    Some((base, s.lp(0)))
}

#[test]
fn atk_pumpking_has_the_anime_versions_attack() {
    let Some((anime, game)) = both(pumpking_and_the_damage_it_takes) else { return };
    assert_eq!(anime.0, Some(1500));
    assert_eq!(game.0, Some(1800));
    assert!(anime.1 < game.1, "the weaker Pumpking costs more: {anime:?} against {game:?}");
}

/// kind: Mind Control (4817) is a Spell in the game and a Trap in the anime: Set, it is activated on the opponent's turn
/// (its Warwolf changes sides until the end of the next turn).
fn mind_control_on_the_opponents_summon(anime: bool) -> Option<bool> {
    let deck: Vec<u16> = [WARWOLF, MIND_CONTROL].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    let played = s.run_both([&[SetSpell(MIND_CONTROL), Chain(MIND_CONTROL)], &[Summon(WARWOLF)]]);
    Some(played.is_ok() && s.monster(0, WARWOLF).is_some())
}

#[test]
fn kind_mind_control_is_a_trap() {
    let Some((anime, game)) = both(mind_control_on_the_opponents_summon) else { return };
    assert!(anime, "a Trap: activated on the opponent's turn, it took the opponent's Warwolf");
    assert!(!game, "the game's Spell cannot be activated on the opponent's turn");
}

/// A Zombie Warrior (4036) the game files as a Fusion Monster is the anime's Normal Monster: an ordinary Main Deck card
/// that is Normal Summoned (type and deck, released together).
#[test]
fn type_and_deck_zombie_warrior_is_a_main_deck_normal_monster() {
    let deck: Vec<u16> = [ZOMBIE_WARRIOR; 40].to_vec();
    let Some(mut s) = start(true, [(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[Summon(ZOMBIE_WARRIOR)]).unwrap();
    assert!(s.monster(0, ZOMBIE_WARRIOR).is_some(), "{}", s.state());
    clean(&s);
}

const THEINEN: u16 = 6089;
const DARK_HOLE: u16 = 4342;

/// Both Sphinxes on the field (summoned from the Deck by Pyramid of Light), then Dark Hole is activated and the duel stops at
/// the answer to it: Theinen the Great Sphinx, in the Deck (the bottom card, never drawn), may Special Summon itself
/// (EVENT_LEAVE_FIELD, range hand or Deck, 1000 LP).
fn theinen_duel() -> Option<Scenario> {
    let hand = [PYRAMID_OF_LIGHT, DARK_HOLE, WARWOLF, WARWOLF, WARWOLF];
    let mut deck: Vec<u16> = vec![WARWOLF; 35];
    deck.extend(hand);
    deck[0] = THEINEN;
    deck[1] = ANDRO_SPHINX;
    deck[2] = SPHINX_TELEIA;
    let mut s = start(true, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    s.run(&[SetSpell(PYRAMID_OF_LIGHT), EndTurn, Activate(PYRAMID_OF_LIGHT), SpSummon(ANDRO_SPHINX), SpSummon(SPHINX_TELEIA)]).unwrap();
    let i = s.prompt()["activates"].as_array().unwrap().iter().position(|c| c["cid"] == DARK_HOLE).expect("Dark Hole is offered");
    s.respond(json!({"type": 1, "action": idle::ACTIVATE, "index": i})).unwrap();
    Some(s)
}

/// Passes until the planned side is asked to chain a card in the Deck (the engine's MSG_SELECT_CHAIN with a card at location 1).
fn until_the_deck_trigger(s: &mut Scenario) -> Option<serde_json::Value> {
    for _ in 0..40 {
        let p = s.prompt().clone();
        if p["type"] == msg::SELECT_CHAIN && p["player"] == 0 && p["selects"].as_array().is_some_and(|v| !v.is_empty()) {
            return Some(p);
        }
        if p["type"] == msg::SELECT_IDLECMD && p["player"] == 0 {
            return None;
        }
        s.pass().unwrap();
    }
    panic!("no end: {}", s.context());
}

/// What Theinen's trigger is asked as, in the real engine with the anime versions on: from the Deck, after both Sphinxes (Andro
/// Sphinx and Sphinx Teleia, summoned from the Deck by Pyramid of Light) were destroyed together by Dark Hole.
#[test]
fn theinen_in_the_deck_is_offered_after_both_sphinxes_are_destroyed() {
    let Some(mut s) = theinen_duel() else { return };
    let p = until_the_deck_trigger(&mut s).expect("Theinen's trigger is offered");
    let selects = p["selects"].as_array().unwrap();
    assert_eq!(selects.len(), 1, "{p}");
    assert_eq!((selects[0]["cid"].as_u64(), selects[0]["code"].as_u64(), selects[0]["location"].as_u64()), (Some(6089), Some(513000030), Some(1)), "the anime card, in the Deck: {p}");
    assert_eq!(p["forced"], false, "an optional trigger");
    clean(&s);
}

#[test]
fn theinen_is_not_offered_when_only_one_sphinx_is_destroyed() {
    let hand = [PYRAMID_OF_LIGHT, DARK_HOLE, WARWOLF, WARWOLF, WARWOLF];
    let mut deck: Vec<u16> = vec![WARWOLF; 35];
    deck.extend(hand);
    deck[0] = THEINEN;
    deck[1] = ANDRO_SPHINX;
    let Some(mut s) = start(true, [(&deck, &[]), (&[WARWOLF; 40], &[])]) else { return };
    s.run(&[SetSpell(PYRAMID_OF_LIGHT), EndTurn, Activate(PYRAMID_OF_LIGHT), SpSummon(ANDRO_SPHINX)]).unwrap();
    let i = s.prompt()["activates"].as_array().unwrap().iter().position(|c| c["cid"] == DARK_HOLE).expect("Dark Hole is offered");
    s.respond(json!({"type": 1, "action": idle::ACTIVATE, "index": i})).unwrap();
    assert!(until_the_deck_trigger(&mut s).is_none(), "one Sphinx alone does not trigger it: {}", s.context());
    assert_eq!(s.count(0, "grave", ANDRO_SPHINX), 1);
    assert!(s.monster(0, THEINEN).is_none());
    clean(&s);
}

/// Accepting: 1000 LP are paid, the engine reveals Theinen to the opponent (the script's own `ConfirmCards`), and it lands on the
/// field face-up in the position its player picks (Attack by default, Defense when chosen), out of the Deck. The second trigger
/// (its ATK, 1000 LP more, the Graveyard's monsters' ATK: Andro Sphinx 3000 and Sphinx Teleia 2500) is asked and takes both answers.
fn accept_theinen(position: Option<u64>, then_atk: bool) -> Option<Scenario> {
    let mut s = theinen_duel()?;
    s.prefer_position(position);
    let p = until_the_deck_trigger(&mut s).expect("offered");
    let before = s.state()[0]["deck"].as_u64().unwrap();
    s.respond(Response::Chain(Some(0)).json()).unwrap();
    assert!(
        s.messages().iter().any(|m| m["type"] == msg::CONFIRM_CARDS && m["player"] == 1 && m["cards"][0]["cid"] == 6089),
        "the opponent is shown Theinen: {:?}\n{p}",
        s.messages()
    );
    assert_eq!(s.lp(0), 7000, "1000 LP paid for the Special Summon: {}", s.context());
    let mut asked_atk = false;
    for _ in 0..30 {
        let p = s.prompt().clone();
        if p["type"] == msg::SELECT_EFFECTYN && p["player"] == 0 {
            asked_atk = true;
            s.respond(Response::yes(resp::EFFECTYN, then_atk).json()).unwrap();
        } else if p["type"] == msg::SELECT_IDLECMD && p["player"] == 0 {
            break;
        } else {
            s.pass().unwrap();
        }
    }
    assert!(asked_atk, "the ATK trigger was asked: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_u64().unwrap(), before - 1, "Theinen left the Deck");
    Some(s)
}

#[test]
fn theinen_accepted_lands_face_up_and_its_atk_trigger_follows() {
    let Some(s) = accept_theinen(None, true) else { return };
    assert_eq!(s.position_of(0, THEINEN), Some(pos::FACEUP_ATTACK), "{}", s.context());
    assert_eq!(s.lp(0), 6000, "1000 for each trigger");
    assert_eq!(s.attack_of(0, THEINEN), Some(3500 + 3000 + 2500), "the Graveyard's monsters' ATK (Dark Hole has none): {}", s.context());
    assert_eq!(s.count(0, "grave", ANDRO_SPHINX) + s.count(0, "grave", SPHINX_TELEIA), 2);
    clean(&s);
}

#[test]
fn theinen_accepted_without_the_atk_trigger_pays_only_the_summon() {
    let Some(s) = accept_theinen(None, false) else { return };
    assert_eq!((s.lp(0), s.attack_of(0, THEINEN)), (7000, Some(3500)), "{}", s.context());
    clean(&s);
}

#[test]
fn theinen_can_be_summoned_in_defense_position() {
    let Some(s) = accept_theinen(Some(pos::FACEUP_DEFENSE), false) else { return };
    assert_eq!(s.position_of(0, THEINEN), Some(pos::FACEUP_DEFENSE), "{}", s.context());
    clean(&s);
}

#[test]
fn theinen_declined_stays_in_the_deck_and_costs_nothing() {
    let Some(mut s) = theinen_duel() else { return };
    let before = s.state()[0]["deck"].as_u64().unwrap();
    until_the_deck_trigger(&mut s).expect("offered");
    s.respond(Response::Chain(None).json()).unwrap();
    assert!(until_the_deck_trigger(&mut s).is_none(), "it is not asked again: {}", s.context());
    assert_eq!((s.lp(0), s.state()[0]["deck"].as_u64().unwrap()), (8000, before), "{}", s.context());
    assert!(s.monster(0, THEINEN).is_none());
    assert!(s.messages().iter().all(|m| m["type"] != msg::CONFIRM_CARDS), "nothing was revealed");
    clean(&s);
}

const CLEAR_VICE_DRAGON: u16 = 8538;
const BLACKWING_AURORA: u16 = 9093;
const GALE: u16 = 8011; // Blackwing - Gale the Whirlwind: Level 3 Tuner
const ARMOR_MASTER: u16 = 8036; // Blackwing Armor Master: Level 7 Synchro, 2500 ATK

/// The "?" ATK cards keep the game's "?" (-2) in the anime record: the core reports it as the card's ATK while it is in the hand
/// and plays it as 0 on the field (a base ATK below 0 is 0 there), so each script's own effects decide the rest.
fn atk_in_hand(s: &Scenario, cid: u16) -> Option<i64> {
    s.state()[0]["hand"].as_array()?.iter().find(|c| c["cid"] == cid)?["attack"].as_i64().map(|a| i64::from(a as u32 as i32))
}

/// Clear Vice Dragon (Level 8, two Tributes): the anime card's ATK is the "?" the game shows, 0 in play, and in the Damage Step it
/// has twice the ATK of the monster it fights: it destroys a Warwolf (2000) as a 4000 ATK monster, and the opponent takes the 2000.
#[test]
fn clear_vice_dragon_is_a_question_mark_in_hand_and_doubles_the_monster_it_fights() {
    let hand = [CLEAR_VICE_DRAGON, WARWOLF, WARWOLF, WARWOLF, WARWOLF];
    let Some(mut s) = start(true, [(&[&[WARWOLF; 35][..], &hand].concat(), &[]), (&[WARWOLF; 40], &[])]) else { return };
    assert_eq!(atk_in_hand(&s, CLEAR_VICE_DRAGON), Some(-2), "the record keeps the game's ?: {}", s.state());
    s.run_both([&[Summon(WARWOLF), Summon(WARWOLF), Summon(CLEAR_VICE_DRAGON)], &[Summon(WARWOLF)]]).unwrap();
    assert_eq!(s.attack_of(0, CLEAR_VICE_DRAGON), Some(0), "a ? plays as 0 on the field: {}", s.context());
    s.run_both([&[Attack(CLEAR_VICE_DRAGON)], &[]]).unwrap();
    assert!(s.monster(1, WARWOLF).is_none(), "the target is destroyed by 4000 ATK: {}", s.context());
    assert_eq!((s.lp(1), s.monster(0, CLEAR_VICE_DRAGON).is_some()), (8000 - 2000, true), "{}", s.context());
    clean(&s);
}

/// Blackwing - Aurora the Northern Lights: Special Summoned from the hand by banishing a Blackwing Tuner and a non-Tuner (Gale and a
/// Warwolf: Level 3 + 4 = 7), it is 0 ATK in play; its effect banishes the Level 7 Blackwing Synchro Monster and its base ATK becomes
/// that monster's (Armor Master, 2500): the record's "?" does not stand in its way.
#[test]
fn aurora_is_a_question_mark_in_hand_and_takes_the_banished_synchros_atk() {
    let hand = [BLACKWING_AURORA, GALE, WARWOLF, WARWOLF, WARWOLF];
    let Some(mut s) = start(true, [(&[&[WARWOLF; 35][..], &hand].concat(), &[ARMOR_MASTER]), (&[WARWOLF; 40], &[])]) else { return };
    assert_eq!(atk_in_hand(&s, BLACKWING_AURORA), Some(-2), "{}", s.state());
    s.run(&[Summon(GALE), Summon(WARWOLF), SpSummon(BLACKWING_AURORA)]).unwrap();
    assert_eq!(s.attack_of(0, BLACKWING_AURORA), Some(0), "{}", s.context());
    assert_eq!(s.level_of(0, BLACKWING_AURORA), Some(7), "the banished monsters' Levels: {}", s.context());
    s.run(&[Activate(BLACKWING_AURORA)]).unwrap();
    assert_eq!(s.attack_of(0, BLACKWING_AURORA), Some(2500), "the banished Synchro's ATK: {}", s.context());
    assert_eq!(s.count(0, "removed", ARMOR_MASTER), 1);
    clean(&s);
}

const CRUSH_CARD_VIRUS: u16 = 4667;

/// Crush Card Virus (4667): ProjectIgnis' anime card (513000009) targets a DARK monster with 1000 or less ATK, and when
/// that monster is destroyed destroys every monster with 1500 or more ATK the opponent controls and holds, and every one
/// in their Deck; the game's card Tributes the monster and lets the opponent destroy at most 3 from their Deck. Set
/// beside Sangan, chained to Warwolf's attack on it. Returns the Warwolves in the opponent's Graveyard.
fn warwolves_destroyed_by_crush_card_virus(anime: bool) -> Option<usize> {
    let deck: Vec<u16> = [SANGAN, CRUSH_CARD_VIRUS].iter().copied().cycle().take(40).collect();
    let mut s = start(anime, [(&deck, &[]), (&[WARWOLF; 40], &[])])?;
    s.run_both([&[Summon(SANGAN), SetSpell(CRUSH_CARD_VIRUS), EndTurn, ChainOnAttack(CRUSH_CARD_VIRUS)], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    Some(s.count(1, "grave", WARWOLF))
}

#[test]
fn anime_crush_card_virus_destroys_every_strong_monster_in_the_deck() {
    let (Some(anime), Some(game)) = (warwolves_destroyed_by_crush_card_virus(true), warwolves_destroyed_by_crush_card_virus(false)) else { return };
    eprintln!("anime {anime} game {game}");
    assert!(anime >= 30, "the anime card destroys every Warwolf on the field, in the hand and in the Deck: {anime}");
    assert!(game < 10, "the game's card destroys only those it sees: {game}");
}
