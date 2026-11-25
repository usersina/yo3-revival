//! Printed anime cards, group 9 (rows 4848-5205 of engine/added/cards.tsv): Legion the Fiend Jester to the Meklord Army
//! monsters. Each effect through the real core with both seats' client models checked after every step
//! (`duelcheck::scenario`); the printed text with anime cards off, the anime version where it differs, with them on.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The cards of this group (client ids).
const LEGION: u16 = 4745; // Legion the Fiend Jester
const LIFE_SHAVER: u16 = 5554;
const LIFE_STREAM: u16 = 9509; // Life Stream Dragon
const LIGHTNING_CRASH: u16 = 5117;
const LITMUS_RITUAL: u16 = 13812; // Litmus Doom Ritual
const LITMUS: u16 = 13811; // Litmus Doom Swordsman
const FOSSIL: u16 = 13813; // Living Fossil
const LOGE: u16 = 14541; // Loge's Flame
const LORD_OF_THE_RED: u16 = 11894;
const LUCKY_LOAN: u16 = 14558;
const MACHINE_ANGEL_RITUAL: u16 = 6850;
const LULLABY: u16 = 5551; // Lullaby of Obedience
const TRICK_MIRROR: u16 = 5577; // Magical Trick Mirror
const MAGNET_FORCE: u16 = 12569;
const MAGNET_REVERSE: u16 = 12568;
const SIGMA_MINUS: u16 = 5089; // Magnet Warrior Sigma Minus (Level 4 EARTH Rock)
const SIGMA_PLUS: u16 = 5087; // Magnet Warrior Sigma Plus (Level 4 EARTH Rock)
const MAIDEN_IN_LOVE: u16 = 5076;
const DES_GARDIUS: u16 = 5097; // Masked Beast Des Gardius
const MATADOR: u16 = 5080; // Matador Archfiend (stand-in for 21753)
const MATADOR_RITUAL: u16 = 5081; // Ritual of the Matador (stand-in for 21755)
const MATHEMATICIAN: u16 = 11168;
const GRANEL: u16 = 9485; // Meklord Army of Granel, 1600 ATK
const SKIEL: u16 = 9484; // Meklord Army of Skiel, 1200 ATK
const WISEL: u16 = 9483; // Meklord Army of Wisel, 1800 ATK
// Added cards the tests play with.
const DRITTE: u16 = 13992; // Valkyrie Dritte, Level 4
const ZWEITE: u16 = 13993; // Valkyrie Zweite, Level 5
const BRUNHILDE: u16 = 13995; // Valkyrie Brunhilde, Level 7
const TRANSMIGRATION: u16 = 11895; // Red-Eyes Transmigration
const FAIRY_HIKARI: u16 = 14550; // Fortune Fairy Hikari, Level 1
const FAIRY_EN: u16 = 14551; // Fortune Fairy En, Level 2
const BENTEN: u16 = 6845; // Cyber Angel Benten, Level 6
const MASK_OF_REMNANTS: u16 = 5098; // The Mask of Remnants
// The game's own cards.
const MAIDEN_OF_THE_MOONLIGHT: u16 = 4637; // Level 4 LIGHT Spellcaster
const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster Normal Monster
const ZABORG: u16 = 5965; // Level 5 Thunder
const DARK_HOLE: u16 = 4342;
const MONSTER_REBORN: u16 = 4842;
const MST: u16 = 4909; // Mystical Space Typhoon
const TRAP_HOLE: u16 = 4836;
const THUNDER_KID: u16 = 4217; // Level 2 Thunder Normal Monster
const POWER_TOOL: u16 = 8137; // Power Tool Dragon
const TURBO_SYNCHRON: u16 = 7870; // a Level 1 Tuner
const AXE_OF_DESPAIR: u16 = 4310; // an Equip Spell
const THUNDER_CRASH: u16 = 5847; // destroy your monsters, 300 damage each to your opponent
const BLUE_EYES: u16 = 4007;
const HEAVY_STORM: u16 = 4891;
const POT_OF_GREED: u16 = 4844;
const RED_EYES: u16 = 4088; // Red-Eyes Black Dragon, Level 7
const RAIGEKI: u16 = 4343;
const GRAVITY_BIND: u16 = 5134; // a Continuous Trap
const OBELISK: u16 = 4998;
const BARREL_ROCK: u16 = 4461; // Level 4 Rock
const BETA: u16 = 4763; // Beta The Magnet Warrior, Level 4 EARTH Rock, 1700 ATK
const ALPHA: u16 = 4744; // Alpha The Magnet Warrior, Level 4 Rock
const VALKYRION: u16 = 5002; // Valkyrion the Magna Warrior, Level 8
const GRAND_TIKI: u16 = 5093; // Grand Tiki Elder
const ARCHFIEND_SOLDIER: u16 = 5762; // an "Archfiend" monster
const ARCHFIEND_MARMOT: u16 = 4208; // another

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// Both players with Kuriboh decks (team 0's has `main` below them, out of the opening hand, and `extra` as its Extra Deck).
fn field(anime: bool, lp: u32, main: &[u16], extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = [KURIBOH; 36].into_iter().chain(main.iter().copied()).collect();
    let decks = [(&mine[..], extra), (&[KURIBOH; 40][..], &[][..])];
    Some(
        if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }
            .unwrap(),
    )
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// Team 0 Normal Summons `summon` and `team` chains `cid` in the first chain window after it where `ready` holds (a
/// `Chain` action takes the first window it is offered in, which comes earlier: at the start of the turn).
fn summon_and_chain(s: &mut Scenario, summon: u16, team: usize, cid: u16, ready: impl Fn(&Scenario) -> bool) {
    use yo3_server::ocg::{Response, idle, msg};
    let listed = |p: &serde_json::Value, list: &str, cid: u16| p[list].as_array().and_then(|v| v.iter().position(|c| c["cid"] == cid));
    s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD && listed(p, "summons", summon).is_some()).unwrap();
    let index = listed(s.prompt(), "summons", summon).unwrap();
    s.respond(serde_json::json!({"type": 1, "action": idle::SUMMON, "index": index})).unwrap();
    loop {
        s.run_to(&[], |p| p["type"] == msg::SELECT_CHAIN && p["player"] == team && listed(p, "selects", cid).is_some()).unwrap();
        if ready(s) {
            let index = listed(s.prompt(), "selects", cid).unwrap();
            s.respond(Response::Chain(Some(index)).json()).unwrap();
            return;
        }
        s.pass().unwrap();
    }
}

