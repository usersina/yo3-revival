//! Group 02 of the anime's printed cards (#75/#60, rows 4848-5205 of engine/added/cards.tsv): Berserker Soul to Clear
//! Cube. Each effect through the real core with both seats' client models checked after every step
//! (`duelcheck::scenario`). Printed text with anime cards off; one test of the anime version with them on where it differs.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use serde_json::json;
use yo3_server::ocg::{Response, idle, msg};

// The added cards (ids from engine/added/cards.tsv).
const BERSERKER_SOUL: u16 = 11153;
const BLACK_ILLUSION: u16 = 11129;
const BLACK_SONIC: u16 = 11175;
const HURRICANE: u16 = 11174; // Blackwing - Hurricane the Tornado
const KOGARASHI: u16 = 9480; // Blackwing - Kogarashi the Wanderer
const OROSHI: u16 = 11172; // Blackwing - Oroshi the Squall
const BLUE_FLAME: u16 = 10864; // Blue Flame Swordsman
const BONFIRE: u16 = 4959;
const BOON: u16 = 9521; // Boon of the Meklord Emperor
const BREAK_THE_SEAL: u16 = 5071;
const BRIGHT_STAR: u16 = 9802; // Bright Star Dragon
const BUSTER_BLASTER: u16 = 9493;
const CABRERA: u16 = 4996; // Cabrera Stone
const CANDELATO: u16 = 5163; // Candelato, the Beast of Light
const CARBONEDDON: u16 = 11167;
const DEMISE: u16 = 5576; // Card of Demise
const EATOS_SWORD: u16 = 11156; // Celestial Sword - Eatos
const MINA: u16 = 4673; // Centaur Mina
const CERULI: u16 = 9714; // Ceruli, Guru of Dark World
const CHAIN_RESONATOR: u16 = 12063;
const CHAOS_CORE: u16 = 15137;
const CHAOS_DISTILL: u16 = 5082;
const CHAOS_INFINITY: u16 = 9536;
const CLEAR_CUBE: u16 = 5058;
const TRIANGLE_O: u16 = 4997; // the stand-in of Triangle O
// The game's own.
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;
const POT_OF_GREED: u16 = 4844;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const BLUE_EYES: u16 = 4007;
const DARK_MAGICIAN: u16 = 4041;
const STARDUST: u16 = 7734; // a Synchro Monster, Level 8
const KRIS: u16 = 8163; // Blackwing - Shura the Blue Flame
const ZEPHYROS: u16 = 9244; // Blackwing - Zephyros the Elite
const BORA: u16 = 8012; // Blackwing - Bora the Spear

const KURIBOHS: [u16; 40] = [KURIBOH; 40];

/// A Deck of 20 copies of `card` and 20 Kuriboh: with spawned cards the Decks are shuffled, so a card a test counts on in
/// the Deck comes in numbers, and the test compares before and after.
fn with_copies(card: u16) -> Vec<u16> {
    [card; 20].into_iter().chain([KURIBOH; 20]).collect()
}

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn field(anime: bool, lp: u32, decks: [(&[u16], &[u16]); 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    Some(if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }.unwrap())
}

fn sp(team: usize, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team: team as u32, place, cid }
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// Cards in a place, whatever they are.
fn total(s: &Scenario, team: usize, place: &str) -> usize {
    s.state()[team][place].as_array().map_or(0, |cards| cards.iter().filter(|c| !c.is_null()).count())
}

/// Whether the idle prompt now pending offers this card in its list (`special_summons`, `activates`...).
fn offered(s: &Scenario, list: &str, cid: u16) -> bool {
    s.prompt()[list].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
}

/// The idle prompt's own action on a card now: activates it (a hand card, a Set card, an ignition effect).
fn activate_now(s: &mut Scenario, cid: u16) {
    let i = s.prompt()["activates"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == cid)).unwrap_or_else(|| panic!("{cid} is not offered: {}", s.context()));
    s.respond(json!({"type": 1, "action": idle::ACTIVATE, "index": i})).unwrap();
}

/// Passes until `team` is offered `cid` in a chain window, then chains it (true), or until an idle prompt (false): the
/// answer to the effect just activated, not the first chance a free-chain card gets.
fn chain_in_answer(s: &mut Scenario, team: usize, cid: u16) -> bool {
    for _ in 0..40 {
        let p = s.prompt().clone();
        if p["type"] == msg::SELECT_CHAIN
            && p["player"] == team as u64
            && let Some(i) = p["selects"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == cid))
        {
            s.respond(Response::Chain(Some(i)).json()).unwrap();
            return true;
        }
        if p["type"] == msg::SELECT_IDLECMD {
            return false;
        }
        s.pass().unwrap();
    }
    panic!("no end: {}", s.context());
}

/// Blackwing - Hurricane the Tornado (printed): "Once per turn: You can target 1 Synchro Monster on the field; this card's
/// ATK becomes that monster's current ATK, until the end of this turn." Stardust Dragon (2500) is the target.
#[test]
fn hurricane_takes_a_synchro_monsters_attack() {
    let spawns = [sp(0, Monster, HURRICANE), sp(1, Monster, STARDUST)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    assert_eq!(s.attack_of(0, HURRICANE), Some(0), "{}", s.state());
    s.prefer(&[(1, STARDUST)]);
    s.run(&[Activate(HURRICANE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, HURRICANE), Some(2500), "{}", s.context());
}

/// Anime Hurricane: "When this card is Summoned: Target 1 face-up Synchro Monster on the field; this card's ATK becomes
/// the same as the targeted monster's." Normal Summoned next to Stardust Dragon it takes its 2500 as it arrives.
#[test]
fn anime_hurricane_takes_the_attack_when_summoned() {
    let spawns = [sp(0, Hand, HURRICANE), sp(1, Monster, STARDUST)];
    let Some(mut s) = field(true, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(1, STARDUST)]);
    s.run(&[Summon(HURRICANE)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, HURRICANE), Some(2500), "{}", s.context());
}

/// Black Illusion (printed): "Until the end of this turn, all DARK Spellcaster-Type monsters with 2000 or more ATK you
/// currently control cannot be destroyed by battle, their effects are negated, also they are unaffected by your
/// opponent's card effects." Set, then chained to Dark Hole: Dark Magician (2500) is left standing, Warwolf is not.
#[test]
fn black_illusion_saves_a_dark_spellcaster_from_dark_hole() {
    let spawns = [sp(0, Monster, DARK_MAGICIAN), sp(0, Monster, WARWOLF), sp(0, SpellSet, BLACK_ILLUSION), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[EndTurn]).unwrap(); // team 1's Main Phase
    activate_now(&mut s, DARK_HOLE);
    assert!(chain_in_answer(&mut s, 0, BLACK_ILLUSION), "the Trap is offered: {}", s.context());
    s.run(&[]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_MAGICIAN).is_some(), "Dark Magician is unaffected: {}", s.context());
    assert!(s.monster(0, WARWOLF).is_none(), "Warwolf is not a DARK Spellcaster: {}", s.context());
    assert_eq!(s.count(0, "szone", BLACK_ILLUSION), 0, "the Trap was used: {}", s.context());
}

