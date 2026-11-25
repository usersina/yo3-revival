//! Scenario tests for the eleventh group of the anime's printed cards (rows 4848-5205 of engine/added/cards.tsv): from
//! "Pair Cycroid" to "Red-Eyes Burn". Each through the real core with both seats' client models checked after every step
//! (`duelcheck::scenario`). Texts: the printed ones (anime cards off) from the pinned official database; an anime version
//! is tested with anime cards on (`start_anime_field`).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The added cards of this group (client ids from the group list).
const PAIR_CYCROID: u16 = 11164;
const PEGASUS_WING: u16 = 14540;
const PENGUIN_SWORD: u16 = 4001;
const PENGUIN_TORPEDO: u16 = 5807;
const PIRI_REIS_MAP: u16 = 4684;
const EITOM: u16 = 5167; // Plasma Warrior Eitom
const POWER_WALL: u16 = 6834;
const POWER_UP_ADAPTER: u16 = 9535;
const POWERFUL_REBIRTH: u16 = 10983;
const PRIMA_LIGHT: u16 = 6849;
const PRINZESSIN: u16 = 13802;
const PSYCHIC_WAVE: u16 = 15373;
const PUMPKIN_CARRIAGE: u16 = 13803;
const QUICK_SUMMON: u16 = 5172;
const BIFROST: u16 = 5115; // Rainbow Bridge Bifrost
const RAPHION: u16 = 13817;
const REAPER_SCYTHE: u16 = 11157; // Reaper Scythe - Dreadscythe
const REBELLION: u16 = 6152;
const REBIRTH_JUDGMENT: u16 = 4005;
const RED_COCOON: u16 = 12066;
const RED_DRAGON_VASE: u16 = 9517;
const RED_NOVA: u16 = 12540;
const RED_EYES_SWORD: u16 = 11891; // Red-Eyes Black Dragon Sword
const RED_EYES_BURN: u16 = 11899;
// Other added cards the scenarios use.
const DREADSCYTHE: u16 = 11155; // Guardian Dreadscythe
const CLAW_OF_HERMOS: u16 = 11892;
const GOLDEN_CASTLE: u16 = 13808; // Golden Castle of Stromberg
const GLASS_SLIPPERS: u16 = 13809;
const CYBER_TUTU: u16 = 6664;
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const MST: u16 = 4909; // Mystical Space Typhoon
const HEAVY_STORM: u16 = 4891;
const PENGUIN_KNIGHT: u16 = 4205; // Level 3, 900 ATK
const VALKYRIE_DRITTE: u16 = 13992; // 1000 ATK
const Y_DRAGON_HEAD: u16 = 5552; // a Union monster
const JINZO: u16 = 4758; // a Machine
const JINZO_RETURNER: u16 = 7579; // 600 ATK
const GADGET_HAULER: u16 = 7881; // a Machine
const CYBER_VALLEY: u16 = 7413; // 0 ATK
const GAIA: u16 = 4044; // Gaia The Fierce Knight: EARTH Warrior, Level 7
const CYBER_PRIMA: u16 = 6666; // LIGHT Warrior
const BLUE_EYES: u16 = 4007; // Level 8, 3000 ATK
const BLUE_EYES_ULTIMATE: u16 = 4386; // Level 12, a Fusion
const CYBER_END: u16 = 6397; // Level 10, a Fusion
const GAMMA_MAGNET: u16 = 4792; // Gamma The Magnet Warrior
const MORPHTRONIC_STAPLEN: u16 = 9482; // 1400 ATK
const SUMMONED_SKULL: u16 = 4028; // Level 6
const STARDUST: u16 = 7734; // a Level 8 Dragon Synchro
const RED_DRAGON_ARCHFIEND: u16 = 7735;
const POWER_TOOL: u16 = 8137; // Power Tool Dragon, a Level 7 Synchro
const TURBO_SYNCHRON: u16 = 7870; // a Level 1 Tuner
const RED_EYES: u16 = 4088; // Red-Eyes Black Dragon: 2400 ATK
const YAMI: u16 = 4341; // a Field Spell
const FORTUNE_CHARIOT: u16 = 13996;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

