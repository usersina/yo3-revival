//! Scenario tests for the thirteenth group of the anime's printed cards (rows 4848-5205 of engine/added/cards.tsv): the
//! cards from "Slash Draw" to "T.G. Halberd Cannon". Each through the real core with both seats' client models checked
//! after every step (`duelcheck::scenario`). Texts: the printed ones (anime cards off) from the pinned official
//! database; an anime version is tested with anime cards on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The added cards of this group (client ids from the group list).
const SLASH_DRAW: u16 = 13814;
const SONIC_STUN: u16 = 14562;
const SONIC_WARRIOR: u16 = 9799;
const SOUL_BINDING_GATE: u16 = 4681;
const SOUL_CHARGE: u16 = 11159;
const SOUL_UNION: u16 = 5085;
const SPACETIME_TRANSCENDENCE: u16 = 13155;
const SPECIMEN_INSPECTION: u16 = 15371;
const SPELL_OF_PAIN: u16 = 5486;
const SPIRIT_SHIELD: u16 = 5054;
const STAND_OFF: u16 = 10955;
const STARDUST_FLASH: u16 = 11467;
const STEAM_SYNCHRON: u16 = 10310;
const STONEHENGE: u16 = 4797;
const SUBSPACE_BATTLE: u16 = 5100;
const SUMMON_DICE: u16 = 13801;
const SUMMON_STORM: u16 = 4006;
const SYNKRON_RESONATOR: u16 = 12062;
const TG_BLADE_BLASTER: u16 = 9513;
const TG_CATAPULT_DRAGON: u16 = 9490;
const TG_CYBER_MAGICIAN: u16 = 9487;
const CATASTOR: u16 = 7552; // Ally of Justice Catastor: Level 5, 1 Tuner + 1 or more non-Tuner monsters
const TG_DRILL_FISH: u16 = 14560;
const TG_GEAR_ZOMBIE: u16 = 14559;
const TG_HALBERD_CANNON: u16 = 9514;
// The game's own, and added cards of earlier groups they play with.
const KURIBOH: u16 = 4064; // Level 1 DARK Fiend, 300 ATK
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const POT_OF_GREED: u16 = 4844;
const Z_ONE: u16 = 9447;
const MIND_CONTROL: u16 = 4817; // a Spell that targets
const TG_STRIKER: u16 = 9488; // T.G. Striker: Level 2 Tuner
const DARK_RESONATOR: u16 = 7708; // a "Resonator" monster
const FOSSIL_FUSION: u16 = 15369;
const STARDUST: u16 = 7734; // Stardust Dragon, a Level 8 Synchro Monster
const OOKAZI: u16 = 4352; // "Inflict 800 damage to your opponent"
const SKULL_SERVANT: u16 = 4030; // a Level 1 Normal Zombie
const SWORDSMAN: u16 = 4405; // Swordsman from a Distant Land: a Level 1 Warrior
const TOMOZAURUS: u16 = 4111; // a Level 2 Normal Dinosaur
const URABY: u16 = 4086; // a Level 4 Normal Dinosaur
const SPROCKET: u16 = 7707; // Sinister Sprocket: a Level 1 Tuner
const AVIAN: u16 = 6310; // Elemental HERO Avian: Normal, 1000 ATK
const BURSTINATRIX: u16 = 6311; // Elemental HERO Burstinatrix: Normal
const FLAME_WINGMAN: u16 = 6344; // Avian + Burstinatrix
const SKY_DRAGON: u16 = 4430; // a Level 6 WIND
const HARPIE_LADY: u16 = 4068; // a Level 4 WIND
const XIAOLONG: u16 = 8437; // Stardust Xiaolong
const PHANTOM: u16 = 9294; // Stardust Phantom: 0 ATK
const ARMORY_ARM: u16 = 7987; // a Level 4 Synchro Monster
const FORMULA_SYNCHRON: u16 = 9118; // Level 2 Tuner Synchro Monster
const TG_RHINO: u16 = 9492; // T.G. Rush Rhino: Level 4
const TG_WARWOLF: u16 = 9491; // T.G. Warwolf: Level 3
const TG_POWER_GLADIATOR: u16 = 9512; // Level 5 Synchro Monster: 1 Tuner + 1 or more non-Tuner "T.G."

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn start_seed(seed: u64, anime: bool, main: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(main[0], &[][..]), (main[1], &[][..])];
    Some(if anime { Scenario::start_anime_field(&dir, seed, 8000, decks, spawns) } else { Scenario::start_field(&dir, seed, 8000, decks, spawns) }.unwrap())
}

fn start(anime: bool, main: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    start_seed(7, anime, main, spawns)
}

/// A duel with Kuriboh decks and `spawns` in place; anime cards off.
fn printed(spawns: &[Spawn]) -> Option<Scenario> {
    start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], spawns)
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn count_all(s: &Scenario, team: usize, place: &str) -> usize {
    s.state()[team][place].as_array().map_or(0, |cards| cards.iter().filter(|c| !c.is_null()).count())
}