/// The Konami ids the planned side can Normal Summon (or Tribute Summon) at the prompt it stopped at.
fn summonable(s: &Scenario, cid: u16) -> bool {
    s.prompt()["summons"].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
}

/// Legion the Fiend Jester: "During your Main Phase, you can Tribute Summon 1 Spellcaster monster in Attack Position, in
/// addition to your Normal Summon/Set." and "If this card is sent from the field to the GY: You can add 1 Spellcaster
/// Normal Monster from your Deck or GY to your hand." Warwolf is Normal Summoned, Dark Magician is then Tribute Summoned
/// on Legion and Warwolf; Legion in the Graveyard fetches the second Dark Magician from the Deck.
#[test]
fn legion_gives_a_second_tribute_summon_and_searches_when_tributed() {
    let spawns = [mon(0, LEGION), hand(0, WARWOLF), hand(0, DARK_MAGICIAN)];
    let Some(mut s) = field(false, 8000, &[DARK_MAGICIAN], &[], &spawns) else { return };
    s.prefer(&[(0, DARK_MAGICIAN)]);
    s.run(&[Summon(WARWOLF), Summon(DARK_MAGICIAN), Chain(LEGION)]).unwrap();
    clean(&s);
    assert!(s.monster(0, DARK_MAGICIAN).is_some(), "Dark Magician was Tribute Summoned: {}", s.context());
    assert_eq!(s.count(0, "grave", LEGION), 1, "Legion was Tributed: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "so was Warwolf: {}", s.context());
    assert_eq!(s.count(0, "hand", DARK_MAGICIAN), 1, "the other Dark Magician came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0);
}

/// Legion's extra Tribute Summon is for a Spellcaster only: after the Normal Summon, Zaborg (Thunder, Level 5) cannot be
/// Tribute Summoned, Dark Magician can.
#[test]
fn legion_extra_tribute_summon_is_for_spellcasters_only() {
    let spawns = [mon(0, LEGION), mon(0, KURIBOH), hand(0, WARWOLF), hand(0, DARK_MAGICIAN), hand(0, ZABORG)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Summon(WARWOLF)]).unwrap();
    clean(&s);
    assert!(summonable(&s, DARK_MAGICIAN), "Dark Magician: {}", s.context());
    assert!(!summonable(&s, ZABORG), "Zaborg is no Spellcaster: {}", s.context());
}

/// Life Shaver: "Once per turn, during your opponent's End Phase: Place 1 counter on this card. During the Main or Battle
/// Phase, if this card has a counter(s) on it: You can send this card to the GY, then make your opponent discard as many
/// cards as possible, up to the number of counters that were on this card." One opponent's End Phase, one counter, one card.
#[test]
fn life_shaver_discards_one_card_per_counter() {
    let Some(mut s) = field(false, 8000, &[], &[], &[spell(0, LIFE_SHAVER)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    let before = hand_size(&s, 1);
    s.run(&[Activate(LIFE_SHAVER)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LIFE_SHAVER), 1, "Life Shaver went to the Graveyard: {}", s.context());
    assert_eq!(hand_size(&s, 1), before - 1, "one counter, one discard: {}", s.context());
    assert_eq!(s.count(1, "grave", KURIBOH), 1);
}

/// Life Shaver cannot be used before it has a counter: with none on it, it is not on offer.
#[test]
fn life_shaver_needs_a_counter() {
    let Some(mut s) = field(false, 8000, &[], &[], &[spell(0, LIFE_SHAVER)]) else { return };
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert!(s.prompt()["activates"].as_array().is_none_or(|v| v.iter().all(|c| c["cid"] != LIFE_SHAVER)), "{}", s.context());
}

/// Life Shaver (anime): "Your opponent discards 1 card from their hand for every 2 turns that this card has been Set
/// (counting the turn you Set this card as the 1st turn)." Set in turn 1, activated in turn 3: one discard.
#[test]
fn life_shaver_anime_discards_for_every_two_turns_set() {
    let Some(mut s) = field(true, 8000, &[], &[], &[hand(0, LIFE_SHAVER)]) else { return };
    s.run_both([&[SetSpell(LIFE_SHAVER), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    let before = hand_size(&s, 1);
    s.run(&[Activate(LIFE_SHAVER)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 1), before - 1, "{}", s.context());
}

/// Life Stream Dragon: "When this card is Synchro Summoned: You can make your LP become 4000." Turbo Synchron (Level 1
/// Tuner) and Power Tool Dragon (Level 7) make it; from 1000 LP it goes to 4000.
#[test]
fn life_stream_dragon_sets_lp_to_4000() {
    let spawns = [mon(0, TURBO_SYNCHRON), mon(0, POWER_TOOL)];
    let Some(mut s) = field(false, 1000, &[], &[LIFE_STREAM], &spawns) else { return };
    s.run(&[SpSummon(LIFE_STREAM), Chain(LIFE_STREAM)]).unwrap();
    clean(&s);
    assert!(s.monster(0, LIFE_STREAM).is_some(), "{}", s.context());
    assert_eq!(s.lp(0), 4000, "{}", s.context());
}

/// "You take no effect damage": the opponent's Thunder Crash (300 damage for each monster it destroys) does nothing to
/// Life Stream Dragon's controller.
#[test]
fn life_stream_dragon_takes_no_effect_damage() {
    let spawns = [mon(0, LIFE_STREAM), hand(1, THUNDER_CRASH), mon(1, WARWOLF), mon(1, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, &[], &[LIFE_STREAM], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(THUNDER_CRASH)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Thunder Crash resolved: {}", s.context());
    assert_eq!(s.lp(0), 8000, "no effect damage: {}", s.context());
}

/// "If this face-up card on the field would be destroyed, you can banish 1 Equip Spell Card from your Graveyard instead."
/// Dark Hole does not destroy it, and the Axe of Despair in the Graveyard is banished.
#[test]
fn life_stream_dragon_banishes_an_equip_spell_instead_of_being_destroyed() {
    let spawns = [mon(0, LIFE_STREAM), grave(0, AXE_OF_DESPAIR), hand(0, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, &[], &[LIFE_STREAM], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(LIFE_STREAM)]).unwrap();
    clean(&s);
    assert!(s.monster(0, LIFE_STREAM).is_some(), "it stayed: {}", s.context());
    assert_eq!(s.count(0, "removed", AXE_OF_DESPAIR), 1, "the Equip Spell was banished: {}", s.context());
}

/// Life Stream Dragon (anime): "Neither player takes damage from card effects." The opponent's own Thunder Crash does
/// nothing to its controller either.
#[test]
fn life_stream_dragon_anime_stops_effect_damage_for_both() {
    let spawns = [mon(0, LIFE_STREAM), hand(1, THUNDER_CRASH), mon(1, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[], &[LIFE_STREAM], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(THUNDER_CRASH)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Thunder Crash resolved: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Lightning Crash: "Add 1 Thunder monster from your Deck to your hand, but you cannot Normal Summon/Set monsters with
/// its name until the end of the next turn after this card resolves."
#[test]
fn lightning_crash_searches_a_thunder_monster_it_cannot_summon_yet() {
    let Some(mut s) = field(false, 8000, &[THUNDER_KID], &[], &[hand(0, LIGHTNING_CRASH), hand(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(LIGHTNING_CRASH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", THUNDER_KID), 2, "one from the Deck: {}", s.context());
    assert!(!summonable(&s, THUNDER_KID), "no Normal Summon of its name: {}", s.context());
}

/// Lightning Crash (anime): "Add 1 Thunder-Type monster from your Deck to your hand, but you cannot Normal Summon it this
/// turn." Only that card: the copy already in the hand can be summoned.
#[test]
fn lightning_crash_anime_blocks_only_the_added_card_this_turn() {
    let Some(mut s) = field(true, 8000, &[THUNDER_KID], &[], &[hand(0, LIGHTNING_CRASH), hand(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, THUNDER_KID)]);
    s.run(&[Activate(LIGHTNING_CRASH)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", THUNDER_KID), 2, "{}", s.context());
    assert!(summonable(&s, THUNDER_KID), "the copy that was already in the hand: {}", s.context());
}

/// Litmus Doom Ritual and Swordsman: "You must also Tribute monsters from your hand or field, whose total Levels equal 8 or
/// more." Dark Magician (Level 7) and Kuriboh (Level 1) are enough; "Gains 3000 ATK/DEF while a Trap is on the field"
/// (a face-up one: Life Shaver, a Continuous Trap).
#[test]
fn litmus_doom_swordsman_is_ritual_summoned_and_gains_3000_with_a_trap() {
    let spawns = [hand(0, LITMUS_RITUAL), hand(0, LITMUS), mon(0, DARK_MAGICIAN), mon(0, KURIBOH), spell(1, LIFE_SHAVER)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(LITMUS_RITUAL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, LITMUS), Some(3000), "0 ATK, plus 3000 for the Trap on the field: {}", s.context());
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "{}", s.context());
}

/// A Set Trap is not "on the field" for it, and without one the Swordsman has 0 ATK.
#[test]
fn litmus_doom_swordsman_has_no_attack_without_a_face_up_trap() {
    let spawns = [hand(0, LITMUS_RITUAL), hand(0, LITMUS), mon(0, DARK_MAGICIAN), mon(0, KURIBOH), set(1, TRAP_HOLE)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(LITMUS_RITUAL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, LITMUS), Some(0), "{}", s.context());
}

/// "It is unaffected by Trap effects and cannot be destroyed by battle." Blue-Eyes (3000 ATK) attacks the 0 ATK Swordsman:
/// it stays, and its controller takes the 3000 battle damage.
#[test]
fn litmus_doom_swordsman_is_not_destroyed_by_battle() {
    let Some(mut s) = field(false, 8000, &[], &[], &[mon(0, LITMUS), mon(1, BLUE_EYES)]) else { return };
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, LITMUS).is_some(), "{}", s.context());
    assert_eq!(s.lp(0), 5000, "{}", s.context());
}

/// "If this card is in the GY: You can target 1 "Litmus Doom Swordsman" in your GY; shuffle both it and this card from the
/// GY into the Deck, then draw 1 card."
#[test]
fn litmus_doom_ritual_shuffles_itself_and_the_swordsman_back_and_draws() {
    let Some(mut s) = field(false, 8000, &[], &[], &[grave(0, LITMUS_RITUAL), grave(0, LITMUS)]) else { return };
    let before = hand_size(&s, 0);
    s.run(&[Activate(LITMUS_RITUAL)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LITMUS_RITUAL) + s.count(0, "grave", LITMUS), 0, "both left the Graveyard: {}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "and one card was drawn: {}", s.context());
}

/// "If this Ritual Summoned card in its owner's control is destroyed by your opponent's card: You can target 1 Trap in
/// either GY; Set it to your Spell & Trap Zone." Ritual Summoned, then destroyed by the opponent's Dark Hole: it Sets Trap
/// Hole from the opponent's Graveyard.
#[test]
fn litmus_doom_swordsman_sets_a_trap_when_the_opponent_destroys_it() {
    let spawns = [hand(0, LITMUS_RITUAL), hand(0, LITMUS), mon(0, DARK_MAGICIAN), mon(0, KURIBOH), hand(1, DARK_HOLE), grave(1, TRAP_HOLE)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[Activate(LITMUS_RITUAL), EndTurn, Chain(LITMUS)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LITMUS), 1, "Dark Hole destroyed it: {}", s.context());
    assert_eq!(s.count(0, "szone", TRAP_HOLE), 1, "and it Set the Trap Hole: {}", s.context());
}

/// Living Fossil: "Activate this card by targeting 1 Level 4 or lower monster in your GY; Special Summon it and equip it
/// with this card, but banish it when it leaves the field (even if this card leaves the field). When this card leaves the
/// field, banish the equipped monster. The equipped monster loses 1000 ATK/DEF". Warwolf comes back with 1000 ATK; Mystical
/// Space Typhoon on the Fossil banishes it.
#[test]
fn living_fossil_revives_a_monster_that_is_banished_with_it() {
    let spawns = [hand(0, FOSSIL), grave(0, WARWOLF), hand(0, MST)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, WARWOLF), (0, FOSSIL)]);
    s.run(&[Activate(FOSSIL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "2000 - 1000: {}", s.context());
    s.run(&[Activate(MST)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "banished with the Fossil: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0);
}

/// The printed Fossil takes a Level 4 or lower monster only: Blue-Eyes (Level 8) is no target.
#[test]
fn living_fossil_cannot_take_a_high_level_monster() {
    let Some(mut s) = field(false, 8000, &[], &[], &[hand(0, FOSSIL), grave(0, BLUE_EYES)]) else { return };
    s.run(&[]).unwrap();
    assert!(s.prompt()["activates"].as_array().is_none_or(|v| v.iter().all(|c| c["cid"] != FOSSIL)), "{}", s.context());
}

/// Living Fossil (anime): "Target 1 monster in your Graveyard; Special Summon it, and equip it with this card, but it loses
/// 1000 ATK and its effects are negated. When this card is destroyed, destroy the equipped monster." Blue-Eyes comes back
/// with 2000 ATK, and is destroyed (not banished) with the Fossil.
#[test]
fn living_fossil_anime_takes_any_monster_and_destroys_it_with_the_fossil() {
    let spawns = [hand(0, FOSSIL), grave(0, BLUE_EYES), hand(0, MST)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, BLUE_EYES), (0, FOSSIL)]);
    s.run(&[Activate(FOSSIL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, BLUE_EYES), Some(2000), "{}", s.context());
    s.run(&[Activate(MST)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BLUE_EYES), 1, "destroyed with the Fossil: {}", s.context());
}

/// Loge's Flame: "While you control a "Valkyrie" monster, monsters your opponent controls with 2000 or less ATK cannot
/// attack." Warwolf (2000 ATK) cannot attack with Valkyrie Dritte on the field, and can without it.
#[test]
fn loges_flame_stops_small_attackers_while_a_valkyrie_is_controlled() {
    let spawns = [spell(0, LOGE), mon(0, DRITTE), mon(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    let e = s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap_err();
    assert!(e.contains("can't attack"), "Warwolf must not be able to attack: {e}");

    let spawns = [spell(0, LOGE), mon(0, KURIBOH), mon(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert!(s.lp(0) < 8000 || s.count(0, "grave", KURIBOH) > 0, "without a Valkyrie it attacks: {}", s.context());
}

/// "If this card in your possession is destroyed by an opponent's card effect: You can Special Summon 1 Level 5 or higher
/// "Valkyrie" monster from your hand or Deck." The opponent's Heavy Storm destroys it; Valkyrie Brunhilde (Level 7) comes
/// from the Deck.
#[test]
fn loges_flame_brings_a_high_level_valkyrie_when_destroyed() {
    let spawns = [spell(0, LOGE), mon(0, DRITTE), hand(1, HEAVY_STORM)];
    let Some(mut s) = field(false, 8000, &[BRUNHILDE], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(LOGE)], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, BRUNHILDE).is_some(), "Brunhilde came: {}", s.context());
    assert_eq!(s.count(0, "grave", LOGE), 1, "{}", s.context());
}

/// Loge's Flame (anime): "Monsters your opponent controls with 2000 or less ATK cannot attack." (no Valkyrie needed) and
/// "When this card leaves the field: You can Special Summon 1 "Valkyrie Brunhilde" from your hand or Deck."
#[test]
fn loges_flame_anime_needs_no_valkyrie_and_brings_brunhilde() {
    let spawns = [spell(0, LOGE), mon(1, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    let e = s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap_err();
    assert!(e.contains("can't attack"), "Warwolf must not be able to attack: {e}");

    let spawns = [spell(0, LOGE), hand(1, HEAVY_STORM)];
    let Some(mut s) = field(true, 8000, &[ZWEITE, BRUNHILDE], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(LOGE)], &[Activate(HEAVY_STORM)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, BRUNHILDE).is_some(), "only Brunhilde: {}", s.context());
    assert!(s.monster(0, ZWEITE).is_none());
}

/// Lord of the Red: "You can Ritual Summon this card with "Red-Eyes Transmigration". You must also Tribute monsters from your
/// hand or field, and/or banish "Red-Eyes" monsters from your GY, whose total Levels equal 8 or more."
#[test]
fn lord_of_the_red_is_ritual_summoned_with_red_eyes_transmigration() {
    let spawns = [hand(0, TRANSMIGRATION), hand(0, LORD_OF_THE_RED), mon(0, RED_EYES), mon(0, KURIBOH)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(TRANSMIGRATION)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, LORD_OF_THE_RED), Some(2400), "{}", s.context());
}

/// "Once per turn, when a card or effect is activated, except "Lord of the Red" (Quick Effect): You can target 1 monster on
/// the field; destroy it. Once per turn, ...: You can target 1 Spell/Trap on the field; destroy it." The opponent's two
/// Pots of Greed (with a monster and a Continuous Trap on their side): the first is answered with the monster effect
/// (Warwolf), the second with the Spell/Trap one (Gravity Bind).
#[test]
fn lord_of_the_red_destroys_a_monster_then_a_spell_or_trap() {
    let spawns = [mon(0, LORD_OF_THE_RED), mon(1, WARWOLF), spell(1, GRAVITY_BIND), hand(1, POT_OF_GREED), hand(1, POT_OF_GREED)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, WARWOLF), (1, GRAVITY_BIND)]);
    s.run_both([&[EndTurn, Chain(LORD_OF_THE_RED), Chain(LORD_OF_THE_RED)], &[Activate(POT_OF_GREED), Activate(POT_OF_GREED)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the monster effect: {}", s.context());
    assert_eq!(s.count(1, "grave", GRAVITY_BIND), 1, "the Spell/Trap effect: {}", s.context());
    assert!(s.monster(0, LORD_OF_THE_RED).is_some(), "Lord itself stayed: {}", s.context());
}

/// Lord of the Red (anime): "Once per turn, if 2 Spell Cards are activated in separate Chains during this turn, after a
/// Spell Card resolves: You can destroy all other monsters on the field." The opponent's two Pots of Greed.
#[test]
fn lord_of_the_red_anime_destroys_all_other_monsters_after_two_spells() {
    let spawns = [mon(0, LORD_OF_THE_RED), mon(0, WARWOLF), mon(1, BLUE_EYES), hand(1, POT_OF_GREED), hand(1, POT_OF_GREED)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(LORD_OF_THE_RED)], &[Activate(POT_OF_GREED), Activate(POT_OF_GREED)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert!(s.monster(0, LORD_OF_THE_RED).is_some(), "Lord stayed: {}", s.context());
}

/// Lucky Loan: "Target 1 "Fortune Fairy" monster you control; Special Summon 1 Spellcaster monster from your hand or Deck,
/// whose original Level is 1 lower than that monster's, also during your next turn after this card resolves, you cannot
/// Normal or Special Summon monsters, except Spellcaster monsters." Fortune Fairy En (Level 2) brings Fortune Fairy Hikari
/// (Level 1); in the next turn Warwolf cannot be Normal Summoned and Maiden of the Moonlight (Spellcaster) can.
#[test]
fn lucky_loan_brings_a_spellcaster_one_level_lower_and_limits_summons() {
    let spawns = [mon(0, FAIRY_EN), hand(0, LUCKY_LOAN), hand(0, WARWOLF), hand(0, MAIDEN_OF_THE_MOONLIGHT)];
    let Some(mut s) = field(false, 8000, &[FAIRY_HIKARI], &[], &spawns) else { return };
    s.prefer(&[(0, FAIRY_EN), (0, FAIRY_HIKARI), (0, KURIBOH)]);
    s.run_both([&[Activate(LUCKY_LOAN), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert!(s.monster(0, FAIRY_HIKARI).is_some(), "Hikari (Level 1): {}", s.context());
    assert!(!summonable(&s, WARWOLF), "no non-Spellcaster Normal Summon: {}", s.context());
    assert!(summonable(&s, MAIDEN_OF_THE_MOONLIGHT), "a Spellcaster can be: {}", s.context());
}

/// Machine Angel Ritual: "This card can be used to Ritual Summon any "Cyber Angel" Ritual Monster. You must also Tribute
/// monsters from your hand or field whose total Levels equal or exceed the Level of the Ritual Monster you Ritual Summon."
/// Dark Magician (Level 7) is enough for Cyber Angel Benten (Level 6).
#[test]
fn machine_angel_ritual_summons_a_cyber_angel() {
    let spawns = [hand(0, MACHINE_ANGEL_RITUAL), hand(0, BENTEN), mon(0, DARK_MAGICIAN)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(MACHINE_ANGEL_RITUAL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, BENTEN), Some(1800), "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "{}", s.context());
}

/// "If a LIGHT monster(s) you control would be destroyed by battle or card effect while this card is in your GY, you can
/// banish this card instead." Dark Hole spares Maiden of the Moonlight (LIGHT), and the Ritual Spell is banished.
#[test]
fn machine_angel_ritual_is_banished_in_place_of_a_light_monster() {
    let spawns = [grave(0, MACHINE_ANGEL_RITUAL), mon(0, MAIDEN_OF_THE_MOONLIGHT), hand(0, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Chain(MACHINE_ANGEL_RITUAL)]).unwrap();
    clean(&s);
    assert!(s.monster(0, MAIDEN_OF_THE_MOONLIGHT).is_some(), "it stayed: {}", s.context());
    assert_eq!(s.count(0, "removed", MACHINE_ANGEL_RITUAL), 1, "{}", s.context());
}

/// Lullaby of Obedience: "Pay 2000 LP and declare 1 Monster Card name; your opponent looks at their Deck, reveals 1 of the
/// declared monsters if there is any, and chooses 1 of these effects". The declared name is the harness's own pick (the
/// first the engine offers), so only the cost is checked: 2000 LP paid.
#[test]
fn lullaby_of_obedience_costs_2000_lp() {
    let Some(mut s) = field(false, 8000, &[], &[], &[hand(0, LULLABY)]) else { return };
    s.run(&[Activate(LULLABY)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", LULLABY), 1, "{}", s.context());
    assert_eq!(s.lp(0), 6000, "{}", s.context());
}

/// Lullaby of Obedience (anime): "Pay 1000 LP, then declare 1 card name; if the declared card is in your opponent's Deck
/// and is a Level 8 or higher monster, add that card from their Deck to your hand." The cost is 1000 LP.
#[test]
fn lullaby_of_obedience_anime_costs_1000_lp() {
    let Some(mut s) = field(true, 8000, &[], &[], &[hand(0, LULLABY)]) else { return };
    s.run(&[Activate(LULLABY)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 7000, "{}", s.context());
}

/// Magical Trick Mirror: "When an opponent's monster declares an attack: Target 1 Spell in your opponent's GY; Set it to
/// your field." Pot of Greed from the opponent's Graveyard is Set on the mirror's side.
#[test]
fn magical_trick_mirror_sets_a_spell_from_the_opponents_graveyard() {
    let spawns = [set(0, TRICK_MIRROR), grave(1, POT_OF_GREED), mon(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, ChainOnAttack(TRICK_MIRROR)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", POT_OF_GREED), 1, "Set on the mirror's side: {}", s.context());
    assert_eq!(s.count(1, "grave", POT_OF_GREED), 0, "{}", s.context());
}

/// "You can banish this card from your GY and send 1 "Monster Reborn" from your hand or face-down field to the GY; Special
/// Summon 1 "Obelisk the Tormentor" from your GY in Defense Position".
#[test]
fn magical_trick_mirror_in_the_graveyard_revives_obelisk() {
    let spawns = [grave(0, TRICK_MIRROR), hand(0, MONSTER_REBORN), grave(0, OBELISK)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(TRICK_MIRROR)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", OBELISK), 1, "{}", s.context());
    assert_eq!(s.position_of(0, OBELISK), Some(4), "face-up Defense Position: {}", s.context());
    assert_eq!(s.count(0, "removed", TRICK_MIRROR), 1, "banished as the cost: {}", s.context());
    assert_eq!(s.count(0, "grave", MONSTER_REBORN), 1, "Monster Reborn sent as the cost: {}", s.context());
}

/// Magical Trick Mirror (anime): "When the opponent declares an attack: Activate 1 Spell Card from your opponent's
/// Graveyard." Pot of Greed from their Graveyard: its activator draws 2.
#[test]
fn magical_trick_mirror_anime_activates_a_spell_from_the_opponents_graveyard() {
    let spawns = [set(0, TRICK_MIRROR), grave(1, POT_OF_GREED), mon(1, WARWOLF)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    let before = hand_size(&s, 0);
    s.run_both([&[EndTurn, ChainOnAttack(TRICK_MIRROR)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), before + 2, "Pot of Greed's two cards: {}", s.context());
}

/// Magnet Force: "This turn, all monsters on the field whose original Types are Machine or Rock, are unaffected by your
/// opponent's monster effects (except their own)." Zaborg (Tribute Summoned: destroy 1 monster) is aimed at Barrel Rock: it
/// destroys it, and with Magnet Force chained to it, it does not.
#[test]
fn magnet_force_protects_rock_monsters_from_monster_effects() {
    let spawns = [mon(1, BARREL_ROCK), mon(0, WARWOLF), hand(0, ZABORG)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, BARREL_ROCK)]);
    s.run(&[Summon(ZABORG)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BARREL_ROCK), 1, "without Magnet Force it is destroyed: {}", s.context());

    let spawns = [mon(1, BARREL_ROCK), set(1, MAGNET_FORCE), mon(0, WARWOLF), hand(0, ZABORG)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, BARREL_ROCK)]);
    summon_and_chain(&mut s, ZABORG, 1, MAGNET_FORCE, |s| s.count(0, "mzone", ZABORG) == 1);
    s.run(&[]).unwrap();
    clean(&s);
    assert!(s.monster(1, BARREL_ROCK).is_some(), "it is unaffected: {}", s.context());
    assert_eq!(s.count(1, "grave", MAGNET_FORCE), 1, "Magnet Force was used: {}", s.context());
}

/// Magnet Force (anime): "When your opponent activates a card or effect that targets a Machine or Rock monster(s) you
/// control: You can target 1 monster your opponent controls for each of those...; that card/effect now targets the new
/// target(s) instead". Zaborg's effect aimed at Barrel Rock is turned on Zaborg itself.
#[test]
fn magnet_force_anime_turns_the_effect_on_an_opposing_monster() {
    let spawns = [mon(1, BARREL_ROCK), set(1, MAGNET_FORCE), mon(0, WARWOLF), hand(0, ZABORG)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(1, BARREL_ROCK), (0, ZABORG)]);
    summon_and_chain(&mut s, ZABORG, 1, MAGNET_FORCE, |s| s.count(0, "mzone", ZABORG) == 1);
    s.run_both([&[], &[Chain(MAGNET_FORCE)]]).unwrap();
    clean(&s);
    assert!(s.monster(1, BARREL_ROCK).is_some(), "Barrel Rock is spared: {}", s.context());
    assert_eq!(s.count(0, "grave", ZABORG), 1, "and the effect's own monster is the target: {}", s.context());
}

/// Magnet Reverse: "Target 1 of your Machine or Rock monsters, that is banished or is in your GY, that cannot be Normal
/// Summoned/Set; Special Summon it." Barrel Rock can be Normal Summoned, so it is no target.
#[test]
fn magnet_reverse_takes_only_monsters_that_cannot_be_normal_summoned() {
    let Some(mut s) = field(false, 8000, &[], &[], &[hand(0, MAGNET_REVERSE), grave(0, BARREL_ROCK)]) else { return };
    s.run(&[]).unwrap();
    assert!(s.prompt()["activates"].as_array().is_none_or(|v| v.iter().all(|c| c["cid"] != MAGNET_REVERSE)), "{}", s.context());
}

/// Magnet Reverse (anime): "Target 1 Rock-Type or Machine-Type monster in your Graveyard that was sent to your Graveyard
/// this turn; Special Summon it in face-up Defense Position." Barrel Rock destroyed by Dark Hole comes back in Defense.
#[test]
fn magnet_reverse_anime_revives_a_rock_sent_to_the_graveyard_this_turn() {
    let spawns = [hand(0, MAGNET_REVERSE), hand(0, DARK_HOLE), mon(0, BARREL_ROCK)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run(&[Activate(DARK_HOLE), Activate(MAGNET_REVERSE)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(0, BARREL_ROCK), Some(4), "face-up Defense Position: {}", s.context());
}

/// Magnet Reverse (anime) takes only a monster sent there this turn: Barrel Rock already in the Graveyard is no target.
#[test]
fn magnet_reverse_anime_needs_a_monster_sent_this_turn() {
    let Some(mut s) = field(true, 8000, &[], &[], &[hand(0, MAGNET_REVERSE), grave(0, BARREL_ROCK)]) else { return };
    s.run(&[]).unwrap();
    assert!(s.prompt()["activates"].as_array().is_none_or(|v| v.iter().all(|c| c["cid"] != MAGNET_REVERSE)), "{}", s.context());
}

/// Magnet Warrior Sigma Minus: "Once per turn, when an attack is declared involving 2 EARTH monsters: You can negate that
/// attack." Beta The Magnet Warrior (EARTH, 1700 ATK) attacks Sigma Minus (EARTH, 1500 ATK): the attack is negated.
#[test]
fn sigma_minus_negates_an_attack_between_earth_monsters() {
    let spawns = [mon(0, SIGMA_MINUS), mon(1, BETA)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(SIGMA_MINUS)], &[Attack(BETA)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, SIGMA_MINUS).is_some(), "Sigma Minus survived: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// "If this card is Normal or Special Summoned: You can send 1 Level 8 "Magna Warrior" monster from your Deck to the GY."
#[test]
fn sigma_minus_sends_a_magna_warrior_when_summoned() {
    let Some(mut s) = field(false, 8000, &[VALKYRION], &[], &[hand(0, SIGMA_MINUS)]) else { return };
    s.prefer(&[(0, VALKYRION)]);
    s.run(&[Summon(SIGMA_MINUS), Chain(SIGMA_MINUS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", VALKYRION), 1, "{}", s.context());
}

/// Magnet Warrior Sigma Plus: "While you control an EARTH monster, your opponent's monsters that can attack must attack
/// EARTH monsters." Blue-Eyes must attack Sigma Plus (1800 ATK, EARTH), not Kuriboh (DARK): 1200 damage.
#[test]
fn sigma_plus_makes_attackers_choose_earth_monsters() {
    let spawns = [mon(0, KURIBOH), mon(0, SIGMA_PLUS), mon(1, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SIGMA_PLUS), 1, "Sigma Plus was the target: {}", s.context());
    assert_eq!(s.count(0, "grave", KURIBOH), 0, "{}", s.context());
}

/// "If this card is sent to the GY: You can target 1 Level 4 or lower "Magnet Warrior" monster in your GY, except "Magnet
/// Warrior Sigma Plus"; add it to your hand or Special Summon it." Dark Hole sends it to the GY; Alpha comes back.
#[test]
fn sigma_plus_gets_a_magnet_warrior_back_when_sent_to_the_graveyard() {
    let spawns = [mon(0, SIGMA_PLUS), grave(0, ALPHA), hand(0, DARK_HOLE)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.prefer(&[(0, ALPHA)]);
    s.run(&[Activate(DARK_HOLE), Chain(SIGMA_PLUS)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ALPHA), 0, "Alpha left the Graveyard: {}", s.context());
    assert_eq!(s.count(0, "hand", ALPHA) + s.count(0, "mzone", ALPHA), 1, "to the hand or the field: {}", s.context());
}

/// Maiden in Love: "Monsters your opponent controls that can attack must attack this card. Cannot be destroyed by battle."
/// Blue-Eyes attacks into Maiden in Love and Warwolf (first in line): Maiden (400 ATK) is the target and stays, its
/// controller takes the 2600 difference, and Warwolf is untouched.
#[test]
fn maiden_in_love_is_the_forced_target_and_cannot_be_destroyed() {
    let spawns = [mon(0, WARWOLF), mon(0, MAIDEN_IN_LOVE), mon(1, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert!(s.monster(0, MAIDEN_IN_LOVE).is_some(), "Maiden stayed: {}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "Warwolf was not attacked: {}", s.context());
    assert_eq!(s.lp(0), 5400, "3000 - 400: {}", s.context());
}

/// "At the end of the Damage Step, if this card battled an opponent's monster: You can activate 1 of these effects;
/// Place 1 Maiden Counter on 1 face-up monster your opponent controls. / Take control of 1 monster your opponent controls
/// with a Maiden Counter." With no counter about, the first battle places one on Blue-Eyes (counter 0x1090).
#[test]
fn maiden_in_love_places_a_maiden_counter() {
    let spawns = [mon(0, MAIDEN_IN_LOVE), mon(1, BLUE_EYES)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(MAIDEN_IN_LOVE)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.monster(1, BLUE_EYES).unwrap()["counters"]["4240"], 1, "a Maiden Counter: {}", s.context());
    assert!(s.monster(0, MAIDEN_IN_LOVE).is_some(), "{}", s.context());
}

/// Masked Beast Des Gardius: "Cannot be Normal Summoned/Set. Must first be Special Summoned (from your hand) by Tributing 2
/// monsters, including at least 1 "Grand Tiki Elder" or "Melchid the Four-Face Beast"." With Grand Tiki Elder and Warwolf it
/// can; with two Warwolves it cannot.
#[test]
fn masked_beast_des_gardius_needs_a_tiki_or_melchid() {
    let spawns = [mon(0, GRAND_TIKI), mon(0, WARWOLF), hand(0, DES_GARDIUS)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[SpSummon(DES_GARDIUS)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DES_GARDIUS), Some(3300), "{}", s.context());
    assert_eq!(s.count(0, "grave", GRAND_TIKI), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());

    let spawns = [mon(0, WARWOLF), mon(0, BARREL_ROCK), hand(0, DES_GARDIUS)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    assert!(
        s.prompt()["special_summons"].as_array().is_none_or(|v| v.iter().all(|c| c["cid"] != DES_GARDIUS)),
        "without a Tiki or Melchid: {}",
        s.context()
    );
    assert!(!summonable(&s, DES_GARDIUS), "and no Normal Summon: {}", s.context());
}

/// "If this card is sent from the field to your GY: Target 1 face-up monster your opponent controls; equip 1 "The Mask of
/// Remnants" from your Deck to it." and The Mask: "While this card is equipped to a monster by the effect of "Masked Beast
/// Des Gardius", take control of that monster." The opponent's Raigeki destroys it; Warwolf comes over.
#[test]
fn masked_beast_des_gardius_takes_a_monster_with_the_mask() {
    let spawns = [mon(0, DES_GARDIUS), mon(1, WARWOLF), hand(1, RAIGEKI)];
    let Some(mut s) = field(false, 8000, &[MASK_OF_REMNANTS], &[], &spawns) else { return };
    s.run_both([&[EndTurn], &[Activate(RAIGEKI)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DES_GARDIUS), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", MASK_OF_REMNANTS), 1, "the Mask is equipped: {}", s.context());
    assert!(s.monster(0, WARWOLF).is_some(), "Warwolf is controlled by the Mask's owner: {}", s.context());
}

/// Matador Archfiend: "If this card is Ritual Summoned: You can add 2 "Archfiend" cards from your Deck to your hand, except
/// "Matador Archfiend"". Ritual of the Matador with Dark Magician (Level 7); two Archfiends come to the hand.
#[test]
fn matador_archfiend_searches_two_archfiends_when_ritual_summoned() {
    let spawns = [hand(0, MATADOR_RITUAL), hand(0, MATADOR), mon(0, DARK_MAGICIAN)];
    let Some(mut s) = field(false, 8000, &[ARCHFIEND_SOLDIER, ARCHFIEND_MARMOT, ARCHFIEND_MARMOT], &[], &spawns) else { return };
    s.prefer(&[(0, ARCHFIEND_SOLDIER), (0, ARCHFIEND_MARMOT)]);
    s.run(&[Activate(MATADOR_RITUAL), Chain(MATADOR)]).unwrap();
    clean(&s);
    assert!(s.monster(0, MATADOR).is_some(), "{}", s.context());
    assert_eq!(s.count(0, "hand", ARCHFIEND_SOLDIER) + s.count(0, "hand", ARCHFIEND_MARMOT), 2, "two Archfiends: {}", s.context());
}

/// "Cannot be destroyed by battle, also you take no battle damage from battles involving this card. After damage
/// calculation, if this card battled a monster: Destroy that monster, and if you do, inflict 1000 damage to your
/// opponent." Matador (0 ATK) attacks Warwolf (2000 ATK): Warwolf is destroyed and the opponent takes 1000; Matador stays
/// and its controller takes nothing.
#[test]
fn matador_archfiend_destroys_what_it_battles_and_burns_for_1000() {
    let spawns = [mon(0, MATADOR), mon(1, WARWOLF)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Attack(MATADOR)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 7000, "1000 damage: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert!(s.monster(0, MATADOR).is_some(), "{}", s.context());
}

/// Mathematician: "When this card is Normal Summoned: You can send 1 Level 4 or lower monster from your Deck to the GY."
#[test]
fn mathematician_sends_a_low_level_monster_when_normal_summoned() {
    let Some(mut s) = field(false, 8000, &[WARWOLF, BLUE_EYES], &[], &[hand(0, MATHEMATICIAN)]) else { return };
    s.prefer(&[(0, WARWOLF)]);
    s.run(&[Summon(MATHEMATICIAN), Chain(MATHEMATICIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", BLUE_EYES), 0, "Level 8 is not allowed: {}", s.context());
}

/// "When this card is destroyed by battle and sent to the GY: You can draw 1 card." Blue-Eyes destroys it.
#[test]
fn mathematician_draws_when_destroyed_by_battle() {
    let Some(mut s) = field(false, 8000, &[], &[], &[mon(0, MATHEMATICIAN), mon(1, BLUE_EYES)]) else { return };
    let before = hand_size(&s, 0);
    s.run_both([&[EndTurn, Chain(MATHEMATICIAN)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", MATHEMATICIAN), 1, "{}", s.context());
    assert_eq!(hand_size(&s, 0), before + 1, "{}", s.context());
}

/// Mathematician (anime): "When this card is Normal Summoned: Send 1 card from your Deck to the Graveyard." Any card: the
/// Level 8 Blue-Eyes qualifies.
#[test]
fn mathematician_anime_sends_any_card() {
    let Some(mut s) = field(true, 8000, &[BLUE_EYES], &[], &[hand(0, MATHEMATICIAN)]) else { return };
    s.prefer(&[(0, BLUE_EYES)]);
    s.run(&[Summon(MATHEMATICIAN)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BLUE_EYES), 1, "{}", s.context());
}

/// Meklord Army of Granel: "Gains 100 ATK for each "Meklord" monster on the field, except this card." Skiel on its side and
/// Wisel on the opponent's: 1600 + 200.
#[test]
fn meklord_army_of_granel_gains_100_attack_per_meklord() {
    let spawns = [mon(0, GRANEL), mon(0, SKIEL), mon(1, WISEL)];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GRANEL), Some(1800), "{}", s.context());
}

/// "When this card is Normal Summoned: You can target 1 face-up monster your opponent controls; halve that target's ATK,
/// until the end of this turn." Blue-Eyes goes to 1500.
#[test]
fn meklord_army_of_granel_halves_an_attack_when_normal_summoned() {
    let Some(mut s) = field(false, 8000, &[], &[], &[hand(0, GRANEL), mon(1, BLUE_EYES)]) else { return };
    s.run(&[Summon(GRANEL), Chain(GRANEL)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, BLUE_EYES), Some(1500), "{}", s.context());
}

/// Meklord Army of Granel (anime): "Once per turn: you can halve the ATK or DEF of 1 face-up monster your opponent
/// controls." The first option (ATK) is taken: Blue-Eyes goes to 1500.
#[test]
fn meklord_army_of_granel_anime_halves_attack_or_defense() {
    let Some(mut s) = field(true, 8000, &[], &[], &[mon(0, GRANEL), mon(1, BLUE_EYES)]) else { return };
    s.run(&[Activate(GRANEL)]).unwrap();
    clean(&s);
    let (atk, def) = (s.attack_of(1, BLUE_EYES), s.monster(1, BLUE_EYES).and_then(|m| m["defense"].as_i64()));
    assert!(atk == Some(1500) || def == Some(1250), "ATK {atk:?}, DEF {def:?}: {}", s.context());
}

/// Meklord Army of Skiel: "Gains 200 ATK for each "Meklord" monster on the field, except this card." With Wisel: 1400.
#[test]
fn meklord_army_of_skiel_gains_200_attack_per_meklord() {
    let Some(mut s) = field(false, 8000, &[], &[], &[mon(0, SKIEL), mon(0, WISEL)]) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, SKIEL), Some(1400), "{}", s.context());
}

/// "When this card is destroyed by battle and sent to the GY: You can Special Summon 1 "Meklord Army" monster from your
/// Deck." Blue-Eyes destroys Skiel; Granel comes from the Deck.
#[test]
fn meklord_army_of_skiel_brings_another_meklord_army_when_destroyed_by_battle() {
    let Some(mut s) = field(false, 8000, &[GRANEL], &[], &[mon(0, SKIEL), mon(1, BLUE_EYES)]) else { return };
    s.prefer(&[(0, GRANEL)]);
    s.run_both([&[EndTurn, Chain(SKIEL)], &[Attack(BLUE_EYES)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", SKIEL), 1, "{}", s.context());
    assert!(s.monster(0, GRANEL).is_some(), "Granel came from the Deck: {}", s.context());
}

/// Meklord Army of Skiel (anime): "If you would take 1000 or more effect damage: You can reduce that damage to 0." The
/// opponent's Thunder Crash with four monsters (1200 damage).
#[test]
fn meklord_army_of_skiel_anime_stops_a_large_effect_damage() {
    let spawns = [mon(0, SKIEL), hand(1, THUNDER_CRASH), mon(1, WARWOLF), mon(1, BLUE_EYES), mon(1, DARK_MAGICIAN), mon(1, KURIBOH)];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(SKIEL)], &[Activate(THUNDER_CRASH)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", BLUE_EYES), 1, "Thunder Crash resolved: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Meklord Army of Wisel: "Gains 100 ATK for each "Meklord" monster on the field, except this card. Once per turn, when
/// another "Meklord" monster you control declares an attack on a Defense Position monster: You can activate this effect;
/// during that battle, if it attacks a Defense Position monster, inflict piercing battle damage to your opponent."
/// Granel (1600 + 100) attacks Warwolf (100 DEF) in Defense Position: 1600 piercing damage.
#[test]
fn meklord_army_of_wisel_gives_piercing_damage_to_another_meklord() {
    let spawns = [mon(0, WISEL), mon(0, GRANEL), Spawn { team: 1, place: MonsterDefense, cid: WARWOLF }];
    let Some(mut s) = field(false, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Attack(GRANEL), Chain(WISEL)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 6400, "1700 - 100 piercing: {}", s.context());
}

/// Meklord Army of Wisel (anime): "Once per turn, when a monster you control attacks a Defense Position monster, inflict
/// piercing battle damage." Any monster: Warwolf (2000 ATK) attacks the opponent's Warwolf (100 DEF): 1900.
#[test]
fn meklord_army_of_wisel_anime_gives_piercing_damage_to_any_monster() {
    let spawns = [mon(0, WISEL), mon(0, WARWOLF), Spawn { team: 1, place: MonsterDefense, cid: WARWOLF }];
    let Some(mut s) = field(true, 8000, &[], &[], &spawns) else { return };
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - (2000 - 100), "{}", s.context());
}
