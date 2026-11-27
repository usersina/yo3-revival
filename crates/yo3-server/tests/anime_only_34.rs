//! Anime-only cards, group 34 (rows 6339-6369 of engine/added/cards.tsv): the video game's versions of anime cards
//! (ProjectIgnis' Video Game scope: the Tag Force texts, shown in the anime too) of Luna, Nicolas, Primo, Rex Goodwin, Roman
//! Goodwin, Sector Security, Sherry LeBlanc, Tetsu Trudge, Vizor, Yusei Fudo and Z-one, with Zero Hole (GX). These cards exist
//! only in the anime, so every duel plays with anime cards on (`Scenario::start_anime_field`). Each effect goes through the
//! real core with both seats' client models checked after every step (`duelcheck::scenario`); the texts are the anime ones.
//! Every Trap is Set first (it cannot be used the turn it is Set), and neither player has a Battle Phase in turn 1, so team 0's
//! attacks wait for turn 3. A Trap given to a team's plan with `Chain` is used at the first chance it is offered.
//! Covered: the Token (Mask Token, made by Imitation: a copy of a Slime Token's stats (the Slime Tokens come from Multiple
//! Slime) on the opponent's side, half its ATK in damage, shattered by Dark Hole), the cards the per-card fuzz flagged (Hook the Hidden Knight, Skiel Carrier 5 (TF6), Wisel
//! Top 3 (TF6), Dark Tuner - Dark Goddess Witaka, Passion of Baimasse, Dark Tuner Chaos Rogue, Discord Counter, Final Penalty,
//! Mach Synchron, Synchro Spirits, Empress's Crown: the random duels never gave them what they wait for: a Meklord Emperor on
//! the field, an Earthbound Immortal, a Synchro Monster and its materials, a Timelord, a Fiend destroyed in battle), the
//! signature cards (Dark Tuner Spider Cocoon with a real Dark Synchro Summon, Witaka's Level change into one, Altar of the
//! Bound Deity, Time Angel, Gate Defender, Terminal Countdown, Defender's Mind) and the seeded sample (Chain Close, Double
//! Type Rescue, Empress's Staff, Imitation, Synchro Monument, Zero Hole (GX)).
//! The fuzz's one failing duel (Dark Tuner Spider Cocoon, seed 70495: the engine reported a "checkop" debug line) no longer
//! fails with the cards of the tree now: the line came from another batch's script and is gone from it.
//! No script note in this batch, so no override. Not covered: Divergence, Twin Vortex, Fleur de Vertiges, Floral Shield, For
//! Our Dreams, Climactic Barricade and High and Low beyond the fuzz and the smoke test (the cheaper cards of the batch).
//! Dark Synchro Summon: a Dark Synchro Monster's Level is the Dark Tuner's minus the non-Tuner's (Spider Cocoon, Level 5, and
//! a Level 1 Insect make Dark Diviner, Level 4; Chaos Rogue, Level 8, and a Level 4 Insect make it too).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

// The game's own cards.
const FILLER: u16 = 4012; // Feral Imp: both Decks' filler, a vanilla Level 4 (DARK Fiend, 1300/1400)
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: Level 4, 2000/100, no effect
const THUNDER_KID: u16 = 4217; // Thunder, 700/600 vanilla
const DARK_MAGICIAN: u16 = 4041; // Level 7 Spellcaster, 2500/2100, vanilla
const DARK_BLADE: u16 = 5581; // Level 4 Warrior, 1800/1500, vanilla
const KATTAPILLAR: u16 = 4505; // a Level 1 Insect, 250/300, vanilla
const BIG_INSECT: u16 = 4055; // a Level 4 Insect, 1200/1500, vanilla
const INSECT_KNIGHT: u16 = 6235; // a Level 4 Insect, 1900/1500, vanilla
const BASIC_INSECT: u16 = 4056; // a Level 2 Insect, 500/700, vanilla
const DIAN_KETO: u16 = 4348; // gain 1000 LP
const DARK_HOLE: u16 = 4342;
const NITRO: u16 = 7700; // Nitro Synchron: a Level 2 Tuner
const MUMMY: u16 = 5420; // Giant Axe Mummy: Level 5, 1700/2000
const MAN_EATER_BUG: u16 = 4507; // FLIP: target 1 monster on the field; destroy it
const MASK_TOKEN: u16 = 9659; // the batch's Mask Token (another card is also named so)
const GRANEL: u16 = 9265; // the anime Meklord Emperor Granel: a Meklord Emperor keeps the Wisels and Skiels alive

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn hand(team: u32, cid: u16) -> Spawn { Spawn { team, place: Hand, cid } }
fn mon(team: u32, cid: u16) -> Spawn { Spawn { team, place: Monster, cid } }
fn mon_def(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterDefense, cid } }
fn mon_set(team: u32, cid: u16) -> Spawn { Spawn { team, place: MonsterSet, cid } }
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

/// A Field Spell: the Earthbound Immortals are destroyed when no face-up one is on the field.
fn umi() -> u16 { card("Umi") }

