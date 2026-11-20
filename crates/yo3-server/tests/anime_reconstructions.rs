//! The anime cards ProjectIgnis has no script for, and Skill Drain against the Egyptian Gods (#60), through the real core with
//! both seats' client models checked after every step (`duelcheck::scenario`): Dark Strike Fighter (8035) and Fortune Lady Fire
//! (8302), whose anime data record and script are ours (`engine/anime/records.json`, `engine/overrides/`), and the override of
//! Skill Drain (`engine/overrides/c82732705.lua`) that makes the anime Gods' effects stop for the turn it is activated and come back.
//! Skipped before `engine/fetch.sh`.
use serde_json::{Value as Json, json};
use yo3_server::duelcheck::scenario::{Action, Action::*, Scenario, engine_dir};
use yo3_server::ocg::{Response, battle, idle, msg, pos};

const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000 ATK, no effect
const HEDGEHOG: u16 = 7701; // Quillbolt Hedgehog: Level 2, 800 ATK
const JUNK_SYNCHRON: u16 = 7687; // Tuner, Level 3
const DSF: u16 = 8035; // Dark Strike Fighter
const FLF: u16 = 8302; // Fortune Lady Fire
const FORTUNE_LADY_LIGHT: u16 = 8301;
const FORTUNE_LADY_CALLING: u16 = 14509;
const FOOLISH_BURIAL: u16 = 5236;
const MONSTER_REBORN: u16 = 4842;
const BLACK_PENDANT: u16 = 4317;
const SOULICER_CRYSTAL: u16 = 5088; // Level 6, cannot be destroyed by card effects
const RAIGEKI_BREAK: u16 = 5544;
const EFFECT_VEILER: u16 = 8933;
const SKILL_DRAIN: u16 = 5740;
const OBELISK: u16 = 4998;
const SLIFER: u16 = 4999;
const RA: u16 = 5000;

fn start(anime: bool, decks: [(&[u16], &[u16]); 2]) -> Option<Scenario> {
    let dir = engine_dir()?;
    if !dir.join("vendor/BabelCDB/cards-unofficial.cdb").is_file() || !dir.join("anime-cards.json").is_file() {
        eprintln!("anime data not fetched; skipping");
        return None;
    }
    Some(if anime { Scenario::start_anime(&dir, 7, 8000, decks) } else { Scenario::start(&dir, 7, 8000, decks) }.expect("the duel starts"))
}

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

/// A 40-card main deck: the five cards of `hand` are the opening hand and `draws` the next draws in order, the rest Warwolves.
fn deck(hand: &[u16], draws: &[u16]) -> Vec<u16> {
    assert_eq!(hand.len(), 5);
    let mut d = vec![WARWOLF; 40 - hand.len() - draws.len()];
    d.extend(draws.iter().rev());
    d.extend(hand);
    d
}

/// A deck with `card` at the bottom: never drawn in a short duel, in the Deck for a search or a send to the Graveyard.
fn deck_with_bottom(hand: &[u16], card: u16) -> Vec<u16> {
    let mut d = deck(hand, &[]);
    d[0] = card;
    d
}

fn passive() -> Vec<u16> {
    vec![WARWOLF; 40]
}

/// What a step of [`drive`] does at the current prompt.
enum Step {
    Pass,
    Answer(Json),
    Stop,
}

/// Plays on with the default answers, asking `act` at every prompt, until it says `Stop`.
fn drive(s: &mut Scenario, mut act: impl FnMut(&Scenario) -> Step) {
    for _ in 0..400 {
        match act(s) {
            Step::Stop => return,
            Step::Answer(r) => s.respond(r).unwrap_or_else(|e| panic!("{e}\n{}", s.context())),
            Step::Pass => s.pass().unwrap_or_else(|e| panic!("{e}\n{}", s.context())),
        }
    }
    panic!("no end\n{}", s.context());
}

fn cids(p: &Json, list: &str) -> Vec<u64> {
    p[list].as_array().map_or(vec![], |v| v.iter().filter_map(|c| c["cid"].as_u64()).collect())
}

fn is_idle(s: &Scenario, team: u64) -> bool {
    s.prompt()["type"] == msg::SELECT_IDLECMD && s.prompt()["player"] == team
}

fn monsters(s: &Scenario, team: usize) -> usize {
    s.state()[team]["mzone"].as_array().map_or(0, |z| z.iter().filter(|c| !c.is_null()).count())
}

fn offers(s: &Scenario, cid: u16) -> bool {
    cids(s.prompt(), "activates").contains(&u64::from(cid))
}

/// Activates the effect of `cid` at the planned side's idle prompt (it must be offered), then plays on with the default
/// answers until the planned side is at an idle or battle prompt again. The opponent chains the card `chain.0` at its first
/// chance when it has it and `chain.1` says so.
/// The opponent's response: the card, and when it may chain it.
type ChainWhen = (u16, fn(&Scenario) -> bool);

