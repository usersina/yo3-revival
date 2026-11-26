//! Added Tokens (#75, `engine/added-cards.json` rows marked `token`): the client shows a Token only by a card of its own,
//! so the cards that make a Token the game never had come with it as a row (kind 10, under a stand-in). Each card makes its
//! Token through the real core, both seats' client views checked after every step: the Token on the field with its stats,
//! and gone (not in a Graveyard) when it leaves. Texts: the official database's, ProjectIgnis' anime one for `[Anime]`.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The cards (rows 5212-5216) and their Tokens (rows 5206-5211), by the ids the client knows.
const STEAM_THE_CLOAK: u16 = 11173; // Blackwing - Steam the Cloak
const STEAM_TOKEN: u16 = 5174;
const ECOLE_DE_ZONE: u16 = 5180;
const MASK_TOKEN: u16 = 5175;
const GADGET_BOX: u16 = 5181;
const GADGET_BOX_TOKEN: u16 = 5176;
const GADGET_TOKEN: u16 = 5177; // the anime Gadget Box's
const DEMIURGE_EMA: u16 = 5182;
const HOMUNCULUS_TOKEN: u16 = 5178;
const BLACK_WING_REVENGE: u16 = 11176;
const BLACK_CREST_TOKEN: u16 = 5179; // Blackwing - Black Crest Token
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: 2000/100, no effect
const DARK_HOLE: u16 = 4342;
const SHURA: u16 = 8163; // Blackwing - Shura the Blue Flame, a Winged Beast (1800)
const BLUE_EYES: u16 = 4007;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn start(anime: bool, spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, 8000, decks, spawns) } else { Scenario::start_field(&dir, 7, 8000, decks, spawns) }.unwrap())
}

/// Blackwing - Steam the Cloak: "If this face-up card leaves the field: Special Summon 1 "Steam Token" (Aqua/WIND/Level
/// 1/ATK 100/DEF 100)." Dark Hole destroys it, the Token comes; a second Dark Hole destroys the Token, which goes to no
/// Graveyard.
#[test]
fn steam_the_cloak_leaves_a_steam_token() {
    let Some(mut s) = start(false, &[at(0, Monster, STEAM_THE_CLOAK), at(0, Hand, DARK_HOLE), at(0, Hand, DARK_HOLE)]) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", STEAM_THE_CLOAK), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, STEAM_TOKEN), Some(100), "the Steam Token: {}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", STEAM_TOKEN), s.count(0, "grave", STEAM_TOKEN)), (0, 0), "the Token is gone: {}", s.context());
}

/// The anime Steam the Cloak (anime cards on): "If this face-up card leaves the field: You can Special Summon 1 "Steam
/// Token"... Destroy that Token during the End Phase."
#[test]
fn anime_steam_the_cloak_leaves_a_steam_token_for_the_turn() {
    let Some(mut s) = start(true, &[at(0, Monster, STEAM_THE_CLOAK), at(0, Hand, DARK_HOLE)]) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(STEAM_THE_CLOAK)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, STEAM_TOKEN), Some(100), "the Steam Token: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", STEAM_TOKEN), 0, "destroyed in the End Phase: {}", s.context());
}

/// Ecole de Zone: "If the turn player Summons exactly 1 monster: Destroy it, and if you do, the controller of that monster
/// Special Summons 1 "Mask Token" (Spellcaster/DARK/Level 1/ATK ?/DEF ?) ... with ATK/DEF equal to what the destroyed
/// monster had on the field." The opponent's Warwolf (2000/100) becomes a 2000/100 Mask Token.
#[test]
fn ecole_de_zone_turns_a_summoned_monster_into_a_mask_token() {
    let Some(mut s) = start(false, &[at(0, Hand, ECOLE_DE_ZONE), at(1, Hand, WARWOLF)]) else { return };
    s.run_both([&[Activate(ECOLE_DE_ZONE), EndTurn], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    let token = s.monster(1, MASK_TOKEN).unwrap_or_else(|| panic!("the Mask Token: {}", s.context())).clone();
    assert_eq!((token["attack"].as_i64(), token["defense"].as_i64()), (Some(2000), Some(100)), "{}", s.context());
}

/// Gadget Box: "When this card is activated: Place 3 Morph Counters on it. Once per turn, during your Main Phase: You can
/// remove 1 Morph Counter from your field, and if you do, Special Summon 1 "Gadget Box Token" (Machine/EARTH/Level 1/ATK
/// 0/DEF 0)."
#[test]
fn gadget_box_makes_a_gadget_box_token() {
    let Some(mut s) = start(false, &[at(0, Hand, GADGET_BOX)]) else { return };
    s.run(&[Activate(GADGET_BOX), Activate(GADGET_BOX)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GADGET_BOX_TOKEN), 1, "{}", s.context());
}

/// The anime Gadget Box (anime cards on): "Once per turn: you can Special Summon 1 "Gadget Token" (Machine-Type/EARTH/Level
/// 1/ATK 0/DEF 0)." Its own Token, another added row.
#[test]
fn anime_gadget_box_makes_a_gadget_token() {
    let Some(mut s) = start(true, &[at(0, Hand, GADGET_BOX)]) else { return };
    s.run(&[Activate(GADGET_BOX), Activate(GADGET_BOX)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", GADGET_TOKEN), 1, "{}", s.context());
}

/// Demiurge Ema: "You can target 1 Spell/Trap on each field; destroy them, then Special Summon 1 "Homunculus Token"
/// (Fairy/LIGHT/Level 2/ATK 800/DEF 800) to both players' fields in Defense Position, and if you do, this card gains 1600
/// ATK."
#[test]
fn demiurge_ema_gives_both_players_a_homunculus_token() {
    let Some(mut s) = start(false, &[at(0, Monster, DEMIURGE_EMA), at(0, SpellSet, DARK_HOLE), at(1, SpellSet, DARK_HOLE)]) else { return };
    s.run(&[Activate(DEMIURGE_EMA)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", HOMUNCULUS_TOKEN), s.count(1, "mzone", HOMUNCULUS_TOKEN)), (1, 1), "{}", s.context());
    assert_eq!(s.attack_of(0, DEMIURGE_EMA), Some(4900), "3300 + 1600: {}", s.context());
}

/// Black Wing Revenge: "When a Winged Beast monster you control is destroyed by battle and sent to the GY: Special Summon 2
/// "Blackwing - Black Crest Tokens" (Winged Beast/DARK/Level 2/ATK 0/DEF 800)." Blue-Eyes destroys Shura.
#[test]
fn black_wing_revenge_brings_two_black_crest_tokens() {
    let Some(mut s) = start(false, &[at(0, Monster, SHURA), at(0, SpellSet, BLACK_WING_REVENGE), at(1, Monster, BLUE_EYES)]) else { return };
    s.prefer(&[(0, SHURA)]);
    s.run_both([&[EndTurn, Chain(BLACK_WING_REVENGE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SHURA), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", BLACK_CREST_TOKEN), 2, "{}", s.context());
}
