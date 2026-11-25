//! Scenario tests for the printed anime cards of group 05 (Dragon Nails to Fortune Fairy Ann, rows 4848-5205 of
//! `engine/added/cards.tsv`): each effect through the real core with both seats' client models checked after every step
//! (`duelcheck::scenario`). The printed text is played with anime cards off; the anime version (`start_anime_field`) is
//! tested where it does something else.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The group's cards (client ids: the Konami id, or the stand-in the added row has).
const DRAGON_NAILS: u16 = 4809;
const DRAGONROID: u16 = 4004;
const DYNA_BASE: u16 = 6836;
const DYNA_TANK: u16 = 6837;
const EARTHBOUND_RELEASE: u16 = 5014;
const EGYPTIAN_GOD_SLIME: u16 = 15332;
const ELECTROMAGNETIC_TURTLE: u16 = 11120;
const EMPTY_MACHINE: u16 = 13820;
const EN_SHUFFLE: u16 = 4704;
const EUCALYPTUS_MOLE: u16 = 5078;
const EYE_OF_ILLUSION: u16 = 4825;
const FALLEN_PARADISE: u16 = 12904;
const FENRIR: u16 = 11466;
const FINAL_FUSION: u16 = 11163;
const FINAL_LIGHT: u16 = 14361;
const FIRE_RECOVERY: u16 = 5039;
const FIVE_STAR_TWILIGHT: u16 = 3996;
const FLASH_FUSION: u16 = 11161;
const FLINT_CRAGGER: u16 = 15879;
const FLYING_ELEPHANT: u16 = 5289;
const FORCE_RESONATOR: u16 = 9477;
const FORTISSIMO: u16 = 9520;
const FORTUNE_CHARIOT: u16 = 13996;
const FORTUNE_FAIRY_ANN: u16 = 14554;
// The game's own cards they play with.
const KURIBOH: u16 = 4064;
const KURIBAH: u16 = 3989;
const KURIBEE: u16 = 3991;
const KURIBOO: u16 = 3992;
const KURIBEH: u16 = 3993;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const BLUE_EYES: u16 = 4007; // 3000 ATK
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const BOOK_OF_MOON: u16 = 5432;
const DARK_BLADE: u16 = 5581; // DARK Warrior, 1800 ATK, no effect
const RED_EYES: u16 = 4088; // DARK Dragon, 2400 ATK
const STEAMROID: u16 = 6386; // EARTH "roid"
const GYROID: u16 = 6385; // WIND "roid"
const GIANT_REX: u16 = 5806; // Dinosaur, 2000 ATK, no effect
const OBELISK: u16 = 4998;
const OBEDIENT_FIEND: u16 = 5666; // Ultimate Obedient Fiend: Level 10, 3500 ATK, two Tributes
const YAMI: u16 = 4341; // a Field Spell: an Earthbound Immortal is destroyed without one
const GAIA: u16 = 4044; // Gaia The Fierce Knight, Level 7, 2300 ATK
const URU: u16 = 8460; // Earthbound Immortal Uru, Level 10, 3000 ATK
const AVIAN: u16 = 6310;
const SPARKMAN: u16 = 6313;
const GRAND_MOLE: u16 = 6770; // Neo-Spacian Grand Mole
const GIANT_RAT: u16 = 4915; // Beast, Level 4
const TREE_OTTER: u16 = 7844;
const DARK_MAGICIAN_GIRL: u16 = 4766;
const DARK_MAGICIAN: u16 = 4041;
const URIA: u16 = 6563;
const THOR: u16 = 9329; // Thor, Lord of the Aesir
const FLAME_WINGMAN: u16 = 6344; // Fusion, 2100 ATK
const CYBER_TWIN: u16 = 6396; // Fusion, 2800 ATK
const BRUNHILDE: u16 = 13995; // Valkyrie Brunhilde: a Fairy, 1800 ATK
const DRITTE: u16 = 13992; // Valkyrie Dritte, 1000 ATK
const INPACHI: u16 = 5876; // Blazing Inpachi, FIRE, 1850 ATK
const FIRE_KRAKEN: u16 = 4525; // FIRE, 1600 ATK
const FOOTBALLER: u16 = 5702; // Battle Footballer, FIRE
const CYBER_DRAGON: u16 = 6390; // Level 5
const GRANEL: u16 = 9485; // Meklord Army of Granel
const CURSE_OF_DRAGON: u16 = 4045;
const SKULLGAR: u16 = 15368; // Fossil Dragon Skullgar, a "Fossil" Fusion Monster
const GAIA_CHAMPION: u16 = 4043; // Gaia the Dragon Champion: Gaia the Fierce Knight + Curse of Dragon

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}\n{}", s.problems, s.context());
}

fn start(anime: bool, lp: u32, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }.unwrap())
}

