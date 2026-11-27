//! Anime-only cards, group 28 (rows 6119-6150 of engine/added/cards.tsv): Hermann's, Max's, Roman Goodwin's, Syd Barlow's,
//! Diablo's and Don Piero's cards. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap and every Quick-Play Spell answering the
//! opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! Covered: the Token (Loan Token, made by Nefarious Trader), the cards the per-card fuzz flagged (Revival Knight, Smoke
//! Mirror, Illegal Business Practices, Insect Costume, Fool's Dice: the fuzz's random decks never hold the cards each needs
//! at once: a Fiend Knight destroyed before an End Phase, three monsters of one name in the Graveyard, a monster whose
//! effect inflicts damage, an Insect in the hand and one on the field, a Level 2 Normal Monster in the hand), the cards
//! `--find` noted (Dark the Hidden Knight and Fool's Dice: their costs are tried with nothing to choose; Destiny Activator,
//! Offerings to the Bound Deity and Nefarious Trader at ∞ LP), the signature cards (Plunder Decoy, Release & Reverse,
//! Lock Dragon, Psychic Cyclone) and a seeded sample of the rest (Ally of Justice Tractor, Anti Emptiness, Assault Wheel,
//! Cannon Wheel, Imperial Register, Power Throw, Soul Anchor).
//! Nefarious Trader has an override (`engine/overrides/c511000969.lua`: ProjectIgnis' script leaves out the last sentence of
//! the text, "Your opponent can send 1 card from their hand to the Graveyard to destroy 1 "Loan Token" they control", so each
//! Loan Token it makes gets that effect). Dark the Hidden Knight's cost needed no override: Tributing itself while it is the
//! Defense Position monster attacked leaves its effect nothing to change but raises no Lua error (the attack is replayed).
//! The Decks are shuffled, so a card an effect fetches from the Deck is given eight times at the bottom (never all in the hand).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{battle, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const BLUE_EYES: u16 = 4007; // LIGHT, 3000/2500
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const RAIGEKI: u16 = 4343;
const MST: u16 = 4909; // Mystical Space Typhoon
const MONSTER_REBORN: u16 = 4842;
const SOLEMN: u16 = 4861; // Counter Trap: pay half your LP, negate a Spell/Trap activation
const MIRROR_FORCE: u16 = 4887;
const SHADOW_SPELL: u16 = 4675; // Continuous Trap: -700 ATK
const SKULL_KNIGHT: u16 = 5306; // Skull Knight #2: a Fiend "Knight"
const BLADE_KNIGHT: u16 = 5488; // a Warrior "Knight"
const CANNON_SOLDIER: u16 = 4518; // Tribute 1 monster: 500 damage
const PETIT_MOTH: u16 = 4284; // Insect, 300/200 vanilla
const GIANT_FLEA: u16 = 4061; // Insect, 1500/1200 vanilla
const KILLER_NEEDLE: u16 = 4059; // Insect, 1200/1000
const GAIA_KNIGHT: u16 = 7697; // Level 6 Synchro: 1 Tuner + 1 or more non-Tuners
const GARADHOLG: u16 = 7541; // Ally of Justice Garadholg: a Level 4 "Ally of Justice" in the Main Deck
const RUDRA: u16 = 7542; // Ally of Justice Rudra: Level 5

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
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

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

// ---------------------------------------------------------------------------------------------------------------
// Revival Knight
// ---------------------------------------------------------------------------------------------------------------

/// Revival Knight: "During the End Phase of the turn a Fiend-Type "Knight" monster you control was destroyed: Special Summon
/// that monster in face-up Defense Position." Dark Hole destroys Skull Knight #2 (a Fiend Knight) in turn 1: in that End
/// Phase it comes back from the Graveyard in face-up Defense Position, and Revival Knight is in the Graveyard. The fuzz
/// never had a Fiend Knight destroyed in the turn before an End Phase with the Trap Set.
#[test]
fn revival_knight_brings_a_destroyed_fiend_knight_back_in_defense_at_the_end_phase() {
    let revival = card("Revival Knight");
    let Some(mut s) = field(&[], &[set(0, revival), mon(0, SKULL_KNIGHT), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE), EndTurn, Chain(revival)]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(0, "grave", revival), 1, "Revival Knight was activated: {}", s.context());
    assert_eq!(s.count(0, "mzone", SKULL_KNIGHT), 1, "the Fiend Knight is back: {}", s.context());
    assert_eq!(s.position_of(0, SKULL_KNIGHT), Some(4), "face-up Defense: {}", s.context());
    assert_eq!(s.count(0, "grave", SKULL_KNIGHT), 0, "{}", s.context());
}

