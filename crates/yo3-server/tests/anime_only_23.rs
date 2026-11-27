//! Anime-only cards, group 23 (rows 5959-5992 of engine/added/cards.tsv): Jack Atlas' and Akiza Izinski's cards. These cards
//! exist only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes
//! through the real core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the
//! anime ones. Every Trap answering the opponent is Set first, and neither player has a Battle Phase in turn 1, so team 0's
//! attacks wait for turn 3.
//! Covered: every card of the batch, the ones the per-card fuzz flagged (it could not reach most of them: a Trap that answers
//! an attack, an ATK change, a destroyed Tuner or a Synchro Monster with its materials needs a scene no random duel
//! builds), Dimension Switch (its "first card unchecked" note: its target destroyed in answer does not stop the effect on a
//! Lua error, so it needs no override), Illusion Destruction (its Pendulum note is only a filter that skips the Pendulum
//! Zone: harmless, there are none), the Ivy Tokens made by Cursed Ivy and Doom Petal Countdown (the fuzz's "never DAMAGE": it
//! merges the Activate and the End Phase effect, and no Plant was in the Graveyard in its duels), the signature cards (Burning
//! Rebirth, Cursed Ivy, Doom Petal Countdown, Rose Flame, Crystal Rose Spirit, Rose Curse, Tuner's Mind) and the seeded
//! sample of the rest (Burning Strike, Compulsory Recoil Device, Rose Blizzard, Rose Curse, Trap Pause).
//! Conquest of the Supreme Ruler has an override (`engine/overrides/c511002734.lua`: ProjectIgnis' script checked that the card
//! was still related to its effect after the effect's own cost had sent it to the Graveyard, so no Summon was ever negated).
//! A Synchro Monster put straight into a Graveyard has not been Summoned properly and cannot be revived by a card that
//! Special Summons it (Burning Rebirth, Rage Resynchro): those scenes Synchro Summon it first, as a game does. The game's
//! own Tokens (the Ivy Token) have no Konami id in the engine's state, so the test finds them by name.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, Level 3, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DARK_HOLE: u16 = 4342;
const KREBONS: u16 = 7716; // a Level 2 Tuner, 1200/400, an effect only when it is attacked
const JUNK_SYNCHRON: u16 = 7687; // a Level 3 Tuner, 1300/500, an effect only when Normal Summoned
const QUILLBOLT: u16 = 7701; // a Level 2 non-Tuner, 800/800, an effect only from the Graveyard
const JUNK_WARRIOR: u16 = 7696; // Synchro, Level 5, 2300/1300: "Junk Synchron" + a non-Tuner
const RED_DRAGON: u16 = 7735; // Red Dragon Archfiend, Synchro, Level 8, 3000/2000
const STARDUST: u16 = 7734; // Stardust Dragon, Synchro, Level 8, 2500/2000

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }
fn banished(team: u32, cid: u16) -> Spawn { Spawn { team, place: Banished, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card, with the Extra Decks given.
fn duel(seed: u64, lp: u32, extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], extra[0]), (&[FILLER; 40][..], extra[1])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, [&[], &[]], spawns) }

/// A duel with the Extra Decks given: `extra[team]`.
fn field_extra(extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, extra, spawns) }

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

// ---------------------------------------------------------------------------------------------------------------
// Break Tune, Power Pressure, Fiend Cannon, Ray of Hope, Ground Capture, Rose Blizzard, Crystal Veil: the battle Traps
// ---------------------------------------------------------------------------------------------------------------