/// Both players with Kuriboh decks, 8000 LP, and `spawns` in place (team 0's Extra Deck `extra`).
fn duel(anime: bool, extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    start(anime, 8000, [(&[KURIBOH; 40], extra), (&[KURIBOH; 40], &[])], spawns)
}

/// A 40-card Deck of Kuriboh with ten copies of each of `cards`, so one is in the Deck whatever the opening hand holds.
fn stacked(cards: &[u16]) -> Vec<u16> {
    let mut deck = vec![KURIBOH; 40 - 10 * cards.len()];
    for &c in cards {
        deck.extend([c; 10]);
    }
    deck
}

fn put(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// "Equip only to a DARK monster. It gains 600 ATK": Dark Blade (1800) goes to 2400.
#[test]
fn dragon_nails_gives_a_dark_monster_600_atk() {
    let Some(mut s) = duel(false, &[], &[put(0, Hand, DRAGON_NAILS), put(0, Monster, DARK_BLADE)]) else { return };
    s.run(&[Activate(DRAGON_NAILS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", DRAGON_NAILS), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(2400), "{}", s.context());
}

/// "If this card is sent to the GY because the equipped monster was destroyed by battle and sent to the GY: You can
/// Special Summon that monster, and if you do, equip this card to it." Blue-Eyes destroys the equipped Dark Blade.
#[test]
fn dragon_nails_brings_its_monster_back_and_equips_again() {
    let spawns = [put(0, Hand, DRAGON_NAILS), put(0, Monster, DARK_BLADE), put(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[Activate(DRAGON_NAILS), EndTurn, Chain(DRAGON_NAILS)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7400, "2400 against 3000: 600 battle damage: {}", s.context());
    assert!(s.monster(0, DARK_BLADE).is_some(), "Dark Blade came back: {}", s.context());
    assert_eq!(s.count(0, "szone", DRAGON_NAILS), 1, "and Dragon Nails is on it again: {}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(2400), "{}", s.context());
}

/// Anime Dragon Nails: "Equip only to a DARK Dragon-Type monster. It gains 600 ATK." Red-Eyes goes to 3000; Dark Blade
/// is not a Dragon and stays at 1800.
#[test]
fn anime_dragon_nails_equips_only_a_dark_dragon() {
    let spawns = [put(0, Hand, DRAGON_NAILS), put(0, Monster, DARK_BLADE), put(0, Monster, RED_EYES)];
    let Some(mut s) = duel(true, &[], &spawns) else { return };
    s.run(&[Activate(DRAGON_NAILS)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, RED_EYES), Some(3000), "{}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(1800), "{}", s.context());
}

/// Dragonroid: "You can discard this card, then activate 1 of these effects; ● Add 1 non-WIND "roid" monster from your
/// Deck to your hand." From the Graveyard it is already discarded by the effect's cost: Steamroid (EARTH) is added.
#[test]
fn dragonroid_adds_a_non_wind_roid() {
    let deck = stacked(&[STEAMROID, GYROID]);
    let Some(mut s) = start(false, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &[put(0, Hand, DRAGONROID)]) else { return };
    let (steam, gyro) = (s.count(0, "hand", STEAMROID), s.count(0, "hand", GYROID));
    s.run(&[Activate(DRAGONROID)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", STEAMROID), steam + 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", GYROID), gyro, "Gyroid is WIND and stays in the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", DRAGONROID), 1, "{}", s.context());
}

/// "If your LP are lower than your opponent's: You can Special Summon this card from your hand." Blue-Eyes hits a
/// Warwolf (1000 damage), then Dyna Base comes out of the hand.
#[test]
fn dyna_base_comes_out_when_behind_in_life_points() {
    let spawns = [put(0, Hand, DYNA_BASE), put(0, Monster, WARWOLF), put(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps Dyna Base
    s.run_both([&[EndTurn, Activate(DYNA_BASE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "{}", s.context());
    assert!(s.monster(0, DYNA_BASE).is_some(), "Dyna Base was Special Summoned: {}", s.context());
}

/// "During your Main Phase: You can Fusion Summon 1 Fusion Monster from your Extra Deck, using this card you control
/// and other monsters from your hand or field as material." Dyna Base + Giant Rex make Dyna Tank, which "gains ATK
/// equal to the original ATK of the Dinosaur monster used" (2000).
#[test]
fn dyna_base_fusion_summons_dyna_tank_with_the_dinosaurs_attack() {
    let spawns = [put(0, Monster, DYNA_BASE), put(0, Hand, GIANT_REX)];
    let Some(mut s) = duel(false, &[DYNA_TANK], &spawns) else { return };
    s.run(&[Activate(DYNA_BASE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DYNA_TANK), Some(2000), "{}", s.context());
    assert!(s.monster(0, DYNA_BASE).is_none() && s.monster(0, GIANT_REX).is_none(), "the materials are gone: {}", s.context());
}

/// Anime Dyna Base: "You can Tribute this card and 1 Dinosaur-Type monster from your hand or field; Special Summon 1
/// "Dyna Tank" from your Extra Deck."
#[test]
fn anime_dyna_base_tributes_for_dyna_tank() {
    let spawns = [put(0, Monster, DYNA_BASE), put(0, Monster, GIANT_REX)];
    let Some(mut s) = duel(true, &[DYNA_TANK], &spawns) else { return };
    s.run(&[Activate(DYNA_BASE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DYNA_TANK), Some(2000), "{}", s.context());
    assert_eq!(s.count(0, "grave", GIANT_REX), 1, "{}", s.context());
}

/// "If this card in its owner's possession is destroyed by an opponent's card: You can Special Summon 1 Dinosaur
/// monster from your GY." The opponent's Dark Hole destroys Dyna Tank; Giant Rex comes back from the Graveyard.
#[test]
fn dyna_tank_revives_a_dinosaur_when_destroyed() {
    let spawns = [put(0, Monster, DYNA_TANK), put(0, Grave, GIANT_REX), put(1, Hand, DARK_HOLE)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(DYNA_TANK)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DYNA_TANK).is_none(), "{}", s.context());
    assert!(s.monster(0, GIANT_REX).is_some(), "Giant Rex came from the Graveyard: {}", s.context());
}

/// "When a card or effect is activated that targets this 1 card on the field (Quick Effect): You can target 1 other
/// card on the field that would be an appropriate target; that card or effect now targets the new target." Book of Moon
/// aimed at Dyna Tank is redirected to Dark Blade, which is turned face-down instead.
#[test]
fn dyna_tank_redirects_a_targeting_effect() {
    let spawns = [put(0, Monster, DYNA_TANK), put(0, Monster, DARK_BLADE), put(1, Hand, BOOK_OF_MOON)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.prefer(&[(0, DYNA_TANK), (0, DARK_BLADE)]);
    s.run_both([&[EndTurn, Chain(DYNA_TANK)], &[Activate(BOOK_OF_MOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, DYNA_TANK), Some(1), "Dyna Tank is still face-up Attack: {}", s.context());
    assert_eq!(s.position_of(0, DARK_BLADE), Some(8), "Dark Blade was turned face-down: {}", s.context());
}

/// "When a Level 6 or higher monster(s) is Normal or Special Summoned: Tribute 1 Level 10 "Earthbound" monster; destroy
/// as many monsters your opponent controls as possible, and if you do, inflict damage equal to the combined original
/// ATK of those." The opponent Tribute Summons Dark Magician Girl (Level 6, 2000 ATK; Yami makes it 2200 on the field):
/// it is destroyed and the opponent takes 2000. Uru, revived under a Field Spell (it is destroyed without one), is the Tribute.
#[test]
fn earthbound_release_destroys_and_burns() {
    let spawns = [put(0, SpellSet, EARTHBOUND_RELEASE), put(0, Grave, URU), put(0, Hand, YAMI), put(0, Hand, MONSTER_REBORN), put(1, Hand, DARK_MAGICIAN_GIRL), put(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.prefer(&[(0, URU), (0, KURIBOH)]);
    s.run_both([&[Activate(YAMI), Activate(MONSTER_REBORN), EndTurn, Chain(EARTHBOUND_RELEASE)], &[Summon(DARK_MAGICIAN_GIRL)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", URU), 1, "Uru was Tributed: {}", s.context());
    assert!(s.monster(1, DARK_MAGICIAN_GIRL).is_none(), "the summoned monster was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "its original ATK (Yami's 200 does not count): {}", s.context());
}

/// "Cannot be destroyed by battle. Your opponent's monsters cannot target for attacks any monsters you control, except
/// "Egyptian God Slime"." Blue-Eyes (3000) can only attack the Slime (3000 ATK): Warwolf is safe, the Slime survives the
/// tie and only Blue-Eyes is destroyed.
#[test]
fn egyptian_god_slime_draws_every_attack_and_survives_battle() {
    let spawns = [put(0, Monster, EGYPTIAN_GOD_SLIME), put(0, Monster, WARWOLF), put(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, EGYPTIAN_GOD_SLIME).is_some(), "the Slime was not destroyed by battle: {}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "Warwolf could not be attacked: {}", s.context());
    assert!(s.monster(1, BLUE_EYES).is_none(), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// "This card can be treated as 1 or 3 Tributes for the Tribute Summon of a monster." The Slime alone is enough for
/// Obelisk the Tormentor (three Tributes).
#[test]
fn egyptian_god_slime_counts_as_three_tributes() {
    let Some(mut s) = duel(false, &[], &[put(0, Monster, EGYPTIAN_GOD_SLIME), put(0, Hand, OBELISK)]) else { return };
    s.run(&[Summon(OBELISK)]).unwrap();
    clean(&s);
    assert!(s.monster(0, OBELISK).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", EGYPTIAN_GOD_SLIME), 1, "{}", s.context());
}

/// "During your opponent's Battle Phase (Quick Effect): You can banish this card from your GY; end the Battle Phase."
/// Blue-Eyes declares an attack on Warwolf; the Turtle ends the phase, and Warwolf takes no damage.
#[test]
fn electromagnetic_turtle_ends_the_battle_phase() {
    let spawns = [put(0, Grave, ELECTROMAGNETIC_TURTLE), put(0, Monster, WARWOLF), put(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(ELECTROMAGNETIC_TURTLE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", ELECTROMAGNETIC_TURTLE), 1, "{}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "the battle never happened: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// "The first time this face-up card on the field would be destroyed by an opponent's card effect, it is not
/// destroyed." One Mystical Space Typhoon leaves it, the second destroys it.
#[test]
fn empty_machine_survives_the_first_destruction_only() {
    let spawns = [put(0, Spell, EMPTY_MACHINE), put(1, Hand, MST), put(1, Hand, MST)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", EMPTY_MACHINE), 1, "the first MST failed to destroy it: {}", s.context());
    s.run_both([&[], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", EMPTY_MACHINE), 0, "the second one destroyed it: {}", s.context());
}

/// "Once per turn, you can activate 1 of these effects. ● Discard 1 Level 10 monster; draw 1 card."
#[test]
fn empty_machine_discards_a_level_10_to_draw() {
    let Some(mut s) = duel(false, &[], &[put(0, Spell, EMPTY_MACHINE), put(0, Hand, OBELISK)]) else { return };
    let (deck, before) = (s.state()[0]["deck"].as_u64().unwrap(), hand_size(&s, 0));
    s.run(&[Activate(EMPTY_MACHINE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", OBELISK), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before, "one card drawn for the one discarded: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_u64().unwrap(), deck - 1, "{}", s.context());
}

/// Anime Empty Machine: "If you control no monsters, you can Normal Summon Level 10 or higher monsters without
/// Tributing. The ATK of all monsters you control becomes 0."
#[test]
fn anime_empty_machine_summons_without_tribute_and_zeroes_attack() {
    let Some(mut s) = duel(true, &[], &[put(0, SpellSet, EMPTY_MACHINE), put(0, Hand, OBEDIENT_FIEND)]) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the Fiend
    s.run(&[EndTurn, Activate(EMPTY_MACHINE), Summon(OBEDIENT_FIEND)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, OBEDIENT_FIEND), Some(0), "{}", s.context());
}

/// "Shuffle 1 "Elemental HERO" or "Neo-Spacian" monster you control into the Deck, and if you do, Special Summon 1
/// "Elemental HERO" or "Neo-Spacian" monster with a different name from your Deck."
#[test]
fn en_shuffle_swaps_a_hero_for_another() {
    let deck = stacked(&[SPARKMAN]);
    let Some(mut s) = start(false, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &[put(0, Hand, EN_SHUFFLE), put(0, Monster, AVIAN)]) else { return };
    s.run(&[Activate(EN_SHUFFLE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, AVIAN).is_none(), "Avian went back into the Deck: {}", s.context());
    assert_eq!(s.attack_of(0, SPARKMAN), Some(1600), "Sparkman came from the Deck: {}", s.context());
}

/// "You can banish this card from your GY; shuffle from your GY into the Deck, either 1 "Elemental HERO" monster and 1
/// "Neo-Spacian" monster, OR 1 "Elemental HERO Neos", then draw 1 card."
#[test]
fn en_shuffle_in_the_graveyard_shuffles_two_and_draws() {
    let spawns = [put(0, Grave, EN_SHUFFLE), put(0, Grave, AVIAN), put(0, Grave, GRAND_MOLE)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run(&[Activate(EN_SHUFFLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", EN_SHUFFLE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", AVIAN) + s.count(0, "grave", GRAND_MOLE), 0, "both went into the Deck: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "and one card was drawn: {}", s.context());
}

/// Anime EN Shuffle: "Return 1 "Elemental HERO" monster you control to the Deck; Special Summon 1 "Neo-Spacian"
/// monster from your Deck, but its effects are negated."
#[test]
fn anime_en_shuffle_calls_a_neo_spacian() {
    let deck = stacked(&[GRAND_MOLE]);
    let Some(mut s) = start(true, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &[put(0, Hand, EN_SHUFFLE), put(0, Monster, AVIAN)]) else { return };
    s.run(&[Activate(EN_SHUFFLE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, AVIAN).is_none(), "{}", s.context());
    assert_eq!(s.attack_of(0, GRAND_MOLE), Some(900), "Grand Mole came from the Deck: {}", s.context());
}

/// "If this card is Normal or Special Summoned: You can Special Summon 1 Level 4 or lower Beast monster from your Deck in
/// face-down Defense Position, then if this card is in Attack Position, change it to Defense Position."
#[test]
fn eucalyptus_mole_sets_a_beast_and_turns_to_defense() {
    let deck = stacked(&[GIANT_RAT]);
    let Some(mut s) = start(false, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &[put(0, Hand, EUCALYPTUS_MOLE)]) else { return };
    s.run(&[Summon(EUCALYPTUS_MOLE), Chain(EUCALYPTUS_MOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, GIANT_RAT), Some(8), "Giant Rat is face-down Defense: {}", s.context());
    assert_eq!(s.position_of(0, EUCALYPTUS_MOLE), Some(4), "the Mole turned to Defense: {}", s.context());
}

/// "If this card is destroyed: You can Special Summon 1 "Koala" monster or 1 "Tree Otter" from your Deck." The
/// opponent's Dark Hole destroys the Mole; Tree Otter comes from the Deck.
#[test]
fn eucalyptus_mole_calls_tree_otter_when_destroyed() {
    let deck = stacked(&[TREE_OTTER]);
    let spawns = [put(0, Monster, EUCALYPTUS_MOLE), put(1, Hand, DARK_HOLE)];
    let Some(mut s) = start(false, 8000, [(&deck, &[]), (&[KURIBOH; 40], &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, Chain(EUCALYPTUS_MOLE)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, EUCALYPTUS_MOLE).is_none(), "{}", s.context());
    assert!(s.monster(0, TREE_OTTER).is_some(), "Tree Otter came from the Deck: {}", s.context());
}

/// "If you control an Illusion or Spellcaster monster: Activate 1 of these effects; ● Your Illusion and Spellcaster
/// monsters cannot be destroyed by battle this turn." Blue-Eyes attacks Dark Magician Girl; the first effect is chosen.
#[test]
fn eye_of_illusion_protects_a_spellcaster_in_battle() {
    let spawns = [put(0, SpellSet, EYE_OF_ILLUSION), put(0, Monster, DARK_MAGICIAN_GIRL), put(1, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(EYE_OF_ILLUSION)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_MAGICIAN_GIRL).is_some(), "{}", s.context());
    assert_eq!(s.lp(0), 7000, "the battle damage is still taken: {}", s.context());
}

/// "Your opponent cannot target ... in your Monster Zones with card effects, also those monsters cannot be destroyed by
/// your opponent's card effects while you control them." Dark Hole destroys Warwolf, not Uria.
#[test]
fn fallen_paradise_keeps_uria_from_the_opponents_effects() {
    let spawns = [put(0, Hand, FALLEN_PARADISE), put(0, Monster, URIA), put(0, Monster, WARWOLF), put(1, Hand, DARK_HOLE)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[Activate(FALLEN_PARADISE), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, URIA).is_some(), "{}", s.context());
    assert!(s.monster(0, WARWOLF).is_none(), "{}", s.context());
}

/// "If you control "Uria, Lord of Searing Flames" ... in your Monster Zone: You can draw 2 cards."
#[test]
fn fallen_paradise_draws_two_with_uria() {
    let spawns = [put(0, Hand, FALLEN_PARADISE), put(0, Monster, URIA)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run(&[Activate(FALLEN_PARADISE), Activate(FALLEN_PARADISE)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), before - 1 + 2, "Paradise left the hand, two cards came: {}", s.context());
}

/// "Must first be Special Summoned (from your hand) to your opponent's field in Defense Position during your Main Phase
/// 2, while an "Aesir" monster is on the field."
#[test]
fn fenrir_is_summoned_to_the_opponents_field_in_defense() {
    let Some(mut s) = duel(false, &[], &[put(0, Monster, THOR), put(0, Hand, FENRIR)]) else { return };
    s.prefer(&[(0, KURIBOH)]); // no Battle Phase on the first turn: the End Phase discard keeps Fenrir
    s.run(&[EndTurn, Attack(THOR), SpSummon(FENRIR)]).unwrap();
    clean(&s);
    assert!(s.monster(1, FENRIR).is_some(), "Fenrir is on the opponent's field: {}", s.context());
    assert_eq!(s.position_of(1, FENRIR), Some(4), "in Defense Position: {}", s.context());
}

/// "At the start of your Battle Phase: Change all Defense Position monsters you control to face-up Attack Position. Both
/// players take any battle damage from attacks involving this card." Fenrir (4000) attacks Thor (3500): both take 500.
#[test]
fn fenrir_turns_to_attack_and_both_take_the_damage() {
    let spawns = [put(0, Monster, THOR), put(1, MonsterDefense, FENRIR)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(FENRIR)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (7500, 7500), "{}", s.context());
}

/// "During either player's Battle Step, when a Fusion Monster you control battles an opponent's Fusion Monster: Target
/// both those Fusion Monsters; negate the attack, and if you do, each player takes damage equal to the combined ATK."
/// Cyber Twin Dragon (2800) attacks Flame Wingman (2100): both players lose 4900.
#[test]
fn final_fusion_negates_the_attack_and_burns_both() {
    let spawns = [put(0, SpellSet, FINAL_FUSION), put(0, Monster, FLAME_WINGMAN), put(1, Monster, CYBER_TWIN)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(FINAL_FUSION)], &[Attack(CYBER_TWIN)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (3100, 3100), "{}", s.context());
    assert!(s.monster(0, FLAME_WINGMAN).is_some() && s.monster(1, CYBER_TWIN).is_some(), "the attack was negated: {}", s.context());
}

/// "Pay LP in multiples of 1000, then target 1 "Valkyrie" monster in your GY with a different name for every 1000 LP
/// paid; Special Summon them, then your opponent can Special Summon monsters with 2000 or less ATK from their GY, up to
/// the number of monsters you Special Summoned." 2000 LP pay for Brunhilde and Dritte; the opponent's Warwolf answers.
#[test]
fn final_light_pays_for_valkyries_and_lets_the_opponent_answer() {
    let spawns = [put(0, Hand, FINAL_LIGHT), put(0, Grave, BRUNHILDE), put(0, Grave, DRITTE), put(1, Grave, WARWOLF), put(1, Grave, BLUE_EYES)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.declare_number(Some(2000));
    s.run(&[Activate(FINAL_LIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6000, "{}", s.context());
    assert!(s.monster(0, BRUNHILDE).is_some() && s.monster(0, DRITTE).is_some(), "both Valkyries came: {}", s.context());
    assert!(s.monster(1, WARWOLF).is_some(), "the opponent's Warwolf (2000 ATK) came: {}", s.context());
    assert!(s.monster(1, BLUE_EYES).is_none(), "Blue-Eyes (3000 ATK) is too strong: {}", s.context());
}

/// "Target 1 FIRE monster in your GY; send 1 card from your hand to the GY, and if you do, Special Summon the targeted
/// monster."
#[test]
fn fire_recovery_sends_a_card_to_revive_a_fire_monster() {
    let spawns = [put(0, Hand, FIRE_RECOVERY), put(0, Hand, WARWOLF), put(0, Grave, INPACHI)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run(&[Activate(FIRE_RECOVERY)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, INPACHI), Some(1850), "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was the card sent: {}", s.context());
}

/// "You can banish this card from your GY, then target 3 of your FIRE monsters that are banished and/or in your GY;
/// shuffle them into the Deck, then draw 1 card."
#[test]
fn fire_recovery_in_the_graveyard_shuffles_three_and_draws() {
    let spawns = [put(0, Grave, FIRE_RECOVERY), put(0, Grave, INPACHI), put(0, Grave, FIRE_KRAKEN), put(0, Grave, FOOTBALLER)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run(&[Activate(FIRE_RECOVERY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", FIRE_RECOVERY), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", INPACHI) + s.count(0, "grave", FIRE_KRAKEN) + s.count(0, "grave", FOOTBALLER), 0, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "one card was drawn: {}", s.context());
}

/// Anime Fire Recovery: "Discard 1 FIRE monster to target 1 FIRE monster in your Graveyard; Special Summon it."
#[test]
fn anime_fire_recovery_discards_a_fire_monster() {
    let spawns = [put(0, Hand, FIRE_RECOVERY), put(0, Hand, FIRE_KRAKEN), put(0, Grave, INPACHI)];
    let Some(mut s) = duel(true, &[], &spawns) else { return };
    s.prefer(&[(0, INPACHI)]); // the discarded Kraken is in the Graveyard too by the time the target is chosen
    s.run(&[Activate(FIRE_RECOVERY)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, INPACHI), Some(1850), "{}", s.context());
    assert_eq!(s.count(0, "grave", FIRE_KRAKEN), 1, "Fire Kraken was the discard: {}", s.context());
}

/// "If the only monster you control is 1 Level 5 monster: Tribute that monster; Special Summon 5 monsters (1 "Kuribah",
/// 1 "Kuribee", 1 "Kuriboo", 1 "Kuribeh", and 1 "Kuriboh") from your hand, Deck, and/or GY."
#[test]
fn five_star_twilight_brings_the_five_kuribohs() {
    let mut spawns = vec![put(0, Hand, FIVE_STAR_TWILIGHT), put(0, Monster, CYBER_DRAGON)];
    spawns.extend([KURIBAH, KURIBEE, KURIBOO, KURIBEH, KURIBOH].map(|c| put(0, Hand, c)));
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run(&[Activate(FIVE_STAR_TWILIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CYBER_DRAGON), 1, "{}", s.context());
    for c in [KURIBAH, KURIBEE, KURIBOO, KURIBEH, KURIBOH] {
        assert_eq!(s.count(0, "mzone", c), 1, "{c} is on the field: {}", s.context());
    }
}

/// "Fusion Summon 1 Fusion Monster from your Extra Deck, using monsters you control as Fusion Material. Destroy it during
/// the End Phase." Gaia the Dragon Champion from Gaia and Curse of Dragon, destroyed when the turn ends.
#[test]
fn flash_fusion_fuses_and_destroys_it_at_the_end_phase() {
    let spawns = [put(0, Hand, FLASH_FUSION), put(0, Monster, GAIA), put(0, Monster, CURSE_OF_DRAGON)];
    let Some(mut s) = duel(false, &[GAIA_CHAMPION], &spawns) else { return };
    s.run(&[Activate(FLASH_FUSION)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GAIA_CHAMPION), Some(2600), "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", GAIA_CHAMPION), 1, "destroyed in the End Phase: {}", s.context());
}

/// Anime Flash Fusion: "Return it to the Extra Deck during the End Phase of this turn."
#[test]
fn anime_flash_fusion_returns_the_monster_to_the_extra_deck() {
    let spawns = [put(0, Hand, FLASH_FUSION), put(0, Monster, GAIA), put(0, Monster, CURSE_OF_DRAGON)];
    let Some(mut s) = duel(true, &[GAIA_CHAMPION], &spawns) else { return };
    s.run(&[Activate(FLASH_FUSION), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "extra", GAIA_CHAMPION), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", GAIA_CHAMPION), 0, "{}", s.context());
}

/// "If this card is Special Summoned: You can discard 1 card, and if you do, send 1 "Fossil" Fusion Monster from your
/// Extra Deck to the GY." Monster Reborn brings Flint Cragger back; Warwolf is discarded for Skullgar.
#[test]
fn flint_cragger_sends_a_fossil_fusion_monster() {
    let spawns = [put(0, Hand, MONSTER_REBORN), put(0, Hand, WARWOLF), put(0, Grave, FLINT_CRAGGER)];
    let Some(mut s) = duel(false, &[SKULLGAR], &spawns) else { return };
    s.prefer(&[(0, FLINT_CRAGGER), (0, WARWOLF), (0, SKULLGAR)]);
    s.run(&[Activate(MONSTER_REBORN), Chain(FLINT_CRAGGER)]).unwrap();
    clean(&s);
    assert!(s.monster(0, FLINT_CRAGGER).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", SKULLGAR), 1, "Skullgar was sent from the Extra Deck: {}", s.context());
}

/// "You can send this card from the field to the GY; inflict 500 damage to your opponent."
#[test]
fn flint_cragger_burns_for_500_from_the_field() {
    let Some(mut s) = duel(false, &[], &[put(0, Monster, FLINT_CRAGGER)]) else { return };
    s.run(&[Activate(FLINT_CRAGGER)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7500, "{}", s.context());
    assert_eq!(s.count(0, "grave", FLINT_CRAGGER), 1, "{}", s.context());
}

/// "Once per opponent's turn, if this card would be destroyed by their card effect, it is not destroyed." Dark Hole
/// destroys Warwolf but not the Elephant.
#[test]
fn flying_elephant_survives_an_opponents_effect() {
    let spawns = [put(0, Monster, FLYING_ELEPHANT), put(0, Monster, WARWOLF), put(1, Hand, DARK_HOLE)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, FLYING_ELEPHANT).is_some(), "{}", s.context());
    assert!(s.monster(0, WARWOLF).is_none(), "{}", s.context());
}

/// "You can send this face-up card you control to the Graveyard, then target 1 face-up monster you control": the cost
/// is paid and the target chosen.
#[test]
fn force_resonator_goes_to_the_graveyard_for_its_effect() {
    let Some(mut s) = duel(false, &[], &[put(0, Monster, FORCE_RESONATOR), put(0, Monster, WARWOLF)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(FORCE_RESONATOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FORCE_RESONATOR), 1, "{}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "{}", s.context());
}

/// "if it attacks this turn, your opponent cannot activate cards or effects that target a monster(s) until the end of
/// the Damage Step." Control: without the Resonator the opponent's Set Book of Moon answers the attack; with it, the
/// answer never becomes possible. The second player uses it (the first can't attack in its first turn, and the effect
/// lasts only this turn).
#[test]
fn force_resonator_stops_the_opponent_targeting_an_attacker() {
    let spawns = [put(1, Monster, WARWOLF), put(0, SpellSet, BOOK_OF_MOON)];
    let Some(mut control) = duel(false, &[], &spawns) else { return };
    control.run_both([&[ChainOnAttack(BOOK_OF_MOON)], &[Attack(WARWOLF)]]).unwrap();
    clean(&control);
    assert_eq!(control.count(0, "grave", BOOK_OF_MOON), 1, "Book of Moon was used: {}", control.context());
    let spawns = [put(1, Monster, FORCE_RESONATOR), put(1, Monster, WARWOLF), put(0, SpellSet, BOOK_OF_MOON)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    let result = s.run_both([&[ChainOnAttack(BOOK_OF_MOON)], &[Activate(FORCE_RESONATOR), Attack(WARWOLF)]]);
    clean(&s);
    assert!(result.as_ref().is_err_and(|e| e.contains("never became possible")), "the opponent could not use Book of Moon: {result:?}\n{}", s.context());
    assert_eq!(s.count(0, "grave", BOOK_OF_MOON), 0, "{}", s.context());
}

/// "Once per turn: You can Special Summon 1 "Meklord Army" monster from your hand."
#[test]
fn fortissimo_summons_a_meklord_army_monster() {
    let Some(mut s) = duel(false, &[], &[put(0, Hand, FORTISSIMO), put(0, Hand, GRANEL)]) else { return };
    s.run(&[Activate(FORTISSIMO), Activate(FORTISSIMO)]).unwrap();
    clean(&s);
    assert!(s.monster(0, GRANEL).is_some(), "{}", s.context());
}

/// Anime Fortissimo: "Once per turn, during their Main Phase, the turn player can Special Summon 1 Level 4 or lower
/// Machine-Type monster from their hand."
#[test]
fn anime_fortissimo_summons_a_machine_from_the_hand() {
    let Some(mut s) = duel(true, &[], &[put(0, Spell, FORTISSIMO), put(0, Hand, STEAMROID)]) else { return };
    s.run(&[Activate(FORTISSIMO)]).unwrap();
    clean(&s);
    assert!(s.monster(0, STEAMROID).is_some(), "{}", s.context());
}

/// "Once per turn, you can either: Target 1 Fairy monster you control; equip this card to that target." Brunhilde is the
/// Fairy; the Chariot leaves the Monster Zone for the Spell & Trap Zone.
#[test]
fn fortune_chariot_equips_to_a_fairy() {
    let Some(mut s) = duel(false, &[], &[put(0, Monster, FORTUNE_CHARIOT), put(0, Monster, BRUNHILDE)]) else { return };
    s.run(&[Activate(FORTUNE_CHARIOT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", FORTUNE_CHARIOT), 1, "{}", s.context());
    assert!(s.monster(0, FORTUNE_CHARIOT).is_none(), "{}", s.context());
}

/// "The equipped monster can attack directly, but when it does so using this effect, the battle damage inflicted to
/// your opponent is halved." Brunhilde (1800) attacks past the opponent's Warwolf for 900.
#[test]
fn fortune_chariot_gives_a_halved_direct_attack() {
    let spawns = [put(0, Monster, FORTUNE_CHARIOT), put(0, Monster, BRUNHILDE), put(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run(&[Activate(FORTUNE_CHARIOT), Attack(BRUNHILDE)]).unwrap();
    clean(&s);
    // Brunhilde gains 500 ATK for each monster the opponent controls: 2300, halved.
    assert_eq!(s.lp(1), 8000 - 1150, "{}", s.context());
}

/// "If the equipped monster would be destroyed by battle or card effect, destroy this card instead."
#[test]
fn fortune_chariot_is_destroyed_in_place_of_the_monster() {
    let spawns = [put(0, Monster, FORTUNE_CHARIOT), put(0, Monster, DRITTE), put(1, Hand, DARK_HOLE)];
    let Some(mut s) = duel(false, &[], &spawns) else { return };
    s.run_both([&[Activate(FORTUNE_CHARIOT), EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DRITTE).is_some(), "Dritte stayed: {}", s.context());
    assert_eq!(s.count(0, "grave", FORTUNE_CHARIOT), 1, "{}", s.context());
}

/// "When you draw this card: You can reveal it; Special Summon it from your hand. If this card is Special Summoned from
/// the hand: You can banish 1 Spellcaster monster from your Deck." The second player's Deck is Anns and one Dark
/// Magician: the Draw Phase's Ann comes out and banishes it.
#[test]
fn fortune_fairy_ann_comes_on_the_draw_and_banishes_a_spellcaster() {
    let deck: Vec<u16> = [FORTUNE_FAIRY_ANN; 30].into_iter().chain([DARK_MAGICIAN; 10]).collect();
    let Some(mut s) = start(false, 8000, [(&[KURIBOH; 40], &[]), (&deck, &[])], &[]) else { return };
    s.run_both([&[EndTurn], &[Chain(FORTUNE_FAIRY_ANN), Chain(FORTUNE_FAIRY_ANN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", FORTUNE_FAIRY_ANN), 1, "{}", s.context());
    let banished = s.count(1, "removed", DARK_MAGICIAN) + s.count(1, "removed", FORTUNE_FAIRY_ANN);
    assert_eq!(banished, 1, "one Spellcaster was banished from the Deck: {}", s.context());
}
