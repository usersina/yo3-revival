//! Printed anime cards, group 16 (rows 4876-5178 of `engine/added/cards.tsv`): Dimension Dice, Dimension Fusion
//! Destruction, Doom Virus Dragon, Double Evolution Pill, Double Fusion and Double Magical Arm Bind. Each effect is
//! played through the real core with both seats' client models checked after every step (`duelcheck::scenario`): the
//! printed effect with anime cards off, and one test of the anime version with them on where it differs.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// Added cards of this group (client ids from engine/added-cards.json).
const DIMENSION_DICE: u16 = 4699;
const DIMENSION_FUSION_DESTRUCTION: u16 = 15144;
const DOOM_VIRUS_DRAGON: u16 = 11882;
const DOUBLE_EVOLUTION_PILL: u16 = 6839;
const DOUBLE_FUSION: u16 = 5075;
const DOUBLE_MAGICAL_ARM_BIND: u16 = 11893;
const FANG_OF_CRITIAS: u16 = 11885; // The Fang of Critias (row 4982)
const DICE_DUNGEON: u16 = 4697; // stand-in for Konami id 17623
const ORGOTH: u16 = 13800; // Orgoth the Relentless
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const BLIND_DESTRUCTION: u16 = 5268; // a Continuous Trap whose effect rolls a die
const MAXIMUM_SIX: u16 = 5943; // a Level 6 effect monster whose effect rolls a die
const AGIDO: u16 = 5366; // an effect monster whose effect rolls a die
const URIA: u16 = 6563;
const HAMON: u16 = 6564;
const RAVIEL: u16 = 6565;
const ARMITYLE: u16 = 7342; // Armityle the Chaos Phantasm, a Phantasm Fusion Monster
const CRUSH_CARD_VIRUS: u16 = 4667;
const URABY: u16 = 4086; // a Dinosaur
const SUPER_CONDUCTOR_TYRANNO: u16 = 6816; // a Level 8 LIGHT Dinosaur
const CYBER_DRAGON: u16 = 6390; // a Machine
const FLAME_MANIPULATOR: u16 = 4148;
const MASAKI: u16 = 4299;
const FLAME_SWORDSMAN: u16 = 4021; // a Fusion Monster, a Normal Monster in the anime
const STEEL_OGRE_GROTTO_1: u16 = 4563;
const LESSER_DRAGON: u16 = 4567;
const METAL_DRAGON: u16 = 4415; // Steel Ogre Grotto #1 + Lesser Dragon
const GAIA_FIERCE_KNIGHT: u16 = 4044;
const CURSE_OF_DRAGON: u16 = 4045;
const GAIA_DRAGON_CHAMPION: u16 = 4043;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn field(anime: bool, lp: u32, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(
        if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }
            .unwrap(),
    )
}

fn mon(team: u32, cid: u16) -> Spawn {
    Spawn { team, place: Monster, cid }
}

fn in_hand(team: u32, cid: u16) -> Spawn {
    Spawn { team, place: Hand, cid }
}

fn in_grave(team: u32, cid: u16) -> Spawn {
    Spawn { team, place: Grave, cid }
}