/// Break Tune: "When an opponent's monster declares an attack: Special Summon 1 Tuner from your hand, then, negate the
/// attack." Team 1's Warwolf attacks in turn 2: Krebons comes out of team 0's hand and no damage is taken, where without
/// the Trap team 0 takes 2000.
#[test]
fn break_tune_summons_a_tuner_from_the_hand_and_negates_the_attack() {
    let [tune, krebons] = ["Break Tune", "Krebons"].map(card);
    let Some(mut s) = field(&[set(0, tune), hand(0, krebons), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(tune)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", krebons), 1, "Krebons came from the hand: {}", s.context());
    assert_eq!(s.count(0, "grave", tune), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "the attack was negated: {}", s.context());
    // The control: without the Trap Warwolf hits for 2000.
    let Some(mut s) = field(&[hand(0, krebons), mon(1, WARWOLF)]) else { return };
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 6000, "{}", s.context());
}

/// Break Tune: "Special Summon 1 Tuner from your hand": a hand without a Tuner leaves it unplayable, and the attack hits.
#[test]
fn break_tune_needs_a_tuner_in_the_hand() {
    let tune = card("Break Tune");
    let Some(mut s) = field(&[set(0, tune), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 6000, "no Tuner, no Break Tune: {}", s.context());
    assert_eq!(s.count(0, "szone", tune), 1, "{}", s.context());
}

/// Power Pressure: "When a monster you control has its attack negated: Inflict 1000 damage to your opponent." Team 0's
/// Warwolf attacks in turn 3 and team 1's Negate Attack stops it: team 1 takes 1000 from Power Pressure and nothing else.
#[test]
fn power_pressure_inflicts_1000_when_my_attack_is_negated() {
    let [pressure, negate] = ["Power Pressure", "Negate Attack"].map(card);
    let Some(mut s) = field(&[set(0, pressure), mon(0, WARWOLF), set(1, negate), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF), Chain(pressure)], &[Chain(negate)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", negate), 1, "Negate Attack was used: {}", s.context());
    assert_eq!(s.count(0, "grave", pressure), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 7000), "{}", s.context());
}

/// Fiend Cannon: "Activate only when a monster your opponent controls is attacked and not destroyed by battle: Inflict
/// 500 damage to your opponent." Warwolf (2000) attacks Dark Magician in Defense (DEF 2100) in turn 3: neither is
/// destroyed, team 0 takes 100 from the battle and team 1 takes 500 from the Trap.
#[test]
fn fiend_cannon_inflicts_500_when_the_attacked_monster_survives() {
    let cannon = card("Fiend Cannon");
    let Some(mut s) = field(&[set(0, cannon), mon(0, WARWOLF), mon_def(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF), Chain(cannon)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", cannon), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 100, 8000 - 500), "{}", s.context());
}

/// Fiend Cannon, the control: an attacked monster that is destroyed by the battle leaves the Trap unplayable.
#[test]
fn fiend_cannon_is_not_offered_when_the_attacked_monster_is_destroyed() {
    let cannon = card("Fiend Cannon");
    let Some(mut s) = field(&[set(0, cannon), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", cannon), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 1300, "no Fiend Cannon: {}", s.context());
}

/// Ray of Hope (Anime): "When you would take 1500 or more battle damage: Halve that battle damage. At the end of this turn's
/// Battle Phase, you can Special Summon 1 Level 1 monster from your hand, but its effects are negated." Team 1's Dark
/// Magician (2500) beats team 0's Thunder Kid (700): 1800 becomes 900, Thunder Kid dies, and Turbo Synchron (Level 1) is
/// Special Summoned from the hand at the end of the Battle Phase.
#[test]
fn ray_of_hope_halves_a_big_battle_damage_and_summons_a_level_1_monster() {
    let [ray, turbo] = ["Ray of Hope (Anime)", "Turbo Synchron"].map(card);
    let Some(mut s) = field(&[set(0, ray), mon(0, THUNDER_KID), hand(0, turbo), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(ray)], &[Attack(DARK_MAGICIAN)]]).unwrap();
    s.run_both([&[], &[EndTurn]]).unwrap(); // the Battle Phase ends, Main Phase 2 and the End Phase pass
    clean(&s);
    assert_eq!(s.count(0, "grave", ray), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 900, "1800 halved: {}", s.context());
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", turbo), 1, "Turbo Synchron came at the end of the Battle Phase: {}", s.context());
}

/// Ray of Hope (Anime), the control: under 1500 battle damage the Trap is not offered (Warwolf 2000 against Thunder Kid
/// 700 is 1300).
#[test]
fn ray_of_hope_is_not_offered_under_1500_damage() {
    let ray = card("Ray of Hope (Anime)");
    let Some(mut s) = field(&[set(0, ray), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
    assert_eq!(s.count(0, "szone", ray), 1, "{}", s.context());
}

/// Ground Capture: "When an opponent's monster declares an attack: Halve any battle damage you would take from that attack.
/// If you took 1000 or more battle damage from this attack, draw 1 card." Red Dragon Archfiend (3000) beats Thunder Kid
/// (700): 2300 becomes 1150 and a card is drawn; Warwolf's 1300 becomes 650 and nothing is drawn.
#[test]
fn ground_capture_halves_the_damage_and_draws_on_1000_or_more() {
    let capture = card("Ground Capture");
    for (attacker, damage, draws) in [(RED_DRAGON, 1150, 1), (WARWOLF, 650, 0)] {
        let Some(mut s) = field_extra([&[], &[RED_DRAGON]], &[set(0, capture), mon(0, THUNDER_KID), mon(1, attacker)]) else { return };
        keep_hands(&mut s);
        let before = hand_size(&s, 0);
        s.run_both([&[Chain(capture)], &[Attack(attacker)]]).unwrap();
        clean(&s);
        assert_eq!(s.lp(0), 8000 - damage, "{attacker}: {}", s.context());
        assert_eq!(s.count(0, "grave", capture), 1, "{}", s.context());
        assert_eq!(hand_size(&s, 0), before + draws, "{attacker} drew {draws}: {}", s.context());
    }
}

/// Rose Blizzard: "When a monster you control is targeted for an attack: Change the attacking monster to Defense Position,
/// and if you do, negate the attack." Warwolf's attack on Thunder Kid is negated and Warwolf is in Defense Position.
#[test]
fn rose_blizzard_turns_the_attacker_to_defense_and_negates_the_attack() {
    let blizzard = card("Rose Blizzard");
    let Some(mut s) = field(&[set(0, blizzard), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(blizzard)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", blizzard), 1, "{}", s.context());
    assert_eq!(s.position_of(1, WARWOLF), Some(4), "face-up Defense: {}", s.context());
    assert_eq!(s.count(0, "mzone", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
}

/// Crystal Veil: "Activate only when an opponent's monster declares an attack. Banish 1 LIGHT monster from your Graveyard to
/// negate that attack and inflict damage to your opponent equal to half the ATK of the banished monster." Blue-Eyes White
/// Dragon (3000, LIGHT) is banished: the attack is negated and team 1 takes 1500. A Graveyard of DARK monsters only (Feral
/// Imp) leaves the Trap unplayable.
#[test]
fn crystal_veil_banishes_a_light_monster_to_negate_the_attack_and_burn() {
    let [veil, blue_eyes] = ["Crystal Veil", "Blue-Eyes White Dragon"].map(card);
    let Some(mut s) = field(&[set(0, veil), grave(0, blue_eyes), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(veil)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", blue_eyes), 1, "Blue-Eyes was banished: {}", s.context());
    assert_eq!(s.count(0, "grave", veil), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1500), "{}", s.context());
    let Some(mut s) = field(&[set(0, veil), grave(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 6000, "no LIGHT monster to banish: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Burning Rebirth, Burning Strike, Compulsory Recoil Device, Conquest of the Supreme Ruler, Dimension Switch
// ---------------------------------------------------------------------------------------------------------------

/// A duel at the moment Burning Rebirth can be played: Red Dragon Archfiend in team 0's Graveyard and Stardust Dragon on
/// its field, both Synchro Summoned for real (a Synchro Monster put straight into the Graveyard has not been Summoned
/// properly and cannot be revived): Turbo Synchron and Dark Magician (1 + 7), Krebons and Dark Magician Girl (2 + 6) are
/// the materials, Dark Hole destroys both Dragons and Monster Reborn brings Stardust Dragon back. `more` are extra cards
/// of team 0's (the Set Burning Rebirth, a Tuner in the hand).
fn red_dragon_in_the_graveyard(more: &[Spawn]) -> Option<Scenario> {
    let reborn = card("Monster Reborn");
    let mut spawns = vec![
        mon(0, card("Turbo Synchron")), mon(0, DARK_MAGICIAN), mon(0, KREBONS), mon(0, card("Dark Magician Girl")),
        hand(0, DARK_HOLE), hand(0, reborn),
    ];
    spawns.extend_from_slice(more);
    let mut s = field_extra([&[RED_DRAGON, STARDUST], &[]], &spawns)?;
    s.prefer(&[(0, STARDUST)]);
    s.run(&[SpSummon(RED_DRAGON), SpSummon(STARDUST), Activate(DARK_HOLE), Activate(reborn)]).unwrap();
    assert_eq!((s.count(0, "mzone", STARDUST), s.count(0, "grave", RED_DRAGON)), (1, 1), "{}", s.context());
    Some(s)
}

/// Burning Rebirth: "Tribute 1 face-up Level 8 or higher Synchro Monster to target 1 "Red Dragon Archfiend" in your
/// Graveyard. Special Summon that monster in face-up Attack Position and equip this card as an Equip Card to it." Stardust
/// Dragon is Tributed and Red Dragon Archfiend comes back equipped. (No Tuner is in the hand, so the second effect is not
/// there to be chained by the CPU's answer.)
#[test]
fn burning_rebirth_revives_red_dragon_archfiend() {
    let rebirth = card("Burning Rebirth");
    let Some(mut s) = red_dragon_in_the_graveyard(&[set(0, rebirth), hand(0, DARK_HOLE)]) else { return };
    s.prefer(&[(0, STARDUST), (0, RED_DRAGON)]);
    s.run(&[Activate(rebirth)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", STARDUST), 1, "Stardust Dragon was Tributed: {}", s.context());
    assert_eq!(s.count(0, "mzone", RED_DRAGON), 1, "Red Dragon Archfiend is back: {}", s.context());
    assert_eq!(s.position_of(0, RED_DRAGON), Some(1), "face-up Attack: {}", s.context());
    assert_eq!(s.count(0, "szone", rebirth), 1, "the card is equipped: {}", s.context());
    // "When that monster leaves the field, destroy this card": Dark Hole destroys the Dragon and the card follows it.
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", RED_DRAGON), s.count(0, "grave", rebirth)), (1, 1), "{}", s.context());
}

/// Burning Rebirth: "You can send 1 Tuner monster from your hand and this face-up card you control to your Graveyard to
/// Special Summon the Synchro Monster you Tributed for this card's activation." With Junk Synchron in the hand the card's
/// second effect is used after its activation: Junk Synchron and the card go to the Graveyard, Stardust Dragon returns, and the equipped monster is destroyed with the card ("When this card leaves the
/// field, destroy that monster").
#[test]
fn burning_rebirth_sends_a_tuner_and_itself_to_bring_back_the_tributed_synchro() {
    let rebirth = card("Burning Rebirth");
    let Some(mut s) = red_dragon_in_the_graveyard(&[set(0, rebirth), hand(0, JUNK_SYNCHRON)]) else { return };
    s.prefer(&[(0, STARDUST), (0, RED_DRAGON), (0, JUNK_SYNCHRON)]);
    s.run(&[Activate(rebirth), Activate(rebirth)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", STARDUST), 1, "Stardust Dragon returns: {}", s.context());
    assert_eq!((s.count(0, "grave", JUNK_SYNCHRON), s.count(0, "grave", rebirth)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", RED_DRAGON), 1, "the card left the field, so the equipped monster is destroyed: {}", s.context());
}

/// Burning Rebirth: only a Level 8 or higher Synchro Monster can be Tributed: Junk Warrior (Level 5) is not enough.
#[test]
fn burning_rebirth_needs_a_level_8_synchro_to_tribute() {
    let rebirth = card("Burning Rebirth");
    let Some(mut s) = field(&[hand(0, rebirth), mon(0, JUNK_WARRIOR), grave(0, RED_DRAGON)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&rebirth.into()), "{:?}", offered(&s, "activates"));
}

/// Burning Strike: "Equip to a monster you control. If it attacks a Defense Position monster, inflict piercing battle
/// damage to your opponent." Warwolf (2000) equipped attacks Dark Blade in Defense (1500): team 1 takes 500, where without
/// the card it takes nothing.
#[test]
fn burning_strike_gives_piercing_damage() {
    let strike = card("Burning Strike");
    let Some(mut s) = field(&[set(0, strike), mon(0, WARWOLF), mon_def(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(strike), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", strike), 1, "equipped: {}", s.context());
    assert_eq!(s.count(1, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 500, "2000 - 1500 pierces: {}", s.context());
    let Some(mut s) = field(&[mon(0, WARWOLF), mon_def(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    assert_eq!(s.lp(1), 8000, "without it nothing pierces: {}", s.context());
}

/// Compulsory Recoil Device: "You can send 1 card from your hand to the Graveyard to target 1 monster you control, and return
/// it to its owner's hand." The Trap is activated, then its effect used: a Feral Imp is discarded and Warwolf is in the
/// hand.
#[test]
fn compulsory_recoil_device_returns_a_monster_for_a_discard() {
    let device = card("Compulsory Recoil Device");
    let Some(mut s) = field(&[set(0, device), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let graves = s.count(0, "grave", FILLER);
    s.run(&[EndTurn, Activate(device)]).unwrap(); // the CPU's answer to the chain chance uses the effect too
    clean(&s);
    assert_eq!(s.count(0, "hand", WARWOLF), 1, "Warwolf went back to the hand: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", FILLER), graves + 1, "one card was discarded: {}", s.context());
    assert_eq!(s.count(0, "szone", device), 1, "it stays on the field: {}", s.context());
}

/// Conquest of the Supreme Ruler: "When a Synchro Monster you control destroys a Synchro Monster your opponent controls by
/// battle: you can send this face-up card to the Graveyard; Until the End Phase, the Normal and Special Summons of your
/// opponent's monsters are negated." Team 1's Stardust Dragon attacks team 0's Red Dragon Archfiend and is destroyed; in
/// Main Phase 2 team 1's Normal Summon of Warwolf is negated (Warwolf is destroyed), where without the card it stays.
#[test]
fn conquest_of_the_supreme_ruler_negates_the_summons_after_a_synchro_is_destroyed() {
    let conquest = card("Conquest of the Supreme Ruler");
    let spawns = [spell(0, conquest), mon(0, RED_DRAGON), mon(1, STARDUST), hand(1, WARWOLF)];
    let Some(mut s) = field_extra([&[], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(conquest)], &[Attack(STARDUST), Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", STARDUST), 1, "Stardust Dragon was destroyed in battle: {}", s.context());
    assert_eq!(s.count(0, "grave", conquest), 1, "Conquest was sent to the Graveyard: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 0, "the Summon was negated: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    // The control: the same turn without the card.
    let Some(mut s) = field_extra([&[], &[]], &[mon(0, RED_DRAGON), mon(1, STARDUST), hand(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(STARDUST), Summon(WARWOLF)]]).unwrap();
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Dimension Switch: "Activate this card by targeting 1 monster you control; banish it. You can send this face-up card to
/// the Graveyard to return that monster to the field." Warwolf is banished; the card's own effect then sends it to the
/// Graveyard and Warwolf returns to the field.
#[test]
fn dimension_switch_banishes_a_monster_and_returns_it() {
    let switch = card("Dimension Switch");
    let Some(mut s) = field(&[set(0, switch), mon(0, WARWOLF)]) else { return };
    s.run(&[EndTurn, Activate(switch)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", switch), 1, "{}", s.context());
    s.run(&[Activate(switch)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf returned: {}", s.context());
    assert_eq!(s.count(0, "grave", switch), 1, "{}", s.context());
    // Unused, the monster stays banished: only the card's own effect brings it back.
    let Some(mut s) = field(&[set(0, switch), mon(0, WARWOLF)]) else { return };
    s.run_both([&[Activate(switch), EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "removed", WARWOLF), 1, "{}", s.context());
}

/// Dimension Switch's target destroyed in answer (the batch's "first card unchecked" note): the Switch is activated on Warwolf
/// and the player's own Raigeki Break answers, destroying Warwolf while the Switch is on the chain. The Switch resolves with a
/// target that is gone and must not stop on a Lua error: it just stays on the field, with nothing banished.
#[test]
fn dimension_switch_survives_its_target_being_destroyed_in_answer() {
    let [switch, break_] = ["Dimension Switch", "Raigeki Break"].map(card);
    let Some(mut s) = field(&[set(0, switch), set(0, break_), mon(0, WARWOLF)]) else { return };
    s.prefer(&[(0, WARWOLF), (0, FILLER)]);
    s.run(&[Activate(switch), Chain(break_)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "Raigeki Break destroyed it: {}", s.context());
    assert_eq!(s.count(0, "grave", break_), 1, "{}", s.context());
    assert_eq!(s.count(0, "removed", WARWOLF), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Gear Changer, Star Gatherer, Half Straight, Overgain
// ---------------------------------------------------------------------------------------------------------------

/// Gear Changer: "If only your opponent controls a monster, you can Special Summon this card (from your hand). When this card
/// is Summoned: you can target 1 face-up monster your opponent controls; increase this card's Level by that monster's
/// Level." Dark Magician (Level 7) is the opponent's only monster: Gear Changer comes from the hand and its Level 4
/// becomes 11. With a monster of its own it cannot be Special Summoned like that.
#[test]
fn gear_changer_special_summons_itself_and_takes_the_level_of_a_monster() {
    let gear = card("Gear Changer");
    let Some(mut s) = field(&[hand(0, gear), mon(1, DARK_MAGICIAN)]) else { return };
    s.run(&[SpSummon(gear), Chain(gear)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", gear), 1, "{}", s.context());
    assert_eq!(s.level_of(0, gear), Some(4 + 7), "Level 4 plus Dark Magician's 7: {}", s.context());
    let Some(mut s) = field(&[hand(0, gear), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&gear.into()), "{:?}", offered(&s, "special_summons"));
    let Some(mut s) = field(&[hand(0, gear)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "special_summons").contains(&gear.into()), "the opponent controls nothing: {:?}", offered(&s, "special_summons"));
}

/// Star Gatherer: "Target 1 Synchro Monster your opponent controls; this card's Level becomes equal to the Level of that
/// monster -1." Red Dragon Archfiend (Level 8) makes the Level 1 Star Gatherer Level 7; a Warwolf is no target.
#[test]
fn star_gatherer_takes_the_level_of_a_synchro_monster_minus_one() {
    let gatherer = card("Star Gatherer");
    let Some(mut s) = field_extra([&[], &[RED_DRAGON]], &[mon(0, gatherer), mon(1, RED_DRAGON)]) else { return };
    assert_eq!(s.level_of(0, gatherer), Some(1), "{}", s.context());
    s.run(&[Activate(gatherer)]).unwrap();
    clean(&s);
    assert_eq!(s.level_of(0, gatherer), Some(7), "8 - 1: {}", s.context());
    let Some(mut s) = field(&[mon(0, gatherer), mon(1, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&gatherer.into()), "{:?}", offered(&s, "activates"));
}

/// Half Straight: "Target 1 monster you control. Until your next Standby Phase, that monster's ATK is halved. If that ATK is
/// lower than the ATK of the monster with the highest ATK your opponent controls, that monster can attack directly this
/// turn." Warwolf (2000) is halved to 1000, under Dark Magician's 2500, so it attacks directly past Dark Magician: team 1
/// takes 1000. The ATK is back at 2000 from team 0's next Standby Phase.
#[test]
fn half_straight_halves_the_atk_and_allows_a_direct_attack() {
    let straight = card("Half Straight");
    let Some(mut s) = field(&[set(0, straight), mon(0, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(straight)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "{}", s.context());
    s.run(&[Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "a direct attack at 1000 ATK: {}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(1000), "still halved in team 1's turn: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(0, WARWOLF), Some(2000), "back at the next Standby Phase: {}", s.context());
}

/// Half Straight: a monster whose halved ATK is not lower than the opponent's highest (Warwolf 1000 against a Thunder Kid
/// 700) gets no direct attack: it attacks the Thunder Kid and destroys it, and team 1 takes 300.
#[test]
fn half_straight_gives_no_direct_attack_against_a_weaker_monster() {
    let straight = card("Half Straight");
    let Some(mut s) = field(&[set(0, straight), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(straight), Attack(WARWOLF)], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "it had to attack the monster: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "1000 - 700: {}", s.context());
}

/// Overgain: "Target 1 face-up monster you control. It gains 1000 ATK until the End Phase. During this turn, that monster
/// cannot declare an attack." Warwolf is 3000 after it, and cannot attack that turn.
#[test]
fn overgain_adds_1000_atk_but_forbids_the_attack() {
    let overgain = card("Overgain");
    let Some(mut s) = field(&[set(0, overgain), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(overgain)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, WARWOLF), Some(3000), "{}", s.context());
    s.run(&[Attack(WARWOLF)]).expect_err("Warwolf cannot declare an attack this turn");
    clean(&s);
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Rage Resynchro, Shining Rebirth, Tuner's Mind, Tuner's Reflect, Revenge Twin Soul, Synchro Stream, Synchro Sonic
// ---------------------------------------------------------------------------------------------------------------

/// A duel at team 0's turn 3 with Junk Warrior in its Graveyard (Synchro Summoned in turn 1 from Junk Synchron and Quillbolt
/// Hedgehog, 3 + 2, and destroyed in turn 2 by team 1's Hammer Shot), a second pair of the same materials on its field,
/// and `more` spawns.
fn junk_warrior_in_the_graveyard(more: &[Spawn]) -> Option<Scenario> {
    let mut spawns = vec![mon(0, JUNK_SYNCHRON), mon(0, QUILLBOLT), mon(0, JUNK_SYNCHRON), mon(0, QUILLBOLT), hand(1, card("Hammer Shot"))];
    spawns.extend_from_slice(more);
    let mut s = field_extra([&[JUNK_WARRIOR], &[]], &spawns)?;
    keep_hands(&mut s);
    s.prefer(&[(0, JUNK_WARRIOR)]);
    s.run_both([&[SpSummon(JUNK_WARRIOR)], &[Activate(card("Hammer Shot"))]]).unwrap();
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "grave", JUNK_WARRIOR), 1, "destroyed by Hammer Shot: {}", s.context());
    assert_eq!((s.count(0, "mzone", JUNK_SYNCHRON), s.count(0, "mzone", QUILLBOLT)), (1, 1), "{}", s.context());
    Some(s)
}

/// Rage Resynchro: "Target 1 Synchro Monster in your Graveyard. Send the appropriate Synchro Materials you control listed on
/// that card to the Graveyard, whose total Levels equal that monster's Level, and Special Summon that monster. It gains 500
/// ATK. Destroy it during the next turn's End Phase." Junk Warrior (Level 5) returns for Junk Synchron and Quillbolt
/// Hedgehog (3 + 2), 2800 ATK, stays through its summoner's turn and the opponent's, and is destroyed at the End Phase of
/// the next turn.
#[test]
fn rage_resynchro_resummons_a_synchro_with_500_atk_for_one_turn() {
    let rage = card("Rage Resynchro");
    let Some(mut s) = junk_warrior_in_the_graveyard(&[set(0, rage)]) else { return };
    s.run(&[Activate(rage)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "Junk Warrior is back: {}", s.context());
    assert_eq!(s.attack_of(0, JUNK_WARRIOR), Some(2300 + 500), "{}", s.context());
    assert_eq!((s.count(0, "mzone", JUNK_SYNCHRON), s.count(0, "mzone", QUILLBOLT)), (0, 0), "the materials were sent: {}", s.context());
    assert_eq!(s.count(0, "grave", rage), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "still there in the opponent's turn: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 0, "destroyed at the next turn's End Phase: {}", s.context());
    assert_eq!(s.count(0, "grave", JUNK_WARRIOR), 1, "{}", s.context());
}

/// Shining Rebirth: "Target 1 Synchro Monster in your Graveyard; Synchro Summon it from the Graveyard, and it cannot be
/// destroyed this turn." Junk Warrior comes out of the Graveyard for Junk Synchron and Quillbolt Hedgehog on the field, and
/// Dark Hole in the same turn destroys the opponent's Warwolf but not it.
#[test]
fn shining_rebirth_synchro_summons_from_the_graveyard_and_protects_the_monster() {
    let rebirth = card("Shining Rebirth");
    let spawns = [hand(0, rebirth), grave(0, JUNK_WARRIOR), mon(0, JUNK_SYNCHRON), mon(0, QUILLBOLT), hand(0, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(rebirth), Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "Synchro Summoned from the Graveyard: {}", s.context());
    assert_eq!((s.count(0, "grave", JUNK_SYNCHRON), s.count(0, "grave", QUILLBOLT)), (1, 1), "the materials: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "Dark Hole resolved: {}", s.context());
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 1, "and could not destroy Junk Warrior: {}", s.context());
    assert_eq!(s.count(0, "grave", rebirth), 1, "{}", s.context());
}

/// Shining Rebirth: with no materials for the Synchro Monster in the Graveyard it cannot be played.
#[test]
fn shining_rebirth_needs_the_materials_on_the_field() {
    let rebirth = card("Shining Rebirth");
    let Some(mut s) = field(&[hand(0, rebirth), grave(0, JUNK_WARRIOR), mon(0, QUILLBOLT)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&rebirth.into()), "{:?}", offered(&s, "activates"));
}

/// Tuner's Mind: "When a Synchro Monster you control is targeted for an attack: Return it to the Extra Deck, then, if all the
/// Synchro Materials used in that monster's Synchro Summon are in your Graveyard, you can Special Summon them, and if you do,
/// the Tuner Special Summoned by this effect becomes the new attack target." Junk Warrior, Synchro Summoned from Junk
/// Synchron and Quillbolt Hedgehog, is attacked by Warwolf: it goes back to the Extra Deck, both materials return, and
/// Warwolf now attacks Junk Synchron (the Tuner): team 0 takes 2000 - 1300 = 700 and Junk Synchron is destroyed.
#[test]
fn tuners_mind_returns_the_synchro_and_brings_back_its_materials() {
    let mind = card("Tuner's Mind");
    let spawns = [set(0, mind), mon(0, JUNK_SYNCHRON), mon(0, QUILLBOLT), mon(1, WARWOLF)];
    let Some(mut s) = field_extra([&[JUNK_WARRIOR], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(JUNK_WARRIOR), Chain(mind)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", JUNK_WARRIOR), 0, "Junk Warrior went back to the Extra Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", JUNK_WARRIOR), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", QUILLBOLT), 1, "Quillbolt Hedgehog is back: {}", s.context());
    assert_eq!(s.count(0, "grave", mind), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", JUNK_SYNCHRON), 1, "the Tuner became the attack target and lost to Warwolf: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 700, "{}", s.context());
}

/// Tuner's Reflect: "During the turn 2 or more Tuners you control were destroyed: Special Summon 2 of those monsters from your
/// Graveyard in face-up Attack Position, then, inflict damage to your opponent equal to the combined ATK of those
/// monsters." Dark Hole destroys Krebons (1200) and Junk Synchron (1300) in turn 3; the Trap brings both back and inflicts
/// 2500.
#[test]
fn tuners_reflect_brings_back_two_destroyed_tuners_and_burns_for_their_atk() {
    let reflect = card("Tuner's Reflect");
    let Some(mut s) = field(&[set(0, reflect), mon(0, KREBONS), mon(0, JUNK_SYNCHRON), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(DARK_HOLE), Activate(reflect)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", KREBONS), s.count(0, "mzone", JUNK_SYNCHRON)), (1, 1), "both Tuners are back: {}", s.context());
    assert_eq!(s.position_of(0, KREBONS), Some(1), "face-up Attack: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2500, "{}", s.context());
    assert_eq!(s.count(0, "grave", reflect), 1, "{}", s.context());
}

/// Tuner's Reflect: one destroyed Tuner is not enough ("2 or more"), so the Trap is not offered.
#[test]
fn tuners_reflect_needs_two_destroyed_tuners() {
    let reflect = card("Tuner's Reflect");
    let Some(mut s) = field(&[set(0, reflect), mon(0, KREBONS), hand(0, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(DARK_HOLE)]).unwrap();
    assert!(!offered(&s, "activates").contains(&reflect.into()), "{:?}", offered(&s, "activates"));
    assert_eq!(s.count(0, "grave", KREBONS), 1, "{}", s.context());
}

/// Revenge Twin Soul: "When your opponent's monster declares an attack: Target 1 Defense Position monster you control, then
/// banish up to 2 Synchro Monsters from your Graveyard; the targeted monster gains DEF equal to the combined Levels of the
/// banished Synchro Monster(s) x100, until the End Phase. During this turn, at the end of the Damage Step, if that monster is
/// attacked by your opponent's monster that has less ATK than the DEF of that monster, destroy the monster that
/// attacked." Red Dragon Archfiend (Level 8) is banished: Dark Blade in Defense (1500) becomes 2300, Warwolf (2000)
/// attacks it, bounces for 300 and is destroyed at the end of the Damage Step.
#[test]
fn revenge_twin_soul_adds_defense_and_destroys_the_weaker_attacker() {
    let twin = card("Revenge Twin Soul");
    let Some(mut s) = field(&[set(0, twin), mon_def(0, DARK_BLADE), grave(0, RED_DRAGON), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(twin)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", RED_DRAGON), 1, "Red Dragon Archfiend was banished: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the attacker was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "2300 - 2000 is reflected: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Synchro Stream: "If 2 or more Synchro Monsters are on the field: Target 1 Synchro monster on the field; if it declares an
/// attack, it gains ATK equal to the ATK of 1 other Synchro Monster on the field until the End Phase." Junk Warrior (2300) is
/// the target and Stardust Dragon (2500) is the other: its attack is 4800 and it destroys Stardust Dragon, with 2300
/// damage.
#[test]
fn synchro_stream_adds_the_atk_of_another_synchro_monster_when_it_attacks() {
    let stream = card("Synchro Stream");
    let Some(mut s) = field_extra([&[], &[]], &[set(0, stream), mon(0, JUNK_WARRIOR), mon(1, STARDUST)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, JUNK_WARRIOR)]);
    s.run_both([&[EndTurn, Activate(stream), Attack(JUNK_WARRIOR)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, JUNK_WARRIOR), Some(2300 + 2500), "{}", s.context());
    assert_eq!(s.count(1, "grave", STARDUST), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 2300, "4800 - 2500: {}", s.context());
}

/// Synchro Stream: one Synchro Monster on the field is not enough ("2 or more").
#[test]
fn synchro_stream_needs_two_synchro_monsters_on_the_field() {
    let stream = card("Synchro Stream");
    let Some(mut s) = field(&[set(0, stream), mon(0, JUNK_WARRIOR)]) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!offered(&s, "activates").contains(&stream.into()), "{:?}", offered(&s, "activates"));
}

/// Synchro Sonic: "Target face-up cards in your opponent's Spell & Trap Zone, up to the number of Synchro Monsters you
/// control; destroy them." Two Synchro Monsters allow two of team 1's three face-up cards (Messenger of Peace, Swords of
/// Revealing Light, Gravity Bind): two are destroyed and one stays.
#[test]
fn synchro_sonic_destroys_as_many_cards_as_there_are_synchro_monsters() {
    let sonic = card("Synchro Sonic");
    let [messenger, swords, bind] = ["Messenger of Peace", "Swords of Revealing Light", "Gravity Bind"].map(card);
    let spawns = [set(0, sonic), mon(0, JUNK_WARRIOR), mon(0, STARDUST), spell(1, messenger), spell(1, swords), spell(1, bind)];
    let Some(mut s) = field_extra([&[], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, messenger), (1, swords), (1, bind)]);
    s.prefer_most(true);
    s.run(&[EndTurn, Activate(sonic)]).unwrap();
    clean(&s);
    let left = s.count(1, "szone", messenger) + s.count(1, "szone", swords) + s.count(1, "szone", bind);
    assert_eq!(left, 1, "two destroyed, one stays: {}", s.context());
    assert_eq!(s.count(1, "grave", messenger) + s.count(1, "grave", swords) + s.count(1, "grave", bind), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", sonic), 1, "{}", s.context());
}

/// Synchro Sonic: without a Synchro Monster of its own it is not offered.
#[test]
fn synchro_sonic_needs_a_synchro_monster() {
    let sonic = card("Synchro Sonic");
    let Some(mut s) = field(&[set(0, sonic), mon(0, WARWOLF), spell(1, card("Gravity Bind"))]) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!offered(&s, "activates").contains(&sonic.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Reaction Summon, Rose Curse, Trap Pause, Sneak Exploder, Closed Plant Gate, Illusion Destruction, Dimension Reversion
// ---------------------------------------------------------------------------------------------------------------

const AXE_OF_DESPAIR: u16 = 4310; // an Equip Spell: +1000 ATK

/// Reaction Summon: "When your opponent's monster's ATK is changed: Target 1 Level 4 or lower monster in your Graveyard;
/// Special Summon it." Team 1 equips Axe of Despair to its Dark Blade (+1000): Warwolf (Level 4) comes back from team 0's
/// Graveyard.
#[test]
fn reaction_summon_answers_an_atk_change_with_a_monster_from_the_graveyard() {
    let reaction = card("Reaction Summon");
    let Some(mut s) = field(&[set(0, reaction), grave(0, WARWOLF), hand(1, AXE_OF_DESPAIR), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(reaction)], &[Activate(AXE_OF_DESPAIR)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "Warwolf was Special Summoned: {}", s.context());
    assert_eq!(s.count(0, "grave", reaction), 1, "{}", s.context());
    assert_eq!(s.attack_of(1, DARK_BLADE), Some(2800), "the Axe resolved: {}", s.context());
}

/// Reaction Summon: only a Level 4 or lower monster can be brought back: a Level 7 Dark Magician in the Graveyard is no
/// target, so the Trap is not offered when the ATK changes.
#[test]
fn reaction_summon_leaves_a_level_7_monster_in_the_graveyard() {
    let reaction = card("Reaction Summon");
    let Some(mut s) = field(&[set(0, reaction), grave(0, DARK_MAGICIAN), hand(1, AXE_OF_DESPAIR), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(AXE_OF_DESPAIR)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "szone", reaction), 1, "{}", s.context());
}

/// Rose Curse: "Activate only if the ATK of a monster your opponent controls changes. Inflict damage to your opponent equal
/// to the difference between that monster's original ATK and current ATK." Team 1's Axe of Despair gives Dark Blade 1000
/// ATK: team 1 takes 1000.
#[test]
fn rose_curse_inflicts_the_difference_between_original_and_current_atk() {
    let curse = card("Rose Curse");
    let Some(mut s) = field(&[set(0, curse), hand(1, AXE_OF_DESPAIR), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(curse)], &[Activate(AXE_OF_DESPAIR)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", curse), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 1000), "{}", s.context());
}

/// Trap Pause: "Negate the effects of your opponent's face-up Continuous Trap Cards, until the End Phase. Then, draw 1 card for
/// each card negated by this effect." Team 1's Gravity Bind (Level 4 or higher monsters cannot attack) and Ultimate
/// Offering are negated: team 0 draws 2 and its Warwolf (Level 4) attacks for 2000, where with Gravity Bind standing it
/// cannot attack at all.
#[test]
fn trap_pause_negates_continuous_traps_and_draws_for_each() {
    let [pause, bind, offering] = ["Trap Pause", "Gravity Bind", "Ultimate Offering"].map(card);
    let spawns = [hand(0, pause), mon(0, WARWOLF), spell(1, bind), spell(1, offering)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let before = hand_size(&s, 0);
    s.run(&[Activate(pause), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), before - 1 + 2, "Trap Pause left the hand and 2 cards came: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 2000, "Gravity Bind was negated, so Warwolf attacked: {}", s.context());
    assert_eq!(s.count(1, "szone", bind) + s.count(1, "szone", offering), 2, "they stay on the field: {}", s.context());
    // The control: Gravity Bind standing, Warwolf cannot attack.
    let Some(mut s) = field(&[mon(0, WARWOLF), spell(1, bind)]) else { return };
    s.run(&[EndTurn, Attack(WARWOLF)]).expect_err("Level 4 or higher monsters cannot attack");
}

/// Trap Pause: with no face-up Continuous Trap Card on the opponent's side (a Continuous Spell is not one) it is not offered.
#[test]
fn trap_pause_needs_a_continuous_trap() {
    let pause = card("Trap Pause");
    let Some(mut s) = field(&[hand(0, pause), spell(1, card("Messenger of Peace"))]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&pause.into()), "{:?}", offered(&s, "activates"));
}

/// Sneak Exploder: "Activate only while your opponent controls no monsters. During your opponent's next Standby Phase: Send
/// this card to the Graveyard, and if you do, inflict 500 damage to your opponent for each monster you control." With two
/// monsters, team 1 takes 1000 at its Standby Phase and the card is in the Graveyard. With a monster on the opponent's side
/// it is not offered.
#[test]
fn sneak_exploder_inflicts_500_for_each_monster_at_the_opponents_standby_phase() {
    let exploder = card("Sneak Exploder");
    let Some(mut s) = field(&[set(0, exploder), mon(0, WARWOLF), mon(0, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(exploder), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "500 for each of 2 monsters: {}", s.context());
    assert_eq!(s.count(0, "grave", exploder), 1, "{}", s.context());
    let Some(mut s) = field(&[set(0, exploder), mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!offered(&s, "activates").contains(&exploder.into()), "{:?}", offered(&s, "activates"));
}

/// Closed Plant Gate: "If you control 2 face-up Plant-Type monsters with the same name: Your opponent cannot declare an attack
/// during their next turn." Two Walls of Ivy are on team 0's field: in team 1's turn 4 its Warwolf cannot attack, where
/// without the Trap it hits for 2000 (a Wall of Ivy is Plant, 300/1200: it attacks it and destroys it, no damage in Defense).
#[test]
fn closed_plant_gate_stops_the_opponents_attacks_for_their_next_turn() {
    let [gate, wall] = ["Closed Plant Gate", "Wall of Ivy"].map(card);
    let Some(mut s) = field(&[set(0, gate), mon(0, wall), mon(0, wall), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(gate), EndTurn], &[EndTurn, Attack(WARWOLF)]]).expect_err("no attack in turn 4");
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(0, "mzone", wall), 2, "{}", s.context());
    // The control: without the Trap team 1's Warwolf attacks in turn 2.
    let Some(mut s) = field(&[mon(0, wall), mon(0, wall), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.count(0, "mzone", wall), 1, "{}", s.context());
}

/// Closed Plant Gate: two Plants of different names (Wall of Ivy and a Thunder Kid) are not enough.
#[test]
fn closed_plant_gate_needs_two_plants_with_the_same_name() {
    let [gate, wall] = ["Closed Plant Gate", "Wall of Ivy"].map(card);
    let Some(mut s) = field(&[set(0, gate), mon(0, wall), mon(0, THUNDER_KID)]) else { return };
    s.run(&[EndTurn]).unwrap();
    assert!(!offered(&s, "activates").contains(&gate.into()), "{:?}", offered(&s, "activates"));
}

/// Illusion Destruction: "Target 1 card on the field; flip it face-down." Team 1's Warwolf is turned face-down in Defense
/// (position 8), and its Gravity Bind face-down on the Spell & Trap Zone (it no longer applies: Warwolf can attack
/// then).
#[test]
fn illusion_destruction_flips_a_monster_or_a_spell_trap_face_down() {
    let [destruction, bind] = ["Illusion Destruction", "Gravity Bind"].map(card);
    let Some(mut s) = field(&[set(0, destruction), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[EndTurn, Activate(destruction)]).unwrap();
    clean(&s);
    assert_eq!(s.position_of(1, WARWOLF), Some(8), "face-down Defense: {}", s.context());
    let Some(mut s) = field(&[set(0, destruction), mon(0, WARWOLF), spell(1, bind)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, bind)]);
    s.run(&[EndTurn, Activate(destruction), Attack(WARWOLF)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 2000, "Gravity Bind is face-down and Warwolf attacked: {}", s.context());
    assert_eq!(s.count(1, "szone", bind), 1, "it stays on the field, face-down: {}", s.context());
}

/// Dimension Reversion: "Return all banished monsters to their owner's Deck(s)." Warwolf banished by team 0 and Dark Blade by
/// team 1 both go back to their own Decks (a card banished face-up that is not a monster, Dian Keto, stays).
#[test]
fn dimension_reversion_returns_every_banished_monster_to_its_owners_deck() {
    let reversion = card("Dimension Reversion");
    let Some(mut s) = field(&[set(0, reversion), banished(0, WARWOLF), banished(1, DARK_BLADE), banished(0, 4348)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    let decks = [s.state()[0]["deck"].clone(), s.state()[1]["deck"].clone()];
    s.run(&[Activate(reversion)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", WARWOLF), s.count(1, "removed", DARK_BLADE)), (0, 0), "{}", s.context());
    assert_eq!(s.count(0, "removed", 4348), 1, "a Spell stays banished: {}", s.context());
    assert_eq!(s.state()[0]["deck"].as_u64().unwrap(), decks[0].as_u64().unwrap() + 1, "{}", s.context());
    assert_eq!(s.state()[1]["deck"].as_u64().unwrap(), decks[1].as_u64().unwrap() + 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Akiza's Plants: Crystal Rose Spirit, Rose Flame, Cursed Ivy and its Ivy Tokens, Doom Petal Countdown
// ---------------------------------------------------------------------------------------------------------------

const WALL_OF_IVY: u16 = 7872; // Level 2 Plant, 300/1200, an "Ivy" monster
const MYSTICAL_SPACE_TYPHOON: u16 = 4909;

/// Crystal Rose Spirit: "After damage calculation, if this card destroys an opponent's monster by battle: Banish that monster
/// instead of sending it to the Graveyard." Its attack (1400) destroys team 1's Thunder Kid (700): it is banished, where a
/// monster destroyed by Warwolf goes to the Graveyard.
#[test]
fn crystal_rose_spirit_banishes_the_monster_it_destroys_in_battle() {
    let spirit = card("Crystal Rose Spirit");
    let Some(mut s) = field(&[mon(0, spirit), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(spirit)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", THUNDER_KID), 1, "banished: {}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 0, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
    let Some(mut s) = field(&[mon(0, WARWOLF), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "removed", THUNDER_KID)), (1, 0), "{}", s.context());
}

/// Crystal Rose Spirit, a monster destroyed *by* it in Defense Position is banished too, and the Spirit itself, when it is the
/// one destroyed (Dark Magician, 2500, attacks it), goes to the Graveyard.
#[test]
fn crystal_rose_spirit_is_sent_to_the_graveyard_when_it_is_destroyed() {
    let spirit = card("Crystal Rose Spirit");
    let Some(mut s) = field(&[mon(0, spirit), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", spirit), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", DARK_MAGICIAN), 1, "{}", s.context());
}

/// Rose Flame: "Each time a Plant-Type monster(s) is Summoned to your opponent's side of the field, inflict 500 damage to your
/// opponent." Team 1 Normal Summons Wall of Ivy (Plant): it takes 500; a Warwolf (not a Plant) costs nothing. A Plant
/// Summoned to Rose Flame's own controller's side hurts nobody.
#[test]
fn rose_flame_inflicts_500_for_each_plant_summoned_to_the_opponents_side() {
    let flame = card("Rose Flame");
    let Some(mut s) = field(&[spell(0, flame), hand(1, WALL_OF_IVY)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Summon(WALL_OF_IVY)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 500), "{}", s.context());
    let Some(mut s) = field(&[spell(0, flame), hand(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Summon(WARWOLF)]]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "not a Plant: {}", s.context());
    let Some(mut s) = field(&[spell(0, flame), hand(0, WALL_OF_IVY)]) else { return };
    keep_hands(&mut s);
    s.run(&[Summon(WALL_OF_IVY)]).unwrap();
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "its own side: {}", s.context());
}

/// Cursed Ivy: "Target 1 "Ivy" monster in your Graveyard; Special Summon it in face-up Defense Position and equip it with this
/// card. When this card leaves the field, destroy that monster. When this card is sent from the field to the Graveyard:
/// Special Summon 2 "Ivy Tokens" (Plant-Type/EARTH/Level 1/ATK 0/DEF 0) to your opponent's side of the field in Defense
/// Position. When an "Ivy Token" is destroyed, inflict 300 damage to its controller."
/// Wall of Ivy comes back in Defense equipped with the card.
#[test]
fn cursed_ivy_revives_an_ivy_monster_in_defense_and_equips_it() {
    let ivy = card("Cursed Ivy");
    let Some(mut s) = field(&[set(0, ivy), grave(0, WALL_OF_IVY)]) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn, Activate(ivy)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WALL_OF_IVY), 1, "{}", s.context());
    assert_eq!(s.position_of(0, WALL_OF_IVY), Some(4), "face-up Defense: {}", s.context());
    assert_eq!(s.count(0, "szone", ivy), 1, "{}", s.context());
}

/// A team's Ivy Tokens on the field: the game's own Tokens have no Konami id in the engine's state (its `cid` is 0), so they
/// are found by their name.
fn ivy_tokens(s: &Scenario, team: usize) -> Vec<serde_json::Value> {
    let zone = s.state()[team]["mzone"].as_array().cloned().unwrap_or_default();
    zone.into_iter().filter(|c| c["name"] == "Ivy Token").collect()
}

/// Cursed Ivy's Tokens: Mystical Space Typhoon destroys the card: Wall of Ivy is destroyed with it, and two Ivy Tokens appear
/// on the opponent's side in Defense Position (Plant, EARTH, Level 1, 0/0). Dark Hole destroys both Tokens: each costs its
/// controller 300, so team 1 takes 600, and a Token is gone for good, not in the Graveyard.
#[test]
fn cursed_ivy_leaving_the_field_gives_the_opponent_two_ivy_tokens_that_hurt_when_destroyed() {
    let ivy = card("Cursed Ivy");
    let spawns = [set(0, ivy), grave(0, WALL_OF_IVY), hand(0, MYSTICAL_SPACE_TYPHOON), hand(0, DARK_HOLE)];
    let Some(mut s) = field(&spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, ivy)]);
    s.run(&[Activate(ivy), Activate(MYSTICAL_SPACE_TYPHOON)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", ivy), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", WALL_OF_IVY), 1, "Wall of Ivy was destroyed with the card: {}", s.context());
    let tokens = ivy_tokens(&s, 1);
    assert_eq!(tokens.len(), 2, "two Tokens on the opponent's side: {}", s.context());
    assert!(ivy_tokens(&s, 0).is_empty(), "{}", s.context());
    for t in &tokens {
        assert_eq!((t["attack"].as_i64(), t["defense"].as_i64(), t["level"].as_i64()), (Some(0), Some(0), Some(1)), "{}", s.context());
        assert_eq!((t["position"].as_u64(), t["race"].as_u64(), t["attribute"].as_u64()), (Some(4), Some(0x400), Some(1)), "face-up Defense, Plant, EARTH: {}", s.context());
    }
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert!(ivy_tokens(&s, 1).is_empty(), "the Tokens are gone: {}", s.context());
    assert_eq!(s.state()[1]["grave"].as_array().unwrap().iter().filter(|c| c["name"] == "Ivy Token").count(), 0, "a Token is not in the Graveyard");
    assert_eq!(s.lp(1), 8000 - 600, "300 for each Token: {}", s.context());
}

/// Doom Petal Countdown: "Once per turn, during your End Phase: You can banish 1 Plant-Type monster from your Graveyard;
/// inflict 300 damage to your opponent, also, even if this card has left the field, if there are no Plant-Type monsters
/// in your Graveyard, inflict 300 damage to your opponent for each Plant-Type monster banished by this card's effect. If
/// there are no Plant-Type monsters in your Graveyard, destroy this card." (The fuzz flagged it as activated 15 times
/// and never inflicting damage: it merges the Activate and the End Phase effect, both "effect 0", and in its duels no
/// Plant was ever in the Graveyard at the End Phase for the second to fire. Both are exercised here.) With two Walls of Ivy in the Graveyard: the
/// first End Phase banishes one for 300; the second banishes the last one, the card is destroyed, and the 300 for each of
/// the two banished (600) is inflicted all the same (batch 23's fuzz: the count was kept on the card and lost with it).
#[test]
fn doom_petal_countdown_counts_every_banished_plant_even_after_it_is_destroyed() {
    let doom = card("Doom Petal Countdown");
    let Some(mut s) = field(&[spell(0, doom), grave(0, WALL_OF_IVY), grave(0, WALL_OF_IVY)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(doom)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", WALL_OF_IVY), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "{}", s.context());
    assert_eq!(s.count(0, "szone", doom), 1, "a Plant is left in the Graveyard: {}", s.context());
    s.run_both([&[EndTurn, Chain(doom)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", WALL_OF_IVY), 2, "{}", s.context());
    assert_eq!(s.count(0, "grave", doom), 1, "no Plant is left, so it is destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 300 - 600, "300 for each of the 2 banished: {}", s.context());
}

/// Doom Petal Countdown with a single Plant: the banish leaves the Graveyard without Plants, the card is destroyed and
/// the damage is 300 for that one Plant.
#[test]
fn doom_petal_countdown_with_the_last_plant_inflicts_300() {
    let doom = card("Doom Petal Countdown");
    let Some(mut s) = field(&[spell(0, doom), grave(0, WALL_OF_IVY)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Chain(doom)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", doom), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 300, "{}", s.context());
}