/// `start_seed` with an Extra Deck for each team too.
fn start_extra(anime: bool, main: [&[u16]; 2], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(main[0], extra[0]), (main[1], extra[1])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

fn hand_of(s: &Scenario, team: usize) -> usize {
    count_all(s, team, "hand")
}

fn deck_of(s: &Scenario, team: usize) -> i64 {
    s.state()[team]["deck"].as_i64().unwrap_or(-1)
}

/// How many cards a team has in a place, whatever they are.
fn n(s: &Scenario, team: usize, place: &str) -> usize {
    count_all(s, team, place)
}

/// Slash Draw: "Discard 1 card; send cards from the top of your Deck to the GY, equal to the number of cards your opponent
/// controls, then draw 1 card and show it. ... If the card you drew was not "Slash Draw", shuffle cards from your GY into
/// your Deck, equal to the number of cards sent from your Deck to the GY by this effect." Two monsters opposite: two
/// cards milled, one drawn, two shuffled back: the Deck is one card lighter, the hand one card short of what it was
/// (Slash Draw and the discard gone, the draw in), and only the discard and Slash Draw itself are in the GY.
#[test]
fn slash_draw_mills_draws_and_shuffles_the_milled_cards_back() {
    let spawns = [at(0, Hand, SLASH_DRAW), at(1, Monster, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    let (hand, deck) = (hand_of(&s, 0), deck_of(&s, 0));
    s.run(&[Activate(SLASH_DRAW)]).unwrap();
    clean(&s);
    assert_eq!(hand_of(&s, 0), hand - 1, "{}", s.context());
    assert_eq!(deck_of(&s, 0), deck - 1, "2 milled, 1 drawn, 2 shuffled back: {}", s.context());
    assert_eq!(n(&s, 0, "grave"), 2, "the discard and Slash Draw: {}", s.context());
    assert_eq!(s.count(0, "grave", SLASH_DRAW), 1, "{}", s.context());
}

/// Slash Draw: "Then, if it was "Slash Draw", send it to the GY, and if you do, destroy as many cards on the field as
/// possible, then inflict 2000 damage to your opponent for each card destroyed and sent to the GY by this effect." A Deck
/// of nothing but Slash Draw: the draw is one. Two monsters opposite and one of ours go: 3 cards, 6000 damage.
#[test]
fn slash_draw_drawing_itself_destroys_the_field_for_2000_each() {
    let spawns = [at(0, Hand, SLASH_DRAW), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(false, [&[SLASH_DRAW; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Activate(SLASH_DRAW)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 2000, "3 cards destroyed: {}", s.context());
    assert_eq!(n(&s, 0, "mzone") + n(&s, 1, "mzone"), 0, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "{}", s.context());
}

/// Slash Draw (anime): "Send cards from the top of your Deck to the Graveyard equal to the total number of cards on the
/// field. Then, draw 1 card. If that card is "Slash Draw", send it to the Graveyard and destroy all cards on the field.
/// Inflict 1000 damage to your opponent for each card destroyed and sent to the Graveyard this way." No discard, no
/// shuffle, 1000 a card, and the card itself stays on the field, so 3 cards (not 4 counted for the milling) go for 3000.
#[test]
fn slash_draw_anime_deals_1000_a_card() {
    let spawns = [at(0, Hand, SLASH_DRAW), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[SLASH_DRAW; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Activate(SLASH_DRAW)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 5000, "3 cards destroyed: {}", s.context());
    assert_eq!(n(&s, 0, "mzone") + n(&s, 1, "mzone"), 0, "{}", s.context());
}

/// Sonic Stun: "When an opponent's monster declares an attack: Negate the attack, then you can Special Summon 1 "T.G."
/// monster, or 1 Level 4 or lower Tuner, from your hand or Deck." The opponent's Warwolf attacks our Kuriboh: nothing is destroyed,
/// and the Level 1 Tuner in the hand is on the field (our Kuriboh, the one the attack was aimed at, is untouched).
#[test]
fn sonic_stun_negates_the_attack_and_brings_a_tuner() {
    let spawns = [at(0, SpellSet, SONIC_STUN), at(0, Hand, SPROCKET), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[Summon(KURIBOH), EndTurn, ChainOnAttack(SONIC_STUN)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "mzone", SPROCKET), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", SONIC_STUN), 1, "{}", s.context());
}

/// Sonic Stun: the "T.G." monster too, not a Tuner: T.G. Rush Rhino (Level 4, not a Tuner) from the hand.
#[test]
fn sonic_stun_brings_a_tg_monster() {
    let spawns = [at(0, SpellSet, SONIC_STUN), at(0, Hand, TG_RHINO), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, TG_RHINO)]);
    s.run_both([&[Summon(KURIBOH), EndTurn, ChainOnAttack(SONIC_STUN)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", TG_RHINO), 1, "{}", s.context());
}

/// Sonic Stun (anime): "Negate the attack. Then, you can Special Summon 1 Level 4 or lower Tuner monster from your
/// hand." The "T.G." monster is not on offer in the anime: with both in the hand, the Tuner is summoned and Rush Rhino
/// stays in the hand (a Kuriboh is discarded at the end of the turn for the hand's size).
#[test]
fn sonic_stun_anime_brings_only_a_tuner_from_the_hand() {
    let spawns = [at(0, SpellSet, SONIC_STUN), at(0, Hand, TG_RHINO), at(0, Hand, SPROCKET), at(0, Monster, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[SetMonster(KURIBOH), EndTurn, ChainOnAttack(SONIC_STUN)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "mzone", SPROCKET), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", TG_RHINO), 1, "{}", s.context());
}

/// Sonic Warrior: "If this card is sent to the Graveyard: All Level 2 or lower monsters you currently control gain 500
/// ATK." Sonic Warrior (Level 2, 1000 ATK) attacks Warwolf and goes; Kuriboh (Level 1) gains 500, Warwolf of ours (Level
/// 4) does not.
#[test]
fn sonic_warrior_gives_low_level_monsters_500_atk_when_it_goes() {
    let spawns = [at(0, Monster, SONIC_WARRIOR), at(0, Monster, KURIBOH), at(0, Monster, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Attack(SONIC_WARRIOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SONIC_WARRIOR), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, KURIBOH), Some(800), "300 and 500: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "Level 4: {}", s.context());
}

/// Soul Binding Gate: "To activate this card you must control no monsters, and a Set Spell/Trap(s) you control must have
/// been destroyed by card effect this turn. During the Main Phase, if exactly 1 monster with less ATK than your LP is
/// Summoned and you have "Z-ONE" in your GY: Destroy that monster, and if you do, take 800 damage, then inflict 800 damage
/// to your opponent." Our own Set Pot of Greed goes to Mystical Space Typhoon, the Gate follows, and a Normal Summoned
/// Kuriboh is destroyed: 800 each.
#[test]
fn soul_binding_gate_destroys_a_summoned_monster_for_800_each() {
    let spawns = [at(0, Hand, MST), at(0, Hand, SOUL_BINDING_GATE), at(0, SpellSet, POT_OF_GREED), at(0, Grave, Z_ONE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, POT_OF_GREED)]);
    s.run(&[Activate(MST), Activate(SOUL_BINDING_GATE), Summon(KURIBOH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "Kuriboh was destroyed: {}", s.context());
    assert_eq!(s.lp(0), 7200, "{}", s.context());
    assert_eq!(s.lp(1), 7200, "{}", s.context());
}

/// Soul Binding Gate: without "Z-ONE" in the GY the Summoned monster stays and nobody takes damage.
#[test]
fn soul_binding_gate_needs_z_one_in_the_graveyard() {
    let spawns = [at(0, Hand, MST), at(0, Hand, SOUL_BINDING_GATE), at(0, SpellSet, POT_OF_GREED)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, POT_OF_GREED)]);
    s.run(&[Activate(MST), Activate(SOUL_BINDING_GATE), Summon(KURIBOH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
}

/// Soul Binding Gate: "a Set Spell/Trap(s) you control must have been destroyed by card effect this turn": with none
/// destroyed it can't be activated, and the plan never gets there.
#[test]
fn soul_binding_gate_needs_a_destroyed_set_card() {
    let spawns = [at(0, Hand, SOUL_BINDING_GATE), at(0, Grave, Z_ONE)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[Activate(SOUL_BINDING_GATE)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Soul Charge: "Target any number of monsters in your GY; Special Summon them, and if you do, you lose 1000 LP for each
/// monster Special Summoned by this effect. You cannot conduct your Battle Phase the turn you activate this card." Two
/// Warwolves in the GY: they come back for 1000 LP each; the turn before, Kuriboh could go to the Battle Phase, now the
/// Main Phase has no way into it.
#[test]
fn soul_charge_brings_every_target_for_1000_lp_each_and_shuts_the_battle_phase() {
    let spawns = [at(0, SpellSet, SOUL_CHARGE), at(0, Monster, KURIBOH), at(0, Grave, WARWOLF), at(0, Grave, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.prompt()["to_bp"], true, "without Soul Charge the Battle Phase is open: {}", s.context());
    s.prefer(&[(0, WARWOLF), (0, WARWOLF)]);
    s.run(&[Activate(SOUL_CHARGE)]).unwrap();
    clean(&s);
    let back = s.count(0, "mzone", WARWOLF);
    assert!(back >= 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1000 * back as i64, "1000 LP for each of {back}: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 2 - back, "{}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase this turn: {}", s.context());
}

/// Soul Charge (anime): "Special Summon any number of monsters from your Graveyard, then, take 500 damage for each of
/// those monsters, but they cannot attack this turn." Half the LP cost; the Battle Phase stays open, but the monsters
/// that came back can't attack.
#[test]
fn soul_charge_anime_costs_500_each_and_keeps_the_monsters_from_attacking() {
    let spawns = [at(0, SpellSet, SOUL_CHARGE), at(0, Monster, KURIBOH), at(0, Grave, WARWOLF), at(0, Grave, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, WARWOLF)]);
    let err = s.run(&[EndTurn, Activate(SOUL_CHARGE), Attack(WARWOLF)]).unwrap_err();
    assert!(err.contains("can't attack"), "{err}");
    clean(&s);
    let back = s.count(0, "mzone", WARWOLF);
    assert!(back >= 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 500 * back as i64, "500 for each of {back}: {}", s.context());
}

/// Soul Union (a Trap): "Target 1 face-up monster on the field and 1 "Elemental HERO" monster in your GY; that target on the field
/// gains ATK equal to the other target's ATK (until the end of this turn), then if you have an "Elemental HERO" Normal
/// Monster in your field or GY, you can Fusion Summon 1 "Elemental HERO" Fusion Monster from your Extra Deck, by banishing
/// its materials from your GY." Avian (1000 ATK) and Burstinatrix in the GY: Warwolf gains 1000, and Flame Wingman is
/// Fusion Summoned with both banished.
#[test]
fn soul_union_gives_atk_and_fusion_summons_from_the_graveyard() {
    let spawns = [at(0, SpellSet, SOUL_UNION), at(0, Monster, WARWOLF), at(0, Grave, AVIAN), at(0, Grave, BURSTINATRIX)];
    let Some(mut s) = start_extra(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[FLAME_WINGMAN], &[]], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, AVIAN)]);
    s.run(&[Activate(SOUL_UNION)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(3000), "2000 and Avian's 1000: {}", s.context());
    assert_eq!(s.count(0, "mzone", FLAME_WINGMAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", AVIAN) + s.count(0, "removed", BURSTINATRIX), 2, "the materials: {}", s.context());
    assert_eq!(n(&s, 0, "grave"), 1, "only Soul Union is left in the GY: {}", s.context());
}

/// Soul Union (anime, a Trap): "When a face-up monster is targeted for an attack: Target 1 "Elemental HERO" monster in
/// your Graveyard; that attack target gains ATK equal to the targeted monster's ATK, until the end of this turn." Our
/// Warwolf (2000) is attacked by their Warwolf (2000): with Avian's 1000 it wins and their Warwolf is destroyed.
#[test]
fn soul_union_anime_strengthens_the_attacked_monster() {
    let spawns = [at(0, SpellSet, SOUL_UNION), at(0, Monster, WARWOLF), at(0, Grave, AVIAN), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(SOUL_UNION)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "their attacker was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Spacetime Transcendence: "Banish 2 or more Dinosaur monsters from your GY; Special Summon 1 Dinosaur monster from your
/// hand or GY whose Level equals the total Levels of those banished monsters, but it cannot attack this turn." Two Level
/// 2 Tomozaurus banished (4), the Level 4 Uraby from the hand comes, and it can't attack.
#[test]
fn spacetime_transcendence_banishes_dinosaurs_for_a_dinosaur_of_their_levels() {
    let spawns = [at(0, SpellSet, SPACETIME_TRANSCENDENCE), at(0, Hand, URABY), at(0, Monster, KURIBOH), at(0, Grave, TOMOZAURUS),
        at(0, Grave, TOMOZAURUS)];
    let Some(mut s) = printed(&spawns) else { return };
    s.declare_number(Some(4));
    let err = s.run(&[EndTurn, Activate(SPACETIME_TRANSCENDENCE), Attack(URABY)]).unwrap_err();
    assert!(err.contains("can't attack"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "removed", TOMOZAURUS), 2, "{}", s.context());
    assert_eq!(s.count(0, "mzone", URABY), 1, "{}", s.context());
}

/// Spacetime Transcendence (anime): "Banish any number of Dinosaur monsters from your Graveyard; Special Summon 1
/// Dinosaur monster from your hand with a Level equal to the number of monsters you banished, but it cannot attack this
/// turn." Four Tomozaurus banished for the Level 4 Uraby: the count, not the Levels.
#[test]
fn spacetime_transcendence_anime_counts_the_banished_monsters() {
    let spawns = [at(0, Hand, SPACETIME_TRANSCENDENCE), at(0, Hand, URABY), at(0, Grave, TOMOZAURUS), at(0, Grave, TOMOZAURUS),
        at(0, Grave, TOMOZAURUS), at(0, Grave, TOMOZAURUS)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Activate(SPACETIME_TRANSCENDENCE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", TOMOZAURUS), 4, "{}", s.context());
    assert_eq!(s.count(0, "mzone", URABY), 1, "{}", s.context());
}

/// Specimen Inspection: "Reveal 1 "Fossil Fusion" in your hand; send 1 monster from your hand to the GY, and if you do,
/// declare a Type and a Level, then your opponent looks at their hand and Deck, and sends 1 monster with the declared Type
/// and Level from one of them to the GY (if possible)." The declaration is the first Type and Level on offer (Warrior,
/// Level 1): the opponent's Swordsman from a Distant Land goes from the Deck, and nothing else of theirs.
#[test]
fn specimen_inspection_sends_the_declared_monster_from_the_opponents_deck() {
    let spawns = [at(0, Hand, SPECIMEN_INSPECTION), at(0, Hand, FOSSIL_FUSION)];
    let deck: Vec<u16> = [SWORDSMAN].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = start(false, [&[KURIBOH; 40], &deck], &spawns) else { return };
    s.run(&[Activate(SPECIMEN_INSPECTION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", SWORDSMAN), 1, "{}", s.context());
    assert_eq!(n(&s, 1, "grave"), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "the monster we sent: {}", s.context());
    assert_eq!(s.count(0, "hand", FOSSIL_FUSION), 1, "only revealed: {}", s.context());
}

/// Specimen Inspection: "Reveal 1 "Fossil Fusion" in your hand": without it the card can't be activated.
#[test]
fn specimen_inspection_needs_fossil_fusion_in_the_hand() {
    let Some(mut s) = printed(&[at(0, Hand, SPECIMEN_INSPECTION)]) else { return };
    let err = s.run(&[Activate(SPECIMEN_INSPECTION)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Spell of Pain: "When your opponent activates a card or effect that would inflict effect damage to you: Your opponent
/// takes the effect damage from that card or effect, instead." Their Ookazi (800) hurts them, not us.
#[test]
fn spell_of_pain_sends_effect_damage_back() {
    let spawns = [at(0, SpellSet, SPELL_OF_PAIN), at(1, Hand, OOKAZI)];
    let Some(mut s) = printed(&spawns) else { return };
    // Ookazi hurts its opponent: aim it at team 0 by being team 1's.
    s.run_both([&[EndTurn, Chain(SPELL_OF_PAIN)], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.lp(1), 7200, "their own 800: {}", s.context());
    assert_eq!(s.count(0, "grave", SPELL_OF_PAIN), 1, "{}", s.context());
}

/// Spell of Pain (anime): "When you would take damage: Your opponent takes that damage instead." Battle damage too: their
/// Warwolf's direct attack (2000) is theirs to take.
#[test]
fn spell_of_pain_anime_sends_battle_damage_back() {
    let spawns = [at(0, SpellSet, SPELL_OF_PAIN), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(SPELL_OF_PAIN)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.lp(1), 6000, "their own 2000: {}", s.context());
}


/// Spirit Shield: "Once per battle, when an opponent's monster declares an attack: You can banish 1 Fiend or Zombie
/// monster from your GY; negate that attack." Their Warwolf attacks our Skull Servant (a Zombie): negated, and the
/// Kuriboh (a Fiend) in the GY is banished for it. (The Shield stays: we control a Zombie at the End Phases.)
#[test]
fn spirit_shield_negates_an_attack_for_a_fiend_in_the_graveyard() {
    let spawns = [at(0, SpellSet, SPIRIT_SHIELD), at(0, Monster, SKULL_SERVANT), at(0, Grave, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[Activate(SPIRIT_SHIELD), EndTurn, ChainOnAttack(SPIRIT_SHIELD)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SKULL_SERVANT), 1, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "removed", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", SPIRIT_SHIELD), 1, "{}", s.context());
}

/// Spirit Shield: "Once per turn, when your opponent activates a card or effect that would destroy a card(s): You can
/// banish 1 Fiend or Zombie monster from your GY; negate that activation." Their Dark Hole is negated: our Skull Servant
/// lives, and Dark Hole is in their GY.
#[test]
fn spirit_shield_negates_a_destroying_activation() {
    let spawns = [at(0, SpellSet, SPIRIT_SHIELD), at(0, Monster, SKULL_SERVANT), at(0, Grave, KURIBOH), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[Activate(SPIRIT_SHIELD), EndTurn, Chain(SPIRIT_SHIELD)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SKULL_SERVANT), 1, "Dark Hole was negated: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", KURIBOH), 1, "{}", s.context());
}

/// Spirit Shield: "Once per turn, during the End Phase, if you do not control a Fiend or Zombie monster: Send this card
/// to the GY." Nothing of ours is a Fiend or a Zombie: the Shield goes at the first End Phase.
#[test]
fn spirit_shield_goes_to_the_graveyard_without_a_fiend_or_zombie() {
    let spawns = [at(0, SpellSet, SPIRIT_SHIELD), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(SPIRIT_SHIELD), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SPIRIT_SHIELD), 1, "{}", s.context());
}

/// Spirit Shield (anime): "When an opponent's monster declares an attack: You can banish 1 Fiend or Zombie-Type monster
/// from your Graveyard; negate that attack. If there are no Fiend or Zombie-Type monsters in your Graveyard, destroy this
/// card." Two Kuriboh in the GY: one is banished, the direct attack is negated and the Shield stays.
#[test]
fn spirit_shield_anime_negates_an_attack_while_the_graveyard_has_fiends() {
    let spawns = [at(0, SpellSet, SPIRIT_SHIELD), at(0, Grave, KURIBOH), at(0, Grave, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[Activate(SPIRIT_SHIELD), EndTurn, Chain(SPIRIT_SHIELD)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "removed", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", SPIRIT_SHIELD), 1, "{}", s.context());
}

/// Spirit Shield (anime): the same with the last Kuriboh of the GY: "negate that attack. If there are no Fiend or Zombie-
/// Type monsters in your Graveyard, destroy this card" says the attack is negated and then the Shield goes. The script's
/// self-destruction rule fires as soon as the cost empties the GY, so the card is gone before the negation applies and
/// the attack lands.
#[test]
fn spirit_shield_anime_negates_the_attack_and_then_goes_with_the_last_fiend() {
    let spawns = [at(0, SpellSet, SPIRIT_SHIELD), at(0, Grave, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[Activate(SPIRIT_SHIELD), EndTurn, Chain(SPIRIT_SHIELD)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", SPIRIT_SHIELD), 1, "{}", s.context());
}

/// Stand-Off: "Target 1 face-up Attack Position Effect Monster on each side of the field; ... negate their effects, then
/// ... as long as those monsters are in face-up Attack Position, they cannot be destroyed by battle, are unaffected by
/// other card effects, also they cannot attack or change their battle positions." Kuriboh on both sides: Dark Hole then
/// takes only the Warwolves (Normal Monsters).
#[test]
fn stand_off_makes_both_monsters_unaffected_by_other_cards() {
    let spawns = [at(0, Hand, STAND_OFF), at(0, Hand, DARK_HOLE), at(0, Monster, KURIBOH), at(0, Monster, WARWOLF),
        at(1, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(STAND_OFF), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF) + s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
}

/// Stand-Off: the targeted monsters "cannot attack": our Kuriboh is not among the attackers next turn (the Warwolf is).
#[test]
fn stand_off_keeps_the_monsters_from_attacking() {
    let spawns = [at(0, Hand, STAND_OFF), at(0, Monster, KURIBOH), at(0, Monster, WARWOLF), at(1, Monster, KURIBOH)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[Activate(STAND_OFF), EndTurn, Attack(KURIBOH)]).unwrap_err();
    assert!(err.contains("can't attack"), "{err}");
    clean(&s);
}

/// Stardust Flash (a Trap): "Target 1 "Stardust" monster in your GY; Special Summon it." Stardust Xiaolong comes back.
#[test]
fn stardust_flash_brings_a_stardust_monster_back() {
    let spawns = [at(0, SpellSet, STARDUST_FLASH), at(0, Grave, XIAOLONG)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(STARDUST_FLASH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", XIAOLONG), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", STARDUST_FLASH), 1, "{}", s.context());
}

/// Steam Synchron: "During your opponent's Main Phase, you can (Quick Effect): Immediately after this effect resolves,
/// Synchro Summon using this card you control." Steam Synchron (Level 3 Tuner) and Kuriboh make Armory Arm (Level 4) in
/// answer to the other player's Pot of Greed.
#[test]
fn steam_synchron_synchro_summons_in_the_opponents_main_phase() {
    let spawns = [at(0, Hand, POT_OF_GREED), at(1, Monster, STEAM_SYNCHRON), at(1, Monster, KURIBOH)];
    let Some(mut s) = start_extra(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[ARMORY_ARM]], &spawns) else { return };
    s.run_both([&[Activate(POT_OF_GREED)], &[Chain(STEAM_SYNCHRON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", ARMORY_ARM), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", STEAM_SYNCHRON), 1, "{}", s.context());
}

/// Steam Synchron (anime): "When a monster is Special Summoned, except "Steam Synchron": You can Special Summon this
/// card from your hand." Monster Reborn on a Warwolf: Steam Synchron comes from the hand too.
#[test]
fn steam_synchron_anime_special_summons_itself_from_the_hand() {
    let spawns = [at(0, Hand, STEAM_SYNCHRON), at(0, Hand, MONSTER_REBORN), at(0, Grave, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Activate(MONSTER_REBORN), Chain(STEAM_SYNCHRON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", STEAM_SYNCHRON), 1, "{}", s.context());
}

/// Stonehenge: "Target 1 monster with 0 ATK in your GY; Special Summon it in Attack Position, and if you do, equip it with
/// this card. When this card, equipped by this effect, leaves the field, destroy that monster." Stardust Phantom (0 ATK)
/// comes back with Stonehenge on it; Mystical Space Typhoon on Stonehenge then destroys the Phantom.
#[test]
fn stonehenge_equips_the_monster_it_revives_and_takes_it_along() {
    let spawns = [at(0, Hand, STONEHENGE), at(0, Hand, MST), at(0, Grave, PHANTOM)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(STONEHENGE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PHANTOM), 1, "{}", s.context());
    assert_eq!(s.position_of(0, PHANTOM), Some(1), "Attack Position: {}", s.context());
    assert_eq!(s.count(0, "szone", STONEHENGE), 1, "equipped: {}", s.context());
    s.prefer(&[(0, STONEHENGE)]);
    s.run(&[Activate(MST)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", STONEHENGE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", PHANTOM), 1, "the equipped monster was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", PHANTOM), 0, "{}", s.context());
}

/// Subspace Battle: "Each player reveals 1 monster from their Deck... Add the monster with the higher ATK to the hand of
/// the player that revealed it, also destroy the monster with the lower ATK and inflict 500 damage to the player that
/// revealed it." Three times, our Warwolf (2000) against their Kuriboh (300): three Warwolves added, three Kuriboh
/// destroyed, 1500 damage to them.
#[test]
fn subspace_battle_adds_the_higher_monster_and_destroys_the_lower_three_times() {
    let Some(mut s) = start(false, [&[WARWOLF; 40], &[KURIBOH; 40]], &[at(0, Hand, SUBSPACE_BATTLE)]) else { return };
    let hand = hand_of(&s, 0);
    s.run(&[Activate(SUBSPACE_BATTLE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6500, "500 three times: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(1, "grave", KURIBOH), 3, "{}", s.context());
    assert_eq!(hand_of(&s, 0), hand - 1 + 3, "{}", s.context());
}

/// Subspace Battle: "If tied, shuffle those monsters into the Decks": equal monsters change nothing.
#[test]
fn subspace_battle_shuffles_tied_monsters_back() {
    let Some(mut s) = start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], &[at(0, Hand, SUBSPACE_BATTLE)]) else { return };
    let (hand, deck) = (hand_of(&s, 0), deck_of(&s, 0));
    s.run(&[Activate(SUBSPACE_BATTLE)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    assert_eq!(n(&s, 1, "grave"), 0, "{}", s.context());
    assert_eq!(hand_of(&s, 0), hand - 1, "{}", s.context());
    assert_eq!(deck_of(&s, 0), deck, "{}", s.context());
}

/// Summon Dice: "Pay 1000 LP; roll a six-sided die and apply the result. 1 or 2: Immediately after this effect resolves,
/// you can Normal Summon 1 monster. 3 or 4: You can Special Summon 1 monster from your GY. 5 or 6: You can Special Summon
/// 1 Level 5 or higher monster from your hand." Each seed rolls its own die; across seeds every pair of faces is seen,
/// and each one does what it says (the roll is read from the engine's message).
#[test]
fn summon_dice_applies_each_result() {
    let mut seen = [false; 3];
    for seed in 1..80u64 {
        let spawns = [at(0, Hand, SUMMON_DICE), at(0, Hand, SKY_DRAGON), at(0, Grave, WARWOLF)];
        let Some(mut s) = start_seed(seed, false, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
        s.run_to(&[Activate(SUMMON_DICE)], |p| p["type"] == 13).unwrap();
        let roll = s.messages().iter().find(|m| m["type"] == 131).map(|m| m["results"][0].as_u64().unwrap());
        s.pass().unwrap();
        s.run(&[]).unwrap();
        clean(&s);
        let Some(roll) = roll else { panic!("seed {seed}: no die in the messages: {}", s.context()) };
        assert_eq!(s.lp(0), 7000, "seed {seed}: 1000 LP paid: {}", s.context());
        assert_eq!(n(&s, 0, "mzone"), 1, "seed {seed}, roll {roll}: {}", s.context());
        match roll {
            1 | 2 => {
                assert_eq!(s.count(0, "grave", WARWOLF), 1, "seed {seed}, roll {roll}: the GY is left alone: {}", s.context());
                seen[0] = true;
            }
            3 | 4 => {
                assert_eq!(s.count(0, "mzone", WARWOLF), 1, "seed {seed}, roll {roll}: {}", s.context());
                seen[1] = true;
            }
            _ => {
                assert_eq!(s.count(0, "mzone", SKY_DRAGON), 1, "seed {seed}, roll {roll}: {}", s.context());
                seen[2] = true;
            }
        }
        if seen == [true; 3] {
            return;
        }
    }
    panic!("not every face was rolled: {seen:?}");
}

/// Summon Storm: "Pay 800 LP; Special Summon 1 Level 6 or lower WIND monster from your hand." Sky Dragon (Level 6).
#[test]
fn summon_storm_special_summons_a_wind_monster_for_800_lp() {
    let spawns = [at(0, Hand, SUMMON_STORM), at(0, Hand, SKY_DRAGON)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(SUMMON_STORM)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7200, "{}", s.context());
    assert_eq!(s.count(0, "mzone", SKY_DRAGON), 1, "{}", s.context());
}

/// Summon Storm: "During your Main Phase, except the turn this card was sent to the GY: You can banish this card from
/// your GY; Special Summon 1 Level 4 or lower WIND monster from your hand." Harpie Lady (Level 4) comes, Summon Storm is
/// banished, no LP is paid.
#[test]
fn summon_storm_in_the_graveyard_banishes_itself_for_a_low_wind_monster() {
    let spawns = [at(0, Hand, HARPIE_LADY), at(0, Grave, SUMMON_STORM)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(SUMMON_STORM)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", HARPIE_LADY), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", SUMMON_STORM), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Summon Storm: the Graveyard effect takes Level 4 or lower only: with Sky Dragon (Level 6) alone it can't be used.
#[test]
fn summon_storm_in_the_graveyard_refuses_a_level_6_monster() {
    let spawns = [at(0, Hand, SKY_DRAGON), at(0, Grave, SUMMON_STORM)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[Activate(SUMMON_STORM)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Synkron Resonator: "If a Synchro Monster is on the field, you can Special Summon this card (from your hand)." The
/// opponent's Stardust Dragon is enough.
#[test]
fn synkron_resonator_special_summons_itself_next_to_a_synchro_monster() {
    let spawns = [at(0, Hand, SYNKRON_RESONATOR), at(1, Monster, STARDUST)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[SpSummon(SYNKRON_RESONATOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SYNKRON_RESONATOR), 1, "{}", s.context());
}

/// Synkron Resonator: "If this card is sent from the field to the GY: You can target 1 "Resonator" monster in your GY,
/// except "Synkron Resonator"; add it to your hand." Dark Hole sends it; Dark Resonator comes to the hand.
#[test]
fn synkron_resonator_adds_a_resonator_when_it_leaves_the_field() {
    let spawns = [at(0, Hand, DARK_HOLE), at(0, Monster, SYNKRON_RESONATOR), at(0, Grave, DARK_RESONATOR)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SYNKRON_RESONATOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", DARK_RESONATOR), 1, "{}", s.context());
}

/// Synkron Resonator (anime): "If your opponent controls a face-up Synchro Monster, you can Special Summon this card from
/// your hand." Our own Synchro Monster is not enough, theirs is.
#[test]
fn synkron_resonator_anime_needs_an_opponents_synchro_monster() {
    let own = [at(0, Hand, SYNKRON_RESONATOR), at(0, Monster, STARDUST)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &own) else { return };
    let err = s.run(&[SpSummon(SYNKRON_RESONATOR)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
    let theirs = [at(0, Hand, SYNKRON_RESONATOR), at(1, Monster, STARDUST)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &theirs) else { return };
    s.run(&[SpSummon(SYNKRON_RESONATOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SYNKRON_RESONATOR), 1, "{}", s.context());
}

/// T.G. Blade Blaster: "During either player's turn, when your opponent activates a Spell/Trap Card that targets this
/// face-up card: You can send 1 card from your hand to the Graveyard; negate the effect." Their Mind Control on it is
/// negated for a card of our hand.
#[test]
fn tg_blade_blaster_negates_a_spell_that_targets_it() {
    let spawns = [at(0, Monster, TG_BLADE_BLASTER), at(1, Hand, MIND_CONTROL)];
    let Some(mut s) = printed(&spawns) else { return };
    let hand = hand_of(&s, 0);
    s.run_both([&[Chain(TG_BLADE_BLASTER)], &[Activate(MIND_CONTROL)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TG_BLADE_BLASTER), 1, "still ours: {}", s.context());
    assert_eq!(s.count(1, "mzone", TG_BLADE_BLASTER), 0, "{}", s.context());
    assert_eq!(s.count(1, "grave", MIND_CONTROL), 1, "{}", s.context());
    assert_eq!(hand_of(&s, 0), hand - 1, "the card sent as the cost: {}", s.context());
}

/// T.G. Blade Blaster: "Once per turn, during your opponent's turn: You can banish 1 "T.G." monster from your Graveyard;
/// banish this face-up card on the field (this is a Quick Effect). During the next Standby Phase after this card was
/// banished by this effect: Special Summon this card." Synchro Summoned properly (Formula Synchron, a Tuner Synchro, and
/// Stardust Dragon: 2 + 8 = Level 10), it leaves in answer to the opponent's Pot of Greed and is back next turn.
#[test]
fn tg_blade_blaster_banishes_itself_and_returns_in_the_next_standby_phase() {
    let spawns = [at(0, Monster, FORMULA_SYNCHRON), at(0, Monster, STARDUST), at(0, Grave, TG_WARWOLF), at(1, Hand, POT_OF_GREED)];
    let Some(mut s) = start_extra(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[TG_BLADE_BLASTER], &[]], &spawns) else { return };
    s.run_both([&[SpSummon(TG_BLADE_BLASTER), EndTurn, Chain(TG_BLADE_BLASTER)], &[Activate(POT_OF_GREED), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", TG_WARWOLF), 1, "the cost: {}", s.context());
    assert_eq!(s.count(0, "mzone", TG_BLADE_BLASTER), 1, "back in our Standby Phase: {}", s.context());
}

/// T.G. Catapult Dragon: "Once per turn: You can Special Summon 1 Level 3 or lower "T.G." Tuner monster from your hand."
/// T.G. Striker (Level 2 Tuner) comes; the Level 4 non-Tuner Rush Rhino is not what it asks for.
#[test]
fn tg_catapult_dragon_special_summons_a_tg_tuner_from_the_hand() {
    let spawns = [at(0, Monster, TG_CATAPULT_DRAGON), at(0, Hand, TG_STRIKER), at(0, Hand, TG_RHINO)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(TG_CATAPULT_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TG_STRIKER), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", TG_RHINO), 1, "{}", s.context());
}

/// T.G. Catapult Dragon: only a Tuner of Level 3 or lower: with Rush Rhino alone in the hand the effect can't be used.
#[test]
fn tg_catapult_dragon_refuses_a_non_tuner() {
    let spawns = [at(0, Monster, TG_CATAPULT_DRAGON), at(0, Hand, TG_RHINO)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[Activate(TG_CATAPULT_DRAGON)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// T.G. Cyber Magician: "Once per turn, during the End Phase, if this card is in the GY because it was destroyed on the
/// field and sent there this turn: You can add 1 "T.G." monster from your Deck to your hand, except "T.G. Cyber
/// Magician"." Dark Hole destroys it; at the End Phase the Rush Rhino of the Deck is in the hand.
#[test]
fn tg_cyber_magician_adds_a_tg_monster_at_the_end_phase_after_it_is_destroyed() {
    let spawns = [at(0, Hand, DARK_HOLE), at(0, Monster, TG_CYBER_MAGICIAN)];
    let deck: Vec<u16> = [TG_RHINO].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &spawns) else { return };
    assert_eq!(s.count(0, "hand", TG_RHINO), 0, "the test needs Rush Rhino in the Deck: {}", s.context());
    s.run(&[Activate(DARK_HOLE), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TG_CYBER_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", TG_RHINO), 1, "{}", s.context());
}

/// T.G. Cyber Magician: "If this card you control would be used as Synchro Material for a "T.G." monster, "T.G." monsters
/// in your hand can be used as the non-Tuner Synchro Materials." Cyber Magician (Level 1 Tuner) and Rush Rhino from the
/// hand (Level 4) make T.G. Power Gladiator (Level 5).
#[test]
fn tg_cyber_magician_lets_a_tg_monster_in_the_hand_be_synchro_material() {
    let spawns = [at(0, Monster, TG_CYBER_MAGICIAN), at(0, Hand, TG_RHINO)];
    let Some(mut s) = start_extra(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[TG_POWER_GLADIATOR], &[]], &spawns) else { return };
    s.run(&[SpSummon(TG_POWER_GLADIATOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TG_POWER_GLADIATOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", TG_RHINO), 1, "the hand's material: {}", s.context());
    assert_eq!(s.count(0, "grave", TG_CYBER_MAGICIAN), 1, "{}", s.context());
}

/// The anime T.G. Cyber Magician: "When using this card as a Synchro Material Monster, the other non-Tuner Synchro
/// Material Monsters can be sent from your hand." Any monster, for any Synchro: Cyber Magician (Level 1 Tuner) and a
/// Warwolf from the hand (Level 4) make Ally of Justice Catastor (Level 5); the printed card allows only T.G. monsters
/// for a T.G. Synchro, so there it isn't on offer. (engine/overrides/proc_synchro.lua keeps this search fast: with the
/// unchanged procedure a hand of eight monsters took 19 s.)
#[test]
fn anime_tg_cyber_magician_lets_any_monster_in_the_hand_be_synchro_material() {
    let spawns = [at(0, Monster, TG_CYBER_MAGICIAN), at(0, Hand, WARWOLF)];
    let Some(mut s) = start_extra(true, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[CATASTOR], &[]], &spawns) else { return };
    s.run(&[SpSummon(CATASTOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", CATASTOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "the hand's material: {}", s.context());
    let Some(mut s) = start_extra(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[CATASTOR], &[]], &spawns) else { return };
    let printed = s.run(&[SpSummon(CATASTOR)]);
    assert!(printed.is_err_and(|e| e.contains("never became possible")), "the printed card: {}", s.context());
}

/// T.G. Drill Fish: "This card can attack directly." Our Drill Fish (100 ATK) attacks past their Warwolf: 100 damage.
#[test]
fn tg_drill_fish_attacks_directly() {
    let spawns = [at(0, Monster, TG_DRILL_FISH), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[EndTurn, Attack(TG_DRILL_FISH)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7900, "{}", s.context());
    assert_eq!(s.count(0, "mzone", TG_DRILL_FISH), 1, "{}", s.context());
}

/// T.G. Drill Fish: "If all monsters you control are "T.G." monsters (min. 1): You can Special Summon this card from your
/// hand."
#[test]
fn tg_drill_fish_special_summons_itself_while_only_tg_monsters_are_controlled() {
    let spawns = [at(0, Hand, TG_DRILL_FISH), at(0, Monster, TG_WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(TG_DRILL_FISH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TG_DRILL_FISH), 1, "{}", s.context());
}

/// T.G. Drill Fish: with a Kuriboh among our monsters, not all of them are "T.G.": no Special Summon.
#[test]
fn tg_drill_fish_does_not_special_summon_next_to_a_non_tg_monster() {
    let spawns = [at(0, Hand, TG_DRILL_FISH), at(0, Monster, TG_WARWOLF), at(0, Monster, KURIBOH)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[Activate(TG_DRILL_FISH)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// T.G. Drill Fish: "When a "T.G." monster you control inflicts battle damage to your opponent: You can target 1 monster
/// your opponent controls; destroy it." Rush Rhino destroys one of their two Kuriboh and the other goes to the Drill Fish.
#[test]
fn tg_drill_fish_destroys_a_monster_after_battle_damage() {
    let spawns = [at(0, Monster, TG_DRILL_FISH), at(0, Monster, TG_RHINO), at(1, Monster, KURIBOH), at(1, Monster, KURIBOH)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(1, KURIBOH)]);
    s.run_both([&[EndTurn, Attack(TG_RHINO), Chain(TG_DRILL_FISH)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 2, "{}", s.context());
    assert_eq!(n(&s, 1, "mzone"), 0, "{}", s.context());
    assert!(s.lp(1) <= 8000 - 1300, "at least the 1300 of the Rhino's battle damage: {}", s.context());
}

/// T.G. Gear Zombie: "You can target 1 "T.G." monster you control; Special Summon this card from your hand, then the
/// targeted monster loses 1000 ATK." Rush Rhino (1600) goes to 600.
#[test]
fn tg_gear_zombie_special_summons_itself_and_weakens_a_tg_monster() {
    let spawns = [at(0, Hand, TG_GEAR_ZOMBIE), at(0, Monster, TG_RHINO)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(TG_GEAR_ZOMBIE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TG_GEAR_ZOMBIE), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, TG_RHINO), Some(600), "{}", s.context());
}

/// T.G. Halberd Cannon: "Once per turn, during either player's turn, when a monster(s) would be Summoned: You can negate
/// the Summon, and if you do, destroy that monster." Their Normal Summon of Kuriboh is negated.
#[test]
fn tg_halberd_cannon_negates_a_summon_and_destroys_the_monster() {
    let spawns = [at(0, Monster, TG_HALBERD_CANNON)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[Chain(TG_HALBERD_CANNON)], &[Summon(KURIBOH)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 1, "{}", s.context());
    assert_eq!(n(&s, 1, "mzone"), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", TG_HALBERD_CANNON), 1, "{}", s.context());
}

/// T.G. Halberd Cannon: "When this card is sent from the field to the Graveyard: You can target 1 "T.G." monster in your
/// Graveyard; Special Summon that target." Dark Hole destroys it; T.G. Warwolf comes back from the GY.
#[test]
fn tg_halberd_cannon_brings_a_tg_monster_back_when_it_goes() {
    let spawns = [at(0, Hand, DARK_HOLE), at(0, Monster, TG_HALBERD_CANNON), at(0, Grave, TG_WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TG_HALBERD_CANNON), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", TG_WARWOLF), 1, "{}", s.context());
}

/// T.G. Halberd Cannon (anime): "When a monster(s) would be Summoned: You can negate the Summon, and if you do, destroy
/// that monster. ... When this card leaves the field: you can Special Summon 1 "T.G. Blade Blaster" from your Extra
/// Deck." Dark Hole destroys it and Blade Blaster comes from the Extra Deck.
#[test]
fn tg_halberd_cannon_anime_brings_blade_blaster_when_it_leaves() {
    let spawns = [at(0, Hand, DARK_HOLE), at(0, Monster, TG_HALBERD_CANNON)];
    let Some(mut s) = start_extra(true, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[TG_BLADE_BLASTER], &[]], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TG_HALBERD_CANNON), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", TG_BLADE_BLASTER), 1, "{}", s.context());
}
