//! Scenario tests for the first group of the anime's printed cards (rows 4848-5205 of engine/added/cards.tsv): the
//! Advanced Crystal Beasts and Advanced Dark, then the cards from "Afterglow" to "Battle of Sleeping Spirits". Each
//! through the real core with both seats' client models checked after every step (`duelcheck::scenario`). Texts: the
//! printed ones (anime cards off) from the pinned official database; an anime version is tested with anime cards on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The added cards of this group (client ids from the group list).
const AMBER: u16 = 4781; // Advanced Crystal Beast Amber Mammoth
const AMETHYST: u16 = 4730; // Advanced Crystal Beast Amethyst Cat
const COBALT: u16 = 4782; // Advanced Crystal Beast Cobalt Eagle
const EMERALD: u16 = 4765; // Advanced Crystal Beast Emerald Tortoise
const RUBY: u16 = 4726; // Advanced Crystal Beast Ruby Carbuncle
const SAPPHIRE: u16 = 4783; // Advanced Crystal Beast Sapphire Pegasus
const TOPAZ: u16 = 4779; // Advanced Crystal Beast Topaz Tiger
const ADVANCED_DARK: u16 = 10214;
const AFTERGLOW: u16 = 15375;
const AMULET_DRAGON: u16 = 10325;
const ANCIENT_DRAGON: u16 = 10116;
const ANCIENT_GEAR_STATUE: u16 = 5074;
const ANTI_MAGIC_ARROWS: u16 = 4831;
const APPLE: u16 = 14362; // Apple of Enlightenment
const HANGMAN: u16 = 5160; // Arcana Force XII - The Hangman
const FIEND: u16 = 4833; // Arcana Force XV - The Fiend
const WHITE_BEAR: u16 = 15374; // Armored White Bear
const ASHOKA_PILLAR: u16 = 4995;
const ASSAULT_DOG: u16 = 11178;
const ATTACK_GUIDANCE_ARMOR: u16 = 5487;
const AYERS_ROCK_SUNRISE: u16 = 11165;
const BATTLE_FUSION: u16 = 11162;
const BATTLE_OF_SLEEPING_SPIRITS: u16 = 4721;
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const CB_AMETHYST: u16 = 7071; // Crystal Beast Amethyst Cat
const CB_COBALT: u16 = 7077; // Crystal Beast Cobalt Eagle
const POLYMERIZATION: u16 = 4837;
const DARK_MAGICIAN: u16 = 4041;
const BLUE_EYES: u16 = 4007;
const MIRROR_FORCE: u16 = 4887;
const ANCIENT_GEAR_GOLEM: u16 = 6315; // cannot be Special Summoned
const VALKYRIE_DRITTE: u16 = 13992; // a "Valkyrie" monster
const VALKYRIE_ZWEITE: u16 = 13993;
const STARDUST: u16 = 7734; // Stardust Dragon, a Synchro Monster
const YAMI: u16 = 4341; // a Field Spell
const SILVER_FANG: u16 = 4071; // a Level 3 Beast
const AXE_OF_DESPAIR: u16 = 4310; // an Equip Spell
const LIGHT_BARRIER: u16 = 7624;
const POT_OF_GREED: u16 = 4844;
const GAZELLE: u16 = 4753; // a Level 4 Beast

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn start(anime: bool, main: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    start_seed(7, anime, main, spawns)
}

fn start_seed(seed: u64, anime: bool, main: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(main[0], &[][..]), (main[1], &[][..])];
    Some(if anime { Scenario::start_anime_field(&dir, seed, 8000, decks, spawns) } else { Scenario::start_field(&dir, seed, 8000, decks, spawns) }.unwrap())
}

/// A duel with Kuriboh decks and `spawns` in place; anime cards off.
fn printed(spawns: &[Spawn]) -> Option<Scenario> {
    start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], spawns)
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

/// Advanced Crystal Beast Topaz Tiger: "All "Advanced Crystal Beast" monsters you control gain 400 ATK/DEF, also monsters
/// your opponent controls lose 400 ATK/DEF."
#[test]
fn topaz_tiger_raises_its_own_and_lowers_the_opponents() {
    let spawns = [at(0, Hand, ADVANCED_DARK), at(0, Hand, TOPAZ), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(ADVANCED_DARK), Summon(TOPAZ)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, TOPAZ), Some(2000), "1600 and 400: {}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(1600), "2000 less 400: {}", s.context());
}

/// "If "Advanced Dark" is not in a Field Zone, send this monster to the GY." (all seven share it; Topaz Tiger shows it.)
#[test]
fn an_advanced_crystal_beast_goes_to_the_graveyard_without_advanced_dark() {
    let Some(mut s) = printed(&[at(0, Hand, TOPAZ)]) else { return };
    s.run(&[Summon(TOPAZ)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TOPAZ), 1, "{}", s.context());
    assert!(s.monster(0, TOPAZ).is_none(), "{}", s.context());
}