/// Revival Knight only revives a Fiend "Knight": Blade Knight (a Warrior) destroyed by the same Dark Hole stays in the
/// Graveyard (the Trap is never offered), and so does a Fiend Knight that was not destroyed that turn.
#[test]
fn revival_knight_ignores_a_knight_that_is_not_a_fiend() {
    let revival = card("Revival Knight");
    let Some(mut s) = field(&[], &[set(0, revival), mon(0, BLADE_KNIGHT), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(DARK_HOLE), EndTurn, Chain(revival)]).expect_err("a Warrior is no Fiend");
    assert!(err.contains("never became possible"), "{err}");
    let Some(mut s) = field(&[], &[set(0, revival), grave(0, SKULL_KNIGHT)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[EndTurn, Chain(revival)]).expect_err("it was not destroyed this turn");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Smoke Mirror
// ---------------------------------------------------------------------------------------------------------------

/// Smoke Mirror: "While there are 3 or more monsters with the same name in your Graveyard: Target 1 face-up monster you
/// control; It gains DEF equal to the ATK of 1 face-up monster your opponent controls, until the End Phase." With three
/// Feral Imps in the Graveyard Thunder Kid (DEF 600) gains Warwolf's 2000 (DEF 2600); with only two it cannot be activated.
#[test]
fn smoke_mirror_gives_the_def_of_an_opponents_atk_with_three_of_a_name_in_the_graveyard() {
    let mirror = card("Smoke Mirror");
    let spawns = [set(0, mirror), mon(0, THUNDER_KID), mon(1, WARWOLF), grave(0, FILLER), grave(0, FILLER), grave(0, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(mirror)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", mirror), 1, "the Trap was used: {}", s.context());
    let t = stats(&s, 0, THUNDER_KID);
    assert_eq!((t.0, t.1), (Some(700), Some(600 + 2000)), "DEF gained, ATK not: {}", s.context());
    // Two of a name are not enough (a different third monster in the Graveyard does not count): at turn 3's first prompt
    // the Trap is not among the cards that can be activated, where with three Feral Imps it is.
    for (imps, can) in [(2, false), (3, true)] {
        let mut spawns = vec![set(0, mirror), mon(0, THUNDER_KID), mon(1, WARWOLF), grave(0, DARK_BLADE)];
        spawns.extend(std::iter::repeat_n(grave(0, FILLER), imps));
        let Some(mut s) = duel(7, 8000, [WARWOLF, WARWOLF], &[], &spawns) else { return };
        s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
        assert_eq!(offered(&s, "activates").contains(&mirror.into()), can, "{imps} Feral Imps: {}", s.context());
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Illegal Business Practices
// ---------------------------------------------------------------------------------------------------------------

/// Illegal Business Practices: "Once per turn: you can double the effect damage inflicted to your opponent by the equipped
/// monster's effect." Equipped to Cannon Soldier ("You can Tribute 1 monster; inflict 500 damage"), answering its effect
/// doubles the damage: 1000. Without the answer it stays 500. Feral Imp is the Tribute.
#[test]
fn illegal_business_practices_doubles_the_effect_damage_of_the_equipped_monster() {
    let illegal = card("Illegal Business Practices");
    let spawns = [hand(0, illegal), mon(0, CANNON_SOLDIER), mon(0, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, CANNON_SOLDIER), (0, FILLER)]);
    s.run(&[Activate(illegal)]).unwrap();
    assert_eq!(s.count(0, "szone", illegal), 1, "equipped: {}", s.context());
    s.prefer(&[(0, FILLER)]);
    s.prefer_most(true); // the Tribute is chosen card by card
    s.run(&[Activate(CANNON_SOLDIER), Chain(illegal)]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "500 doubled: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "the Tribute: {}", s.context());
    // The control: the equip card left unused.
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, CANNON_SOLDIER), (0, FILLER)]);
    s.run(&[Activate(illegal)]).unwrap();
    s.prefer(&[(0, FILLER)]);
    s.prefer_most(true);
    s.run(&[Activate(CANNON_SOLDIER)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Insect Costume
// ---------------------------------------------------------------------------------------------------------------

/// Insect Costume: "Send 1 Insect-Type monster from your hand to the Graveyard. Add 1 Insect-Type monster from your Deck to
/// your hand. Then, destroy 1 Insect-Type monster you control." Petit Moth goes from the hand to the Graveyard, Killer
/// Needle comes from the Deck, and Giant Flea on the field is destroyed (the Needle in the hand is not).
#[test]
fn insect_costume_swaps_an_insect_from_the_hand_for_one_from_the_deck_and_destroys_one() {
    let costume = card("Insect Costume");
    // The Decks are shuffled: four Killer Needles at the bottom leave one in the Deck whatever the opening hand holds.
    let needles = [KILLER_NEEDLE; 8];
    let Some(mut s) = field(&needles, &[hand(0, costume), hand(0, PETIT_MOTH), mon(0, GIANT_FLEA)]) else { return };
    let before = s.count(0, "hand", KILLER_NEEDLE);
    s.run(&[Activate(costume)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", costume), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", PETIT_MOTH), 1, "the cost: {}", s.context());
    assert_eq!(s.count(0, "hand", KILLER_NEEDLE), before + 1, "added from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", GIANT_FLEA), 1, "destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", GIANT_FLEA), 0, "{}", s.context());
}

/// Insect Costume needs an Insect on the field to destroy: with none it is not offered, and its cost is not paid.
#[test]
fn insect_costume_needs_an_insect_to_destroy() {
    let costume = card("Insect Costume");
    let Some(mut s) = field(&[KILLER_NEEDLE; 8], &[hand(0, costume), hand(0, PETIT_MOTH), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Activate(costume)], &[EndTurn]]).expect_err("no Insect on the field");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", PETIT_MOTH), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Fool's Dice
// ---------------------------------------------------------------------------------------------------------------

/// Fool's Dice: "When a monster your opponent controls leaves the field: Discard 1 Level 2 or lower Normal Monster; Inflict
/// damage to your opponent equal to the Level of the monster that left the field x 200." Raigeki destroys Warwolf (Level 4):
/// Apprentice Knight (a Level 2 Normal Monster) is discarded and the opponent takes 800. Without such a monster in the hand
/// the Trap cannot be activated.
#[test]
fn fools_dice_discards_a_small_normal_monster_for_damage_by_the_level() {
    let [dice, apprentice] = ["Fool's Dice", "Apprentice Knight"].map(card);
    let spawns = [set(0, dice), hand(0, apprentice), hand(0, RAIGEKI), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.run(&[Activate(RAIGEKI), Chain(dice)]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(0, "grave", apprentice), 1, "discarded as the cost: {}", s.context());
    assert_eq!(s.count(0, "grave", dice), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 4 * 200, "Level 4 x 200: {}", s.context());
    // No Level 2 or lower Normal Monster in the hand: the Trap is not offered.
    let spawns = [set(0, dice), hand(0, RAIGEKI), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(RAIGEKI), Chain(dice)]).expect_err("nothing to discard");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dark the Hidden Knight (engine/overrides/c511000957.lua, if it needs one)
// ---------------------------------------------------------------------------------------------------------------

/// Dark the Hidden Knight: "This card can attack directly." Warwolf stands on the other side, and Dark (800 ATK) attacks the
/// opponent's LP all the same: 800 damage, Dark alive.
#[test]
fn dark_the_hidden_knight_attacks_directly() {
    let dark = card("Dark the Hidden Knight");
    let Some(mut s) = field(&[], &[mon(0, dark), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Attack(dark)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 800, "a direct attack: {}", s.context());
    assert_eq!(s.count(0, "mzone", dark), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Dark the Hidden Knight: "When a Defense Position monster you control is targeted for an attack: You can Tribute this card;
/// change that monster's battle position and repeat that battle." Warwolf (2000) attacks Dark Magician in Defense Position
/// (DEF 2100, no damage either way): Dark is Tributed, Dark Magician turns to Attack Position (2500) and the repeated battle
/// destroys Warwolf: 500 damage to its controller. Without Dark the attack bounces off Dark Magician's DEF.
#[test]
fn dark_the_hidden_knight_tributes_to_turn_the_attacked_monster_and_repeat_the_battle() {
    let dark = card("Dark the Hidden Knight");
    let spawns = [mon_def(0, dark), mon_def(0, DARK_MAGICIAN), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run_both([&[Chain(dark)], &[Attack(WARWOLF)]]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(0, "grave", dark), 1, "Dark was Tributed: {}", s.context());
    assert_eq!(s.position_of(0, DARK_MAGICIAN), Some(1), "face-up Attack Position: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the repeated battle: 2500 against 2000: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
    // The control: no Dark.
    let Some(mut s) = field(&[], &[mon_def(0, DARK_MAGICIAN), mon(1, WARWOLF)]) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    // Warwolf's 2000 against a DEF of 2100: it takes the 100 itself.
    assert_eq!((s.count(1, "mzone", WARWOLF), s.lp(1), s.lp(0)), (1, 8000 - 100, 8000), "{}", s.context());
}

/// Dark the Hidden Knight's cost can leave the effect nothing to change: Dark itself, in Defense Position, is the monster
/// targeted for the attack, and the Tribute takes it off the field. That must not stop on a Lua error: the duel goes on, Dark
/// is in the Graveyard and, with no monster left to attack, Warwolf's attack is replayed at the player (2000 damage).
#[test]
fn dark_the_hidden_knight_attacked_in_defense_does_not_stop_on_its_own_tribute() {
    let dark = card("Dark the Hidden Knight");
    let Some(mut s) = field(&[], &[mon_def(0, dark), mon(1, WARWOLF)]) else { return };
    s.run_both([&[Chain(dark)], &[Attack(WARWOLF)]]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(0, "grave", dark), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "the attack was replayed directly: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Destiny Activator
// ---------------------------------------------------------------------------------------------------------------

/// A duel whose team 0 Deck is forty of `card` (the Decks are shuffled, so this is how a Deck's top card is known), team 1's
/// of Feral Imps.
fn field_deck_of(card: u16, spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[card; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

/// Destiny Activator: "When you activate this card: Send the top card of your Deck to the Graveyard. When your opponent draws
/// a card(s) during their Draw Phase: Check the card your opponent draws. If it was a card of the same type of the card you
/// sent to the Graveyard (Monster, Spell or Trap), destroy this card and halve your opponent's LP." The top card is a
/// monster (Feral Imp) and the opponent draws a Feral Imp: this card is destroyed and the opponent's 8000 LP become 4000.
#[test]
fn destiny_activator_halves_the_lp_when_the_draw_is_of_the_type_sent_to_the_graveyard() {
    let activator = card("Destiny Activator");
    let Some(mut s) = field(&[], &[hand(0, activator)]) else { return };
    let before = s.state()[0]["grave"].as_array().unwrap().len();
    s.run(&[Activate(activator), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.state()[0]["grave"].as_array().unwrap().len(), before + 2, "the top card and this card: {}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), 1, "the top card was a Feral Imp: {}", s.context());
    assert_eq!(s.count(0, "grave", activator), 1, "destroyed: {}", s.context());
    assert_eq!(s.lp(1), 4000, "halved: {}", s.context());
}

/// Destiny Activator: the card sent was a Spell (Dian Keto) and the opponent draws a monster: no match, nothing is destroyed
/// or halved, and the Continuous Spell stays on the field.
#[test]
fn destiny_activator_does_nothing_when_the_draw_is_of_another_type() {
    let activator = card("Destiny Activator");
    let Some(mut s) = field_deck_of(DIAN_KETO, &[hand(0, activator)]) else { return };
    s.run(&[Activate(activator), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DIAN_KETO), 1, "the top card was Dian Keto: {}", s.context());
    assert_eq!(s.count(0, "szone", activator), 1, "still on the field: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Nefarious Trader and the Loan Token (engine/overrides/c511000969.lua)
// ---------------------------------------------------------------------------------------------------------------

const LOAN_TOKEN_RACE_ROCK: i64 = 0x100;
const LOAN_TOKEN_LIGHT: i64 = 0x10;

/// Nefarious Trader: "If your LP is lower than your opponent's: You can Special Summon this card from your hand in Attack
/// Position. If this card is destroyed by battle: Special Summon 1 "Loan Token" (Rock-Type/LIGHT/Level 1/ATK 0/DEF 0) to your
/// opponent's side of the field in Defense Position." Team 1's Dian Keto puts team 0 behind (8000 against 9000), the Trader
/// comes from the hand in Attack Position, and Warwolf destroys it by battle: the Token arrives on team 1's side (the
/// attacker's) in face-up Defense Position, with the stats of the text, and the Trader's controller took 2000.
#[test]
fn nefarious_trader_comes_when_behind_and_leaves_a_loan_token_on_the_opponents_side() {
    let [trader, token] = ["Nefarious Trader", "Loan Token"].map(card);
    let spawns = [hand(0, trader), hand(1, DIAN_KETO), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    // Level in LP: the Trader cannot come while the LP are equal.
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&trader.into()), "equal LP: {:?}", offered(&s, "special_summons"));
    s.run_both([&[EndTurn, SpSummon(trader)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 9000), "{}", s.context());
    assert_eq!(s.position_of(0, trader), Some(1), "face-up Attack Position: {}", s.context());
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", trader), 1, "destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    assert_eq!(s.count(1, "mzone", token), 1, "the Token is on the attacker's side: {}", s.context());
    assert_eq!(s.count(0, "mzone", token), 0, "{}", s.context());
    assert_eq!(s.position_of(1, token), Some(4), "face-up Defense Position: {}", s.context());
    let t = stats(&s, 1, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(0), Some(0), Some(1), LOAN_TOKEN_RACE_ROCK, LOAN_TOKEN_LIGHT), "Rock, LIGHT, Level 1, 0/0: {}", s.context());
}

/// The Loan Token "cannot be Tributed": team 1 holds Warwolf and the Token, and Dark Magician (Level 7, two Tributes) is not
/// offered to be Tribute Summoned; with a Dark Blade on the field as well the two non-Token monsters can be Tributed and it is.
#[test]
fn a_loan_token_cannot_be_tributed() {
    let [trader, token] = ["Nefarious Trader", "Loan Token"].map(card);
    for (extra, can) in [(vec![], false), (vec![mon(1, DARK_BLADE)], true)] {
        let mut spawns = vec![mon(0, trader), hand(1, DARK_MAGICIAN), mon(1, WARWOLF)];
        spawns.extend(extra);
        let Some(mut s) = field(&[], &spawns) else { return };
        keep_hands(&mut s);
        // Warwolf destroys the Trader (Attack Position, 0 ATK); the Token comes; then team 1's Main Phase 2.
        s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
        if s.count(1, "mzone", token) == 0 {
            panic!("no Token: {}", s.context());
        }
        if s.prompt()["type"] == msg::SELECT_BATTLECMD {
            s.respond(serde_json::json!({"type": 1, "action": battle::TO_M2, "index": null})).unwrap(); // Dark Blade could still attack
            s.run(&[]).unwrap();
        }
        assert_eq!(offered(&s, "summons").contains(&DARK_MAGICIAN.into()), can, "{:?}\n{}", offered(&s, "summons"), s.context());
    }
}

/// Nefarious Trader: "Your opponent can send 1 card from their hand to the Graveyard to destroy 1 "Loan Token" they control."
/// The Token's controller (team 1) activates it from the field: a Feral Imp goes from their hand to the Graveyard and the Token
/// is destroyed (it is gone: Tokens never reach the Graveyard).
#[test]
fn the_loan_token_is_destroyed_by_sending_a_card_from_the_hand() {
    let [trader, token] = ["Nefarious Trader", "Loan Token"].map(card);
    let Some(mut s) = field(&[], &[mon(0, trader), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(1, "mzone", token), 1, "{}", s.context());
    let (hand_before, grave_before) = (hand_size(&s, 1), s.state()[1]["grave"].as_array().unwrap().len());
    s.run_both([&[], &[Activate(token)]]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(1, "mzone", token), 0, "the Token was destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", token), 0, "a Token is gone, not in the Graveyard: {}", s.context());
    assert_eq!(hand_size(&s, 1), hand_before - 1, "a card was sent from the hand: {}", s.context());
    assert_eq!(s.state()[1]["grave"].as_array().unwrap().len(), grave_before + 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Offerings to the Bound Deity
// ---------------------------------------------------------------------------------------------------------------

/// The Earthbound Immortal Counters (0x94) on a team's first card of this Konami id in the Spell & Trap Zones.
fn immortal_counters(s: &Scenario, team: usize, cid: u16) -> i64 {
    let c = s.state()[team]["szone"].as_array().unwrap().iter().find(|c| c["cid"] == cid).unwrap_or_else(|| panic!("{cid} is not in team {team}'s Spell & Trap Zone: {}", s.context()));
    c["counters"]["148"].as_i64().unwrap_or(0)
}

/// Offerings to the Bound Deity: "Pay half of your LP; place 2 Earthbound Immortal Counters on 1 "Altar of the Bound Deity"
/// you control." The Altar holds none, and after the Trap 2 counters; the cost took half of 8000.
#[test]
fn offerings_to_the_bound_deity_pays_half_the_lp_for_two_counters_on_the_altar() {
    let [offerings, altar] = ["Offerings to the Bound Deity", "Altar of the Bound Deity"].map(card);
    let Some(mut s) = field(&[], &[set(0, offerings), spell(0, altar)]) else { return };
    assert_eq!(immortal_counters(&s, 0, altar), 0, "{}", s.context());
    s.run(&[Activate(offerings)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "half of 8000: {}", s.context());
    assert_eq!(immortal_counters(&s, 0, altar), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", offerings), 1, "{}", s.context());
    // Without the Altar it is not offered.
    let Some(mut s) = field(&[], &[set(0, offerings)]) else { return };
    let err = s.run_both([&[Activate(offerings)], &[EndTurn]]).expect_err("no Altar");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// At ∞ LP (docs/added-cards.md, "Infinite ATK and LP"; the duel is built as infinite_lp.rs builds it)
// ---------------------------------------------------------------------------------------------------------------

const INFINITE: i64 = 19_999_998;
const KURIBOH: u16 = 4064;
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK
const YU_JO: u16 = 5754; // both players' LP become half of their total

/// Team 0 (or team 1) at ∞ LP (Mystik Wok on the anime Geh: 8000 and ∞ is ∞), the other team at 8000, with `spawns` in place:
/// Kuriboh Decks (a monster at the top of both). The planned side's first plan step is spent: for team 1 it is its turn 2.
fn infinite(team: usize, spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(team as u32, WOK), mon(team as u32, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &field).expect("the duel starts");
    s.prefer(&[(team, GEH), (0, KURIBOH), (1, KURIBOH)]); // Wok's Tribute, and hand-size discards take Kuribohs
    let plans: [&[yo3_server::duelcheck::scenario::Action]; 2] = if team == 0 { [&[Activate(WOK)], &[]] } else { [&[], &[Activate(WOK)]] };
    s.run_both(plans).unwrap();
    assert_eq!(s.lp(team), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[(0, KURIBOH), (1, KURIBOH)]);
    Some(s)
}

/// Offerings to the Bound Deity at ∞ LP: its cost is half of ∞, which is ∞ (the LP stay ∞), and the Altar still gets its 2
/// counters.
#[test]
fn offerings_to_the_bound_deity_at_infinite_lp_leaves_infinite_lp() {
    let [offerings, altar] = ["Offerings to the Bound Deity", "Altar of the Bound Deity"].map(card);
    let Some(mut s) = infinite(0, &[set(0, offerings), spell(0, altar)]) else { return };
    s.run(&[Activate(offerings)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), INFINITE, "half of ∞ is ∞: {}", s.context());
    assert_eq!(immortal_counters(&s, 0, altar), 2, "{}", s.context());
}

/// Destiny Activator at ∞ LP: both players at ∞ (Yu-Jo Friendship), Activator Set up: the opponent draws a Kuriboh (a monster,
/// as the top card sent: a Kuriboh) in the Draw Phase, the card is destroyed (proving the halving ran), and half of ∞ is ∞.
#[test]
fn destiny_activator_halves_an_infinite_opponent_to_infinity() {
    let activator = card("Destiny Activator");
    let Some(mut s) = infinite(0, &[hand(0, YU_JO), hand(0, activator)]) else { return };
    s.run(&[Activate(YU_JO), Activate(activator), EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, INFINITE), "Yu-Jo Friendship: {}", s.context());
    assert_eq!(s.count(0, "grave", activator), 1, "destroyed by the matching draw: {}", s.context());
}

/// Nefarious Trader at ∞ LP: "If your LP is lower than your opponent's". Team 1 at ∞ and team 0 at 8000: lower, the Trader is
/// offered. Team 0 at ∞ and team 1 at 8000: higher, not offered. Both at ∞ (Yu-Jo Friendship): equal, not offered.
#[test]
fn nefarious_trader_compares_infinite_lp_with_finite_lp() {
    let trader = card("Nefarious Trader");
    let Some(mut s) = infinite(1, &[hand(0, trader)]) else { return };
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(offered(&s, "special_summons").contains(&trader.into()), "8000 is lower than ∞: {}", s.context());
    s.run(&[SpSummon(trader)]).unwrap();
    assert_eq!(s.count(0, "mzone", trader), 1, "{}", s.context());
    let Some(mut s) = infinite(0, &[hand(0, trader)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&trader.into()), "∞ is not lower than 8000: {:?}", offered(&s, "special_summons"));
    let Some(mut s) = infinite(0, &[hand(0, trader), hand(0, YU_JO)]) else { return };
    s.run(&[Activate(YU_JO)]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (INFINITE, INFINITE), "{}", s.context());
    assert!(!offered(&s, "special_summons").contains(&trader.into()), "∞ is not lower than ∞: {:?}", offered(&s, "special_summons"));
}

// ---------------------------------------------------------------------------------------------------------------
// Signature cards: Plunder Decoy, Release & Reverse, Lock Dragon, Psychic Cyclone
// ---------------------------------------------------------------------------------------------------------------

/// Plunder Decoy: "When a monster you control is targeted for an attack: Take control of a Level 4 or lower monster your
/// opponent controls, except the attacking monster." Warwolf attacks Thunder Kid; Giant Flea (Level 4) changes sides, Dark
/// Magician (Level 7) cannot be taken and stays. The attack on Thunder Kid goes on (700 against 2000: 1300 damage).
#[test]
fn plunder_decoy_takes_a_small_monster_when_a_monster_is_attacked() {
    let decoy = card("Plunder Decoy");
    let spawns = [set(0, decoy), mon(0, THUNDER_KID), mon(1, WARWOLF), mon(1, GIANT_FLEA), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[Chain(decoy)], &[Attack(WARWOLF)]]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(0, "grave", decoy), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", GIANT_FLEA), s.count(1, "mzone", GIANT_FLEA)), (1, 0), "Giant Flea changed sides: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "a Level 7 monster is not taken: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "the attacker is not taken: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "the attack went on: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}

/// Release & Reverse: "Tribute 1 monster, and take damage equal to the Tributed monster's ATK. Then, Special Summon that
/// monster to your opponent's side of the field." Warwolf (2000) is Tributed: 2000 damage to its controller, and it comes up
/// on the opponent's side (in the face-up Attack Position the CPU picks), not in the Graveyard.
#[test]
fn release_and_reverse_gives_the_opponent_the_tributed_monster_for_its_atk_in_damage() {
    let reverse = card("Release & Reverse");
    let Some(mut s) = field(&[], &[set(0, reverse), mon(0, WARWOLF)]) else { return };
    s.run(&[Activate(reverse)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", reverse), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "damage equal to its ATK: {}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", WARWOLF)), (0, 1), "Special Summoned to the opponent's side: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

/// Lock Dragon: "Must be Special Summoned (from your hand) when a Counter Trap Card you control negates the activation of an
/// opponent's card(s) or effect(s)... While this card is in face-up Defense Position, neither player can Special Summon."
/// Team 1's Dian Keto is negated by Solemn Judgment (team 0 pays half its LP) and Lock Dragon comes from the hand, in Defense
/// Position. Then team 1 cannot Monster Reborn (that Special Summon is barred); with Lock Dragon in Attack Position it can.
#[test]
fn lock_dragon_comes_after_a_counter_trap_and_stops_special_summons_in_defense() {
    let lock = card("Lock Dragon");
    for (position, can_reborn) in [(4, false), (1, true)] {
        let spawns = [hand(0, lock), set(0, SOLEMN), hand(1, DIAN_KETO), hand(1, MONSTER_REBORN), grave(1, WARWOLF)];
        let Some(mut s) = field(&[], &spawns) else { return };
        keep_hands(&mut s);
        s.prefer_position(Some(position));
        s.run_both([&[Chain(SOLEMN), Chain(lock)], &[Activate(DIAN_KETO)]]).unwrap_or_else(|e| panic!("{e}"));
        clean(&s);
        assert_eq!(s.lp(0), 4000, "Solemn Judgment's cost: {}", s.context());
        assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "negated and destroyed: {}", s.context());
        assert_eq!(s.lp(1), 8000, "Dian Keto did nothing: {}", s.context());
        assert_eq!(s.count(0, "mzone", lock), 1, "Lock Dragon came from the hand: {}", s.context());
        assert_eq!(s.position_of(0, lock), Some(position), "{}", s.context());
        // Team 1's Main Phase: Monster Reborn.
        s.run_both([&[], &[]]).unwrap();
        assert_eq!(offered(&s, "activates").contains(&MONSTER_REBORN.into()), can_reborn, "Lock Dragon in position {position}: {}", s.context());
    }
}

/// Lock Dragon cannot be Special Summoned any other way: with no Counter Trap having negated anything it is not offered.
#[test]
fn lock_dragon_cannot_be_summoned_without_a_counter_trap() {
    let lock = card("Lock Dragon");
    let Some(mut s) = field(&[], &[hand(0, lock)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&lock.into()), "{:?}", offered(&s, "special_summons"));
    assert!(!offered(&s, "summons").contains(&lock.into()) && !offered(&s, "monster_sets").contains(&lock.into()), "{}", s.context());
}

/// Psychic Cyclone: "Target 1 face-down Spell/Trap your opponent controls; declare if it is a Spell or Trap, and destroy it,
/// and if you do, if you were right, draw 1 card, otherwise, your opponent draws 1 card." Team 1's Set card is Mirror Force
/// (a Trap). Declaring Trap is right: the Trap is destroyed, I draw (my hand is as before the Spell left it, plus 1: the
/// same size), the opponent does not. Declaring Spell is wrong: the opponent draws and I do not.
#[test]
fn psychic_cyclone_draws_for_a_right_guess_and_gives_the_opponent_a_card_for_a_wrong_one() {
    let cyclone = card("Psychic Cyclone");
    for (option, right) in [(1, true), (0, false)] {
        let Some(mut s) = field(&[], &[hand(0, cyclone), set(1, MIRROR_FORCE)]) else { return };
        let (mine, theirs) = (hand_size(&s, 0), hand_size(&s, 1));
        s.choose_options(&[option]);
        s.run(&[Activate(cyclone)]).unwrap();
        clean(&s);
        assert_eq!(s.count(1, "grave", MIRROR_FORCE), 1, "destroyed either way: {}", s.context());
        assert_eq!(s.count(0, "grave", cyclone), 1, "{}", s.context());
        let (mine_now, theirs_now) = (hand_size(&s, 0), hand_size(&s, 1));
        match right {
            true => assert_eq!((mine_now, theirs_now), (mine, theirs), "I drew for the Spell I played: {}", s.context()),
            false => assert_eq!((mine_now, theirs_now), (mine - 1, theirs + 1), "the opponent drew: {}", s.context()),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The seeded sample: Ally of Justice Tractor, Anti Emptiness, Assault Wheel, Cannon Wheel, Imperial Register, Power Throw,
// Soul Anchor
// ---------------------------------------------------------------------------------------------------------------

/// Ally of Justice Tractor: "If your opponent controls a face-up LIGHT monster: you can Tribute this card; add 2 "Ally of
/// Justice" monsters from your Deck to your hand." Against Blue-Eyes (LIGHT) it is Tributed and two Allies come
/// from the Deck; against a DARK monster it cannot be used.
#[test]
fn ally_of_justice_tractor_adds_two_allies_when_the_opponent_has_a_light_monster() {
    let tractor = card("Ally of Justice Tractor");
    // The Decks are shuffled: eight Allies at the bottom leave at least three in the Deck whatever the opening hand holds.
    let allies = [GARADHOLG, RUDRA, GARADHOLG, RUDRA, GARADHOLG, RUDRA, GARADHOLG, RUDRA];
    let Some(mut s) = field(&allies, &[mon(0, tractor), mon(1, BLUE_EYES)]) else { return };
    let in_hand = |s: &Scenario| s.count(0, "hand", GARADHOLG) + s.count(0, "hand", RUDRA);
    let before = in_hand(&s);
    s.run(&[Activate(tractor)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", tractor), 1, "Tributed: {}", s.context());
    assert_eq!(in_hand(&s), before + 2, "two Allies were added: {}", s.context());
    let Some(mut s) = field(&allies, &[mon(0, tractor), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&tractor.into()), "no LIGHT monster: {}", s.context());
}

/// Anti Emptiness: "When your opponent activates a Spell/Trap Card that would destroy a Spell/Trap on the field: Negate the
/// activation, and if you do, destroy it, also, the ATK of 1 Attack Position monster you control becomes 0 until the End
/// Phase." Team 1's Mystical Space Typhoon targets the Set Anti Emptiness: negated and destroyed, and Warwolf's ATK is 0.
/// Raigeki (a card that destroys monsters only) is not answered.
#[test]
fn anti_emptiness_negates_a_card_that_destroys_a_spell_or_trap_and_zeroes_an_atk() {
    let anti = card("Anti Emptiness");
    let Some(mut s) = field(&[], &[set(0, anti), mon(0, WARWOLF), hand(1, MST)]) else { return };
    s.run_both([&[Chain(anti)], &[Activate(MST)]]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(1, "grave", MST), 1, "negated and destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", anti), 1, "Anti Emptiness itself stayed out of it: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(0), "ATK 0: {}", s.context());
    // Raigeki destroys monsters only: not answered.
    let Some(mut s) = field(&[], &[set(0, anti), mon(0, WARWOLF), hand(1, RAIGEKI)]) else { return };
    s.run_both([&[Chain(anti)], &[Activate(RAIGEKI)]]).expect_err("Anti Emptiness is not offered against Raigeki");
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Raigeki destroyed it: {}", s.context());
}

/// Assault Wheel: "If this card attacks a Defense Position monster, inflict piercing battle damage. At the end of the Battle
/// Phase, if this card attacked, its ATK becomes 0 until your next Standby Phase." Assault Wheel (2300) attacks Thunder Kid
/// in Defense (DEF 600): 1700 piercing damage, Thunder Kid destroyed; the Wheel's ATK is 0 after the Battle Phase, through
/// the opponent's turn (checked at its first prompt), and back to 2300 once its controller's Standby Phase has come.
#[test]
fn assault_wheel_pierces_and_then_loses_its_atk_until_its_next_standby_phase() {
    let wheel = card("Assault Wheel");
    let Some(mut s) = field(&[], &[mon(0, wheel), mon_def(1, THUNDER_KID)]) else { return };
    s.run_both([&[EndTurn, Attack(wheel), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1700, "piercing damage: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, wheel), Some(0), "ATK 0 after the Battle Phase: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, wheel), Some(2300), "its next Standby Phase brought it back: {}", s.context());
}

/// Cannon Wheel: "If this card is used for a Synchro Summon: inflict 500 damage to your opponent." Cannon Wheel (a Level 2
/// Tuner) and a Feral Imp (Level 4) Synchro Summon Gaia Knight, the Force of Earth (Level 6): the opponent takes 500.
#[test]
fn cannon_wheel_inflicts_500_damage_when_used_for_a_synchro_summon() {
    let wheel = card("Cannon Wheel");
    let Some(mut s) = field_extra([&[GAIA_KNIGHT], &[]], &[mon(0, wheel), mon(0, FILLER)]) else { return };
    s.run(&[SpSummon(GAIA_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GAIA_KNIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", wheel), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "{}", s.context());
}

/// Imperial Register: "Once per turn, if a monster you control loses ATK by a card effect, you can target 1 face-up monster
/// you control: it gains ATK equal to the changed amount until the End Phase." Warwolf is my only monster. Team 1's Shadow
/// Spell takes 700 from it (2000 to 1300) and the Register's answer gives the 700 back until the End Phase (2000); without
/// the Register on the field it stays 1300.
#[test]
fn imperial_register_gives_back_the_atk_a_card_effect_took() {
    let register = card("Imperial Register");
    for (with_register, atk) in [(true, 2000), (false, 1300)] {
        let mut spawns = vec![mon(0, WARWOLF), set(1, SHADOW_SPELL)];
        if with_register {
            spawns.push(set(0, register));
        }
        let Some(mut s) = field(&[], &spawns) else { return };
        let mine: &[yo3_server::duelcheck::scenario::Action] = if with_register { &[Activate(register), EndTurn, Chain(register)] } else { &[] };
        s.run_both([mine, &[EndTurn, Activate(SHADOW_SPELL)]]).unwrap_or_else(|e| panic!("{e}"));
        clean(&s);
        assert_eq!(s.count(1, "szone", SHADOW_SPELL), 1, "{}", s.context());
        assert_eq!(s.attack_of(0, WARWOLF), Some(atk), "with the Register {with_register}: {}", s.context());
    }
}

/// Power Throw: "Activate only when a monster you control with an original ATK of 1000 or lower destroys your opponent's
/// monster by battle. Tribute 1 monster you control with 1000 or less ATK to inflict damage to your opponent equal to twice
/// that monster's ATK." Thunder Kid (700) destroys Petit Moth (300) by battle: 400 damage, then Thunder Kid is Tributed for 1400.
#[test]
fn power_throw_tributes_a_weak_monster_for_twice_its_atk_in_damage() {
    let throw = card("Power Throw");
    let Some(mut s) = field(&[], &[set(0, throw), mon(0, THUNDER_KID), mon(1, PETIT_MOTH)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.prefer_most(true);
    s.run(&[EndTurn, Attack(THUNDER_KID), Chain(throw)]).unwrap_or_else(|e| panic!("{e}"));
    clean(&s);
    assert_eq!(s.count(1, "grave", PETIT_MOTH), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", throw), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "Tributed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 400 - 1400, "battle damage and twice 700: {}", s.context());
}

/// Soul Anchor: "Equip this card to a monster on the field. The equipped monster cannot be destroyed by battle. Also, it cannot
/// change its battle position." Thunder Kid is equipped, so Warwolf's attack (2000 against 700) does not destroy it (1300
/// damage to its controller), and its position is not on offer to change, where Dark Blade's is.
#[test]
fn soul_anchor_stops_battle_destruction_and_position_changes() {
    let anchor = card("Soul Anchor");
    let spawns = [set(0, anchor), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(anchor)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", anchor), 1, "equipped: {}", s.context());
    let pos = offered(&s, "pos_changes");
    assert!(!pos.contains(&THUNDER_KID.into()) && pos.contains(&DARK_BLADE.into()), "{pos:?}: {}", s.context());
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
}
