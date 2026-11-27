//! Anime-only cards, group 26 (rows 6055-6085 of engine/added/cards.tsv): Primo's, Professor Frank's, Aporia's, Familiar of
//! Red Nova's, Kalin Kessler's and Luna's cards. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after every
//! step (`duelcheck::scenario`); the texts are the anime ones. Most of the group's cards are Traps, and a Trap answering the
//! opponent (or used on the team's own turn 3) is Set first, with the plan ending turn 1 before it is used; neither player
//! has a Battle Phase in turn 1, so team 0's attacks wait for turn 3.
//! Covered: the cards the per-card fuzz flagged (Echo Mirror, Wise G3, Wise T3, Immortal Homeostasis, Wave of Ill Intent,
//! Damage Boost, Level Cannon, Power Gravity, Magic Law, Spirit Wave Barrier, Fairy Breath), the sample of the rest (Dodge
//! Roll, Double Tribute, Infernity Burst, Infinity Guard Formation, Spirit Contamination, Twinkle Wall), the signature cards
//! (Yomi Boat Watchman and Yomi Kid, Ancient Sunshine, Wicked Trample, Healing Wind, Oberon's Prank) and Explosive Blast,
//! Gestalt Trap, Light to the Depths, Guidance of Yomi and Symmetry Rorschach on the way. The group has no Token.
//! Why the fuzz could not reach some: Echo Mirror needs a Defense Position monster attacked and not destroyed, Wise G3 and
//! Wise T3 the "Wisel Guard 3" / "Wisel Top 3" of batches 32 and 34 in the hand, Damage Boost a negation of its own damage
//! effect, Power Gravity another monster's ATK change at the right moment, Fairy Breath an "Ancient Fairy Dragon" on the
//! field; "promised, never done" on Wave of Ill Intent, Level Cannon, Magic Law and Spirit Wave Barrier is a label of the
//! script (Destroy, Position and To Grave declared on an effect that damages or negates), the effects themselves work.
//! Overrides: Double Tribute (`engine/overrides/c511000098.lua`: another card asking whether it can be activated, with
//! no summon to look at, crashed it), Yomi Kid (`c511001689.lua`) and Light to the Depths (`c511000834.lua`).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 2, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const MECHANICALCHASER: u16 = 4421; // Machine, Level 4, 1850/800 vanilla
const BLOCKER: u16 = 4422; // Machine, Level 4, 850/1800 vanilla
const GIGA_TECH_WOLF: u16 = 4418; // Machine, Level 4, 1200/1400 vanilla
const ELDEEN: u16 = 4536; // LIGHT, Level 3, 950/1000 vanilla
const AXE_OF_DESPAIR: u16 = 4310; // Equip Spell, +1000 ATK
const YAMI: u16 = 4341; // Field Spell: Fiend and Spellcaster monsters gain 200 ATK and DEF
const MAGIC_JAMMER: u16 = 4862; // Counter Trap: discard 1 card, negate a Spell Card's activation
const INFERNITY_DWARF: u16 = 8273; // Infernity, Level 2, 800/500
const INFERNITY_GUARDIAN: u16 = 8274; // Infernity, Level 4, 1200/1700
const ANCIENT_FAIRY_DRAGON: u16 = 8090; // Synchro, 2100/3000
const REGULUS: u16 = 8442; // Level 4, 1700/1000
const YOMI_SHIP: u16 = 5521; // "Yomi", Level 3, 800/1400

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card (shuffled, so only a Deck of a
/// single card gives a known draw): team 0's has `bottom` added for a card an effect fetches from the Deck, and `extra` is
/// team 0's Extra Deck.
fn duel(seed: u64, lp: u32, fillers: [u16; 2], bottom: &[u16], extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(fillers[0], 40 - bottom.len())).collect();
    let theirs = [fillers[1]; 40];
    let decks = [(&mine[..], extra), (&theirs[..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of Feral Imps.
fn field(spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [FILLER, FILLER], &[], &[], spawns) }

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

fn defense_of(s: &Scenario, team: usize, cid: u16) -> Option<i64> {
    s.monster(team, cid)?["defense"].as_i64()
}

// ---------------------------------------------------------------------------------------------------------------
// Echo Mirror
// ---------------------------------------------------------------------------------------------------------------

/// Echo Mirror: "During the turn a Defense Position monster you control was attacked, but not destroyed by battle: Draw 1
/// card and reveal it. If that card is a monster, inflict damage to your opponent equal to that card's Level x300." Team 1's
/// Thunder Kid (700) attacks team 0's Dark Blade in Defense Position (DEF 1500): nothing is destroyed. Team 0's Echo Mirror
/// then draws the top card of its Deck: a Feral Imp (Level 4) hurts team 1 for 1200, where a Spell (Dian Keto) hurts nobody
/// (team 1 lost 800 to its own attack: the attacker pays the difference, 1500 - 700).
#[test]
fn echo_mirror_draws_and_inflicts_the_level_times_300_of_a_monster() {
    let [echo, kid_attacker] = [card("Echo Mirror"), THUNDER_KID];
    for (top, damage) in [(FILLER, 1200), (DIAN_KETO, 0)] {  // team 0's whole Deck is of that card
        let spawns = [set(0, echo), mon_def(0, DARK_BLADE), mon(1, kid_attacker)];
        let Some(mut s) = duel(7, 8000, [top, FILLER], &[], &[], &spawns) else { return };
        let hand_before = s.state()[0]["hand"].as_array().unwrap().len();
        let top_before = s.count(0, "hand", top);
        s.run_both([&[Chain(echo)], &[Attack(kid_attacker)]]).unwrap();
        clean(&s);
        assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "Dark Blade survived the attack: {}", s.context());
        assert_eq!(s.count(0, "grave", echo), 1, "Echo Mirror was activated: {}", s.context());
        assert_eq!(s.state()[0]["hand"].as_array().unwrap().len(), hand_before + 1, "one card drawn: {}", s.context());
        assert_eq!(s.count(0, "hand", top), top_before + 1, "it was the top card: {}", s.context());
        assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 800 - damage), "top card {top}: {}", s.context());
    }
}