fn activate_with(s: &mut Scenario, cid: u16, mut chain: Option<ChainWhen>) {
    let p = s.prompt().clone();
    let i = cids(&p, "activates").iter().position(|&c| c == u64::from(cid)).unwrap_or_else(|| panic!("{cid} is not offered: {p}\n{}", s.context()));
    s.respond(json!({"type": 1, "action": idle::ACTIVATE, "index": i})).unwrap();
    drive(s, |s| {
        let p = s.prompt();
        let kind = p["type"].as_u64();
        if (kind == Some(msg::SELECT_IDLECMD) || kind == Some(msg::SELECT_BATTLECMD)) && p["player"] == 0 {
            return Step::Stop;
        }
        if let Some((c, when)) = chain
            && kind == Some(msg::SELECT_CHAIN)
            && p["player"] == 1
            && when(s)
            && let Some(i) = cids(p, "selects").iter().position(|&x| x == u64::from(c))
        {
            chain = None;
            return Step::Answer(Response::Chain(Some(i)).json());
        }
        Step::Pass
    });
}

fn activate(s: &mut Scenario, cid: u16) {
    activate_with(s, cid, None);
}

/// Passes (the chain windows with nothing to chain) until the planned side has an idle or battle prompt again.
fn settle(s: &mut Scenario) -> Json {
    drive(s, |s| {
        let kind = s.prompt()["type"].as_u64();
        if (kind == Some(msg::SELECT_IDLECMD) || kind == Some(msg::SELECT_BATTLECMD)) && s.prompt()["player"] == 0 { Step::Stop } else { Step::Pass }
    });
    s.prompt().clone()
}

/// From the planned side's idle prompt to its Battle Phase: the battle prompt.
fn to_battle(s: &mut Scenario) -> Json {
    assert!(is_idle(s, 0) && s.prompt()["to_bp"] == true, "the Battle Phase is open: {}", s.prompt());
    s.respond(json!({"type": 1, "action": idle::TO_BP, "index": null})).unwrap();
    settle(s)
}

/// From a battle prompt: leave the Battle Phase for the Main Phase 2.
fn to_main2(s: &mut Scenario) {
    assert_eq!(s.prompt()["type"], msg::SELECT_BATTLECMD, "{}", s.prompt());
    s.respond(json!({"type": 1, "action": battle::TO_M2, "index": null})).unwrap();
    settle(s);
}

/// Passes on to the planned side's next turn (its idle prompt).
fn next_own_turn(s: &mut Scenario) {
    let mut left = false;
    drive(s, |s| {
        if s.prompt()["player"] == 1 && s.prompt()["type"] == msg::SELECT_IDLECMD {
            left = true;
        }
        if left && is_idle(s, 0) { Step::Stop } else { Step::Pass }
    });
}

// ---------------------------------------------------------------------------------------------------------------------
// Dark Strike Fighter (8035)

/// The first own turn a Warwolf, the second a Junk Synchron and the Synchro Summon (Level 4 + 3), the later ones what a test adds.
fn dsf_hand() -> [u16; 5] {
    [WARWOLF, JUNK_SYNCHRON, HEDGEHOG, WARWOLF, WARWOLF]
}

fn dsf_duel(hand: [u16; 5], opponent: Vec<u16>, extra: &[u16]) -> Option<Scenario> {
    start(true, [(&deck(&hand, &[]), extra), (&opponent, &[])])
}

#[test]
fn dark_strike_fighter_is_the_corrected_anime_card_and_cannot_tribute_itself() {
    let Some(mut s) = dsf_duel(dsf_hand(), passive(), &[DSF]) else { return };
    s.run(&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF)]).unwrap();
    assert_eq!((s.attack_of(0, DSF), s.level_of(0, DSF)), (Some(2600), Some(7)), "the anime stats, not the placeholder's 2700: {}", s.context());
    assert!(is_idle(&s, 0) && !offers(&s, DSF), "alone on the field it has no other monster to Tribute: {}", s.prompt());
    clean(&s);
}

#[test]
fn dark_strike_fighters_damage_is_the_tributes_current_level_times_200() {
    // Fortune Lady Fire gains a Level in each of its controller's Standby Phases: Level 4 on the third own turn (printed 2).
    let hand = [FLF, WARWOLF, JUNK_SYNCHRON, WARWOLF, WARWOLF];
    let Some(mut s) = dsf_duel(hand, passive(), &[DSF]) else { return };
    s.prefer(&[(0, FLF)]);
    s.run(&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(FLF)]).unwrap();
    assert_eq!(s.level_of(0, FLF), Some(2));
    next_own_turn(&mut s);
    assert_eq!(s.level_of(0, FLF), Some(3), "{}", s.context());
    activate(&mut s, DSF);
    assert_eq!(s.lp(1), 8000 - 3 * 200, "the Level it had when it was Tributed, not its printed 2: {}", s.context());
    assert_eq!(s.count(0, "grave", FLF), 1);
    clean(&s);
}

