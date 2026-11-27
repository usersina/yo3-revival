//! Anime-only cards, group 33 (rows 6303-6338 of engine/added/cards.tsv): the video game's versions of anime cards
//! (ProjectIgnis' Video Game scope: the Tag Force texts) of Greiger, Halldor, Hunter Pace, Jakob, Jean, Kalin Kessler, Lazar,
//! Leo and Lester. These cards exist only in the anime, so every duel plays with anime cards on
//! (`Scenario::start_anime_field`). Each effect goes through the real core with both seats' client models checked after
//! every step (`duelcheck::scenario`); the texts are the anime ones. Most of the batch's Traps are Continuous or Normal Traps, so
//! they start Set (a Trap cannot be used the turn it is Set), and neither player has a Battle Phase in turn 1, so team 0's attacks
//! wait for turn 3. A card given to a team's plan with `Chain` is used at the first chance it is offered, so a test that needs a
//! later one starts the plan with `EndTurn`. The CPU's answer to an offered optional Quick Effect is to use it (Basara's and Infinite
//! Prison's second step follow their activation by themselves). The batch has no Token.
//! Covered: the cards the per-card fuzz flagged (Gjallarhorn, Grand Core, Sky Core, Granel Guard (TF6), Superficial Peace, Thousand
//! Crisscross, Infernity Zero, Skiel Attack 3 (TF6), Skiel Carrier 3 (TF6) and, in the second run, Skiel Guard (TF6),
//! Pain to Power, Double Ripple and Chariot Pile's second effect: the random duels never gave them what they wait for: an
//! Aesir monster, a destroyed Core with the five Meklord Emperor parts in the Deck, a Meklord Emperor on the field and an attack, a
//! battle damage, LP below 2000), the script notes (Thousand Crisscross and Infernity Zero at infinite LP), the signature cards (Dark
//! Tuner Doom Submarine with a real Dark Synchro Summon, the Skiel line with Skiel Attack 5, Infinity Force and Convert Ghost (TF6) with
//! the Meklord Emperors, Contaminated Earth, Gjallarhorn) and the seeded sample (Absurd Stealer, Card of Distrain, Double Ripple, Granel
//! Attack (TF6), Granel Top (TF6), Infinite Prison, Power Converter, Skiel Carrier (TF6)), with Basara, Chariot Pile, Diving Exploder
//! and Jester Queen, being cheap.
//! Dark Synchro Summon: the anime Dark Tuners and Dark Synchro Monsters carry setcodes the Dark Synchro procedure checks, and the Dark
//! Tuner's Level is the Dark Synchro Monster's plus the non-Tuner's (Dark Tuner Doom Submarine, Level 9, with a Level 3 non-Tuner:
//! Underground Arachnid, Level 6), that is, the Dark Synchro Monster's Level is the Dark Tuner's minus the non-Tuner's.
//! Meklord Emperors: the anime Meklord Emperor Granel ∞ and Skiel ∞ forbid every other monster of their controller to attack, so the
//! tests that need an attack of another monster use the opponent's field or only the Meklord Emperor's own attack.
//! Thousand Crisscross promises a "Recover" the fuzz never sees: its script sets the LP (`Duel.SetLP`), not `Duel.Recover`.
//! Contaminated Earth has an override (`engine/overrides/c100000307.lua`): the script handed `nil` to `Duel.SpecialSummonStep` when the
//! "Earthbound Immortal" it had found was no longer there to summon at resolution (the fuzz's seed 70054); that duel, which depends on
//! the pool of added cards at the time, cannot be rebuilt and no answer the scenarios can give reaches the same moment (a free-chain
//! Trap is only offered at the timings its hint declares), so the guard is covered by the tests that the Trap still works with it.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};
use yo3_server::ocg::msg;

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const WOLF: u16 = 4067; // Wolf: a Level 3 Beast, 1200/800, vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const OOKAZI: u16 = 4352; // 800 damage to the opponent
const DARK_HOLE: u16 = 4342;
const MST: u16 = 4909; // Mystical Space Typhoon
const PREMATURE_BURIAL: u16 = 4966; // pay 800 LP, revive a monster
const NEGATE_ATTACK: u16 = 4804;
const WABOKU: u16 = 4886;
const INFINITE: i64 = 19_999_998;
const KURIBOH: u16 = 4064;
const GEH: u16 = 12671; // the anime Divine Serpent Geh: a printed ∞ ATK
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK
const GRANEL: u16 = 9265; // the anime Meklord Emperor Granel ∞: its original ATK is its controller's LP
const SKIEL: u16 = 9266; // the anime Meklord Emperor Skiel ∞

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