/// Echo Mirror: the turn "a Defense Position monster you control was attacked": with no attack it is not offered.
#[test]
fn echo_mirror_needs_a_defense_monster_attacked_and_not_destroyed() {
    let echo = card("Echo Mirror");
    let Some(mut s) = field(&[set(0, echo), mon_def(0, DARK_BLADE), mon(1, THUNDER_KID)]) else { return };
    let err = s.run_both([&[Chain(echo)], &[EndTurn]]).expect_err("nothing was attacked");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    // A Defense Position monster destroyed by the attack does not count either (Warwolf 2000 beats Blade's DEF 1500).
    let Some(mut s) = field(&[set(0, echo), mon_def(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    let err = s.run_both([&[Chain(echo)], &[Attack(WARWOLF)]]).expect_err("the defender was destroyed");
    assert!(err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Explosive Blast
// ---------------------------------------------------------------------------------------------------------------

/// Explosive Blast: "During your turn: you can target up to 3 Machine-Type monsters you control that have not battled this
/// turn; Destroy those monster(s), then inflict 400 damage to your opponent for each monster destroyed." Three Machines are
/// destroyed (the Warrior beside them is not) and the opponent takes 1200.
#[test]
fn explosive_blast_destroys_up_to_three_machines_and_inflicts_400_each() {
    let blast = card("Explosive Blast");
    let machines = [MECHANICALCHASER, BLOCKER, GIGA_TECH_WOLF];
    let mut spawns = vec![set(0, blast), mon(0, DARK_BLADE)];
    spawns.extend(machines.iter().map(|&m| mon(0, m)));
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&machines.map(|m| (0, m)));
    s.prefer_most(true);
    s.run_both([&[EndTurn, Activate(blast)], &[EndTurn]]).unwrap();
    clean(&s);
    for m in machines {
        assert_eq!(s.count(0, "grave", m), 1, "{m} was destroyed: {}", s.context());
    }
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "not a Machine: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1200, "{}", s.context());
}

/// Explosive Blast: a Machine that battled this turn cannot be the target. Mechanicalchaser (1850) destroys team 1's Dark
/// Blade in battle; in Main Phase 2 the Spell destroys only Blocker, which did not battle: 400 damage.
#[test]
fn explosive_blast_leaves_a_machine_that_battled() {
    let blast = card("Explosive Blast");
    let spawns = [set(0, blast), mon(0, MECHANICALCHASER), mon(0, BLOCKER), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, BLOCKER)]);
    s.prefer_most(true);
    s.run_both([&[EndTurn, Attack(MECHANICALCHASER), Activate(blast)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "the battle: {}", s.context());
    assert_eq!((s.count(0, "mzone", MECHANICALCHASER), s.count(0, "grave", BLOCKER)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 50 - 400, "50 from the battle (1850 against 1800), 400 from the Spell: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Infinity Guard Formation
// ---------------------------------------------------------------------------------------------------------------

/// Infinity Guard Formation: "Activate only when an effect is activated that would destroy a card(s) you control. Negate the
/// effects of all face-up Effect Monsters you control this turn, then negate that card effect." Team 1's Dark Hole would
/// destroy team 0's Dark Blade (and its own Warwolf): the Formation negates it, so every monster stays and Dark Hole is in
/// the Graveyard. Without the Formation both monsters are destroyed.
#[test]
fn infinity_guard_formation_negates_an_effect_that_would_destroy_my_cards() {
    let formation = card("Infinity Guard Formation");
    let spawns = [set(0, formation), mon(0, DARK_BLADE), hand(1, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(formation)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", formation), 1, "the Formation was activated: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(1, "mzone", WARWOLF)), (1, 1), "Dark Hole was negated: {}", s.context());
    let spawns = [mon(0, DARK_BLADE), hand(1, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(1, "grave", WARWOLF)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Wise G3 and Wise T3
// ---------------------------------------------------------------------------------------------------------------

/// Wise G3: "Send 1 face-up "Wisel Guard" monster you control to the Graveyard. Special Summon 1 "Wisel Guard 3" from your
/// hand." Wisel Guard (and the Meklord Emperor that keeps it from destroying itself) are on the field in turn 3: the Guard
/// goes to the Graveyard as the cost and Wisel Guard 3 comes from the hand.
#[test]
fn wise_g3_sends_a_wisel_guard_and_summons_wisel_guard_3_from_the_hand() {
    let [wise, guard, guard3, emperor] = ["Wise G3", "Wisel Guard (TF6)", "Wisel Guard 3 (TF6)", "Meklord Emperor Granel ∞"].map(card);
    let Some(mut s) = field(&[mon(0, emperor), set(0, wise), mon(0, guard), hand(0, guard3)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(wise)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", guard), s.count(0, "grave", wise)), (1, 1), "the cost: {}", s.context());
    assert_eq!((s.count(0, "mzone", guard3), s.count(0, "hand", guard3)), (1, 0), "Wisel Guard 3 came from the hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", emperor), 1, "{}", s.context());
}

/// Wise G3: without a face-up "Wisel Guard" to send, or without "Wisel Guard 3" in the hand, it is not offered.
#[test]
fn wise_g3_needs_a_wisel_guard_and_wisel_guard_3() {
    let [wise, guard, guard3, emperor] = ["Wise G3", "Wisel Guard (TF6)", "Wisel Guard 3 (TF6)", "Meklord Emperor Granel ∞"].map(card);
    for spawns in [vec![mon(0, emperor), set(0, wise), hand(0, guard3)], vec![mon(0, emperor), set(0, wise), mon(0, guard)]] {
        let Some(mut s) = field(&spawns) else { return };
        let err = s.run_both([&[EndTurn, Activate(wise)], &[EndTurn]]).expect_err("not possible");
        assert!(err.contains("never became possible"), "{err}");
        clean(&s);
    }
}

/// Wise T3: "Send 1 face-up "Wisel Top" monster you control to the Graveyard. Special Summon 1 "Wisel Top 3" from your hand."
#[test]
fn wise_t3_sends_a_wisel_top_and_summons_wisel_top_3_from_the_hand() {
    let [wise, top, top3, emperor] = ["Wise T3", "Wisel Top (TF6)", "Wisel Top 3 (TF6)", "Meklord Emperor Granel ∞"].map(card);
    let Some(mut s) = field(&[mon(0, emperor), set(0, wise), mon(0, top), hand(0, top3)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(wise)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", top), s.count(0, "grave", wise)), (1, 1), "the cost: {}", s.context());
    assert_eq!((s.count(0, "mzone", top3), s.count(0, "hand", top3)), (1, 0), "Wisel Top 3 came from the hand: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Gestalt Trap and Immortal Homeostasis
// ---------------------------------------------------------------------------------------------------------------

/// Gestalt Trap: "Equip this card to a monster your opponent controls. The equipped monster's ATK/DEF become 0, and its
/// effect(s) is negated." Warwolf (2000/100) is equipped in turn 3: 0/0, and the Trap stays in team 0's Spell & Trap Zone.
#[test]
fn gestalt_trap_equips_to_the_opponents_monster_and_makes_it_0() {
    let gestalt = card("Gestalt Trap");
    let Some(mut s) = field(&[set(0, gestalt), mon(1, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn, Activate(gestalt)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(1, WARWOLF), defense_of(&s, 1, WARWOLF)), (Some(0), Some(0)), "{}", s.context());
    assert_eq!((s.attack_of(1, THUNDER_KID), defense_of(&s, 1, THUNDER_KID)), (Some(700), Some(600)), "only the target: {}", s.context());
    assert_eq!(s.count(0, "szone", gestalt), 1, "equipped, so on the field: {}", s.context());
}

/// Immortal Homeostasis: "The equipped monster cannot be destroyed by battle." Dark Blade (1800) wears it; team 1's Dark
/// Magician (2500) attacks it in turn 2: Dark Blade stays and team 0 takes the 700.
#[test]
fn immortal_homeostasis_keeps_the_equipped_monster_from_being_destroyed_by_battle() {
    let homeostasis = card("Immortal Homeostasis");
    let Some(mut s) = field(&[hand(0, homeostasis), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(homeostasis)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", homeostasis), 1, "equipped: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "{}", s.context());
    // The control: without it Dark Blade is destroyed.
    let Some(mut s) = field(&[mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
}

/// Immortal Homeostasis: "During the equipped monster's controller's Standby Phase, if the equipped monster's ATK is
/// different from its original ATK, its controller takes 300 damage." With the Axe of Despair (+1000 ATK) beside it, team 0
/// takes 300 at turn 3's Standby Phase; with its ATK unchanged it takes nothing.
#[test]
fn immortal_homeostasis_hurts_its_controller_when_the_atk_differs_from_the_original() {
    let homeostasis = card("Immortal Homeostasis");
    let Some(mut s) = field(&[hand(0, homeostasis), hand(0, AXE_OF_DESPAIR), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(homeostasis), Activate(AXE_OF_DESPAIR), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(2800), "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 300, "the Standby Phase damage: {}", s.context());
    let Some(mut s) = field(&[hand(0, homeostasis), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(homeostasis), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, DARK_BLADE), s.lp(0)), (Some(1800), 8000), "the ATK is the original one: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Light to the Depths and Symmetry Rorschach
// ---------------------------------------------------------------------------------------------------------------

/// Light to the Depths: "When your opponent Summons a LIGHT monster: Your opponent sends the top 5 cards of their Deck to the
/// Graveyard, then, they draw 1 card and reveal it. During the End Phase, if that card or a card of the same name is not
/// played during this turn, they take 2000 damage." Team 1 Summons Eldeen (LIGHT) in turn 2: five Feral Imps go to its
/// Graveyard, one is drawn (and shown to team 0), and at the End Phase team 1, not team 0, takes the 2000.
#[test]
fn light_to_the_depths_mills_five_draws_one_and_hurts_the_opponent_at_the_end_phase() {
    let depths = card("Light to the Depths");
    let Some(mut s) = field(&[set(0, depths), hand(1, ELDEEN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(depths)], &[Summon(ELDEEN), EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", depths), 1, "activated: {}", s.context());
    assert!(s.count(1, "grave", FILLER) >= 5, "the top 5 cards: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2000), "the opponent takes the 2000: {}", s.context());
}

/// Symmetry Rorschach: "If this card destroys an opponent's monster by battle: look at the top card of your opponent's Deck."
/// Rorschach (1200) destroys Thunder Kid in turn 3 and team 0 is shown team 1's top card.
#[test]
fn symmetry_rorschach_shows_the_top_card_of_the_opponents_deck_after_destroying_a_monster() {
    let rorschach = card("Symmetry Rorschach");
    let Some(mut s) = field(&[mon(0, rorschach), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    // The attack by hand, to see every step's messages.
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
    while s.prompt()["type"] != msg::SELECT_BATTLECMD {
        s.pass().unwrap();
    }
    let i = s.prompt()["attacks"].as_array().unwrap().iter().position(|a| a["cid"] == rorschach).unwrap_or_else(|| panic!("{}", s.prompt()));
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::battle::ATTACK, "index": i})).unwrap();
    let mut shown = vec![];
    for _ in 0..20 {
        shown.extend(s.messages().iter().filter(|m| m["type"] == msg::CONFIRM_DECKTOP || m["type"] == msg::CONFIRM_CARDS).cloned());
        if s.prompt()["type"] == msg::SELECT_BATTLECMD && s.prompt()["attacks"].as_array().is_some_and(Vec::is_empty) {
            break;
        }
        s.pass().unwrap();
    }
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "destroyed in battle: {}", s.context());
    assert_eq!(shown.len(), 1, "one card shown: {shown:?}");
    assert_eq!(shown[0]["player"], 0, "to team 0: {shown:?}");
}

/// From the Battle Phase prompt a plan stopped at, on to the Main Phase 2 prompt.
fn to_main2(s: &mut Scenario) {
    assert_eq!(s.prompt()["type"], msg::SELECT_BATTLECMD, "{}", s.context());
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::battle::TO_M2, "index": null})).unwrap();
    while s.prompt()["type"] != msg::SELECT_IDLECMD {
        s.pass().unwrap(); // the chain windows of the phase change
    }
}

fn deck_size(s: &Scenario, team: usize) -> usize {
    match &s.state()[team]["deck"] {
        serde_json::Value::Array(a) => a.len(),
        v => v.as_u64().unwrap_or(0) as usize,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Spirit Contamination
// ---------------------------------------------------------------------------------------------------------------

/// Spirit Contamination: "During the Main Phase 2 of the turn a monster you control was destroyed: Target 1 Spell/Trap your
/// opponent controls; destroy it." Dark Blade (1800) is destroyed attacking Warwolf (2000) in turn 3, and in Main Phase 2 the
/// Spell destroys team 1's Set Dian Keto.
#[test]
fn spirit_contamination_destroys_a_spell_or_trap_after_a_monster_of_mine_was_destroyed() {
    let contamination = card("Spirit Contamination");
    let spawns = [hand(0, contamination), mon(0, DARK_BLADE), mon(1, WARWOLF), set(1, DIAN_KETO)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(DARK_BLADE), Activate(contamination)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the battle: {}", s.context());
    assert_eq!(s.count(1, "grave", DIAN_KETO), 1, "the Set Spell was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", contamination), 1, "{}", s.context());
    // With no monster destroyed that turn it is not offered.
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(contamination)], &[EndTurn]]).expect_err("no monster of mine was destroyed");
    assert!(err.contains("never became possible"), "{err}");
}

/// Spirit Contamination: "During your Draw Phase, if this card is in your GY, instead of conducting your normal draw: You
/// can add this card from your GY to your hand." In the Graveyard at turn 3's Draw Phase it comes to the hand and nothing is
/// drawn (the Deck keeps its size); with no such card in the Graveyard the Draw Phase draws.
#[test]
fn spirit_contamination_replaces_the_draw_by_returning_from_the_graveyard() {
    let contamination = card("Spirit Contamination");
    let Some(mut s) = field(&[]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, EndTurn], &[EndTurn]]).unwrap();
    let normal = deck_size(&s, 0);
    let Some(mut s) = field(&[grave(0, contamination)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "hand", contamination), s.count(0, "grave", contamination)), (1, 0), "{}", s.context());
    assert_eq!(deck_size(&s, 0), normal + 1, "no card was drawn at the Draw Phase, the normal draw takes one: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Wave of Ill Intent
// ---------------------------------------------------------------------------------------------------------------

/// Wave of Ill Intent: "Each time a monster you control is destroyed by battle, inflict 300 damage to your opponent." In
/// turn 3 Dark Blade (1800) is destroyed by Warwolf (2000): team 0 takes 200 in the battle and team 1 takes 300. The
/// script declares the Destroy category on the effect, which is why the fuzz called it "never done"; it only inflicts damage.
#[test]
fn wave_of_ill_intent_inflicts_300_when_a_monster_of_mine_is_destroyed_by_battle() {
    let wave = card("Wave of Ill Intent");
    let Some(mut s) = field(&[hand(0, wave), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(wave), EndTurn, Attack(DARK_BLADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 200, 8000 - 300), "{}", s.context());
    assert_eq!(s.count(0, "szone", wave), 1, "it stays on the field: {}", s.context());
}

/// Wave of Ill Intent: only "a monster you control": team 1's monster destroyed by battle costs team 1 the battle damage and
/// nothing more.
#[test]
fn wave_of_ill_intent_ignores_the_opponents_monsters() {
    let wave = card("Wave of Ill Intent");
    let Some(mut s) = field(&[hand(0, wave), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(wave), EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 200), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Infernity Burst and Damage Boost
// ---------------------------------------------------------------------------------------------------------------

/// Infernity Burst: "Inflict 800 damage to your opponent for each "Infernity" monster you control." Two Infernity monsters
/// (and a Warrior that is not one) inflict 1600; with none it is not offered.
#[test]
fn infernity_burst_inflicts_800_for_each_infernity_monster() {
    let burst = card("Infernity Burst");
    let spawns = [hand(0, burst), mon(0, INFERNITY_DWARF), mon(0, INFERNITY_GUARDIAN), mon(0, DARK_BLADE)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(burst)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1600), "{}", s.context());
    assert_eq!(s.count(0, "grave", burst), 1, "{}", s.context());
    let Some(mut s) = field(&[hand(0, burst), mon(0, DARK_BLADE)]) else { return };
    let err = s.run(&[Activate(burst)]).expect_err("no Infernity monster");
    assert!(err.contains("never became possible"), "{err}");
}

/// Damage Boost: "When your opponent activates a card or effect that would negate the effect of a card you control that
/// inflicts damage to your opponent: Negate that effect, and if you do, destroy that card, then, double the damage inflicted
/// to your opponent." Team 0's Infernity Burst (two Infernity monsters: 1600) is answered by team 1's Magic Jammer; Damage
/// Boost negates the Jammer and destroys it, and the Burst inflicts 3200. Without Damage Boost the Burst is negated.
#[test]
fn damage_boost_negates_the_negation_and_doubles_the_damage() {
    let [boost, burst] = ["Damage Boost", "Infernity Burst"].map(card);
    let spawns = [set(0, boost), hand(0, burst), mon(0, INFERNITY_DWARF), mon(0, INFERNITY_GUARDIAN), set(1, MAGIC_JAMMER)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(burst), Chain(boost)], &[Chain(MAGIC_JAMMER)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", boost), 1, "Damage Boost was activated: {}", s.context());
    assert_eq!(s.count(1, "grave", MAGIC_JAMMER), 1, "Magic Jammer was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", burst), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 3200), "double the 1600: {}", s.context());
    let spawns = [hand(0, burst), mon(0, INFERNITY_DWARF), mon(0, INFERNITY_GUARDIAN), set(1, MAGIC_JAMMER)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(burst)], &[Chain(MAGIC_JAMMER)]]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "the Burst was negated: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Double Tribute (engine/overrides/c511000098.lua)
// ---------------------------------------------------------------------------------------------------------------

/// Double Tribute: "When a monster is Normal or Special Summoned to your side of the field: Destroy that monster and 1
/// monster your opponent controls." Team 0 Normal Summons Dark Blade in turn 3 and answers with the Trap: Dark Blade and
/// team 1's Warwolf are both destroyed. With no monster on the other side it is not offered.
#[test]
fn double_tribute_destroys_the_summoned_monster_and_one_of_the_opponents() {
    let tribute = card("Double Tribute");
    let spawns = [set(0, tribute), hand(0, DARK_BLADE), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER), (1, FILLER), (1, WARWOLF)]);
    s.run_both([&[EndTurn, Summon(DARK_BLADE), Chain(tribute)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the summoned monster: {}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", THUNDER_KID)), (1, 1), "one monster of the opponent: {}", s.context());
    assert_eq!(s.count(0, "grave", tribute), 1, "{}", s.context());
    let Some(mut s) = field(&[set(0, tribute), hand(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Summon(DARK_BLADE), Chain(tribute)], &[EndTurn]]).expect_err("nothing to destroy");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Level Cannon
// ---------------------------------------------------------------------------------------------------------------

/// Level Cannon: "If a monster(s) is Summoned: The player who Summoned it takes damage equal to its Level x200." Team 0
/// Summons Dark Blade (Level 4) after activating it in turn 3 and takes 800; team 1 Summons Thunder Kid (Level 2) in turn 4
/// and takes 400. (The script declares the Position category on the effect, which is all the fuzz's "never done" is.)
#[test]
fn level_cannon_hurts_the_player_who_summoned_by_the_level_times_200() {
    let cannon = card("Level Cannon");
    let Some(mut s) = field(&[set(0, cannon), hand(0, DARK_BLADE), hand(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(cannon), Summon(DARK_BLADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 800, 8000), "{}", s.context());
    s.run_both([&[EndTurn], &[Summon(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 800, 8000 - 400), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Power Gravity
// ---------------------------------------------------------------------------------------------------------------

/// Power Gravity: "Equip this card to a face-up monster when a monster your opponent controls would gain/lose ATK. The ATK
/// of your opponent's monster becomes 0, instead. The equipped monster gains ATK equal to the amount of ATK your opponent's
/// monster would have gained/lost." Team 1's Yami (Spellcaster and Fiend monsters gain 200 ATK) raises Dark Magician from 2500
/// to 2700; team 0's Power Gravity equips to Dark Blade, which gains the 200 (2000), and Dark Magician's ATK becomes 0.
#[test]
fn power_gravity_moves_an_atk_gain_of_the_opponents_monster_to_the_equipped_one() {
    let gravity = card("Power Gravity");
    let spawns = [set(0, gravity), mon(0, DARK_BLADE), mon(1, DARK_MAGICIAN), hand(1, YAMI)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER), (1, FILLER), (0, DARK_BLADE)]);
    s.run_both([&[Chain(gravity)], &[Activate(YAMI)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", gravity), 1, "equipped: {}", s.context());
    assert_eq!(s.attack_of(0, DARK_BLADE), Some(1800 + 200), "{}", s.context());
    assert_eq!(s.attack_of(1, DARK_MAGICIAN), Some(0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Magic Law and Spirit Wave Barrier
// ---------------------------------------------------------------------------------------------------------------

/// Magic Law: "You can send 3 monster with the same name from your hand to the Graveyard, and if you do, your opponent
/// cannot activate Spell Cards, until your next End Phase." Three Feral Imps are discarded by its effect in turn 1 and in
/// turn 2 team 1 is not offered its Dian Keto; with no Magic Law it is.
#[test]
fn magic_law_sends_three_monsters_of_one_name_and_locks_the_opponents_spells() {
    let law = card("Magic Law");
    let on_team1_idle = |p: &serde_json::Value| p["type"] == msg::SELECT_IDLECMD && p["player"] == 1;
    let Some(mut s) = field(&[hand(0, law), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    let imps = s.count(0, "grave", FILLER);
    s.run_to(&[Activate(law), Activate(law), EndTurn], on_team1_idle).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FILLER), imps + 3, "three Feral Imps: {}", s.context());
    assert_eq!(s.count(0, "szone", law), 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&DIAN_KETO.into()), "{:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_to(&[EndTurn], on_team1_idle).unwrap();
    assert!(offered(&s, "activates").contains(&DIAN_KETO.into()), "{:?}", offered(&s, "activates"));
}

/// Spirit Wave Barrier: "When an opponent's monster declares a direct attack, negate the attack and destroy that monster.
/// Destroy this card during your 3rd End Phase after activation." Warwolf attacks directly in turn 2: no damage, and
/// Warwolf is destroyed. The Spell stays through the End Phases of turns 1 and 3 and is destroyed at the End Phase of turn 5.
#[test]
fn spirit_wave_barrier_negates_a_direct_attack_and_destroys_the_attacker() {
    let barrier = card("Spirit Wave Barrier");
    let Some(mut s) = field(&[hand(0, barrier), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(barrier)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the attacker was destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    // Turn 3's End Phase is the second.
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "szone", barrier), 1, "still there after turn 3's End Phase: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", barrier), s.count(0, "grave", barrier)), (0, 1), "the 3rd End Phase: {}", s.context());
}

/// Spirit Wave Barrier: an attack on a monster is not a direct attack: Dark Blade is attacked, so Warwolf is not destroyed.
#[test]
fn spirit_wave_barrier_leaves_an_attack_on_a_monster_alone() {
    let barrier = card("Spirit Wave Barrier");
    let Some(mut s) = field(&[hand(0, barrier), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(barrier)], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!((s.count(1, "mzone", WARWOLF), s.count(0, "grave", DARK_BLADE)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Fairy Breath and Ancient Sunshine
// ---------------------------------------------------------------------------------------------------------------

/// Fairy Breath: "If you control a face-up "Ancient Fairy Dragon": Target 1 face-up monster on the field; It gains 3000 DEF
/// until the End Phase." Dark Blade (DEF 1500) gains 3000 in turn 3 (4500), Ancient Fairy Dragon keeps its 3000, and the
/// gain is gone in turn 4. Without the Dragon it is not offered.
#[test]
fn fairy_breath_gives_3000_def_until_the_end_phase() {
    let breath = card("Fairy Breath");
    let Some(mut s) = field(&[set(0, breath), mon(0, ANCIENT_FAIRY_DRAGON), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[EndTurn, Activate(breath)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((defense_of(&s, 0, DARK_BLADE), defense_of(&s, 0, ANCIENT_FAIRY_DRAGON)), (Some(4500), Some(3000)), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(defense_of(&s, 0, DARK_BLADE), Some(1500), "until the End Phase: {}", s.context());
    let Some(mut s) = field(&[set(0, breath), mon(0, DARK_BLADE)]) else { return };
    let err = s.run_both([&[EndTurn, Activate(breath)], &[EndTurn]]).expect_err("no Ancient Fairy Dragon");
    assert!(err.contains("never became possible"), "{err}");
}

/// Ancient Sunshine: "Activate only if you control a face-up "Regulus". Banish 1 "Ancient Fairy Dragon" from your Extra
/// Deck; inflict 2100 damage to your opponent. Then, destroy all cards you control." Regulus and Dark Blade are destroyed
/// with the Trap, the Dragon is banished from the Extra Deck, and team 1 takes 2100.
#[test]
fn ancient_sunshine_banishes_the_dragon_hurts_for_2100_and_destroys_my_cards() {
    let sunshine = card("Ancient Sunshine");
    let spawns = [set(0, sunshine), mon(0, REGULUS), mon(0, DARK_BLADE)];
    let Some(mut s) = duel(7, 8000, [FILLER, FILLER], &[], &[ANCIENT_FAIRY_DRAGON], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(sunshine)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", ANCIENT_FAIRY_DRAGON), 1, "banished: {}", s.context());
    assert_eq!((s.count(0, "grave", REGULUS), s.count(0, "grave", DARK_BLADE), s.count(0, "grave", sunshine)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 2100), "{}", s.context());
    // Without Regulus it is not offered.
    let Some(mut s) = duel(7, 8000, [FILLER, FILLER], &[], &[ANCIENT_FAIRY_DRAGON], &[set(0, sunshine), mon(0, DARK_BLADE)]) else { return };
    let err = s.run_both([&[EndTurn, Activate(sunshine)], &[EndTurn]]).expect_err("no Regulus");
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Dodge Roll, Twinkle Wall, Wicked Trample, Healing Wind and Oberon's Prank
// ---------------------------------------------------------------------------------------------------------------

/// Dodge Roll: "The next damage the next player would take becomes 0." Team 1's Warwolf attacks directly in turn 2, Dodge
/// Roll is chained to the attack, and the 2000 is 0; the next attack, in turn 4, hurts: 6000.
#[test]
fn dodge_roll_makes_the_next_damage_0_once() {
    let dodge = card("Dodge Roll");
    let Some(mut s) = field(&[set(0, dodge), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(dodge)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", dodge), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the first damage was 0: {}", s.context());
    s.run_both([&[], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 6000, "only the next damage: {}", s.context());
}

/// Twinkle Wall: "When an opponent's monster declares an attack: Negate the attack, then, take damage equal to half of that
/// monster's ATK. Your opponent cannot activate Spell/Trap Cards until the End Phase." Warwolf (2000) attacks in turn 2:
/// the attack is negated, team 0 takes 1000, and team 1's Main Phase 2 does not offer its Dian Keto (it does without it).
#[test]
fn twinkle_wall_negates_the_attack_and_costs_half_its_atk_and_locks_spells_and_traps() {
    let wall = card("Twinkle Wall");
    let Some(mut s) = field(&[set(0, wall), mon(1, WARWOLF), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(wall)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", wall), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1000, 8000), "negated, and half the ATK: {}", s.context());
    to_main2(&mut s);
    assert!(!offered(&s, "activates").contains(&DIAN_KETO.into()), "{:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[mon(1, WARWOLF), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    to_main2(&mut s);
    assert!(offered(&s, "activates").contains(&DIAN_KETO.into()), "{:?}", offered(&s, "activates"));
}

/// Wicked Trample: "Double the battle damage your opponent takes, except from direct attacks." Warwolf (2000) beats Dark
/// Blade (1800) in turn 3: 200 doubled to 400; a direct attack takes the whole 2000.
#[test]
fn wicked_trample_doubles_battle_damage_except_from_direct_attacks() {
    let trample = card("Wicked Trample");
    let Some(mut s) = field(&[set(0, trample), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(trample), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 400, "{}", s.context());
    let Some(mut s) = field(&[set(0, trample), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(trample), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(1), 8000 - 2000, "{}", s.context());
}

/// Healing Wind: "Gain 200 LP for each monster on the field." Three monsters (one of mine, two of the opponent's) give 600.
#[test]
fn healing_wind_gains_200_for_each_monster_on_the_field() {
    let wind = card("Healing Wind");
    let Some(mut s) = field(&[hand(0, wind), mon(0, DARK_BLADE), mon(1, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(wind)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8600, 8000), "{}", s.context());
    let Some(mut s) = field(&[hand(0, wind)]) else { return };
    let err = s.run(&[Activate(wind)]).expect_err("no monster on the field");
    assert!(err.contains("never became possible"), "{err}");
}

/// Oberon's Prank: "When a player would gain LP by a card effect: Negate that effect, and both players take damage equal
/// to the amount of LP that player(s) would have gained." Team 1's Dian Keto (1000) is answered: both players take 1000.
#[test]
fn oberons_prank_turns_a_gain_into_damage_for_both_players() {
    let prank = card("Oberon's Prank");
    let Some(mut s) = field(&[set(0, prank), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(prank)], &[Activate(DIAN_KETO)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", prank), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (7000, 7000), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Yomi Boat Watchman, Yomi Kid and Guidance of Yomi
// ---------------------------------------------------------------------------------------------------------------

/// Yomi Boat Watchman: "When this card is destroyed and sent to the Graveyard: inflict 800 damage to your opponent. If you
/// controlled this card when it was destroyed and sent to the Graveyard, then while this card is in the Graveyard, it gains
/// this effect. When an opponent's monster declares an attack: you can target 1 "Yomi" monster in your Graveyard; Special
/// Summon it." Thunder Kid destroys the Watchman (0 ATK) in turn 2: team 1 takes 800. In turn 4 Thunder Kid attacks again
/// and the Watchman's Graveyard effect brings Yomi Ship back.
#[test]
fn yomi_boat_watchman_burns_when_destroyed_and_summons_a_yomi_when_attacked() {
    let watchman = card("Yomi Boat Watchman");
    let Some(mut s) = field(&[mon(0, watchman), grave(0, YOMI_SHIP), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, YOMI_SHIP)]);
    s.run_both([&[], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", watchman), 1, "destroyed in battle: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 700, 8000 - 800), "{}", s.context());
    s.run_both([&[Chain(watchman)], &[EndTurn, Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", YOMI_SHIP), 1, "Yomi Ship came back: {}", s.context());
}

/// Yomi Boat Watchman: "When this card is destroyed and sent to the Graveyard: inflict 800 damage to your opponent", whatever
/// destroyed it: my Dark Hole destroys the Watchman and team 1's Warwolf, and team 1 takes 800.
#[test]
fn yomi_boat_watchman_burns_when_destroyed_by_an_effect() {
    let watchman = card("Yomi Boat Watchman");
    let Some(mut s) = field(&[hand(0, DARK_HOLE), mon(0, watchman), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", watchman), s.count(1, "grave", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
}

/// Yomi Kid: "If this card is destroyed and sent to the Graveyard: Inflict 800 damage to your opponent. While this card is
/// in your Graveyard, and was destroyed and sent there while you controlled it, destroy all monsters you control." Thunder
/// Kid destroys it (0 ATK) in turn 2: team 1 takes 800, Dark Blade is destroyed with it at once, and Thunder Kid, Summoned
/// in turn 3, is destroyed too.
#[test]
fn yomi_kid_burns_and_destroys_every_monster_i_control_while_it_is_in_the_graveyard() {
    let kid = card("Yomi Kid");
    let Some(mut s) = field(&[mon(0, kid), mon(0, DARK_BLADE), mon(1, THUNDER_KID), hand(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(THUNDER_KID)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", kid), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "destroyed with it: {}", s.context());
    s.run_both([&[Summon(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(0, "grave", WARWOLF)), (0, 1), "{}", s.context());
}

/// Guidance of Yomi: "If there are exactly 2 "Yomi" monsters in your Graveyard: Special Summon 1 "Yomi" monster from your
/// Deck." Yomi Ship and Treeborn Frog are in the Graveyard in turn 3 and the Watchman comes from the Deck; with three, it
/// is not offered.
#[test]
fn guidance_of_yomi_summons_a_yomi_from_the_deck_with_exactly_two_in_the_graveyard() {
    let [guidance, watchman, frog] = ["Guidance of Yomi", "Yomi Boat Watchman", "Treeborn Frog"].map(card);
    let spawns = [set(0, guidance), grave(0, YOMI_SHIP), grave(0, frog)];
    let Some(mut s) = duel(7, 8000, [FILLER, FILLER], &[watchman], &[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(guidance)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", watchman), 1, "{}", s.context());
    let spawns = [set(0, guidance), grave(0, YOMI_SHIP), grave(0, frog), grave(0, watchman)];
    let Some(mut s) = duel(7, 8000, [FILLER, FILLER], &[watchman], &[], &spawns) else { return };
    let err = s.run_both([&[EndTurn, Activate(guidance)], &[EndTurn]]).expect_err("three Yomi monsters");
    assert!(err.contains("never became possible"), "{err}");
}

/// Yomi Kid with Insect Garden ("you can give control of any Level 4 or lower Insect-Type monster on your field to your
/// opponent"), batch 18's fuzz: Insect Garden's Adjust effect (team 1's) hands a monster to team 0 while Yomi Kid's own
/// Adjust effect from team 0's Graveyard destroys every monster team 0 controls, in the middle of the control change.
/// Thunder Kid destroys the Kid in turn 2 and team 1 Summons Korogashi (an Insect, Level 2) in Main Phase 2: Insect Garden
/// gives it to team 0 and Yomi Kid destroys it (at the next Adjust), without the engine reporting an error. The override
/// (`engine/overrides/c511001689.lua`) is a precaution: this crash did not reproduce here.
#[test]
fn yomi_kid_beside_insect_garden_does_not_crash_the_engine() {
    let [kid, garden] = ["Yomi Kid", "Insect Garden"].map(card);
    let korogashi = card("Korogashi");
    let Some(mut s) = field(&[spell(1, garden), mon(0, kid), mon(1, THUNDER_KID), hand(1, korogashi)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(THUNDER_KID), Summon(korogashi)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", kid), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", korogashi) + s.count(1, "mzone", korogashi), 0, "destroyed by the Kid: {}", s.context());
    assert_eq!((s.count(0, "grave", korogashi), s.count(1, "grave", korogashi)), (0, 1), "in its owner's Graveyard: {}", s.context());
}