#[test]
fn dark_strike_fighter_once_per_turn_for_each_copy() {
    // Two copies. The first Tributes a Hedgehog (Level 2: 400); a second use of the same copy is refused although a Warwolf is
    // there to Tribute; the other copy works at once and Tributes the first (Level 7: 1400).
    let hand = [WARWOLF, JUNK_SYNCHRON, WARWOLF, JUNK_SYNCHRON, HEDGEHOG];
    let Some(mut s) = dsf_duel(hand, passive(), &[DSF, DSF]) else { return };
    s.prefer(&[(0, HEDGEHOG)]);
    s.run(&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(HEDGEHOG)]).unwrap();
    assert_eq!(monsters(&s, 0), 3, "two copies and the Hedgehog: {}", s.context());
    activate(&mut s, DSF);
    assert_eq!(s.lp(1), 8000 - 400);
    assert!(offers(&s, DSF), "the other copy is offered: {}", s.prompt());
    s.prefer(&[]);
    activate(&mut s, DSF);
    assert_eq!(s.lp(1), 8000 - 400 - 1400, "the other copy Tributed the first one, Level 7: {}", s.context());
    assert!(!offers(&s, DSF), "both copies have used it: {}", s.prompt());
    assert_eq!(monsters(&s, 0), 1, "one copy is left: {}", s.context());
    clean(&s);
}

#[test]
fn dark_strike_fighter_cannot_be_used_twice_in_a_turn() {
    let Some(mut s) = dsf_duel(dsf_hand(), passive(), &[DSF]) else { return };
    s.prefer(&[(0, HEDGEHOG)]);
    s.run(&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(WARWOLF), Summon(HEDGEHOG)]).unwrap();
    assert!(offers(&s, DSF));
    activate(&mut s, DSF);
    assert_eq!(s.lp(1), 8000 - 400);
    assert!(s.monster(0, WARWOLF).is_some() && s.monster(0, DSF).is_some());
    assert!(is_idle(&s, 0) && !offers(&s, DSF), "a second use is refused with a Warwolf to Tribute: {}", s.prompt());
    clean(&s);
}

#[test]
fn dark_strike_fighter_cannot_use_its_effect_after_attacking() {
    let Some(mut s) = dsf_duel(dsf_hand(), passive(), &[DSF]) else { return };
    s.run(&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(HEDGEHOG), Attack(DSF)]).unwrap();
    assert_eq!(s.lp(1), 8000 - 2600, "it attacked directly: {}", s.context());
    to_main2(&mut s);
    assert!(is_idle(&s, 0) && s.monster(0, HEDGEHOG).is_some() && !offers(&s, DSF), "it attacked this turn, so no effect, with a Hedgehog to Tribute: {}", s.prompt());
    clean(&s);
}

/// The turn Dark Strike Fighter uses its effect with a Warwolf beside it to attack with: its battle prompt.
fn after_the_effect() -> Option<Scenario> {
    let mut s = dsf_duel(dsf_hand(), passive(), &[DSF])?;
    s.prefer(&[(0, HEDGEHOG)]);
    s.run(&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(WARWOLF), Summon(HEDGEHOG)]).unwrap();
    // The Warwolf summoned on the second own turn can attack on this one.
    activate(&mut s, DSF);
    Some(s)
}

#[test]
fn dark_strike_fighter_cannot_attack_after_its_effect_and_may_the_next_turn() {
    let Some(mut s) = after_the_effect() else { return };
    assert_eq!(s.lp(1), 8000 - 400);
    let battle_prompt = to_battle(&mut s);
    let attackers = cids(&battle_prompt, "attacks");
    assert!(attackers.contains(&u64::from(WARWOLF)) && !attackers.contains(&u64::from(DSF)), "the Warwolf can attack, Dark Strike Fighter cannot: {battle_prompt}");
    s.respond(json!({"type": 1, "action": battle::TO_EP, "index": null})).unwrap();
    next_own_turn(&mut s);
    let battle_prompt = to_battle(&mut s);
    assert!(cids(&battle_prompt, "attacks").contains(&u64::from(DSF)), "the restriction ended with the turn: {battle_prompt}");
    clean(&s);
}

#[test]
fn a_negated_dark_strike_fighter_effect_deals_no_damage_and_the_restriction_stays() {
    let mut s = {
        let opponent = deck(&[EFFECT_VEILER, WARWOLF, WARWOLF, WARWOLF, WARWOLF], &[]);
        let Some(mut s) = dsf_duel(dsf_hand(), opponent, &[DSF]) else { return };
        s.prefer(&[(0, HEDGEHOG)]);
        s.run(&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(WARWOLF), Summon(HEDGEHOG)]).unwrap();
        s
    };
    activate_with(&mut s, DSF, Some((EFFECT_VEILER, |_| true)));
    assert_eq!(s.count(1, "grave", EFFECT_VEILER), 1, "Effect Veiler negated it: {}", s.context());
    assert_eq!(s.lp(1), 8000, "no damage: {}", s.context());
    assert!(s.monster(0, DSF).is_some() && s.count(0, "grave", HEDGEHOG) == 1, "the cost was paid");
    let battle_prompt = to_battle(&mut s);
    let attackers = cids(&battle_prompt, "attacks");
    assert!(attackers.contains(&u64::from(WARWOLF)) && !attackers.contains(&u64::from(DSF)), "still cannot attack: {battle_prompt}");
    clean(&s);
}