/// Advanced Crystal Beast Amethyst Cat: "Your "Advanced Crystal Beast" monsters can attack directly, but when they do so
/// using this effect, the battle damage inflicted to your opponent is halved."
#[test]
fn amethyst_cat_attacks_directly_for_half() {
    let spawns = [at(0, Hand, ADVANCED_DARK), at(0, Hand, AMETHYST), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(ADVANCED_DARK), Summon(AMETHYST), EndTurn, Attack(AMETHYST)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7400, "1200 halved: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "the monster was passed by: {}", s.context());
}

/// Advanced Crystal Beast Cobalt Eagle: "You can send this card from your hand or field to the GY; add 1 "Advanced Dark"
/// from your Deck to your hand."
#[test]
fn cobalt_eagle_sends_itself_to_fetch_advanced_dark() {
    let deck: Vec<u16> = [ADVANCED_DARK].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &[at(0, Hand, COBALT)]) else { return };
    s.run(&[Activate(COBALT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", COBALT), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", ADVANCED_DARK), 1, "{}", s.context());
}

/// Cobalt Eagle: "Once per turn: You can target 1 "Advanced Crystal Beast" card you control; return it to the hand or
/// place it on top of the Deck." (no Advanced Dark in the Deck, so the other effect is not on offer.)
#[test]
fn cobalt_eagle_returns_an_advanced_crystal_beast() {
    let spawns = [at(0, Spell, ADVANCED_DARK), at(0, Monster, COBALT), at(0, Monster, TOPAZ)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, TOPAZ)]);
    s.run(&[Activate(COBALT)]).unwrap();
    clean(&s);
    assert!(s.monster(0, TOPAZ).is_none(), "Topaz Tiger left the field: {}", s.context());
    assert!(s.monster(0, COBALT).is_some(), "{}", s.context());
}

/// Advanced Crystal Beast Emerald Tortoise: "Once per turn (Quick Effect): You can target 1 face-up monster on the field;
/// change its battle position."
#[test]
fn emerald_tortoise_changes_a_battle_position() {
    let spawns = [at(0, Spell, ADVANCED_DARK), at(0, Monster, EMERALD), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(EMERALD)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "face-up Defense: {}", s.context());
}