/// Whether the planned side's idle prompt offers to activate this card.
fn can_activate(s: &Scenario, cid: u16) -> bool {
    s.prompt()["activates"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
}

const PLAIN: [u16; 40] = [KURIBOH; 40];

// ---- Dimension Dice ----

/// Dimension Dice: "If you control a card with an effect that requires a die roll: You can Tribute 1 monster; Special
/// Summon 1 monster with a monster effect that requires a die roll from your hand or Deck." Blind Destruction is the die
/// card, Kuriboh the Tribute, Maximum Six waits in the Deck.
#[test]
fn dimension_dice_tributes_one_and_summons_a_dice_monster_from_the_deck() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([MAXIMUM_SIX]).collect();
    let spawns = [in_hand(0, DIMENSION_DICE), Spawn { team: 0, place: Spell, cid: BLIND_DESTRUCTION }, mon(0, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(DIMENSION_DICE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", MAXIMUM_SIX), 1, "Maximum Six from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "the Tribute: {}", s.context());
    assert_eq!(s.count(0, "szone", BLIND_DESTRUCTION), 1, "the die card stays: {}", s.context());
    assert_eq!(s.count(0, "grave", DIMENSION_DICE), 1, "{}", s.context());
}

/// The same from the hand: Agido is in the hand, not the Deck.
#[test]
fn dimension_dice_summons_a_dice_monster_from_the_hand() {
    let spawns = [in_hand(0, DIMENSION_DICE), in_hand(0, AGIDO), Spawn { team: 0, place: Spell, cid: BLIND_DESTRUCTION }, mon(0, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(DIMENSION_DICE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", AGIDO), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", AGIDO), 0, "{}", s.context());
}

/// "If you control a card with an effect that requires a die roll": with only Kuriboh and Warwolf, which roll no die,
/// the card cannot be activated.
#[test]
fn dimension_dice_needs_a_dice_card_on_the_field() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([MAXIMUM_SIX]).collect();
    let spawns = [in_hand(0, DIMENSION_DICE), mon(0, WARWOLF), mon(0, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert!(!can_activate(&s, DIMENSION_DICE), "offered without a die card: {}", s.context());
    assert_eq!(s.count(0, "mzone", MAXIMUM_SIX), 0, "{}", s.context());
}

/// Dimension Dice, anime version: with "Dice Dungeon" face-up (activated first), Tribute 2 monsters and Special Summon "Orgoth the
/// Relentless" from the hand or Deck.
#[test]
fn dimension_dice_anime_tributes_two_for_orgoth_with_dice_dungeon() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([ORGOTH]).collect();
    let spawns = [in_hand(0, DIMENSION_DICE), in_hand(0, DICE_DUNGEON), mon(0, KURIBOH), mon(0, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DICE_DUNGEON), Activate(DIMENSION_DICE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ORGOTH), 1, "Orgoth from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH) + s.count(0, "grave", WARWOLF), 2, "both Tributes: {}", s.context());
}

// ---- Dimension Fusion Destruction ----

/// Dimension Fusion Destruction: "Banish from your hand, field, and/or GY, the Fusion Materials that are listed on a
/// "Phantasm" Fusion Monster, then Special Summon that Fusion Monster from your Extra Deck, ignoring its Summoning
/// conditions." Uria, Hamon and Raviel on the field: they are banished, Armityle comes.
#[test]
fn dimension_fusion_destruction_banishes_the_three_lords_for_armityle() {
    let spawns = [in_hand(0, DIMENSION_FUSION_DESTRUCTION), mon(0, URIA), mon(0, HAMON), mon(0, RAVIEL)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[ARMITYLE]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DIMENSION_FUSION_DESTRUCTION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ARMITYLE), 1, "{}", s.context());
    for lord in [URIA, HAMON, RAVIEL] {
        assert_eq!(s.count(0, "removed", lord), 1, "{lord} banished: {}", s.context());
        assert_eq!(s.count(0, "mzone", lord), 0, "{lord} left the field: {}", s.context());
    }
}

/// "from your hand, field, and/or GY": Uria on the field, Hamon in the Graveyard and Raviel in the hand all count.
#[test]
fn dimension_fusion_destruction_takes_materials_from_hand_field_and_grave() {
    let spawns = [in_hand(0, DIMENSION_FUSION_DESTRUCTION), mon(0, URIA), in_grave(0, HAMON), in_hand(0, RAVIEL)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[ARMITYLE]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DIMENSION_FUSION_DESTRUCTION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ARMITYLE), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", URIA), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", HAMON), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", RAVIEL), 1, "{}", s.context());
}

/// "You take no battle damage from attacks involving the monster Special Summoned by this effect": the opponent's
/// Warwolf attacks Armityle (0 ATK on the opponent's turn) and the controller's Life Points stay.
#[test]
fn dimension_fusion_destruction_gives_no_battle_damage() {
    let spawns = [in_hand(0, DIMENSION_FUSION_DESTRUCTION), mon(0, URIA), mon(0, HAMON), mon(0, RAVIEL), in_hand(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[ARMITYLE]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[Activate(DIMENSION_FUSION_DESTRUCTION), EndTurn], &[Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ARMITYLE), 1, "Armityle cannot be destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
}

/// Dimension Fusion Destruction, anime version: the three Fusion Materials come from the field only; with them in the
/// hand it cannot be activated.
#[test]
fn dimension_fusion_destruction_anime_takes_the_field_only() {
    let spawns = [in_hand(0, DIMENSION_FUSION_DESTRUCTION), mon(0, URIA), mon(0, HAMON), mon(0, RAVIEL)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[ARMITYLE]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DIMENSION_FUSION_DESTRUCTION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ARMITYLE), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", URIA) + s.count(0, "removed", HAMON) + s.count(0, "removed", RAVIEL), 3, "{}", s.context());
    let spawns = [in_hand(0, DIMENSION_FUSION_DESTRUCTION), mon(0, URIA), mon(0, HAMON), in_hand(0, RAVIEL)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[ARMITYLE]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!can_activate(&s, DIMENSION_FUSION_DESTRUCTION), "a lord in the hand does not count: {}", s.context());
}

// ---- Doom Virus Dragon ----

/// Doom Virus Dragon: "Must be Special Summoned with "The Fang of Critias", using "Crush Card Virus". If this card is
/// Special Summoned: Look at your opponent's hand, all monsters they control, and ... destroy all those monsters with
/// 1500 or more ATK." The Fang sends the Set Crush Card Virus, the Dragon comes and destroys Warwolf (on the field and
/// in the hand) but not Kuriboh.
#[test]
fn doom_virus_dragon_destroys_the_strong_monsters_it_sees() {
    let spawns = [
        in_hand(0, FANG_OF_CRITIAS),
        Spawn { team: 0, place: SpellSet, cid: CRUSH_CARD_VIRUS },
        mon(1, WARWOLF),
        mon(1, KURIBOH),
        in_hand(1, WARWOLF),
        in_hand(1, KURIBOH),
    ];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[DOOM_VIRUS_DRAGON]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(FANG_OF_CRITIAS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DOOM_VIRUS_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", CRUSH_CARD_VIRUS), 1, "the Trap the Fang sent: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "the monster on the field: {}", s.context());
    assert_eq!(s.count(1, "hand", WARWOLF), 0, "the monster in the hand: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 2, "{}", s.context());
    assert_eq!(s.count(1, "mzone", KURIBOH), 1, "300 ATK stays: {}", s.context());
    assert!(s.count(1, "hand", KURIBOH) >= 1, "300 ATK stays: {}", s.context());
}

/// "...and all cards they draw until the end of their 3rd turn after this effect's activation": the opponent's deck is
/// all Warwolf, each drawn one is destroyed in their Draw Phase.
#[test]
fn doom_virus_dragon_destroys_the_strong_monsters_the_opponent_draws() {
    let spawns = [in_hand(0, FANG_OF_CRITIAS), Spawn { team: 0, place: SpellSet, cid: CRUSH_CARD_VIRUS }];
    let warwolves = [WARWOLF; 40];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[DOOM_VIRUS_DRAGON]), (&warwolves, &[])], &spawns) else { return };
    s.run_both([&[Activate(FANG_OF_CRITIAS), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DOOM_VIRUS_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(1, "hand", WARWOLF), 0, "the drawn Warwolf is destroyed: {}", s.context());
    assert!(s.count(1, "grave", WARWOLF) >= 1, "{}", s.context());
}

/// "Must be Special Summoned with The Fang of Critias": without the Fang the Extra Deck copy cannot come out.
#[test]
fn doom_virus_dragon_cannot_be_special_summoned_by_itself() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: CRUSH_CARD_VIRUS }, mon(0, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[DOOM_VIRUS_DRAGON]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!s.prompt()["special_summons"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == DOOM_VIRUS_DRAGON)),
        "{}", s.context());
    assert_eq!(s.count(0, "mzone", DOOM_VIRUS_DRAGON), 0, "{}", s.context());
}

/// Doom Virus Dragon, anime version: it destroys every monster of the opponent's with 1500 or more ATK at all times, so
/// a Warwolf the opponent Normal Summons next turn is destroyed at once.
#[test]
fn doom_virus_dragon_anime_destroys_monsters_the_opponent_summons() {
    let spawns = [in_hand(0, FANG_OF_CRITIAS), Spawn { team: 0, place: SpellSet, cid: CRUSH_CARD_VIRUS }, mon(1, WARWOLF),
        in_hand(1, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[DOOM_VIRUS_DRAGON]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[Activate(FANG_OF_CRITIAS), EndTurn], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DOOM_VIRUS_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 2, "{}", s.context());
}

// ---- Double Evolution Pill ----

/// Double Evolution Pill: "Banish 1 Dinosaur monster and 1 non-Dinosaur monster from your hand and/or GY; Special Summon
/// 1 Level 7 or higher Dinosaur monster from your hand or Deck, ignoring its Summoning conditions."
#[test]
fn double_evolution_pill_banishes_two_and_summons_a_big_dinosaur() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([SUPER_CONDUCTOR_TYRANNO]).collect();
    let spawns = [in_hand(0, DOUBLE_EVOLUTION_PILL), in_hand(0, URABY), in_grave(0, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.prefer(&[(0, URABY), (0, WARWOLF)]);
    s.run(&[Activate(DOUBLE_EVOLUTION_PILL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SUPER_CONDUCTOR_TYRANNO), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", URABY), 1, "the Dinosaur is banished: {}", s.context());
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "the non-Dinosaur is banished: {}", s.context());
}

/// "You can only activate 1 "Double Evolution Pill" per turn": with a second Pill and cost monsters left, it is not offered.
#[test]
fn double_evolution_pill_is_once_per_turn() {
    let deck: Vec<u16> = [KURIBOH; 38].into_iter().chain([SUPER_CONDUCTOR_TYRANNO; 2]).collect();
    let spawns = [in_hand(0, DOUBLE_EVOLUTION_PILL), in_hand(0, DOUBLE_EVOLUTION_PILL), in_hand(0, URABY), in_hand(0, URABY),
        in_grave(0, WARWOLF), in_grave(0, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DOUBLE_EVOLUTION_PILL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SUPER_CONDUCTOR_TYRANNO), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", DOUBLE_EVOLUTION_PILL), 1, "{}", s.context());
    assert!(!can_activate(&s, DOUBLE_EVOLUTION_PILL), "a second Pill the same turn: {}", s.context());
}

/// Both a Dinosaur and a non-Dinosaur are needed: two Dinosaurs alone cannot pay the cost.
#[test]
fn double_evolution_pill_needs_a_dinosaur_and_a_non_dinosaur() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([SUPER_CONDUCTOR_TYRANNO]).collect();
    let spawns = [in_hand(0, DOUBLE_EVOLUTION_PILL), in_hand(0, URABY), in_grave(0, URABY)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!can_activate(&s, DOUBLE_EVOLUTION_PILL), "{}", s.context());
}

/// Double Evolution Pill, anime version: banish a Machine and a Dinosaur from the hand, field and/or GY; Special Summon
/// a LIGHT Dinosaur from the hand or Deck.
#[test]
fn double_evolution_pill_anime_banishes_a_machine_and_a_dinosaur_for_a_light_dinosaur() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([SUPER_CONDUCTOR_TYRANNO]).collect();
    let spawns = [in_hand(0, DOUBLE_EVOLUTION_PILL), in_hand(0, URABY), in_grave(0, CYBER_DRAGON)];
    let Some(mut s) = field(true, 8000, [(&deck, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DOUBLE_EVOLUTION_PILL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SUPER_CONDUCTOR_TYRANNO), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", URABY), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", CYBER_DRAGON), 1, "{}", s.context());
}

// ---- Double Fusion ----

/// Double Fusion: "Pay 500 LP; Fusion Summon 1 Fusion Monster from your Extra Deck, using monsters from your hand or
/// field, then you can Fusion Summon 1 more Fusion Monster from your Extra Deck, using monsters from your hand or
/// field." Flame Swordsman and Gaia the Dragon Champion, all four materials in the hand.
#[test]
fn double_fusion_summons_two_fusion_monsters_for_500_lp() {
    let spawns = [in_hand(0, DOUBLE_FUSION), in_hand(0, FLAME_MANIPULATOR), in_hand(0, MASAKI), in_hand(0, GAIA_FIERCE_KNIGHT),
        in_hand(0, CURSE_OF_DRAGON)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[FLAME_SWORDSMAN, GAIA_DRAGON_CHAMPION]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DOUBLE_FUSION)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "the 500 LP cost: {}", s.context());
    assert_eq!(s.count(0, "mzone", FLAME_SWORDSMAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", GAIA_DRAGON_CHAMPION), 1, "{}", s.context());
    for material in [FLAME_MANIPULATOR, MASAKI, GAIA_FIERCE_KNIGHT, CURSE_OF_DRAGON] {
        assert_eq!(s.count(0, "grave", material), 1, "{material} used: {}", s.context());
    }
}

/// With materials for only one Fusion Monster, one is summoned (the second is "you can").
#[test]
fn double_fusion_with_one_set_of_materials_summons_one() {
    let spawns = [in_hand(0, DOUBLE_FUSION), in_hand(0, FLAME_MANIPULATOR), in_hand(0, MASAKI)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[FLAME_SWORDSMAN, GAIA_DRAGON_CHAMPION]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DOUBLE_FUSION)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FLAME_SWORDSMAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", GAIA_DRAGON_CHAMPION), 0, "{}", s.context());
}

/// Double Fusion, anime version: the same two Fusion Summons, the materials from the hand or the field (Steel
/// Ogre Grotto #1 and Lesser Dragon on the field, Gaia and Curse of Dragon in the hand for Metal Dragon and Gaia the
/// Dragon Champion; Flame Swordsman is a Normal Monster in the anime).
#[test]
fn double_fusion_anime_summons_two_from_hand_and_field() {
    let spawns = [in_hand(0, DOUBLE_FUSION), mon(0, STEEL_OGRE_GROTTO_1), mon(0, LESSER_DRAGON), in_hand(0, GAIA_FIERCE_KNIGHT),
        in_hand(0, CURSE_OF_DRAGON)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[METAL_DRAGON, GAIA_DRAGON_CHAMPION]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DOUBLE_FUSION)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "{}", s.context());
    assert_eq!(s.count(0, "mzone", METAL_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", GAIA_DRAGON_CHAMPION), 1, "{}", s.context());
}

// ---- Double Magical Arm Bind ----

/// Double Magical Arm Bind: "Tribute 2 monsters, then target 2 face-up monsters your opponent controls; take control of
/// both targets until your End Phase." Both Warwolves are mine after it resolves.
#[test]
fn double_magical_arm_bind_takes_two_monsters() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DOUBLE_MAGICAL_ARM_BIND }, mon(0, KURIBOH), mon(0, KURIBOH), mon(1, WARWOLF), mon(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DOUBLE_MAGICAL_ARM_BIND)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 2, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 2, "the two Tributes: {}", s.context());
}

/// "...until your End Phase": at the start of the opponent's turn both monsters are back with their owner.
#[test]
fn double_magical_arm_bind_returns_them_at_the_end_phase() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DOUBLE_MAGICAL_ARM_BIND }, mon(0, KURIBOH), mon(0, KURIBOH), mon(1, WARWOLF), mon(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[Activate(DOUBLE_MAGICAL_ARM_BIND), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", WARWOLF), 2, "returned: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
}

/// It needs 2 face-up monsters of the opponent's to target: with one it is not offered.
#[test]
fn double_magical_arm_bind_needs_two_targets() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DOUBLE_MAGICAL_ARM_BIND }, mon(0, KURIBOH), mon(0, KURIBOH), mon(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!can_activate(&s, DOUBLE_MAGICAL_ARM_BIND), "{}", s.context());
}

/// Double Magical Arm Bind, anime version: the control is kept (no End Phase return), but the stolen monsters cannot
/// attack or change their battle position.
#[test]
fn double_magical_arm_bind_anime_keeps_them_but_they_cannot_attack() {
    let spawns = [Spawn { team: 0, place: SpellSet, cid: DOUBLE_MAGICAL_ARM_BIND }, mon(0, KURIBOH), mon(0, KURIBOH), mon(1, WARWOLF), mon(1, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&PLAIN, &[]), (&PLAIN, &[])], &spawns) else { return };
    s.run_both([&[Activate(DOUBLE_MAGICAL_ARM_BIND), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 2, "still mine next turn: {}", s.context());
    assert!(s.run(&[Attack(WARWOLF)]).is_err(), "a stolen monster cannot attack: {}", s.context());
}
