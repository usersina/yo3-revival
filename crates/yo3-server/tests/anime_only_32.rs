//! Anime-only cards, group 32 (rows 6270-6302 of engine/added/cards.tsv): the video game's versions of anime cards
//! (ProjectIgnis' Video Game scope: the Tag Force texts) of Akiza Izinski, Andre, Aporia, Bolt Tanner, Breo, Devack, Don
//! Piero, Ghost, Gill Randsborg and Grady. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Every Trap is Set first (it cannot be used the turn
//! it is Set), and neither player has a Battle Phase in turn 1, so team 0's attacks wait for turn 3. A Trap given to a
//! team's plan with `Chain` is used at the first chance it is offered, so a test that needs a later one starts the plan
//! with `EndTurn`.
//! Covered: the Token (Dark Matter Token, made by Dark Matter (VG): two of them in Defense Position, not destroyed by
//! battle, gone at the End Phase of the next turn), the cards the per-card fuzz flagged (Synchro Back, Beast Burial Ritual,
//! Dimension Equilibrium, Destruction Trigger, Tuning Collapse, Dark Matter (VG), Dark Tuner Dark Ape, Toichi the Nefarious
//! Debt Collector, Wise Core, Wisel Guard (TF6), Blizzard Lizard: the random duels never gave them what they wait for: a
//! Synchro Monster to return, a Beast, a destroyed Synchro or Dark Synchro Monster, a Loan Token, a Meklord Emperor), the
//! script note (Chaos Blast (VG): its cost cannot leave nothing, but its target can be gone before it resolves), the
//! signature cards (Dark Tuner Dark Ape with Dark Wave and a Dark Synchro Summon, Dark Matter, Meklord Emperor Creation,
//! Wise Core, Endless Loan with Toichi, the Masked Knights' three Levels, Labyrinth of Kline, Spark Breaker) and the
//! seeded sample (Battle Return, Cursed Prison, Cursed Synchro, Dark Wave, Meklord Emperor Creation, Wisel Guard 3 (TF6)),
//! with Chaos Bloom, Future Destruction, Giant Ushi Oni and Aurora Draw, being cheap.
//! One override: Aurora Draw (`engine/overrides/c100000038.lua`: ProjectIgnis' condition could never be met from the hand,
//! and asked for no Meklord monster when Set). Chaos Blast (VG)'s script note needs none: the cost sends cards from the Deck, so
//! it cannot take the target away, and a target gone before it resolves (returned to the hand, or a Token that shattered)
//! raised no Lua error (`chaos_blast_with_its_target_gone_does_nothing_and_does_not_stop_the_duel`).
//! Not covered: Wisel Top, Attack, Attack 3 and Carrier (TF6) beyond Wise Core's summons and the Guards (the Wisels'
//! other cards of batches 26-29 share their scripts' shape), Meklord Emperor Creation's replacement of a destruction (it
//! needs a Wisel, Granel or Skiel in the Graveyard and a choice the client asks for), and Dark Wave's reset at the End Phase.
//! Dark Synchro Summon: the anime Dark Tuners have the setcode 0x600 and the Dark Synchro Monsters the Dark Synchro
//! procedure (the game's printed "Dark Tuner" cards have neither, so they cannot make one). A Dark Synchro Monster's Level is
//! the Dark Tuner's minus the non-Tuner's (Dark Tuner Level 8 - non-Tuner 2 = Level 6: Underground Arachnid), so a Level 2
//! Dark Ape, which is below every Dark Synchro Monster's Level, makes one only with Dark Wave, which gives the non-Tuner a
//! negative Level (Level 2 + 4 = Underground Arachnid's 6).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::{Response, msg};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const BAT: u16 = 4417; // Bat: a Level 1 Machine, 300/350, vanilla
const WOLF: u16 = 4067; // Wolf: a Level 3 Beast, 1200/800, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const FISSURE: u16 = 4835; // destroy the opponent's face-up monster with the lowest ATK
const TRUNADE: u16 = 4910; // Giant Trunade: every Spell and Trap Card on the field returns to the hand
const NITRO: u16 = 7700; // Nitro Synchron: a Level 2 Tuner
const GRANEL: u16 = 9265; // the anime Meklord Emperor Granel: a Meklord Emperor on the field keeps the Wisels alive

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn grave(team: u32, cid: u16) -> Spawn { Spawn { team, place: Grave, cid } }
fn spell(team: u32, cid: u16) -> Spawn { Spawn { team, place: Spell, cid } }
fn set(team: u32, cid: u16) -> Spawn { Spawn { team, place: SpellSet, cid } }