/// Advanced Crystal Beast Ruby Carbuncle: "If this face-up card is destroyed in a Monster Zone, you can place it face-up
/// in your Spell & Trap Zone as a Continuous Spell ... While this card is treated as a Continuous Spell: You can Special
/// Summon this card, then you can Special Summon as many "Advanced Crystal Beast" Monster Cards as possible from your
/// Spell & Trap Zones." Dark Hole destroys Ruby and Topaz Tiger; both wait in the Spell & Trap Zone; Ruby brings both back.
#[test]
fn ruby_carbuncle_brings_the_monster_cards_back() {
    let spawns = [at(0, Spell, ADVANCED_DARK), at(0, Hand, DARK_HOLE), at(0, Monster, RUBY), at(0, Monster, TOPAZ)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", RUBY), 1, "Ruby is a Continuous Spell: {}", s.context());
    assert_eq!(s.count(0, "szone", TOPAZ), 1, "Topaz Tiger too: {}", s.context());
    s.run(&[Activate(RUBY)]).unwrap();
    clean(&s);
    assert!(s.monster(0, RUBY).is_some() && s.monster(0, TOPAZ).is_some(), "both Special Summoned: {}", s.context());
}

/// Advanced Crystal Beast Sapphire Pegasus: "If this card is Summoned: You can place 1 of your "Advanced Crystal Beast"
/// monsters that is banished, or in your hand, Deck, or GY, face-up in your Spell & Trap Zone as a Continuous Spell."
#[test]
fn sapphire_pegasus_places_one_from_the_graveyard() {
    let spawns = [at(0, Hand, ADVANCED_DARK), at(0, Hand, SAPPHIRE), at(0, Grave, TOPAZ)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(ADVANCED_DARK), Summon(SAPPHIRE), Chain(SAPPHIRE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", TOPAZ), 1, "Topaz Tiger sits in the Spell & Trap Zone: {}", s.context());
    assert_eq!(s.count(0, "grave", TOPAZ), 0, "{}", s.context());
}

/// Advanced Crystal Beast Amber Mammoth: "Once per turn, when your "Advanced Crystal Beast" monster is targeted for an
/// attack: You can negate that attack." Warwolf (2000) attacks Amber Mammoth (1700): no damage, Amber stays.
#[test]
fn amber_mammoth_negates_an_attack() {
    let spawns = [at(0, Spell, ADVANCED_DARK), at(0, Monster, AMBER), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn, Chain(AMBER)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, AMBER).is_some(), "Amber Mammoth was not destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Amber Mammoth: "Once per turn, when your opponent activates a card or effect that targets your "Advanced Crystal
/// Beast" card(s) or "Advanced Dark" (Quick Effect): You can negate the activation." Mystical Space Typhoon aimed at
/// Advanced Dark is negated.
#[test]
fn amber_mammoth_negates_a_targeting_activation() {
    let spawns = [at(0, Spell, ADVANCED_DARK), at(0, Monster, AMBER), at(1, Hand, MST)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, ADVANCED_DARK)]);
    s.run_both([&[EndTurn, Chain(AMBER)], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", ADVANCED_DARK), 1, "Advanced Dark stays: {}", s.context());
    assert!(s.monster(0, AMBER).is_some(), "{}", s.context());
}

/// Advanced Dark: "During damage calculation, if a "Crystal Beast" monster you control battles and you would take damage:
/// You can send 1 "Crystal Beast" monster from your Deck to the GY; you take no battle damage from that battle."
#[test]
fn advanced_dark_prevents_battle_damage_for_a_crystal_beast_from_the_deck() {
    let deck: Vec<u16> = [CB_COBALT].into_iter().chain([KURIBOH; 39]).collect();
    let spawns = [at(0, Hand, ADVANCED_DARK), at(0, Monster, CB_AMETHYST), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[Activate(ADVANCED_DARK), EndTurn, Chain(ADVANCED_DARK)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "no battle damage: {}", s.context());
    assert_eq!(s.count(0, "grave", CB_COBALT), 1, "the cost: {}", s.context());
}
/// The anime Crystal Beast Topaz Tiger (anime cards on): "If this card attacks an opponent's monster, it gains 400 ATK during
/// the Damage Step only." No field-wide boost: Warwolf stays at 2000, Topaz Tiger (1600 + 400) trades with it.
#[test]
fn anime_topaz_tiger_gains_400_only_when_it_attacks() {
    let spawns = [at(0, Spell, ADVANCED_DARK), at(0, Monster, TOPAZ), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    assert_eq!(s.attack_of(1, WARWOLF), Some(2000), "no 400 loss: {}", s.context());
    assert_eq!(s.attack_of(0, TOPAZ), Some(1600), "no 400 gain: {}", s.context());
    s.run(&[EndTurn, Attack(TOPAZ)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "2000 against 2000 destroys both: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
}

/// The test harness: a Field Spell spawned on the field is in the Field Zone, where it applies (skills.lua
/// `YO3Skill.Spawn`): with Advanced Dark there, a spawned Advanced Crystal Beast stays.
#[test]
fn a_spawned_field_spell_applies_from_the_field_zone() {
    let spawns = [at(0, Spell, ADVANCED_DARK), at(0, Monster, TOPAZ)];
    let Some(mut s) = start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", TOPAZ), s.count(0, "grave", TOPAZ)), (1, 0), "{}", s.context());
}

/// The anime Crystal Beast Emerald Tortoise: "Once per turn: You can change 1 monster you control that attacked this turn
/// to Defense Position." Topaz Tiger attacks, then the Tortoise puts it in Defense Position in the Main Phase 2.
#[test]
fn anime_emerald_tortoise_turns_an_attacker_to_defense() {
    // Advanced Dark activated from the hand (a Field Spell spawned on the field doesn't reach the Field Zone), then
    // the Advanced Crystal Beasts summoned, which would otherwise destroy themselves at once.
    let spawns = [at(0, Hand, ADVANCED_DARK), at(0, Hand, TOPAZ), at(0, Hand, EMERALD), at(1, Monster, KURIBOH)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, TOPAZ), (0, KURIBOH)]); // Topaz for the Tortoise, a Kuriboh for the End Phase discard
    s.run(&[Activate(ADVANCED_DARK), Summon(TOPAZ), EndTurn, Summon(EMERALD), Attack(TOPAZ), Activate(EMERALD)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, TOPAZ), Some(4), "{}", s.context());
}

/// Afterglow (printed): "Banish this card, and as many "Afterglow" from your hand, Deck, GY, and face-up field as
/// possible, then shuffle 1 of your banished "Afterglow" into the Deck. During your next Draw Phase, reveal the card you
/// draw for your normal draw, and inflict 4000 damage to your opponent if it is "Afterglow"." The Deck is all Afterglow,
/// so the one shuffled back in is the next draw.
#[test]
fn afterglow_banishes_them_all_and_the_draw_hurts() {
    let Some(mut s) = start(false, [&[AFTERGLOW; 40], &[KURIBOH; 40]], &[]) else { return };
    s.run(&[Activate(AFTERGLOW)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", AFTERGLOW), 39, "all but the one shuffled back: {}", s.context());
    assert_eq!(s.count(0, "hand", AFTERGLOW), 0, "{}", s.context());
    s.run(&[EndTurn, EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 4000, "the draw was Afterglow: {}", s.context());
}
/// Amulet Dragon: "If this card is Special Summoned: Target any number of Spells in any GY(s) (min.1); banish those
/// targets, and if you do, this card gains 100 ATK for each card banished by this effect." Fusion Summoned with
/// Polymerization from Dark Magician and Blue-Eyes, over a Spell in each Graveyard.
#[test]
fn amulet_dragon_banishes_spells_and_grows() {
    let spawns = [at(0, Hand, POLYMERIZATION), at(0, Hand, DARK_MAGICIAN), at(0, Hand, BLUE_EYES), at(0, Grave, MONSTER_REBORN),
        at(1, Grave, DARK_HOLE)];
    let Some(dir) = engine_dir() else { return };
    let decks = [(&[KURIBOH; 40][..], &[AMULET_DRAGON][..]), (&[KURIBOH; 40][..], &[][..])];
    let mut s = Scenario::start_field(&dir, 7, 8000, decks, &spawns).unwrap();
    s.prefer(&[(0, DARK_MAGICIAN), (0, BLUE_EYES), (0, MONSTER_REBORN), (1, DARK_HOLE)]);
    s.run(&[Activate(POLYMERIZATION)]).unwrap();
    clean(&s);
    assert!(s.monster(0, AMULET_DRAGON).is_some(), "Fusion Summoned: {}", s.context());
    let banished = s.count(0, "removed", MONSTER_REBORN) + s.count(1, "removed", DARK_HOLE);
    assert!(banished >= 1, "a Spell was banished: {}", s.context());
    assert_eq!(s.attack_of(0, AMULET_DRAGON), Some(2900 + 100 * banished as i64), "{}", s.context());
}

/// Amulet Dragon: "If this card is destroyed: You can target 1 Spellcaster monster in your GY; Special Summon that
/// target." Dark Hole destroys it; Dark Magician comes back from the Graveyard.
#[test]
fn amulet_dragon_revives_a_spellcaster_when_destroyed() {
    let spawns = [at(0, Hand, DARK_HOLE), at(0, Monster, AMULET_DRAGON), at(0, Grave, DARK_MAGICIAN)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(AMULET_DRAGON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", AMULET_DRAGON), 1, "{}", s.context());
    assert!(s.monster(0, DARK_MAGICIAN).is_some(), "Dark Magician was Special Summoned: {}", s.context());
}

/// Ancient Dragon: "When this card inflicts battle damage to your opponent by a direct attack: You can increase this
/// card's Level by 1 and ATK by 500."
#[test]
fn ancient_dragon_grows_after_a_direct_attack() {
    let Some(mut s) = printed(&[at(0, Monster, ANCIENT_DRAGON)]) else { return };
    let (level, attack) = (s.level_of(0, ANCIENT_DRAGON).unwrap(), s.attack_of(0, ANCIENT_DRAGON).unwrap());
    s.run(&[EndTurn, Attack(ANCIENT_DRAGON), Chain(ANCIENT_DRAGON)]).unwrap();
    clean(&s);
    assert!(s.lp(1) < 8000, "the direct attack landed: {}", s.context());
    assert_eq!(s.level_of(0, ANCIENT_DRAGON), Some(level + 1), "{}", s.context());
    assert_eq!(s.attack_of(0, ANCIENT_DRAGON), Some(attack + 500), "{}", s.context());
}

/// Ancient Gear Statue: "If your opponent controls more monsters than you do, you can Special Summon this card (from your
/// hand)." Then: "You can Tribute this card; Special Summon from your hand or Deck, 1 "Ancient Gear Golem" ..., ignoring
/// its Summoning conditions" (the Golem cannot be Special Summoned otherwise).
#[test]
fn ancient_gear_statue_comes_and_brings_the_golem() {
    let spawns = [at(0, Hand, ANCIENT_GEAR_STATUE), at(0, Hand, ANCIENT_GEAR_GOLEM), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[SpSummon(ANCIENT_GEAR_STATUE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, ANCIENT_GEAR_STATUE).is_some(), "Special Summoned from the hand: {}", s.context());
    s.run(&[Activate(ANCIENT_GEAR_STATUE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, ANCIENT_GEAR_GOLEM).is_some(), "the Golem came: {}", s.context());
    assert!(s.monster(0, ANCIENT_GEAR_STATUE).is_none(), "the Statue was Tributed: {}", s.context());
}

/// The anime Ancient Gear Statue (anime cards on): "You can Tribute this card; Special Summon 1 "Ancient Gear Golem" from
/// your hand, ignoring its Summoning conditions."
#[test]
fn anime_ancient_gear_statue_brings_the_golem_from_the_hand() {
    let spawns = [at(0, Monster, ANCIENT_GEAR_STATUE), at(0, Hand, ANCIENT_GEAR_GOLEM)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run(&[Activate(ANCIENT_GEAR_STATUE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, ANCIENT_GEAR_GOLEM).is_some(), "{}", s.context());
    assert!(s.monster(0, ANCIENT_GEAR_STATUE).is_none(), "{}", s.context());
}

/// Anti-Magic Arrows (printed): "At the start of the Battle Phase: For the rest of this turn after this card resolves,
/// neither player can activate Spell/Trap Cards, or their effects." Without it the opponent's Set Mirror Force gets its
/// chance at the attack; with it, it does not.
#[test]
fn anti_magic_arrows_stops_the_opponents_trap_in_the_battle_phase() {
    use yo3_server::ocg::{Response, msg};
    let chain_of = |team: u64, cid: u16| {
        move |p: &serde_json::Value| {
            p["type"] == msg::SELECT_CHAIN && p["player"] == team && p["selects"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
        }
    };
    // Control: no Arrows, Mirror Force is offered.
    let spawns = [at(0, Monster, WARWOLF), at(1, SpellSet, MIRROR_FORCE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_to(&[EndTurn, Attack(WARWOLF)], chain_of(1, MIRROR_FORCE)).unwrap();
    assert!(chain_of(1, MIRROR_FORCE)(s.prompt()), "Mirror Force is on offer at the attack: {}", s.context());
    // With Arrows chained at the start of the Battle Phase.
    let spawns = [at(0, Monster, WARWOLF), at(0, SpellSet, ANTI_MAGIC_ARROWS), at(1, SpellSet, MIRROR_FORCE)];
    let mut s = printed(&spawns).unwrap();
    s.run_to(&[EndTurn, Attack(WARWOLF)], chain_of(0, ANTI_MAGIC_ARROWS)).unwrap();
    let i = s.prompt()["selects"].as_array().unwrap().iter().position(|c| c["cid"] == ANTI_MAGIC_ARROWS).unwrap();
    s.respond(Response::Chain(Some(i)).json()).unwrap();
    s.run_to(&[Attack(WARWOLF)], chain_of(1, MIRROR_FORCE)).unwrap();
    clean(&s);
    assert!(!chain_of(1, MIRROR_FORCE)(s.prompt()), "no chance for Mirror Force: {}", s.context());
    assert_eq!(s.lp(1), 6000, "the attack went through: {}", s.context());
    assert_eq!(s.count(1, "szone", MIRROR_FORCE), 1, "{}", s.context());
}

/// The anime Anti-Magic Arrows (anime cards on): "Your opponent cannot activate face-down Spell or Trap Cards this turn."
/// The opponent (team 0) has a Set Mirror Force and a plan to chain it at the attack; after Arrows the chance never
/// comes. Without Arrows (the control) it does, and Warwolf is destroyed.
#[test]
fn anime_anti_magic_arrows_stops_face_down_cards() {
    let plan1: &[yo3_server::duelcheck::scenario::Action] = &[Activate(ANTI_MAGIC_ARROWS), Attack(WARWOLF)];
    let spawns = [at(0, SpellSet, MIRROR_FORCE), at(1, Monster, WARWOLF), at(1, Hand, ANTI_MAGIC_ARROWS)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    let err = s.run_both([&[EndTurn, ChainOnAttack(MIRROR_FORCE)], plan1]).unwrap_err();
    assert!(err.contains("ChainOnAttack(4887) never became possible"), "Mirror Force was not on offer: {err}");
    assert_eq!(s.lp(0), 6000, "the attack went through: {}", s.context());
    assert_eq!(s.count(0, "szone", MIRROR_FORCE), 1, "{}", s.context());
    // Control: the same attack without Arrows meets Mirror Force.
    let mut s = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns[..2]).unwrap();
    s.run_both([&[EndTurn, ChainOnAttack(MIRROR_FORCE)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Mirror Force destroyed the attacker: {}", s.context());
}

/// Apple of Enlightenment (printed): "Target cards in your opponent's GY, up to the number of "Valkyrie" monsters you
/// control; banish them." One Valkyrie, two cards in their Graveyard: one is banished.
#[test]
fn apple_of_enlightenment_banishes_up_to_the_valkyries() {
    let spawns = [at(0, SpellSet, APPLE), at(0, Monster, VALKYRIE_DRITTE), at(1, Grave, WARWOLF), at(1, Grave, BLUE_EYES)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[EndTurn, Activate(APPLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "only one target for one Valkyrie: {}", s.context());
}

/// Apple (printed): "If this Set card in its owner's control has left the field because of an opponent's effect, and is
/// now in the GY or banished: You can draw cards equal to the number of "Valkyrie" monsters you control +1." Two
/// Valkyries, Mystical Space Typhoon on the Set Apple: three cards, against a control duel without the destruction.
#[test]
fn apple_of_enlightenment_draws_when_destroyed_while_set() {
    let deck_after = |hit: bool| -> u64 {
        let spawns = [at(0, SpellSet, APPLE), at(0, Monster, VALKYRIE_DRITTE), at(0, Monster, VALKYRIE_ZWEITE), at(1, Hand, MST)];
        let mut s = printed(&spawns).unwrap();
        s.prefer(&[(0, APPLE)]);
        if hit {
            s.run_both([&[EndTurn, Chain(APPLE)], &[Activate(MST)]]).unwrap();
        } else {
            s.run_both([&[EndTurn], &[]]).unwrap();
        }
        clean(&s);
        s.state()[0]["deck"].as_u64().unwrap()
    };
    assert_eq!(deck_after(false) - deck_after(true), 3, "two Valkyries and one");
}

/// The anime Apple of Enlightenment (anime cards on): "If this card leaves the field: Draw 1 card."
#[test]
fn anime_apple_of_enlightenment_draws_one_when_it_leaves() {
    let deck_after = |hit: bool| -> u64 {
        let spawns = [at(0, SpellSet, APPLE), at(0, Monster, VALKYRIE_DRITTE), at(1, Hand, MST)];
        let mut s = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns).unwrap();
        s.prefer(&[(0, APPLE)]);
        if hit {
            s.run_both([&[EndTurn], &[Activate(MST)]]).unwrap();
        } else {
            s.run_both([&[EndTurn], &[]]).unwrap();
        }
        clean(&s);
        s.state()[0]["deck"].as_u64().unwrap()
    };
    assert_eq!(deck_after(false) - deck_after(true), 1);
}

/// Arcana Force XII - The Hangman: "When a card or effect is activated (Quick Effect): You can reveal this card in your
/// hand; Special Summon 1 "Arcana Force" monster from your hand in Defense Position." Hangman is itself the only Arcana
/// Force in the hand, so it comes in Defense Position (seed 1: its coin toss is tails, with nothing of the opponent's to hit).
#[test]
fn hangman_summons_an_arcana_force_from_the_hand_in_defense() {
    let spawns = [at(0, Hand, HANGMAN), at(1, Hand, POT_OF_GREED)];
    let Some(mut s) = start_seed(1, false, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.prefer_position(Some(4));
    s.run_both([&[EndTurn, Chain(HANGMAN)], &[Activate(POT_OF_GREED)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, HANGMAN), Some(4), "{}", s.context());
}

/// Hangman: "If this card is Summoned: Toss a coin. Heads: Destroy 1 monster you control, and if you do, take damage equal
/// to its original ATK." Seed 4 tosses heads: Warwolf (the one other monster, after the Tribute) goes, 2000 damage.
#[test]
fn hangman_heads_destroys_your_monster_and_hurts_you() {
    let spawns = [at(0, Hand, HANGMAN), at(0, Monster, KURIBOH), at(0, Monster, WARWOLF), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = start_seed(4, false, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (0, WARWOLF), (1, BLUE_EYES)]);
    s.run(&[Summon(HANGMAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 6000, "its original ATK: {}", s.context());
    assert_eq!(s.count(1, "mzone", BLUE_EYES), 1, "the opponent's monster is safe: {}", s.context());
}

/// Hangman: "Tails: Destroy 1 monster your opponent controls, and if you do, inflict damage to your opponent equal to
/// its original ATK." Seed 1 tosses tails: Blue-Eyes goes, 3000 damage.
#[test]
fn hangman_tails_destroys_their_monster_and_hurts_them() {
    let spawns = [at(0, Hand, HANGMAN), at(0, Monster, KURIBOH), at(0, Monster, WARWOLF), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = start_seed(1, false, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (0, WARWOLF), (1, BLUE_EYES)]);
    s.run(&[Summon(HANGMAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(s.lp(1), 5000, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Arcana Force XV - The Fiend: "You can discard this card; add 1 "Light Barrier" from your Deck or GY to your hand."
#[test]
fn fiend_discards_to_fetch_light_barrier() {
    let deck: Vec<u16> = [LIGHT_BARRIER].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &[at(0, Hand, FIEND)]) else { return };
    s.run(&[Activate(FIEND)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FIEND), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", LIGHT_BARRIER), 1, "{}", s.context());
}

/// Fiend: "If this card is Summoned: Toss a coin ... Heads: When an attack is declared involving this card: You can target
/// 1 monster on the field; destroy it, and if you do, inflict 500 damage to its controller." Seed 4 tosses heads: the
/// opponent's Warwolf is destroyed (500), then the Fiend's 2500 attack is direct.
#[test]
fn fiend_heads_destroys_a_monster_when_it_attacks() {
    let spawns = [at(0, Hand, FIEND), at(0, Monster, KURIBOH), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = start_seed(4, false, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, WARWOLF)]);
    s.run(&[Summon(FIEND), EndTurn, Attack(FIEND), Chain(FIEND)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500 - 2500, "{}", s.context());
    assert!(s.monster(0, FIEND).is_some(), "{}", s.context());
}

/// Fiend: "Tails: If an attack is declared involving this card: Destroy all monsters on the field." Seed 1 tosses tails.
#[test]
fn fiend_tails_destroys_all_monsters_at_an_attack() {
    let spawns = [at(0, Hand, FIEND), at(0, Monster, KURIBOH), at(0, Monster, KURIBOH), at(1, Monster, WARWOLF)];
    let Some(mut s) = start_seed(1, false, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH), (1, WARWOLF)]);
    s.run(&[Summon(FIEND), EndTurn, Attack(FIEND)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", FIEND), 1, "the Fiend too: {}", s.context());
}

/// Armored White Bear: "If this card is Normal or Special Summoned while a Synchro Monster is on the field: You can
/// target 1 Field Spell in your GY; add it to your hand."
#[test]
fn armored_white_bear_fetches_a_field_spell() {
    let spawns = [at(0, Hand, WHITE_BEAR), at(0, Monster, STARDUST), at(0, Grave, YAMI)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(WHITE_BEAR), Chain(WHITE_BEAR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", YAMI), 1, "{}", s.context());
}

/// Armored White Bear: "If this card is destroyed by battle and sent to the GY ...: You can Special Summon 1 Level 4 or
/// lower Beast monster from your Deck or GY, except "Armored White Bear"." Warwolf destroys it; Silver Fang comes from the Deck.
#[test]
fn armored_white_bear_brings_a_beast_when_destroyed_by_battle() {
    let deck: Vec<u16> = [SILVER_FANG].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &[at(0, Monster, WHITE_BEAR), at(1, Monster, WARWOLF)]) else { return };
    s.prefer(&[(0, SILVER_FANG)]);
    s.run_both([&[EndTurn, Chain(WHITE_BEAR)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WHITE_BEAR), 1, "{}", s.context());
    assert!(s.monster(0, SILVER_FANG).is_some(), "{}", s.context());
}

/// Ashoka Pillar: "If this card is Summoned: You can add 1 Equip Spell from your Deck to your hand, then if this card is
/// in Attack Position, change it to Defense Position."
#[test]
fn ashoka_pillar_fetches_an_equip_spell_and_turns_to_defense() {
    let deck: Vec<u16> = [AXE_OF_DESPAIR].into_iter().chain([KURIBOH; 39]).collect();
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &[at(0, Hand, ASHOKA_PILLAR)]) else { return };
    s.run(&[Summon(ASHOKA_PILLAR), Chain(ASHOKA_PILLAR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", AXE_OF_DESPAIR), 1, "{}", s.context());
    assert_eq!(s.position_of(0, ASHOKA_PILLAR), Some(4), "{}", s.context());
}

/// Ashoka Pillar: "If this card is destroyed by battle or card effect: Take 2000 damage."
#[test]
fn ashoka_pillar_hurts_when_destroyed() {
    let Some(mut s) = printed(&[at(0, Hand, DARK_HOLE), at(0, Monster, ASHOKA_PILLAR)]) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6000, "{}", s.context());
}

/// Assault Dog: "When this card is destroyed by battle and sent to the GY: You can Special Summon any number of "Assault
/// Dog" from your Deck." Two more are in the Deck; Warwolf destroys the one on the field. (The harness answers a "any number"
/// choice with its minimum, one card: it checks that one comes from the Deck.)
#[test]
fn assault_dog_brings_the_others_from_the_deck() {
    let deck: Vec<u16> = [ASSAULT_DOG; 2].into_iter().chain([KURIBOH; 38]).collect();
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &[at(0, Monster, ASSAULT_DOG), at(1, Monster, WARWOLF)]) else { return };
    s.run_both([&[EndTurn, Chain(ASSAULT_DOG)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ASSAULT_DOG), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", ASSAULT_DOG), 1, "one came from the Deck: {}", s.context());
}

/// Assault Dog: "... You can Special Summon any number of "Assault Dog" from your Deck." With the harness taking as many
/// as the choice allows, both Dogs in the Deck come out.
#[test]
fn assault_dog_brings_more_than_one_from_the_deck() {
    let deck: Vec<u16> = [ASSAULT_DOG; 2].into_iter().chain([KURIBOH; 38]).collect();
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], &[at(0, Monster, ASSAULT_DOG), at(1, Monster, WARWOLF)]) else { return };
    s.prefer(&[(0, ASSAULT_DOG)]);
    s.prefer_most(true);
    s.run_both([&[EndTurn, Chain(ASSAULT_DOG)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ASSAULT_DOG), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", ASSAULT_DOG), 2, "both Dogs came from the Deck: {}", s.context());
}

/// Attack Guidance Armor (printed): "When a monster declares an attack: Activate 1 of these effects; Destroy the attacking
/// monster, or change the attack target ..." The Set trap meets Warwolf's attack; the attacker is destroyed or the attack
/// is redirected: either way Warwolf does not hurt the player as an unanswered attack would (800 damage to Silver Fang).
#[test]
fn attack_guidance_armor_answers_an_attack() {
    let spawns = [at(0, SpellSet, ATTACK_GUIDANCE_ARMOR), at(0, Monster, SILVER_FANG), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(ATTACK_GUIDANCE_ARMOR)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ATTACK_GUIDANCE_ARMOR), 1, "the trap was used: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the attacker was destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// The anime Attack Guidance Armor (anime cards on): "When a monster declares an attack: Change the attack target to a
/// monster, except the attacking monster or the attack target." Warwolf attacks one of two monsters and is redirected to
/// the other: one of them is destroyed, and no attacker is destroyed.
#[test]
fn anime_attack_guidance_armor_redirects_the_attack() {
    let spawns = [at(0, SpellSet, ATTACK_GUIDANCE_ARMOR), at(0, Monster, SILVER_FANG), at(0, Monster, GAZELLE), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(ATTACK_GUIDANCE_ARMOR)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ATTACK_GUIDANCE_ARMOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", SILVER_FANG) + s.count(0, "grave", GAZELLE), 1, "one monster was hit: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Ayers Rock Sunrise: "Target 1 Beast monster in your GY; Special Summon that target, and if you do, all face-up monsters
/// your opponent currently controls (if any) lose 200 ATK for each Beast, Plant, and Winged Beast monster currently in
/// your GY." Silver Fang returns; Gazelle stays in the Graveyard: Warwolf loses 200.
#[test]
fn ayers_rock_sunrise_revives_a_beast_and_weakens_the_opponent() {
    let spawns = [at(0, Hand, AYERS_ROCK_SUNRISE), at(0, Grave, SILVER_FANG), at(0, Grave, GAZELLE), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, SILVER_FANG)]);
    s.run(&[Activate(AYERS_ROCK_SUNRISE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, SILVER_FANG).is_some(), "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(1800), "{}", s.context());
}

/// Battle Fusion (printed): "When an attack is declared involving a Fusion Monster you control and an opponent's monster:
/// That monster you control gains ATK equal to the ATK of the opponent's monster, until the end of the Damage Step."
/// Blue-Eyes (3000) attacks Amulet Dragon (2900): with 5900 it survives and Blue-Eyes is destroyed, 2900 damage.
#[test]
fn battle_fusion_adds_the_attackers_atk() {
    let spawns = [at(0, SpellSet, BATTLE_FUSION), at(0, Monster, AMULET_DRAGON), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn, Chain(BATTLE_FUSION)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "{}", s.context());
    assert!(s.monster(0, AMULET_DRAGON).is_some(), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2900, "{}", s.context());
}

/// The anime Battle Fusion (anime cards on): "Activate only when a face-up monster you control attacks or is attacked. It
/// gains ATK equal to the ATK of the opposing monster, until the end of the Damage Step." Any monster, not only a Fusion:
/// Warwolf (2000) attacked by Blue-Eyes (3000) goes to 5000 and destroys it.
#[test]
fn anime_battle_fusion_works_on_any_monster() {
    let spawns = [at(0, SpellSet, BATTLE_FUSION), at(0, Monster, WARWOLF), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(BATTLE_FUSION)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "{}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "{}", s.context());
}

/// Battle of Sleeping Spirits (printed): "Each player can Special Summon 1 monster from their GY that was destroyed by
/// battle this turn, in Attack Position." Warwolf destroys Silver Fang; the trap, chained after the battle, brings it back.
#[test]
fn battle_of_sleeping_spirits_revives_a_monster_destroyed_by_battle() {
    let spawns = [at(0, SpellSet, BATTLE_OF_SLEEPING_SPIRITS), at(0, Monster, SILVER_FANG), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn, Chain(BATTLE_OF_SLEEPING_SPIRITS)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BATTLE_OF_SLEEPING_SPIRITS), 1, "{}", s.context());
    assert!(s.monster(0, SILVER_FANG).is_some(), "Silver Fang is back: {}", s.context());
}

/// The anime Battle of Sleeping Spirits (anime cards on): "During the Battle Phase: Each player can Special Summon 1 of
/// their monsters that was destroyed by battle this turn, with their effects negated. Those monsters are destroyed at the
/// end of the Battle Phase."
#[test]
fn anime_battle_of_sleeping_spirits_revives_until_the_end_of_the_battle_phase() {
    let spawns = [at(0, SpellSet, BATTLE_OF_SLEEPING_SPIRITS), at(0, Monster, SILVER_FANG), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(BATTLE_OF_SLEEPING_SPIRITS)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, SILVER_FANG).is_some(), "Silver Fang is back for the Battle Phase: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(s.monster(0, SILVER_FANG).is_none(), "destroyed at the end of the Battle Phase: {}", s.context());
}