/// Black Illusion: "cannot be destroyed by battle". Blue-Eyes (3000) attacks Dark Magician (2500, Attack Position): with
/// the Trap chained to the attack Dark Magician stays; its owner still takes the 500 damage.
#[test]
fn black_illusion_keeps_a_dark_spellcaster_alive_in_battle() {
    let spawns = [sp(0, Monster, DARK_MAGICIAN), sp(0, SpellSet, BLACK_ILLUSION), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, ChainOnAttack(BLACK_ILLUSION)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_MAGICIAN).is_some(), "Dark Magician survived the battle: {}", s.context());
    assert_eq!(s.lp(0), 7500, "{}", s.context());
}

/// Anime Black Illusion: "Your opponent draws 1 card, also, until the end of this turn, make 1 "Dark Magician" you
/// control unaffected by monster effects, also it cannot be destroyed by battle." The opponent draws one more card, and
/// Dark Magician survives Blue-Eyes' attack.
#[test]
fn anime_black_illusion_gives_the_opponent_a_card_and_protects_dark_magician() {
    let spawns = [sp(0, Monster, DARK_MAGICIAN), sp(0, SpellSet, BLACK_ILLUSION), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = field(true, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, ChainOnAttack(BLACK_ILLUSION)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_MAGICIAN).is_some(), "Dark Magician survived the battle: {}", s.context());
    assert_eq!(s.count(0, "szone", BLACK_ILLUSION), 0, "the Trap was used: {}", s.context());
    // Team 1 drew for its turn (6) and for the Trap (7); the attack needs no card.
    assert_eq!(hand(&s, 1), 7, "{}", s.context());
}