/// A duel with the given Main and Extra Decks of each team and `spawns` in place.
fn start(anime: bool, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// Kuriboh Main Decks, no Extra Decks.
fn duel(anime: bool, spawns: &[Spawn]) -> Option<Scenario> {
    start(anime, [(&[KURIBOH; 40], &[][..]), (&[KURIBOH; 40], &[][..])], spawns)
}

/// A Main Deck of one card then Kuriboh.
fn deck_with(cards: &[u16]) -> Vec<u16> {
    cards.iter().copied().chain(std::iter::repeat(KURIBOH)).take(40).collect()
}

/// Pair Cycroid: "This card can attack directly."
#[test]
fn pair_cycroid_attacks_directly() {
    let Some(mut s) = duel(false, &[at(0, Monster, PAIR_CYCROID), at(1, Monster, WARWOLF)]) else { return };
    s.run(&[EndTurn, Attack(PAIR_CYCROID)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6400, "1600 damage past Warwolf: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Pair Cycroid (anime): "it loses 500 ATK during damage calculation only" when it attacks directly.
#[test]
fn pair_cycroid_anime_hits_for_500_less() {
    let Some(mut s) = duel(true, &[at(0, Monster, PAIR_CYCROID), at(1, Monster, WARWOLF)]) else { return };
    s.run(&[EndTurn, Attack(PAIR_CYCROID)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6900, "1100 damage: {}", s.context());
    assert_eq!(s.attack_of(0, PAIR_CYCROID), Some(1600), "its ATK is back after the damage: {}", s.context());
}

/// Pegasus Wing: "If you have a Union monster in your GY: Target any number of "Valkyrie" monsters you control; those
/// monsters can attack directly this turn, but ... their battle damage inflicted to your opponent is halved."
#[test]
fn pegasus_wing_lets_a_valkyrie_attack_directly_for_half() {
    let spawns = [at(1, Hand, PEGASUS_WING), at(1, Monster, VALKYRIE_DRITTE), at(1, Grave, Y_DRAGON_HEAD), at(0, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(1, VALKYRIE_DRITTE)]);
    s.run_both([&[], &[Activate(PEGASUS_WING), Attack(VALKYRIE_DRITTE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "1000 halved: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Pegasus Wing can't be activated without a Union monster in the GY.
#[test]
fn pegasus_wing_needs_a_union_monster_in_the_graveyard() {
    let spawns = [at(0, Hand, PEGASUS_WING), at(0, Monster, VALKYRIE_DRITTE), at(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    let err = s.run(&[Activate(PEGASUS_WING)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Pegasus Wing (anime): "If there is a "Fortune Chariot" in your Graveyard: Target 1 "Valkyrie" monster; halve its ATK
/// and it can attack directly this turn."
#[test]
fn pegasus_wing_anime_halves_the_attack_and_goes_direct() {
    let spawns = [at(1, Hand, PEGASUS_WING), at(1, Monster, VALKYRIE_DRITTE), at(1, Grave, FORTUNE_CHARIOT), at(0, Monster, WARWOLF)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.prefer(&[(1, VALKYRIE_DRITTE)]);
    s.run_both([&[], &[Activate(PEGASUS_WING), Attack(VALKYRIE_DRITTE)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "500 ATK left after halving: {}", s.context());
}

/// Penguin Sword: "Equip only to a "Penguin" monster. It gains 800 ATK."
#[test]
fn penguin_sword_equips_a_penguin_for_800() {
    let Some(mut s) = duel(false, &[at(0, Hand, PENGUIN_SWORD), at(0, Monster, PENGUIN_KNIGHT)]) else { return };
    s.run(&[Activate(PENGUIN_SWORD)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, PENGUIN_KNIGHT), Some(1700), "900 and 800: {}", s.context());
    assert_eq!(s.count(0, "szone", PENGUIN_SWORD), 1, "{}", s.context());
}

/// Penguin Sword equips only to a Penguin: with just Warwolf it can't be activated.
#[test]
fn penguin_sword_does_not_equip_a_non_penguin() {
    let Some(mut s) = duel(false, &[at(0, Hand, PENGUIN_SWORD), at(0, Monster, WARWOLF)]) else { return };
    let err = s.run(&[Activate(PENGUIN_SWORD)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Penguin Sword: "When the equipped monster inflicts battle damage to your opponent: You can return 1 card your opponent
/// controls to the hand."
#[test]
fn penguin_sword_returns_a_card_after_battle_damage() {
    let spawns = [at(0, Hand, PENGUIN_SWORD), at(0, Monster, PENGUIN_KNIGHT), at(1, SpellSet, MST)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(PENGUIN_SWORD), EndTurn, Attack(PENGUIN_KNIGHT), Chain(PENGUIN_SWORD)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6300, "1700 damage: {}", s.context());
    assert_eq!(s.count(1, "szone", MST), 0, "the Set card went back: {}", s.context());
    assert_eq!(s.count(1, "hand", MST), 1, "{}", s.context());
}

/// Penguin Sword (anime): "Equip only to a "Penguin" monster. It gains 800 ATK." and nothing else: no card is returned.
#[test]
fn penguin_sword_anime_only_adds_800() {
    let spawns = [at(0, Hand, PENGUIN_SWORD), at(0, Monster, PENGUIN_KNIGHT), at(1, SpellSet, MST)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(PENGUIN_SWORD), EndTurn, Attack(PENGUIN_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6300, "{}", s.context());
    assert_eq!(s.count(1, "szone", MST), 1, "no card returned: {}", s.context());
}

/// Penguin Torpedo: "This card can attack directly. When this card inflicts battle damage to your opponent: You can target
/// 1 Level 6 or lower monster your opponent controls; take control of it until the End Phase ... At the end of the Damage
/// Step, if this card attacked: Destroy this card."
#[test]
fn penguin_torpedo_attacks_directly_takes_a_monster_and_is_destroyed() {
    let Some(mut s) = duel(false, &[at(0, Monster, PENGUIN_TORPEDO), at(1, Monster, WARWOLF)]) else { return };
    s.run(&[EndTurn, Attack(PENGUIN_TORPEDO), Chain(PENGUIN_TORPEDO)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7450, "550 damage past Warwolf: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf is controlled: {}", s.context());
    assert_eq!(s.count(0, "grave", PENGUIN_TORPEDO), 1, "Torpedo destroyed itself: {}", s.context());
}

/// Penguin Torpedo (anime): "This card can attack directly. At the end of the Damage Step, if this card attacked
/// directly: Destroy this card." No control change.
#[test]
fn penguin_torpedo_anime_destroys_itself_only() {
    let Some(mut s) = duel(true, &[at(0, Monster, PENGUIN_TORPEDO), at(1, Monster, WARWOLF)]) else { return };
    s.run(&[EndTurn, Attack(PENGUIN_TORPEDO)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7450, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "Warwolf stays: {}", s.context());
    assert_eq!(s.count(0, "grave", PENGUIN_TORPEDO), 1, "{}", s.context());
}

/// Piri Reis Map: "At the start of your Main Phase 1: Add 1 monster with 0 ATK from your Deck to your hand, but halve your LP."
#[test]
fn piri_reis_map_adds_a_0_atk_monster_and_halves_lp() {
    let deck = deck_with(&[CYBER_VALLEY]);
    let Some(mut s) = start(false, [(&deck, &[][..]), (&[KURIBOH; 40], &[][..])], &[at(0, Hand, PIRI_REIS_MAP)]) else { return };
    s.run(&[Activate(PIRI_REIS_MAP)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", CYBER_VALLEY), 1, "{}", s.context());
    assert_eq!(s.lp(0), 4000, "{}", s.context());
}

/// Piri Reis Map (anime): "Add 1 monster with 0 ATK from your Deck to your hand. If you Summon it this turn, halve your LP."
#[test]
fn piri_reis_map_anime_halves_lp_only_on_summon() {
    let deck = deck_with(&[CYBER_VALLEY]);
    let Some(mut s) = start(true, [(&deck, &[][..]), (&[KURIBOH; 40], &[][..])], &[at(0, Hand, PIRI_REIS_MAP)]) else { return };
    s.run(&[Activate(PIRI_REIS_MAP)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", CYBER_VALLEY), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "no loss before it is Summoned: {}", s.context());
    s.run(&[Summon(CYBER_VALLEY)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "halved by the Summon: {}", s.context());
}

/// Plasma Warrior Eitom: "Must be Special Summoned (from your hand or GY) by Tributing 1 Level 7 or higher monster."
#[test]
fn eitom_is_special_summoned_by_tributing_a_level_7() {
    let spawns = [at(0, Hand, EITOM), at(0, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(0, BLUE_EYES)]);
    s.run(&[SpSummon(EITOM)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", EITOM), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", BLUE_EYES), 1, "{}", s.context());
}

/// Eitom can't be Normal Summoned.
#[test]
fn eitom_cannot_be_normal_summoned() {
    let Some(mut s) = duel(false, &[at(0, Hand, EITOM)]) else { return };
    let err = s.run(&[Summon(EITOM)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Eitom: "Once per turn: You can send 1 "Magnet Warrior" ... monster from your Deck to the GY ...; for the rest of this
/// turn, this card's original ATK becomes 1500, also it can attack directly."
#[test]
fn eitom_sends_a_magnet_warrior_to_attack_directly_for_1500() {
    let deck = deck_with(&[GAMMA_MAGNET]);
    let decks = [(&deck[..], &[][..]), (&[KURIBOH; 40], &[][..])];
    let Some(mut s) = start(false, decks, &[at(0, Monster, EITOM), at(1, Monster, WARWOLF)]) else { return };
    s.run(&[EndTurn, Activate(EITOM), Attack(EITOM)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GAMMA_MAGNET), 1, "{}", s.context());
    assert_eq!(s.lp(1), 6500, "1500 direct: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Power Wall: "...when you would take battle damage from an opponent's attacking monster: Send 1 card from the top of your
/// Deck to the GY for every 500 damage (round up) you would take, instead."
#[test]
fn power_wall_mills_a_card_per_500_instead_of_damage() {
    let Some(mut s) = duel(false, &[at(0, SpellSet, POWER_WALL), at(1, Monster, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Chain(POWER_WALL)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no damage taken: {}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 4, "2000 damage is 4 cards: {}", s.context());
}

/// Power Wall (anime): "Send any number of cards from the top of your Deck to the Graveyard, and if you do, you take 100
/// less battle damage for each card sent."
#[test]
fn power_wall_anime_reduces_damage_per_card_sent() {
    let Some(mut s) = duel(true, &[at(0, SpellSet, POWER_WALL), at(1, Monster, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Chain(POWER_WALL)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    let milled = s.count(0, "grave", KURIBOH) as i64;
    assert_eq!(s.lp(0), 8000 - 2000 + 100 * milled, "100 less per card sent ({milled}): {}", s.context());
}

/// Power-Up Adapter: "...equip it to 1 face-up "Morphtronic" monster you control. The equipped monster cannot attack. Then,
/// select 1 face-up monster on the field, except the equipped monster. It gains ATK equal to the ATK of the equipped monster."
#[test]
fn power_up_adapter_equips_stops_the_attack_and_passes_the_atk() {
    let spawns = [at(0, SpellSet, POWER_UP_ADAPTER), at(0, Monster, MORPHTRONIC_STAPLEN), at(0, Monster, KURIBOH)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(POWER_UP_ADAPTER)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, KURIBOH), Some(1700), "300 and 1400: {}", s.context());
    assert_eq!(s.count(0, "szone", POWER_UP_ADAPTER), 1, "it stays as an Equip Card: {}", s.context());
    let err = s.run(&[EndTurn, Attack(MORPHTRONIC_STAPLEN)]).unwrap_err();
    assert!(err.contains("can't attack"), "the equipped monster must not attack: {err}");
}

/// Powerful Rebirth: "Special Summon that target. Increase its Level by 1 and ATK/DEF by 100. When that monster is
/// destroyed, destroy this card."
#[test]
fn powerful_rebirth_revives_a_level_4_stronger_and_follows_its_destruction() {
    let spawns = [at(0, SpellSet, POWERFUL_REBIRTH), at(0, Hand, DARK_HOLE), at(0, Grave, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(POWERFUL_REBIRTH)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2100), "{}", s.context());
    assert_eq!(s.level_of(0, WARWOLF), Some(5), "{}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", POWERFUL_REBIRTH), 1, "the card went with it: {}", s.context());
}

/// Powerful Rebirth (anime): the target is equipped with this card, and "Once per turn, during your Standby Phase: Take
/// 1000 damage."
#[test]
fn powerful_rebirth_anime_hurts_each_standby_phase() {
    let Some(mut s) = duel(true, &[at(0, SpellSet, POWERFUL_REBIRTH), at(0, Grave, WARWOLF)]) else { return };
    s.run(&[Activate(POWERFUL_REBIRTH), EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2100), "{}", s.context());
    assert_eq!(s.lp(0), 7000, "{}", s.context());
}

/// Prima Light: "Tribute 1 EARTH Warrior monster, and if you do, Special Summon 1 LIGHT Warrior monster from your hand or Deck."
#[test]
fn prima_light_tributes_an_earth_warrior_for_a_light_warrior() {
    let spawns = [at(0, Hand, PRIMA_LIGHT), at(0, Hand, CYBER_PRIMA), at(0, Monster, GAIA)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(PRIMA_LIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", CYBER_PRIMA), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", GAIA), 1, "{}", s.context());
}

/// Prima Light: "During your Main Phase: You can banish this card from your GY; add 1 Warrior or Fairy "Cyber" monster from
/// your Deck to your hand."
#[test]
fn prima_light_in_the_graveyard_fetches_a_cyber_monster() {
    let deck = deck_with(&[CYBER_PRIMA]);
    let Some(mut s) = start(false, [(&deck, &[][..]), (&[KURIBOH; 40], &[][..])], &[at(0, Grave, PRIMA_LIGHT)]) else { return };
    s.run(&[Activate(PRIMA_LIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", PRIMA_LIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", CYBER_PRIMA), 1, "{}", s.context());
}

/// Prima Light (anime): "Send 1 face-up "Cyber Tutu" you control to the Graveyard; Special Summon 1 "Cyber Prima" from your hand."
#[test]
fn prima_light_anime_swaps_cyber_tutu_for_cyber_prima() {
    let spawns = [at(0, Hand, PRIMA_LIGHT), at(0, Hand, CYBER_PRIMA), at(0, Monster, CYBER_TUTU)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(PRIMA_LIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", CYBER_PRIMA), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", CYBER_TUTU), 1, "{}", s.context());
}

/// Prinzessin: "If this card is Normal or Special Summoned: You can Special Summon 1 "Pumpkin Carriage" from your hand or Deck".
#[test]
fn prinzessin_summons_pumpkin_carriage() {
    let Some(mut s) = duel(false, &[at(0, Hand, PRINZESSIN), at(0, Hand, PUMPKIN_CARRIAGE)]) else { return };
    s.run(&[Summon(PRINZESSIN), Chain(PRINZESSIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PUMPKIN_CARRIAGE), 1, "{}", s.context());
}

/// Prinzessin: "...then, if "Golden Castle of Stromberg" is in a Field Zone, you can equip 1 "Glass Slippers" from your
/// Deck to this card."
#[test]
fn prinzessin_equips_glass_slippers_with_the_castle() {
    let deck: Vec<u16> = [GLASS_SLIPPERS].into_iter().chain([KURIBOH; 43]).collect();
    let spawns = [at(0, Hand, PRINZESSIN), at(0, Hand, PUMPKIN_CARRIAGE), at(0, Hand, GOLDEN_CASTLE)];
    let Some(mut s) = start(false, [(&deck, &[][..]), (&[KURIBOH; 40], &[][..])], &spawns) else { return };
    s.run(&[Activate(GOLDEN_CASTLE), Summon(PRINZESSIN), Chain(PRINZESSIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PUMPKIN_CARRIAGE), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", GLASS_SLIPPERS), 1, "{}", s.context());
}

/// Prinzessin (anime): "When this card is Summoned: You can Special Summon 1 "Pumpkin Carriage" from your hand, Deck, or
/// Graveyard".
#[test]
fn prinzessin_anime_summons_the_carriage_from_the_graveyard() {
    let Some(mut s) = duel(true, &[at(0, Hand, PRINZESSIN), at(0, Grave, PUMPKIN_CARRIAGE)]) else { return };
    s.run(&[Summon(PRINZESSIN), Chain(PRINZESSIN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", PUMPKIN_CARRIAGE), 1, "{}", s.context());
}

/// Psychic Wave: "If you control a Machine monster: Send 1 "Jinzo" from your hand or Deck to the GY; inflict 600 damage to
/// your opponent."
#[test]
fn psychic_wave_sends_a_jinzo_for_600_damage() {
    let spawns = [at(0, Hand, PSYCHIC_WAVE), at(0, Hand, JINZO), at(0, Monster, GADGET_HAULER)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(PSYCHIC_WAVE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7400, "{}", s.context());
    assert_eq!(s.count(0, "grave", JINZO), 1, "{}", s.context());
}

/// Psychic Wave needs a Machine monster.
#[test]
fn psychic_wave_needs_a_machine() {
    let Some(mut s) = duel(false, &[at(0, Hand, PSYCHIC_WAVE), at(0, Hand, JINZO), at(0, Monster, WARWOLF)]) else { return };
    let err = s.run(&[Activate(PSYCHIC_WAVE)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Psychic Wave: "During your Main Phase, except the turn this card was sent to the GY: You can banish this card from your
/// GY, then target 1 Machine monster in your GY; send 1 "Jinzo" monster from your Deck to the GY, and if you do, add that
/// targeted monster to your hand."
#[test]
fn psychic_wave_in_the_graveyard_trades_a_jinzo_for_a_machine() {
    let deck = deck_with(&[JINZO]);
    let spawns = [at(0, Grave, PSYCHIC_WAVE), at(0, Grave, GADGET_HAULER)];
    let Some(mut s) = start(false, [(&deck, &[][..]), (&[KURIBOH; 40], &[][..])], &spawns) else { return };
    s.run(&[Activate(PSYCHIC_WAVE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", PSYCHIC_WAVE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", JINZO), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", GADGET_HAULER), 1, "{}", s.context());
}

/// Psychic Wave (anime): "Send 1 "Jinzo" from your hand or Deck to the Graveyard while you control an Attack Position
/// "Jinzo - Returner": Inflict damage to your opponent equal to the ATK of a "Jinzo - Returner" you control."
#[test]
fn psychic_wave_anime_burns_for_returners_attack() {
    let spawns = [at(0, Hand, PSYCHIC_WAVE), at(0, Hand, JINZO), at(0, Monster, JINZO_RETURNER)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(PSYCHIC_WAVE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7400, "600, Returner's ATK: {}", s.context());
    assert_eq!(s.count(0, "grave", JINZO), 1, "{}", s.context());
}

/// Pumpkin Carriage: ""Prinzessin" you control can attack directly."
#[test]
fn pumpkin_carriage_lets_prinzessin_attack_directly() {
    let spawns = [at(0, Monster, PRINZESSIN), at(0, Monster, PUMPKIN_CARRIAGE), at(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[EndTurn, Attack(PRINZESSIN)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7700, "{}", s.context());
}

/// Pumpkin Carriage: "Your opponent cannot target "Golden Castle of Stromberg" you control with card effects, also it
/// cannot be destroyed by card effects." Without the Carriage Mystical Space Typhoon can target the Castle; with it the
/// opponent's Mystical Space Typhoon never becomes possible.
#[test]
fn pumpkin_carriage_stops_targeting_the_golden_castle() {
    let plain = [at(0, Hand, GOLDEN_CASTLE), at(1, Hand, MST)];
    let Some(mut s) = duel(false, &plain) else { return };
    s.prefer(&[(0, GOLDEN_CASTLE)]);
    s.run_both([&[Activate(GOLDEN_CASTLE)], &[Activate(MST)]]).unwrap();
    assert_eq!(s.count(0, "grave", GOLDEN_CASTLE), 1, "without the Carriage the Castle falls: {}", s.context());
    let guarded = [at(0, Hand, GOLDEN_CASTLE), at(0, Monster, PUMPKIN_CARRIAGE), at(1, Hand, MST)];
    let Some(mut s) = duel(false, &guarded) else { return };
    let err = s.run_both([&[Activate(GOLDEN_CASTLE)], &[Activate(MST)]]).unwrap_err();
    assert!(err.contains("never became possible") || err.contains("duel is over"), "the Castle can't be targeted: {err}");
}

/// The same Carriage: Heavy Storm, which doesn't target, destroys the Castle without it and can't with it.
#[test]
fn pumpkin_carriage_stops_the_castles_destruction_by_effects() {
    let plain = [at(0, Hand, GOLDEN_CASTLE), at(1, Hand, HEAVY_STORM)];
    let Some(mut s) = duel(false, &plain) else { return };
    s.run_both([&[Activate(GOLDEN_CASTLE)], &[Activate(HEAVY_STORM)]]).unwrap();
    assert_eq!(s.count(0, "grave", GOLDEN_CASTLE), 1, "without the Carriage the Castle falls: {}", s.context());
    let guarded = [at(0, Hand, GOLDEN_CASTLE), at(0, Monster, PUMPKIN_CARRIAGE), at(1, Hand, HEAVY_STORM)];
    let Some(mut s) = duel(false, &guarded) else { return };
    s.run_both([&[Activate(GOLDEN_CASTLE)], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", HEAVY_STORM), 1, "Heavy Storm resolved: {}", s.context());
    assert_eq!(s.count(0, "szone", GOLDEN_CASTLE), 1, "the Castle stays: {}", s.context());
}

/// Pumpkin Carriage (anime): ""Prinzessin" can attack directly."
#[test]
fn pumpkin_carriage_anime_lets_prinzessin_attack_directly() {
    let spawns = [at(0, Monster, PRINZESSIN), at(0, Monster, PUMPKIN_CARRIAGE), at(1, Monster, WARWOLF)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[EndTurn, Attack(PRINZESSIN)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7700, "{}", s.context());
}

/// Quick Summon: "Immediately after this effect resolves, Normal Summon/Set 1 monster. When you do, if your opponent
/// controls a monster, you can Normal Summon a Level 5 or higher monster without Tributing."
#[test]
fn quick_summon_summons_a_high_level_monster_without_tributes() {
    let spawns = [at(0, Hand, QUICK_SUMMON), at(0, Hand, BLUE_EYES), at(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(QUICK_SUMMON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BLUE_EYES), 1, "{}", s.context());
}

/// Quick Summon without an opposing monster does not waive the Tributes: Blue-Eyes (Level 8) can't be summoned alone.
#[test]
fn quick_summon_needs_an_opposing_monster_to_skip_tributes() {
    let Some(mut s) = duel(false, &[at(0, Hand, QUICK_SUMMON), at(0, Hand, BLUE_EYES)]) else { return };
    s.run(&[Activate(QUICK_SUMMON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BLUE_EYES), 0, "{}", s.context());
    assert_eq!(s.count(0, "hand", BLUE_EYES), 1, "{}", s.context());
}

/// Quick Summon: "During your Main Phase, if this card is in your GY, except the turn it was sent there: You can banish it;
/// add 1 Level 5 or higher monster that can be Normal Summoned/Set from your Deck or GY to your hand, then immediately after
/// this effect resolves, Tribute Summon it."
#[test]
fn quick_summon_in_the_graveyard_fetches_and_tribute_summons() {
    let deck = deck_with(&[SUMMONED_SKULL]);
    let spawns = [at(0, Grave, QUICK_SUMMON), at(0, Monster, KURIBOH)];
    let Some(mut s) = start(false, [(&deck, &[][..]), (&[KURIBOH; 40], &[][..])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(QUICK_SUMMON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", QUICK_SUMMON), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", SUMMONED_SKULL), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "the Tribute: {}", s.context());
}

/// Rainbow Bridge Bifrost: "Banish 3 Level 10 or higher monsters from your Extra Deck, including a LIGHT Synchro Monster;
/// add 1 Field Spell from your Deck to your hand." (The game has no Level 10+ LIGHT Synchro Monster, so the positive
/// case can't be built from cards a player holds: this checks that three Level 10+ Fusions don't pay the cost.)
#[test]
fn bifrost_needs_a_light_synchro_in_the_extra_deck() {
    let deck = deck_with(&[YAMI]);
    let extra = [BLUE_EYES_ULTIMATE, CYBER_END, BLUE_EYES_ULTIMATE];
    let Some(mut s) = start(false, [(&deck, &extra[..]), (&[KURIBOH; 40], &[][..])], &[at(0, Hand, BIFROST)]) else { return };
    let err = s.run(&[Activate(BIFROST)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "removed", BLUE_EYES_ULTIMATE) + s.count(0, "removed", CYBER_END), 0, "{}", s.context());
}

/// Rainbow Bridge Bifrost (anime): "Activate only while you control a face-up "Valkyrie" monster. This turn, 1 "Valkyrie"
/// monster you control gains 500 ATK for each of your opponent's banished monsters."
#[test]
fn bifrost_anime_raises_a_valkyrie_per_banished_monster() {
    let spawns = [at(0, Hand, BIFROST), at(0, Monster, VALKYRIE_DRITTE), at(1, Banished, WARWOLF), at(1, Banished, KURIBOH)];
    // Valkyrie Dritte has its own ATK gain per banished monster: the control duel without Bifrost gives its baseline.
    let Some(control) = duel(true, &spawns[1..]) else { return };
    let base = control.attack_of(0, VALKYRIE_DRITTE).unwrap();
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(BIFROST)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, VALKYRIE_DRITTE), Some(base + 1000), "2 x 500 more than {base}: {}", s.context());
}

/// Raphion, the Timelord: "If you control no monsters, you can Normal Summon this card without Tributing. Cannot be
/// destroyed by battle or card effects."
#[test]
fn raphion_is_summoned_without_tributes_and_survives_dark_hole() {
    let Some(mut s) = duel(false, &[at(0, Hand, RAPHION), at(1, Hand, DARK_HOLE)]) else { return };
    s.run_both([&[Summon(RAPHION)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", RAPHION), 1, "{}", s.context());
}

/// Raphion: "You take no battle damage from attacks involving this card. At the end of the Battle Phase, if this card
/// battled: Inflict damage to your opponent equal to the ATK of 1 monster your opponent controls that battled this card."
#[test]
fn raphion_reflects_the_attackers_atk_and_takes_no_damage() {
    let Some(mut s) = duel(false, &[at(0, Hand, RAPHION), at(1, Monster, WARWOLF)]) else { return };
    s.run_both([&[Summon(RAPHION)], &[Attack(WARWOLF), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.lp(1), 6000, "Warwolf's 2000 ATK: {}", s.context());
    assert_eq!(s.count(0, "grave", RAPHION), 0, "not destroyed by battle (it is shuffled away at its Standby): {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Raphion: "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn raphion_shuffles_itself_back_at_the_standby_phase() {
    let Some(mut s) = duel(false, &[at(0, Hand, RAPHION)]) else { return };
    s.run(&[Summon(RAPHION), EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", RAPHION), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", RAPHION), 0, "{}", s.context());
    assert_eq!(s.count(0, "hand", RAPHION), 0, "{}", s.context());
}

/// Raphion (anime): "At the end of the Damage Step, if this card battled: Return any opponent's monster that battled this
/// card to the hand, and if you do, inflict damage to your opponent equal to the ATK the returned monster had on the field."
#[test]
fn raphion_anime_returns_the_attacker_and_burns_for_its_atk() {
    let spawns = [at(0, Hand, RAPHION), at(0, Monster, KURIBOH), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[Summon(RAPHION)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "hand", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 6000, "{}", s.context());
}

/// Reaper Scythe - Dreadscythe: "Equip only to "Guardian Dreadscythe". It gains 500 ATK for each monster in the GYs."
#[test]
fn reaper_scythe_gains_500_per_monster_in_the_graveyards() {
    let spawns = [
        at(0, Hand, REAPER_SCYTHE),
        at(0, Monster, DREADSCYTHE),
        at(0, Grave, KURIBOH),
        at(0, Grave, KURIBOH),
        at(1, Grave, WARWOLF),
    ];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(REAPER_SCYTHE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DREADSCYTHE), Some(4000), "2500 and 3 x 500: {}", s.context());
}

/// Reaper Scythe equips only to Guardian Dreadscythe.
#[test]
fn reaper_scythe_does_not_equip_another_monster() {
    let Some(mut s) = duel(false, &[at(0, Hand, REAPER_SCYTHE), at(0, Monster, WARWOLF)]) else { return };
    let err = s.run(&[Activate(REAPER_SCYTHE)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Rebellion: "During either player's Battle Phase: Target 1 monster your opponent controls; take control of it until the
/// end of the Battle Phase".
#[test]
fn rebellion_takes_the_attacker_for_the_battle_phase() {
    let spawns = [at(0, SpellSet, REBELLION), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run_both([&[ChainOnAttack(REBELLION)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf is ours for the Battle Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", REBELLION), 1, "{}", s.context());
}

/// Rebellion (anime): "Target 1 face-up monster; It attacks its controller directly."
#[test]
fn rebellion_anime_makes_a_monster_attack_its_controller() {
    let Some(mut s) = duel(true, &[at(0, Hand, REBELLION), at(1, Monster, WARWOLF)]) else { return };
    s.run(&[Activate(REBELLION)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6000, "Warwolf hit its controller for 2000: {}", s.context());
}

/// Rebirth Judgment: "Once per turn: You can declare 1 Monster Type; monsters in the GYs become that Type for the rest of
/// this turn". The card is a Continuous Trap that activates on its own.
#[test]
fn rebirth_judgment_activates_and_stays() {
    let Some(mut s) = duel(false, &[at(0, SpellSet, REBIRTH_JUDGMENT), at(0, Grave, WARWOLF)]) else { return };
    s.run(&[Activate(REBIRTH_JUDGMENT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", REBIRTH_JUDGMENT), 1, "it stays on the field: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// Red Cocoon: "Target 1 Dragon-Type Synchro Monster you control; equip this card to it."
#[test]
fn red_cocoon_equips_to_a_dragon_synchro() {
    let Some(mut s) = duel(false, &[at(0, SpellSet, RED_COCOON), at(0, Monster, STARDUST)]) else { return };
    s.run(&[Activate(RED_COCOON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", RED_COCOON), 1, "{}", s.context());
}

/// Red Cocoon equips only to a Dragon Synchro Monster.
#[test]
fn red_cocoon_does_not_equip_a_non_synchro() {
    let Some(mut s) = duel(false, &[at(0, SpellSet, RED_COCOON), at(0, Monster, WARWOLF)]) else { return };
    let err = s.run(&[Activate(RED_COCOON)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Red Cocoon: "During the End Phase, if this card is in the Graveyard because it was sent there this turn: You can target
/// 1 "Red Dragon Archfiend" in your Graveyard; Special Summon it." Red Cocoon on Red Dragon Archfiend, both sent to the
/// Graveyard by Dark Hole, and the Archfiend comes back in the End Phase. (A Synchro Monster put straight into the
/// Graveyard or onto the field was never properly summoned, so nothing could bring it back: it is Synchro Summoned
/// from Power Tool Dragon and Turbo Synchron.)
#[test]
fn red_cocoon_revives_red_dragon_archfiend_from_the_graveyard() {
    let spawns = [at(0, SpellSet, RED_COCOON), at(0, Monster, POWER_TOOL), at(0, Monster, TURBO_SYNCHRON), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = start(false, [(&[KURIBOH; 40], &[RED_DRAGON_ARCHFIEND][..]), (&[KURIBOH; 40], &[][..])], &spawns) else { return };
    s.prefer(&[(0, RED_DRAGON_ARCHFIEND)]);
    s.run(&[SpSummon(RED_DRAGON_ARCHFIEND), Activate(RED_COCOON), Activate(DARK_HOLE), EndTurn, Chain(RED_COCOON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", RED_COCOON), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", RED_DRAGON_ARCHFIEND), 1, "{}", s.context());
}

/// Red Cocoon (anime): "After activation, equip this card to a face-up "Red Nova Dragon" you control" (not Stardust Dragon).
#[test]
fn red_cocoon_anime_does_not_equip_stardust() {
    let Some(mut s) = duel(true, &[at(0, SpellSet, RED_COCOON), at(0, Monster, STARDUST)]) else { return };
    let err = s.run(&[Activate(RED_COCOON)]).unwrap_err();
    assert!(err.contains("never became possible") || err.contains("can't"), "{err}");
}

/// Red Dragon Vase: "If you control a "Red Dragon Archfiend": Draw 2 cards. You cannot Normal or Special Summon monsters
/// the turn you activate this card, and until the end of your opponent's turn after this card is activated."
#[test]
fn red_dragon_vase_draws_2_and_forbids_summons() {
    let spawns = [at(0, Hand, RED_DRAGON_VASE), at(0, Hand, WARWOLF), at(0, Monster, RED_DRAGON_ARCHFIEND)];
    let Some(mut s) = duel(false, &spawns) else { return };
    let before = s.count(0, "hand", KURIBOH);
    s.run(&[Activate(RED_DRAGON_VASE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", KURIBOH), before + 2, "drew 2: {}", s.context());
    let err = s.run(&[Summon(WARWOLF)]).unwrap_err();
    assert!(err.contains("never became possible"), "no Normal Summon after the Vase: {err}");
}

/// Red Dragon Vase needs a Red Dragon Archfiend.
#[test]
fn red_dragon_vase_needs_the_archfiend() {
    let Some(mut s) = duel(false, &[at(0, Hand, RED_DRAGON_VASE), at(0, Monster, WARWOLF)]) else { return };
    let err = s.run(&[Activate(RED_DRAGON_VASE)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Red Dragon Vase (anime): "While you control a face-up "Red Dragon Archfiend": Draw 2 cards." No Summon restriction.
#[test]
fn red_dragon_vase_anime_draws_2_and_allows_summons() {
    let spawns = [at(0, Hand, RED_DRAGON_VASE), at(0, Hand, WARWOLF), at(0, Monster, RED_DRAGON_ARCHFIEND)];
    let Some(mut s) = duel(true, &spawns) else { return };
    let before = s.count(0, "hand", KURIBOH);
    s.run(&[Activate(RED_DRAGON_VASE), Summon(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", KURIBOH), before + 2, "{}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Red Nova: "If a Level 8 or higher Dragon Synchro Monster is on the field, you can Special Summon this card (from your hand)."
#[test]
fn red_nova_special_summons_beside_a_dragon_synchro() {
    let Some(mut s) = duel(false, &[at(0, Hand, RED_NOVA), at(0, Monster, STARDUST)]) else { return };
    s.run(&[SpSummon(RED_NOVA)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", RED_NOVA), 1, "{}", s.context());
}

/// Red Nova can't be Special Summoned without such a monster on the field.
#[test]
fn red_nova_needs_a_dragon_synchro_on_the_field() {
    let Some(mut s) = duel(false, &[at(0, Hand, RED_NOVA), at(0, Monster, WARWOLF)]) else { return };
    let err = s.run(&[SpSummon(RED_NOVA)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Red-Eyes Black Dragon Sword: "Must be Special Summoned with "The Claw of Hermos", using a Dragon monster. If this card is
/// Special Summoned: Target 1 other face-up monster on the field; equip this card to it. It gains 1000 ATK, and 500 ATK/DEF
/// for each Dragon monster on the field and in the GYs."
#[test]
fn red_eyes_sword_is_summoned_by_the_claw_and_equips() {
    let spawns = [at(0, Hand, CLAW_OF_HERMOS), at(0, Hand, RED_EYES), at(0, Monster, WARWOLF)];
    let decks = [(&[KURIBOH; 40][..], &[RED_EYES_SWORD][..]), (&[KURIBOH; 40], &[][..])];
    let Some(mut s) = start(false, decks, &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(CLAW_OF_HERMOS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", RED_EYES), 1, "the Dragon was sent: {}", s.context());
    assert_eq!(s.count(0, "szone", RED_EYES_SWORD), 1, "equipped to Warwolf: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000 + 1000 + 500), "1000 and one Dragon in the GY (it is an Equip now): {}", s.context());
}

/// Red-Eyes Black Dragon Sword (anime): "The equipped monster gains 1000 ATK. It also gains 500 ATK for each Dragon-Type
/// monster on the field."
#[test]
fn red_eyes_sword_anime_counts_dragons_on_the_field() {
    let spawns = [at(0, Hand, CLAW_OF_HERMOS), at(0, Monster, RED_EYES), at(0, Monster, WARWOLF)];
    let decks = [(&[KURIBOH; 40][..], &[RED_EYES_SWORD][..]), (&[KURIBOH; 40], &[][..])];
    let Some(mut s) = start(true, decks, &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(CLAW_OF_HERMOS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", RED_EYES_SWORD), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000 + 1000), "no Dragon on the field: {}", s.context());
}

/// Red-Eyes Burn: "If a face-up "Red-Eyes" monster(s) you control is destroyed by battle or card effect: Target 1 of those
/// monsters; both players take damage equal to its original ATK."
#[test]
fn red_eyes_burn_hurts_both_players_for_the_original_atk() {
    let spawns = [at(0, SpellSet, RED_EYES_BURN), at(0, Hand, DARK_HOLE), at(0, Monster, RED_EYES)];
    let Some(mut s) = duel(false, &spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(RED_EYES_BURN)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 5600, "{}", s.context());
    assert_eq!(s.lp(1), 5600, "{}", s.context());
}

/// Red-Eyes Burn (anime): "When a "Red-Eyes" monster you control is destroyed: Inflict damage to both players equal to its ATK."
#[test]
fn red_eyes_burn_anime_hurts_both_players() {
    let spawns = [at(0, SpellSet, RED_EYES_BURN), at(0, Hand, DARK_HOLE), at(0, Monster, RED_EYES)];
    let Some(mut s) = duel(true, &spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(RED_EYES_BURN)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 5600, "{}", s.context());
    assert_eq!(s.lp(1), 5600, "{}", s.context());
}
