//! Group 03 of the printed anime cards (rows 4848-5205 of `engine/added/cards.tsv`), Clear Effector to Dark Corridor: one
//! scenario per effect through the real core, with both seats' client models checked after every step
//! (`duelcheck::scenario`). Anime cards off plays the printed text; a few tests turn them on to play the anime version.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The added cards of this group (client ids).
const CLEAR_EFFECTOR: u16 = 12999;
const CLEAR_PHANTOM: u16 = 5056;
const CLEAR_RAGE_GOLEM: u16 = 5057;
const CLEAR_VICIOUS_KNIGHT: u16 = 5064;
const CLEAR_WALL: u16 = 5068;
const CLOCK_RESONATOR: u16 = 9478;
const CLOCKWORK_NIGHT: u16 = 4701;
const COMBAT_WHEEL: u16 = 4994;
const COMIC_HAND: u16 = 11904;
const CONDUCTION_WARRIOR: u16 = 5077; // Conduction Warrior Linear Magnum Plus Minus
const CONVERGING_WISHES: u16 = 13001;
const COUNTERFORCE: u16 = 9938;
const CRIMSON_BLADER: u16 = 9801;
const CRYSTAL_SKULL: u16 = 4793;
const CURSE_REFLECTION_DOLL: u16 = 4796;
const BENTEN: u16 = 6845; // Cyber Angel Benten
const DAKINI: u16 = 6847; // Cyber Angel Dakini
const IDATEN: u16 = 6846; // Cyber Angel Idaten
const CYBER_PETIT_ANGEL: u16 = 6848;
const CYBERNETIC_FUSION_SUPPORT: u16 = 10982;
const D_BURST: u16 = 5173;
const D_FORCE: u16 = 4002;
const D_CUBED: u16 = 12708;
const DARK_CORRIDOR: u16 = 4824;
// The game's own.
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_HOLE: u16 = 4342;
const BLUE_EYES: u16 = 4007;
const DARK_MAGICIAN: u16 = 4041; // Level 7, 2500 ATK
const CLEAR_WORLD: u16 = 8539;
const TOON_WORLD: u16 = 4798;
const MAGNET_SIGMA_PLUS: u16 = 5087; // an added card: the stand-in id, EARTH Level 4
const MAGNET_SIGMA_MINUS: u16 = 5089; // an added card: the stand-in id
const JUNK_SYNCHRON: u16 = 7687;
const JUNK_WARRIOR: u16 = 7696; // Junk Synchron + a non-Tuner, Level 5
const STARDUST: u16 = 7734;
const NEGATE_ATTACK: u16 = 4804;
const OVERDRIVE: u16 = 4982;
const MACHINE_ANGEL_RITUAL: u16 = 6850;
const CYBER_DRAGON: u16 = 6390;
const CYBER_TWIN_DRAGON: u16 = 6396;
const POLYMERIZATION: u16 = 4837;
const AXE_OF_DESPAIR: u16 = 4310;
const DESTINY_PLASMA: u16 = 7096; // Destiny HERO - Plasma
const DESTINY_FEAR_MONGER: u16 = 6756;
const DESTINY_DEFENDER: u16 = 6755;
const WEEPING_IDOL: u16 = 8632; // a Rock with 0 ATK
const BRRON: u16 = 6503; // Brron, Mad King of Dark World
const BLACK_ROSE: u16 = 7898;
const ANCIENT_FAIRY: u16 = 8090;
const DARK_END: u16 = 7371;
const BLACK_WINGED: u16 = 8815;
const BLACK_BRUTDRAGO: u16 = 8478;

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn spawn(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// A duel with `anime` cards on or off: each main deck starts with the listed cards and is filled up with Kuriboh to 40,
/// `spawns` in place. The seed is the first one that leaves those listed cards in the Deck (not in the opening hand),
/// so a search effect really searches.
fn duel(anime: bool, main: [&[u16]; 2], extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let fill = |m: &[u16]| -> Vec<u16> { m.iter().copied().chain(std::iter::repeat(KURIBOH)).take(40).collect() };
    let (m0, m1) = (fill(main[0]), fill(main[1]));
    for seed in 7..60 {
        let decks: [(&[u16], &[u16]); 2] = [(&m0, extra[0]), (&m1, extra[1])];
        let s = if anime { Scenario::start_anime_field(&dir, seed, 8000, decks, spawns) } else { Scenario::start_field(&dir, seed, 8000, decks, spawns) }.unwrap();
        let in_hand = |team: usize, listed: &[u16]| {
            listed.iter().all(|&c| s.count(team, "hand", c) == spawns.iter().filter(|p| p.team as usize == team && p.cid == c && matches!(p.place, Hand)).count())
        };
        if in_hand(0, main[0]) && in_hand(1, main[1]) {
            return Some(s);
        }
    }
    panic!("no seed keeps {main:?} out of the opening hands");
}

fn plain(anime: bool, spawns: &[Spawn]) -> Option<Scenario> {
    duel(anime, [&[], &[]], [&[], &[]], spawns)
}

// ---- Clear Effector ----

/// Clear Effector: "If this card is sent to the Graveyard as a Synchro Material: Draw 1 card. A Synchro Monster that used
/// this card as a Synchro Material cannot be destroyed by card effects." Junk Synchron (3) and Clear Effector (2) make
/// Junk Warrior (5): a card is drawn, and Dark Hole leaves Junk Warrior while it takes the opponent's monster.
#[test]
fn clear_effector_draws_and_protects_its_synchro_monster() {
    let spawns = [spawn(0, Monster, CLEAR_EFFECTOR), spawn(0, Monster, JUNK_SYNCHRON), spawn(0, Hand, DARK_HOLE), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, [&[], &[]], [&[JUNK_WARRIOR], &[]], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[SpSummon(JUNK_WARRIOR)]).unwrap();
    clean(&s);
    assert!(s.monster(0, JUNK_WARRIOR).is_some(), "Junk Warrior came: {}", s.context());
    assert_eq!(s.count(0, "grave", CLEAR_EFFECTOR), 1, "{}", s.context());
    assert_eq!(hand(&s, 0), before + 1, "one card drawn: {}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert!(s.monster(0, JUNK_WARRIOR).is_some(), "Junk Warrior survived Dark Hole: {}", s.context());
    assert!(s.monster(1, WARWOLF).is_none(), "the opponent's monster did not: {}", s.context());
}

/// Clear Effector (anime): "If this card is sent to the Graveyard for a Synchro Summon: Draw 1 card."
#[test]
fn clear_effector_anime_draws_for_a_synchro_summon() {
    let spawns = [spawn(0, Monster, CLEAR_EFFECTOR), spawn(0, Monster, JUNK_SYNCHRON)];
    let Some(mut s) = duel(true, [&[], &[]], [&[JUNK_WARRIOR], &[]], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[SpSummon(JUNK_WARRIOR)]).unwrap();
    clean(&s);
    assert!(s.monster(0, JUNK_WARRIOR).is_some(), "{}", s.context());
    assert_eq!(hand(&s, 0), before + 1, "one card drawn: {}", s.context());
}

// ---- The Clear World family ----

/// Clear Phantom: "You can discard 2 cards, including this card; add 2 cards from your Deck to your hand, that are "Clear
/// World" or Spells/Traps that mention it." Clear World and Clear Wall are in the Deck.
#[test]
fn clear_phantom_discards_two_for_clear_world_cards() {
    let spawns = [spawn(0, Hand, CLEAR_PHANTOM)];
    let Some(mut s) = duel(false, [&[CLEAR_WORLD, CLEAR_WALL], &[]], [&[], &[]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(CLEAR_PHANTOM)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CLEAR_PHANTOM), 1, "Phantom was discarded: {}", s.context());
    assert_eq!((s.count(0, "hand", CLEAR_WORLD), s.count(0, "hand", CLEAR_WALL)), (1, 1), "{}", s.context());
}

/// Clear Phantom: "If this card is destroyed: You can target 1 monster your opponent controls; destroy it, and if you do,
/// send the top 3 cards of their Deck to the GY." The opponent's Warwolf destroys Phantom in battle, then is destroyed by it.
#[test]
fn clear_phantom_destroyed_destroys_a_monster_and_mills_three() {
    let spawns = [spawn(0, Monster, CLEAR_PHANTOM), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[EndTurn, Chain(CLEAR_PHANTOM)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CLEAR_PHANTOM), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", KURIBOH), 3, "three cards milled: {}", s.context());
}

/// Clear Rage Golem: "When this card inflicts battle damage to your opponent: You can inflict 300 damage to your opponent
/// for each card in their hand." The opponent's Golem (the second player can enter its Battle Phase in the first round)
/// hits directly for 1600, then 300 a card.
#[test]
fn clear_rage_golem_burns_for_each_card_in_their_hand() {
    let Some(mut s) = plain(false, &[spawn(1, Monster, CLEAR_RAGE_GOLEM)]) else { return };
    s.run_both([&[EndTurn], &[Attack(CLEAR_RAGE_GOLEM), Chain(CLEAR_RAGE_GOLEM)]]).unwrap();
    clean(&s);
    let cards = hand(&s, 0) as i64;
    assert!(cards > 0, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 1600 - 300 * cards, "{}", s.context());
}

/// Clear Rage Golem: "If this card is Normal or Special Summoned: You can activate this effect; this turn, your monsters
/// that mention "Clear World" can attack directly." Clear Phantom (it mentions the card) hits for 1200 past a defender.
#[test]
fn clear_rage_golem_lets_clear_world_monsters_attack_directly() {
    let spawns = [spawn(1, Hand, CLEAR_RAGE_GOLEM), spawn(1, Monster, CLEAR_PHANTOM), spawn(0, MonsterDefense, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[EndTurn], &[Summon(CLEAR_RAGE_GOLEM), Chain(CLEAR_RAGE_GOLEM), Attack(CLEAR_PHANTOM)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1200, "Phantom attacked directly: {}", s.context());
}

/// Clear Vicious Knight: "You can Tribute Summon this card by Tributing 1 monster that mentions "Clear World". Gains ATK
/// equal to the highest original ATK among monsters your opponent controls." One Tribute (Phantom) for a Level 7;
/// 2300 + Blue-Eyes' 3000.
#[test]
fn clear_vicious_knight_takes_one_tribute_and_gains_their_highest_atk() {
    let spawns = [spawn(0, Hand, CLEAR_VICIOUS_KNIGHT), spawn(0, Monster, CLEAR_PHANTOM), spawn(1, Monster, BLUE_EYES), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.prefer(&[(0, CLEAR_PHANTOM)]);
    s.run(&[Summon(CLEAR_VICIOUS_KNIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CLEAR_PHANTOM), 1, "the Tribute: {}", s.context());
    assert_eq!(s.attack_of(0, CLEAR_VICIOUS_KNIGHT), Some(5300), "{}", s.context());
}

/// Clear Wall: "If "Clear World" is on the field: You can add 1 monster that mentions "Clear World" from your Deck to your
/// hand." Clear Phantom is the one in the Deck.
#[test]
fn clear_wall_adds_a_clear_world_monster_with_clear_world_out() {
    let spawns = [spawn(0, Hand, CLEAR_WORLD), spawn(0, Hand, CLEAR_WALL)];
    let Some(mut s) = duel(false, [&[CLEAR_PHANTOM], &[]], [&[], &[]], &spawns) else { return };
    s.run(&[Activate(CLEAR_WORLD), Activate(CLEAR_WALL), Activate(CLEAR_WALL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", CLEAR_PHANTOM), 1, "{}", s.context());
}

/// Clear Wall: "Monsters that mention "Clear World" cannot be destroyed by battle, also you take no battle damage from
/// that battle." Warwolf (2000) attacks Clear Phantom (1200): Phantom stays and no damage is taken.
#[test]
fn clear_wall_keeps_clear_world_monsters_alive_and_unhurt() {
    let spawns = [spawn(0, Hand, CLEAR_WALL), spawn(0, Monster, CLEAR_PHANTOM), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[Activate(CLEAR_WALL), EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, CLEAR_PHANTOM).is_some(), "Phantom survived: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Clear Wall (anime): "If there is no "Clear World" on the field, destroy this card."
#[test]
fn clear_wall_anime_is_destroyed_without_clear_world() {
    let Some(mut s) = plain(true, &[spawn(0, Hand, CLEAR_WALL)]) else { return };
    s.run(&[Activate(CLEAR_WALL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", CLEAR_WALL), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", CLEAR_WALL), 1, "{}", s.context());
}

// ---- Clock Resonator ----

/// Clock Resonator: "Once per turn, this face-up Defense Position card cannot be destroyed by battle or by card effects."
/// Warwolf (2000) attacks it in Defense (600): it stays.
#[test]
fn clock_resonator_survives_a_battle_in_defense() {
    let spawns = [spawn(0, MonsterDefense, CLOCK_RESONATOR), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, CLOCK_RESONATOR).is_some(), "{}", s.context());
}

/// Clock Resonator: the same protects against a card effect (Dark Hole), but only once per turn: the battle used it, the
/// Dark Hole after it destroys.
#[test]
fn clock_resonator_is_protected_only_once_a_turn() {
    let spawns = [spawn(0, MonsterDefense, CLOCK_RESONATOR), spawn(1, Monster, WARWOLF), spawn(1, Hand, DARK_HOLE)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, CLOCK_RESONATOR).is_some(), "Dark Hole alone does not destroy it: {}", s.context());
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF), Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, CLOCK_RESONATOR).is_none(), "the second attempt of the turn destroys it: {}", s.context());
}

// ---- Clockwork Night ----

/// Clockwork Night: "Machine monsters you control gain 500 ATK/DEF, also Machine monsters your opponent controls lose 500
/// ATK/DEF", and all face-up monsters become Machines: Warwolf is 2500 for its owner, 1500 for the opponent.
#[test]
fn clockwork_night_turns_monsters_into_machines_that_gain_and_lose() {
    let spawns = [spawn(0, Hand, CLOCKWORK_NIGHT), spawn(0, Monster, WARWOLF), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(CLOCKWORK_NIGHT)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, WARWOLF), s.attack_of(1, WARWOLF)), (Some(2500), Some(1500)), "{}", s.context());
}

/// Clockwork Night: "You can banish this card from your GY, then discard 1 card; add 1 EARTH Machine monster from your
/// Deck to your hand." Overdrive is the EARTH Machine.
#[test]
fn clockwork_night_in_the_graveyard_adds_an_earth_machine() {
    let Some(mut s) = duel(false, [&[OVERDRIVE], &[]], [&[], &[]], &[spawn(0, Grave, CLOCKWORK_NIGHT)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(CLOCKWORK_NIGHT)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", CLOCKWORK_NIGHT), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", OVERDRIVE), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "one card discarded: {}", s.context());
}

// ---- Combat Wheel ----

/// Combat Wheel: "The first time this card would be destroyed by an opponent's card effect each turn, it is not destroyed."
/// Dark Hole from the opponent takes Warwolf and leaves the Wheel.
#[test]
fn combat_wheel_survives_the_first_destruction_by_an_effect() {
    let spawns = [spawn(0, Monster, COMBAT_WHEEL), spawn(0, Monster, WARWOLF), spawn(1, Hand, DARK_HOLE)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, COMBAT_WHEEL).is_some(), "the Wheel stays: {}", s.context());
    assert!(s.monster(0, WARWOLF).is_none(), "Warwolf does not: {}", s.context());
}

/// Combat Wheel: "Once per turn, during your opponent's Battle Phase (Quick Effect): You can discard 1 card; this card gains
/// ATK equal to half the total ATK of all other monsters you control." 2500 + half of Warwolf's 2000 = 3500 beats Blue-Eyes.
#[test]
fn combat_wheel_gains_half_the_others_atk_in_their_battle_phase() {
    let spawns = [spawn(0, Monster, COMBAT_WHEEL), spawn(0, Monster, WARWOLF), spawn(1, Monster, BLUE_EYES)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, Chain(COMBAT_WHEEL)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, COMBAT_WHEEL), Some(3500), "{}", s.context());
}

// ---- Comic Hand ----

/// Comic Hand: "If you control "Toon World", equip to an opponent's monster. Take control of it." Warwolf changes sides.
#[test]
fn comic_hand_takes_control_of_a_monster_with_toon_world() {
    let spawns = [spawn(0, Hand, TOON_WORLD), spawn(0, Hand, COMIC_HAND), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(TOON_WORLD), Activate(COMIC_HAND)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", WARWOLF)), (1, 0), "{}", s.context());
}

/// Comic Hand: "If your opponent controls no Toon monsters, it can attack your opponent directly." The stolen Warwolf hits
/// for 2000 although the opponent has another monster.
#[test]
fn comic_hand_lets_the_stolen_monster_attack_directly() {
    let spawns = [spawn(0, Hand, TOON_WORLD), spawn(0, Hand, COMIC_HAND), spawn(1, Monster, WARWOLF), spawn(1, Monster, BLUE_EYES)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(TOON_WORLD), Activate(COMIC_HAND), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
}

// ---- Conduction Warrior Linear Magnum Plus Minus ----

/// Conduction Warrior: "Must first be Special Summoned (from your hand) by sending 2 "Magnet Warrior" monsters with an
/// original Level of 4 or lower from your hand, Deck, and/or face-up field to the GY."
#[test]
fn conduction_warrior_is_summoned_by_sending_two_magnet_warriors() {
    let spawns = [spawn(0, Hand, CONDUCTION_WARRIOR), spawn(0, Hand, MAGNET_SIGMA_PLUS), spawn(0, Hand, MAGNET_SIGMA_MINUS)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.prefer(&[(0, MAGNET_SIGMA_PLUS), (0, MAGNET_SIGMA_MINUS)]);
    s.run(&[SpSummon(CONDUCTION_WARRIOR)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, CONDUCTION_WARRIOR), Some(2700), "{}", s.context());
    // Sigma Plus' own effect (sent to the GY: add another Magnet Warrior from the GY to hand) takes Sigma Minus back.
    assert_eq!(s.count(0, "grave", MAGNET_SIGMA_PLUS), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", MAGNET_SIGMA_MINUS) + s.count(0, "hand", MAGNET_SIGMA_MINUS), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", MAGNET_SIGMA_MINUS), 0, "{}", s.context());
}

/// Conduction Warrior: "You can target 1 other EARTH monster on the field; this card gains half that monster's ATK until
/// the end of this turn." Warwolf (EARTH, 2000): 2700 + 1000.
#[test]
fn conduction_warrior_gains_half_an_earth_monsters_atk() {
    let spawns = [spawn(0, Monster, CONDUCTION_WARRIOR), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(CONDUCTION_WARRIOR)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, CONDUCTION_WARRIOR), Some(3700), "{}", s.context());
}

/// Conduction Warrior: "If this card is destroyed and sent to the GY: Add it to your hand." Dark Hole destroys it.
#[test]
fn conduction_warrior_returns_to_the_hand_when_destroyed() {
    let spawns = [spawn(0, Monster, CONDUCTION_WARRIOR), spawn(0, Hand, DARK_HOLE)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", CONDUCTION_WARRIOR), 1, "{}", s.context());
}

// ---- Converging Wishes ----

/// Converging Wishes: "If you have 5 or more Dragon Synchro Monsters with different names in your GY: Special Summon 1
/// "Stardust Dragon" from your Extra Deck, and equip it with this card, but banish it during the End Phase. That equipped
/// monster gains ATK equal to the combined ATK of all Dragon Synchro Monsters in your GY."
#[test]
fn converging_wishes_summons_stardust_with_the_graveyards_atk() {
    let graveyard = [BLACK_ROSE, ANCIENT_FAIRY, DARK_END, BLACK_WINGED, BLACK_BRUTDRAGO];
    let mut spawns: Vec<Spawn> = graveyard.iter().map(|&c| spawn(0, Grave, c)).collect();
    spawns.push(spawn(0, SpellSet, CONVERGING_WISHES));
    let Some(mut s) = duel(false, [&[], &[]], [&[STARDUST], &[]], &spawns) else { return };
    s.run(&[Activate(CONVERGING_WISHES)]).unwrap();
    clean(&s);
    // 2400 + 2100 + 2600 + 2800 + 3000 = 12900, on Stardust's own 2500.
    assert_eq!(s.attack_of(0, STARDUST), Some(2500 + 12900), "{}", s.context());
}

// ---- Counterforce ----

/// Counterforce: "When a monster's attack is negated that targeted a face-up Attack Position monster: Inflict damage to
/// your opponent equal to the difference in ATK between the 2 monsters." Negate Attack stops Blue-Eyes (3000) at Warwolf
/// (2000): 1000 damage.
#[test]
fn counterforce_inflicts_the_difference_after_a_negated_attack() {
    let spawns = [spawn(0, Monster, WARWOLF), spawn(0, SpellSet, NEGATE_ATTACK), spawn(0, SpellSet, COUNTERFORCE), spawn(1, Monster, BLUE_EYES)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn, ChainOnAttack(NEGATE_ATTACK), Chain(COUNTERFORCE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

// ---- Crimson Blader ----

/// Crimson Blader: "If this card destroys an opponent's monster by battle and sends it to the Graveyard: Your opponent
/// cannot Normal or Special Summon Level 5 or higher monsters during their next turn." The opponent's Blader attacks in
/// the second turn; the first player's Cyber Dragon (Level 5) is summonable in its own next turn only if nothing was killed.
#[test]
fn crimson_blader_stops_level_5_or_higher_summons_after_a_kill() {
    let spawns = [spawn(1, Monster, CRIMSON_BLADER), spawn(0, Monster, WARWOLF), spawn(0, Hand, CYBER_DRAGON)];
    let Some(mut s) = plain(false, &spawns) else { return };
    let outcome = s.run_both([&[EndTurn, SpSummon(CYBER_DRAGON)], &[Attack(CRIMSON_BLADER)]]);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Warwolf was destroyed: {}", s.context());
    assert!(outcome.is_err(), "Cyber Dragon never became possible: {}", s.context());
    assert_eq!(s.count(0, "mzone", CYBER_DRAGON), 0, "{}", s.context());
    // The same board without the kill (no Warwolf to destroy): Cyber Dragon comes at once.
    let Some(mut s) = plain(false, &[spawn(1, Monster, CRIMSON_BLADER), spawn(0, Hand, CYBER_DRAGON)]) else { return };
    s.run(&[SpSummon(CYBER_DRAGON)]).unwrap();
    assert_eq!(s.count(0, "mzone", CYBER_DRAGON), 1, "{}", s.context());
}

/// Crimson Blader (anime): "If this card destroys an opponent's Synchro Monster by battle: Your opponent cannot Summon
/// during their next turn." Junk Warrior falls to it, and Warwolf can't be Normal Summoned after.
#[test]
fn crimson_blader_anime_stops_all_summons_after_a_synchro_kill() {
    let spawns = [spawn(1, Monster, CRIMSON_BLADER), spawn(0, Monster, JUNK_WARRIOR), spawn(0, Hand, WARWOLF)];
    let Some(mut s) = plain(true, &spawns) else { return };
    let outcome = s.run_both([&[EndTurn, Summon(WARWOLF)], &[Attack(CRIMSON_BLADER)]]);
    assert_eq!(s.count(0, "grave", JUNK_WARRIOR), 1, "Junk Warrior was destroyed: {}", s.context());
    assert!(outcome.is_err(), "Warwolf never became summonable: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
}

// ---- Crystal Skull and Curse Reflection Doll ----

/// Crystal Skull: "If this card is Summoned: Take 1000 damage, then change this card to Defense Position."
#[test]
fn crystal_skull_hurts_its_player_and_turns_to_defense() {
    let Some(mut s) = plain(false, &[spawn(0, Hand, CRYSTAL_SKULL)]) else { return };
    s.run(&[Summon(CRYSTAL_SKULL)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "{}", s.context());
    assert_eq!(s.position_of(0, CRYSTAL_SKULL), Some(4), "{}", s.context());
}

/// Curse Reflection Doll: "When a monster effect is activated that would inflict damage to you: Your opponent takes that
/// damage instead." Crystal Skull's 1000 goes to the opponent (and, with no damage taken, Skull does not turn to Defense).
#[test]
fn curse_reflection_doll_reflects_crystal_skulls_damage() {
    let spawns = [spawn(0, Hand, CRYSTAL_SKULL), spawn(0, SpellSet, CURSE_REFLECTION_DOLL)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Summon(CRYSTAL_SKULL), Chain(CURSE_REFLECTION_DOLL)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7000), "{}", s.context());
    assert_eq!(s.count(0, "grave", CURSE_REFLECTION_DOLL), 1, "{}", s.context());
}

/// Crystal Skull: "During the End Phase, if you did not take effect damage this turn: You can either add to your hand or
/// Special Summon 1 Rock monster with 0 ATK from your Deck." Skull is already on the field, nothing hurt its player.
#[test]
fn crystal_skull_fetches_a_rock_in_the_end_phase_without_effect_damage() {
    let Some(mut s) = duel(false, [&[WEEPING_IDOL], &[]], [&[], &[]], &[spawn(0, Monster, CRYSTAL_SKULL)]) else { return };
    s.run(&[EndTurn, Chain(CRYSTAL_SKULL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", WEEPING_IDOL) + s.count(0, "mzone", WEEPING_IDOL), 1, "{}", s.context());
}

/// Curse Reflection Doll: "When you take battle damage, while this card is in your GY: You can Set this card."
#[test]
fn curse_reflection_doll_sets_itself_after_battle_damage() {
    let spawns = [spawn(0, Grave, CURSE_REFLECTION_DOLL), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run_both([&[EndTurn, Chain(CURSE_REFLECTION_DOLL)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6000, "{}", s.context());
    assert_eq!(s.count(0, "szone", CURSE_REFLECTION_DOLL), 1, "{}", s.context());
}

// ---- The Cyber Angels ----

/// Cyber Angel Benten: "You can Ritual Summon this card with "Machine Angel Ritual"." Two Warwolves (8 Levels) pay for
/// the Level 6.
#[test]
fn cyber_angel_benten_is_ritual_summoned_with_machine_angel_ritual() {
    let spawns = [spawn(0, Hand, BENTEN), spawn(0, Hand, MACHINE_ANGEL_RITUAL), spawn(0, Monster, WARWOLF), spawn(0, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(MACHINE_ANGEL_RITUAL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", BENTEN), 1, "{}", s.context());
}

/// Cyber Angel Benten: "If this card destroys a monster by battle and sends it to the GY: Inflict damage to your opponent
/// equal to that monster's original DEF in the GY." Benten (1800) beats Kuriboh (300/200): 1500 + 200.
#[test]
fn cyber_angel_benten_burns_for_the_defense_of_what_it_destroys() {
    let spawns = [spawn(0, Monster, BENTEN), spawn(1, Monster, KURIBOH)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Attack(BENTEN)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1500 - 200, "{}", s.context());
}

/// Cyber Angel Benten: "If this card is Tributed: You can add 1 LIGHT Fairy monster from your Deck to your hand." Benten
/// and Warwolf are the two Tributes of Dark Magician; Idaten comes.
#[test]
fn cyber_angel_benten_tributed_adds_a_light_fairy() {
    let spawns = [spawn(0, Monster, BENTEN), spawn(0, Monster, WARWOLF), spawn(0, Hand, DARK_MAGICIAN)];
    let Some(mut s) = duel(false, [&[IDATEN], &[]], [&[], &[]], &spawns) else { return };
    s.prefer(&[(0, BENTEN), (0, WARWOLF)]);
    s.run(&[Summon(DARK_MAGICIAN), Chain(BENTEN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", IDATEN), 1, "{}", s.context());
}

/// Cyber Angel Dakini: "If this card is Ritual Summoned: You can make your opponent send 1 monster they control to the GY
/// (their choice)."
#[test]
fn cyber_angel_dakini_makes_the_opponent_send_a_monster() {
    let spawns = [spawn(0, Hand, DAKINI), spawn(0, Hand, MACHINE_ANGEL_RITUAL), spawn(0, Monster, BLUE_EYES), spawn(1, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(MACHINE_ANGEL_RITUAL), Chain(DAKINI)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", DAKINI), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
}

/// Cyber Angel Dakini: "If a Ritual Monster you control attacks a Defense Position monster, inflict piercing battle damage
/// to your opponent." Dakini (2700) over Kuriboh in Defense (200): 2500.
#[test]
fn cyber_angel_dakini_pierces_defense() {
    let spawns = [spawn(0, Monster, DAKINI), spawn(1, MonsterDefense, KURIBOH)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Attack(DAKINI)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2500, "{}", s.context());
}

/// Cyber Angel Dakini: "Once per turn, during your End Phase: You can target 1 Ritual Monster or 1 "Machine Angel Ritual"
/// in your GY; add it to your hand."
#[test]
fn cyber_angel_dakini_recovers_a_ritual_card_in_the_end_phase() {
    let spawns = [spawn(0, Monster, DAKINI), spawn(0, Grave, MACHINE_ANGEL_RITUAL)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[EndTurn, Chain(DAKINI)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", MACHINE_ANGEL_RITUAL), 1, "{}", s.context());
}

/// Cyber Angel Idaten: "If this card is Ritual Summoned: You can add 1 Ritual Spell from your Deck or GY to your hand."
/// A second Machine Angel Ritual in the Deck.
#[test]
fn cyber_angel_idaten_adds_a_ritual_spell() {
    let spawns = [spawn(0, Hand, IDATEN), spawn(0, Hand, MACHINE_ANGEL_RITUAL), spawn(0, Monster, BLUE_EYES)];
    let Some(mut s) = duel(false, [&[MACHINE_ANGEL_RITUAL], &[]], [&[], &[]], &spawns) else { return };
    assert_eq!(s.count(0, "hand", MACHINE_ANGEL_RITUAL), 1, "the one in hand, the other in the Deck");
    s.run(&[Activate(MACHINE_ANGEL_RITUAL), Chain(IDATEN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", IDATEN), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", MACHINE_ANGEL_RITUAL), s.count(0, "grave", MACHINE_ANGEL_RITUAL)), (1, 1), "{}", s.context());
}

/// Cyber Angel Idaten: "If this card is Tributed: You can make all Ritual Monsters you control gain 1000 ATK/DEF." Idaten
/// and Warwolf are the Tributes for Dark Magician; Benten (1800) becomes 2800.
#[test]
fn cyber_angel_idaten_tributed_gives_ritual_monsters_1000() {
    let spawns = [spawn(0, Monster, IDATEN), spawn(0, Monster, WARWOLF), spawn(0, Monster, BENTEN), spawn(0, Hand, DARK_MAGICIAN)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.prefer(&[(0, IDATEN), (0, WARWOLF)]);
    s.run(&[Summon(DARK_MAGICIAN), Chain(IDATEN)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, BENTEN), Some(2800), "{}", s.context());
}

/// Cyber Petit Angel: "If this card is Summoned: You can add 1 "Cyber Angel" monster or "Machine Angel Ritual" from your
/// Deck to your hand." Idaten is chosen over the ritual Spell.
#[test]
fn cyber_petit_angel_adds_a_cyber_angel() {
    let Some(mut s) = duel(false, [&[IDATEN, MACHINE_ANGEL_RITUAL], &[]], [&[], &[]], &[spawn(0, Hand, CYBER_PETIT_ANGEL)]) else { return };
    s.prefer(&[(0, IDATEN)]);
    s.run(&[Summon(CYBER_PETIT_ANGEL), Chain(CYBER_PETIT_ANGEL)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", IDATEN), s.count(0, "hand", MACHINE_ANGEL_RITUAL)), (1, 0), "{}", s.context());
}

/// Cyber Petit Angel (anime): "When this card is Normal or Flip Summoned: Add 1 "Machine Angel Ritual" from your Deck to
/// your hand." The ritual Spell, not Idaten.
#[test]
fn cyber_petit_angel_anime_adds_only_the_ritual_spell() {
    let Some(mut s) = duel(true, [&[IDATEN, MACHINE_ANGEL_RITUAL], &[]], [&[], &[]], &[spawn(0, Hand, CYBER_PETIT_ANGEL)]) else { return };
    s.run(&[Summon(CYBER_PETIT_ANGEL)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", IDATEN), s.count(0, "hand", MACHINE_ANGEL_RITUAL)), (0, 1), "{}", s.context());
}

// ---- Cybernetic Fusion Support ----

/// Cybernetic Fusion Support: "Pay half your LP; once, if you Fusion Summon a Machine Fusion Monster this turn, you can
/// banish monsters from your hand, field, and/or GY as the Fusion Materials." Two Cyber Dragons in the GY make a Cyber Twin
/// Dragon.
#[test]
fn cybernetic_fusion_support_banishes_the_graveyard_as_materials() {
    let spawns = [spawn(0, Hand, CYBERNETIC_FUSION_SUPPORT), spawn(0, Hand, POLYMERIZATION), spawn(0, Grave, CYBER_DRAGON), spawn(0, Grave, CYBER_DRAGON)];
    let Some(mut s) = duel(false, [&[], &[]], [&[CYBER_TWIN_DRAGON], &[]], &spawns) else { return };
    s.run(&[Activate(CYBERNETIC_FUSION_SUPPORT), Activate(POLYMERIZATION)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 4000, "half the LP: {}", s.context());
    assert_eq!(s.count(0, "mzone", CYBER_TWIN_DRAGON), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", CYBER_DRAGON), 2, "{}", s.context());
}

// ---- D - Burst, D - Force, D Cubed ----

/// D - Burst: "Target 1 face-up Spell you control; destroy it, and if you do, draw 1 card, then if you control a "Destiny
/// HERO" monster, you can Special Summon 1 "Destiny HERO" monster from your hand, GY, or banishment."
#[test]
fn d_burst_destroys_a_spell_draws_and_summons_a_destiny_hero() {
    let spawns = [spawn(0, Hand, D_BURST), spawn(0, Spell, TOON_WORLD), spawn(0, Monster, DESTINY_FEAR_MONGER), spawn(0, Grave, DESTINY_DEFENDER)];
    let Some(mut s) = plain(false, &spawns) else { return };
    let before = hand(&s, 0);
    s.prefer(&[(0, TOON_WORLD), (0, DESTINY_DEFENDER)]);
    s.run(&[Activate(D_BURST)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", TOON_WORLD), 1, "the Spell was destroyed: {}", s.context());
    assert_eq!(hand(&s, 0), before, "D - Burst left, one card drawn: {}", s.context());
    assert_eq!(s.count(0, "mzone", DESTINY_DEFENDER), 1, "{}", s.context());
}

/// D - Burst: "At the end of the Damage Step, if a monster equipped with an Equip Card ... attacked: You can banish this
/// card from your GY; that monster can make a second attack in a row." Warwolf with Axe of Despair (3000) hits the empty
/// field twice.
#[test]
fn d_burst_from_the_graveyard_gives_a_second_attack() {
    let spawns = [spawn(0, Grave, D_BURST), spawn(0, Hand, AXE_OF_DESPAIR), spawn(0, Monster, WARWOLF)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(AXE_OF_DESPAIR), Attack(WARWOLF), Chain(D_BURST), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2 * 3000, "{}", s.context());
    assert_eq!(s.count(0, "removed", D_BURST), 1, "{}", s.context());
}

/// D - Force: "When this card is activated: You can add 1 "Destiny HERO - Plasma" from your Deck or GY to your hand."
#[test]
fn d_force_adds_plasma() {
    let Some(mut s) = duel(false, [&[DESTINY_PLASMA], &[]], [&[], &[]], &[spawn(0, Hand, D_FORCE)]) else { return };
    s.run(&[Activate(D_FORCE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DESTINY_PLASMA), 1, "{}", s.context());
}

/// D - Force: "Each "Destiny HERO - Plasma" you control gains 100 ATK for each monster in the GYs": Warwolf in one
/// Graveyard, Kuriboh in the other, Plasma 1900 + 200.
#[test]
fn d_force_gives_plasma_100_atk_per_monster_in_the_graveyards() {
    let spawns = [spawn(0, Hand, D_FORCE), spawn(0, Monster, DESTINY_PLASMA), spawn(0, Grave, WARWOLF), spawn(1, Grave, KURIBOH)];
    let Some(mut s) = plain(false, &spawns) else { return };
    s.run(&[Activate(D_FORCE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DESTINY_PLASMA), Some(2100), "{}", s.context());
}

/// D Cubed: "You can discard up to 2 cards; Special Summon "D Cubed" from your hand, Deck, and/or GY, equal to the number
/// discarded."
#[test]
fn d_cubed_calls_copies_for_each_discard() {
    let spawns = [spawn(0, Monster, D_CUBED)];
    let Some(mut s) = duel(false, [&[D_CUBED, D_CUBED], &[]], [&[], &[]], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(D_CUBED)]).unwrap();
    clean(&s);
    // The scenario's default answer discards the fewest ("up to 2" is one): one discard, one more D Cubed.
    assert_eq!(s.count(0, "mzone", D_CUBED), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "{}", s.context());
}

/// D Cubed: "If this card is destroyed by battle or card effect: You can send 1 "Destiny HERO" monster from your Deck to
/// the GY." Dark Hole destroys it; Fear Monger goes.
#[test]
fn d_cubed_destroyed_sends_a_destiny_hero() {
    let spawns = [spawn(0, Monster, D_CUBED), spawn(0, Hand, DARK_HOLE)];
    let Some(mut s) = duel(false, [&[DESTINY_FEAR_MONGER], &[]], [&[], &[]], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(D_CUBED)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DESTINY_FEAR_MONGER), 1, "{}", s.context());
}

// ---- Dark Corridor ----

/// Dark Corridor: "Add 1 "Dark World" monster from your Deck to your hand, then discard 1 card."
#[test]
fn dark_corridor_adds_a_dark_world_monster_then_discards() {
    let Some(mut s) = duel(false, [&[BRRON], &[]], [&[], &[]], &[spawn(0, Hand, DARK_CORRIDOR)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(DARK_CORRIDOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", BRRON), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 1, "one card discarded: {}", s.context());
}