fn never_possible(err: &str) {
    assert!(err.contains("never became possible"), "{err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Hook the Hidden Knight
// ---------------------------------------------------------------------------------------------------------------

/// Hook the Hidden Knight: "When this face-up Attack Position card is targeted for an attack: Change this card and the
/// attacking monster to Defense Position. If this card is changed from Attack Position to Defense Position: Inflict 800
/// damage to your opponent." Team 1's Warwolf (2000) attacks it (1600) in turn 2: both are face-up Defense Position, nobody
/// is destroyed, I take no damage and team 1 takes 800.
#[test]
fn hook_the_hidden_knight_puts_itself_and_the_attacker_in_defense_and_burns_800() {
    let hook = card("Hook the Hidden Knight");
    let Some(mut s) = field(&[], &[mon(0, hook), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.position_of(0, hook), s.position_of(1, WARWOLF)), (Some(4), Some(4)), "both face-up Defense: {}", s.context());
    assert_eq!((s.count(0, "mzone", hook), s.count(1, "mzone", WARWOLF)), (1, 1), "no one was destroyed: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 800), "{}", s.context());
}

/// Hook the Hidden Knight in Defense Position is not "face-up Attack Position": the attack goes on (DEF 0, destroyed), and no
/// damage is inflicted.
#[test]
fn hook_the_hidden_knight_in_defense_position_does_nothing() {
    let hook = card("Hook the Hidden Knight");
    let Some(mut s) = field(&[], &[mon_def(0, hook), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", hook), 1, "destroyed by the attack: {}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The Skiels and Wisels of the Tag Force games
// ---------------------------------------------------------------------------------------------------------------

/// Skiel Carrier 5 (TF6): "Cannot be Normal Summoned/Set. Must first be Special Summoned (from your hand) by Tributing 1
/// "Skiel Carrier 3"." Carrier 3 stands beside Meklord Emperor Granel; Carrier 5 comes from the hand and Carrier 3 goes to
/// the Graveyard.
#[test]
fn skiel_carrier_5_is_special_summoned_by_tributing_skiel_carrier_3() {
    let [carrier3, carrier5] = ["Skiel Carrier 3 (TF6)", "Skiel Carrier 5 (TF6)"].map(card);
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, carrier3), hand(0, carrier5)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(carrier5)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", carrier5), s.count(0, "grave", carrier3)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "hand", carrier5), 0, "{}", s.context());
}

/// Skiel Carrier 5 (TF6): "Once per turn, when a monster you control is targeted for an attack: You can negate the attack."
/// Team 1's Dark Blade (1800) attacks my Warwolf (2000) beside Meklord Emperor Granel and Carrier 5: the attack is negated,
/// no one is destroyed. (Granel is mine: a Meklord Emperor's controller's other monsters cannot attack, so my Warwolf only
/// has to be the target.)
#[test]
fn skiel_carrier_5_negates_an_attack_on_a_monster_of_mine() {
    let carrier5 = card("Skiel Carrier 5 (TF6)");
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, carrier5), mon(0, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, WARWOLF)]);
    s.run_both([&[Chain(carrier5)], &[Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", WARWOLF), s.count(1, "mzone", DARK_BLADE)), (1, 1), "the attack was negated: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
}

/// Skiel Carrier 5 (TF6): "When a monster you control destroys an opponent's monster by battle and sends it to the Graveyard:
/// Inflict 300 damage to your opponent." It is team 1's, beside its Warwolf (2000), and Meklord Emperor Granel is team 0's
/// (a card spawned for team 0 is placed first, so the Meklord Emperor has to be team 0's for the Skiel to survive; a Meklord
/// Emperor's controller's other monsters cannot attack, which team 0 does not need to do): Warwolf destroys my Dark Blade
/// (1800) in turn 2: 200 battle damage and 300 more.
#[test]
fn skiel_carrier_5_inflicts_300_when_my_monster_destroys_one_in_battle() {
    let carrier5 = card("Skiel Carrier 5 (TF6)");
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, DARK_BLADE), mon(1, carrier5), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 200 - 300, "battle damage and Skiel Carrier 5's 300: {}", s.context());
    assert_eq!(s.count(1, "mzone", carrier5), 1, "{}", s.context());
}

/// Skiel Carrier 5 (TF6): "If there is no "Meklord Emperor" monster on the field, destroy this card." With none anywhere,
/// it is destroyed at once.
#[test]
fn skiel_carrier_5_is_destroyed_without_a_meklord_emperor() {
    let carrier5 = card("Skiel Carrier 5 (TF6)");
    let Some(mut s) = field(&[], &[mon(0, carrier5)]) else { return };
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", carrier5), s.count(0, "grave", carrier5)), (0, 1), "{}", s.context());
}

/// Wisel Top 3 (TF6): "You can Special Summon this card (from your hand) by Tributing 1 "Wisel Top"." Wisel Top is Tributed.
#[test]
fn wisel_top_3_is_special_summoned_by_tributing_wisel_top() {
    let [top, top3] = ["Wisel Top (TF6)", "Wisel Top 3 (TF6)"].map(card);
    let Some(mut s) = field(&[], &[mon(0, GRANEL), mon(0, top), hand(0, top3)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(top3)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", top3), s.count(0, "grave", top)), (1, 1), "{}", s.context());
}

/// Wisel Top 3 (TF6): "Once per turn, when your opponent activates a Spell/Trap Card (Quick Effect): You can negate the
/// activation, and if you do, destroy it." Team 1 has two Dark Holes: the first is negated and destroyed (Granel and Wisel
/// Top 3 stay), the second, in the same turn, is not (once per turn) and destroys every monster on the field.
#[test]
fn wisel_top_3_negates_one_spell_card_a_turn() {
    let top3 = card("Wisel Top 3 (TF6)");
    let spawns = [mon(0, GRANEL), mon(0, top3), hand(1, DARK_HOLE), hand(1, DARK_HOLE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(top3)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "the first Dark Hole was negated and destroyed: {}", s.context());
    assert_eq!((s.count(0, "mzone", GRANEL), s.count(0, "mzone", top3)), (1, 1), "nothing was destroyed: {}", s.context());
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 2, "{}", s.context());
    assert_eq!((s.count(0, "grave", GRANEL), s.count(0, "grave", top3)), (1, 1), "the second one was not: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Dark Tuners and the Dark Synchro Summon
// ---------------------------------------------------------------------------------------------------------------

/// Dark Tuner Spider Cocoon: "If only your opponent controls a monster, you can Special Summon this card (from your hand)."
/// Then a Dark Synchro Summon: the Cocoon (Level 5) and a Level 1 Insect (Kattapillar) make Dark Diviner (Level 4 = 5 - 1)
/// and both go to the Graveyard.
#[test]
fn spider_cocoon_is_special_summoned_and_makes_a_dark_synchro_summon() {
    let [cocoon, diviner] = ["Dark Tuner Spider Cocoon", "Dark Diviner"].map(card);
    let spawns = [hand(0, cocoon), hand(0, KATTAPILLAR), mon(1, WARWOLF)];
    let Some(mut s) = field_extra([&[diviner], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, cocoon), (0, KATTAPILLAR)]);
    s.run(&[SpSummon(cocoon), Summon(KATTAPILLAR), SpSummon(diviner)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", diviner), 1, "Dark Diviner was Dark Synchro Summoned: {}", s.context());
    assert_eq!((s.count(0, "grave", cocoon), s.count(0, "grave", KATTAPILLAR)), (1, 1), "{}", s.context());
    assert_eq!(s.level_of(0, diviner), Some(4), "{}", s.context());
}

/// Dark Tuner Spider Cocoon: "If only your opponent controls a monster": with a monster of mine on the field it is not
/// offered, and with none on either side it is not either.
#[test]
fn spider_cocoon_needs_only_the_opponent_to_control_a_monster() {
    let cocoon = card("Dark Tuner Spider Cocoon");
    let Some(mut s) = field(&[], &[hand(0, cocoon), mon(0, KATTAPILLAR), mon(1, WARWOLF)]) else { return };
    never_possible(&s.run(&[SpSummon(cocoon)]).expect_err("I control a monster"));
    clean(&s);
    let Some(mut s) = field(&[], &[hand(0, cocoon)]) else { return };
    never_possible(&s.run(&[SpSummon(cocoon)]).expect_err("nobody controls a monster"));
    clean(&s);
}

/// Dark Tuner Spider Cocoon: "Cannot be used as a Synchro Material, except for the Synchro Summon of a Dark Synchro
/// Monster." Gaia Knight (Level 6, any Tuner) is offered for Nitro Synchron (Level 2) and Warwolf (Level 4) and not for a
/// Cocoon and a Level 1 monster (5 + 1 is also 6).
#[test]
fn spider_cocoon_is_no_material_for_an_ordinary_synchro_monster() {
    let [cocoon, gaia] = ["Dark Tuner Spider Cocoon", "Gaia Knight, the Force of Earth"].map(card);
    let Some(mut s) = field_extra([&[gaia], &[]], &[mon(0, NITRO), mon(0, WARWOLF)]) else { return };
    s.run(&[SpSummon(gaia)]).unwrap();
    assert_eq!(s.count(0, "mzone", gaia), 1, "the control: {}", s.context());
    let Some(mut s) = field_extra([&[gaia], &[]], &[mon(0, cocoon), mon(0, KATTAPILLAR)]) else { return };
    never_possible(&s.run(&[SpSummon(gaia)]).expect_err("the Cocoon is no material for it"));
    clean(&s);
}

/// Dark Tuner Chaos Rogue: "If this card is used for a Synchro Summon and sent to the Graveyard: Send the top 5 cards of your
/// opponent's Deck to the Graveyard." Chaos Rogue (Level 8) and Big Insect (Level 4) make Dark Diviner (Level 4 = 8 - 4):
/// team 1's Deck is 5 cards smaller, Chaos Rogue and Big Insect are in my Graveyard.
#[test]
fn chaos_rogue_in_a_dark_synchro_summon_sends_five_cards_of_the_opponents_deck() {
    let [rogue, diviner] = ["Dark Tuner Chaos Rogue", "Dark Diviner"].map(card);
    let Some(mut s) = field_extra([&[diviner], &[]], &[mon(0, rogue), mon(0, BIG_INSECT)]) else { return };
    s.prefer(&[(0, rogue), (0, BIG_INSECT)]);
    let d = deck(&s, 1);
    s.run(&[SpSummon(diviner)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", diviner), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", rogue), s.count(0, "grave", BIG_INSECT)), (1, 1), "{}", s.context());
    assert_eq!(deck(&s, 1), d - 5, "the top 5 cards of the opponent's Deck: {}", s.context());
    assert_eq!(s.state()[1]["grave"].as_array().unwrap().len(), 5, "{}", s.context());
}

/// Dark Tuner - Dark Goddess Witaka: "Cannot be Normal Summoned/Set. Must first be Special Summoned (from your hand) by
/// paying 1000 LP. Once per turn: You can target 1 Synchro Monster you control; this card's Level becomes that monster's
/// Level." Gaia Knight (Level 6) is on my field: Witaka (Level 3) comes for 1000 LP and her Level becomes 6; the effect
/// cannot be used a second time that turn.
#[test]
fn witaka_costs_1000_lp_and_takes_a_synchro_monsters_level() {
    let [witaka, gaia] = ["Dark Tuner - Dark Goddess Witaka", "Gaia Knight, the Force of Earth"].map(card);
    let Some(mut s) = field(&[], &[hand(0, witaka), mon(0, gaia)]) else { return };
    keep_hands(&mut s);
    s.run(&[SpSummon(witaka)]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.count(0, "mzone", witaka)), (8000 - 1000, 1), "{}", s.context());
    assert_eq!(s.level_of(0, witaka), Some(3), "{}", s.context());
    s.run(&[Activate(witaka)]).unwrap();
    clean(&s);
    assert_eq!(s.level_of(0, witaka), Some(6), "Witaka's Level became Gaia Knight's: {}", s.context());
    assert!(!offered(&s, "activates").contains(&witaka.into()), "once per turn: {:?}", offered(&s, "activates"));
}

/// Dark Tuner - Dark Goddess Witaka: the Level change makes a Dark Synchro Summon possible: Witaka (Level 3, then 6 as Gaia
/// Knight's) and Basic Insect (Level 2) make Dark Diviner (Level 4 = 6 - 2); Level 3 alone would not (3 - 2 is 1).
#[test]
fn witaka_with_gaia_knights_level_makes_a_dark_synchro_summon() {
    let [witaka, gaia, diviner] = ["Dark Tuner - Dark Goddess Witaka", "Gaia Knight, the Force of Earth", "Dark Diviner"].map(card);
    let spawns = [hand(0, witaka), mon(0, gaia), mon(0, BASIC_INSECT)];
    let Some(mut s) = field_extra([&[diviner], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, witaka), (0, BASIC_INSECT)]);
    s.run(&[SpSummon(witaka), Activate(witaka), SpSummon(diviner)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", diviner), 1, "{}", s.context());
    assert_eq!((s.count(0, "grave", witaka), s.count(0, "grave", BASIC_INSECT)), (1, 1), "{}", s.context());
    // Without the Level change (Level 3) the same two monsters make nothing.
    let Some(mut s) = field_extra([&[diviner], &[]], &spawns) else { return };
    never_possible(&s.run(&[SpSummon(witaka), SpSummon(diviner)]).expect_err("Level 3 - 2 is not 4"));
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------
// Passion of Baimasse, Altar of the Bound Deity
// ---------------------------------------------------------------------------------------------------------------

/// Passion of Baimasse: "When your opponent activates a monster effect, while you control an "Earthbound Immortal" monster:
/// Negate the activation and destroy it." Team 1 flips Man-Eater Bug in turn 2 (its FLIP effect targets my Earthbound
/// Immortal Aslla piscu, which would be destroyed): the effect is negated and the Bug destroyed, Aslla piscu stays. (Umi is
/// the face-up Field Spell the Earthbound Immortal needs.)
#[test]
fn passion_of_baimasse_negates_a_monster_effect_and_destroys_it() {
    let [passion, aslla] = ["Passion of Baimasse", "Earthbound Immortal Aslla piscu"].map(card);
    let spawns = [set(0, passion), mon(0, aslla), spell(0, umi()), mon_set(1, MAN_EATER_BUG)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, aslla)]);
    s.run_both([&[Chain(passion)], &[Reposition(MAN_EATER_BUG)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", MAN_EATER_BUG), 1, "the Bug was destroyed: {}", s.context());
    assert_eq!(s.count(0, "mzone", aslla), 1, "its effect was negated: {}", s.context());
    assert_eq!(s.count(0, "grave", passion), 1, "{}", s.context());
}

/// Passion of Baimasse: "...while you control an "Earthbound Immortal" monster": without one the Trap is not offered, and
/// the Bug's FLIP effect destroys the target.
#[test]
fn passion_of_baimasse_needs_an_earthbound_immortal() {
    let passion = card("Passion of Baimasse");
    let spawns = [set(0, passion), mon(0, DARK_BLADE), mon_set(1, MAN_EATER_BUG)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, DARK_BLADE)]);
    never_possible(&s.run_both([&[Chain(passion)], &[Reposition(MAN_EATER_BUG)]]).expect_err("no Earthbound Immortal"));
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "the FLIP effect destroyed it: {}", s.context());
}

/// Altar of the Bound Deity: "Once per turn, during your Standby Phase: Place 1 Earthbound Immortal Counter on this card for
/// each face-up Defense Position monster on the field. You can send this card with 4 or more Earthbound Immortal Counters to
/// the Graveyard; Special Summon 1 "Earthbound Immortal" monster from your Deck." Two Defense monsters of mine and two of
/// team 1's give 4 counters at each of my Standby Phases (turns 1 and 3); the Altar goes to the Graveyard and Aslla piscu
/// comes from the Deck.
#[test]
fn altar_of_the_bound_deity_counts_defense_monsters_and_summons_an_earthbound_immortal() {
    let [altar, aslla] = ["Altar of the Bound Deity", "Earthbound Immortal Aslla piscu"].map(card);
    let spawns = [spell(0, altar), spell(0, umi()), mon_def(0, FILLER), mon_def(0, DARK_BLADE), mon_def(1, FILLER), mon_def(1, WARWOLF)];
    let Some(mut s) = field(&[aslla], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.state()[0]["szone"][0]["counters"]["148"], 8, "4 + 4 counters: {}", s.state()[0]["szone"]);
    assert!(offered(&s, "activates").contains(&altar.into()), "{:?}", offered(&s, "activates"));
    s.run(&[Activate(altar)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", aslla), 1, "Aslla piscu came from the Deck: {}", s.context());
    assert_eq!(s.count(0, "grave", altar), 1, "the Altar was the cost: {}", s.context());
}

/// Altar of the Bound Deity: "4 or more Earthbound Immortal Counters". One Defense monster gives 1 + 1 counters at turn 3 and
/// the effect is not offered; two give 2 + 2 and it is.
#[test]
fn altar_of_the_bound_deity_needs_four_counters() {
    let [altar, aslla] = ["Altar of the Bound Deity", "Earthbound Immortal Aslla piscu"].map(card);
    for (defenders, counters, offered_at_turn_3) in [(1, 2, false), (2, 4, true)] {
        let mut spawns = vec![spell(0, altar), spell(0, umi())];
        spawns.extend([FILLER, DARK_BLADE].iter().take(defenders).map(|&c| mon_def(0, c)));
        let Some(mut s) = field(&[aslla], &spawns) else { return };
        keep_hands(&mut s);
        s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
        clean(&s);
        assert_eq!(s.state()[0]["szone"][0]["counters"]["148"], counters, "{}", s.state()[0]["szone"]);
        assert_eq!(offered(&s, "activates").contains(&altar.into()), offered_at_turn_3, "{counters} counters: {:?}", offered(&s, "activates"));
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Discord Counter, Synchro Spirits, Synchro Monument, Mach Synchron
// ---------------------------------------------------------------------------------------------------------------

/// Discord Counter: "When your opponent Synchro Summons a monster: Return it to the Extra Deck. Then, Special Summon all of the
/// monsters that were used for the Synchro Summon from your opponent's Graveyard to their side of the field. Your opponent
/// cannot Summon a monster until the End Phase of their next turn." Team 1 Synchro Summons Gaia Knight from Nitro Synchron and
/// Warwolf in turn 2: Gaia Knight is back in its Extra Deck and Nitro and Warwolf stand on team 1's field again.
#[test]
fn discord_counter_returns_the_synchro_monster_and_brings_its_materials_back_to_the_opponent() {
    let [discord, gaia] = ["Discord Counter", "Gaia Knight, the Force of Earth"].map(card);
    let Some(mut s) = field_extra([&[], &[gaia]], &[set(0, discord), mon(1, NITRO), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(discord)], &[SpSummon(gaia)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", gaia), s.count(1, "extra", gaia)), (0, 1), "Gaia Knight went back to the Extra Deck: {}", s.context());
    assert_eq!((s.count(1, "mzone", NITRO), s.count(1, "mzone", WARWOLF)), (1, 1), "the materials came back: {}", s.context());
    assert_eq!((s.count(1, "grave", NITRO), s.count(1, "grave", WARWOLF)), (0, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", discord), 1, "{}", s.context());
}

/// Discord Counter: "Your opponent cannot Summon a monster until the End Phase of their next turn." After its Synchro Summon
/// is returned, team 1's Dark Blade (in its hand) cannot be Normal Summoned that turn; the control, without Discord Counter,
/// can.
#[test]
fn discord_counter_stops_the_opponent_summoning_monsters() {
    let [discord, gaia] = ["Discord Counter", "Gaia Knight, the Force of Earth"].map(card);
    let spawns = [set(0, discord), mon(1, NITRO), mon(1, WARWOLF), hand(1, DARK_BLADE)];
    let Some(mut s) = field_extra([&[], &[gaia]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(discord)], &[SpSummon(gaia)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", discord), 1, "{}", s.context());
    assert_eq!(s.prompt()["player"], 1, "{}", s.context());
    assert!(!offered(&s, "summons").contains(&DARK_BLADE.into()), "no Normal Summon: {:?}", offered(&s, "summons"));
    let spawns = [mon(1, NITRO), mon(1, WARWOLF), hand(1, DARK_BLADE)];
    let Some(mut s) = field_extra([&[], &[gaia]], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[SpSummon(gaia)]]).unwrap();
    assert!(offered(&s, "summons").contains(&DARK_BLADE.into()), "the control: {:?}", offered(&s, "summons"));
}

/// Synchro Spirits: "Target 1 Synchro Monster in your Graveyard; banish it, then, if all of the monsters that were used for
/// the Synchro Summon of that monster are in your Graveyard, you can Special Summon all of them." Gaia Knight is Synchro
/// Summoned from Nitro Synchron and Warwolf in turn 1 and destroyed by team 1's Dark Hole in turn 2; in turn 3 the Trap
/// banishes it from the Graveyard and both materials, which are still there, come back (a Synchro Monster on the field is not
/// a target: the script's filter takes the Graveyard only).
#[test]
fn synchro_spirits_banishes_a_synchro_monster_and_brings_its_materials_back() {
    let [spirits, gaia] = ["Synchro Spirits", "Gaia Knight, the Force of Earth"].map(card);
    let spawns = [set(0, spirits), mon(0, NITRO), mon(0, WARWOLF), hand(1, DARK_HOLE)];
    let Some(mut s) = field_extra([&[gaia], &[]], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, gaia)]);
    s.run_both([&[SpSummon(gaia), EndTurn, Activate(spirits)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", gaia), s.count(0, "grave", gaia)), (1, 0), "Gaia Knight is banished: {}", s.context());
    assert_eq!((s.count(0, "mzone", NITRO), s.count(0, "mzone", WARWOLF)), (1, 1), "its materials are back: {}", s.context());
    assert_eq!(s.count(0, "grave", spirits), 1, "{}", s.context());
}

/// Synchro Monument: "Synchro Summons cannot be negated. Neither player can activate any Spell/Trap Cards when a monster is
/// Synchro Summoned." With Synchro Monument on the field team 1's Discord Counter is never offered at my Synchro Summon of
/// Gaia Knight; the control, without it, is.
#[test]
fn synchro_monument_stops_trap_cards_at_a_synchro_summon() {
    let [discord, gaia, monument] = ["Discord Counter", "Gaia Knight, the Force of Earth", "Synchro Monument"].map(card);
    let materials = [mon(0, NITRO), mon(0, WARWOLF), set(1, discord)];
    let Some(mut s) = field_extra([&[gaia], &[]], &materials) else { return };
    s.run_both([&[SpSummon(gaia)], &[Chain(discord)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", gaia), 0, "the control: Discord Counter returned it: {}", s.context());
    let spawns = [mon(0, NITRO), mon(0, WARWOLF), set(1, discord), spell(0, monument)];
    let Some(mut s) = field_extra([&[gaia], &[]], &spawns) else { return };
    never_possible(&s.run_both([&[SpSummon(gaia)], &[Chain(discord)]]).expect_err("no Trap during a Synchro Summon"));
    clean(&s);
    assert_eq!((s.count(0, "mzone", gaia), s.count(1, "szone", discord)), (1, 1), "{}", s.context());
}

/// Mach Synchron: "Once per turn, during the End Phase, if this card is in the Graveyard because it was sent there this turn
/// as Synchro Material: You can target 1 other monster in your Graveyard, that was used as material for that Synchro Summon;
/// add it to your hand." Mach Synchron (Level 1) and Giant Axe Mummy (Level 5) make Gaia Knight (Level 6); at the End Phase
/// Giant Axe Mummy returns to my hand and Mach Synchron stays in the Graveyard.
#[test]
fn mach_synchron_returns_another_material_to_the_hand_at_the_end_phase() {
    let [mach, gaia] = ["Mach Synchron", "Gaia Knight, the Force of Earth"].map(card);
    let Some(mut s) = field_extra([&[gaia], &[]], &[mon(0, mach), mon(0, MUMMY)]) else { return };
    keep_hands(&mut s); // the End Phase's hand-size discard (7 cards) takes Feral Imps
    s.prefer(&[(0, FILLER), (0, MUMMY)]);
    s.run(&[SpSummon(gaia), Chain(mach)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", gaia), 1, "{}", s.context());
    assert_eq!((s.count(0, "hand", MUMMY), s.count(0, "grave", mach)), (1, 1), "Giant Axe Mummy is in the hand: {}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Final Penalty, Empress's Crown and Staff, Time Angel
// ---------------------------------------------------------------------------------------------------------------

/// Final Penalty: "When a Fiend-Type monster you control is destroyed by an opponent's attacking monster and sent to the
/// Graveyard: Inflict damage to your opponent equal to that monster's ATK, then, destroy the attacking monster." My Feral Imp
/// (Fiend, 1300) is destroyed by team 1's Warwolf (2000): I take 700 from the battle, team 1 takes 2000 and Warwolf is
/// destroyed.
#[test]
fn final_penalty_burns_the_attackers_atk_and_destroys_it() {
    let penalty = card("Final Penalty");
    let Some(mut s) = field(&[], &[set(0, penalty), mon(0, FILLER), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(penalty)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", FILLER), 1, "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 700, 8000 - 2000), "{}", s.context());
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", WARWOLF)), (1, 0), "the attacker was destroyed: {}", s.context());
    assert_eq!(s.count(0, "grave", penalty), 1, "{}", s.context());
}

/// Final Penalty: "a Fiend-Type monster": a Warrior (Dark Blade) destroyed in battle does not offer it.
#[test]
fn final_penalty_is_for_fiend_monsters_only() {
    let penalty = card("Final Penalty");
    let Some(mut s) = field(&[], &[set(0, penalty), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    never_possible(&s.run_both([&[Chain(penalty)], &[Attack(WARWOLF)]]).expect_err("a Warrior is no Fiend"));
    assert_eq!((s.count(0, "grave", DARK_BLADE), s.lp(1)), (1, 8000), "{}", s.context());
}

/// Empress's Crown: "If you control a "Timelord" monster: Draw 2 cards for each Synchro Monster your opponent controls." With
/// Sadion, the Timelord on my side and two Synchro Monsters (Gaia Knight and Goyo Guardian) on team 1's, I use it from the Set
/// position when team 1 Normal Summons a monster in turn 2 (at my own Standby Phase Sadion goes back into the Deck): my Deck is
/// 4 cards smaller than in the control duel without it.
#[test]
fn empress_crown_draws_two_cards_for_each_synchro_monster_of_the_opponent() {
    let [crown, sadion, gaia, goyo] = ["Empress's Crown", "Sadion, the Timelord", "Gaia Knight, the Force of Earth", "Goyo Guardian"].map(card);
    let others = [mon(0, sadion), mon(1, gaia), mon(1, goyo), hand(1, DARK_BLADE)];
    let Some(mut control) = field(&[], &others) else { return };
    keep_hands(&mut control);
    control.run_both([&[], &[Summon(DARK_BLADE)]]).unwrap();
    let spawns = [set(0, crown), mon(0, sadion), mon(1, gaia), mon(1, goyo), hand(1, DARK_BLADE)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(crown)], &[Summon(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", crown), 1, "{}", s.context());
    assert_eq!(deck(&s, 0), deck(&control, 0) - 4, "2 cards for each of 2 Synchro Monsters: {}", s.context());
}

/// Empress's Crown: "When a Synchro Monster is Special Summoned to your opponent's side of the field, you can activate this
/// card from your hand." Team 1 holds it with Sadion on its side: my Synchro Summon of Gaia Knight lets it play the Crown from
/// the hand and draw 2 cards.
#[test]
fn empress_crown_is_played_from_the_hand_when_the_opponent_synchro_summons() {
    let [crown, sadion, gaia] = ["Empress's Crown", "Sadion, the Timelord", "Gaia Knight, the Force of Earth"].map(card);
    let spawns = [mon(0, NITRO), mon(0, WARWOLF), hand(1, crown), mon(1, sadion)];
    let Some(mut s) = field_extra([&[gaia], &[]], &spawns) else { return };
    keep_hands(&mut s);
    let before = deck(&s, 1);
    s.run_both([&[SpSummon(gaia)], &[Chain(crown)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "hand", crown), s.count(1, "grave", crown)), (0, 1), "{}", s.context());
    assert_eq!(deck(&s, 1), before - 2, "two cards drawn: {}", s.context());
}

/// Empress's Crown: "If you control a "Timelord" monster": without Sadion the Set card is not offered.
#[test]
fn empress_crown_needs_a_timelord() {
    let [crown, gaia] = ["Empress's Crown", "Gaia Knight, the Force of Earth"].map(card);
    let Some(mut s) = field(&[], &[set(0, crown), mon(1, gaia)]) else { return };
    keep_hands(&mut s);
    never_possible(&s.run_both([&[EndTurn, Activate(crown)], &[EndTurn]]).expect_err("no Timelord"));
}

/// Empress's Staff: "When an opponent's monster declares an attack: Negate the attack, then, inflict 500 damage to your
/// opponent." Set, with no monsters on my side: team 1's Warwolf attacks directly in turn 2 and nothing is taken; team 1
/// loses 500.
#[test]
fn empress_staff_negates_an_attack_and_burns_500() {
    let staff = card("Empress's Staff");
    let Some(mut s) = field(&[], &[set(0, staff), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(staff)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 500), "{}", s.context());
    assert_eq!(s.count(0, "grave", staff), 1, "{}", s.context());
}

/// Empress's Staff: "If you control no monsters, you can activate this card from your hand." From the hand, the same; with
/// a monster of mine on the field the card in the hand is not offered.
#[test]
fn empress_staff_is_played_from_the_hand_when_i_control_no_monsters() {
    let staff = card("Empress's Staff");
    let Some(mut s) = field(&[], &[hand(0, staff), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(staff)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 500), "{}", s.context());
    assert_eq!(s.count(0, "grave", staff), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[hand(0, staff), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    s.prefer(&[(0, FILLER), (0, DARK_BLADE)]);
    never_possible(&s.run_both([&[Chain(staff)], &[Attack(WARWOLF)]]).expect_err("I control a monster"));
    assert_eq!(s.count(0, "hand", staff), 1, "{}", s.context());
}

/// Time Angel: "When this face-up Attack Position card is destroyed by battle and sent to the Graveyard: Return all monsters
/// on the field to the hand." Team 1's Warwolf destroys it (I take 2000): my Dark Blade and team 1's Warwolf and Thunder Kid
/// are in their owners' hands.
#[test]
fn time_angel_destroyed_in_attack_position_returns_every_monster_to_the_hand() {
    let angel = card("Time Angel");
    let spawns = [mon(0, angel), mon(0, DARK_BLADE), mon(1, WARWOLF), mon(1, THUNDER_KID)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, angel)]);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", angel), 1, "{}", s.context());
    assert_eq!(s.lp(0), 8000 - 2000, "{}", s.context());
    assert_eq!((s.count(0, "hand", DARK_BLADE), s.count(1, "hand", WARWOLF), s.count(1, "hand", THUNDER_KID)), (1, 1, 1), "{}", s.context());
    assert!(s.state()[0]["mzone"].as_array().into_iter().chain(s.state()[1]["mzone"].as_array()).flatten().all(|m| m.is_null()), "{}", s.context());
}

/// Time Angel in Defense Position is not "face-up Attack Position": it is destroyed (DEF 0) and nothing returns.
#[test]
fn time_angel_destroyed_in_defense_position_returns_nothing() {
    let angel = card("Time Angel");
    let Some(mut s) = field(&[], &[mon_def(0, angel), mon(0, DARK_BLADE), mon(1, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, angel)]);
    s.run_both([&[], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", angel), 1, "{}", s.context());
    assert_eq!((s.count(0, "mzone", DARK_BLADE), s.count(1, "mzone", WARWOLF)), (1, 1), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// Gate Defender, Defender's Mind, Terminal Countdown
// ---------------------------------------------------------------------------------------------------------------

/// Gate Defender: "Once per turn, when an opponent's monster declares an attack: You can negate that attack." In Attack
/// Position (0 ATK) it is attacked by Warwolf, then Dark Blade in the same turn: the first attack is negated (no damage), the
/// second is not (once per turn): Gate Defender is destroyed and I take 1800.
#[test]
fn gate_defender_negates_one_attack_a_turn() {
    let gate = card("Gate Defender");
    let Some(mut s) = field(&[], &[mon(0, gate), mon(1, WARWOLF), mon(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(gate)], &[Attack(WARWOLF), Attack(DARK_BLADE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", gate), 1, "destroyed by the second attack: {}", s.context());
    assert_eq!(s.lp(0), 8000 - 1800, "the first attack did no damage: {}", s.context());
}

/// Gate Defender: "While you control another monster, your opponent cannot target this card for attacks. If a monster you
/// control is destroyed by battle: Destroy this face-up card." Gate Defender (Defense, DEF 2000) stands beside Dark Blade
/// (1800). Team 1's Warwolf attacks first and Gate Defender negates it (the engine's default answer to the optional
/// effect is yes); then Insect Knight (1900, less than Gate Defender's DEF) attacks, and its attack can only go to Dark
/// Blade, which is destroyed (had Gate Defender been targetable it would have survived the 1900), and Gate Defender follows it.
#[test]
fn gate_defender_cannot_be_attacked_beside_another_monster_and_follows_it_to_the_graveyard() {
    let gate = card("Gate Defender");
    let spawns = [mon_def(0, gate), mon(0, DARK_BLADE), mon(1, WARWOLF), mon(1, INSECT_KNIGHT)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.prefer(&[(0, gate)]);
    s.run_both([&[], &[Attack(WARWOLF), Attack(INSECT_KNIGHT)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "grave", DARK_BLADE), 1, "Dark Blade took the second attack: {}", s.context());
    assert_eq!(s.count(0, "grave", gate), 1, "Gate Defender was destroyed after it: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000 - 100, 8000), "only Dark Blade's 100 points of battle damage: {}", s.context());
}

/// Defender's Mind: "Activate only during the Main Phase. Change all face-up monsters you control to Defense Position. Double
/// the DEF of all Defense Position monsters you control. You cannot change these monsters to Attack Position." Dark Blade
/// (1800/1500) and Warwolf (2000/100) go to Defense Position with DEF 3000 and 200, and cannot be repositioned.
#[test]
fn defenders_mind_puts_my_monsters_in_defense_with_double_def() {
    let mind = card("Defender's Mind");
    let Some(mut s) = field(&[], &[set(0, mind), mon(0, DARK_BLADE), mon(0, WARWOLF)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(mind)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.position_of(0, DARK_BLADE), s.position_of(0, WARWOLF)), (Some(4), Some(4)), "{}", s.context());
    assert_eq!((stats(&s, 0, DARK_BLADE).1, stats(&s, 0, WARWOLF).1), (Some(3000), Some(200)), "{}", s.context());
    assert!(!offered(&s, "pos_changes").contains(&DARK_BLADE.into()), "{:?}", offered(&s, "pos_changes"));
}

/// Terminal Countdown: "While this card is face-up on the field, you cannot activate Spell/Trap Cards. Once per turn, during
/// your opponent's End Phase: You can choose 1 Spell/Trap in your Deck, reveal it to your opponent and set it in your Spell &
/// Trap Zone. During your Main Phase: You can send this card and all Spells/Traps Set by this effect from your side of the
/// field to the Graveyard; inflict an amount of damage to your opponent, depending on the number of cards sent. 2: 500"
/// Dian Keto (the only Spell in my Deck) is Set at team 1's End Phase of turn 2; in turn 3 both go to the Graveyard and team 1
/// takes 500.
#[test]
fn terminal_countdown_sets_a_card_and_burns_500_for_two_cards() {
    let countdown = card("Terminal Countdown");
    let Some(mut s) = field(&[DIAN_KETO], &[spell(0, countdown)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(countdown), Activate(countdown)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", countdown), s.count(0, "grave", DIAN_KETO)), (1, 1), "both were sent: {}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000 - 500), "{}", s.context());
}

// ---------------------------------------------------------------------------------------------------------------
// The seeded sample: Chain Close, Double Type Rescue, Imitation (and the Mask Token), Zero Hole (GX)
// ---------------------------------------------------------------------------------------------------------------

/// Chain Close: "When a card(s) on the field is destroyed by battle or card effect and sent to the Graveyard: Your opponent
/// cannot activate Spell/Trap Cards this turn." Team 1's Dark Hole destroys my Feral Imp and I chain it: team 1's Dian Keto is
/// not offered afterwards (it is the same turn); the control, without Chain Close, offers it.
#[test]
fn chain_close_stops_the_opponents_spell_and_trap_cards_for_the_turn() {
    let close = card("Chain Close");
    let spawns = [set(0, close), mon(0, FILLER), hand(1, DARK_HOLE), hand(1, DIAN_KETO)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(close)], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", FILLER), s.count(0, "grave", close)), (1, 1), "{}", s.context());
    assert_eq!(s.prompt()["player"], 1, "{}", s.context());
    assert!(!offered(&s, "activates").contains(&DIAN_KETO.into()), "Dian Keto is locked: {:?}", offered(&s, "activates"));
    let Some(mut s) = field(&[], &[mon(0, FILLER), hand(1, DARK_HOLE), hand(1, DIAN_KETO)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(offered(&s, "activates").contains(&DIAN_KETO.into()), "the control: {:?}", offered(&s, "activates"));
}

/// Double Type Rescue: "If you control 2 or more Types of monsters that are face-up, and your opponent controls more monsters
/// than you do: Target 1 monster in your Graveyard; Special Summon it." My Dark Blade (Warrior) and Thunder Kid (Thunder)
/// face team 1's three monsters: Dark Magician comes back from the Graveyard in turn 3. With only two opposing monsters
/// (not more than mine) it is not offered.
#[test]
fn double_type_rescue_summons_a_monster_from_the_graveyard() {
    let rescue = card("Double Type Rescue");
    let spawns = [set(0, rescue), mon(0, DARK_BLADE), mon(0, THUNDER_KID), grave(0, DARK_MAGICIAN), mon(1, WARWOLF), mon(1, FILLER), mon(1, KATTAPILLAR)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(rescue)], &[EndTurn]]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", DARK_MAGICIAN), s.count(0, "grave", DARK_MAGICIAN)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", rescue), 1, "{}", s.context());
    let spawns = [set(0, rescue), mon(0, DARK_BLADE), mon(0, THUNDER_KID), grave(0, DARK_MAGICIAN), mon(1, WARWOLF), mon(1, FILLER)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    never_possible(&s.run_both([&[EndTurn, Activate(rescue)], &[EndTurn]]).expect_err("2 is not more than 2"));
}

/// Imitation, with the Mask Token: "Target 1 Token on the field; Special Summon 1 "Mask Token" with the same original Level,
/// Type, Attribute, ATK and DEF as that Token to your opponent's side of the field, then, inflict damage to your opponent
/// equal to half the ATK of the targeted Token." Team 1's Warwolf destroys my Multiple Slime in turn 2 and three Slime Tokens
/// (Level 1 Aqua WATER 500/500) come; in turn 3 Imitation targets one: a Mask Token of the same stats stands on team 1's
/// side (the client's view agrees) and team 1 takes 250. Then my Dark Hole destroys every monster: the Mask Token leaves the
/// field and is gone (not in the Graveyard).
#[test]
fn imitation_makes_a_mask_token_like_the_target_and_burns_half_its_atk() {
    let [imitation, multiple, slime] = ["Imitation", "Multiple Slime", "Slime Token"].map(card);
    let mask = MASK_TOKEN;
    let spawns = [mon(0, multiple), hand(0, imitation), hand(0, DARK_HOLE), mon(1, WARWOLF)];
    let Some(mut s) = field(&[], &spawns) else { return };
    keep_hands(&mut s);
    s.run_both([&[EndTurn, Activate(imitation)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", slime), 3, "the three Slime Tokens: {}", s.context());
    assert_eq!(s.count(1, "mzone", mask), 1, "the Mask Token is on the opponent's side: {}", s.context());
    assert_eq!(stats(&s, 1, mask), stats(&s, 0, slime), "same ATK, DEF, Level, Type and Attribute: {}", s.context());
    assert_eq!(stats(&s, 1, mask).0, Some(500), "{}", s.context());
    assert_eq!(s.lp(1), 8000 - 250, "half of 500: {}", s.context());
    assert_eq!(s.count(0, "grave", imitation), 1, "{}", s.context());
    s.run(&[Activate(DARK_HOLE)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "mzone", mask), s.count(1, "grave", mask), s.count(0, "mzone", slime)), (0, 0, 0), "the Token left the field and is gone: {}", s.context());
}

/// Zero Hole (GX): "Negate the effects of 1 Effect Monster Summoned by your opponent, and if you do, destroy that monster."
/// Team 1 Normal Summons Time Angel (an Effect Monster) in turn 2 and I chain it: Time Angel is destroyed. Dark Blade (a
/// Normal Monster, summoned the same way) is not an Effect Monster: nothing is offered for it.
#[test]
fn zero_hole_destroys_an_effect_monster_the_opponent_summoned() {
    let [hole, angel] = ["Zero Hole (GX)", "Time Angel"].map(card);
    let Some(mut s) = field(&[], &[set(0, hole), hand(1, angel)]) else { return };
    keep_hands(&mut s);
    s.run_both([&[Chain(hole)], &[Summon(angel)]]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", angel), s.count(1, "mzone", angel)), (1, 0), "{}", s.context());
    assert_eq!(s.count(0, "grave", hole), 1, "{}", s.context());
    let Some(mut s) = field(&[], &[set(0, hole), hand(1, DARK_BLADE)]) else { return };
    keep_hands(&mut s);
    never_possible(&s.run_both([&[Chain(hole)], &[Summon(DARK_BLADE)]]).expect_err("a Normal Monster"));
    assert_eq!(s.count(1, "mzone", DARK_BLADE), 1, "{}", s.context());
}