#[test]
fn dark_strike_fighters_damage_is_dealt_even_if_it_leaves_the_field_before_it_resolves() {
    let opponent = deck(&[RAIGEKI_BREAK, WARWOLF, WARWOLF, WARWOLF, WARWOLF], &[]);
    let Some(mut s) = dsf_duel(dsf_hand(), opponent, &[DSF]) else { return };
    // Cost: the Hedgehog; Raigeki Break, Set on the opponent's turn, then destroys Dark Strike Fighter in response.
    s.prefer(&[(0, HEDGEHOG), (0, DSF)]);
    s.run_both([&[Summon(WARWOLF), Summon(JUNK_SYNCHRON), SpSummon(DSF), Summon(WARWOLF), Summon(HEDGEHOG)], &[SetSpell(RAIGEKI_BREAK)]]).unwrap();
    activate_with(&mut s, DSF, Some((RAIGEKI_BREAK, |_| true)));
    assert_eq!(s.count(0, "grave", DSF), 1, "Raigeki Break destroyed it: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 400, "the damage belongs to the activation: {}", s.context());
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------------
// Fortune Lady Fire (8302)

/// The opponent's Warwolf from its first turn; the planned side's turns start with `EndTurn`, so that it has a monster to hit.
fn flf_duel(anime: bool, hand: [u16; 5], opponent: Vec<u16>) -> Option<Scenario> {
    start(anime, [(&deck_with_bottom(&hand, FLF), &[]), (&opponent, &[])])
}

/// Foolish Burial sends it to the Graveyard and Monster Reborn Special Summons it there on the third turn.
fn reborn(anime: bool, defense: bool, opponent: Vec<u16>, opp_plan: &[Action], lead: &[Action]) -> Option<Scenario> {
    let mut s = flf_duel(anime, [FOOLISH_BURIAL, MONSTER_REBORN, WARWOLF, WARWOLF, WARWOLF], opponent)?;
    // The anime Foolish Burial sends the monster to the opponent's Graveyard: Monster Reborn finds it there ("either").
    s.prefer(&[(0, FLF), (1, FLF)]);
    if defense {
        s.prefer_position(Some(pos::FACEUP_DEFENSE));
    }
    let mut plan: Vec<_> = lead.to_vec();
    plan.extend([Activate(FOOLISH_BURIAL), Activate(MONSTER_REBORN)]);
    s.run_both([&plan, opp_plan]).unwrap();
    Some(s)
}

#[test]
fn fortune_lady_fire_destroys_and_burns_when_monster_reborn_summons_it_in_attack_position() {
    let Some(s) = reborn(true, false, passive(), &[Summon(WARWOLF)], &[EndTurn]) else { return };
    assert_eq!(s.position_of(0, FLF), Some(1), "face-up Attack Position: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the Warwolf was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "damage equal to its ATK: {}", s.context());
    clean(&s);
}

#[test]
fn fortune_lady_fire_still_triggers_when_a_fortune_lady_effect_summons_it() {
    let mut s = flf_duel(true, [FORTUNE_LADY_LIGHT, FORTUNE_LADY_CALLING, WARWOLF, WARWOLF, WARWOLF], passive()).unwrap_or_else(|| panic!());
    s.prefer(&[(1, WARWOLF), (0, FLF)]);
    s.run_both([&[EndTurn, Summon(FORTUNE_LADY_LIGHT), Activate(FORTUNE_LADY_CALLING)], &[Summon(WARWOLF)]]).unwrap();
    assert!(s.monster(0, FLF).is_some(), "Fortune Lady Calling summoned it: {}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.lp(1)), (1, 6000), "{}", s.context());
    clean(&s);
}

#[test]
fn fortune_lady_fire_does_not_trigger_in_defense_position_or_when_normal_summoned() {
    let Some(s) = reborn(true, true, passive(), &[Summon(WARWOLF)], &[EndTurn]) else { return };
    assert_eq!(s.position_of(0, FLF), Some(4), "Special Summoned in Defense Position: {}", s.context());
    assert!(s.monster(1, WARWOLF).is_some() && s.lp(1) == 8000, "nothing was destroyed and no damage: {}", s.context());
    clean(&s);
    // Normal Summoned (Level 2: no Tribute) on the second own turn, the opponent's Warwolf there.
    let mut s = flf_duel(true, [FLF, WARWOLF, WARWOLF, WARWOLF, WARWOLF], passive()).unwrap();
    s.run_both([&[EndTurn, Summon(FLF)], &[Summon(WARWOLF)]]).unwrap();
    assert!(s.monster(0, FLF).is_some() && s.monster(1, WARWOLF).is_some() && s.lp(1) == 8000, "{}", s.context());
    clean(&s);
}

#[test]
fn fortune_lady_fires_damage_is_the_destroyed_monsters_modified_atk() {
    // The opponent equips the Warwolf with Black Pendant (+500 ATK): 2500 damage, not the printed 2000.
    let opponent = deck(&[BLACK_PENDANT, WARWOLF, WARWOLF, WARWOLF, WARWOLF], &[]);
    let Some(s) = reborn(true, false, opponent, &[Summon(WARWOLF), Activate(BLACK_PENDANT)], &[EndTurn]) else { return };
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2500, "{}", s.context());
    clean(&s);
}

#[test]
fn a_monster_that_cannot_be_destroyed_takes_no_damage_from_fortune_lady_fire() {
    // Soulicer Crystal cannot be destroyed by card effects: nothing destroyed, nothing inflicted.
    let opponent = deck(&[SOULICER_CRYSTAL, WARWOLF, WARWOLF, WARWOLF, WARWOLF], &[]);
    let Some(s) = reborn(true, false, opponent, &[Summon(WARWOLF), Summon(SOULICER_CRYSTAL)], &[EndTurn, EndTurn]) else { return };
    assert!(s.monster(0, FLF).is_some() && s.monster(1, SOULICER_CRYSTAL).is_some(), "{}", s.context());
    assert_eq!(s.lp(1), 8000, "no damage without the destruction: {}", s.context());
    clean(&s);
}

#[test]
fn if_the_target_leaves_before_the_effect_resolves_there_is_no_damage() {
    // Raigeki Break, Set on the opponent's turn, destroys the opponent's own Warwolf in response to Fortune Lady Fire's trigger.
    let opponent = deck(&[RAIGEKI_BREAK, WARWOLF, WARWOLF, WARWOLF, WARWOLF], &[]);
    let Some(mut s) = flf_duel(true, [FOOLISH_BURIAL, MONSTER_REBORN, WARWOLF, WARWOLF, WARWOLF], opponent) else { return };
    s.prefer(&[(1, WARWOLF), (0, FLF)]);
    s.run_both([&[EndTurn, Activate(FOOLISH_BURIAL)], &[Summon(WARWOLF), SetSpell(RAIGEKI_BREAK)]]).unwrap();
    activate_with(&mut s, MONSTER_REBORN, Some((RAIGEKI_BREAK, |s| s.monster(0, FLF).is_some())));
    assert_eq!(s.count(1, "grave", RAIGEKI_BREAK), 1, "it was chained: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 2, "the destroyed Warwolf and the discarded one: {}", s.context());
    assert_eq!(s.lp(1), 8000, "the target was gone: {}", s.context());
    clean(&s);
}

#[test]
fn fortune_lady_fires_level_stops_at_12() {
    let Some(mut s) = flf_duel(true, [FLF, WARWOLF, WARWOLF, WARWOLF, WARWOLF], passive()) else { return };
    s.run(&[Summon(FLF)]).unwrap();
    assert_eq!((s.level_of(0, FLF), s.attack_of(0, FLF)), (Some(2), Some(400)));
    for turn in 1..=12u64 {
        next_own_turn(&mut s);
        let level = (2 + turn).min(12);
        assert_eq!((s.level_of(0, FLF), s.attack_of(0, FLF)), (Some(level), Some(level as i64 * 200)), "own turn {}: {}", turn + 1, s.context());
    }
    clean(&s);
}

#[test]
fn in_the_official_mode_fortune_lady_fire_keeps_its_fortune_lady_source() {
    // Monster Reborn: no trigger. A Fortune Lady effect (Calling): the trigger.
    let Some(s) = reborn(false, false, passive(), &[Summon(WARWOLF)], &[EndTurn]) else { return };
    assert!(s.monster(0, FLF).is_some(), "{}", s.context());
    assert!(s.monster(1, WARWOLF).is_some() && s.lp(1) == 8000, "no Fortune Lady source, no trigger: {}", s.context());
    let mut s = flf_duel(false, [FORTUNE_LADY_LIGHT, FORTUNE_LADY_CALLING, WARWOLF, WARWOLF, WARWOLF], passive()).unwrap();
    s.prefer(&[(1, WARWOLF), (0, FLF)]);
    s.run_both([&[EndTurn, Summon(FORTUNE_LADY_LIGHT), Activate(FORTUNE_LADY_CALLING)], &[Summon(WARWOLF)]]).unwrap();
    assert_eq!((s.count(1, "grave", WARWOLF), s.lp(1)), (1, 6000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------------
// Skill Drain and the Egyptian Gods

/// Skill Drain Set on the first own turn, then three Warwolves, then the God; `extra` follows (a Summon, the activation).
fn gods_duel(anime: bool, god: u16) -> Option<Scenario> {
    start(anime, [(&deck(&[SKILL_DRAIN, WARWOLF, WARWOLF, WARWOLF, god], &[]), &[]), (&passive(), &[])])
}

/// The ATK a God has when it is not negated: Ra the Tributes' total (3 Warwolves), Slifer 1000 per card in the hand.
fn god_atk(s: &Scenario, god: u16) -> i64 {
    match god {
        RA => 3 * 2000,
        SLIFER => 1000 * s.state()[0]["hand"].as_array().map_or(0, Vec::len) as i64,
        _ => 4000,
    }
}

#[test]
fn skill_drain_negates_the_gods_on_the_field_until_the_turn_ends() {
    for god in [RA, SLIFER] {
        let Some(mut s) = gods_duel(true, god) else { return };
        s.run(&[SetSpell(SKILL_DRAIN), Summon(WARWOLF), Summon(WARWOLF), Summon(WARWOLF), Summon(god)]).unwrap();
        let normal = god_atk(&s, god);
        assert!(normal > 0 && s.attack_of(0, god) == Some(normal), "{god} has its ATK, {normal}: {}", s.context());
        activate(&mut s, SKILL_DRAIN);
        assert_eq!(s.count(0, "szone", SKILL_DRAIN), 1, "Skill Drain is face-up: {}", s.context());
        let negated = s.attack_of(0, god).expect("the God is on the field");
        assert!(negated <= 0, "{god}: its effects are negated at once, so the ATK that its effects give is gone: {negated} ({})", s.context());
        // The next own turn: Skill Drain is still face-up and the God has its effects back.
        next_own_turn(&mut s);
        assert_eq!(s.count(0, "szone", SKILL_DRAIN), 1);
        assert_eq!(s.attack_of(0, god), Some(god_atk(&s, god)), "{god}: back from the next turn: {}", s.context());
        assert!(s.attack_of(0, god).is_some_and(|a| a > 0));
        clean(&s);
    }
}

#[test]
fn ras_atk_is_zero_for_the_rest_of_the_turn_under_skill_drain_and_the_total_again_after() {
    let Some(mut s) = gods_duel(true, RA) else { return };
    s.run(&[SetSpell(SKILL_DRAIN), Summon(WARWOLF), Summon(WARWOLF), Summon(WARWOLF), Summon(RA)]).unwrap();
    assert_eq!(s.attack_of(0, RA), Some(6000), "the three Tributes' ATK: {}", s.context());
    activate(&mut s, SKILL_DRAIN);
    assert_eq!(s.attack_of(0, RA), Some(0), "0 for the rest of the turn: {}", s.context());
    // Still 0 in the Battle Phase of that turn, then the total back at the start of the next own turn.
    next_own_turn(&mut s);
    assert_eq!(s.attack_of(0, RA), Some(6000), "{}", s.context());
    clean(&s);
}

/// Obelisk with Skill Drain and Raigeki Break both Set, `before` its Summon (the steps of the plan between). Obelisk is
/// unaffected by Trap effects while its effects are not negated, so Raigeki Break (activated by its own controller here, on
/// Obelisk) destroys it only while they are.
fn obelisk_duel(before: &[Action]) -> Option<Scenario> {
    let mut s = start(true, [(&deck(&[SKILL_DRAIN, WARWOLF, WARWOLF, WARWOLF, OBELISK], &[RAIGEKI_BREAK]), &[]), (&passive(), &[])])?;
    s.prefer(&[(0, OBELISK)]);
    let mut plan = vec![SetSpell(SKILL_DRAIN), Summon(WARWOLF), SetSpell(RAIGEKI_BREAK), Summon(WARWOLF)];
    plan.extend(before);
    plan.extend([Summon(WARWOLF), Summon(OBELISK)]);
    s.run(&plan).unwrap();
    assert!(s.monster(0, OBELISK).is_some(), "{}", s.context());
    Some(s)
}

fn obelisk_survives_raigeki_break(s: &mut Scenario) -> bool {
    activate(s, RAIGEKI_BREAK);
    assert_eq!(s.count(0, "grave", RAIGEKI_BREAK), 1, "Raigeki Break resolved: {}", s.context());
    s.monster(0, OBELISK).is_some()
}

#[test]
fn obelisk_is_unaffected_by_a_trap_effect_without_skill_drain() {
    let Some(mut s) = obelisk_duel(&[]) else { return };
    assert!(obelisk_survives_raigeki_break(&mut s), "the control: Raigeki Break does not destroy it: {}", s.context());
    clean(&s);
}

#[test]
fn skill_drain_negates_obelisks_effects_for_the_turn_only() {
    let Some(mut s) = obelisk_duel(&[]) else { return };
    activate(&mut s, SKILL_DRAIN);
    assert!(!obelisk_survives_raigeki_break(&mut s), "negated this turn, it is not unaffected: {}", s.context());
    assert_eq!(s.count(0, "grave", OBELISK), 1);
    clean(&s);
    let Some(mut s) = obelisk_duel(&[]) else { return };
    activate(&mut s, SKILL_DRAIN);
    next_own_turn(&mut s);
    assert_eq!(s.count(0, "szone", SKILL_DRAIN), 1);
    assert!(obelisk_survives_raigeki_break(&mut s), "the next turn its effects are back, Skill Drain still face-up: {}", s.context());
    clean(&s);
}

#[test]
fn obelisk_summoned_while_skill_drain_is_face_up_is_not_affected() {
    let Some(mut s) = obelisk_duel(&[Activate(SKILL_DRAIN)]) else { return };
    assert_eq!(s.count(0, "szone", SKILL_DRAIN) + s.count(0, "szone", RAIGEKI_BREAK), 2, "{}", s.context());
    assert!(obelisk_survives_raigeki_break(&mut s), "{}", s.context());
    clean(&s);
}

#[test]
fn a_god_summoned_while_skill_drain_is_face_up_is_not_affected() {
    for god in [RA, SLIFER] {
        let Some(mut s) = gods_duel(true, god) else { return };
        // Skill Drain is activated on the second own turn, the God arrives on the fourth.
        s.run(&[SetSpell(SKILL_DRAIN), Summon(WARWOLF), Summon(WARWOLF)]).unwrap();
        activate(&mut s, SKILL_DRAIN);
        s.run(&[Summon(WARWOLF), Summon(god)]).unwrap();
        assert_eq!(s.count(0, "szone", SKILL_DRAIN), 1);
        assert_eq!(s.attack_of(0, god), Some(god_atk(&s, god)), "{god} has its ATK the turn it arrives, with Skill Drain face-up: {}", s.context());
        next_own_turn(&mut s);
        assert_eq!(s.attack_of(0, god), Some(god_atk(&s, god)), "{god}, a turn later: {}", s.context());
        clean(&s);
    }
}

#[test]
fn skill_drain_still_negates_other_monsters_for_good() {
    // Fortune Lady Fire's ATK is its effect: 0 under Skill Drain, and still 0 next turn, while Ra gets its effect back.
    for anime in [true, false] {
        let hand = [SKILL_DRAIN, FLF, WARWOLF, WARWOLF, WARWOLF];
        let Some(mut s) = start(anime, [(&deck(&hand, &[RA]), &[]), (&passive(), &[])]) else { return };
        s.prefer(&[(0, WARWOLF), (0, WARWOLF), (0, WARWOLF)]);
        let mut plan = vec![SetSpell(SKILL_DRAIN), Summon(FLF), Summon(WARWOLF), Summon(WARWOLF), Summon(WARWOLF)];
        if anime {
            plan.push(Summon(RA));
        }
        s.run(&plan).unwrap();
        assert!(s.attack_of(0, FLF).is_some_and(|a| a > 0), "{}", s.context());
        activate(&mut s, SKILL_DRAIN);
        assert_eq!(s.attack_of(0, FLF), Some(0), "negated: {}", s.context());
        next_own_turn(&mut s);
        assert_eq!(s.attack_of(0, FLF), Some(0), "Skill Drain keeps negating a monster that is not a God (anime {anime}): {}", s.context());
        if anime {
            assert_eq!(s.attack_of(0, RA), Some(6000), "{}", s.context());
        }
        clean(&s);
    }
}

/// Ra with Tributes of different ATK (2000, 800 and 2000): the total is exactly 4800, 0 for the rest of the turn Skill Drain
/// is activated, and exactly 4800 again from the next, not just some non-zero number.
#[test]
fn ras_exact_total_comes_back_after_skill_drain() {
    let hand = [SKILL_DRAIN, WARWOLF, HEDGEHOG, WARWOLF, RA];
    let Some(mut s) = start(true, [(&deck(&hand, &[]), &[]), (&passive(), &[])]) else { return };
    s.run(&[SetSpell(SKILL_DRAIN), Summon(WARWOLF), Summon(HEDGEHOG), Summon(WARWOLF), Summon(RA)]).unwrap();
    assert_eq!(s.attack_of(0, RA), Some(4800), "{}", s.context());
    activate(&mut s, SKILL_DRAIN);
    assert_eq!(s.attack_of(0, RA), Some(0), "{}", s.context());
    next_own_turn(&mut s);
    assert_eq!(s.attack_of(0, RA), Some(4800), "the exact total: {}", s.context());
    next_own_turn(&mut s);
    assert_eq!(s.attack_of(0, RA), Some(4800), "and it stays: {}", s.context());
    clean(&s);
}

/// With the setting off the official Gods and Skill Drain are the game's: Skill Drain negates the God for good, as it does
/// any effect monster (the override does nothing without the anime Gods).
#[test]
fn standard_skill_drain_negates_a_god_for_good() {
    let Some(mut s) = gods_duel(false, SLIFER) else { return };
    s.run(&[SetSpell(SKILL_DRAIN), Summon(WARWOLF), Summon(WARWOLF), Summon(WARWOLF), Summon(SLIFER)]).unwrap();
    activate(&mut s, SKILL_DRAIN);
    let now = s.attack_of(0, SLIFER);
    next_own_turn(&mut s);
    let later = s.attack_of(0, SLIFER);
    next_own_turn(&mut s);
    assert_eq!(s.attack_of(0, SLIFER), later, "standard Skill Drain does not give the God its effects back: {} / {now:?} {later:?}", s.context());
    assert!(later.is_some_and(|a| a <= 0), "negated for good in standard mode: {later:?} {}", s.context());
    clean(&s);
}

/// Two copies. Skill Drain affects a God once ("until the end of the turn in which it first affects it"): the second copy,
/// activated a turn later, does not negate a God the first one already affected, and does negate one that the first did not.
#[test]
fn a_second_skill_drain_does_not_affect_a_god_again() {
    for god in [RA, SLIFER] {
        let hand = [SKILL_DRAIN, SKILL_DRAIN, WARWOLF, WARWOLF, WARWOLF];
        let Some(mut s) = start(true, [(&deck(&hand, &[god]), &[]), (&passive(), &[])]) else { return };
        s.run(&[SetSpell(SKILL_DRAIN), SetSpell(SKILL_DRAIN), Summon(WARWOLF), Summon(WARWOLF), Summon(WARWOLF), Summon(god)]).unwrap();
        activate(&mut s, SKILL_DRAIN);
        assert!(s.attack_of(0, god).is_some_and(|a| a <= 0), "{god}: the first copy negates it: {}", s.context());
        next_own_turn(&mut s);
        assert_eq!(s.attack_of(0, god), Some(god_atk(&s, god)), "{god}: back: {}", s.context());
        activate(&mut s, SKILL_DRAIN);
        assert_eq!(s.count(0, "szone", SKILL_DRAIN), 2, "both are face-up: {}", s.context());
        assert_eq!(s.attack_of(0, god), Some(god_atk(&s, god)), "{god}: the second copy does not affect it again: {}", s.context());
        next_own_turn(&mut s);
        assert_eq!(s.attack_of(0, god), Some(god_atk(&s, god)), "{god}: {}", s.context());
        clean(&s);
    }
    // A God that arrives after the first copy is affected by the second, once.
    let hand = [SKILL_DRAIN, SKILL_DRAIN, WARWOLF, WARWOLF, WARWOLF];
    let Some(mut s) = start(true, [(&deck(&hand, &[RA]), &[]), (&passive(), &[])]) else { return };
    s.run(&[SetSpell(SKILL_DRAIN), SetSpell(SKILL_DRAIN), Summon(WARWOLF), Summon(WARWOLF), Summon(WARWOLF)]).unwrap();
    activate(&mut s, SKILL_DRAIN);
    s.run(&[Summon(RA)]).unwrap();
    assert_eq!(s.attack_of(0, RA), Some(6000), "Ra arrives under the first copy, unaffected: {}", s.context());
    activate(&mut s, SKILL_DRAIN);
    assert_eq!(s.attack_of(0, RA), Some(0), "the second copy affects it for the first time: {}", s.context());
    next_own_turn(&mut s);
    assert_eq!(s.attack_of(0, RA), Some(6000), "{}", s.context());
    clean(&s);
}

/// A God that leaves the field and returns is a new card to the game: Skill Drain face-up does not affect it, and the next
/// Skill Drain activated does, once. Slifer is destroyed by Raigeki Break in the turn it is negated (it is unaffected otherwise)
/// and brought back by Monster Reborn a turn later.
#[test]
fn a_god_that_comes_back_is_a_new_card() {
    let hand = [SKILL_DRAIN, SKILL_DRAIN, WARWOLF, WARWOLF, WARWOLF];
    let Some(mut s) = start(true, [(&deck(&hand, &[RAIGEKI_BREAK, SLIFER, WARWOLF, WARWOLF, MONSTER_REBORN]), &[]), (&passive(), &[])]) else { return };
    s.prefer(&[(0, SLIFER)]);
    s.run(&[SetSpell(SKILL_DRAIN), SetSpell(SKILL_DRAIN), Summon(WARWOLF), SetSpell(RAIGEKI_BREAK), Summon(WARWOLF), Summon(WARWOLF), Summon(SLIFER)]).unwrap();
    activate(&mut s, SKILL_DRAIN);
    assert!(s.attack_of(0, SLIFER).is_some_and(|a| a <= 0), "{}", s.context());
    activate(&mut s, RAIGEKI_BREAK);
    assert_eq!(s.count(0, "grave", SLIFER), 1, "negated, Slifer is destroyed: {}", s.context());
    while !offers(&s, MONSTER_REBORN) {
        next_own_turn(&mut s);
    }
    activate(&mut s, MONSTER_REBORN);
    assert!(s.monster(0, SLIFER).is_some(), "Slifer is back: {}", s.context());
    assert_eq!(s.attack_of(0, SLIFER), Some(god_atk(&s, SLIFER)), "a new card, Skill Drain face-up does not affect it: {}", s.context());
    assert!(s.attack_of(0, SLIFER).is_some_and(|a| a > 0), "{}", s.context());
    activate(&mut s, SKILL_DRAIN);
    assert!(s.attack_of(0, SLIFER).is_some_and(|a| a <= 0), "the next Skill Drain affects the new card once: {}", s.context());
    // The anime Gods Special Summoned by a Spell effect go back to where they came from at the End Phase (their own rule).
    next_own_turn(&mut s);
    assert_eq!(s.count(0, "grave", SLIFER), 1, "{}", s.context());
    clean(&s);
}
