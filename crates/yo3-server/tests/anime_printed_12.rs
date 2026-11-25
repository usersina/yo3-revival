//! Scenario tests for the twelfth group of the anime's printed cards (rows 4848-5205 of engine/added/cards.tsv): from
//! "Red-Eyes Spirit" to "Singularity Fiend". Each through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`). Texts: the printed ones (anime cards off) from the pinned official database; an
//! anime version is tested with anime cards on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg};

// The added cards of this group (client ids from the group list).
const RED_EYES_SPIRIT: u16 = 11898;
const RED_EYES_TRANSMIGRATION: u16 = 11895;
const RELAY_SOUL: u16 = 11154;
const RESONATOR_CALL: u16 = 9518;
const RIDE_OF_THE_VALKYRIES: u16 = 13997;
const RITUAL_OF_THE_MATADOR: u16 = 5081;
const RITUAL_SANCTUARY: u16 = 12539;
const ROCKET_HERMOS_CANNON: u16 = 11889;
const ROULETTE_SPIDER: u16 = 4820;
const ROYAL_STRAIGHT: u16 = 4690;
const ROYAL_STRAIGHT_SLASHER: u16 = 4689;
const SABATIEL: u16 = 11160; // Sabatiel - The Philosopher's Stone
const SADION: u16 = 13159;
const SALVAGE_WARRIOR: u16 = 9474;
const SANDAION: u16 = 13819;
const SCARLET_SECURITY: u16 = 9516;
const SCRAMBLE_EGG: u16 = 11673;
const SCRUM_FORCE: u16 = 12543;
const SEPHYLON: u16 = 9708; // Sephylon, the Ultimate Timelord
const SHADOW_IMPULSE: u16 = 11177;
const SHADOW_TOON: u16 = 11903;
const SHELL_KNIGHT: u16 = 15880;
const SHOOTING_STAR: u16 = 11171;
const SINGULARITY_FIEND: u16 = 5070;
// Other added cards they play with.
const LORD_OF_THE_RED: u16 = 11894;
const MATADOR_ARCHFIEND: u16 = 5080; // stand-in (Konami id 21753)
const VALKYRIE_DRITTE: u16 = 13992;
const VALKYRIE_ERSTE: u16 = 13994;
const MISCHIEF: u16 = 13998; // Mischief of the Time Goddess
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const OOKAZI: u16 = 4352;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const RED_EYES: u16 = 4088; // Red-Eyes Black Dragon
const CREATION_RESONATOR: u16 = 9090;
const BRUNHILDE: u16 = 13995; // Valkyrie Brunhilde
const CLAW_OF_HERMOS: u16 = 11892;
const QUEEN: u16 = 5503; // Queen's Knight
const KING: u16 = 5558; // King's Knight
const JACK: u16 = 5559; // Jack's Knight
const SONIC_CHICK: u16 = 7686;
const POLYMERIZATION: u16 = 4837;
const TOON_WORLD: u16 = 4798;
const STARDUST: u16 = 7734; // Stardust Dragon, a Synchro Monster
const RED_DRAGON_ARCHFIEND: u16 = 7735;
const BLUE_EYES: u16 = 4007;
const HOURGLASS: u16 = 4235; // Hourglass of Life: a Level 2 Fairy
const SILVER_FANG: u16 = 4071; // Level 3
const ORION: u16 = 4132; // Orion the Battle King: Level 5
const ARCHFIEND_SOLDIER: u16 = 5762;
const FLAMVELL_GUARD: u16 = 7535; // a Tuner
const TUNE_WARRIOR: u16 = 7684; // a Tuner
const WINGED_KURIBOH: u16 = 6314;
const GIANT_SOLDIER: u16 = 4080; // Giant Soldier of Stone: 1300/2000
const OFFERINGS: u16 = 5214; // Offerings to the Doomed
const SWORDS: u16 = 4354; // Swords of Revealing Light
const KEY_MACE: u16 = 4198; // a Level 1 LIGHT Fairy
const GAIA_PLATE: u16 = 7558; // Gaia Plate the Earth Giant, a Level 8 Rock
const TUALATIN: u16 = 6525; // a Level 8 Fairy
const CYBER_DRAGON: u16 = 6390;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn start(anime: bool, main: [&[u16]; 2], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    start_lp(8000, anime, main, extra, spawns)
}

fn start_lp(lp: u32, anime: bool, main: [&[u16]; 2], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(main[0], extra[0]), (main[1], extra[1])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }.unwrap())
}

/// A prompt that is the end of the duel: the engine's win message, for the player in `winner`.
fn won(winner: u64) -> impl Fn(&serde_json::Value) -> bool {
    move |p| p["type"] == 5 && p["player"] == winner
}

/// A duel with Kuriboh decks and `spawns` in place; anime cards off.
fn printed(spawns: &[Spawn]) -> Option<Scenario> {
    start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[]], spawns)
}

/// The same with anime cards on.
fn anime(spawns: &[Spawn]) -> Option<Scenario> {
    start(true, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[]], spawns)
}