/// Black Sonic (printed): "When an opponent's monster declares an attack on a "Blackwing" monster you control: Banish all
/// face-up Attack Position monsters your opponent controls." Set, then chained to Blue-Eyes' attack on Shura: Blue-Eyes
/// and Warwolf are banished, the one in Defense Position (Kuriboh) is not.
#[test]
fn black_sonic_banishes_every_attack_position_monster() {
    let spawns = [sp(0, Monster, KRIS), sp(0, SpellSet, BLACK_SONIC), sp(1, Monster, BLUE_EYES), sp(1, Monster, WARWOLF),
        sp(1, MonsterDefense, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, ChainOnAttack(BLACK_SONIC)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "removed", BLUE_EYES), s.count(1, "removed", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.count(1, "mzone", KURIBOH), 1, "the Defense Position monster stays: {}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack never lands: {}", s.context());
}

/// Black Sonic: "If you control exactly 3 "Blackwing" monsters (and no other monsters), you can activate this card from
/// your hand." Three Blackwings and Black Sonic in hand: Blue-Eyes' attack is answered from the hand.
#[test]
fn black_sonic_comes_from_the_hand_with_three_blackwings() {
    let spawns = [sp(0, Monster, KRIS), sp(0, Monster, ZEPHYROS), sp(0, Monster, BORA), sp(0, Hand, BLACK_SONIC), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, ChainOnAttack(BLACK_SONIC)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", BLACK_SONIC), 1, "used from the hand: {}", s.context());
}

/// Blackwing - Kogarashi the Wanderer: "When a face-up "Blackwing" monster you control is destroyed by a card effect and
/// sent to the Graveyard (except during the Damage Step): You can Special Summon this card from your hand." Dark Hole
/// destroys Bora; Kogarashi comes from the hand.
#[test]
fn kogarashi_comes_when_a_blackwing_is_destroyed_by_an_effect() {
    let spawns = [sp(0, Monster, BORA), sp(0, Hand, KOGARASHI), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, Chain(KOGARASHI)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BORA), 1, "Bora was destroyed: {}", s.context());
    assert_eq!(s.attack_of(0, KOGARASHI), Some(2300), "Kogarashi came: {}", s.context());
}

/// Kogarashi's trigger is for a Blackwing destroyed by a card effect: one destroyed in battle does not bring it.
#[test]
fn kogarashi_ignores_a_blackwing_destroyed_in_battle() {
    let spawns = [sp(0, Monster, KRIS), sp(0, Hand, KOGARASHI), sp(1, Monster, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", KRIS), 1, "Shura died in battle: {}", s.context());
    assert!(s.monster(0, KOGARASHI).is_none() && s.count(0, "hand", KOGARASHI) == 1, "{}", s.context());
}

/// Blackwing - Oroshi the Squall: "If you control a "Blackwing" monster other than "Blackwing - Oroshi the Squall", you
/// can Special Summon this card (from your hand). You can only Special Summon "Blackwing - Oroshi the Squall" once per
/// turn this way." With Shura it is offered, and comes; with only Warwolf it is not offered.
#[test]
fn oroshi_needs_another_blackwing_to_special_summon() {
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &[sp(0, Monster, WARWOLF), sp(0, Hand, OROSHI)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons", OROSHI), "no other Blackwing: {}", s.context());
    let spawns = [sp(0, Monster, KRIS), sp(0, Hand, OROSHI), sp(0, Hand, OROSHI)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[SpSummon(OROSHI)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", OROSHI), 1, "{}", s.context());
    assert!(!offered(&s, "special_summons", OROSHI), "only once per turn: {}", s.context());
}

/// Oroshi: "If this card is sent to the GY as a Synchro Material: You can target 1 monster on the field; change that
/// target's battle position." Oroshi (Level 1 Tuner) and Dark Magician (7) make Stardust Dragon; Warwolf goes to Defense.
#[test]
fn oroshi_changes_a_position_when_used_for_a_synchro_summon() {
    let spawns = [sp(0, Monster, OROSHI), sp(0, Monster, DARK_MAGICIAN), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[STARDUST]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[SpSummon(STARDUST), Chain(OROSHI)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", STARDUST), 1, "{}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "Warwolf turned to Defense: {}", s.context());
}

const POTS: [u16; 40] = [POT_OF_GREED; 40];

/// Berserker Soul (printed): "When a monster you control inflicts 1500 or less damage to your opponent by a direct attack:
/// Discard your entire hand (min. 1); excavate the top card of your Deck, and if it is a Monster Card, send it to the GY,
/// and if you do that, inflict 500 damage to your opponent, then repeat this effect up to 7 more times or until you
/// excavate a non-Monster Card." A Deck of Monsters: the hand goes, 8 monsters are sent, 8 x 500 + the attack's 300.
#[test]
fn berserker_soul_mills_eight_monsters_for_500_each() {
    let spawns = [sp(0, Monster, KURIBOH), sp(0, SpellSet, BERSERKER_SOUL), sp(0, Hand, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&POTS, &[])], &spawns) else { return };
    s.run(&[Attack(KURIBOH), Chain(BERSERKER_SOUL)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), 0, "the entire hand was discarded: {}", s.context());
    assert_eq!(s.count(0, "grave", BERSERKER_SOUL), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 300 - 8 * 500, "{}", s.context());
}

/// Berserker Soul: "If a non-Monster Card is excavated, place it on the top of your Deck." With a Deck of spells the first
/// excavation stops the effect: no damage but the attack's, only the hand went to the Graveyard, the Deck is as it was.
#[test]
fn berserker_soul_stops_at_a_spell_and_leaves_it_on_the_deck() {
    let spawns = [sp(0, Monster, KURIBOH), sp(0, SpellSet, BERSERKER_SOUL)];
    let Some(mut s) = field(false, 8000, [(&[DARK_HOLE; 40], &[]), (&POTS, &[])], &spawns) else { return };
    let offered_soul = |p: &serde_json::Value| p["type"] == msg::SELECT_CHAIN && p["selects"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == BERSERKER_SOUL));
    s.run_to(&[Attack(KURIBOH)], offered_soul).unwrap();
    let (discarded, deck) = (hand(&s, 0), s.state()[0]["deck"].as_u64());
    assert!(discarded > 0, "{}", s.context());
    let i = s.prompt()["selects"].as_array().unwrap().iter().position(|c| c["cid"] == BERSERKER_SOUL).unwrap();
    s.respond(Response::Chain(Some(i)).json()).unwrap();
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BERSERKER_SOUL), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "no 500 damage: a spell was excavated: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_HOLE), discarded, "only the hand was discarded: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_u64(), deck, "the excavated spell is back on the Deck: {}", s.context());
}

/// Anime Berserker Soul: "Discard your entire hand (min. 1); draw 1 card, and if you do, show it, then, if it is a monster,
/// discard it to the Graveyard, and if you do that, you can choose a monster you control with 1500 or less ATK, it can
/// attack again in a row. ... Repeat this effect until you draw a non-monster." A Deck of Kuriboh with Pot of Greed among
/// them: every drawn Kuriboh is another 300 damage direct attack, the first Pot ends it and stays in the hand.
#[test]
fn anime_berserker_soul_draws_and_attacks_again_until_a_spell() {
    let spawns = [sp(0, Monster, KURIBOH), sp(0, SpellSet, BERSERKER_SOUL), sp(0, Hand, WARWOLF)];
    let mut deck: Vec<u16> = vec![KURIBOH; 35];
    deck.extend([POT_OF_GREED; 5]);
    let Some(mut s) = field(true, 8000, [(&deck, &[]), (&POTS, &[])], &spawns) else { return };
    let offered_soul = |p: &serde_json::Value| p["type"] == msg::SELECT_CHAIN && p["selects"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == BERSERKER_SOUL));
    s.run_to(&[Attack(KURIBOH)], offered_soul).unwrap();
    let (discarded, deck_before) = (hand(&s, 0), s.state()[0]["deck"].as_u64().unwrap());
    assert!(discarded > 0, "{}", s.context());
    let i = s.prompt()["selects"].as_array().unwrap().iter().position(|c| c["cid"] == BERSERKER_SOUL).unwrap();
    s.respond(Response::Chain(Some(i)).json()).unwrap();
    s.run(&[]).unwrap();
    while s.prompt()["type"] == msg::SELECT_BATTLECMD && s.prompt()["attacks"].as_array().is_some_and(|a| !a.is_empty()) {
        s.run(&[Attack(KURIBOH)]).unwrap();
    }
    clean(&s);
    let drawn = deck_before - s.state()[0]["deck"].as_u64().unwrap();
    assert!(drawn >= 2, "at least one Kuriboh and then the Pot: {}", s.context());
    assert_eq!((hand(&s, 0), s.count(0, "hand", POT_OF_GREED)), (1, 1), "the non-monster stays in the hand: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 300 * drawn as i64, "the first attack and one more for each drawn monster: {}", s.context());
    assert_eq!(s.count(0, "grave", BERSERKER_SOUL), 1, "{}", s.context());
}

const FLAME_SWORDSMAN: u16 = 4021; // a FIRE Warrior Fusion Monster, 1800 ATK
const GLASSJAW: u16 = 5017; // Command Knight: a Level 4 FIRE Warrior
const DARKFIRE_SOLDIER: u16 = 4973; // Level 4 Pyro
const MEKLORD_GRANEL: u16 = 9485;
const MEKLORD_SKIEL: u16 = 9484;
const LEFT_ARM: u16 = 4026;

/// Blue Flame Swordsman (printed): "Once per turn, during either player's Battle Phase: You can target 1 other Warrior-Type
/// monster you control; this card loses exactly 600 ATK, and if it does, that monster gains 600 ATK." Kuriboh attacks; in
/// answer Flame Swordsman goes from 1800 to 2400, the Swordsman from 1800 to 1200.
#[test]
fn blue_flame_swordsman_gives_600_attack_in_the_battle_phase() {
    let spawns = [sp(0, Monster, BLUE_FLAME), sp(0, Monster, FLAME_SWORDSMAN), sp(1, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&POTS, &[]), (&POTS, &[])], &spawns) else { return };
    s.prefer(&[(0, FLAME_SWORDSMAN)]);
    s.run_both([&[EndTurn, ChainOnAttack(BLUE_FLAME)], &[Attack(KURIBOH)]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, BLUE_FLAME), Some(1200), "{}", s.context());
    assert_eq!(s.attack_of(0, FLAME_SWORDSMAN), Some(2400), "{}", s.context());
}

/// Blue Flame Swordsman: "When this card you control is destroyed by your opponent's card (by battle or card effect) and
/// sent to your Graveyard: You can banish this card from your Graveyard, then target 1 FIRE Warrior-Type monster in your
/// Graveyard; Special Summon that target." Dark Hole destroys it; it is banished and Command Knight returns
/// (Flame Swordsman itself is a Fusion Monster, never properly Summoned there).
#[test]
fn blue_flame_swordsman_banishes_itself_to_revive_a_fire_warrior() {
    let spawns = [sp(0, Monster, BLUE_FLAME), sp(0, Grave, GLASSJAW), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&POTS, &[]), (&POTS, &[])], &spawns) else { return };
    s.prefer(&[(0, GLASSJAW)]);
    s.run_both([&[EndTurn, Chain(BLUE_FLAME)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", BLUE_FLAME), 1, "banished from the Graveyard: {}", s.context());
    assert!(s.monster(0, GLASSJAW).is_some(), "{}", s.context());
}

/// Anime Blue Flame Swordsman: "During either player's turn, you can have this card lose ATK in multiples of 100 to have
/// another face-up monster on the field gain an equal amount of ATK." In the Main Phase, 300 to Warwolf: 1500 and 2300.
#[test]
fn anime_blue_flame_swordsman_moves_any_multiple_of_100() {
    let spawns = [sp(0, Monster, BLUE_FLAME), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&POTS, &[]), (&POTS, &[])], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.declare_number(Some(300));
    s.run(&[Activate(BLUE_FLAME)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, BLUE_FLAME), s.attack_of(0, WARWOLF)), (Some(1500), Some(2300)), "{}", s.context());
}

/// Anime Blue Flame Swordsman: "When this card is destroyed: You can Special Summon 1 "Flame Swordsman" from your Deck or
/// Graveyard." Dark Hole destroys it; Flame Swordsman comes from the Deck.
#[test]
fn anime_blue_flame_swordsman_brings_flame_swordsman_when_destroyed() {
    let spawns = [sp(0, Monster, BLUE_FLAME), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(true, 8000, [(&with_copies(FLAME_SWORDSMAN), &[]), (&POTS, &[])], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(BLUE_FLAME)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BLUE_FLAME), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, FLAME_SWORDSMAN), Some(1800), "{}", s.context());
}