/// An anime-cards duel with the given seed and starting LP, each Deck of one filler card: team 0's has `bottom` at its very
/// bottom (out of the opening hand and the first draws), for a card an effect fetches from the Deck.
fn duel(seed: u64, lp: u32, bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mine: Vec<u16> = bottom.iter().copied().chain(std::iter::repeat_n(FILLER, 40 - bottom.len())).collect();
    let decks = [(&mine[..], &[][..]), (&[FILLER; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, seed, lp, decks, spawns).unwrap())
}

/// The usual duel: 8000 LP, both Decks of filler.
fn field(bottom: &[u16], spawns: &[Spawn]) -> Option<Scenario> { duel(7, 8000, bottom, spawns) }

/// A duel with the Extra Decks given: `extra[team]`.
fn field_extra(extra: [&[u16]; 2], spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[FILLER; 40][..], extra[0]), (&[FILLER; 40][..], extra[1])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

/// Hand-size discards take Feral Imps and no other card, for both players.
fn keep_hands(s: &mut Scenario) {
    s.prefer(&[(0, FILLER), (1, FILLER)]);
}

fn hand_size(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

fn deck(s: &Scenario, team: usize) -> u64 {
    s.state()[team]["deck"].as_u64().unwrap()
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

// ---------------------------------------------------------------------------------------------------------------
// Dark Tuner Dark Ape, Dark Wave and the Dark Synchro Summon
// ---------------------------------------------------------------------------------------------------------------

/// Dark Wave: "Target 1 non-Tuner monster you control; if that monster is used as a Synchro Material for the Synchro Summon
/// of a Dark Synchro Monster this turn, treat its level as a negative number." Dark Tuner Dark Ape (Level 2) and a Feral Imp
/// (Level 4) make Underground Arachnid (Level 6, the anime's Dark Synchro Monster): with Dark Wave on the Imp and not
/// without it (a Level 2 Dark Tuner and a Level 4 non-Tuner make no Level 6 Dark Synchro Monster by themselves). Dark Ape:
/// "If this card is used for a Synchro Summon and sent to the Graveyard: Draw 1 card." The hand is the same size at the
/// end (Dark Wave left it, a card came in) and the Deck is one card smaller; Dark Ape and the Imp are in the Graveyard.
#[test]
fn dark_ape_and_dark_wave_make_a_dark_synchro_summon_and_dark_ape_draws() {
    let [ape, wave, arachnid] = ["Dark Tuner Dark Ape", "Dark Wave", "Underground Arachnid"].map(card);
    let spawns = [mon(0, ape), mon(0, FILLER), hand(0, wave)];
    let Some(mut s) = field_extra([&[arachnid], &[]], &spawns) else { return };
    s.prefer(&[(0, FILLER), (0, ape)]);
    let (before_hand, before_deck) = (hand_size(&s, 0), deck(&s, 0));
    s.run(&[Activate(wave), SpSummon(arachnid)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", arachnid), 1, "Underground Arachnid was Dark Synchro Summoned: {}", s.context());
    assert_eq!((s.count(0, "grave", ape), s.count(0, "grave", FILLER), s.count(0, "grave", wave)), (1, 1, 1), "{}", s.context());
    assert_eq!(hand_size(&s, 0), before_hand, "Dark Wave left the hand and Dark Ape drew a card: {}", s.context());
    assert_eq!(deck(&s, 0), before_deck - 1, "one card drawn: {}", s.context());
    assert_eq!(stats(&s, 0, arachnid).2, Some(6), "{}", s.context());
}

/// The plain Dark Synchro Summon, with no Dark Wave: a Dark Tuner of Level 8 (Dark Tuner Chaos Rogue, batch 33's, the
/// same kind as Dark Ape: setcode 0x600, a Tuner) and a Level 2 non-Tuner (Thunder Kid) make Underground Arachnid, Level
/// 6 = 8 - 2; Chaos Rogue's own effect ("If this card is used for a Synchro Summon and sent to the Graveyard: Send the top
/// 5 cards of your opponent's Deck to the Graveyard") works too. A Level 4 non-Tuner (Feral Imp) with it is not offered
/// (8 - 4 = 4 is Dark Diviner's Level, not Underground Arachnid's).
#[test]
fn a_dark_tuner_minus_a_non_tuner_makes_the_dark_synchro_monsters_level() {
    let [rogue, arachnid] = ["Dark Tuner Chaos Rogue", "Underground Arachnid"].map(card);
    let Some(mut s) = field_extra([&[arachnid], &[]], &[mon(0, rogue), mon(0, THUNDER_KID)]) else { return };
    s.prefer(&[(0, rogue), (0, THUNDER_KID)]);
    let d = deck(&s, 1);
    s.run(&[SpSummon(arachnid)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", arachnid), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", rogue), s.count(0, "grave", THUNDER_KID)), (1, 1), "{}", s.context());
    assert_eq!(deck(&s, 1), d - 5, "Dark Tuner Chaos Rogue sent 5 cards of my opponent's Deck: {}", s.context());
    let Some(mut s) = field_extra([&[arachnid], &[]], &[mon(0, rogue), mon(0, FILLER)]) else { return };
    let err = s.run(&[SpSummon(arachnid)]).expect_err("8 - 4 is not 6");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

/// Without Dark Wave the same two monsters make no Dark Synchro Summon: the Dark Tuner's Level has to be the Dark Synchro
/// Monster's plus the non-Tuner's (2 is not 6 + 4), and the Synchro Monster is never offered.
#[test]
fn dark_ape_alone_with_a_level_4_monster_makes_no_underground_arachnid() {
    let [ape, arachnid] = ["Dark Tuner Dark Ape", "Underground Arachnid"].map(card);
    let Some(mut s) = field_extra([&[arachnid], &[]], &[mon(0, ape), mon(0, FILLER)]) else { return };
    let err = s.run(&[SpSummon(arachnid)]).expect_err("no Dark Synchro Summon");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", ape), 1, "{}", s.context());
}

/// Dark Tuner Dark Ape: "Cannot be used as a Synchro Material, except for the Synchro Summon of a Dark Synchro Monster."
/// Goyo Guardian (a Synchro Monster, Level 6) is offered for Nitro Synchron (a Level 2 Tuner) and Warwolf (Level 4), and
/// not for Dark Ape (also a Level 2 Tuner) and Warwolf.
#[test]
fn dark_ape_cannot_be_the_material_of_an_ordinary_synchro_monster() {
    let [ape, goyo] = ["Dark Tuner Dark Ape", "Goyo Guardian"].map(card);
    let Some(mut s) = field_extra([&[goyo], &[]], &[mon(0, NITRO), mon(0, WARWOLF)]) else { return };
    s.run(&[SpSummon(goyo)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", goyo), 1, "the control: a Level 2 Tuner and Warwolf make Goyo Guardian: {}", s.context());
    let Some(mut s) = field_extra([&[goyo], &[]], &[mon(0, ape), mon(0, WARWOLF)]) else { return };
    let err = s.run(&[SpSummon(goyo)]).expect_err("Dark Ape is no material for it");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", ape), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dark Matter (VG) and the Dark Matter Tokens
// ---------------------------------------------------------------------------------------------------------------

/// Dark Matter (VG): "When a Dark Synchro Monster is destroyed by battle or card effect: Special Summon 2 "Dark Matter
/// Tokens" (Fiend-Type/DARK/Level 1/ATK 0/DEF 0) to your side of the field in face-up Defense Position. These Tokens cannot
/// be destroyed by battle, but destroy them during the End Phase of the next turn." Team 1's Dark Hole destroys my
/// Underground Arachnid in turn 2: two Tokens of those stats stand in Defense Position, Dark Matter is in the Graveyard,
/// the Tokens are still there at the end of turn 2 and gone (not in the Graveyard: Tokens are gone) after the End Phase of turn 3.
#[test]
fn dark_matter_makes_two_tokens_that_last_until_the_end_of_the_next_turn() {
    let [matter, arachnid, token] = ["Dark Matter (VG)", "Underground Arachnid", "Dark Matter Token"].map(card);
    let Some(mut s) = field(&[], &[set(0, matter), mon(0, arachnid), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(matter)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", arachnid), 1, "Dark Hole destroyed Underground Arachnid: {}", s.context());
    assert_eq!(s.count(0, "grave", matter), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", token), 2, "two Tokens: {}", s.context());
    let t = stats(&s, 0, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(0), Some(0), Some(1), 0x8, 0x20), "Fiend, DARK, Level 1, 0/0: {}", s.context());
    assert_eq!(s.position_of(0, token), Some(4), "face-up Defense: {}", s.context());
    // The End Phase of turn 2 leaves them; the one of turn 3 takes them.
    s.run_both([&[], &[EndTurn]]).unwrap();
    assert_eq!(s.count(0, "mzone", token), 2, "turn 3 starts with both Tokens: {}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 0, "destroyed during the End Phase of the next turn: {}", s.context());
    assert_eq!(s.count(0, "grave", token), 0, "a Token is gone: {}", s.context());
}

/// Dark Matter Tokens "cannot be destroyed by battle": team 1's Warwolf (2000) attacks a Token in Defense Position (DEF 0)
/// in the turn the Tokens arrive, and both are still there; no damage is taken either (Defense Position).
#[test]
fn a_dark_matter_token_is_not_destroyed_by_battle() {
    let [matter, arachnid, token] = ["Dark Matter (VG)", "Underground Arachnid", "Dark Matter Token"].map(card);
    let spawns = [set(0, matter), mon(0, arachnid), hand(1, DARK_HOLE), hand(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(matter)], &[Activate(DARK_HOLE), Summon(WARWOLF), Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", token), 2, "both Tokens survived the attack: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Synchro Back, Cursed Prison and Cursed Synchro
// ---------------------------------------------------------------------------------------------------------------

/// Synchro Back: "Target 1 Synchro Monster you control; return that target to the Extra Deck, also during the Standby Phase
/// of your next turn, Special Summon from your Extra Deck, 1 Synchro Monster with the same name as that monster returned to
/// the Extra Deck." Goyo Guardian goes back to the Extra Deck in turn 3 and is not on the field in turn 4, and is Special
/// Summoned again in turn 5's Standby Phase.
#[test]
fn synchro_back_returns_a_synchro_monster_and_brings_it_back_in_the_next_own_standby_phase() {
    let [back, goyo] = ["Synchro Back", "Goyo Guardian"].map(card);
    let Some(mut s) = field(&[], &[set(0, back), mon(0, goyo)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(back)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", goyo), s.count(0, "extra", goyo)), (0, 1), "Goyo Guardian is in the Extra Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", back), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    assert_eq!(s.count(0, "mzone", goyo), 0, "not in the opponent's turn: {}", s.context());
    s.run_both([&[], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", goyo), s.count(0, "extra", goyo)), (1, 0), "back in my next Standby Phase: {}", s.context());
}

/// Synchro Back: "Target 1 Synchro Monster you control": with none on the field (a Warwolf only) it is not offered.
#[test]
fn synchro_back_needs_a_synchro_monster() {
    let back = card("Synchro Back");
    let Some(mut s) = field(&[], &[set(0, back), mon(0, WARWOLF)]) else { return };
    let err = s.run_both([&[EndTurn, Activate(back)], &[EndTurn]]).expect_err("no target");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Cursed Prison: "Special Summon 1 Synchro Monster from your Extra Deck in face-up Defense Position and target it with this
/// card, but it cannot activate its effects and its effects are negated, also, it cannot change its battle position. If
/// this card leaves the field, destroy that monster. If that monster leaves the field, destroy this card." Goyo Guardian comes
/// from the Extra Deck in Defense Position and Cursed Prison stays on the field; in turn 5 Dark Blade can change position
/// and Goyo Guardian cannot.
#[test]
fn cursed_prison_summons_a_synchro_monster_in_defense_position_that_cannot_change_it() {
    let [prison, goyo] = ["Cursed Prison", "Goyo Guardian"].map(card);
    let Some(mut s) = field_extra([&[goyo], &[]], &[set(0, prison), mon(0, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(prison)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", goyo), s.count(0, "extra", goyo)), (1, 0), "{}", s.context());
    assert_eq!(s.position_of(0, goyo), Some(4), "face-up Defense Position: {}", s.context());
    assert_eq!(s.count(0, "szone", prison), 1, "Cursed Prison stays on the field: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert!(offered(&s, "pos_changes").contains(&DARK_BLADE.into()), "{:?}", offered(&s, "pos_changes"));
    assert!(!offered(&s, "pos_changes").contains(&goyo.into()), "it cannot change its battle position: {:?}", offered(&s, "pos_changes"));
}

/// Cursed Prison: "If this card leaves the field, destroy that monster." Team 1's Giant Trunade returns it to my hand in turn
/// 4 and Goyo Guardian is destroyed.
#[test]
fn cursed_prison_leaving_the_field_destroys_its_monster() {
    let [prison, goyo] = ["Cursed Prison", "Goyo Guardian"].map(card);
    let Some(mut s) = field_extra([&[goyo], &[]], &[set(0, prison), hand(1, TRUNADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(prison)], &[EndTurn, Activate(TRUNADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "hand", prison), 1, "Giant Trunade returned Cursed Prison: {}", s.context());
    assert_eq!((s.count(0, "mzone", goyo), s.count(0, "grave", goyo)), (0, 1), "Goyo Guardian was destroyed: {}", s.context());
}

/// Cursed Prison: "If that monster leaves the field, destroy this card." Dark Hole destroys Goyo Guardian in turn 4 and
/// Cursed Prison follows it to the Graveyard.
#[test]
fn cursed_prison_is_destroyed_when_its_monster_leaves() {
    let [prison, goyo] = ["Cursed Prison", "Goyo Guardian"].map(card);
    let Some(mut s) = field_extra([&[goyo], &[]], &[set(0, prison), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(prison)], &[EndTurn, Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", goyo), s.count(0, "grave", prison)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "szone", prison), 0, "{}", s.context());
}

/// Cursed Synchro: "Activate only when you take battle damage from an opponent's attacking monster. Synchro Monsters cannot
/// attack." Team 1's Warwolf beats my Thunder Kid in turn 2 (1300 damage) and I activate it; in turn 3 Goyo Guardian cannot
/// attack and Dark Blade can. Not offered before such damage (the Trap is only Set, nobody attacked).
#[test]
fn cursed_synchro_stops_synchro_monsters_from_attacking_after_battle_damage() {
    let [cursed, goyo] = ["Cursed Synchro", "Goyo Guardian"].map(card);
    let spawns = [set(0, cursed), mon(0, goyo), mon(0, THUNDER_KID), mon(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[Chain(cursed)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 1300, "{}", s.context());
    assert_eq!(s.count(0, "szone", cursed), 1, "Cursed Synchro was activated: {}", s.context());
    let err = s.run_both([&[EndTurn, Attack(goyo)], &[EndTurn]]).expect_err("a Synchro Monster cannot attack");
    assert!(err.contains("can't attack") && err.contains(&format!("\"cid\":{DARK_BLADE}")) && !err.contains(&format!("\"cid\":{goyo}")), "{err}");
    clean(&s);
}

/// Cursed Synchro, the control: without it Goyo Guardian attacks (and is offered as an attacker).
#[test]
fn goyo_guardian_attacks_without_cursed_synchro() {
    let goyo = card("Goyo Guardian");
    let Some(mut s) = field(&[], &[mon(0, goyo), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(goyo)], &[EndTurn]]).unwrap();
    clean(&s);
    // Goyo Guardian's own effect takes the monster it destroys (in Defense Position) to its side.
    assert_eq!((s.count(1, "mzone", THUNDER_KID), s.count(0, "mzone", THUNDER_KID)), (0, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Beast Burial Ritual, Dimension Equilibrium and Blizzard Lizard
// ---------------------------------------------------------------------------------------------------------------

/// Beast Burial Ritual: "Target 1 Beast-Type monster you control; destroy that target, and if you do, draw 2 cards during
/// the next Standby Phase." Wolf (a Beast) is destroyed in turn 3 and I draw 2 in turn 4's Standby Phase (the next one,
/// the opponent's), not before and not again.
#[test]
fn beast_burial_ritual_destroys_a_beast_and_draws_two_in_the_next_standby_phase() {
    let ritual = card("Beast Burial Ritual");
    let Some(mut s) = field(&[], &[set(0, ritual), mon(0, WOLF), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WOLF)]);
    s.run_both([&[EndTurn, Activate(ritual)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", WOLF), s.count(0, "mzone", WARWOLF)), (1, 1), "only the Beast was destroyed: {}", s.context());
    let d = deck(&s, 0);
    s.run_both([&[EndTurn], &[]]).unwrap();
    clean(&s);
    assert_eq!(deck(&s, 0), d - 2, "two cards drawn in the next Standby Phase: {}", s.context());
    assert_eq!(s.count(0, "mzone", WARWOLF), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(deck(&s, 0), d - 2 - 1, "only the turn's own draw after it: {}", s.context());
}

/// Beast Burial Ritual: "Target 1 Beast-Type monster": Warwolf (a Beast-Warrior, not a Beast) is no target.
#[test]
fn beast_burial_ritual_needs_a_beast() {
    let ritual = card("Beast Burial Ritual");
    let Some(mut s) = field(&[], &[set(0, ritual), mon(0, WARWOLF)]) else { return };
    let err = s.run_both([&[EndTurn, Activate(ritual)], &[EndTurn]]).expect_err("no Beast");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Dimension Equilibrium: "If a Beast-Type monster you control is destroyed by battle with an opponent's attacking monster
/// and sent to the Graveyard: Special Summon the destroyed monster, and if you do, banish the attacking monster." Team 1's
/// Warwolf (2000) destroys my Wolf (1200): Wolf is back in face-up Attack Position, Warwolf is banished, and I took the
/// 800 of the battle.
#[test]
fn dimension_equilibrium_revives_the_beast_and_banishes_its_attacker() {
    let equilibrium = card("Dimension Equilibrium");
    let Some(mut s) = field(&[], &[set(0, equilibrium), mon(0, WOLF), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(equilibrium)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", WOLF), 1, "Wolf came back: {}", s.context());
    assert_eq!(s.position_of(0, WOLF), Some(1), "face-up Attack Position: {}", s.context());
    assert_eq!((s.count(1, "removed", WARWOLF), s.count(1, "grave", WARWOLF), s.count(1, "mzone", WARWOLF)), (1, 0, 0), "banished: {}", s.context());
    assert_eq!(s.count(0, "grave", equilibrium), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 800, "{}", s.context());
}

/// Dimension Equilibrium: "a Beast-Type monster you control": a Dark Blade (not a Beast) destroyed in the same battle does
/// not offer it, and Dark Blade stays in the Graveyard.
#[test]
fn dimension_equilibrium_is_for_beasts_only() {
    let equilibrium = card("Dimension Equilibrium");
    let Some(mut s) = field(&[], &[set(0, equilibrium), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(equilibrium)], &[Attack(WARWOLF)]]).expect_err("no Beast was destroyed");
    assert!(err.contains("never became possible") || !err.is_empty(), "{err}");
    clean(&s);
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.count(1, "removed", WARWOLF)), (1, 0), "{}", s.context());
}

/// Blizzard Lizard: "If this card is destroyed by battle: Inflict 800 damage to your opponent." Team 1's Warwolf (2000)
/// destroys my Blizzard Lizard (600): I take 1400 of the battle and team 1 takes 800.
#[test]
fn blizzard_lizard_inflicts_800_when_destroyed_by_battle() {
    let lizard = card("Blizzard Lizard");
    let Some(mut s) = field(&[], &[mon(0, lizard), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", lizard), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 1400, 8000 - 800), "{}", s.context());
}

/// Blizzard Lizard, the other way round: team 1's Lizard destroyed by my Dark Blade inflicts the 800 to me (its
/// opponent), and a Lizard destroyed by a card effect (Dark Hole) inflicts nothing.
#[test]
fn blizzard_lizard_burns_the_player_who_destroyed_it_by_battle_only() {
    let lizard = card("Blizzard Lizard");
    let Some(mut s) = field(&[], &[mon(0, DARK_BLADE), mon(1, lizard)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(DARK_BLADE)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", lizard), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 800, 8000 - 1200), "1800 against 600, then the Lizard's 800: {}", s.context());
    let Some(mut s) = field(&[], &[hand(0, DARK_HOLE), mon(1, lizard)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", lizard), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "not by battle: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Destruction Trigger and Tuning Collapse
// ---------------------------------------------------------------------------------------------------------------

/// Destruction Trigger: "When a card you control is destroyed by a card effect: Send the top 5 cards of your opponent's
/// Deck to the Graveyard." Team 1's Dark Hole destroys my Warwolf in turn 2: team 1's Deck is 5 cards smaller than in the
/// same duel without it.
#[test]
fn destruction_trigger_sends_the_top_five_cards_of_the_opponents_deck() {
    let trigger = card("Destruction Trigger");
    let Some(mut with) = field(&[], &[set(0, trigger), mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut with);
    with.run_both([&[Chain(trigger)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&with);
    let Some(mut without) = field(&[], &[mon(0, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut without);
    without.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!(with.count(0, "grave", trigger), 1, "it was activated: {}", with.context());
    assert_eq!(deck(&with, 1), deck(&without, 1) - 5, "five cards from the top of team 1's Deck: {}", with.context());
    assert_eq!(deck(&with, 0), deck(&without, 0), "mine untouched: {}", with.context());
    assert_eq!(with.count(1, "grave", FILLER), without.count(1, "grave", FILLER) + 5, "{}", with.context());
}

/// Destruction Trigger: "destroyed by a card effect": a monster of mine destroyed by battle does not offer it: it stays Set
/// and team 1's Deck is only 1 card smaller (its own draw).
#[test]
fn destruction_trigger_ignores_a_monster_destroyed_by_battle() {
    let trigger = card("Destruction Trigger");
    let Some(mut s) = field(&[], &[set(0, trigger), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let d = deck(&s, 1);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!((s.count(0, "szone", trigger), s.count(0, "grave", trigger)), (1, 0), "not activated: {}", s.context());
    assert_eq!(deck(&s, 1), d - 1, "only team 1's own draw: {}", s.context());
}

/// Tuning Collapse: "When a Synchro Monster(s) your opponent controls is destroyed by battle or card effect and sent to the
/// Graveyard: Target 1 of those monsters; send a number of cards from the top of your opponent's Deck to the Graveyard
/// equal to the Level of that target." Team 1's Dark Hole destroys its own Goyo Guardian (Level 6): team 1's Deck is 6 cards
/// smaller than in the same duel without Tuning Collapse.
#[test]
fn tuning_collapse_sends_cards_equal_to_the_level_of_the_destroyed_synchro_monster() {
    let [collapse, goyo] = ["Tuning Collapse", "Goyo Guardian"].map(card);
    let Some(mut with) = field(&[], &[set(0, collapse), mon(1, goyo), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut with);
    let level = with.level_of(1, goyo).unwrap();
    with.run_both([&[Chain(collapse)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&with);
    let Some(mut without) = field(&[], &[mon(1, goyo), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut without);
    without.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert_eq!(level, 6, "Goyo Guardian is Level 6");
    assert_eq!(with.count(0, "grave", collapse), 1, "it was activated: {}", with.context());
    assert_eq!(deck(&with, 1), deck(&without, 1) - level, "Level {level}: {}", with.context());
    assert_eq!(deck(&with, 0), deck(&without, 0), "{}", with.context());
}

/// Tuning Collapse: a Synchro Monster of my own destroyed does not count ("your opponent controls"), and neither does a
/// monster that is not a Synchro Monster.
#[test]
fn tuning_collapse_is_for_the_opponents_synchro_monsters_only() {
    let [collapse, goyo] = ["Tuning Collapse", "Goyo Guardian"].map(card);
    let Some(mut s) = field(&[], &[set(0, collapse), mon(0, goyo), mon(1, WARWOLF), hand(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[Chain(collapse)], &[Activate(DARK_HOLE)]]).expect_err("not offered");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "szone", collapse), 1, "still Set: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Chaos Blast (VG), Chaos Bloom, Future Destruction
// ---------------------------------------------------------------------------------------------------------------

/// Chaos Blast (VG): "Send 3 Level 1 Machine-Type monsters from your Deck to the Graveyard, then target 1 Level 4 or lower
/// monster on the field; destroy it." Three Bats go from the Deck to the Graveyard and team 1's Warwolf (Level 4) is
/// destroyed; Dark Magician (Level 7) is no target and stays.
#[test]
fn chaos_blast_sends_three_machines_and_destroys_a_level_4_or_lower_monster() {
    let blast = card("Chaos Blast (VG)");
    let Some(mut s) = field(&[BAT, BAT, BAT], &[hand(0, blast), mon(1, WARWOLF), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    let d = deck(&s, 0);
    s.run(&[Activate(blast)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", BAT), 3, "three Level 1 Machines sent as the cost: {}", s.context());
    assert_eq!(deck(&s, 0), d - 3, "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", DARK_MAGICIAN)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", blast), 1, "{}", s.context());
}

/// Chaos Blast (VG): "Send 3 Level 1 Machine-Type monsters from your Deck": with only two in the Deck the cost cannot be
/// paid and it is not offered.
#[test]
fn chaos_blast_needs_three_level_1_machines_in_the_deck() {
    let blast = card("Chaos Blast (VG)");
    let Some(mut s) = field(&[BAT, BAT], &[hand(0, blast), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let err = s.run(&[Activate(blast)]).expect_err("the cost cannot be paid");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Chaos Blast (VG), the script's note (`Duel.GetFirstTarget()` used unchecked): the target is gone before it resolves.
/// Team 1 answers with Compulsory Evacuation Device on the target, which returns to its hand (a Warwolf) or is gone for
/// good (a Loan Token, made by my Endless Loan when team 1 Summons a monster, which shatters when it leaves the field):
/// Chaos Blast then has nothing to destroy and the duel goes on without an error (the Bats were still sent: the cost).
#[test]
fn chaos_blast_with_its_target_gone_does_nothing_and_does_not_stop_the_duel() {
    let [blast, device, loan, loan_token] = ["Chaos Blast (VG)", "Compulsory Evacuation Device", "Endless Loan", "Loan Token"].map(card);
    for target in [WARWOLF, loan_token] {
        let spawns = [hand(0, blast), set(1, device), spell(0, loan), mon(1, if target == WARWOLF { WARWOLF } else { DARK_BLADE })];
        let Some(mut s) = field(&[BAT, BAT, BAT], &spawns) else { return };
        s.prefer(&[(1, target), (0, FILLER), (1, FILLER)]);
        if target == loan_token {
            // Team 1 Summons a monster in turn 2, and a Loan Token comes to its side.
            s.run_both([&[], &[Summon(FILLER)]]).unwrap();
            assert_eq!(s.count(1, "mzone", loan_token), 1, "{}", s.context());
            assert_eq!(s.count(1, "szone", device), 1, "the Device is still Set: {}", s.context());
            s.run_to(&[], |p| p["type"] == msg::SELECT_IDLECMD && p["player"] == 0).unwrap(); // turn 3
        }
        // Team 1 answers when Chaos Blast is on the chain (a chain of one link), not at the first chance it gets.
        s.run_to(&[Activate(blast)], |p| p["type"] == msg::SELECT_CHAIN && p["player"] == 1 && p["spe_count"] == 1).unwrap();
        let at = offered(&s, "selects").iter().position(|c| *c == device).expect("the Device is offered to answer");
        s.respond(Response::Chain(Some(at)).json()).unwrap();
        s.run(&[]).unwrap();
        clean(&s);
        assert_eq!(s.count(1, "grave", device), 1, "the Device answered: {}", s.context());
        assert_eq!(s.count(1, "grave", target), 0, "{target} was returned, not destroyed: {}", s.context());
        assert_eq!(s.count(1, "mzone", target), 0, "{}", s.context());
        assert_eq!((s.count(0, "grave", blast), s.count(0, "grave", BAT)), (1, 3), "{}", s.context());
    }
}

/// Chaos Bloom: "Depending on the number of "Chaos Bloom" in your Graveyard, apply 1 of these effects. 0: Destroy 1 monster
/// on the field with 1000 or less ATK." Of team 1's Thunder Kid (700) and Warwolf (2000) only the Thunder Kid can be destroyed.
#[test]
fn chaos_bloom_with_none_in_the_graveyard_destroys_a_monster_with_1000_or_less_atk() {
    let bloom = card("Chaos Bloom");
    let Some(mut s) = field(&[], &[hand(0, bloom), mon(1, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(bloom)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "mzone", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", bloom), 1, "{}", s.context());
}

/// Chaos Bloom: "1: Destroy 1 card in either player's Spell & Trap Zone." One in the Graveyard: team 1's Set Dian Keto is
/// destroyed and its Thunder Kid (a monster) is not.
#[test]
fn chaos_bloom_with_one_in_the_graveyard_destroys_a_spell_or_trap() {
    let bloom = card("Chaos Bloom");
    let Some(mut s) = field(&[], &[grave(0, bloom), hand(0, bloom), set(1, DIAN_KETO), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run(&[Activate(bloom)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", DIAN_KETO), s.count(1, "mzone", THUNDER_KID)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", bloom), 2, "{}", s.context());
}

/// Chaos Bloom: "2 or more: Destroy 1 card on the field." Two in the Graveyard: Warwolf (2000 ATK, no Spell or Trap) is destroyed.
#[test]
fn chaos_bloom_with_two_in_the_graveyard_destroys_any_card() {
    let bloom = card("Chaos Bloom");
    let spawns = [grave(0, bloom), grave(0, bloom), hand(0, bloom), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Activate(bloom)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", THUNDER_KID)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", bloom), 3, "{}", s.context());
}

/// Future Destruction: "Send cards from the top of your Deck to the Graveyard equal to the number of cards in your hand."
/// The cards left in my hand after it is played are sent: the Deck is that much smaller and my Graveyard that much larger.
#[test]
fn future_destruction_sends_as_many_cards_as_are_in_the_hand() {
    let destruction = card("Future Destruction");
    let Some(mut s) = field(&[], &[hand(0, destruction)]) else { return };
    keep_hands(&mut s);
    let (h, d, g) = (hand_size(&s, 0) - 1, deck(&s, 0), s.state()[0]["grave"].as_array().unwrap().len());
    s.run(&[Activate(destruction)]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), h, "{}", s.context());
    assert_eq!(deck(&s, 0), d - h as u64, "{h} cards from the top: {}", s.context());
    assert_eq!(s.state()[0]["grave"].as_array().unwrap().len(), g + h + 1, "the cards and Future Destruction: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Wise Core and the Wisels
// ---------------------------------------------------------------------------------------------------------------

/// Wise Core: "If this card is destroyed by a card effect: Destroy all monsters you control, then Special Summon 1 each of
/// "Meklord Emperor Wisel ∞", "Wisel Top", "Wisel Attack", "Wisel Guard", and "Wisel Carrier" from your hand, Deck or
/// Graveyard." Team 1's Fissure destroys it (the lowest ATK) in turn 2, with my Warwolf beside it: Warwolf is destroyed
/// too and the five come from the Deck (Wisel Attack, Guard, Top and Carrier of the Tag Force games stay: Meklord Emperor
/// Wisel is on the field).
#[test]
fn wise_core_destroyed_by_an_effect_brings_the_five_wisels() {
    let [core, wisel, top, attack, guard, carrier] =
        ["Wise Core", "Meklord Emperor Wisel ∞", "Wisel Top (TF6)", "Wisel Attack (TF6)", "Wisel Guard (TF6)", "Wisel Carrier (TF6)"].map(card);
    let bottom = [wisel, top, attack, guard, carrier];
    let Some(mut s) = field(&bottom, &[mon(0, core), mon(0, WARWOLF), hand(1, FISSURE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(FISSURE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", core), 1, "Fissure destroyed Wise Core: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 1, "every monster I control was destroyed: {}", s.context());
    for c in bottom {
        assert_eq!(s.count(0, "mzone", c), 1, "{c} was Special Summoned: {}", s.context());
    }
}

/// Wise Core: "Once per turn, this card cannot be destroyed by battle." Two attackers on it in Defense Position (DEF 0):
/// the first battle does not destroy it, the second does; a destruction by battle is no card effect, so no Wisel comes.
#[test]
fn wise_core_survives_one_battle_a_turn() {
    let [core, wisel, top] = ["Wise Core", "Meklord Emperor Wisel ∞", "Wisel Top (TF6)"].map(card);
    let spawns = [mon_def(0, core), mon(1, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[wisel, top], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", core), 1, "the first battle did not destroy it: {}", s.context());
    let Some(mut s) = field(&[wisel, top], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", core), 1, "the second one did: {}", s.context());
    assert_eq!((s.count(0, "mzone", wisel), s.count(0, "mzone", top)), (0, 0), "not an effect: nothing came: {}", s.context());
}

/// Wisel Guard (TF6): "If there is no "Meklord Emperor" monster on the field, destroy this card. When another monster you
/// control is targeted for an attack: You can change the attack target to this card." Team 1's Dark Blade (1800) attacks my
/// Warwolf; Wisel Guard (DEF 1200) takes the attack: Warwolf is not touched, Guard is destroyed by the 1800, and team 1's
/// Dark Blade is not (Defense Position). The control, with no Wisel Guard: Warwolf (2000) destroys Dark Blade.
#[test]
fn wisel_guard_takes_an_attack_aimed_at_another_monster() {
    let guard = card("Wisel Guard (TF6)");
    // The Meklord Emperor is placed first: a Wisel Guard placed before it is destroyed at once. It is mine, because a
    // Meklord Emperor's controller's other monsters cannot attack.
    let spawns = [mon(0, GRANEL), mon_def(0, guard), mon(0, WARWOLF), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(guard)], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", guard), 1, "the Guard took the blow: {}", s.context());
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "no battle damage: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", DARK_BLADE), s.lp(1)), (1, 8000 - 200), "{}", s.context());
}

/// Wisel Guard (TF6): "If there is no "Meklord Emperor" monster on the field, destroy this card." Alone on the field (no
/// Meklord Emperor anywhere) it is destroyed at once; with Meklord Emperor Granel beside it it stays.
#[test]
fn wisel_guard_is_destroyed_without_a_meklord_emperor_on_the_field() {
    let guard = card("Wisel Guard (TF6)");
    let Some(s) = field(&[], &[mon(0, guard)]) else { return };
    assert_eq!((s.count(0, "mzone", guard), s.count(0, "grave", guard)), (0, 1), "{}", s.context());
    let Some(s) = field(&[], &[mon(0, GRANEL), mon(0, guard)]) else { return };
    assert_eq!((s.count(0, "mzone", guard), s.count(0, "grave", guard)), (1, 0), "{}", s.context());
}

/// Wisel Guard 3 (TF6): "You can Special Summon this card (from your hand) by Tributing 1 "Wisel Guard". Once per turn, this
/// card cannot be destroyed by battle." Wisel Guard is Tributed for it in turn 1; in turn 2 team 1's two Dark Magicians
/// (2500 against its DEF 2000) attack it: the first battle does not destroy it, the second does.
#[test]
fn wisel_guard_3_is_summoned_by_tributing_a_guard_and_survives_one_battle() {
    let [guard, guard3] = ["Wisel Guard (TF6)", "Wisel Guard 3 (TF6)"].map(card);
    let spawns = [mon(0, GRANEL), mon(0, guard), hand(0, guard3), mon(1, DARK_MAGICIAN), mon(1, DARK_MAGICIAN)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(guard3)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", guard3), s.count(0, "grave", guard), s.count(0, "hand", guard3)), (1, 1, 0), "{}", s.context());
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", guard3), 1, "the first battle did not destroy it: {}", s.context());
    // The same turn's second attack.
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(guard3)], &[]]).unwrap();
    s.run_both([&[EndTurn], &[Attack(DARK_MAGICIAN), Attack(DARK_MAGICIAN)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", guard3), s.count(0, "grave", guard3)), (0, 1), "the second one did: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Meklord Emperor Creation and Battle Return
// ---------------------------------------------------------------------------------------------------------------

/// Meklord Emperor Creation: "Banish 1 "Wise Core", "Grand Core", and "Sky Core" from your Graveyard. Special Summon 1
/// "Meklord Astro Mekanikle" from your hand or Deck, ignoring the Summoning conditions, then, equip this card to it." The three
/// Cores are banished, Meklord Astro Mekanikle comes from the hand and Meklord Emperor Creation is equipped to it.
#[test]
fn meklord_emperor_creation_banishes_the_three_cores_and_summons_mekanikle() {
    let [creation, wise, grand, sky, mekanikle] =
        ["Meklord Emperor Creation", "Wise Core", "Grand Core", "Sky Core", "Meklord Astro Mekanikle"].map(card);
    let spawns = [set(0, creation), grave(0, wise), grave(0, grand), grave(0, sky), hand(0, mekanikle)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(creation)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", wise), s.count(0, "removed", grand), s.count(0, "removed", sky)), (1, 1, 1), "{}", s.context());
    assert_eq!((s.count(0, "mzone", mekanikle), s.count(0, "hand", mekanikle)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "szone", creation), 1, "Meklord Emperor Creation is equipped to it: {}", s.context());
}

/// Meklord Emperor Creation: without all three Cores in the Graveyard (Sky Core missing) it is not offered.
#[test]
fn meklord_emperor_creation_needs_all_three_cores() {
    let [creation, wise, grand, mekanikle] = ["Meklord Emperor Creation", "Wise Core", "Grand Core", "Meklord Astro Mekanikle"].map(card);
    let spawns = [set(0, creation), grave(0, wise), grave(0, grand), hand(0, mekanikle)];
    let Some(mut s) = field(&[], &spawns) else { return };
    let err = s.run_both([&[EndTurn, Activate(creation)], &[EndTurn]]).expect_err("a Core is missing");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Battle Return: "During your Battle Phase: Target 1 "Meklord Emperor" monster you control; halve its ATK until the end of
/// this turn, and if you do, it can make a second attack during each Battle Phase this turn." Meklord Emperor Granel
/// (ATK equal to my Life Points: 8000; the anime's Wisel has 0 ATK of its own) attacks Thunder Kid in turn 3 and Battle Return is chained to the declaration: it fights with half its ATK, and attacks
/// again (directly, Thunder Kid is gone): the damage is its halved ATK less Thunder Kid's 700, then the halved ATK.
#[test]
fn battle_return_halves_the_emperors_atk_and_gives_it_a_second_attack() {
    let [back, emperor] = ["Battle Return", "Meklord Emperor Granel ∞"].map(card);
    let Some(mut s) = field(&[], &[set(0, back), mon(0, emperor), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let atk = s.attack_of(0, emperor).unwrap();
    assert_eq!(atk, 8000, "as much as my Life Points: {}", s.context());
    s.run_both([&[EndTurn, Attack(emperor), ChainOnAttack(back), Attack(emperor)], &[EndTurn]]).unwrap();
    clean(&s);
    let half = atk / 2;
    assert_eq!(s.attack_of(0, emperor), Some(half), "{}", s.context());
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
    assert_eq!(s.lp(1), 8000 - (half - 700) - half, "two attacks with {half}: {}", s.context());
    assert_eq!(s.count(0, "grave", back), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Endless Loan and Toichi
// ---------------------------------------------------------------------------------------------------------------

/// Endless Loan: "If your opponent Normal or Special Summons a monster, except a "Loan Token": Activate this effect for each
/// of those monsters; Special Summon 1 "Loan Token" (Rock-Type/LIGHT/Level 1/ATK 0/DEF 0) to your opponent's side of the
/// field in Defense Position" and Toichi the Nefarious Debt Collector: "Once per turn: You can inflict 1000 damage to your
/// opponent for each "Loan Token" on the field." Team 1 Normal Summons Warwolf in turn 2: a Loan Token appears on its side
/// in Defense Position; in turn 3 Toichi inflicts 1000 for it.
#[test]
fn endless_loan_gives_the_opponent_a_loan_token_and_toichi_burns_for_it() {
    let [loan, toichi, token] = ["Endless Loan", "Toichi the Nefarious Debt Collector", "Loan Token"].map(card);
    let Some(mut s) = field(&[], &[spell(0, loan), mon(0, toichi), hand(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Summon(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", token), 1, "a Loan Token for the summoned monster: {}", s.context());
    assert_eq!(s.position_of(1, token), Some(4), "in Defense Position: {}", s.context());
    let t = stats(&s, 1, token);
    assert_eq!((t.0, t.1, t.2, t.3, t.4), (Some(0), Some(0), Some(1), 0x100, 0x10), "Rock, LIGHT, Level 1, 0/0: {}", s.context());
    s.run_both([&[EndTurn, Activate(toichi)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 1000, "1000 for the one Loan Token: {}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Toichi: "Cannot be destroyed by battle while there is a "Loan Token" on the field." Team 1's Warwolf (2000) attacks
/// Toichi (800/1300) in Defense Position; with a Loan Token on the field he stays, without it he is destroyed.
#[test]
fn toichi_cannot_be_destroyed_by_battle_while_a_loan_token_is_on_the_field() {
    let [toichi, token] = ["Toichi the Nefarious Debt Collector", "Loan Token"].map(card);
    let Some(mut s) = field(&[], &[mon_def(0, toichi), mon(1, token), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", toichi), 1, "protected by the Token: {}", s.context());
    let Some(mut s) = field(&[], &[mon_def(0, toichi), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", toichi), 1, "no Token, no protection: {}", s.context());
}

/// Endless Loan: "During your Opponent's Main Phase: Your opponent can send 1 card from their hand to the Graveyard, then
/// target 1 "Loan Token" on the field; destroy it." After the Token comes, team 1 discards a Feral Imp in its own Main Phase
/// and destroys it.
#[test]
fn the_opponent_can_pay_a_card_to_destroy_a_loan_token() {
    let [loan, token] = ["Endless Loan", "Loan Token"].map(card);
    let Some(mut s) = field(&[], &[spell(0, loan), hand(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    let g = s.count(1, "grave", FILLER);
    s.run_both([&[], &[Summon(WARWOLF), Activate(loan)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", token), 0, "the Loan Token was destroyed: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), g + 1, "for a card of the hand: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Masked Knight LV3, LV5 and LV7
// ---------------------------------------------------------------------------------------------------------------

/// Masked Knight LV3: "During your Standby Phase: You can send this face-up card to the Graveyard; Special Summon 1 "Masked
/// Knight LV5" from your hand or Deck." and LV5: "During your Standby Phase, except the turn this card is Summoned or
/// flipped face-up: You can send this face-up card to the Graveyard; Special Summon 1 "Masked Knight LV7" from your hand
/// or Deck." LV3 becomes LV5 in turn 1's Standby Phase, LV5 becomes LV7 in turn 3's, not in the turn it came.
#[test]
fn masked_knight_levels_up_in_the_standby_phase() {
    let [lv3, lv5, lv7] = ["Masked Knight LV3", "Masked Knight LV5", "Masked Knight LV7"].map(card);
    let Some(mut s) = field(&[lv5, lv7], &[mon(0, lv3)]) else { return };
    keep_hands(&mut s);
    s.run(&[Chain(lv3)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", lv3), s.count(0, "mzone", lv5), s.count(0, "mzone", lv7)), (1, 1, 0), "LV3 became LV5 in turn 1's Standby Phase: {}", s.context());
    assert_eq!(stats(&s, 0, lv5).2, Some(5), "{}", s.context());
    s.run_both([&[Chain(lv5)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", lv5), s.count(0, "mzone", lv7)), (1, 1), "LV5 became LV7 in turn 3's, not in the turn it came: {}", s.context());
    assert_eq!(stats(&s, 0, lv7).2, Some(7), "{}", s.context());
}

/// Masked Knight LV3, LV5 and LV7: "Once per turn: You can inflict 400 damage to your opponent" (LV3), 1000 (LV5), 1500 (LV7).
/// The first two cannot attack the turn they do it.
#[test]
fn masked_knights_inflict_400_1000_and_1500() {
    for (name, damage) in [("Masked Knight LV3", 400), ("Masked Knight LV5", 1000), ("Masked Knight LV7", 1500)] {
        let knight = card(name);
        let Some(mut s) = field(&[], &[mon(0, knight), mon(1, THUNDER_KID)]) else { return };
        keep_hands(&mut s);
        s.run(&[Activate(knight)]).unwrap();
        clean(&s);
        assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - damage), "{name}: {}", s.context());
        // Not twice in a turn.
        assert!(!offered(&s, "activates").contains(&knight.into()), "once per turn: {:?}", offered(&s, "activates"));
    }
}

/// Masked Knight LV3: "This card cannot attack the turn you activate this effect": after the 400 it is not offered as an
/// attacker in turn 3, and without the effect it attacks.
#[test]
fn masked_knight_cannot_attack_the_turn_it_burns() {
    let lv3 = card("Masked Knight LV3");
    let Some(mut s) = field(&[], &[mon(0, lv3), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(lv3), Attack(lv3)], &[EndTurn]]).expect_err("it cannot attack");
    assert!(err.contains("can't attack"), "{err}");
    let Some(mut s) = field(&[], &[mon(0, lv3), mon(1, THUNDER_KID)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(lv3)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Labyrinth of Kline and Spark Breaker
// ---------------------------------------------------------------------------------------------------------------

/// Labyrinth of Kline: "When an opponent's monster declares an attack: Target the attacking monster; negate the attack, and
/// if you do, switch the original ATK and DEF of that monster until the end of the next turn." Team 1's Warwolf (2000/100)
/// attacks in turn 2: the attack is negated (no damage, nothing destroyed) and Warwolf is 100/2000 afterwards.
#[test]
fn labyrinth_of_kline_negates_the_attack_and_switches_atk_and_def() {
    let kline = card("Labyrinth of Kline");
    let Some(mut s) = field(&[], &[set(0, kline), mon(0, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[ChainOnAttack(kline)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", kline), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.count(0, "mzone", THUNDER_KID)), (8000, 1), "the attack was negated: {}", s.context());
    let w = stats(&s, 1, WARWOLF);
    assert_eq!((w.0, w.1), (Some(100), Some(2000)), "ATK and DEF switched: {}", s.context());
}

/// Spark Breaker: "Once per turn: You can target 1 monster you control; destroy that target." Set, activated in turn 3: the
/// Trap is activated and, at the next chance (the CPU's answer takes it), its effect destroys my Thunder Kid (Warwolf stays)
/// and it stays on the field; the effect is not offered again in the same turn.
#[test]
fn spark_breaker_destroys_a_monster_of_mine_once_per_turn() {
    let breaker = card("Spark Breaker");
    let Some(mut s) = field(&[], &[set(0, breaker), mon(0, THUNDER_KID), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, THUNDER_KID)]);
    s.run_both([&[EndTurn, Activate(breaker)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", THUNDER_KID), s.count(0, "mzone", WARWOLF)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "szone", breaker), 1, "Spark Breaker stays: {}", s.context());
    assert!(!offered(&s, "activates").contains(&breaker.into()), "once per turn: {:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Aurora Draw (engine/overrides/c100000038.lua) and Giant Ushi Oni
// ---------------------------------------------------------------------------------------------------------------

/// An anime-cards duel whose Deck is all Dian Keto (a card that plays itself away) against a Feral Imp one, for an empty hand.
fn ketos(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[DIAN_KETO; 40][..], &[][..]), (&[FILLER; 40][..], &[][..])];
    Some(Scenario::start_anime_field(&dir, 7, 8000, decks, spawns).unwrap())
}

/// Aurora Draw: "If you control a "Meklord" monster and no other cards in your hand: Draw 2 cards." ProjectIgnis' script never
/// let it be played from the hand, and Set it asked for no Meklord monster (engine/overrides/c100000038.lua). With a
/// Meklord Emperor on my field and the other cards of the hand played away, it is offered and draws 2; with another card in the
/// hand it is not offered, and with an empty hand but no Meklord monster it is not offered either.
#[test]
fn aurora_draw_draws_two_with_a_meklord_and_no_other_card_in_the_hand() {
    let [aurora, granel] = ["Aurora Draw", "Meklord Emperor Granel ∞"].map(card);
    let Some(mut s) = ketos(&[mon(0, granel), hand(0, aurora)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&aurora.into()), "other cards in the hand: {:?}", offered(&s, "activates"));
    let others = hand_size(&s, 0) - 1;
    s.run(&vec![Activate(DIAN_KETO); others]).unwrap();
    clean(&s);
    assert_eq!(hand_size(&s, 0), 1, "only Aurora Draw is left: {}", s.context());
    assert!(offered(&s, "activates").contains(&aurora.into()), "{:?}", offered(&s, "activates"));
    let d = deck(&s, 0);
    s.run(&[Activate(aurora)]).unwrap();
    clean(&s);
    assert_eq!((hand_size(&s, 0), deck(&s, 0)), (2, d - 2), "two cards drawn: {}", s.context());
    assert_eq!(s.count(0, "grave", aurora), 1, "{}", s.context());
    // No Meklord monster: not offered even with an empty hand.
    let Some(mut s) = ketos(&[hand(0, aurora)]) else { return };
    s.run(&[]).unwrap();
    let others = hand_size(&s, 0) - 1;
    s.run(&vec![Activate(DIAN_KETO); others]).unwrap();
    assert_eq!(hand_size(&s, 0), 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&aurora.into()), "no Meklord monster: {:?}", offered(&s, "activates"));
}

/// Giant Ushi Oni: "You can Special Summon this card (from your hand) by Tributing 1 "Ushi Oni" you control. If this card
/// destroys an opponent's monster by battle: It can attack once again in a row." Ushi Oni is Tributed for it in turn 1; in turn 3
/// it (2600) destroys Thunder Kid (700) and attacks again, destroying Warwolf (2000): 1900 and 600 of damage.
#[test]
fn giant_ushi_oni_attacks_again_after_destroying_a_monster() {
    let [giant, ushi] = ["Giant Ushi Oni", "Ushi Oni"].map(card);
    let Some(mut s) = field(&[], &[mon(0, ushi), hand(0, giant), mon(1, THUNDER_KID), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(giant)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", giant), s.count(0, "grave", ushi)), (1, 1), "Ushi Oni was the Tribute: {}", s.context());
    s.run_both([&[EndTurn, Attack(giant), Chain(giant), Attack(giant)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", THUNDER_KID), s.count(1, "grave", WARWOLF)), (1, 1), "both destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1900 - 600, "{}", s.context());
}