fn deck(s: &Scenario, team: usize) -> u64 {
    s.state()[team]["deck"].as_u64().unwrap()
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`, `selects`...).
fn offered(s: &Scenario, list: &str) -> Vec<serde_json::Value> {
    s.prompt()[list].as_array().map_or(vec![], |v| v.iter().map(|c| c["cid"].clone()).collect())
}

/// A monster's (ATK, DEF, Level, race, attribute) as the client's view holds it.
fn stats(s: &Scenario, team: usize, cid: u16) -> (Option<i64>, Option<i64>, Option<i64>, i64, i64) {
    let m = s.monster(team, cid).unwrap_or_else(|| panic!("{cid} is not on team {team}'s field: {}", s.context()));
    (m["attack"].as_i64(), m["defense"].as_i64(), m["level"].as_i64(), m["race"].as_i64().unwrap(), m["attribute"].as_i64().unwrap())
}

/// Team 0 at ∞ LP (Mystik Wok on the anime Geh: 8000 and ∞ is ∞), with `spawns` in place; team 1 at 8000 (the way
/// `infinite_lp.rs` builds it).
fn infinite(spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![hand(0, WOK), mon(0, GEH)];
    field.extend_from_slice(spawns);
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, decks, &field).expect("the duel starts");
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), INFINITE, "Mystik Wok on ∞ ATK: {}", s.context());
    s.prefer(&[]);
    Some(s)
}

// ---------------------------------------------------------------------------------------------------------------
// Dark Tuner Doom Submarine and the Dark Synchro Summon
// ---------------------------------------------------------------------------------------------------------------

/// Dark Tuner Doom Submarine (Level 9, 0/300): "Cannot be used as a Synchro Material, except for the Synchro Summon of a Dark
/// Synchro Monster." With anime cards on it and a Level 3 non-Tuner (Wolf) make Underground Arachnid (Level 6, 9 = 6 + 3),
/// a real Dark Synchro Summon: both materials go to the Graveyard and the Dark Synchro Monster is on the field, face-up.
/// A Level 4 non-Tuner (Feral Imp) makes no Level 6 (9 is not 6 + 4): the Dark Synchro Monster is never offered.
#[test]
fn doom_submarine_and_a_level_3_monster_make_underground_arachnid() {
    let [dtds, arachnid] = ["Dark Tuner Doom Submarine", "Underground Arachnid"].map(card);
    let Some(mut s) = field_extra([&[arachnid], &[]], &[mon(0, dtds), mon(0, WOLF)]) else { return };
    s.run(&[SpSummon(arachnid)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", arachnid), 1, "Underground Arachnid was Dark Synchro Summoned: {}", s.context());
    assert_eq!((s.count(0, "grave", dtds), s.count(0, "grave", WOLF)), (1, 1), "both materials left the field: {}", s.context());
    assert_eq!(stats(&s, 0, arachnid).2, Some(6), "{}", s.context());
    assert_eq!(s.position_of(0, arachnid), Some(1), "face-up Attack: {}", s.context());
    // The control: a Level 4 non-Tuner.
    let Some(mut s) = field_extra([&[arachnid], &[]], &[mon(0, dtds), mon(0, FILLER)]) else { return };
    let err = s.run(&[SpSummon(arachnid)]).expect_err("9 is not 6 + 4");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
    assert_eq!(s.count(0, "mzone", dtds), 1, "{}", s.context());
}

/// The same Dark Tuner makes the other Dark Synchro Monsters the game has with their own Levels: Frozen Fitzgerald (Level 5, a
/// Beast) with a Level 4 non-Tuner Beast and Dark Diviner (Level 4, an Insect) with a Level 5 Insect are not testable with the
/// vanilla monsters at hand, but Hundred Eyes Dragon (Level 8, a Fiend) is: a Level 1 Fiend non-Tuner would make it, and a
/// Level 4 Fiend (Feral Imp) never does.
#[test]
fn doom_submarine_makes_no_dark_synchro_monster_of_a_level_that_does_not_add_up() {
    let [dtds, eyes] = ["Dark Tuner Doom Submarine", "Hundred Eyes Dragon"].map(card);
    let Some(mut s) = field_extra([&[eyes], &[]], &[mon(0, dtds), mon(0, FILLER)]) else { return };
    let err = s.run(&[SpSummon(eyes)]).expect_err("9 is not 8 + 4");
    assert!(err.contains("never became possible"), "{err}");
    clean(&s);
}

/// Dark Tuner Doom Submarine: "If you control no monsters: You can Special Summon this card from your Graveyard. You can only
/// use this effect of "Dark Tuner Doom Submarine" once per Duel." From the Graveyard with an empty field it comes to the field;
/// destroyed again (Dark Hole) it cannot come a second time, though the field is empty again. With a monster on the field it is
/// not offered at all.
#[test]
fn doom_submarine_comes_back_from_the_graveyard_once_per_duel_with_no_monsters() {
    let dtds = card("Dark Tuner Doom Submarine");
    let Some(mut s) = field(&[], &[grave(0, dtds), hand(0, DARK_HOLE)]) else { return };
    s.run(&[Activate(dtds)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", dtds), s.count(0, "grave", dtds)), (1, 0), "{}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", dtds), s.count(0, "grave", dtds)), (0, 1), "Dark Hole destroyed it: {}", s.context());
    assert!(!offered(&s, "activates").contains(&dtds.into()), "once per Duel: {:?}", offered(&s, "activates"));
    // With a monster on the field the effect is not offered.
    let Some(mut s) = field(&[], &[grave(0, dtds), mon(0, WARWOLF)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&dtds.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Contaminated Earth (engine/overrides/c100000307.lua)
// ---------------------------------------------------------------------------------------------------------------

/// Contaminated Earth (a Normal Trap, so it starts Set): "If a face-up Level 5 or higher monster(s) you control was destroyed by a card effect this turn: Special
/// Summon 1 "Earthbound Immortal" monster from your hand, Deck, or Graveyard, but it cannot attack, nor activate its effect this
/// turn." In turn 3 my Dark Magician (Level 7) is destroyed by my own Dark Hole and Earthbound Immortal Aslla piscu comes from the
/// Graveyard; it is on the field and cannot attack that turn. (In turn 1 the script's own reset, the turn's first Adjust, runs
/// after the destruction, so these tests use turn 3.)
#[test]
fn contaminated_earth_summons_an_earthbound_immortal_that_cannot_attack() {
    let [earth, aslla] = ["Contaminated Earth", "Earthbound Immortal Aslla piscu"].map(card);
    let spawns = [mon(0, DARK_MAGICIAN), hand(0, DARK_HOLE), set(0, earth), grave(0, aslla)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(DARK_HOLE), Activate(earth)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", aslla), 1, "Aslla piscu came from the Graveyard: {}", s.context());
    assert_eq!((s.count(0, "grave", DARK_MAGICIAN), s.count(0, "grave", earth)), (1, 1), "{}", s.context());
    // It cannot attack this turn.
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Activate(DARK_HOLE), Activate(earth), Attack(aslla)], &[EndTurn]]).expect_err("it cannot attack");
    assert!(err.contains("can't attack") || err.contains("never became possible"), "{err}");
    assert_eq!(s.count(0, "mzone", aslla), 1, "{}", s.context());
}

/// Contaminated Earth: "...destroyed by a card effect this turn". My Dark Magician is destroyed in turn 3 and I keep
/// Contaminated Earth Set; in turn 5 it is not offered (nothing was destroyed that turn).
#[test]
fn contaminated_earth_needs_the_destruction_to_be_this_turn() {
    let [earth, aslla] = ["Contaminated Earth", "Earthbound Immortal Aslla piscu"].map(card);
    let spawns = [mon(0, DARK_MAGICIAN), hand(0, DARK_HOLE), set(0, earth), grave(0, aslla)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(DARK_HOLE), EndTurn], &[EndTurn, EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_MAGICIAN), 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&earth.into()), "no destruction this turn: {:?}\n{}", offered(&s, "activates"), s.context());
    assert_eq!(s.count(0, "mzone", aslla), 0, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Gjallarhorn
// ---------------------------------------------------------------------------------------------------------------

/// Gjallarhorn (a Continuous Trap that is activated in the Graveyard): "If this card is in your Graveyard: You can target 1
/// "Aesir" monster you control; that face-up monster is unaffected by the effects of other Spells/Traps this turn, also,
/// after that, place this card face-up in your Spell & Trap Zone. During your 3rd End Phase after activation, inflict damage
/// to your opponent equal to the total ATK of all monsters you control, and if you do, banish this card and all monsters you
/// control." From the Graveyard it goes to my Spell & Trap Zone targeting Thor (3500 ATK); my own Dark Hole destroys the
/// opponent's Warwolf and not Thor; the card stays through two of my End Phases and at the third deals 3500 and banishes
/// itself and Thor.
#[test]
fn gjallarhorn_protects_an_aesir_and_three_end_phases_later_burns_for_the_total_atk() {
    let [gjall, thor] = ["Gjallarhorn", "Thor, Lord of the Aesir"].map(card);
    let Some(mut s) = field(&[], &[mon(0, thor), grave(0, gjall), hand(0, DARK_HOLE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Activate(gjall), Activate(DARK_HOLE), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", gjall), s.count(0, "grave", gjall)), (1, 0), "placed face-up in my Spell & Trap Zone: {}", s.context());
    assert_eq!(s.count(0, "mzone", thor), 1, "Thor was unaffected by Dark Hole: {}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the opponent's Warwolf was destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!((s.count(0, "szone", gjall), s.lp(1)), (1, 8000), "still waiting after my second End Phase: {}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 3500, "Thor's ATK in damage at the third End Phase: {}", s.context());
    assert_eq!((s.count(0, "removed", gjall), s.count(0, "removed", thor)), (1, 1), "both banished: {}", s.context());
    assert_eq!((s.count(0, "szone", gjall), s.count(0, "mzone", thor)), (0, 0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Grand Core, Sky Core and the Meklord Emperors' parts
// ---------------------------------------------------------------------------------------------------------------

/// Grand Core: "If this card is destroyed by a card effect: Destroy all monsters you control, then Special Summon 1 each of
/// "Meklord Emperor Granel ∞", "Granel Top", "Granel Attack", "Granel Guard", and "Granel Carrier" from your hand, Deck or
/// Graveyard." My own Dark Hole destroys it and the five come from the bottom of the Deck: each stands on the field, the Deck
/// is five cards smaller, and Grand Core is in the Graveyard.
#[test]
fn grand_core_destroyed_by_an_effect_brings_the_five_granel_cards() {
    let [core, top, attack, guard, carrier] = ["Grand Core", "Granel Top (TF6)", "Granel Attack (TF6)", "Granel Guard (TF6)", "Granel Carrier (TF6)"].map(card);
    let parts = [GRANEL, top, attack, guard, carrier];
    let Some(mut s) = field(&parts, &[mon(0, core), hand(0, DARK_HOLE)]) else { return };
    let before = deck(&s, 0);
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", core), 1, "{}", s.context());
    for part in parts {
        assert_eq!(s.count(0, "mzone", part), 1, "{part} is on the field: {}", s.context());
    }
    assert_eq!(deck(&s, 0), before - 5, "five cards left the Deck: {}", s.context());
}

/// Sky Core: "If this card is destroyed by a card effect: Destroy all monsters you control, then Special Summon 1 each of
/// "Meklord Emperor Skiel ∞", "Skiel Top", "Skiel Attack", "Skiel Guard", and "Skiel Carrier" from your hand, Deck or
/// Graveyard." The same through Dark Hole, with the five in the Deck.
#[test]
fn sky_core_destroyed_by_an_effect_brings_the_five_skiel_cards() {
    let [core, top, attack, guard, carrier] = ["Sky Core", "Skiel Top (TF6)", "Skiel Attack (TF6)", "Skiel Guard (TF6)", "Skiel Carrier (TF6)"].map(card);
    let parts = [SKIEL, top, attack, guard, carrier];
    let Some(mut s) = field(&parts, &[mon(0, core), hand(0, DARK_HOLE)]) else { return };
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", core), 1, "{}", s.context());
    for part in parts {
        assert_eq!(s.count(0, "mzone", part), 1, "{part} is on the field: {}", s.context());
    }
}

/// Grand Core: "Once per turn, this card cannot be destroyed by battle." Team 1's Warwolf attacks it (0 ATK, Attack Position) in
/// turn 2: Grand Core stays on the field and I take 2000.
#[test]
fn grand_core_is_not_destroyed_by_battle_once_per_turn() {
    let core = card("Grand Core");
    let Some(mut s) = field(&[], &[mon(0, core), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", core), 1, "{}", s.context());
    assert_eq!(s.lp(0), 6000, "{}", s.context());
}

/// Skiel Carrier (TF6) and the other Meklord Emperor parts: "If there is no "Meklord Emperor" monster on the field, destroy this
/// card." Alone it is destroyed (to the Graveyard); beside Meklord Emperor Skiel ∞ it stays.
#[test]
fn skiel_carrier_stays_only_beside_a_meklord_emperor() {
    let carrier = card("Skiel Carrier (TF6)");
    let Some(mut s) = field(&[], &[mon(0, carrier)]) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", carrier), s.count(0, "grave", carrier)), (0, 1), "no Meklord Emperor: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, SKIEL), mon(0, carrier)]) else { return };
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", carrier), s.count(0, "mzone", SKIEL)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Granel Guard (TF6), Skiel Carrier 3 (TF6) and Skiel Attack 3 (TF6)
// ---------------------------------------------------------------------------------------------------------------

/// Granel Guard (TF6): "When another monster you control is targeted for an attack: You can change the attack target to this
/// card." Beside Meklord Emperor Granel ∞ and a Warwolf, team 1's Warwolf attacks in turn 2; the Guard takes the attack, so
/// Granel ∞ and my Warwolf keep their places, the Guard (500/1000, Defense Position) is destroyed by 2000 ATK and I take no damage.
#[test]
fn granel_guard_takes_an_attack_aimed_at_another_monster() {
    let guard = card("Granel Guard (TF6)");
    let spawns = [mon(0, GRANEL), mon_def(0, guard), mon(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(guard)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", guard), 1, "the Guard took the attack and was destroyed: {}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(0, "mzone", GRANEL)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
}

/// Skiel Carrier 3 (TF6): "You can Special Summon this card (from your hand) by Tributing 1 "Skiel Carrier". ... Once per turn,
/// when a monster you control is targeted for an attack: You can negate the attack." It comes from the hand by Tributing Skiel
/// Carrier (the Carrier is in the Graveyard), and team 1's Warwolf's attack in turn 2 is negated: no damage, no monster lost.
#[test]
fn skiel_carrier_3_comes_by_tributing_a_carrier_and_negates_an_attack() {
    let [carrier, carrier3] = ["Skiel Carrier (TF6)", "Skiel Carrier 3 (TF6)"].map(card);
    let spawns = [mon(0, SKIEL), mon(0, carrier), hand(0, carrier3), mon(0, DARK_BLADE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(carrier3), Chain(carrier3)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", carrier3), s.count(0, "grave", carrier)), (1, 1), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "{}", s.context());
}

/// Skiel Attack 3 (TF6): "You can Special Summon this card (from your hand) by Tributing 1 "Skiel Attack". ... Once per turn: You
/// can target 1 monster you control; its ATK is halved until the End Phase, and it can attack directly this turn." From the hand by
/// Tributing Skiel Attack (in the Graveyard afterwards). In turn 3 it targets Meklord Emperor Granel ∞ (the anime one forbids every
/// other monster of its controller to attack, and its ATK is my LP): 8000 becomes 4000 and with team 1's Dark Blade on the field the
/// battle prompt offers it a direct attack, which takes team 1 to 4000. Without the effect there is no direct attack.
#[test]
fn skiel_attack_3_makes_a_monster_attack_directly_with_half_its_atk() {
    let [attack, attack3] = ["Skiel Attack (TF6)", "Skiel Attack 3 (TF6)"].map(card);
    let spawns = [mon(0, GRANEL), mon(0, attack), hand(0, attack3), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(attack3), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", attack3), s.count(0, "grave", attack)), (1, 1), "{}", s.context());
    // The control: without the effect no direct attack is offered.
    s.run_to(&[Attack(GRANEL)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    assert_eq!(s.prompt()["attacks"][0]["can_direct"], false, "{}", s.prompt());
    // With it: Granel ∞ at half its ATK attacks directly.
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(attack3), EndTurn], &[EndTurn]]).unwrap();
    s.run_to(&[Activate(attack3), Attack(GRANEL)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, GRANEL), Some(4000), "halved until the End Phase: {}", s.context());
    assert_eq!(s.prompt()["attacks"][0]["can_direct"], true, "{}", s.prompt());
}

// ---------------------------------------------------------------------------------------------------------------
// Superficial Peace
// ---------------------------------------------------------------------------------------------------------------

/// Superficial Peace (a Normal Trap): "During this turn, monsters cannot be destroyed by battle, also, if a monster you control
/// inflicts battle damage to your opponent: Target 1 Spell/Trap your opponent controls; destroy it." In turn 3 my Warwolf (2000)
/// attacks team 1's Feral Imp (1300): the Imp is not destroyed, team 1 takes 700 and its Set Dark Hole is destroyed.
#[test]
fn superficial_peace_stops_battle_destruction_and_destroys_a_spell_or_trap_after_battle_damage() {
    let peace = card("Superficial Peace");
    let spawns = [set(0, peace), mon(0, WARWOLF), mon(1, FILLER), set(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(peace), Attack(WARWOLF)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", FILLER), 1, "the Imp was not destroyed by battle: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 700, "{}", s.context());
    assert_eq!((s.count(1, "szone", DARK_HOLE), s.count(1, "grave", DARK_HOLE)), (0, 1), "the Set card was destroyed: {}", s.context());
    // The control: without it the Imp is destroyed and the Set card stays.
    let Some(mut s) = field(&[], &[mon(0, WARWOLF), mon(1, FILLER), set(1, DARK_HOLE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(WARWOLF)], &[EndTurn]]).unwrap();
    assert_eq!((s.count(1, "mzone", FILLER), s.count(1, "szone", DARK_HOLE)), (0, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Thousand Crisscross (and at ∞)
// ---------------------------------------------------------------------------------------------------------------

/// Thousand Crisscross (a Continuous Trap): "While your LP is less than 2000: You can make your LP become 2000. After this effect
/// is used 3 times, destroy this card." At 1000 LP the activation is the first use (2000); two Premature Burials (800 each) bring
/// me to 1200 and each time it is used again at once; the third use destroys it. It is not offered at 2000 LP. (It sets the
/// LP, never recovers them, which is why the per-card fuzz never saw a "Recover".)
#[test]
fn thousand_crisscross_makes_the_lp_2000_three_times() {
    let cross = card("Thousand Crisscross");
    let spawns = [set(0, cross), hand(0, PREMATURE_BURIAL), hand(0, PREMATURE_BURIAL), grave(0, WARWOLF), grave(0, DARK_BLADE)];
    let Some(mut s) = duel(7, 1000, &[], &spawns) else { return };
    keep_hands(&mut s);
    s.run(&[EndTurn]).unwrap();
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    s.run(&[Activate(cross)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.count(0, "szone", cross)), (2000, 1), "the first use: {}", s.context());
    assert!(!offered(&s, "activates").contains(&cross.into()), "not at 2000 LP: {:?}", offered(&s, "activates"));
    s.run(&[Activate(PREMATURE_BURIAL)]).unwrap();
    assert_eq!(s.lp(0), 1200, "{}", s.context());
    s.run(&[Activate(cross)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.count(0, "szone", cross)), (2000, 1), "the second use: {}", s.context());
    s.run(&[Activate(PREMATURE_BURIAL)]).unwrap();
    assert_eq!(s.lp(0), 1200, "{}", s.context());
    s.run(&[Activate(cross)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 2000, "the third use: {}", s.context());
    assert_eq!((s.count(0, "szone", cross), s.count(0, "grave", cross)), (0, 1), "destroyed after the third: {}", s.context());
}

/// Thousand Crisscross at ∞ LP (script note: it sets LP and works an amount out from LP): ∞ is not less than 2000, so the
/// activation changes nothing (the LP stay ∞, not 2000), the card stays face-up and its effect is not offered.
#[test]
fn thousand_crisscross_at_infinite_lp_does_nothing() {
    let cross = card("Thousand Crisscross");
    let Some(mut s) = infinite(&[set(0, cross)]) else { return };
    s.run_both([&[EndTurn, Activate(cross)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), INFINITE, "{}", s.context());
    assert_eq!(s.count(0, "szone", cross), 1, "still on the field: {}", s.context());
    assert!(!offered(&s, "activates").contains(&cross.into()), "{:?}", offered(&s, "activates"));
}

// ---------------------------------------------------------------------------------------------------------------
// Infernity Zero (and at ∞)
// ---------------------------------------------------------------------------------------------------------------

/// Infernity Zero: "When your opponent activates a card or effect that would inflict damage while your LP is 2000 or less: You
/// can discard all other cards in your hand; Special Summon this card from your hand. You do not lose the Duel even if your LP
/// become 0." At 1500 LP team 1's Ookazi is answered from the hand: every other card is discarded, Zero is Special Summoned and
/// the 800 damage leaves 700.
#[test]
fn infernity_zero_discards_the_hand_and_comes_in_answer_to_damage() {
    let zero = card("Infernity Zero");
    let Some(mut s) = duel(7, 1500, &[], &[hand(0, zero), hand(1, OOKAZI)]) else { return };
    let imps = s.state()[0]["hand"].as_array().unwrap().len() - 1;
    assert!(imps >= 5, "{}", s.context());
    keep_hands(&mut s);
    s.run_both([&[Chain(zero)], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", zero), 1, "Zero was Special Summoned: {}", s.context());
    assert_eq!(s.count(0, "hand", FILLER), 0, "the rest of the hand was discarded: {}", s.context());
    assert_eq!(s.lp(0), 700, "{}", s.context());
    assert_eq!(s.count(1, "grave", OOKAZI), 1, "{}", s.context());
}

/// Infernity Zero: "You do not lose the Duel even if your LP become 0." At 500 LP the same Ookazi takes me to 0 and the duel goes
/// on (a turn later it is still being played).
#[test]
fn infernity_zero_keeps_the_duel_going_at_zero_lp() {
    let zero = card("Infernity Zero");
    let Some(mut s) = duel(7, 500, &[], &[hand(0, zero), hand(1, OOKAZI)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(zero)], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 0, "{}", s.context());
    assert_eq!(s.count(0, "mzone", zero), 1, "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", zero), 1, "the duel went on: {}", s.context());
}

/// Infernity Zero at ∞ LP (script note: it works an amount out from LP: "LP 2000 or less"): ∞ is more than 2000, so team 1's
/// Ookazi cannot be answered; Zero stays in the hand and the LP stay ∞.
#[test]
fn infernity_zero_at_infinite_lp_cannot_be_summoned() {
    let zero = card("Infernity Zero");
    let Some(mut s) = infinite(&[hand(0, zero), hand(1, OOKAZI)]) else { return };
    s.prefer(&[(0, KURIBOH), (1, KURIBOH)]);
    s.run_both([&[EndTurn], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", OOKAZI), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.count(0, "hand", zero), s.count(0, "mzone", zero)), (INFINITE, 1, 0), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Skiel Attack 5, Granel Attack, Granel Top (TF6): the Meklord Emperors' parts at work
// ---------------------------------------------------------------------------------------------------------------

/// Skiel Attack 5 (TF6): "Cannot be Normal Summoned/Set. Must first be Special Summoned by Tributing 1 "Skiel Attack 3". ...
/// Monsters you control can attack directly." From the hand by Tributing Skiel Attack 3 (it is in the Graveyard); in turn 3 the
/// battle prompt offers Meklord Emperor Granel ∞ a direct attack with team 1's Dark Blade on the field, which it does not
/// without Skiel Attack 5 (two Skiel Attack 3s' worth of the same field: just the Meklord Emperor and Skiel Attack 3).
#[test]
fn skiel_attack_5_lets_every_monster_attack_directly() {
    let [attack3, attack5] = ["Skiel Attack 3 (TF6)", "Skiel Attack 5 (TF6)"].map(card);
    let spawns = [mon(0, GRANEL), mon(0, attack3), hand(0, attack5), mon(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[SpSummon(attack5), EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", attack5), s.count(0, "grave", attack3)), (1, 1), "{}", s.context());
    s.run_to(&[Attack(GRANEL)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    assert_eq!(s.prompt()["attacks"][0]["can_direct"], true, "{}", s.prompt());
    // The control.
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, attack3), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    s.run_to(&[Attack(GRANEL)], |p| p["type"] == msg::SELECT_BATTLECMD).unwrap();
    assert_eq!(s.prompt()["attacks"][0]["can_direct"], false, "{}", s.prompt());
}

/// Granel Attack (TF6): "At the end of the Damage Step, if a "Meklord Emperor" monster you control attacks a Defense Position
/// monster, it can attack once again in a row." Meklord Emperor Granel ∞ (8000 ATK, my LP) attacks one of team 1's two Feral Imps
/// in Defense Position and then the other, in one Battle Phase: both are destroyed. Without Granel Attack only the first falls.
#[test]
fn granel_attack_lets_a_meklord_emperor_attack_again_after_a_defense_position_monster() {
    let attack = card("Granel Attack (TF6)");
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, attack), mon_def(1, FILLER), mon_def(1, FILLER)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(GRANEL), Attack(GRANEL)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", FILLER), 0, "both Imps were destroyed: {}", s.context());
    // The control: Granel ∞ alone attacks once.
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon_def(1, FILLER), mon_def(1, FILLER)]) else { return };
    keep_hands(&mut s);
    let err = s.run_both([&[EndTurn, Attack(GRANEL), Attack(GRANEL)], &[EndTurn]]).expect_err("only one attack");
    assert!(err.contains("can't attack"), "{err}");
    assert_eq!(s.count(1, "mzone", FILLER), 1, "{}", s.context());
}

/// Granel Top (TF6): "Once per turn: You can target 1 monster your opponent controls; negate its effect(s) until the End Phase."
/// Marshmallon ("Cannot be destroyed by battle") is attacked by Meklord Emperor Granel ∞ in turn 3: it survives; with its effect
/// negated by Granel Top first, the same attack destroys it.
#[test]
fn granel_top_negates_an_effect_monster_until_the_end_phase() {
    let top = card("Granel Top (TF6)");
    let marshmallon = card("Marshmallon");
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, top), mon(1, marshmallon)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Attack(GRANEL)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "mzone", marshmallon), 1, "the control: it cannot be destroyed by battle: {}", s.context());
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, top), mon(1, marshmallon)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(top), Attack(GRANEL)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", marshmallon), s.count(1, "grave", marshmallon)), (0, 1), "negated, it was destroyed: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Infinity Force (TF6) and Convert Ghost (TF6)
// ---------------------------------------------------------------------------------------------------------------

/// Infinity Force (TF6): "When you take damage while you control a "Meklord Emperor" monster: Destroy all monsters your opponent
/// controls." Team 1's Ookazi takes 800 from me in turn 2 and Infinity Force, Set beside Meklord Emperor Granel ∞, destroys team 1's
/// Warwolf and Feral Imp; my Granel stays. Without a Meklord Emperor it is not offered.
#[test]
fn infinity_force_destroys_the_opponents_monsters_when_i_take_damage() {
    let force = card("Infinity Force (TF6)");
    let spawns = [mon(0, GRANEL), set(0, force), mon(1, WARWOLF), mon(1, FILLER), hand(1, OOKAZI)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(force)], &[Activate(OOKAZI)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", FILLER)), (1, 0), "team 1's monsters were destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", GRANEL), 1, "{}", s.context());
    assert_eq!(s.count(0, "grave", force), 1, "{}", s.context());
}

/// Convert Ghost (TF6): "When your opponent activates a card or effect while you control a "Meklord Emperor" monster: Banish 1
/// Machine-Type monster from your Graveyard; negate that activation, and if you do, destroy that card, also, after that, Set this
/// card face-down instead of sending it to the Graveyard." Team 1's Dark Hole in turn 2 is negated and destroyed, with Skiel Top
/// banished from my Graveyard; Granel ∞ and my other monsters are untouched, and Convert Ghost is Set again.
#[test]
fn convert_ghost_negates_a_card_and_sets_itself_again() {
    let [ghost, top] = ["Convert Ghost (TF6)", "Skiel Top (TF6)"].map(card);
    let spawns = [mon(0, GRANEL), mon(0, DARK_BLADE), set(0, ghost), grave(0, top), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, top)]);
    s.run_both([&[Chain(ghost)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "Dark Hole was negated and destroyed: {}", s.context());
    assert_eq!((s.count(0, "mzone", GRANEL), s.count(0, "mzone", DARK_BLADE)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "removed", top), 1, "the cost: {}", s.context());
    assert_eq!((s.count(0, "szone", ghost), s.count(0, "grave", ghost)), (1, 0), "Set again, not in the Graveyard: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Power Converter, Double Ripple, Absurd Stealer
// ---------------------------------------------------------------------------------------------------------------

/// Power Converter: "Equip only to a monster you control. Once per turn, during your Main Phase 1: You can gain LP equal to half
/// the ATK of that monster by reducing its ATK to 0 until the End Phase." Equipped to Warwolf (2000): +1000 LP and Warwolf's ATK is 0;
/// the effect is used once per turn.
#[test]
fn power_converter_trades_half_the_atk_for_life_points() {
    let converter = card("Power Converter");
    let Some(mut s) = field(&[], &[hand(0, converter), mon(0, WARWOLF)]) else { return };
    s.run(&[Activate(converter), Activate(converter)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 9000, "half of 2000 gained: {}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(0), "{}", s.context());
    assert!(!offered(&s, "activates").contains(&converter.into()), "once per turn: {:?}", offered(&s, "activates"));
}

/// Double Ripple: "Send 2 face-up monsters you control to the Graveyard (1 Tuner and 1 non-Tuner), whose total Levels equal 7,
/// then Special Summon both 1 "Power Tool Dragon" and 1 "Ancient Fairy Dragon" in Defense Position from your Extra Deck." Junk
/// Synchron (Level 3, a Tuner) and Warwolf (Level 4) go to the Graveyard and both Dragons come in Defense Position.
#[test]
fn double_ripple_sends_a_tuner_and_a_non_tuner_of_level_7_for_two_dragons() {
    let [ripple, junk, power_tool, fairy] = ["Double Ripple", "Junk Synchron", "Power Tool Dragon", "Ancient Fairy Dragon"].map(card);
    let Some(mut s) = field_extra([&[power_tool, fairy], &[]], &[hand(0, ripple), mon(0, junk), mon(0, WARWOLF)]) else { return };
    s.run(&[Activate(ripple)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", junk), s.count(0, "grave", WARWOLF)), (1, 1), "{}", s.context());
    for dragon in [power_tool, fairy] {
        assert_eq!(s.count(0, "mzone", dragon), 1, "{dragon} came from the Extra Deck: {}", s.context());
        assert_eq!(s.position_of(0, dragon), Some(4), "face-up Defense: {}", s.context());
    }
    // Levels that do not add up (Warwolf, Level 4, and a Level 4 Tuner would be 8): not offered.
    let Some(mut s) = field_extra([&[power_tool, fairy], &[]], &[hand(0, ripple), mon(0, junk), mon(0, FILLER), mon(0, FILLER)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "activates").contains(&ripple.into()), "Junk Synchron 3 + Feral Imp 4: {:?}", offered(&s, "activates"));
    let Some(mut s) = field_extra([&[power_tool, fairy], &[]], &[hand(0, ripple), mon(0, WARWOLF), mon(0, FILLER)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&ripple.into()), "no Tuner: {:?}", offered(&s, "activates"));
}

/// Absurd Stealer: "If this card is Normal Summoned: Target 1 Attack Position Level 4 or lower monster your opponent controls; its
/// ATK is halved, and if you do, this card's ATK becomes equal to the DEF of the targeted monster until the End Phase." Against
/// team 1's Dark Blade (1800/1500): Dark Blade has 900 and Absurd Stealer (100) 1500.
#[test]
fn absurd_stealer_halves_a_monsters_atk_and_takes_its_def() {
    let stealer = card("Absurd Stealer");
    let Some(mut s) = field(&[], &[hand(0, stealer), mon(1, DARK_BLADE)]) else { return };
    s.run(&[Summon(stealer)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(1, DARK_BLADE), Some(900), "{}", s.context());
    assert_eq!(s.attack_of(0, stealer), Some(1500), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Card of Distrain and Infinite Prison
// ---------------------------------------------------------------------------------------------------------------

/// Card of Distrain: "If your opponent controls 2 or more Set cards in their Spell & Trap Zone: Target 1 of those cards; that Set
/// card cannot be activated this turn, also each time your opponent activates a Set card in their Spell & Trap Zone this turn, they
/// take 800 damage immediately after it resolves." Team 1 has Negate Attack and Waboku Set; I target Waboku, attack with Warwolf in
/// turn 3 and team 1 answers with Negate Attack: the attack is negated and team 1 takes 800 (2000 would have been the attack).
#[test]
fn card_of_distrain_hurts_for_each_set_card_activated() {
    let distrain = card("Card of Distrain");
    let spawns = [hand(0, distrain), mon(0, WARWOLF), set(1, NEGATE_ATTACK), set(1, WABOKU)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(1, WABOKU), (0, FILLER), (1, FILLER)]);
    s.run_both([&[EndTurn, Activate(distrain), Attack(WARWOLF)], &[EndTurn, ChainOnAttack(NEGATE_ATTACK)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", NEGATE_ATTACK), 1, "Negate Attack was activated: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 800, "800 after it resolved, and no attack damage: {}", s.context());
    assert_eq!(s.count(1, "szone", WABOKU), 1, "{}", s.context());
}

/// Card of Distrain: "If your opponent controls 2 or more Set cards": with one Set card it is not offered.
#[test]
fn card_of_distrain_needs_two_set_cards() {
    let distrain = card("Card of Distrain");
    let Some(mut s) = field(&[], &[hand(0, distrain), set(1, WABOKU)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates").contains(&distrain.into()), "{:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[hand(0, distrain), set(1, WABOKU), set(1, NEGATE_ATTACK)]) else { return };
    s.run(&[]).unwrap();
    assert!(offered(&s, "activates").contains(&distrain.into()), "{:?}", offered(&s, "activates"));
}

/// Infinite Prison (a Continuous Trap): "Once per turn: You can discard 1 card, then target 1 Level 4 or lower monster in your
/// Graveyard; place it face-up in your Spell & Trap Zone as a Spell. If this card is destroyed and sent to the Graveyard: Return
/// all monsters placed by this card's effect in your Spell & Trap Zone to the hand." Warwolf goes from my Graveyard to my Spell &
/// Trap Zone (a Feral Imp is discarded for it); when my Mystical Space Typhoon destroys Infinite Prison, Warwolf is in my hand.
#[test]
fn infinite_prison_places_a_monster_as_a_spell_and_returns_it_when_destroyed() {
    let prison = card("Infinite Prison");
    let spawns = [set(0, prison), grave(0, WARWOLF), set(0, MST)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF), (0, prison)]);
    let imps = s.count(0, "grave", FILLER);
    s.run_both([&[EndTurn, Activate(prison)], &[EndTurn]]).unwrap(); // the CPU's answer then uses the Quick Effect itself
    clean(&s);
    assert_eq!(s.count(0, "szone", WARWOLF), 1, "Warwolf is a Spell in my Spell & Trap Zone: {}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 0, "{}", s.context());
    assert!(s.count(0, "grave", FILLER) > imps, "a card was discarded: {}", s.context());
    s.prefer(&[(0, prison)]);
    s.run_both([&[EndTurn, Activate(MST)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", prison), 1, "Infinite Prison was destroyed: {}", s.context());
    assert_eq!((s.count(0, "hand", WARWOLF), s.count(0, "szone", WARWOLF)), (1, 0), "Warwolf returned to my hand: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Basara, Chariot Pile, Diving Exploder and Jester Queen
// ---------------------------------------------------------------------------------------------------------------

/// Basara (a Continuous Trap): "Once per turn: You can Tribute 1 monster, then target 1 monster on the field whose Level is higher
/// than the Tributed monster's; destroy it, and if you do, inflict 800 damage to its controller." Feral Imp (Level 4) is Tributed for
/// team 1's Dark Magician (Level 7): destroyed, and team 1 takes 800.
#[test]
fn basara_tributes_a_monster_to_destroy_a_higher_level_one_and_burn_its_controller() {
    let basara = card("Basara");
    let Some(mut s) = field(&[], &[set(0, basara), mon(0, FILLER), mon(1, DARK_MAGICIAN)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, FILLER), (1, DARK_MAGICIAN)]);
    s.run(&[EndTurn, Activate(basara)]).unwrap(); // the CPU's answer then uses the Quick Effect itself
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_MAGICIAN), 1, "{}", s.context());
    assert_eq!(s.count(0, "mzone", FILLER), 0, "the Tribute: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 800, "{}", s.context());
}

/// Chariot Pile (a Continuous Trap): "Once per turn, during your Main Phase: You can inflict 300 damage to your opponent. Once per
/// turn, if your opponent declares a direct attack: You can pay 800 LP; destroy the attacking monster." In turn 1 it does 300; in
/// turn 2 team 1's Warwolf attacks me directly and it is destroyed for 800 LP.
#[test]
fn chariot_pile_burns_in_my_main_phase_and_destroys_a_direct_attacker() {
    let pile = card("Chariot Pile");
    let Some(mut s) = field(&[], &[spell(0, pile)]) else { return };
    s.run(&[Activate(pile)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 300, "{}", s.context());
    let Some(mut s) = field(&[], &[spell(0, pile), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(pile)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "the direct attacker was destroyed: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 800, "I paid 800, no battle damage: {}", s.context());
}

/// Diving Exploder: "(Quick Effect): You can send this card from your hand to the Graveyard; switch the original ATK/DEF of all
/// monsters currently on the field, until the End Phase." Warwolf (2000/100) is 100/2000 and team 1's Dark Blade (1800/1500) is
/// 1500/1800 for the turn.
#[test]
fn diving_exploder_switches_the_original_atk_and_def_of_all_monsters() {
    let exploder = card("Diving Exploder");
    let Some(mut s) = field(&[], &[hand(0, exploder), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    s.run(&[Activate(exploder)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", exploder), 1, "{}", s.context());
    assert_eq!((stats(&s, 0, WARWOLF).0, stats(&s, 0, WARWOLF).1), (Some(100), Some(2000)), "{}", s.context());
    assert_eq!((stats(&s, 1, DARK_BLADE).0, stats(&s, 1, DARK_BLADE).1), (Some(1500), Some(1800)), "{}", s.context());
}

/// Jester Queen: "If this card is Normal or Special Summoned: Destroy all cards in your Spell & Trap Zone. If you control no other
/// monsters, for each card in your Spell & Trap Zone, this card gains 1 additional attack during each Battle Phase." Summoned with
/// two Set cards of mine, both are destroyed; in turn 3 with one Spell Set again it attacks twice (a direct attack each time, 800).
#[test]
fn jester_queen_destroys_my_spells_and_traps_and_attacks_once_more_for_each() {
    let queen = card("Jester Queen");
    let Some(mut s) = field(&[], &[hand(0, queen), set(0, DARK_HOLE), set(0, DIAN_KETO), hand(0, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Summon(queen), SetSpell(DIAN_KETO), Attack(queen), Attack(queen)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", DARK_HOLE), s.count(0, "grave", DIAN_KETO)), (1, 1), "both Set cards were destroyed: {}", s.context());
    assert_eq!(s.lp(1), 8000 - 1600, "two direct attacks of 800: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Skiel Guard (TF6) and Pain to Power (flagged by the second fuzz run)
// ---------------------------------------------------------------------------------------------------------------

/// Skiel Guard (TF6): "Once per turn, when a monster you control is targeted for an attack: You can negate the attack." Team 1's
/// Warwolf attacks in turn 2 beside Meklord Emperor Granel ∞ and a Dark Blade of mine: negated, I take nothing and lose nothing.
#[test]
fn skiel_guard_negates_an_attack_on_a_monster_of_mine() {
    let guard = card("Skiel Guard (TF6)");
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, guard), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(guard)], &[EndTurn, Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000, "the attack was negated: {}", s.context());
    assert_eq!((s.count(0, "mzone", GRANEL), s.count(0, "mzone", guard), s.count(0, "mzone", DARK_BLADE)), (1, 1, 1), "{}", s.context());
}

/// Pain to Power (an Equip Spell): "If a monster you control other than the equipped monster attacks and you take battle damage: The
/// equipped monster gains ATK equal to the damage you took until the end of this turn." Equipped to Warwolf (2000); my Feral Imp (1300)
/// attacks team 1's Dark Blade (1800) in turn 3 and I take 500: Warwolf has 2500.
#[test]
fn pain_to_power_gives_the_equipped_monster_the_battle_damage_i_took() {
    let pain = card("Pain to Power");
    let Some(mut s) = field(&[], &[hand(0, pain), mon(0, WARWOLF), mon(0, FILLER), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Activate(pain), Attack(FILLER)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!(s.lp(0), 8000 - 500, "{}", s.context());
    assert_eq!(s.attack_of(0, WARWOLF), Some(2500), "{}", s.context());
}