/// Answers the card-by-card choice now pending (`SelectUnselectGroup`) with its first `n` cards, then finishes.
fn pick_first(s: &mut Scenario, n: usize) {
    for _ in 0..n {
        if s.prompt()["type"] != msg::SELECT_UNSELECT_CARD {
            return; // the engine ended the choice itself: nothing left to pick
        }
        s.respond(Response::UnselectCard(Some(0)).json()).unwrap();
    }
    if s.prompt()["type"] == msg::SELECT_UNSELECT_CARD {
        s.respond(Response::UnselectCard(None).json()).unwrap();
    }
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn at(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

/// Red-Eyes Spirit: "Target 1 "Red-Eyes" monster in your GY; Special Summon it." Set, then activated a turn later.
#[test]
fn red_eyes_spirit_brings_back_a_red_eyes() {
    let spawns = [at(0, Grave, RED_EYES), at(0, Hand, RED_EYES_SPIRIT), at(0, Grave, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, RED_EYES)]);
    s.run(&[SetSpell(RED_EYES_SPIRIT), EndTurn, Activate(RED_EYES_SPIRIT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", RED_EYES), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "only a Red-Eyes comes back: {}", s.context());
    assert_eq!(s.count(0, "grave", RED_EYES_SPIRIT), 1, "{}", s.context());
}

/// Anime Red-Eyes Spirit: Special Summons a "Red-Eyes" monster destroyed this turn (not one that has long been in the GY).
#[test]
fn red_eyes_spirit_anime_takes_one_destroyed_this_turn() {
    let spawns = [at(0, Grave, RED_EYES), at(0, Hand, RED_EYES_SPIRIT), at(0, Monster, RED_EYES), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = anime(&spawns) else { return };
    s.run(&[SetSpell(RED_EYES_SPIRIT), SetSpell(DARK_HOLE), EndTurn, Activate(DARK_HOLE), Activate(RED_EYES_SPIRIT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", RED_EYES), 1, "the one Dark Hole destroyed this turn is back: {}", s.context());
    assert_eq!(s.count(0, "grave", RED_EYES), 1, "the older one stays: {}", s.context());
}

/// Red-Eyes Transmigration: "Ritual Summon "Lord of the Red"... banish "Red-Eyes" monsters from your GY, whose total
/// Levels equal 8 or more." Red-Eyes Black Dragon (7) from the GY and a Level 4 from the hand.
#[test]
fn red_eyes_transmigration_summons_lord_of_the_red() {
    let spawns = [at(0, Hand, RED_EYES_TRANSMIGRATION), at(0, Hand, LORD_OF_THE_RED), at(0, Grave, RED_EYES), at(0, Hand, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, RED_EYES), (0, WARWOLF)]);
    s.run(&[Activate(RED_EYES_TRANSMIGRATION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", LORD_OF_THE_RED), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", RED_EYES), 1, "the Red-Eyes is banished from the GY: {}", s.context());
}

/// Relay Soul: "Special Summon 1 monster from your hand. While it is face-up on your field, you take no damage." Set,
/// then activated a turn later; the opponent's Ookazi (800 damage) then does nothing.
#[test]
fn relay_soul_stops_all_damage() {
    let spawns = [at(0, Hand, RELAY_SOUL), at(0, Hand, WARWOLF), at(1, Hand, OOKAZI)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[SetSpell(RELAY_SOUL), EndTurn, Activate(RELAY_SOUL), EndTurn], &[SetSpell(OOKAZI), EndTurn, Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", OOKAZI), 1, "Ookazi resolved: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no damage while it is on the field: {}", s.context());
}

/// "When it leaves the field, your opponent wins the Duel." Dark Hole destroys the monster Relay Soul brought.
#[test]
fn relay_soul_loses_the_duel_when_its_monster_leaves() {
    let spawns = [at(0, Hand, RELAY_SOUL), at(0, Hand, WARWOLF), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_to(&[SetSpell(RELAY_SOUL), SetSpell(DARK_HOLE), EndTurn, Activate(RELAY_SOUL), Activate(DARK_HOLE)], won(1)).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 0, "{}", s.context());
    assert!(won(1)(s.prompt()), "the opponent wins: {}", s.context());
}

/// Anime Relay Soul: activated as lethal damage would take the LP to 0; the player survives with the monster summoned.
#[test]
fn relay_soul_anime_survives_lethal_damage() {
    let spawns = [at(0, Hand, RELAY_SOUL), at(0, Hand, WARWOLF), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = start_lp(2000, true, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_both([&[SetSpell(RELAY_SOUL), EndTurn, Chain(RELAY_SOUL)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "{}", s.context());
    assert!(!won(1)(s.prompt()), "the duel goes on at 0 LP: {}", s.context());
}

/// Resonator Call: "Add 1 "Resonator" monster from your Deck to your hand."
#[test]
fn resonator_call_searches_a_resonator() {
    let deck: Vec<u16> = [KURIBOH; 38].into_iter().chain([CREATION_RESONATOR, WARWOLF]).collect();
    let spawns = [at(0, Hand, RESONATOR_CALL)];
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run(&[Activate(RESONATOR_CALL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", CREATION_RESONATOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", WARWOLF), 0, "only a Resonator: {}", s.context());
}

/// Ride of the Valkyries: "Special Summon any number of "Valkyrie" monsters with different names from your hand. If you
/// Special Summoned 3 or more monsters with this effect, you take no battle damage until the end of the next turn.
/// During the End Phase of the turn you activated this card, shuffle all monsters you control into the Deck."
#[test]
fn ride_of_the_valkyries_special_summons_three_different_valkyries() {
    let spawns = [at(0, Hand, RIDE_OF_THE_VALKYRIES), at(0, Hand, VALKYRIE_DRITTE), at(0, Hand, VALKYRIE_ERSTE), at(0, Hand, BRUNHILDE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_to(&[Activate(RIDE_OF_THE_VALKYRIES)], |p| p["type"] == msg::SELECT_UNSELECT_CARD).unwrap();
    pick_first(&mut s, 3);
    s.run(&[]).unwrap();
    clean(&s);
    for v in [VALKYRIE_DRITTE, VALKYRIE_ERSTE, BRUNHILDE] {
        assert_eq!(s.count(0, "mzone", v), 1, "{v}: {}", s.context());
    }
}

/// The same, to the End Phase and the opponent's direct attack: the three are shuffled into the Deck, and the 3 or more
/// Special Summoned give "no battle damage until the end of the next turn".
#[test]
fn ride_of_the_valkyries_shuffles_back_and_stops_battle_damage() {
    let spawns = [at(0, Hand, RIDE_OF_THE_VALKYRIES), at(0, Hand, VALKYRIE_DRITTE), at(0, Hand, VALKYRIE_ERSTE), at(0, Hand, BRUNHILDE),
        at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_to(&[Activate(RIDE_OF_THE_VALKYRIES)], |p| p["type"] == msg::SELECT_UNSELECT_CARD).unwrap();
    pick_first(&mut s, 3);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BRUNHILDE) + s.count(0, "mzone", VALKYRIE_DRITTE) + s.count(0, "mzone", VALKYRIE_ERSTE), 0, "shuffled away: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no battle damage from the direct attack: {}", s.context());
}

/// "You can banish this card from your GY; add 1 "Mischief of the Time Goddess" from your Deck to your hand."
#[test]
fn ride_of_the_valkyries_banishes_itself_to_search_mischief() {
    let deck: Vec<u16> = [KURIBOH; 39].into_iter().chain([MISCHIEF]).collect();
    let spawns = [at(0, Grave, RIDE_OF_THE_VALKYRIES)];
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run(&[Activate(RIDE_OF_THE_VALKYRIES)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", MISCHIEF), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", RIDE_OF_THE_VALKYRIES), 1, "{}", s.context());
}

/// Anime Ride of the Valkyries: "Special Summon all "Valkyrie" cards in your hand, but they are returned to the Deck during
/// the End Phase." (Two Valkyries in hand, both come, both go back.)
#[test]
fn ride_of_the_valkyries_anime_summons_the_whole_hand() {
    let spawns = [at(0, Hand, RIDE_OF_THE_VALKYRIES), at(0, Hand, VALKYRIE_DRITTE), at(0, Hand, BRUNHILDE)];
    let Some(mut s) = anime(&spawns) else { return };
    s.run(&[Activate(RIDE_OF_THE_VALKYRIES)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", VALKYRIE_DRITTE) + s.count(0, "mzone", BRUNHILDE), 2, "{}", s.context());
    let Some(mut s) = anime(&spawns) else { return };
    s.run(&[Activate(RIDE_OF_THE_VALKYRIES), EndTurn, SetSpell(DARK_HOLE)]).ok();
    clean(&s);
    assert_eq!(s.count(0, "mzone", VALKYRIE_DRITTE) + s.count(0, "mzone", BRUNHILDE), 0, "back in the Deck: {}", s.context());
}

/// Ritual of the Matador: "This card can be used to Ritual Summon "Matador Archfiend". You must also Tribute monsters from
/// your hand or field whose total Levels equal 6 or more." A Level 4 and a Level 2 from the hand.
#[test]
fn ritual_of_the_matador_summons_matador_archfiend() {
    let spawns = [at(0, Hand, RITUAL_OF_THE_MATADOR), at(0, Hand, MATADOR_ARCHFIEND), at(0, Hand, WARWOLF), at(0, Hand, HOURGLASS)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, HOURGLASS)]);
    s.run(&[Activate(RITUAL_OF_THE_MATADOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", MATADOR_ARCHFIEND), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF) + s.count(0, "grave", HOURGLASS), 2, "both Tributed: {}", s.context());
}

/// "You can banish this card from your GY; Special Summon 1 non-Ritual "Archfiend" monster from your hand."
#[test]
fn ritual_of_the_matador_banishes_itself_for_an_archfiend() {
    let spawns = [at(0, Grave, RITUAL_OF_THE_MATADOR), at(0, Hand, ARCHFIEND_SOLDIER), at(0, Hand, MATADOR_ARCHFIEND)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(RITUAL_OF_THE_MATADOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", ARCHFIEND_SOLDIER), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", MATADOR_ARCHFIEND), 0, "a Ritual Monster is not on offer: {}", s.context());
    assert_eq!(s.count(0, "removed", RITUAL_OF_THE_MATADOR), 1, "{}", s.context());
}

/// Ritual Sanctuary: "You can discard 1 Spell; add 1 LIGHT Ritual Monster or 1 Ritual Spell from your Deck to your hand."
#[test]
fn ritual_sanctuary_discards_a_spell_to_search_a_ritual_spell() {
    let deck: Vec<u16> = [KURIBOH; 37].into_iter().chain([RED_EYES_TRANSMIGRATION, LORD_OF_THE_RED, WARWOLF]).collect();
    let spawns = [at(0, Hand, RITUAL_SANCTUARY), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run(&[Activate(RITUAL_SANCTUARY), Activate(RITUAL_SANCTUARY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "the discarded Spell: {}", s.context());
    assert_eq!(s.count(0, "hand", RED_EYES_TRANSMIGRATION), 1, "the Ritual Spell: {}", s.context());
    assert_eq!(s.count(0, "hand", LORD_OF_THE_RED), 0, "a DARK Ritual Monster is not on offer: {}", s.context());
}

/// "You can shuffle any number of Spells from your GY into the Deck, then target 1 LIGHT Fairy monster in your GY whose
/// Level equals the number of cards you shuffled into the Deck; Special Summon it." One Spell for a Level 1.
#[test]
fn ritual_sanctuary_shuffles_spells_to_revive_a_fairy_of_that_level() {
    let spawns = [at(0, Hand, RITUAL_SANCTUARY), at(0, Grave, KEY_MACE), at(0, Grave, HOURGLASS), at(0, Grave, DARK_HOLE), at(0, Grave, MONSTER_REBORN)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, KEY_MACE)]);
    s.run(&[Activate(RITUAL_SANCTUARY), Activate(RITUAL_SANCTUARY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KEY_MACE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", HOURGLASS), 1, "the Level 2 one stays: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE) + s.count(0, "grave", MONSTER_REBORN), 1, "one Spell shuffled away: {}", s.context());
}

/// Anime Ritual Sanctuary: "Discard 1 Spell; add 1 Ritual Spell from your Deck to your hand."
#[test]
fn ritual_sanctuary_anime_searches_a_ritual_spell() {
    let deck: Vec<u16> = [KURIBOH; 38].into_iter().chain([RED_EYES_TRANSMIGRATION, WARWOLF]).collect();
    let spawns = [at(0, Hand, RITUAL_SANCTUARY), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = start(true, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run(&[Activate(RITUAL_SANCTUARY), Activate(RITUAL_SANCTUARY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", RED_EYES_TRANSMIGRATION), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE), 1, "{}", s.context());
}

/// Rocket Hermos Cannon: "Must be Special Summoned with "The Claw of Hermos", using a Warrior monster. If this card is
/// Special Summoned: Target 1 other face-up monster on the field; equip this card to it. It can make a second attack
/// during each Battle Phase". The Claw sends a Warrior to the GY, the Cannon comes and equips itself to Warwolf.
#[test]
fn rocket_hermos_cannon_equips_and_gives_a_second_attack() {
    let spawns = [at(0, Hand, CLAW_OF_HERMOS), at(0, Monster, QUEEN), at(0, Monster, WARWOLF)];
    let Some(mut s) = start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[ROCKET_HERMOS_CANNON], &[]], &spawns) else { return };
    s.prefer(&[(0, QUEEN), (0, WARWOLF)]);
    s.run(&[Activate(CLAW_OF_HERMOS), Attack(WARWOLF), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", ROCKET_HERMOS_CANNON), 1, "equipped: {}", s.context());
    assert_eq!(s.count(0, "grave", QUEEN), 1, "the Warrior the Claw sent: {}", s.context());
    assert_eq!(s.lp(1), 4000, "two direct attacks of 2000: {}", s.context());
}

/// "...if it attacks a Defense Position monster, inflict piercing battle damage to your opponent."
#[test]
fn rocket_hermos_cannon_pierces() {
    let spawns = [at(0, Hand, CLAW_OF_HERMOS), at(0, Monster, QUEEN), at(0, Monster, WARWOLF), at(1, MonsterDefense, KURIBOH)];
    let Some(mut s) = start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[ROCKET_HERMOS_CANNON], &[]], &spawns) else { return };
    s.prefer(&[(0, QUEEN), (0, WARWOLF)]);
    s.run(&[Activate(CLAW_OF_HERMOS), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1800, "2000 ATK against 200 DEF: {}", s.context());
}

/// Roulette Spider: "When an opponent's monster declares an attack: Roll a six-sided die and apply the result." The die
/// follows the duel's seed, so the same duel is played under many seeds; each ends in one of the results the text gives
/// (1: halved LP; 2: a direct attack; 4: the attacker hits the opponent's other monster; 5: negated and damage equal to
/// its ATK; 6: the attacker is destroyed; 3 has no other monster of the player's to choose) and every one of 1, 2, 4, 5
/// and 6 turns up.
#[test]
fn roulette_spider_applies_each_die_result() {
    let mut seen = [false; 7];
    for seed in 1..=40u64 {
        let Some(dir) = engine_dir() else { return };
        let spawns = [at(0, Hand, ROULETTE_SPIDER), at(0, MonsterDefense, GIANT_SOLDIER), at(1, Monster, WARWOLF), at(1, Monster, KURIBOH)];
        let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
        let mut s = Scenario::start_field(&dir, seed, 8000, decks, &spawns).unwrap();
        s.run_both([&[SetSpell(ROULETTE_SPIDER), EndTurn, ChainOnAttack(ROULETTE_SPIDER)], &[Attack(WARWOLF)]]).unwrap();
        clean(&s);
        let (me, them) = (s.lp(0), s.lp(1));
        let warwolf_dead = s.count(1, "grave", WARWOLF) == 1;
        let kuriboh_dead = s.count(1, "grave", KURIBOH) == 1;
        let result = match (me, them) {
            (4000, 8000) if !warwolf_dead => 1,
            (6000, 8000) if !warwolf_dead => 2,
            (8000, 6300) if kuriboh_dead => 4,
            (8000, 6000) if !warwolf_dead => 5,
            (8000, 8000) if warwolf_dead => 6,
            (8000, 8000) => 3,
            _ => panic!("seed {seed}: no result of the die fits LP {me}/{them}: {}", s.context()),
        };
        seen[result] = true;
    }
    for face in [1, 2, 4, 5, 6] {
        assert!(seen[face], "the die's {face} never came up in 40 duels: {seen:?}");
    }
}

/// Royal Straight: "Send 1 "Queen's Knight", "King's Knight", and "Jack's Knight" from your hand and/or face-up Monster Zone
/// to the GY, then Special Summon 1 monster from your hand, Deck, Extra Deck, or GY that mentions all of them."
#[test]
fn royal_straight_sends_the_three_knights_and_summons_the_slasher() {
    let spawns = [at(0, Hand, ROYAL_STRAIGHT), at(0, Hand, QUEEN), at(0, Monster, KING), at(0, Hand, JACK), at(0, Hand, ROYAL_STRAIGHT_SLASHER)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(ROYAL_STRAIGHT)]).unwrap();
    clean(&s);
    for knight in [QUEEN, KING, JACK] {
        assert_eq!(s.count(0, "grave", knight), 1, "{knight}: {}", s.context());
    }
    assert_eq!(s.count(0, "mzone", ROYAL_STRAIGHT_SLASHER), 1, "{}", s.context());
}

/// Anime Royal Straight: "Tribute 1 "Queen's Knight", "King's Knight", and "Jack's Knight" in face-up Attack Position;
/// Special Summon 1 "Royal Straight Slasher" from your hand, Deck, or Graveyard."
#[test]
fn royal_straight_anime_tributes_the_three_knights() {
    let spawns = [at(0, Hand, ROYAL_STRAIGHT), at(0, Monster, QUEEN), at(0, Monster, KING), at(0, Monster, JACK), at(0, Hand, ROYAL_STRAIGHT_SLASHER)];
    let Some(mut s) = anime(&spawns) else { return };
    s.run(&[Activate(ROYAL_STRAIGHT)]).unwrap();
    clean(&s);
    for knight in [QUEEN, KING, JACK] {
        assert_eq!(s.count(0, "mzone", knight), 0, "{knight} Tributed: {}", s.context());
    }
    assert_eq!(s.count(0, "mzone", ROYAL_STRAIGHT_SLASHER), 1, "{}", s.context());
}

/// Royal Straight Slasher: "If you have all of "Queen's Knight", "King's Knight", and "Jack's Knight" in your GY: You can
/// send 5 monsters, with different Levels of 1, 2, 3, 4, and 5, from your hand and/or Deck to the GY, and if you do,
/// destroy all cards your opponent controls."
#[test]
fn royal_straight_slasher_destroys_everything_the_opponent_controls() {
    let spawns = [at(0, Monster, ROYAL_STRAIGHT_SLASHER), at(0, Grave, QUEEN), at(0, Grave, KING), at(0, Grave, JACK), at(0, Hand, KURIBOH),
        at(0, Hand, HOURGLASS), at(0, Hand, SILVER_FANG), at(0, Hand, WARWOLF), at(0, Hand, ORION), at(1, Monster, BLUE_EYES), at(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(ROYAL_STRAIGHT_SLASHER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
    for level in [HOURGLASS, SILVER_FANG, WARWOLF, ORION] {
        assert_eq!(s.count(0, "grave", level), 1, "{level} sent: {}", s.context());
    }
}

/// "When this card is destroyed by battle: You can target up to 3 LIGHT Warrior monsters in your GY; Special Summon them."
#[test]
fn royal_straight_slasher_brings_back_light_warriors_when_destroyed_by_battle() {
    let spawns = [at(0, Monster, ROYAL_STRAIGHT_SLASHER), at(0, Grave, QUEEN), at(0, Grave, KING), at(0, Grave, JACK), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&spawns) else { return };
    // The Slasher (2400) attacks Blue-Eyes (3000) and is destroyed; its trigger asks for up to 3 targets.
    s.run_to(&[Attack(ROYAL_STRAIGHT_SLASHER), Chain(ROYAL_STRAIGHT_SLASHER)], |p| p["type"] == msg::SELECT_CARD && p["max"] == 3).unwrap();
    s.respond(Response::Card(Some(vec![0, 1, 2])).json()).unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ROYAL_STRAIGHT_SLASHER), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", QUEEN) + s.count(0, "mzone", KING) + s.count(0, "mzone", JACK), 3, "{}", s.context());
}

/// Anime Royal Straight Slasher: "You can send 1 monster of each Level between 1 and 5 from your Deck to the Graveyard;
/// destroy all cards your opponent controls." (No Knights in the Graveyard needed.)
#[test]
fn royal_straight_slasher_anime_needs_only_the_five_levels_in_the_deck() {
    let deck: Vec<u16> = [KURIBOH, HOURGLASS, SILVER_FANG, WARWOLF, ORION].into_iter().cycle().take(40).collect();
    let spawns = [at(0, Monster, ROYAL_STRAIGHT_SLASHER), at(1, Monster, BLUE_EYES), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run(&[Activate(ROYAL_STRAIGHT_SLASHER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES) + s.count(1, "grave", WARWOLF), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", ORION), 1, "the Level 5 was sent: {}", s.context());
}

/// Sabatiel - The Philosopher's Stone: "If you have a "Winged Kuriboh" monster in your GY: Pay half your LP; add 1
/// "Polymerization" Spell or "Fusion" Spell from your Deck to your hand."
#[test]
fn sabatiel_pays_half_the_lp_to_search_polymerization() {
    // Several copies: the seed shuffles the Deck, and some are drawn.
    let deck: Vec<u16> = [KURIBOH; 30].into_iter().chain([POLYMERIZATION; 10]).collect();
    let spawns = [at(0, Hand, SABATIEL), at(0, Grave, WINGED_KURIBOH)];
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD).unwrap();
    let before = s.count(0, "hand", POLYMERIZATION);
    s.run(&[Activate(SABATIEL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", POLYMERIZATION), before + 1, "{}", s.context());
    assert_eq!(s.lp(0), 4000, "half of 8000: {}", s.context());
}

/// "If this card is in your GY: You can banish 3 copies of "Sabatiel - The Philosopher's Stone" from your GY, then target
/// 1 monster on the field; it gains ATK equal to the ATK of the monster on the field with the highest ATK."
#[test]
fn sabatiel_banishes_three_copies_for_the_highest_atk() {
    let spawns = [at(0, Grave, SABATIEL), at(0, Grave, SABATIEL), at(0, Grave, SABATIEL), at(0, Monster, WARWOLF), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(SABATIEL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", SABATIEL), 3, "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(5000), "2000 plus Blue-Eyes' 3000: {}", s.context());
}

/// Salvage Warrior: "When this card is Tribute Summoned, you can Special Summon 1 Tuner monster from your hand or
/// Graveyard." From the hand.
#[test]
fn salvage_warrior_summons_a_tuner_from_the_hand() {
    let spawns = [at(0, Hand, SALVAGE_WARRIOR), at(0, Monster, KURIBOH), at(0, Hand, FLAMVELL_GUARD)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(SALVAGE_WARRIOR), Chain(SALVAGE_WARRIOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SALVAGE_WARRIOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FLAMVELL_GUARD), 1, "{}", s.context());
}

/// The same from the Graveyard.
#[test]
fn salvage_warrior_summons_a_tuner_from_the_graveyard() {
    let spawns = [at(0, Hand, SALVAGE_WARRIOR), at(0, Monster, KURIBOH), at(0, Grave, TUNE_WARRIOR)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(SALVAGE_WARRIOR), Chain(SALVAGE_WARRIOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TUNE_WARRIOR), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "the Tributed Kuriboh: {}", s.context());
}

/// Scarlet Security: "If you control a "Red Dragon Archfiend": Destroy all Spell and Trap Cards your opponent controls."
#[test]
fn scarlet_security_destroys_the_opponents_spells_and_traps() {
    let spawns = [at(0, Hand, SCARLET_SECURITY), at(0, Monster, RED_DRAGON_ARCHFIEND), at(0, SpellSet, DARK_HOLE), at(1, SpellSet, MST), at(1, Spell, SWORDS),
        at(1, Monster, WARWOLF)];
    let Some(mut s) = start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[RED_DRAGON_ARCHFIEND], &[]], &spawns) else { return };
    s.run(&[Activate(SCARLET_SECURITY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MST) + s.count(1, "grave", SWORDS), 2, "{}", s.context());
    assert_eq!(s.count(0, "szone", DARK_HOLE), 1, "its own Set card stays: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "monsters are not touched: {}", s.context());
}

/// Without a "Red Dragon Archfiend" the card cannot be activated.
#[test]
fn scarlet_security_needs_the_red_dragon_archfiend() {
    let spawns = [at(0, Hand, SCARLET_SECURITY), at(1, SpellSet, MST)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[Activate(SCARLET_SECURITY)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(1, "szone", MST), 1, "{}", s.context());
}

/// Scrum Force: "While you control 2 or more face-up Defense Position monsters, ... they cannot be destroyed by your
/// opponent's card effects." The opponent's Dark Hole destroys only its own monster.
#[test]
fn scrum_force_makes_defense_monsters_indestructible_by_effects() {
    let spawns = [at(0, Spell, SCRUM_FORCE), at(0, MonsterDefense, WARWOLF), at(0, MonsterDefense, GIANT_SOLDIER), at(1, Hand, DARK_HOLE), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF) + s.count(0, "mzone", GIANT_SOLDIER), 2, "{}", s.context());
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "the opponent's own monster is destroyed: {}", s.context());
}

/// "...your opponent cannot target Defense Position monsters you control with card effects": Offerings to the Doomed has
/// no target with two Defense Position monsters, and destroys the only one when there is a single one.
#[test]
fn scrum_force_stops_the_opponent_targeting_defense_monsters() {
    let spawns = [at(0, Spell, SCRUM_FORCE), at(0, MonsterDefense, WARWOLF), at(0, MonsterDefense, GIANT_SOLDIER), at(1, Hand, OFFERINGS)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run_both([&[EndTurn], &[Activate(OFFERINGS)]]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", WARWOLF) + s.count(0, "mzone", GIANT_SOLDIER), 2, "{}", s.context());
    // With one Defense Position monster the card is not in force.
    let spawns = [at(0, Spell, SCRUM_FORCE), at(0, MonsterDefense, WARWOLF), at(1, Hand, OFFERINGS)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(OFFERINGS)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// "If this card would be destroyed by card effect, you can destroy 1 card in your Spell & Trap Zone instead."
#[test]
fn scrum_force_destroys_another_spell_or_trap_in_its_place() {
    let spawns = [at(0, Spell, SCRUM_FORCE), at(0, Spell, SWORDS), at(1, Hand, MST)];
    let Some(mut s) = printed(&spawns) else { return };
    s.prefer(&[(0, SCRUM_FORCE)]);
    s.run_both([&[EndTurn], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", SCRUM_FORCE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", SWORDS), 1, "the other Spell was destroyed instead: {}", s.context());
}

/// Shadow Toon: "If you control "Toon World": Target 1 face-up monster your opponent controls; inflict damage to your
/// opponent equal to the ATK of that face-up monster."
#[test]
fn shadow_toon_inflicts_the_atk_of_the_target() {
    let spawns = [at(0, Hand, SHADOW_TOON), at(0, Spell, TOON_WORLD), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Activate(SHADOW_TOON)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 6000, "{}", s.context());
}

/// Without "Toon World" it cannot be activated.
#[test]
fn shadow_toon_needs_toon_world() {
    let spawns = [at(0, Hand, SHADOW_TOON), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[Activate(SHADOW_TOON)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Anime Shadow Toon: "If you control "Toon World": Target 1 monster your opponent controls; Inflict damage to your
/// opponent equal to that monster's ATK."
#[test]
fn shadow_toon_anime_inflicts_the_atk_of_the_target() {
    let spawns = [at(0, Hand, SHADOW_TOON), at(0, Spell, TOON_WORLD), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = anime(&spawns) else { return };
    s.run(&[Activate(SHADOW_TOON)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 5000, "{}", s.context());
}

/// Shooting Star: "If a "Stardust" monster is on the field: Target 1 card on the field; destroy that target."
#[test]
fn shooting_star_destroys_a_card_on_the_field() {
    let spawns = [at(0, Hand, SHOOTING_STAR), at(0, Monster, STARDUST), at(1, Monster, BLUE_EYES), at(1, Monster, WARWOLF)];
    let Some(mut s) = start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[STARDUST], &[]], &spawns) else { return };
    s.prefer(&[(1, BLUE_EYES)]);
    s.run(&[SetSpell(SHOOTING_STAR), EndTurn, Activate(SHOOTING_STAR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "only the target: {}", s.context());
}

/// Without a "Stardust" monster on the field it cannot be activated.
#[test]
fn shooting_star_needs_a_stardust_monster() {
    let spawns = [at(0, Hand, SHOOTING_STAR), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = printed(&spawns) else { return };
    let err = s.run(&[SetSpell(SHOOTING_STAR), EndTurn, Activate(SHOOTING_STAR)]).unwrap_err();
    assert!(err.contains("never became possible"), "{err}");
}

/// Singularity Fiend: "When your opponent Special Summons a monster(s) (except during the Damage Step): You can discard this
/// card and 1 Spell; destroy that monster(s)." The opponent's Monster Reborn brings Warwolf back, and it is destroyed.
#[test]
fn singularity_fiend_destroys_a_special_summoned_monster() {
    let spawns = [at(1, Hand, SINGULARITY_FIEND), at(1, Hand, DARK_HOLE), at(0, Hand, MONSTER_REBORN), at(0, Grave, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[Activate(MONSTER_REBORN)], &[Chain(SINGULARITY_FIEND)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", SINGULARITY_FIEND), 1, "discarded: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "the discarded Spell: {}", s.context());
}

/// Anime Singularity Fiend: "When your opponent would Special Summon a monster(s) (Quick Effect): You can discard this card and
/// 1 Spell from your hand; negate the Special Summon, and if you do, destroy it." (Outside a chain: Cyber Dragon's own
/// Special Summon from the hand is negated and the Dragon destroyed.)
#[test]
fn singularity_fiend_anime_negates_the_special_summon() {
    let spawns = [at(1, Hand, SINGULARITY_FIEND), at(1, Hand, DARK_HOLE), at(0, Hand, CYBER_DRAGON), at(1, Monster, WARWOLF)];
    let Some(mut s) = anime(&spawns) else { return };
    s.run_both([&[SpSummon(CYBER_DRAGON)], &[Chain(SINGULARITY_FIEND)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", CYBER_DRAGON), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", CYBER_DRAGON), 1, "negated and destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", SINGULARITY_FIEND), 1, "{}", s.context());
}

/// Shell Knight: "When this card is Normal Summoned: You can change this card to Defense Position, and if you do, inflict
/// 500 damage to your opponent."
#[test]
fn shell_knight_turns_to_defense_and_burns_500() {
    let spawns = [at(0, Hand, SHELL_KNIGHT)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(SHELL_KNIGHT), Chain(SHELL_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, SHELL_KNIGHT), Some(4), "{}", s.context());
    assert_eq!(s.lp(1), 7500, "{}", s.context());
}

/// "If this card is destroyed by battle...: You can add 1 Level 8 Rock monster from your Deck to your hand." The
/// opponent's Blue-Eyes destroys the Shell Knight in Defense Position (0/2000).
#[test]
fn shell_knight_adds_a_level_8_rock_when_destroyed_by_battle() {
    let deck: Vec<u16> = [KURIBOH; 30].into_iter().chain([GAIA_PLATE; 10]).collect();
    let spawns = [at(0, MonsterDefense, SHELL_KNIGHT), at(1, Monster, BLUE_EYES)];
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD).unwrap();
    let (hand_before, deck_before) = (hand(&s, 0), s.state()[0]["deck"].as_u64().unwrap());
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SHELL_KNIGHT), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), hand_before + 1, "{}", s.context());
    assert_eq!(s.state()[0]["deck"].as_u64().unwrap(), deck_before - 1, "{}", s.context());
}

/// "...or sent to the GY by a card effect": Dark Hole.
#[test]
fn shell_knight_adds_a_level_8_rock_when_destroyed_by_an_effect() {
    let deck: Vec<u16> = [KURIBOH; 30].into_iter().chain([GAIA_PLATE; 10]).collect();
    let spawns = [at(0, MonsterDefense, SHELL_KNIGHT), at(0, Hand, DARK_HOLE)];
    let Some(mut s) = start(false, [&deck, &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD).unwrap();
    let (hand_before, deck_before) = (hand(&s, 0), s.state()[0]["deck"].as_u64().unwrap());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), hand_before, "Dark Hole left the hand, the Rock came: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_u64().unwrap(), deck_before - 1, "{}", s.context());
}

/// Anime Shell Knight: "When this card is Summoned: Inflict 500 damage to your opponent." (No position change, no search.)
#[test]
fn shell_knight_anime_only_burns_500() {
    let spawns = [at(0, Hand, SHELL_KNIGHT)];
    let Some(mut s) = anime(&spawns) else { return };
    s.run(&[Summon(SHELL_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 7500, "{}", s.context());
    assert_eq!(s.position_of(0, SHELL_KNIGHT), Some(1), "stays in Attack Position: {}", s.context());
}

/// A Deck of 30 Kuriboh and 10 of `card`, for effects that fetch it (the seed shuffles the Deck, some are drawn).
fn deck_with(card: u16) -> Vec<u16> {
    [KURIBOH; 30].into_iter().chain([card; 10]).collect()
}

/// Scramble Egg: "If a monster you control is destroyed by battle or card effect and sent to the GY: You can Special
/// Summon 1 "Sonic Chick" from your hand, Deck or GY." Set, then Dark Hole destroys the player's own Warwolf.
#[test]
fn scramble_egg_summons_a_sonic_chick_when_a_monster_is_destroyed() {
    let spawns = [at(0, Hand, SCRAMBLE_EGG), at(0, Hand, DARK_HOLE), at(0, Monster, WARWOLF)];
    let Some(mut s) = start(false, [&deck_with(SONIC_CHICK), &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run(&[SetSpell(SCRAMBLE_EGG), SetSpell(DARK_HOLE), EndTurn, Activate(DARK_HOLE), Chain(SCRAMBLE_EGG)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", SONIC_CHICK), 1, "{}", s.context());
}

/// Anime Scramble Egg: "When a monster you control is destroyed: Special Summon 1 "Sonic Chick" from your hand, Deck, or Graveyard."
#[test]
fn scramble_egg_anime_summons_a_sonic_chick() {
    let spawns = [at(0, Hand, SCRAMBLE_EGG), at(0, Hand, DARK_HOLE), at(0, Monster, WARWOLF)];
    let Some(mut s) = start(true, [&deck_with(SONIC_CHICK), &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run(&[SetSpell(SCRAMBLE_EGG), SetSpell(DARK_HOLE), EndTurn, Activate(DARK_HOLE), Chain(SCRAMBLE_EGG)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SONIC_CHICK), 1, "{}", s.context());
}

/// Shadow Impulse: "When a Synchro Monster(s) you control is destroyed by battle or card effect and sent to the GY: Target 1
/// of those Synchro Monsters; Special Summon from your Extra Deck, 1 Synchro Monster with the same Level and Type as that
/// monster, but with a different name." Stardust Dragon (Level 8 Dragon) for Red Dragon Archfiend (Level 8 Dragon).
#[test]
fn shadow_impulse_replaces_a_destroyed_synchro_monster() {
    let spawns = [at(0, Hand, SHADOW_IMPULSE), at(0, Hand, DARK_HOLE), at(0, Monster, STARDUST)];
    let Some(mut s) = start(false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[RED_DRAGON_ARCHFIEND], &[]], &spawns) else { return };
    s.run(&[SetSpell(SHADOW_IMPULSE), SetSpell(DARK_HOLE), EndTurn, Activate(DARK_HOLE), Chain(SHADOW_IMPULSE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", STARDUST), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", RED_DRAGON_ARCHFIEND), 1, "{}", s.context());
}

/// Sadion, the Timelord: "If you control no monsters, you can Normal Summon this card without Tributing."
#[test]
fn sadion_is_summoned_without_tributes() {
    let spawns = [at(0, Hand, SADION)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(SADION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SADION), 1, "{}", s.context());
}

/// "Cannot be destroyed by battle or card effects": the opponent's Dark Hole spares it. (The Timelords shuffle themselves
/// into the Deck in their controller's Standby Phase, so a spawned one would be gone at once: the player Normal Summons it.)
#[test]
fn sadion_cannot_be_destroyed_by_an_effect() {
    let spawns = [at(1, Hand, SADION), at(0, Hand, DARK_HOLE), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[SetSpell(DARK_HOLE), EndTurn, Activate(DARK_HOLE)], &[Summon(SADION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", SADION), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "only the other monster is destroyed: {}", s.context());
}

/// "You take no battle damage from attacks involving this card. At the end of the Battle Phase, if this card battled and
/// your LP is less than 4000: Your LP becomes 4000." 2000 LP, then Blue-Eyes attacks Sadion in Attack Position (0 ATK).
#[test]
fn sadion_restores_the_lp_to_4000_after_battle() {
    let spawns = [at(1, Hand, SADION), at(0, Monster, BLUE_EYES)];
    let Some(mut s) = start_lp(2000, false, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_both([&[EndTurn, Attack(BLUE_EYES), EndTurn], &[Summon(SADION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", SADION), 0, "not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(1), 4000, "{}", s.context());
}

/// Anime Sadion: "At the end of the Damage Step, if this card battled while you have 4000 or less LP: Make your LP 4000."
#[test]
fn sadion_anime_restores_the_lp_to_4000() {
    // Two Tributes and nothing else beside Sadion: Blue-Eyes attacks it (an anime Kuriboh attacked negates the attack
    // itself, for its own ATK as damage).
    let spawns = [at(1, Hand, SADION), at(1, Monster, KURIBOH), at(1, Monster, KURIBOH), at(0, Monster, BLUE_EYES)];
    let Some(mut s) = start_lp(3500, true, [&[KURIBOH; 40], &[KURIBOH; 40]], [&[], &[]], &spawns) else { return };
    s.run_both([&[EndTurn, Attack(BLUE_EYES), EndTurn], &[Summon(SADION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 4000, "{}", s.context());
}

/// "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn sadion_shuffles_itself_into_the_deck_in_the_standby_phase() {
    let spawns = [at(0, Hand, SADION)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(SADION), EndTurn, SetMonster(KURIBOH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SADION), 0, "{}", s.context());
}

/// Sandaion, the Timelord: "If only your opponent controls a monster, you can Normal Summon this card without Tributing."
#[test]
fn sandaion_is_summoned_without_tributes_against_a_monster() {
    let spawns = [at(0, Hand, SANDAION), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(SANDAION)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SANDAION), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, SANDAION), Some(4000), "{}", s.context());
}

/// "Neither player takes battle damage from attacks involving this card. At the end of the Battle Phase, if this card
/// battled: Inflict 2000 damage to your opponent." It destroys Warwolf, and the opponent loses 2000 only.
#[test]
fn sandaion_inflicts_2000_after_battle() {
    let spawns = [at(1, Hand, SANDAION), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[EndTurn], &[Summon(SANDAION), Attack(SANDAION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(0), 6000, "no battle damage, 2000 from the effect: {}", s.context());
}

/// "Cannot be destroyed by battle or card effects": Dark Hole.
#[test]
fn sandaion_survives_dark_hole() {
    let spawns = [at(1, Hand, SANDAION), at(0, Hand, DARK_HOLE), at(0, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run_both([&[SetSpell(DARK_HOLE), EndTurn, Activate(DARK_HOLE)], &[Summon(SANDAION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", SANDAION), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
}

/// "Once per turn, during your Standby Phase: Shuffle this card into the Deck."
#[test]
fn sandaion_shuffles_itself_into_the_deck_in_the_standby_phase() {
    let spawns = [at(0, Hand, SANDAION), at(1, Monster, WARWOLF)];
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[Summon(SANDAION), EndTurn, SetMonster(KURIBOH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SANDAION), 0, "{}", s.context());
}

/// Anime Sandaion: "At the end of the Damage Step, if this card battled: Inflict 4000 damage to your opponent."
#[test]
fn sandaion_anime_inflicts_4000_after_battle() {
    // The anime card has no free Normal Summon: three Tributes.
    let spawns = [at(1, Hand, SANDAION), at(1, Monster, KURIBOH), at(1, Monster, KURIBOH), at(1, Monster, KURIBOH), at(0, Monster, WARWOLF)];
    let Some(mut s) = anime(&spawns) else { return };
    s.run_both([&[EndTurn], &[Summon(SANDAION), Attack(SANDAION), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "{}", s.context());
}

/// Sephylon, the Ultimate Timelord: "Must be Special Summoned (from your hand) by having 10 or more monsters in your GY."
#[test]
fn sephylon_is_special_summoned_with_ten_monsters_in_the_graveyard() {
    let mut spawns = vec![at(0, Hand, SEPHYLON)];
    spawns.extend([at(0, Grave, KURIBOH); 10]);
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[SpSummon(SEPHYLON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", SEPHYLON), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, SEPHYLON), Some(4000), "{}", s.context());
}

/// "Once per turn: You can Special Summon 1 Level 8 or higher Fairy monster from your hand or GY, but its effects are
/// negated, also its ATK becomes 4000."
#[test]
fn sephylon_summons_a_high_level_fairy_with_4000_atk() {
    let mut spawns = vec![at(0, Hand, SEPHYLON), at(0, Hand, TUALATIN)];
    spawns.extend([at(0, Grave, KURIBOH); 10]);
    let Some(mut s) = printed(&spawns) else { return };
    s.run(&[SpSummon(SEPHYLON), Activate(SEPHYLON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", TUALATIN), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, TUALATIN), Some(4000), "2800 printed: {}", s.context());
}