/// Bonfire: "Add 1 Level 4 or lower Pyro monster from your Deck to your hand. You can only activate 1 "Bonfire" per turn."
/// Darkfire Soldier #1 (Level 4 Pyro) is in the Deck; two Bonfires in hand, the second cannot be activated this turn.
#[test]
fn bonfire_adds_a_pyro_once_per_turn() {
    let spawns = [sp(0, Hand, BONFIRE), sp(0, Hand, BONFIRE)];
    let Some(mut s) = field(false, 8000, [(&with_copies(DARKFIRE_SOLDIER), &[]), (&POTS, &[])], &spawns) else { return };
    let before = s.count(0, "hand", DARKFIRE_SOLDIER);
    s.run(&[Activate(BONFIRE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", DARKFIRE_SOLDIER), before + 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", BONFIRE), 1, "{}", s.context());
    assert!(!offered(&s, "activates", BONFIRE), "the second Bonfire waits for the next turn: {}", s.context());
}

/// Bonfire needs a Level 4 or lower Pyro in the Deck: with only a Level 8 Dragon and spells it cannot be activated.
#[test]
fn bonfire_needs_a_pyro_in_the_deck() {
    let Some(mut s) = field(false, 8000, [(&with_copies(BLUE_EYES), &[]), (&POTS, &[])], &[sp(0, Hand, BONFIRE)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates", BONFIRE), "{}", s.context());
}

/// Boon of the Meklord Emperor (printed): "If the only face-up monsters on the field are 2 "Meklord" monsters: Draw 2
/// cards. You cannot conduct your Battle Phase the turn you activate this card." Granel and Skiel alone: 2 cards, no
/// Battle Phase.
#[test]
fn boon_of_the_meklord_emperor_draws_two_and_forbids_the_battle_phase() {
    let spawns = [sp(0, Monster, MEKLORD_GRANEL), sp(0, Monster, MEKLORD_SKIEL), sp(0, Hand, BOON)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the Spell
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap(); // the first turn has no Battle Phase
    let before = hand(&s, 0);
    assert_eq!(s.prompt()["to_bp"], true, "{}", s.context());
    s.run(&[Activate(BOON)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before + 1, "-1 for the Spell, +2 drawn: {}", s.context());
    assert_eq!(s.prompt()["to_bp"], false, "no Battle Phase this turn: {}", s.context());
}

/// Boon: a third monster on the field (even the opponent's) means it cannot be activated.
#[test]
fn boon_of_the_meklord_emperor_needs_only_two_meklords_on_the_field() {
    let spawns = [sp(0, Monster, MEKLORD_GRANEL), sp(0, Monster, MEKLORD_SKIEL), sp(0, Hand, BOON), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates", BOON), "{}", s.context());
}

/// Anime Boon of the Meklord Emperor: "Draw 1 card for each face-up "Meklord" monster you control." With Warwolf beside two
/// Meklords it still draws 2 (the printed card would not be activatable), and the Battle Phase stays open.
#[test]
fn anime_boon_draws_one_card_per_meklord() {
    let spawns = [sp(0, Monster, MEKLORD_GRANEL), sp(0, Monster, MEKLORD_SKIEL), sp(0, Monster, WARWOLF), sp(0, Hand, BOON)];
    let Some(mut s) = field(true, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let before = hand(&s, 0);
    s.run(&[Activate(BOON)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before + 1, "-1 for the Spell, +2 drawn: {}", s.context());
    assert_eq!(s.prompt()["to_bp"], true, "{}", s.context());
}

/// Break the Seal: "You can activate 1 of these effects; ● Place 1 "Break the Seal" from your Deck or GY, face-up on your
/// field." Its quick effect from the field places the one from the Graveyard: two face-up on the field.
#[test]
fn break_the_seal_places_another_from_the_graveyard() {
    let spawns = [sp(0, Spell, BREAK_THE_SEAL), sp(0, Grave, BREAK_THE_SEAL)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[Activate(BREAK_THE_SEAL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", BREAK_THE_SEAL), 2, "{}", s.context());
}

/// Break the Seal: "● Send 2 face-up "Break the Seal" you control to the GY, including this card; add 1 "Forbidden One"
/// monster from your Deck to your hand." Two on the field: both go, a Left Arm of the Forbidden One is added.
#[test]
fn break_the_seal_sends_two_to_add_a_forbidden_one() {
    let spawns = [sp(0, Spell, BREAK_THE_SEAL), sp(0, Spell, BREAK_THE_SEAL)];
    let Some(mut s) = field(false, 8000, [(&with_copies(LEFT_ARM), &[]), (&KURIBOHS, &[])], &spawns) else { return };
    let before = s.count(0, "hand", LEFT_ARM);
    s.run(&[Activate(BREAK_THE_SEAL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BREAK_THE_SEAL), 2, "{}", s.context());
    assert_eq!(s.count(0, "hand", LEFT_ARM), before + 1, "{}", s.context());
}

/// Break the Seal: "If this card on the field is destroyed: You can reveal up to 5 "Forbidden One" monsters in your hand;
/// return that many cards your opponent controls to the hand." Mystical Space Typhoon destroys it; one revealed Left Arm
/// returns the opponent's Warwolf to their hand.
#[test]
fn break_the_seal_destroyed_returns_opponents_cards_for_revealed_forbidden_ones() {
    let spawns = [sp(0, Spell, BREAK_THE_SEAL), sp(0, Hand, LEFT_ARM), sp(1, Hand, MST), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps Left Arm
    s.run_both([&[EndTurn, Chain(BREAK_THE_SEAL)], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BREAK_THE_SEAL), 1, "{}", s.context());
    assert!(s.monster(1, WARWOLF).is_none() && s.count(1, "hand", WARWOLF) == 1, "Warwolf went back to the hand: {}", s.context());
    assert_eq!(s.count(0, "hand", LEFT_ARM), 1, "the revealed card stays in the hand: {}", s.context());
}

/// Bright Star Dragon (printed): "When this card is Normal Summoned: You can target 1 other face-up monster on the field;
/// increase that target's Level by 2 until the end of this turn." Warwolf goes from Level 4 to 6.
#[test]
fn bright_star_dragon_raises_a_level_by_two() {
    let spawns = [sp(0, Hand, BRIGHT_STAR), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Summon(BRIGHT_STAR), Chain(BRIGHT_STAR)]).unwrap();
    clean(&s);
    assert_eq!(s.level_of(0, WARWOLF), Some(6), "{}", s.context());
}

/// Anime Bright Star Dragon: "When this card is Special Summoned: You can target 1 other monster you control; increase the
/// Level of that target by 1 or 2." Monster Reborn brings it back from the Graveyard; Warwolf rises by the 2 declared.
#[test]
fn anime_bright_star_dragon_raises_a_level_when_special_summoned() {
    let spawns = [sp(0, Grave, BRIGHT_STAR), sp(0, Hand, MONSTER_REBORN), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, BRIGHT_STAR), (0, WARWOLF)]);
    s.declare_number(Some(2));
    s.run(&[Activate(MONSTER_REBORN), Chain(BRIGHT_STAR)]).unwrap();
    clean(&s);
    assert!(s.monster(0, BRIGHT_STAR).is_some(), "{}", s.context());
    assert_eq!(s.level_of(0, WARWOLF), Some(6), "{}", s.context());
}

const GUARDIAN_EATOS: u16 = 8536;
const LUSTER_DRAGON: u16 = 5644; // Level 4 Dragon Normal Monster
const MIRROR_FORCE: u16 = 4887;
const CARD_DESTRUCTION: u16 = 4821;
const CREATION_RESONATOR: u16 = 9090;
const URIA: u16 = 6563;
const HAMON: u16 = 6564;
const RAVIEL: u16 = 6565;
const MACRO_COSMOS: u16 = 6682;
const FORTUNE_LADY_LIGHT: u16 = 8301; // LIGHT, ? ATK
const FIEND_COMEDIAN: u16 = 5405; // a coin tossing card
const CLEAR_PHANTOM: u16 = 5056; // mentions "Clear World"

/// Buster Blaster: "Once per turn, you can either: Target 1 face-up monster you control; equip this card to that target ...
/// While equipped by this effect, the equipped monster loses 500 ATK and DEF." Warwolf goes from 2000 to 1500.
#[test]
fn buster_blaster_equips_and_weakens_the_monster() {
    let spawns = [sp(0, Monster, BUSTER_BLASTER), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(BUSTER_BLASTER)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(1500), "{}", s.context());
    assert_eq!((s.count(0, "szone", BUSTER_BLASTER), s.count(0, "mzone", BUSTER_BLASTER)), (1, 0), "{}", s.context());
}

/// Buster Blaster: "OR: Unequip this card and Special Summon it in face-up Attack Position." The next turn it leaves
/// Warwolf, which is back to 2000, and stands on the field.
#[test]
fn buster_blaster_unequips_and_special_summons_itself() {
    let spawns = [sp(0, Monster, BUSTER_BLASTER), sp(0, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, KURIBOH)]);
    s.run(&[Activate(BUSTER_BLASTER), EndTurn, Activate(BUSTER_BLASTER)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "{}", s.context());
    assert!(s.monster(0, BUSTER_BLASTER).is_some(), "{}", s.context());
}

/// Buster Blaster: "If the equipped monster destroys an opponent's monster by battle: Destroy all face-up monsters on the
/// field with the same Monster Type as the destroyed monster." Warwolf (1500 equipped) destroys a Fiend Kuriboh; the other
/// Fiend Kuriboh, on the opponent's side too, goes with it, and Warwolf (Beast-Warrior) stays.
#[test]
fn buster_blaster_destroys_every_monster_of_the_type_destroyed() {
    let spawns = [sp(0, Monster, BUSTER_BLASTER), sp(0, Monster, WARWOLF), sp(1, Monster, KURIBOH), sp(1, Monster, KURIBOH)];
    let Some(mut s) = field(false, 8000, [(&POTS, &[]), (&POTS, &[])], &spawns) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Activate(BUSTER_BLASTER), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", KURIBOH), 2, "both Kuriboh are gone: {}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "{}", s.context());
}

/// Cabrera Stone: "If this card is Summoned: You can add 1 "Triangle O" from your Deck to your hand, then if this card is in
/// Attack Position, change it to Defense Position."
#[test]
fn cabrera_stone_adds_triangle_o_and_turns_to_defense() {
    let Some(mut s) = field(false, 8000, [(&with_copies(TRIANGLE_O), &[]), (&POTS, &[])], &[sp(0, Hand, CABRERA)]) else { return };
    let before = s.count(0, "hand", TRIANGLE_O);
    s.run(&[Summon(CABRERA), Chain(CABRERA)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", TRIANGLE_O), before + 1, "{}", s.context());
    assert_eq!(s.position_of(0, CABRERA), Some(4), "{}", s.context());
}

/// Cabrera Stone: "If this card is destroyed by battle or card effect: Take 1000 damage." Dark Hole.
#[test]
fn cabrera_stone_destroyed_costs_1000() {
    let spawns = [sp(0, Monster, CABRERA), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&POTS, &[]), (&POTS, &[])], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CABRERA), 1, "{}", s.context());
    assert_eq!(s.lp(0), 7000, "{}", s.context());
}

/// Carboneddon: "If this card battles a FIRE monster, during damage calculation: This card gains 1000 ATK during damage
/// calculation only." Its 800 ATK becomes 1800 against Darkfire Soldier #1 (FIRE, 1700): the Soldier is destroyed, and
/// Carboneddon survives.
#[test]
fn carboneddon_beats_a_fire_monster_in_battle() {
    let spawns = [sp(0, Monster, CARBONEDDON), sp(1, Monster, DARKFIRE_SOLDIER)];
    let Some(mut s) = field(false, 8000, [(&POTS, &[]), (&POTS, &[])], &spawns) else { return };
    s.run(&[Attack(CARBONEDDON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARKFIRE_SOLDIER), 1, "{}", s.context());
    assert!(s.monster(0, CARBONEDDON).is_some(), "{}", s.context());
    assert_eq!(s.lp(1), 7900, "{}", s.context());
}

/// Carboneddon: "You can banish this card from your GY; Special Summon, from your hand or Deck, 1 Level 7 or lower Dragon
/// Normal Monster in face-up Defense Position." Luster Dragon comes; Blue-Eyes (Level 8) in the hand does not qualify.
#[test]
fn carboneddon_banishes_itself_for_a_dragon_in_defense() {
    let spawns = [sp(0, Grave, CARBONEDDON), sp(0, Hand, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, [(&with_copies(LUSTER_DRAGON), &[]), (&POTS, &[])], &spawns) else { return };
    s.run(&[Activate(CARBONEDDON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", CARBONEDDON), 1, "{}", s.context());
    assert_eq!(s.position_of(0, LUSTER_DRAGON), Some(4), "{}", s.context());
    assert!(s.monster(0, BLUE_EYES).is_none(), "{}", s.context());
}

/// Card of Demise (printed): "Draw until you have 3 cards in your hand, also for the rest of this turn after this card
/// resolves, your opponent takes no damage. During the End Phase of this turn, send your entire hand to the GY." Four
/// Spells are Set first to bring the hand to 3 with the Card (2 after it is played): 1 drawn; Kuriboh's attack does no
/// damage; the End Phase empties the hand.
#[test]
fn card_of_demise_draws_to_three_protects_and_sends_the_hand() {
    let spawns = [sp(0, Monster, KURIBOH), sp(0, Hand, DEMISE)];
    let Some(mut s) = field(false, 8000, [(&[DARK_HOLE; 40], &[]), (&POTS, &[])], &spawns) else { return };
    s.prefer(&[(0, DARK_HOLE)]);
    s.run(&[EndTurn, SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), Activate(DEMISE), Attack(KURIBOH)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), 3, "{}", s.context());
    assert_eq!(s.lp(1), 8000, "the opponent takes no damage: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), 0, "the entire hand went to the Graveyard: {}", s.context());
}

/// Anime Card of Demise: "Draw cards until you have 5 cards in your hand. During your 5th Standby Phase after activation,
/// send all cards in your hand to the Graveyard." Three Spells Set first (it needs a hand of 4 with itself): the hand is 5 after it, and still 5 after
/// the End Phase (the printed card would have emptied it).
#[test]
fn anime_card_of_demise_draws_to_five_and_keeps_the_hand() {
    let Some(mut s) = field(true, 8000, [(&[DARK_HOLE; 40], &[]), (&POTS, &[])], &[sp(0, Hand, DEMISE)]) else { return };
    s.prefer(&[(0, DARK_HOLE)]);
    s.run(&[EndTurn, SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), SetSpell(DARK_HOLE), Activate(DEMISE)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), 5, "{}", s.context());
    s.run(&[EndTurn]).unwrap();
    assert_eq!(hand(&s, 0), 5, "{}", s.context());
}

/// Celestial Sword - Eatos: "The equipped monster gains 500 ATK. When this card is sent from the field to the GY: You can
/// target 1 "Guardian Eatos" you control; it gains 500 ATK for each banished monster." Equipped to Guardian Eatos (2500 to
/// 3000); Mystical Space Typhoon destroys it with two monsters banished: 2500 + 2 x 500.
#[test]
fn celestial_sword_eatos_equips_and_pays_back_for_banished_monsters() {
    let spawns = [sp(0, Monster, GUARDIAN_EATOS), sp(0, Hand, EATOS_SWORD), sp(0, Banished, KURIBOH), sp(0, Banished, KURIBOH),
        sp(1, Hand, MST)];
    let Some(mut s) = field(false, 8000, [(&POTS, &[]), (&POTS, &[])], &spawns) else { return };
    s.run(&[Activate(EATOS_SWORD)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GUARDIAN_EATOS), Some(3000), "{}", s.context());
    s.run_both([&[EndTurn, Chain(EATOS_SWORD)], &[Activate(MST)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", EATOS_SWORD), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, GUARDIAN_EATOS), Some(3500), "{}", s.context());
}

/// Centaur Mina: "During your Main Phase: You can Special Summon 1 Level 2 or lower monster from your hand or GY."
#[test]
fn centaur_mina_special_summons_a_low_level_monster() {
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[MINA]), (&KURIBOHS, &[])], &[sp(0, Monster, MINA)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[Activate(MINA)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", KURIBOH), 1, "{}", s.context());
}

/// Centaur Mina: "Once per turn, during your turn, when your opponent activates a Trap Card (Quick Effect): You can negate
/// the activation, and if you do, Set that card." Warwolf attacks; Mirror Force is negated and Set again: it
/// destroys nothing, and the attack goes through.
#[test]
fn centaur_mina_negates_a_trap_and_sets_it_again() {
    let spawns = [sp(0, Monster, MINA), sp(0, Monster, WARWOLF), sp(1, SpellSet, MIRROR_FORCE)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[MINA]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run_both([&[Attack(WARWOLF), Chain(MINA)], &[ChainOnAttack(MIRROR_FORCE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "szone", MIRROR_FORCE), 1, "Mirror Force is Set again: {}", s.context());
    assert!(s.monster(0, WARWOLF).is_some() && s.monster(0, MINA).is_some(), "{}", s.context());
    assert_eq!(s.lp(1), 6000, "{}", s.context());
}

/// Ceruli, Guru of Dark World: "If this card is discarded to the GY by card effect: Special Summon this card to your
/// opponent's field in Defense Position." Card Destruction (each player discards their hand) makes team 0 discard it.
#[test]
fn ceruli_discarded_by_an_effect_goes_to_the_opponents_field() {
    let spawns = [sp(0, Hand, CERULI), sp(1, Hand, CARD_DESTRUCTION)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, CERULI)]);
    s.run_both([&[SetMonster(KURIBOH), EndTurn], &[Activate(CARD_DESTRUCTION)]]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, CERULI), Some(4), "{}", s.context());
    assert_eq!(s.count(0, "grave", CERULI), 0, "{}", s.context());
}

/// Chain Resonator: "When this card is Normal Summoned while a Synchro Monster is on the field: You can Special Summon 1
/// "Resonator" monster from your Deck, except "Chain Resonator"." Stardust Dragon is on the field.
#[test]
fn chain_resonator_brings_a_resonator_when_a_synchro_monster_is_on_the_field() {
    let spawns = [sp(0, Hand, CHAIN_RESONATOR), sp(1, Monster, STARDUST)];
    let Some(mut s) = field(false, 8000, [(&with_copies(CREATION_RESONATOR), &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[Summon(CHAIN_RESONATOR), Chain(CHAIN_RESONATOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", CREATION_RESONATOR), 1, "{}", s.context());
}

/// Chain Resonator without a Synchro Monster on the field does nothing.
#[test]
fn chain_resonator_needs_a_synchro_monster() {
    let Some(mut s) = field(false, 8000, [(&with_copies(CREATION_RESONATOR), &[]), (&KURIBOHS, &[])], &[sp(0, Hand, CHAIN_RESONATOR)]) else { return };
    s.run(&[Summon(CHAIN_RESONATOR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", CREATION_RESONATOR), 0, "{}", s.context());
}

/// Chaos Core: "When ... your opponent's monster targets it for an attack (Quick Effect): You can send up to 1 each of
/// "Uria", "Hamon", and "Raviel" from your hand and/or Deck to the GY, and if you do, place 1 Phantasm Counter on this card
/// for each card sent to the GY, and if you place at least 1, you take no battle damage this turn. ... If this card would be
/// destroyed by battle or card effect, you can remove 1 Phantasm Counter from this card instead." Warwolf attacks Chaos
/// Core (0 ATK): a Lord is sent, there is no damage, and the Core stays.
#[test]
fn chaos_core_sends_a_lord_and_takes_no_damage() {
    let deck: Vec<u16> = [URIA; 8].into_iter().chain([HAMON; 8]).chain([RAVIEL; 8]).chain([KURIBOH; 16]).collect();
    let spawns = [sp(0, Monster, CHAOS_CORE), sp(1, Monster, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&deck, &[]), (&POTS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]); // the End Phase discard keeps the Lords
    s.run_both([&[EndTurn], &[]]).unwrap();
    let lords = |s: &Scenario| [URIA, HAMON, RAVIEL].iter().map(|&c| s.count(0, "grave", c)).sum::<usize>();
    let before = lords(&s);
    s.run_both([&[ChainOnAttack(CHAOS_CORE)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(lords(&s) > before, "a Lord was sent to the Graveyard: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert!(s.monster(0, CHAOS_CORE).is_some(), "a counter was removed instead of the Core's destruction: {}", s.context());
}

/// Chaos Distill: "While this card is face-up on the field, any card sent to your GY is banished instead." Dark Hole
/// destroys Warwolf: it and Dark Hole itself are banished.
#[test]
fn chaos_distill_banishes_cards_sent_to_the_graveyard() {
    let spawns = [sp(0, Spell, CHAOS_DISTILL), sp(0, Monster, WARWOLF), sp(0, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", WARWOLF), s.count(0, "grave", WARWOLF)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "removed", DARK_HOLE), 1, "{}", s.context());
}

/// Chaos Distill: "● Add 1 "Macro Cosmos", or 1 card that mentions it, from your Deck to your hand, except "Chaos Distill"."
#[test]
fn chaos_distill_adds_macro_cosmos() {
    let Some(mut s) = field(false, 8000, [(&with_copies(MACRO_COSMOS), &[]), (&KURIBOHS, &[])], &[sp(0, Spell, CHAOS_DISTILL)]) else { return };
    let before = s.count(0, "hand", MACRO_COSMOS);
    s.run(&[Activate(CHAOS_DISTILL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", MACRO_COSMOS), before + 1, "{}", s.context());
}

/// Chaos Distill: "● If "Macro Cosmos" is on the field: Banish this face-up card from the Spell & Trap Zone; add 1 LIGHT
/// monster with ? ATK from your Deck to your hand." Fortune Lady Light is the one in the Deck.
#[test]
fn chaos_distill_banishes_itself_for_a_light_monster_with_unknown_attack() {
    let spawns = [sp(0, Spell, CHAOS_DISTILL), sp(0, Spell, MACRO_COSMOS)];
    let Some(mut s) = field(false, 8000, [(&with_copies(FORTUNE_LADY_LIGHT), &[]), (&KURIBOHS, &[])], &spawns) else { return };
    let before = s.count(0, "hand", FORTUNE_LADY_LIGHT);
    s.run(&[Activate(CHAOS_DISTILL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", CHAOS_DISTILL), 1, "{}", s.context());
    assert_eq!(s.count(0, "hand", FORTUNE_LADY_LIGHT), before + 1, "{}", s.context());
}

/// Chaos Infinity (printed): "Change as many Defense Position monsters on the field as possible to face-up Attack
/// Position, then Special Summon 1 "Meklord" monster from your Deck or GY, but its effects are negated, also it is
/// destroyed during the End Phase." Warwolf (Defense, the opponent's) turns to Attack; Granel comes from the Graveyard and
/// is destroyed at the End Phase.
#[test]
fn chaos_infinity_turns_defenders_up_and_summons_a_short_lived_meklord() {
    let spawns = [sp(0, SpellSet, CHAOS_INFINITY), sp(0, Grave, MEKLORD_GRANEL), sp(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[Activate(CHAOS_INFINITY)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, WARWOLF), Some(1), "{}", s.context());
    assert!(s.monster(0, MEKLORD_GRANEL).is_some(), "{}", s.context());
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", MEKLORD_GRANEL), 1, "destroyed during the End Phase: {}", s.context());
}

/// Anime Chaos Infinity: "Special Summon 2 "Meklord" monsters, 1 from your Deck and 1 from your Graveyard." Granel from
/// the Graveyard, Skiel from the Deck.
#[test]
fn anime_chaos_infinity_summons_two_meklords() {
    let spawns = [sp(0, SpellSet, CHAOS_INFINITY), sp(0, Grave, MEKLORD_GRANEL), sp(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = field(true, 8000, [(&with_copies(MEKLORD_SKIEL), &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[Activate(CHAOS_INFINITY)]).unwrap();
    clean(&s);
    assert!(s.monster(0, MEKLORD_GRANEL).is_some() && s.monster(0, MEKLORD_SKIEL).is_some(), "{}", s.context());
}

/// Candelato (printed): "Gains 1000 ATK/DEF for each card in your hand."
#[test]
fn candelato_gains_1000_per_card_in_hand() {
    let Some(s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &[sp(0, Monster, CANDELATO)]) else { return };
    assert_eq!(s.attack_of(0, CANDELATO), Some(1000 * hand(&s, 0) as i64), "{}", s.context());
}

/// Candelato: "You can discard 2 other cards; Special Summon this card from your hand, then you can add 1 card from your
/// Deck to your hand that has a coin tossing effect." Fiend Comedian is the card.
#[test]
fn candelato_special_summons_itself_and_searches_a_coin_card() {
    let Some(mut s) = field(false, 8000, [(&with_copies(FIEND_COMEDIAN), &[]), (&KURIBOHS, &[])], &[sp(0, Hand, CANDELATO)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    let before = s.count(0, "hand", FIEND_COMEDIAN);
    s.run(&[Activate(CANDELATO)]).unwrap();
    clean(&s);
    assert!(s.monster(0, CANDELATO).is_some(), "{}", s.context());
    assert_eq!(total(&s, 0, "grave"), 2, "two cards were discarded: {}", s.context());
    assert_eq!(s.count(0, "hand", FIEND_COMEDIAN), before + 1, "{}", s.context());
}

/// Candelato: "When a card or effect is activated that includes an effect that adds a card(s) from the Deck to the hand
/// (Quick Effect): You can negate that effect." Bonfire's search is negated.
#[test]
fn candelato_negates_a_search() {
    let spawns = [sp(0, Monster, CANDELATO), sp(1, Hand, BONFIRE)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&with_copies(DARKFIRE_SOLDIER), &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[EndTurn]).unwrap();
    let before = s.count(1, "hand", DARKFIRE_SOLDIER);
    activate_now(&mut s, BONFIRE);
    assert!(chain_in_answer(&mut s, 0, CANDELATO), "the negation is offered: {}", s.context());
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BONFIRE), 1, "{}", s.context());
    assert_eq!(s.count(1, "hand", DARKFIRE_SOLDIER), before, "nothing was added: {}", s.context());
}

/// Anime Candelato: "Must be Special Summoned (from your hand) by sending 2 cards from your hand to the Graveyard ... Gains
/// 1000 ATK for each card in your hand. While you control this face-up card, you must skip your Draw Phase." The next own
/// turn draws nothing.
#[test]
fn anime_candelato_skips_the_draw_phase() {
    let Some(mut s) = field(true, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &[sp(0, Hand, CANDELATO)]) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run(&[SpSummon(CANDELATO)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, CANDELATO), Some(1000 * hand(&s, 0) as i64), "{}", s.context());
    let before = hand(&s, 0);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(hand(&s, 0), before, "no draw this turn: {}", s.context());
}

/// Clear Cube: "During your Main Phase, you can Normal Summon 1 monster that mentions "Clear World", in addition to your
/// Normal Summon/Set." Warwolf, then Clear Phantom as well; without the Cube the Phantom is not offered after it.
#[test]
fn clear_cube_gives_an_extra_normal_summon_for_clear_world_monsters() {
    let spawns = [sp(0, Monster, CLEAR_CUBE), sp(0, Hand, CLEAR_PHANTOM), sp(0, Hand, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[Summon(WARWOLF)]).unwrap();
    assert!(offered(&s, "summons", CLEAR_PHANTOM), "{}", s.context());
    s.run(&[Summon(CLEAR_PHANTOM)]).unwrap();
    clean(&s);
    assert!(s.monster(0, WARWOLF).is_some() && s.monster(0, CLEAR_PHANTOM).is_some(), "{}", s.context());
    let spawns = [sp(0, Hand, CLEAR_PHANTOM), sp(0, Hand, WARWOLF)];
    let Some(mut s) = field(false, 8000, [(&KURIBOHS, &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.run(&[Summon(WARWOLF)]).unwrap();
    assert!(!offered(&s, "summons", CLEAR_PHANTOM), "one Normal Summon only: {}", s.context());
}

/// Clear Cube: "If this face-up card in its owner's control leaves the field by an opponent's card: You can Special Summon
/// 1 monster that mentions "Clear World" from your Deck." Dark Hole destroys it; Clear Phantom comes from the Deck.
#[test]
fn clear_cube_destroyed_brings_a_clear_world_monster() {
    let spawns = [sp(0, Monster, CLEAR_CUBE), sp(1, Hand, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, [(&with_copies(CLEAR_PHANTOM), &[]), (&KURIBOHS, &[])], &spawns) else { return };
    s.prefer(&[(0, KURIBOH)]);
    s.run_both([&[EndTurn, Chain(CLEAR_CUBE)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", CLEAR_CUBE), 1, "{}", s.context());
    assert!(s.monster(0, CLEAR_PHANTOM).is_some(), "{}", s.context());
}
