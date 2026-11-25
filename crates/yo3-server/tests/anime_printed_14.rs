//! Scenario tests for the fourteenth group of the anime's printed cards (rows 4848-5205 of engine/added/cards.tsv): the
//! T.G. cards, the Legendary Dragon cards (The Claw of Hermos, The Eye of Timaeus, The Fang of Critias and what they
//! summon), the Toon support, and a few others from "T.G. Hyper Librarian" to "Totem Pole". Each through the real core
//! with both seats' client models checked after every step (`duelcheck::scenario`). Texts: the printed ones (anime cards
//! off) from the pinned official database; an anime version is tested with anime cards on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg};

// The added cards of this group (client ids from the group list).
const HYPER_LIBRARIAN: u16 = 9554; // T.G. Hyper Librarian
const JET_FALCON: u16 = 9489; // T.G. Jet Falcon
const METAL_SKELETON: u16 = 14561; // T.G. Metal Skeleton
const POWER_GLADIATOR: u16 = 9512; // T.G. Power Gladiator
const RECIPRO: u16 = 9510; // T.G. Recipro Dragonfly
const RUSH_RHINO: u16 = 9492; // T.G. Rush Rhino
const TG_STRIKER: u16 = 9488; // T.G. Striker
const TG_WARWOLF: u16 = 9491; // T.G. Warwolf
const WONDER_MAGICIAN: u16 = 9511; // T.G. Wonder Magician
const CLAW: u16 = 11892; // The Claw of Hermos
const DUKE: u16 = 5052; // The Duke of Demise
const EYE: u16 = 10326; // The Eye of Timaeus
const FANG: u16 = 11885; // The Fang of Critias
const MASK_OF_REMNANTS: u16 = 5098; // The Mask of Remnants
const TIES: u16 = 6154; // Ties of the Brethren
const KNIGHT_OF_DESTINY: u16 = 11880; // Timaeus the Knight of Destiny
const TIME_MAGIC_HAMMER: u16 = 11888; // Time Magic Hammer
const TIME_MAIDEN: u16 = 13161; // Time Maiden
const TIME_STREAM: u16 = 15370; // Time Stream
const TOON_BRIEFCASE: u16 = 11906; // Toon Briefcase
const TOON_KINGDOM: u16 = 11901; // Toon Kingdom
const TOON_MASK: u16 = 11905; // Toon Mask
const TOON_ROLLBACK: u16 = 11902; // Toon Rollback
const TOTEM_POLE: u16 = 5013; // Totem Pole
// Other added cards these play with.
const BLADE_BLASTER: u16 = 9513; // T.G. Blade Blaster
const LEGENDARY_TIMAEUS: u16 = 11149; // Legendary Knight Timaeus
const LEGENDARY_CRITIAS: u16 = 11881; // Legendary Knight Critias
const LEGENDARY_HERMOS: u16 = 11887; // Legendary Knight Hermos
const AMULET_DRAGON: u16 = 10325;
const MIRROR_FORCE_DRAGON: u16 = 11884;
const RED_EYES_SWORD: u16 = 11891; // Red-Eyes Black Dragon Sword
const ROCKET_HERMOS_CANNON: u16 = 11889;
const DES_GARDIUS: u16 = 5097; // Masked Beast Des Gardius
const ZAPHION: u16 = 13158; // Zaphion, the Timelord
const SKULLGAR: u16 = 15368; // Fossil Dragon Skullgar
const SKULL_KNIGHT: u16 = 15366; // Fossil Warrior Skull Knight
const SKULL_KING: u16 = 7253; // Fossil Warrior Skull King
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const BLUE_EYES: u16 = 4007;
const FOSSIL_FUSION: u16 = 15369;
const BARREL_ROCK: u16 = 4461; // a Rock monster, 1000 ATK
const MIND_CONTROL: u16 = 4817;
const GEMINI_ELF: u16 = 4538;
const ZOMBIE_MASTER: u16 = 7215; // a Level 4 Zombie
const GAZELLE: u16 = 4753; // a Level 4 EARTH Beast
const PREVENT_RAT: u16 = 4261; // the same
const MON_LARVAS: u16 = 4593; // the same
const SOUL_TIGER: u16 = 5813; // the same
const GAIA: u16 = 4044; // Gaia The Fierce Knight, a Warrior
const TIME_WIZARD: u16 = 4022;
const SEVEN_SWORDS: u16 = 9194; // Seven Swords Warrior, a Level 7 Synchro Monster
const DARK_MAGICIAN: u16 = 4041;
const MIRROR_FORCE: u16 = 4887;
const TOON_WORLD: u16 = 4798;
const TOON_MERMAID: u16 = 4734; // Level 4
const TOON_GEMINI_ELF: u16 = 5476; // Level 4, 1900 ATK
const TOON_SUMMONED_SKULL: u16 = 4735; // Level 6
const NEGATE_ATTACK: u16 = 4804;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn duel(anime: bool, main: [&[u16]; 2], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(main[0], extra[0]), (main[1], extra[1])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// A duel with Kuriboh decks, team 0's Extra Deck `extra` and `spawns` in place; anime cards off.
fn printed(extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    duel(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [extra, &[]], spawns)
}

/// The same with anime cards on.
fn anime(extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    duel(true, [&[KURIBOH; 40], &[KURIBOH; 40]], [extra, &[]], spawns)
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

/// The cards in a team's hand.
fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// A Main Deck of Kuriboh with `bottom` cards at its far end (never drawn: the opening hand and draws come from the other end).
fn deck_with(bottom: &[u16]) -> Vec<u16> {
    bottom.iter().copied().chain(std::iter::repeat_n(KURIBOH, 40 - bottom.len())).collect()
}

/// T.G. Hyper Librarian: "If a monster is Synchro Summoned, while this monster is on the field: Draw 1 card." Another
/// Synchro Monster (T.G. Power Gladiator, from T.G. Striker and T.G. Warwolf) summoned while it stays: one card.
#[test]
fn hyper_librarian_draws_when_a_monster_is_synchro_summoned() {
    let spawns = [at(0, Monster, HYPER_LIBRARIAN), at(0, Monster, TG_STRIKER), at(0, Monster, TG_WARWOLF)];
    let Some(mut s) = printed(&[POWER_GLADIATOR], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[SpSummon(POWER_GLADIATOR)]).unwrap();
    clean(&s);
    assert!(s.monster(0, POWER_GLADIATOR).is_some(), "Power Gladiator was Synchro Summoned: {}", s.context());
    assert_eq!(hand(&s, 0), before + 1, "Hyper Librarian drew a card: {}", s.context());
}

/// "If a monster is Synchro Summoned": the opponent's too. Team 1 Synchro Summons T.G. Power Gladiator on its turn, team 0
/// (whose Hyper Librarian is on the field) draws.
#[test]
fn hyper_librarian_draws_for_the_opponents_synchro_summon() {
    let spawns = [at(0, Monster, HYPER_LIBRARIAN), at(1, Monster, TG_STRIKER), at(1, Monster, TG_WARWOLF)];
    let Some(mut s) = duel(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[POWER_GLADIATOR]], &spawns) else { return };
    let before = hand(&s, 0);
    s.run_both([&[EndTurn], &[SpSummon(POWER_GLADIATOR)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, POWER_GLADIATOR).is_some(), "team 1 Synchro Summoned: {}", s.context());
    assert_eq!(hand(&s, 0), before + 1, "Hyper Librarian drew for the opponent's summon: {}", s.context());
}

/// T.G. Jet Falcon: "If this card is sent to the Graveyard as a Synchro Material: Inflict 500 damage to your opponent."
/// T.G. Jet Falcon (Level 3 Tuner) and T.G. Metal Skeleton (2) make Hyper Librarian (5).
#[test]
fn jet_falcon_burns_when_used_as_synchro_material() {
    let spawns = [at(0, Monster, JET_FALCON), at(0, Monster, METAL_SKELETON)];
    let Some(mut s) = printed(&[HYPER_LIBRARIAN], &spawns) else { return };
    s.run(&[SpSummon(HYPER_LIBRARIAN)]).unwrap();
    clean(&s);
    assert!(s.monster(0, HYPER_LIBRARIAN).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", JET_FALCON), 1, "Jet Falcon was the material: {}", s.context());
    assert_eq!(s.lp(1), 7500, "500 damage: {}", s.context());
}

/// The printed Jet Falcon does nothing else when it is destroyed.
#[test]
fn jet_falcon_destroyed_does_not_search_by_the_printed_text() {
    let spawns = [at(0, Monster, JET_FALCON), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = duel(false, [&deck_with(&[METAL_SKELETON]), &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", JET_FALCON), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", METAL_SKELETON), 0, "{}", s.context());
}

/// Anime Jet Falcon: "When this card is destroyed, you can add 1 "T.G." monster from your Deck to your hand." Dark Hole
/// destroys it and T.G. Metal Skeleton comes to the hand; the Synchro Material damage is the same.
#[test]
fn anime_jet_falcon_adds_a_tg_monster_when_destroyed() {
    let spawns = [at(0, Monster, JET_FALCON), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = duel(true, [&deck_with(&[METAL_SKELETON]), &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(JET_FALCON)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", JET_FALCON), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", METAL_SKELETON), 1, "a T.G. monster was added: {}", s.context());
}

#[test]
fn anime_jet_falcon_burns_when_used_as_synchro_material() {
    let spawns = [at(0, Monster, JET_FALCON), at(0, Monster, METAL_SKELETON)];
    let Some(mut s) = anime(&[HYPER_LIBRARIAN], &spawns) else { return };
    s.run(&[SpSummon(HYPER_LIBRARIAN)]).unwrap();
    clean(&s);
    assert!(s.monster(0, HYPER_LIBRARIAN).is_some(), "{}", s.context());
    assert_eq!(s.lp(1), 7500, "500 damage: {}", s.context());
}

/// T.G. Metal Skeleton: "If a monster(s) on the field is destroyed by battle or an opponent's card effect: You can Special
/// Summon this card from your hand." Blue-Eyes destroys Gene-Warped Warwolf in battle; the Skeleton comes.
#[test]
fn metal_skeleton_comes_when_a_monster_is_destroyed() {
    let spawns = [at(0, Monster, WARWOLF), at(0, Hand, METAL_SKELETON), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, Chain(METAL_SKELETON)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, METAL_SKELETON).is_some(), "the Skeleton came: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// "If a "T.G." monster(s) you control would be destroyed by battle or card effect, you can banish this card from your
/// field or GY instead." T.G. Warwolf is attacked by Blue-Eyes: the Skeleton (on the field) is banished and Warwolf stays.
#[test]
fn metal_skeleton_is_banished_instead_of_a_tg_monster() {
    let spawns = [at(0, Monster, TG_WARWOLF), at(0, Monster, METAL_SKELETON), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(METAL_SKELETON)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, TG_WARWOLF).is_some(), "T.G. Warwolf was not destroyed: {}", s.context());
    assert_eq!(s.count(0, "removed", METAL_SKELETON), 1, "the Skeleton was banished instead: {}", s.context());
}

/// The same from the Graveyard, against Dark Hole.
#[test]
fn metal_skeleton_banishes_from_the_graveyard_against_dark_hole() {
    let spawns = [at(0, Monster, TG_WARWOLF), at(0, Grave, METAL_SKELETON), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(METAL_SKELETON)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, TG_WARWOLF).is_some(), "T.G. Warwolf was not destroyed: {}", s.context());
    assert_eq!(s.count(0, "removed", METAL_SKELETON), 1, "{}", s.context());
}

/// T.G. Power Gladiator: "If this card attacks a Defense Position monster, inflict piercing battle damage to your
/// opponent." 2300 ATK against a Kuriboh in Defense (200 DEF): 2100 damage.
#[test]
fn power_gladiator_pierces() {
    let spawns = [at(0, Monster, POWER_GLADIATOR), at(1, MonsterDefense, KURIBOH)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(POWER_GLADIATOR)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2100, "{}", s.context());
    assert_eq!(s.count(1, "grave", KURIBOH), 1, "{}", s.context());
}

/// "If this card on the field is destroyed: Draw 1 card." Dark Hole destroys it.
#[test]
fn power_gladiator_draws_when_destroyed() {
    let spawns = [at(0, Monster, POWER_GLADIATOR), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    let before = hand(&s, 0);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", POWER_GLADIATOR), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before + 1, "{}", s.context());
}

/// T.G. Recipro Dragonfly: "Once per turn: You can target 1 other "T.G." Synchro Monster you control; send it to the
/// Graveyard, then, if all the Synchro Monsters that were used for its Synchro Summon are in your Graveyard, you can
/// Special Summon all of them." T.G. Wonder Magician (a Tuner) and T.G. Power Gladiator, themselves Synchro Summoned, make T.G. Blade Blaster (10); the
/// Dragonfly sends it away and both come back.
#[test]
fn recipro_dragonfly_returns_the_synchro_materials() {
    let spawns = [at(0, Monster, TG_STRIKER), at(0, Monster, TG_WARWOLF), at(0, Monster, JET_FALCON), at(0, Monster, METAL_SKELETON), at(0, Monster, RECIPRO)];
    let Some(mut s) = printed(&[WONDER_MAGICIAN, POWER_GLADIATOR, BLADE_BLASTER], &spawns) else { return };
    s.run(&[SpSummon(WONDER_MAGICIAN), SpSummon(POWER_GLADIATOR), SpSummon(BLADE_BLASTER)]).unwrap();
    clean(&s);
    assert!(s.monster(0, BLADE_BLASTER).is_some() && s.monster(0, WONDER_MAGICIAN).is_none(), "{}", s.context());
    s.run(&[Activate(RECIPRO)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BLADE_BLASTER), 1, "Blade Blaster was sent to the Graveyard: {}", s.context());
    assert!(s.monster(0, WONDER_MAGICIAN).is_some() && s.monster(0, POWER_GLADIATOR).is_some(), "both materials came back: {}", s.context());
}

/// Anime Recipro Dragonfly: "target 1 Synchro Monster you control; Send it to the Graveyard, OR return it to the Extra
/// Deck, then, if all of the Synchro Monsters that were used for the Synchro Summon of that monster are in your Graveyard,
/// you can Special Summon them." The same board, anime cards on.
#[test]
fn anime_recipro_dragonfly_returns_the_synchro_materials() {
    let spawns = [at(0, Monster, TG_STRIKER), at(0, Monster, TG_WARWOLF), at(0, Monster, JET_FALCON), at(0, Monster, METAL_SKELETON), at(0, Monster, RECIPRO)];
    let Some(mut s) = anime(&[WONDER_MAGICIAN, POWER_GLADIATOR, BLADE_BLASTER], &spawns) else { return };
    s.prefer(&[(0, BLADE_BLASTER)]); // the anime effect may target the Dragonfly itself
    s.run(&[SpSummon(WONDER_MAGICIAN), SpSummon(POWER_GLADIATOR), SpSummon(BLADE_BLASTER)]).unwrap();
    s.run(&[Activate(RECIPRO)]).unwrap();
    clean(&s);
    assert!(s.monster(0, BLADE_BLASTER).is_none(), "Blade Blaster left the field: {}", s.context());
    assert!(s.monster(0, WONDER_MAGICIAN).is_some() && s.monster(0, POWER_GLADIATOR).is_some(), "both materials came back: {}", s.context());
}

/// Blue-Eyes (3000 ATK) destroys `card` in battle on the opponent's turn; the card's End Phase search is taken: a "T.G."
/// monster (T.G. Metal Skeleton, at the bottom of the Deck) comes to the hand.
fn searches_after_destruction(anime_on: bool, card: u16) -> Scenario {
    let spawns = [at(0, Monster, card), at(1, Monster, BLUE_EYES)];
    let mains = [&deck_with(&[METAL_SKELETON])[..], &[KURIBOH; 40][..]];
    let mut s = duel(anime_on, mains, [&[], &[]], &spawns).expect("engine");
    s.run_both([&[EndTurn, Chain(card)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", card), 1, "{}", s.context());
    s
}

/// T.G. Rush Rhino: "If this card attacks, it gains 400 ATK during the Damage Step only." It attacks a Kuriboh (300 ATK):
/// 1600 + 400 - 300 = 1700 damage.
#[test]
fn rush_rhino_gains_400_atk_in_the_damage_step() {
    let spawns = [at(0, Monster, RUSH_RHINO), at(1, Monster, KURIBOH)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run(&[EndTurn, Attack(RUSH_RHINO)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1700, "{}", s.context());
    assert_eq!(s.attack_of(0, RUSH_RHINO), Some(1600), "the bonus is for the Damage Step only: {}", s.context());
}

/// "Once per turn, during the End Phase, if this card is in the GY because it was destroyed on the field and sent there
/// this turn: You can add 1 "T.G." monster from your Deck to your hand, except "T.G. Rush Rhino"."
#[test]
fn rush_rhino_searches_in_the_end_phase_after_being_destroyed() {
    let s = searches_after_destruction(false, RUSH_RHINO);
    assert_eq!(s.count(0, "hand", METAL_SKELETON), 1, "a T.G. monster was added: {}", s.context());
}

/// Anime Rush Rhino: "If this card is destroyed, you can add 1 "T.G." monster from your Deck to your hand."
#[test]
fn anime_rush_rhino_searches_when_destroyed() {
    let s = searches_after_destruction(true, RUSH_RHINO);
    assert_eq!(s.count(0, "hand", METAL_SKELETON), 1, "a T.G. monster was added: {}", s.context());
}

/// T.G. Striker: "If only your opponent controls a monster, you can Special Summon this card (from your hand)."
#[test]
fn striker_comes_when_only_the_opponent_controls_a_monster() {
    let spawns = [at(0, Hand, TG_STRIKER), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run(&[SpSummon(TG_STRIKER)]).unwrap();
    clean(&s);
    assert!(s.monster(0, TG_STRIKER).is_some(), "{}", s.context());
}

/// ... and not when its controller has a monster too, or the opponent none.
#[test]
fn striker_cannot_come_when_its_controller_has_a_monster() {
    let spawns = [at(0, Hand, TG_STRIKER), at(0, Monster, WARWOLF), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    assert!(s.run(&[SpSummon(TG_STRIKER)]).is_err(), "{}", s.context());
    let Some(mut s) = printed(&[], &[at(0, Hand, TG_STRIKER)]) else { return };
    assert!(s.run(&[SpSummon(TG_STRIKER)]).is_err(), "{}", s.context());
}

/// "Once per turn, during the End Phase, if this card is in the GY because it was destroyed on the field and sent there
/// this turn: You can add 1 "T.G." monster from your Deck to your hand, except "T.G. Striker"."
#[test]
fn striker_searches_in_the_end_phase_after_being_destroyed() {
    let s = searches_after_destruction(false, TG_STRIKER);
    assert_eq!(s.count(0, "hand", METAL_SKELETON), 1, "a T.G. monster was added: {}", s.context());
}

/// T.G. Warwolf: "When a Level 4 or lower monster(s) is Special Summoned (except during the Damage Step): You can Special
/// Summon this card from your hand." Monster Reborn brings back Gene-Warped Warwolf (Level 4); T.G. Warwolf follows.
#[test]
fn tg_warwolf_comes_when_a_low_level_monster_is_special_summoned() {
    let spawns = [at(0, Hand, MONSTER_REBORN), at(0, Grave, WARWOLF), at(0, Hand, TG_WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run(&[Activate(MONSTER_REBORN), Chain(TG_WARWOLF)]).unwrap();
    clean(&s);
    assert!(s.monster(0, WARWOLF).is_some() && s.monster(0, TG_WARWOLF).is_some(), "both came: {}", s.context());
}

/// The printed text is "a Level 4 or lower monster(s)": the opponent's Special Summon counts as well.
#[test]
fn tg_warwolf_comes_when_the_opponent_special_summons() {
    let spawns = [at(0, Hand, TG_WARWOLF), at(1, Hand, MONSTER_REBORN), at(1, Grave, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, WARWOLF)]); // the End Phase discard (seven cards) keeps T.G. Warwolf
    s.run_both([&[EndTurn, Chain(TG_WARWOLF)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, TG_WARWOLF).is_some(), "{}", s.context());
}

/// Anime T.G. Warwolf: "When you Special Summon a Level 4 or lower monster, you can Special Summon this card from your
/// hand": the opponent's Special Summon does not bring it, the owner's does.
#[test]
fn anime_tg_warwolf_answers_only_its_owners_special_summons() {
    let spawns = [at(0, Hand, TG_WARWOLF), at(1, Hand, MONSTER_REBORN), at(1, Grave, WARWOLF)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, WARWOLF)]); // the End Phase discard (seven cards) keeps T.G. Warwolf
    s.run_both([&[EndTurn], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert_eq!(s.state()[1]["mzone"].as_array().map_or(0, |m| m.iter().filter(|c| !c.is_null()).count()), 1, "the opponent Special Summoned: {}", s.context());
    assert_eq!(s.count(0, "hand", TG_WARWOLF), 1, "it stayed in the hand: {}", s.context());
    let spawns = [at(0, Hand, MONSTER_REBORN), at(0, Grave, WARWOLF), at(0, Hand, TG_WARWOLF)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.run(&[Activate(MONSTER_REBORN), Chain(TG_WARWOLF)]).unwrap();
    clean(&s);
    assert!(s.monster(0, TG_WARWOLF).is_some(), "{}", s.context());
}

/// "Once per turn, during the End Phase, if this card is in the GY because it was destroyed on the field and sent there
/// this turn: You can add 1 "T.G." monster from your Deck to your hand, except "T.G. Warwolf"."
#[test]
fn tg_warwolf_searches_in_the_end_phase_after_being_destroyed() {
    let s = searches_after_destruction(false, TG_WARWOLF);
    assert_eq!(s.count(0, "hand", METAL_SKELETON), 1, "a T.G. monster was added: {}", s.context());
}

/// T.G. Wonder Magician: "If this card is Synchro Summoned: Target 1 Spell/Trap on the field; destroy that target." The
/// opponent's Set Mystical Space Typhoon goes.
#[test]
fn wonder_magician_destroys_a_spell_or_trap_when_synchro_summoned() {
    let spawns = [at(0, Monster, TG_STRIKER), at(0, Monster, TG_WARWOLF), at(1, SpellSet, MST)];
    let Some(mut s) = printed(&[WONDER_MAGICIAN], &spawns) else { return };
    s.run(&[SpSummon(WONDER_MAGICIAN)]).unwrap();
    clean(&s);
    assert!(s.monster(0, WONDER_MAGICIAN).is_some(), "{}", s.context());
    assert_eq!(s.count(1, "grave", MST), 1, "the Set card was destroyed: {}", s.context());
}

/// "If this card on the field is destroyed: Draw 1 card."
#[test]
fn wonder_magician_draws_when_destroyed() {
    let spawns = [at(0, Monster, WONDER_MAGICIAN), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    let before = hand(&s, 0);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WONDER_MAGICIAN), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before + 1, "{}", s.context());
}

/// "Once per Chain, during your opponent's Main Phase, you can (Quick Effect): Immediately after this effect resolves,
/// Synchro Summon using this card you control." The opponent Normal Summons; Wonder Magician (a Tuner, Level 5) and T.G.
/// Metal Skeleton (2) become Seven Swords Warrior (7).
#[test]
fn wonder_magician_synchro_summons_in_the_opponents_main_phase() {
    let spawns = [at(0, Monster, WONDER_MAGICIAN), at(0, Monster, METAL_SKELETON)];
    let Some(mut s) = printed(&[SEVEN_SWORDS], &spawns) else { return };
    s.run_both([&[Chain(WONDER_MAGICIAN)], &[Summon(KURIBOH)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, SEVEN_SWORDS).is_some(), "Seven Swords Warrior was Synchro Summoned: {}", s.context());
    assert_eq!(s.count(0, "grave", WONDER_MAGICIAN), 1, "{}", s.context());
}

/// The cards in a team's Monster Zones.
fn monsters(s: &Scenario, team: usize) -> usize {
    s.state()[team]["mzone"].as_array().map_or(0, |m| m.iter().filter(|c| !c.is_null()).count())
}

/// The Claw of Hermos: "Send 1 monster from your hand or field to the GY, of the Type that is listed on a Fusion Monster
/// that can only be Special Summoned with "The Claw of Hermos", then Special Summon that Fusion Monster from your Extra
/// Deck." Dark Magician (a Spellcaster) is sent and Time Magic Hammer comes ("using a Spellcaster monster").
#[test]
fn claw_of_hermos_summons_time_magic_hammer_for_a_spellcaster() {
    let spawns = [at(0, Hand, CLAW), at(0, Monster, DARK_MAGICIAN), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&[TIME_MAGIC_HAMMER], &spawns) else { return };
    s.run(&[Activate(CLAW)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "the Spellcaster was sent: {}", s.context());
    assert_eq!(s.count(0, "grave", CLAW), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", TIME_MAGIC_HAMMER), 1, "Time Magic Hammer came and equipped itself: {}", s.state());
    assert!(s.monster(0, WARWOLF).is_some(), "{}", s.context());
}

/// A Warrior sent instead brings Rocket Hermos Cannon, a Dragon Red-Eyes Black Dragon Sword: the Type chooses the Fusion
/// Monster.
#[test]
fn claw_of_hermos_follows_the_type_of_the_monster_sent() {
    let spawns = [at(0, Hand, CLAW), at(0, Hand, GAIA), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&[TIME_MAGIC_HAMMER, ROCKET_HERMOS_CANNON, RED_EYES_SWORD], &spawns) else { return };
    s.run(&[Activate(CLAW)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GAIA), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", ROCKET_HERMOS_CANNON), 1, "a Warrior brings Rocket Hermos Cannon: {}", s.state());
    assert_eq!(s.count(0, "szone", TIME_MAGIC_HAMMER) + s.count(0, "szone", RED_EYES_SWORD), 0, "{}", s.context());
}

/// "You can only activate 1 "The Claw of Hermos" per turn", and "There must be" a monster of a listed Type: with only
/// Gene-Warped Warwolf (a Beast-Warrior) it can't be activated.
#[test]
fn claw_of_hermos_needs_a_monster_of_a_listed_type() {
    let spawns = [at(0, Hand, CLAW), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&[TIME_MAGIC_HAMMER], &spawns) else { return };
    assert!(s.run(&[Activate(CLAW)]).is_err(), "{}", s.context());
}

/// Anime Claw of Hermos: "Send this card and 1 monster you control, then activate 1 Fusion Monster from your Extra Deck
/// that lists both cards as Fusion Materials." Time Wizard on the field, Time Magic Hammer (anime: "Time Wizard" + "The
/// Claw of Hermos") from the Extra Deck.
#[test]
fn anime_claw_of_hermos_brings_the_anime_time_magic_hammer_with_time_wizard() {
    let spawns = [at(0, Hand, CLAW), at(0, Monster, TIME_WIZARD), at(1, Monster, WARWOLF)]; // the Hammer needs another monster to equip to
    let Some(mut s) = anime(&[TIME_MAGIC_HAMMER], &spawns) else { return };
    s.run(&[Activate(CLAW)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TIME_WIZARD), 1, "Time Wizard was sent: {}", s.context());
    assert_eq!(s.count(0, "szone", TIME_MAGIC_HAMMER), 1, "the Hammer is an Equip Spell in the anime: {}", s.state());
}

/// The Eye of Timaeus: "Target 1 "Dark Magician" monster you control; Fusion Summon 1 Fusion Monster from your Extra Deck
/// that lists that monster on the field as Fusion Material, using it as the entire Fusion Material." Dark Magician becomes
/// Amulet Dragon ("Dark Magician" + 1 Dragon monster, "or with The Eye of Timaeus").
#[test]
fn eye_of_timaeus_fusion_summons_amulet_dragon() {
    let spawns = [at(0, Hand, EYE), at(0, Monster, DARK_MAGICIAN)];
    let Some(mut s) = printed(&[AMULET_DRAGON], &spawns) else { return };
    s.run(&[Activate(EYE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, AMULET_DRAGON).is_some(), "Amulet Dragon came: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "Dark Magician was the material: {}", s.context());
}

/// Anime Eye of Timaeus: "Send this card and 1 monster you control to the Graveyard. Special Summon a Fusion Monster
/// from your Extra Deck that lists both cards as Fusion Materials." Dark Magician and the Eye bring Amulet Dragon.
#[test]
fn anime_eye_of_timaeus_sends_itself_and_a_monster() {
    let spawns = [at(0, Hand, EYE), at(0, Monster, DARK_MAGICIAN)];
    let Some(mut s) = anime(&[AMULET_DRAGON], &spawns) else { return };
    s.run(&[Activate(EYE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, AMULET_DRAGON).is_some(), "Amulet Dragon came: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "Dark Magician was sent: {}", s.context());
    assert_eq!(s.count(0, "removed", EYE), 1, "the Eye was sent too, and Amulet Dragon banished that Spell from the Graveyard: {}", s.context());
    assert!(s.attack_of(0, AMULET_DRAGON) > Some(2900), "Amulet Dragon gained ATK for the banished Spell: {}", s.context());
}

/// The Fang of Critias: "Send 1 Trap from your hand or field to the GY, that is listed on a Fusion Monster that can only
/// be Special Summoned with "The Fang of Critias", then Special Summon that Fusion Monster." Mirror Force is sent and
/// Mirror Force Dragon comes.
#[test]
fn fang_of_critias_summons_mirror_force_dragon() {
    let spawns = [at(0, Hand, FANG), at(0, Hand, MIRROR_FORCE)];
    let Some(mut s) = printed(&[MIRROR_FORCE_DRAGON], &spawns) else { return };
    s.run(&[Activate(FANG)]).unwrap();
    clean(&s);
    assert!(s.monster(0, MIRROR_FORCE_DRAGON).is_some(), "Mirror Force Dragon came: {}", s.context());
    assert_eq!((s.count(0, "grave", MIRROR_FORCE), s.count(0, "grave", FANG)), (1, 1), "{}", s.context());
}

/// "if that card is Set, reveal it": a Set Mirror Force works too.
#[test]
fn fang_of_critias_takes_a_set_trap_from_the_field() {
    let spawns = [at(0, Hand, FANG), at(0, SpellSet, MIRROR_FORCE)];
    let Some(mut s) = printed(&[MIRROR_FORCE_DRAGON], &spawns) else { return };
    s.run(&[Activate(FANG)]).unwrap();
    clean(&s);
    assert!(s.monster(0, MIRROR_FORCE_DRAGON).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", MIRROR_FORCE), 1, "{}", s.context());
}

/// Anime Fang of Critias: "Send this card and 1 Trap you control to the Graveyard, then Special Summon 1 Fusion Monster
/// from your Extra Deck that lists both cards as Fusion Materials."
#[test]
fn anime_fang_of_critias_sends_itself_and_a_trap() {
    let spawns = [at(0, Hand, FANG), at(0, SpellSet, MIRROR_FORCE)];
    let Some(mut s) = anime(&[MIRROR_FORCE_DRAGON], &spawns) else { return };
    s.run(&[Activate(FANG)]).unwrap();
    clean(&s);
    assert!(s.monster(0, MIRROR_FORCE_DRAGON).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", FANG), 1, "{}", s.context());
}

/// Time Magic Hammer (printed): "Must be Special Summoned with "The Claw of Hermos"": with no Claw it can't be Special
/// Summoned from the Extra Deck, even with a Spellcaster on the field.
#[test]
fn time_magic_hammer_cannot_be_summoned_without_the_claw() {
    let spawns = [at(0, Monster, DARK_MAGICIAN)];
    let Some(mut s) = printed(&[TIME_MAGIC_HAMMER], &spawns) else { return };
    assert!(s.run(&[SpSummon(TIME_MAGIC_HAMMER)]).is_err(), "{}", s.context());
}

/// "If this card is Special Summoned: Target 1 other face-up monster on the field; equip this card to it. At the start of
/// the Damage Step, if it battles an opponent's monster: You can roll a six-sided die, and banish that opponent's monster
/// until the Standby Phase of the Nth turn after this effect's activation (N = the result)." Warwolf, carrying the Hammer,
/// attacks Blue-Eyes: it is banished.
#[test]
fn time_magic_hammer_banishes_the_monster_it_battles() {
    let spawns = [at(0, Hand, CLAW), at(0, Monster, DARK_MAGICIAN), at(0, Monster, WARWOLF), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&[TIME_MAGIC_HAMMER], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]); // the Hammer equips to Warwolf, not Dark Magician (sent) or Blue-Eyes
    s.run(&[Activate(CLAW), EndTurn, Attack(WARWOLF), Chain(TIME_MAGIC_HAMMER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", BLUE_EYES), 1, "Blue-Eyes was banished: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no battle damage, the battle never happened: {}", s.context());
}

/// The Duke of Demise: "Once per turn, during your Standby Phase, pay 500 LP or destroy this card." The question is a choice
/// between the two; the CPU's answer pays, which keeps it.
#[test]
fn duke_of_demise_is_kept_for_500_lp() {
    let Some(mut s) = printed(&[], &[at(0, Monster, DUKE)]) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7500, "500 LP paid: {}", s.context());
    assert!(s.monster(0, DUKE).is_some(), "{}", s.context());
}

/// ... and not paying destroys it.
#[test]
fn duke_of_demise_is_destroyed_when_the_500_lp_are_not_paid() {
    let Some(mut s) = printed(&[], &[at(0, Monster, DUKE)]) else { return };
    s.run_to(&[], |p| p["type"] == msg::SELECT_OPTION).unwrap();
    s.respond(Response::Option(1).json()).unwrap(); // "Destroy this card"
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(0, "grave", DUKE), 1, "{}", s.context());
}

/// "Cannot be destroyed by battle": Blue-Eyes (3000) attacks the Duke (2000 ATK): it stays, 1000 damage.
#[test]
fn duke_of_demise_cannot_be_destroyed_by_battle() {
    let spawns = [at(0, Monster, DUKE), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DUKE).is_some(), "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 500 - 1000, "500 for the Standby Phase, 1000 battle damage: {}", s.context());
}

/// "During your Main Phase, you can: Immediately after this effect resolves, Normal Summon 1 monster." A Normal Summon
/// was used (Kuriboh), the effect gives another: Gene-Warped Warwolf.
#[test]
fn duke_of_demise_gives_an_extra_normal_summon() {
    let spawns = [at(0, Monster, DUKE), at(0, Hand, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Summon(KURIBOH), Activate(DUKE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, KURIBOH).is_some() && s.monster(0, WARWOLF).is_some(), "two Normal Summons: {}", s.context());
}

/// "You can banish this card from your GY, then target 1 Level 4 or higher Fiend or Zombie monster in your GY; add it to
/// your hand." Zombie Master (Level 4 Zombie).
#[test]
fn duke_of_demise_banishes_itself_to_add_a_zombie_from_the_graveyard() {
    let spawns = [at(0, Grave, DUKE), at(0, Grave, ZOMBIE_MASTER), at(0, Grave, KURIBOH)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(0, ZOMBIE_MASTER)]);
    s.run(&[Activate(DUKE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", ZOMBIE_MASTER), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", DUKE), 1, "{}", s.context());
}

/// Anime Duke of Demise: "Cannot be destroyed by battle. Once per turn, during your Standby Phase, pay 500 LP or destroy
/// this card." It has no Normal Summon effect.
#[test]
fn anime_duke_of_demise_is_kept_for_500_lp_and_has_no_other_effect() {
    let spawns = [at(0, Monster, DUKE), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    assert!(s.run(&[Activate(DUKE)]).is_err(), "the anime Duke has no ignition effect: {}", s.context());
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DUKE).is_some(), "not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 500 - 1000, "{}", s.context());
}

/// The Mask of Remnants: "When this card is activated: Shuffle this card from the field into the Deck."
#[test]
fn mask_of_remnants_shuffles_itself_into_the_deck() {
    let Some(mut s) = printed(&[], &[at(0, Hand, MASK_OF_REMNANTS)]) else { return };
    s.run(&[Activate(MASK_OF_REMNANTS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", MASK_OF_REMNANTS) + s.count(0, "grave", MASK_OF_REMNANTS), 0, "it left the field: {}", s.state());
    assert_eq!(s.count(0, "hand", MASK_OF_REMNANTS), 0, "{}", s.context());
}

/// "While this card is equipped to a monster by the effect of "Masked Beast Des Gardius", take control of that monster."
/// Blue-Eyes destroys Des Gardius (in Defense, 2500 DEF); it equips the Mask from the Deck to Blue-Eyes, which changes sides.
#[test]
fn mask_of_remnants_takes_control_of_the_monster_it_is_equipped_to() {
    let spawns = [at(0, MonsterDefense, DES_GARDIUS), at(1, Monster, BLUE_EYES)];
    let mains = [&deck_with(&[MASK_OF_REMNANTS])[..], &[KURIBOH; 40][..]];
    let Some(dir) = engine_dir() else { return };
    let mut s = Scenario::start_field(&dir, 7, 8000, [(mains[0], &[]), (mains[1], &[])], &spawns).unwrap();
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DES_GARDIUS), 1, "{}", s.context());
    assert!(s.monster(0, BLUE_EYES).is_some(), "Blue-Eyes was taken: {}", s.context());
}

/// Ties of the Brethren: "Pay 2000 LP, then target 1 Level 4 or lower monster you control; ... Special Summon 2 monsters
/// from your Deck, with the same Type, Attribute, and Level as that monster, but with different names from each other
/// and that monster. You cannot conduct your Battle Phase the turn you activate this card." Gazelle (Level 4 EARTH
/// Beast) brings two other Level 4 EARTH Beasts.
#[test]
fn ties_of_the_brethren_summons_two_monsters_of_the_same_kind() {
    let spawns = [at(0, Hand, TIES), at(0, Monster, GAZELLE)];
    let mains = [&deck_with(&[PREVENT_RAT, MON_LARVAS, SOUL_TIGER])[..], &[KURIBOH; 40][..]];
    let Some(dir) = engine_dir() else { return };
    let mut s = Scenario::start_field(&dir, 7, 8000, [(mains[0], &[]), (mains[1], &[])], &spawns).unwrap();
    s.run(&[Activate(TIES)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6000, "{}", s.context());
    assert_eq!(monsters(&s, 0), 3, "two came: {}", s.context());
    let beasts = [PREVENT_RAT, MON_LARVAS, SOUL_TIGER].iter().filter(|&&c| s.monster(0, c).is_some()).count();
    assert_eq!(beasts, 2, "from the three in the Deck: {}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase this turn: {}", s.context());
}

/// Anime Ties of the Brethren: "Pay 1000 LP; Special Summon up to 2 Level 4 monsters of the same Type, from your hand or
/// Deck with the same Type as a monster you control, but they cannot attack or be Tributed."
#[test]
fn anime_ties_of_the_brethren_pays_1000_and_summons_two() {
    let spawns = [at(0, Hand, TIES), at(0, Monster, GAZELLE)];
    let mains = [&deck_with(&[PREVENT_RAT, MON_LARVAS, SOUL_TIGER])[..], &[KURIBOH; 40][..]];
    let Some(dir) = engine_dir() else { return };
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, [(mains[0], &[]), (mains[1], &[])], &spawns).unwrap();
    s.run(&[Activate(TIES)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "{}", s.context());
    assert!(monsters(&s, 0) >= 2, "up to two came (the CPU takes one): {}", s.context());
}

/// Anime Ties of the Brethren: "Special Summon up to 2 Level 4 monsters of the same Type". With `prefer_most` both
/// preferred Beasts are taken: Gazelle plus two, three monsters, none of them left in the Deck for the third.
#[test]
fn anime_ties_of_the_brethren_summons_two_when_both_are_chosen() {
    let spawns = [at(0, Hand, TIES), at(0, Monster, GAZELLE)];
    let mains = [&deck_with(&[PREVENT_RAT, MON_LARVAS, SOUL_TIGER])[..], &[KURIBOH; 40][..]];
    let Some(dir) = engine_dir() else { return };
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, [(mains[0], &[]), (mains[1], &[])], &spawns).unwrap();
    s.prefer(&[(0, PREVENT_RAT), (0, MON_LARVAS), (0, SOUL_TIGER)]);
    s.prefer_most(true);
    s.run(&[Activate(TIES)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "{}", s.context());
    assert_eq!(monsters(&s, 0), 3, "Gazelle and two more: {}", s.context());
}

/// Timaeus the Knight of Destiny: "Must be Special Summoned (from your Extra Deck) by sending the above cards you control
/// to the GY." The three Legendary Knights on the field go; "Unaffected by other cards' effects": Dark Hole spares it.
#[test]
fn knight_of_destiny_is_summoned_by_the_three_knights_and_unaffected() {
    let spawns = [at(0, Monster, LEGENDARY_TIMAEUS), at(0, Monster, LEGENDARY_CRITIAS), at(0, Monster, LEGENDARY_HERMOS),
        at(1, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&[KNIGHT_OF_DESTINY], &spawns) else { return };
    s.run(&[SpSummon(KNIGHT_OF_DESTINY)]).unwrap();
    clean(&s);
    assert!(s.monster(0, KNIGHT_OF_DESTINY).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", LEGENDARY_TIMAEUS) + s.count(0, "grave", LEGENDARY_CRITIAS) + s.count(0, "grave", LEGENDARY_HERMOS), 3, "{}", s.context());
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, KNIGHT_OF_DESTINY).is_some(), "Dark Hole did not touch it: {}", s.context());
}

/// "Once per battle, if this card battles, during damage calculation (Quick Effect): You can make this card's ATK/DEF
/// become equal to the ATK of the monster on the field with the highest ATK." It attacks Blue-Eyes and becomes 3000: both
/// are destroyed. "When this card is destroyed by battle: You can Special Summon 3 "Legendary Knight" monsters from your
/// hand, Deck, and/or GY": the three come back.
#[test]
fn knight_of_destiny_copies_the_highest_atk_and_brings_the_knights_back() {
    let spawns = [at(0, Monster, LEGENDARY_TIMAEUS), at(0, Monster, LEGENDARY_CRITIAS), at(0, Monster, LEGENDARY_HERMOS),
        at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&[KNIGHT_OF_DESTINY], &spawns) else { return };
    s.run(&[SpSummon(KNIGHT_OF_DESTINY), EndTurn, Attack(KNIGHT_OF_DESTINY), Chain(KNIGHT_OF_DESTINY), Chain(KNIGHT_OF_DESTINY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "Blue-Eyes was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", KNIGHT_OF_DESTINY), 1, "so was the Knight of Destiny (3000 against 3000): {}", s.context());
    assert_eq!([LEGENDARY_TIMAEUS, LEGENDARY_CRITIAS, LEGENDARY_HERMOS].iter().filter(|&&c| s.monster(0, c).is_some()).count(), 3, "{}", s.context());
}

/// Time Maiden: "If you control no monsters, you can Special Summon this card (from your hand)." The opponent's monster
/// does not matter.
#[test]
fn time_maiden_comes_when_its_controller_has_no_monsters() {
    let spawns = [at(0, Hand, TIME_MAIDEN), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run(&[SpSummon(TIME_MAIDEN)]).unwrap();
    clean(&s);
    assert!(s.monster(0, TIME_MAIDEN).is_some(), "{}", s.context());
    let spawns = [at(0, Hand, TIME_MAIDEN), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    assert!(s.run(&[SpSummon(TIME_MAIDEN)]).is_err(), "not with a monster of its own: {}", s.context());
}

/// "You can Tribute this card; add 1 "Timelord" monster with 0 ATK from your Deck to your hand."
#[test]
fn time_maiden_tributes_to_add_a_timelord() {
    let spawns = [at(0, Monster, TIME_MAIDEN)];
    let mains = [&deck_with(&[ZAPHION])[..], &[KURIBOH; 40][..]];
    let Some(dir) = engine_dir() else { return };
    let mut s = Scenario::start_field(&dir, 7, 8000, [(mains[0], &[]), (mains[1], &[])], &spawns).unwrap();
    s.run(&[Activate(TIME_MAIDEN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TIME_MAIDEN), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", ZAPHION), 1, "Zaphion (0 ATK) was added: {}", s.context());
}

/// "This card can be treated as 2 Tributes for the Tribute Summon of a "Timelord" monster." Alone on the field, it pays
/// for Zaphion (Level 10: two Tributes).
#[test]
fn time_maiden_counts_as_two_tributes_for_a_timelord() {
    let spawns = [at(0, Monster, TIME_MAIDEN), at(0, Hand, ZAPHION)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run(&[Summon(ZAPHION)]).unwrap();
    clean(&s);
    assert!(s.monster(0, ZAPHION).is_some(), "Zaphion was Tribute Summoned: {}", s.context());
    assert_eq!(s.count(0, "grave", TIME_MAIDEN), 1, "{}", s.context());
}

/// "You can banish this card from your GY; Special Summon 1 "Timelord" monster with 0 ATK from your Deck, ignoring its
/// Summoning conditions. You cannot Special Summon other monsters during the turn you activate this effect."
#[test]
fn time_maiden_banishes_from_the_graveyard_to_summon_a_timelord() {
    let spawns = [at(0, Grave, TIME_MAIDEN)];
    let mains = [&deck_with(&[ZAPHION])[..], &[KURIBOH; 40][..]];
    let Some(dir) = engine_dir() else { return };
    let mut s = Scenario::start_field(&dir, 7, 8000, [(mains[0], &[]), (mains[1], &[])], &spawns).unwrap();
    s.run(&[Activate(TIME_MAIDEN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", TIME_MAIDEN), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, ZAPHION), Some(0), "Zaphion came: {}", s.context());
}

/// Anime Time Maiden: "You can Special Summon this card (from your hand) if there are no cards on the field": the
/// opponent's monster stops it, an empty field allows it.
#[test]
fn anime_time_maiden_needs_a_field_without_cards() {
    let spawns = [at(0, Hand, TIME_MAIDEN), at(1, Monster, WARWOLF)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    assert!(s.run(&[SpSummon(TIME_MAIDEN)]).is_err(), "{}", s.context());
    let Some(mut s) = anime(&[], &[at(0, Hand, TIME_MAIDEN)]) else { return };
    s.run(&[SpSummon(TIME_MAIDEN)]).unwrap();
    clean(&s);
    assert!(s.monster(0, TIME_MAIDEN).is_some(), "{}", s.context());
}

/// Time Stream: "Target 1 "Fossil" Fusion Monster you control; Tribute it, and if you do, Special Summon 1 "Fossil" Fusion
/// Monster from your Extra Deck, whose original Level is 2 higher than that monster's." Skullgar (Level 4) becomes Skull
/// Knight (6).
#[test]
fn time_stream_upgrades_a_fossil_fusion_monster() {
    let spawns = [at(0, Hand, TIME_STREAM), at(0, Monster, SKULLGAR)];
    let Some(mut s) = printed(&[SKULL_KNIGHT, SKULL_KING], &spawns) else { return };
    s.run(&[Activate(TIME_STREAM)]).unwrap();
    clean(&s);
    assert!(s.monster(0, SKULL_KNIGHT).is_some(), "Skull Knight (Level 6) came: {}", s.context());
    assert!(s.monster(0, SKULL_KING).is_none(), "not the Level 8 one: {}", s.context());
    assert_eq!(s.count(0, "grave", SKULLGAR), 1, "Skullgar was Tributed: {}", s.context());
}

/// "During your Main Phase, except the turn this card was sent to the GY: You can banish this card and 1 "Fossil" Fusion
/// Monster from your GY, then target 1 "Fossil" Fusion Monster in your GY; Special Summon it." Fossil Fusion brings Skullgar
/// (Barrel Rock from the one Graveyard, a Kuriboh from the other), Time Stream makes it Skull Knight, Dark Hole destroys that,
/// and next turn Time Stream and one of the two (banished) bring the other back.
#[test]
fn time_stream_revives_from_the_graveyard() {
    let spawns = [at(0, Hand, FOSSIL_FUSION), at(0, Hand, TIME_STREAM), at(0, Grave, BARREL_ROCK), at(1, Grave, KURIBOH), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&[SKULLGAR, SKULL_KNIGHT], &spawns) else { return };
    s.run_both([&[Activate(FOSSIL_FUSION), Activate(TIME_STREAM)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", SKULL_KNIGHT), s.count(0, "grave", SKULLGAR), s.count(0, "grave", TIME_STREAM)), (1, 1, 1), "{}", s.context());
    s.run(&[EndTurn, Activate(TIME_STREAM)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", TIME_STREAM), 1, "Time Stream was banished: {}", s.context());
    assert_eq!(monsters(&s, 0), 1, "one Fossil Fusion Monster came back: {}", s.context());
    assert_eq!(s.count(0, "removed", SKULLGAR) + s.count(0, "removed", SKULL_KNIGHT), 1, "and one was banished with it: {}", s.context());
}

/// Toon Briefcase: "When your opponent Summons a monster(s) while you control a Toon monster: Shuffle that monster(s) into
/// the Deck." The opponent Normal Summons Gene-Warped Warwolf: gone.
#[test]
fn toon_briefcase_shuffles_a_summoned_monster() {
    let spawns = [at(0, SpellSet, TOON_BRIEFCASE), at(0, Monster, TOON_MERMAID), at(1, Hand, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[Chain(TOON_BRIEFCASE)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(monsters(&s, 1), 0, "Warwolf was shuffled into the Deck: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF) + s.count(1, "hand", WARWOLF), 0, "{}", s.context());
}

/// The printed card shuffles Special Summoned monsters too ("Summons"): Monster Reborn.
#[test]
fn toon_briefcase_shuffles_a_special_summoned_monster() {
    let spawns = [at(0, SpellSet, TOON_BRIEFCASE), at(0, Monster, TOON_MERMAID), at(1, Hand, MONSTER_REBORN), at(1, Grave, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[Chain(TOON_BRIEFCASE)], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, WARWOLF).is_none(), "{}", s.context());
}

/// ... and only "while you control a Toon monster".
#[test]
fn toon_briefcase_needs_a_toon_monster() {
    let spawns = [at(0, SpellSet, TOON_BRIEFCASE), at(1, Hand, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, WARWOLF).is_some(), "{}", s.context());
}

/// Anime Toon Briefcase: "When a monster is Normal Summoned to your opponent's field while you control "Toon World":
/// Shuffle that monster(s) into the Deck." A Normal Summon is shuffled, a Special Summon is not.
#[test]
fn anime_toon_briefcase_answers_only_normal_summons_with_toon_world() {
    let spawns = [at(0, SpellSet, TOON_BRIEFCASE), at(0, Spell, TOON_WORLD), at(1, Hand, WARWOLF)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.run_both([&[Chain(TOON_BRIEFCASE)], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(monsters(&s, 1), 0, "the Normal Summoned monster was shuffled: {}", s.context());
    let spawns = [at(0, SpellSet, TOON_BRIEFCASE), at(0, Spell, TOON_WORLD), at(1, Hand, MONSTER_REBORN), at(1, Grave, WARWOLF)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[], &[Activate(MONSTER_REBORN)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, WARWOLF).is_some(), "a Special Summon is left alone: {}", s.context());
}

/// Toon Kingdom: "When this card is activated: Banish 3 cards from the top of your Deck, face-down."
#[test]
fn toon_kingdom_banishes_three_cards_from_the_deck() {
    let Some(mut s) = printed(&[], &[at(0, Hand, TOON_KINGDOM)]) else { return };
    let before = s.state()[0]["removed"].as_array().map_or(0, Vec::len);
    s.run(&[Activate(TOON_KINGDOM)]).unwrap();
    clean(&s);
    assert_eq!(s.state()[0]["removed"].as_array().map_or(0, Vec::len), before + 3, "{}", s.state());
    assert_eq!(s.count(0, "szone", TOON_KINGDOM), 1, "{}", s.state());
}

/// "This card's name becomes "Toon World" while in the Field Zone": Toon Gemini Elf, which "can attack your opponent
/// directly" while its controller controls Toon World, attacks the empty field for 1900.
#[test]
fn toon_kingdom_counts_as_toon_world() {
    let spawns = [at(0, Hand, TOON_KINGDOM), at(0, Monster, TOON_GEMINI_ELF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run(&[Activate(TOON_KINGDOM), EndTurn, Attack(TOON_GEMINI_ELF)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1900, "{}", s.context());
}

/// "If a Toon monster(s) you control would be destroyed by battle or card effect, you can banish 1 card from the top of
/// your Deck, face-down, for each of those monster(s) instead." Dark Hole: Toon Mermaid stays, one more card is banished.
#[test]
fn toon_kingdom_banishes_deck_cards_instead_of_a_toon_monster() {
    let spawns = [at(0, Hand, TOON_KINGDOM), at(0, Monster, TOON_MERMAID), at(1, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    let before = s.state()[0]["removed"].as_array().map_or(0, Vec::len);
    s.run_both([&[Activate(TOON_KINGDOM), EndTurn, Chain(TOON_KINGDOM)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, TOON_MERMAID).is_some(), "Toon Mermaid was not destroyed: {}", s.context());
    assert_eq!(s.state()[0]["removed"].as_array().map_or(0, Vec::len), before + 4, "3 and 1: {}", s.state());
}

/// "Your opponent cannot target Toon monsters you control with card effects." Mind Control has no target among Toon
/// monsters, but takes an ordinary monster.
#[test]
fn toon_kingdom_protects_toon_monsters_from_targeting() {
    let spawns = [at(0, Hand, TOON_KINGDOM), at(0, Monster, TOON_MERMAID), at(1, Hand, MIND_CONTROL)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    assert!(s.run_both([&[Activate(TOON_KINGDOM)], &[Activate(MIND_CONTROL)]]).is_err(), "no target: {}", s.context());
    let spawns = [at(0, Hand, TOON_KINGDOM), at(0, Monster, WARWOLF), at(1, Hand, MIND_CONTROL)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[Activate(TOON_KINGDOM)], &[Activate(MIND_CONTROL)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, WARWOLF).is_some(), "an ordinary monster is a target: {}", s.context());
}

/// Toon Mask (a Trap Card): "If you control "Toon World": Target 1 face-up monster your opponent controls; Special Summon 1
/// Toon monster from your hand or Deck, whose Level is less than or equal to the Level/Rank of that monster, ignoring its
/// Summoning conditions." Set, then activated a turn later against a Level 4 monster: Toon Mermaid (4) comes and Toon
/// Summoned Skull (6) does not.
#[test]
fn toon_mask_summons_a_toon_monster_of_at_most_the_targets_level() {
    let spawns = [at(0, SpellSet, TOON_MASK), at(0, Spell, TOON_WORLD), at(0, Hand, TOON_MERMAID), at(0, Hand, TOON_SUMMONED_SKULL), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the Toon monsters
    s.run(&[EndTurn, Activate(TOON_MASK)]).unwrap();
    clean(&s);
    assert!(s.monster(0, TOON_MERMAID).is_some(), "Toon Mermaid came: {}", s.context());
    assert_eq!(s.count(0, "hand", TOON_SUMMONED_SKULL), 1, "Level 6 is too high: {}", s.context());
}

/// Without Toon World it can't be activated.
#[test]
fn toon_mask_needs_toon_world() {
    let spawns = [at(0, SpellSet, TOON_MASK), at(0, Hand, TOON_MERMAID), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    assert!(s.run(&[EndTurn, Activate(TOON_MASK)]).is_err(), "{}", s.context());
}

/// Anime Toon Mask: "When your opponent Summons a monster(s): Tribute 1 Toon monster; Special Summon 1 Toon monster of
/// the same name as 1 of those monster(s) your opponent Summoned." The opponent Summons Gemini Elf; a Toon Mermaid is
/// Tributed and Toon Gemini Elf comes from the hand.
#[test]
fn anime_toon_mask_answers_a_summon_with_the_toon_version() {
    let spawns = [at(0, SpellSet, TOON_MASK), at(0, Monster, TOON_MERMAID), at(0, Hand, TOON_GEMINI_ELF), at(1, Hand, GEMINI_ELF)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps Toon Gemini Elf
    s.run_both([&[Chain(TOON_MASK)], &[Summon(GEMINI_ELF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TOON_MERMAID), 1, "the Toon was Tributed: {}", s.context());
    assert!(s.monster(0, TOON_GEMINI_ELF).is_some(), "Toon Gemini Elf came: {}", s.context());
}

/// Toon Rollback: "Target 1 Toon monster you control; it can make a second attack during each Battle Phase this turn."
/// Toon Gemini Elf (1900, "While you control Toon World and your opponent controls no Toon monsters, this card can attack
/// your opponent directly") attacks twice: 3800.
#[test]
fn toon_rollback_gives_a_second_attack() {
    let spawns = [at(0, Hand, TOON_ROLLBACK), at(0, Spell, TOON_WORLD), at(0, Monster, TOON_GEMINI_ELF)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps Toon Rollback
    s.run(&[EndTurn, Activate(TOON_ROLLBACK), Attack(TOON_GEMINI_ELF), Attack(TOON_GEMINI_ELF)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 3800, "{}", s.context());
}

/// Anime Toon Rollback: "When a Toon Monster's attack is negated: This turn, you can conduct your Battle Phase again."
/// The first attack is negated by Negate Attack; Rollback is chained and the second attack gets through.
#[test]
fn anime_toon_rollback_gives_another_battle_phase_after_a_negated_attack() {
    let spawns = [at(0, SpellSet, TOON_ROLLBACK), at(0, Spell, TOON_WORLD), at(0, Monster, TOON_GEMINI_ELF), at(1, SpellSet, NEGATE_ATTACK)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.run_both([&[EndTurn, Attack(TOON_GEMINI_ELF), Chain(TOON_ROLLBACK), Attack(TOON_GEMINI_ELF)], &[ChainOnAttack(NEGATE_ATTACK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", NEGATE_ATTACK), 1, "the first attack was negated: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1900, "the second attack got through: {}", s.context());
}

/// Totem Pole: "Once per battle, when an opponent's monster declares an attack: You can negate that attack, and if you do,
/// place 1 counter on this card." Blue-Eyes attacks the empty field: no damage, the Totem Pole stays with a counter. (Its
/// third counter and Graveyard effect are not reached: the harness's plan can't make the opponent attack on later turns.)
#[test]
fn totem_pole_negates_an_attack_and_gains_a_counter() {
    let spawns = [at(0, SpellSet, TOTEM_POLE), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[ChainOnAttack(TOTEM_POLE)], &[Attack(BLUE_EYES), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.count(0, "szone", TOTEM_POLE)), (8000, 1), "the attack was negated, the Totem Pole stays: {}", s.context());
    let counters = s.state()[0]["szone"].as_array().and_then(|z| z.iter().find(|c| c["cid"] == TOTEM_POLE)).map(|c| c["counters"].to_string());
    assert!(counters.is_some_and(|c| c != "{}"), "it carries a counter: {}", s.state());
}

/// "Your opponent cannot target Rock monsters you control with 0 original ATK, with card effects." Barrel Rock has 1000
/// ATK, so Mind Control takes it; with Totem Pole and no 0 ATK Rock, nothing is protected.
#[test]
fn totem_pole_does_not_protect_a_rock_with_atk() {
    let spawns = [at(0, SpellSet, TOTEM_POLE), at(0, Monster, BARREL_ROCK), at(1, Hand, MIND_CONTROL)];
    let Some(mut s) = printed(&[], &spawns) else { return };
    s.run_both([&[], &[Activate(MIND_CONTROL)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, BARREL_ROCK).is_some(), "{}", s.context());
}

/// Anime Totem Pole: "During the Battle Step, while an opponent's monster is attacking a monster: You can negate that
/// attack." A direct attack is not answered: Blue-Eyes' 3000 damage goes through.
#[test]
fn anime_totem_pole_does_not_answer_a_direct_attack() {
    let spawns = [at(0, SpellSet, TOTEM_POLE), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.run_both([&[], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 5000, "{}", s.context());
}

/// ... while an attack on a monster is negated. The card is activated first (face-up), then its effect answers the attack.
#[test]
fn anime_totem_pole_negates_an_attack_on_a_monster() {
    let spawns = [at(0, SpellSet, TOTEM_POLE), at(0, Monster, WARWOLF), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = anime(&[], &spawns) else { return };
    s.run_both([&[Chain(TOTEM_POLE), ChainOnAttack(TOTEM_POLE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, WARWOLF).is_some() && s.lp(0) == 8000, "the attack was negated: {}", s.context());
}
