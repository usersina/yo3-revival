//! The Dark Side of Dimensions (2016 movie), rows 6372-6391 of engine/added/cards.tsv: Aigami's Cubic cards, Kaiba's Chaos
//! MAX and Yugi's Dragon Knight. Each effect through the real core with both seats' client models checked after every step
//! (`duelcheck::scenario`); printed text with anime cards off, the anime version with them on where it differs.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, card, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const FILLER: u16 = 4012; // Feral Imp, a vanilla Level 4 (1300/1400)

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
}

fn sp(team: u32, place: yo3_server::engine::SpawnPlace, cid: u16) -> Spawn {
    Spawn { team, place, cid }
}

fn hand(s: &Scenario, team: usize) -> usize {
    s.state()[team]["hand"].as_array().map_or(0, Vec::len)
}

/// Both Decks of Feral Imps, team 0's with `with` among them (one copy each: a test that uses one from the Deck checks the effect found it).
fn duel(anime: bool, lp: u32, extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    duel_with(anime, lp, &[], extra, spawns)
}

fn duel_with(anime: bool, lp: u32, with: &[u16], extra: &[u16], spawns: &[Spawn]) -> Option<Scenario> {
    duel_foe(anime, lp, with, extra, FILLER, spawns)
}

/// [`duel_with`] with team 1's whole Deck of `foe`.
fn duel_foe(anime: bool, lp: u32, with: &[u16], extra: &[u16], foe: u16, spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let deck = [FILLER; 40];
    let mine: Vec<u16> = with.iter().copied().chain(deck).take(40).collect();
    let theirs = [foe; 40];
    let decks = [(&mine[..], extra), (&theirs[..], &[][..])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }.unwrap())
}

/// A hand-summoned Cubic: `cost` Vijam on the field go to the Graveyard (printed), the monster comes with its bonus ATK and,
/// when it says so, burns the opponent for 800.
fn printed_summon(name: &str, vijams: usize, atk: i64, burn: bool) {
    let (vijam, big) = (card("Vijam the Cubic Seed"), card(name));
    let mut spawns = vec![sp(0, Hand, big)];
    spawns.extend((0..vijams).map(|_| sp(0, Monster, vijam)));
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[SpSummon(big)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, big), Some(atk), "{name}: {}", s.context());
    assert_eq!(s.count(0, "grave", vijam), vijams, "{name}: the cost went to the Graveyard: {}", s.context());
    assert_eq!(s.lp(1), if burn { 7200 } else { 8000 }, "{name}: {}", s.context());
}

/// Geira Guile: "Must first be Special Summoned (from your hand) by sending 1 "Cubic" monster you control to the Graveyard.
/// If Summoned this way, this card gains 800 ATK. If this card is Special Summoned from the hand: Inflict 800 damage."
#[test]
fn geira_guile_gains_800_and_burns() {
    printed_summon("Geira Guile the Cubic King", 1, 800, true);
}

/// Dark Garnex: sends 1 Cubic, gains 1000 ATK.
#[test]
fn dark_garnex_gains_1000() {
    printed_summon("Dark Garnex the Cubic Beast", 1, 1000, false);
}

/// Blade Garoodia: sends 2 Cubic, gains 2000 ATK.
#[test]
fn blade_garoodia_gains_2000() {
    printed_summon("Blade Garoodia the Cubic Beast", 2, 2000, false);
}

/// Vulcan Dragni: sends 2 Cubic, gains 1600 ATK, 800 damage.
#[test]
fn vulcan_dragni_gains_1600_and_burns() {
    printed_summon("Vulcan Dragni the Cubic King", 2, 1600, true);
}

/// Indiora Doom Volt: sends 3 Cubic, gains 2400 ATK, 800 damage.
#[test]
fn indiora_gains_2400_and_burns() {
    printed_summon("Indiora Doom Volt the Cubic Emperor", 3, 2400, true);
}

/// Buster Gundil: sends 3 Cubic, gains 3000 ATK (no damage).
#[test]
fn buster_gundil_gains_3000() {
    printed_summon("Buster Gundil the Cubic Behemoth", 3, 3000, false);
}

/// Crimson Nova: "Must be Special Summoned (from your hand) by revealing 3 other "Cubic" cards in your hand with different
/// names... During your End Phase: Each player takes 3000 damage."
#[test]
fn crimson_nova_comes_by_revealing_three_cubics_and_burns_both_in_the_end_phase() {
    let nova = card("Crimson Nova the Dark Cubic Lord");
    let others = ["Vijam the Cubic Seed", "Cubic Karma", "Geira Guile the Cubic King"].map(card);
    let mut spawns = vec![sp(0, Hand, nova)];
    spawns.extend(others.iter().map(|&c| sp(0, Hand, c)));
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[SpSummon(nova)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, nova), Some(3000), "{}", s.context());
    assert!(others.iter().all(|&c| s.count(0, "hand", c) == 1), "revealing keeps them in the hand: {}", s.context());
    s.run(&[EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.lp(0), s.lp(1)), (5000, 5000), "{}", s.context());
}


/// The anime version: the Cubic's own effect (Quick, or an ignition effect) overlays `vijams` Vijam from the field and
/// the monster's ATK is its bonus for each; the Graveyard stays empty.
fn anime_summon(name: &str, vijams: usize, atk: i64, burn: bool) {
    let (vijam, big) = (card("Vijam the Cubic Seed"), card(name));
    let mut spawns = vec![sp(0, Hand, big)];
    spawns.extend((0..vijams).map(|_| sp(0, Monster, vijam)));
    let Some(mut s) = duel(true, 8000, &[], &spawns) else { return };
    s.run(&[Activate(big)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, big), Some(atk), "{name}: {}", s.context());
    assert_eq!(s.count(0, "grave", vijam), 0, "{name}: the Vijam are its materials: {}", s.context());
    assert_eq!(s.count(0, "mzone", vijam), 0, "{name}: {}", s.context());
    assert_eq!(s.lp(1), if burn { 7200 } else { 8000 }, "{name}: {}", s.context());
}

/// Anime Geira Guile: ignition effect from the hand, 1 Vijam overlaid, 800 ATK each, 800 damage on arrival.
#[test]
fn anime_geira_guile_overlays_a_vijam() {
    anime_summon("Geira Guile the Cubic King", 1, 800, true);
}

#[test]
fn anime_dark_garnex_overlays_a_vijam() {
    anime_summon("Dark Garnex the Cubic Beast", 1, 1000, false);
}

#[test]
fn anime_blade_garoodia_overlays_two_vijam() {
    anime_summon("Blade Garoodia the Cubic Beast", 2, 2000, false);
}

#[test]
fn anime_buster_gundil_overlays_three_vijam() {
    anime_summon("Buster Gundil the Cubic Behemoth", 3, 3000, false);
}

#[test]
fn anime_vulcan_dragni_overlays_two_vijam_and_burns() {
    anime_summon("Vulcan Dragni the Cubic King", 2, 1600, true);
}

#[test]
fn anime_indiora_overlays_three_vijam_and_burns() {
    anime_summon("Indiora Doom Volt the Cubic Emperor", 3, 2400, true);
}

/// Anime Cubic Karma, "When this card is activated": it sends nothing, it overlays the Vijam in the hand onto a Cubic
/// monster (the client's hand loses the card: the translator's attach from the hand).
#[test]
fn karma_anime_overlays_vijam_from_the_hand() {
    let (vijam, geira, karma) = (card("Vijam the Cubic Seed"), card("Geira Guile the Cubic King"), card("Cubic Karma"));
    let spawns = [sp(0, Monster, geira), sp(0, Hand, vijam), sp(0, Hand, karma)];
    let Some(mut s) = duel(true, 8000, &[], &spawns) else { return };
    let before = hand(&s, 0);
    s.run(&[Activate(karma)]).unwrap();
    clean(&s);
    assert_eq!(hand(&s, 0), before - 2, "{}", s.context());
    assert_eq!(s.count(0, "hand", vijam), 0, "{}", s.context());
}

/// Cubic Karma's trigger. Team 1's Geira Guile (0 ATK) attacks team 0's: "if this card battled" Geira brings Vijam back
/// (printed: from the Graveyard, here a tie of 0 ATK each; anime: the Vijam it holds, here its 800 ATK wins) and, "during
/// your opponent's turn", Karma goes to the Graveyard and halves the attacker's LP (printed: rounded up).
fn karma_halves(anime: bool, lp: u32) -> Scenario {
    let (vijam, geira, karma, vulcan) = (card("Vijam the Cubic Seed"), card("Geira Guile the Cubic King"), card("Cubic Karma"), card("Vulcan Dragni the Cubic King"));
    let theirs = sp(1, Monster, geira);
    let s = if anime {
        // The anime Geira's trigger needs its Vulcan Dragni in the Deck, and asks (an optional trigger).
        let spawns = [sp(0, Spell, karma), sp(0, Monster, vijam), sp(0, Hand, geira), theirs];
        let mut s = duel_with(true, lp, &[vulcan], &[], &spawns).expect("the engine");
        s.run_both([&[Activate(geira), EndTurn, Chain(geira)], &[Attack(geira)]]).unwrap();
        s
    } else {
        let spawns = [sp(0, Spell, karma), sp(0, Monster, geira), sp(0, Grave, vijam), sp(0, Grave, vijam), theirs];
        let mut s = duel(false, lp, &[], &spawns).expect("the engine");
        s.run_both([&[EndTurn], &[Attack(geira)]]).unwrap();
        s
    };
    clean(&s);
    assert_eq!(s.count(0, "grave", karma), 1, "Karma was sent to the Graveyard: {}", s.context());
    assert!(s.count(0, "mzone", vijam) >= 1, "Geira brought a Vijam: {}", s.context());
    s
}

#[test]
fn karma_halves_the_attackers_lp_when_a_vijam_comes_in_the_opponents_turn() {
    if engine_dir().is_none() { return }
    let s = karma_halves(false, 8000);
    assert_eq!((s.lp(0), s.lp(1)), (8000, 4000), "{}", s.context());
}

#[test]
fn anime_karma_halves_the_attackers_lp_too() {
    if engine_dir().is_none() { return }
    let s = karma_halves(true, 8000);
    // 800 for Geira's arrival and 800 of battle damage, then half.
    assert_eq!((s.lp(0), s.lp(1)), (8000, 3200), "{}", s.context());
}

/// Half of infinity is infinity (guards.lua: any LP of 9,999,999 or more is the one infinite value), whichever script sets the LP.
#[test]
fn karma_at_infinite_lp_leaves_it_infinite() {
    if engine_dir().is_none() { return }
    for anime in [false, true] {
        let s = karma_halves(anime, 20_000_000);
        assert_eq!(s.lp(1), 19_999_998, "anime {anime}: {}", s.context());
    }
}

const THUNDER_KID: u16 = 4217; // Level 2, 700/600
const WARWOLF: u16 = 6960; // Level 4, 2000/100, no effect
const BLUE_EYES: u16 = 4007;
const DARK_MAGICIAN: u16 = 4041;
const DARK_HOLE: u16 = 4342;
const RAIGEKI: u16 = 4343;
const POLYMERIZATION: u16 = 4837;
const MST: u16 = 4909;
const HEAVY_STORM: u16 = 4891;
const BOOK_OF_MOON: u16 = 5432;

/// Cubic Wave: "Target 1 "Cubic" monster you control and 1 face-up monster your opponent controls; the ATK of that monster
/// you control becomes double its current ATK, and if it does, the ATK of that monster your opponent controls become half."
/// Crimson Nova (3000) doubles, Warwolf (2000) halves. The anime Wave (a Continuous effect that stays) gives the same.
fn wave(anime: bool) {
    let (nova, wave) = (card("Crimson Nova the Dark Cubic Lord"), card("Cubic Wave"));
    let spawns = [sp(0, Monster, nova), sp(0, Hand, wave), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(anime, 8000, &[], &spawns) else { return };
    s.run(&[Activate(wave)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, nova), Some(6000), "anime {anime}: {}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(1000), "anime {anime}: {}", s.context());
}

#[test]
fn cubic_wave_doubles_a_cubic_and_halves_a_monster() {
    wave(false);
}

#[test]
fn anime_cubic_wave_does_the_same_while_it_stays() {
    wave(true);
}

/// Cubic Wave's banish effect: it and the Cubic monsters in the Graveyard are banished, a Cubic Counter goes on the same
/// number of the opponent's face-up monsters. Two Vijam and Wave in the Graveyard: Wave and both leave for banishment.
#[test]
fn cubic_wave_banishes_itself_and_cubics_for_counters() {
    let (vijam, wave) = (card("Vijam the Cubic Seed"), card("Cubic Wave"));
    let spawns = [sp(0, Grave, wave), sp(0, Grave, vijam), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.prefer_most(true);
    s.run(&[Activate(wave)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "removed", wave), s.count(0, "removed", vijam)), (1, 1), "{}", s.context());
}

/// Cubic Mandala (printed): "If you control a "Cubic" monster: ... target monsters in your opponent's Graveyard because they
/// were destroyed and sent there this turn; Special Summon them to your opponent's field, each ATK 0 with a Cubic Counter."
/// Raigeki destroys Warwolf; Mandala (Set earlier) brings it back to the opponent's side with 0 ATK.
#[test]
fn cubic_mandala_returns_the_monsters_destroyed_this_turn_with_no_attack() {
    let (vijam, mandala) = (card("Vijam the Cubic Seed"), card("Cubic Mandala"));
    let spawns = [sp(0, Monster, vijam), sp(0, Hand, RAIGEKI), sp(0, SpellSet, mandala), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[Activate(RAIGEKI), Activate(mandala)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 0, "{}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(0), "Warwolf is back on the opponent's field with no attack: {}", s.context());
    assert_eq!(s.count(0, "szone", mandala), 1, "{}", s.context());
}

/// Anime Cubic Mandala brings back only the monsters that had a Cubic Counter when they were destroyed: Warwolf destroyed
/// by Raigeki had none, so it is not offered.
#[test]
fn anime_cubic_mandala_wants_a_monster_that_died_with_a_cubic_counter() {
    let (vijam, mandala) = (card("Vijam the Cubic Seed"), card("Cubic Mandala"));
    let spawns = [sp(0, Monster, vijam), sp(0, Hand, RAIGEKI), sp(0, SpellSet, mandala), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(true, 8000, &[], &spawns) else { return };
    s.run(&[Activate(RAIGEKI)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "{}", s.context());
    assert!(!offered(&s, "activates", mandala), "no countered monster died: {}", s.context());
    assert_eq!(s.count(0, "szone", mandala), 1, "{}", s.context());
}

/// The cids a prompt offers in one of its lists (`summons`, `activates`...).
fn offered(s: &Scenario, list: &str, cid: u16) -> bool {
    s.prompt()[list].as_array().is_some_and(|v| v.iter().any(|c| c["cid"] == cid))
}

/// Cubic Rebirth: "When an opponent's monster declares a direct attack: Target the attacking monster; your opponent Special
/// Summons, in Attack Position, as many monsters as possible with the same name... the ATK of the targeted monster and
/// those become 0 and place 1 Cubic Counter on each. Also, after that, Special Summon 1 Level 4 or lower "Cubic" monster
/// from your hand, ignoring its Summoning conditions." The opponent's Deck is Warwolves: the attacker deals no damage.
fn rebirth(anime: bool) {
    let (rebirth, geira) = (card("Cubic Rebirth"), card("Geira Guile the Cubic King"));
    let spawns = [sp(0, SpellSet, rebirth), sp(0, Hand, geira), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel_foe(anime, 8000, &[], &[], WARWOLF, &spawns) else { return };
    s.prefer(&[(0, FILLER)]); // the end-of-turn discard takes Feral Imps
    s.run_both([&[EndTurn, Chain(rebirth)], &[Attack(WARWOLF)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", geira), 1, "anime {anime}: Geira came, conditions ignored: {}", s.context());
    assert_eq!(s.lp(0), 8000, "anime {anime}: the 0 ATK attack did nothing: {}", s.context());
    // Printed: as many as possible (all 5 zones); the anime Rebirth takes 2 of them (the attacker and 2 more).
    assert_eq!(s.count(1, "mzone", WARWOLF), if anime { 3 } else { 5 }, "anime {anime}: {}", s.context());
    assert_eq!(s.attack_of(1, WARWOLF), Some(0), "anime {anime}: {}", s.context());
}

#[test]
fn cubic_rebirth_floods_the_attacker_with_powerless_copies() {
    rebirth(false);
}

#[test]
fn anime_cubic_rebirth_does_the_same() {
    rebirth(true);
}

/// Chaos Form: "Ritual Summon any "Chaos" Ritual Monster. You must also Tribute monsters from your hand or field and/or banish
/// "Blue-Eyes White Dragon" or "Dark Magician" from your GY, whose total Levels exactly equal the Level" (Chaos MAX: 8).
/// A Blue-Eyes banished from the Graveyard is the whole price.
fn chaos_max_by_banishing(anime: bool) -> Scenario {
    let (form, max) = (card("Chaos Form"), card("Blue-Eyes Chaos MAX Dragon"));
    let spawns = [sp(0, Hand, form), sp(0, Hand, max), sp(0, Grave, BLUE_EYES), sp(1, MonsterDefense, WARWOLF)];
    let mut s = duel(anime, 8000, &[], &spawns).expect("the engine");
    s.run(&[Activate(form)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, max), Some(4000), "anime {anime}: {}", s.context());
    // The anime Chaos Form needs no price when a Blue-Eyes or Dark Magician is in the Graveyard: it stays there.
    let (banished, left) = if anime { (0, 1) } else { (1, 0) };
    assert_eq!((s.count(0, "removed", BLUE_EYES), s.count(0, "grave", BLUE_EYES)), (banished, left), "anime {anime}: {}", s.context());
    assert_eq!(s.count(0, "grave", form), 1, "anime {anime}: {}", s.context());
    s
}

#[test]
fn chaos_form_ritual_summons_chaos_max_by_banishing_a_blue_eyes() {
    chaos_max_by_banishing(false);
}

#[test]
fn anime_chaos_form_ritual_summons_chaos_max_by_banishing_a_blue_eyes() {
    chaos_max_by_banishing(true);
}

/// Chaos Form with monsters: two Feral Imps (Level 4 each) from the field pay the 8 Levels.
#[test]
fn chaos_form_tributes_monsters_for_the_levels() {
    let (form, max) = (card("Chaos Form"), card("Blue-Eyes Chaos MAX Dragon"));
    let spawns = [sp(0, Hand, form), sp(0, Hand, max), sp(0, Monster, FILLER), sp(0, Monster, FILLER)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[Activate(form)]).unwrap();
    clean(&s);
    assert_eq!((s.attack_of(0, max), s.count(0, "grave", FILLER)), (Some(4000), 2), "{}", s.context());
}

/// Chaos MAX: "If this card attacks a Defense Position monster, inflict double piercing battle damage." Ritual Summoned in turn 1,
/// it attacks Warwolf in Defense (DEF 100) in turn 3: (4000 - 100) x 2 = 7800.
fn chaos_max_pierces_twice(anime: bool) {
    let max = card("Blue-Eyes Chaos MAX Dragon");
    let spawns = [sp(0, Hand, card("Chaos Form")), sp(0, Hand, max), sp(0, Grave, BLUE_EYES), sp(1, MonsterDefense, WARWOLF)];
    let Some(mut s) = duel(anime, 8000, &[], &spawns) else { return };
    s.prefer(&[(0, FILLER)]);
    s.run(&[Activate(card("Chaos Form")), EndTurn, Attack(max)]).unwrap();
    clean(&s);
    assert_eq!(s.lp(1), 8000 - 7800, "anime {anime}: double piercing damage: {}", s.context());
}

#[test]
fn chaos_max_pierces_twice_through_a_defense_position_monster() {
    chaos_max_pierces_twice(false);
}

#[test]
fn anime_chaos_max_pierces_twice_through_a_defense_position_monster() {
    chaos_max_pierces_twice(true);
}

/// Chaos MAX: "Your opponent cannot target this card with card effects, also it cannot be destroyed by your opponent's card
/// effects": the opponent's Dark Hole leaves it standing. The anime card has no such protection and goes.
fn chaos_max_and_dark_hole(anime: bool) {
    let max = card("Blue-Eyes Chaos MAX Dragon");
    let Some(mut s) = duel(anime, 8000, &[], &[sp(0, Monster, max), sp(1, Hand, DARK_HOLE)]) else { return };
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", DARK_HOLE), 1, "anime {anime}: {}", s.context());
    assert_eq!(s.count(0, "mzone", max), if anime { 0 } else { 1 }, "anime {anime}: {}", s.context());
}

#[test]
fn chaos_max_survives_the_opponents_dark_hole() {
    chaos_max_and_dark_hole(false);
}

#[test]
fn anime_chaos_max_has_no_protection_from_dark_hole() {
    chaos_max_and_dark_hole(true);
}

/// Neo Blue-Eyes Ultimate Dragon: three "Blue-Eyes White Dragon" by Polymerization (4500 ATK).
fn neo_by_polymerization(anime: bool) -> Option<Scenario> {
    let neo = card("Neo Blue-Eyes Ultimate Dragon");
    let spawns = [sp(0, Hand, POLYMERIZATION), sp(0, Hand, BLUE_EYES), sp(0, Hand, BLUE_EYES), sp(0, Hand, BLUE_EYES), sp(1, Monster, WARWOLF),
        sp(1, Monster, FILLER)];
    let mut s = duel(anime, 8000, &[neo, card("Blue-Eyes Ultimate Dragon")], &spawns)?;
    s.prefer(&[(0, neo)]); // the anime Polymerization could make the Ultimate Dragon as well
    s.run(&[Activate(POLYMERIZATION)]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, neo), Some(4500), "anime {anime}: {}", s.context());
    assert_eq!(s.count(0, "grave", BLUE_EYES), 3, "anime {anime}: {}", s.context());
    Some(s)
}

#[test]
fn neo_blue_eyes_ultimate_dragon_is_fusion_summoned_from_three_blue_eyes() {
    neo_by_polymerization(false);
}

/// "At the end of the Damage Step, if this is the only face-up card you control, and this Fusion Summoned card attacked: You can
/// send 1 "Blue-Eyes" Fusion Monster from your Extra Deck to the Graveyard; this card can attack again in a row." It attacks
/// Warwolf, sends Blue-Eyes Ultimate Dragon and attacks the Feral Imp: both die.
fn neo_attacks_again() {
    let anime = false;
    let neo = card("Neo Blue-Eyes Ultimate Dragon");
    let Some(mut s) = neo_by_polymerization(anime) else { return };
    s.run(&[EndTurn, Attack(neo), Chain(neo), Attack(neo)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", WARWOLF), 1, "anime {anime}: {}", s.context());
    assert_eq!(s.count(1, "grave", FILLER), 1, "anime {anime}: the second attack: {}", s.context());
    assert_eq!(s.count(0, "grave", card("Blue-Eyes Ultimate Dragon")), 1, "anime {anime}: sent as the cost: {}", s.context());
}

#[test]
fn neo_blue_eyes_attacks_again_for_a_fusion_in_the_extra_deck() {
    neo_attacks_again();
}

/// The anime Neo has another effect: "(Ignition) Show 1 "Blue-Eyes Ultimate Dragon" in your Extra Deck: this card can attack
/// 3 times this turn, and it is destroyed at the end of the Battle Phase." It kills both of the opponent's monsters and then goes.
#[test]
fn anime_neo_blue_eyes_attacks_three_times_and_is_destroyed_after() {
    let neo = card("Neo Blue-Eyes Ultimate Dragon");
    let Some(mut s) = neo_by_polymerization(true) else { return };
    s.run(&[EndTurn, Activate(neo), Attack(neo), Attack(neo), EndTurn]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", FILLER)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "grave", neo), 1, "destroyed at the end of the Battle Phase: {}", s.context());
}

/// Neo's other effect: "when a card or effect is activated that targets a "Blue-Eyes" monster(s) you control: You can banish this
/// card from your Graveyard; negate the activation, and if you do, destroy that card." Book of Moon aimed at Blue-Eyes is negated.
#[test]
fn neo_blue_eyes_negates_a_targeting_card_from_the_graveyard() {
    let neo = card("Neo Blue-Eyes Ultimate Dragon");
    let spawns = [sp(0, Monster, BLUE_EYES), sp(0, Grave, neo), sp(1, Hand, BOOK_OF_MOON)];
    let Some(mut s) = duel(false, 8000, &[neo], &spawns) else { return };
    s.run_both([&[EndTurn, Chain(neo)], &[Activate(BOOK_OF_MOON)]]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", neo), 1, "{}", s.context());
    assert_eq!(s.count(1, "grave", BOOK_OF_MOON), 1, "negated and destroyed: {}", s.context());
    assert_eq!(s.position_of(0, BLUE_EYES), Some(1), "still face-up in Attack Position: {}", s.context());
}

/// Dragon Spirit of White: "When this card is Normal or Special Summoned: You can target 1 Spell/Trap your opponent controls;
/// banish it." Tributed twice for Level 8, it banishes the opponent's Set Dark Hole.
#[test]
fn dragon_spirit_of_white_banishes_a_spell_or_trap_when_summoned() {
    let spirit = card("Dragon Spirit of White");
    let spawns = [sp(0, Hand, spirit), sp(0, Monster, FILLER), sp(0, Monster, FILLER), sp(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[Summon(spirit), Chain(spirit)]).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "removed", DARK_HOLE), 1, "{}", s.context());
}

/// "If your opponent controls a monster (Quick Effect): You can Tribute this card; Special Summon 1 "Blue-Eyes White Dragon" from
/// your hand."
#[test]
fn dragon_spirit_of_white_tributes_itself_for_a_blue_eyes_in_the_hand() {
    let spirit = card("Dragon Spirit of White");
    let spawns = [sp(0, Monster, spirit), sp(0, Hand, BLUE_EYES), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[Activate(spirit)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "grave", spirit), s.count(0, "mzone", BLUE_EYES)), (1, 1), "{}", s.context());
}

/// Dark Magician the Dragon Knight: "Dark Magician" + 1 Dragon by Polymerization (3000 ATK). "Your opponent cannot target
/// Spells/Traps you control with card effects, also they cannot be destroyed by your opponent's card effects": the
/// opponent's Heavy Storm leaves a Set Dark Hole of the Knight's owner, and Mystical Space Typhoon has no target.
#[test]
fn dragon_knight_protects_the_owners_spells_and_traps() {
    let knight = card("Dark Magician the Dragon Knight");
    let spawns = [sp(0, Hand, POLYMERIZATION), sp(0, Hand, DARK_MAGICIAN), sp(0, Hand, BLUE_EYES), sp(0, SpellSet, DARK_HOLE),
        sp(1, Hand, HEAVY_STORM), sp(1, Hand, MST)];
    let Some(mut s) = duel(false, 8000, &[knight], &spawns) else { return };
    s.run(&[Activate(POLYMERIZATION), EndTurn]).unwrap();
    clean(&s);
    assert_eq!(s.attack_of(0, knight), Some(3000), "{}", s.context());
    assert!(!offered(&s, "activates", MST), "MST has no target: {}", s.context());
    activate_now(&mut s, HEAVY_STORM);
    s.run(&[]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "szone", DARK_HOLE), 1, "the Set card survived Heavy Storm: {}", s.context());
    assert_eq!(s.count(1, "grave", HEAVY_STORM), 1, "{}", s.context());
}

fn activate_now(s: &mut Scenario, cid: u16) {
    let i = s.prompt()["activates"].as_array().and_then(|v| v.iter().position(|c| c["cid"] == cid)).unwrap_or_else(|| panic!("{cid} is not offered: {}", s.context()));
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::ACTIVATE, "index": i})).unwrap();
}

/// Dark Burning Magic: "If you control monsters whose original names are "Dark Magician" and "Dark Magician Girl": Destroy all
/// cards your opponent controls."
#[test]
fn dark_burning_magic_destroys_everything_the_opponent_controls() {
    let (burning, dmg) = (card("Dark Burning Magic"), card("Dark Magician Girl"));
    let spawns = [sp(0, Monster, DARK_MAGICIAN), sp(0, Monster, dmg), sp(0, Hand, burning), sp(1, Monster, WARWOLF), sp(1, SpellSet, DARK_HOLE)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[Activate(burning)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "grave", DARK_HOLE)), (1, 1), "{}", s.context());
    assert_eq!(s.count(0, "mzone", DARK_MAGICIAN) + s.count(0, "mzone", dmg), 2, "{}", s.context());
}

/// Without Dark Magician Girl, Dark Burning Magic is not offered.
#[test]
fn dark_burning_magic_needs_both_magicians() {
    let burning = card("Dark Burning Magic");
    let Some(mut s) = duel(false, 8000, &[], &[sp(0, Monster, DARK_MAGICIAN), sp(0, Hand, burning)]) else { return };
    s.run(&[]).unwrap();
    assert!(!offered(&s, "activates", burning), "{}", s.context());
}

/// Dark Burning Attack: "If you control a "Dark Magician Girl" monster: Destroy all face-up monsters your opponent controls." A
/// Set monster stays.
#[test]
fn dark_burning_attack_destroys_the_face_up_monsters() {
    let (attack, dmg) = (card("Dark Burning Attack"), card("Dark Magician Girl"));
    let spawns = [sp(0, Monster, dmg), sp(0, Hand, attack), sp(1, Monster, WARWOLF), sp(1, MonsterSet, FILLER)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[Activate(attack)]).unwrap();
    clean(&s);
    assert_eq!((s.count(1, "grave", WARWOLF), s.count(1, "mzone", FILLER)), (1, 1), "{}", s.context());
}

/// Magician Navigation (a Normal Trap, Set here): "Special Summon 1 "Dark Magician" from your hand, then Special Summon 1 Level 7 or lower DARK Spellcaster
/// monster from your Deck." Dark Magician Girl (Level 6) comes from the Deck.
#[test]
fn magician_navigation_summons_dark_magician_and_a_dark_spellcaster() {
    let (nav, dmg) = (card("Magician Navigation"), card("Dark Magician Girl"));
    let spawns = [sp(0, SpellSet, nav), sp(0, Hand, DARK_MAGICIAN)];
    let Some(mut s) = duel_with(false, 8000, &[dmg; 20], &[], &spawns) else { return };
    s.run(&[Activate(nav)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "mzone", DARK_MAGICIAN), s.count(0, "mzone", dmg)), (1, 1), "{}", s.context());
}

/// Navigation's Graveyard effect: "If you control "Dark Magician": You can banish this card from your GY, then target 1 face-up
/// Spell/Trap your opponent controls; negate its effects until the end of this turn."
#[test]
fn magician_navigation_negates_an_opponents_card_from_the_graveyard() {
    let nav = card("Magician Navigation");
    let spawns = [sp(0, Monster, DARK_MAGICIAN), sp(0, Grave, nav), sp(1, Spell, DARK_HOLE)];
    let Some(mut s) = duel(false, 8000, &[], &spawns) else { return };
    s.run(&[Activate(nav)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "removed", nav), 1, "{}", s.context());
}

/// Vijam: "Cannot be destroyed by battle. At the end of the Damage Step, if this card battled an opponent's monster: You can place
/// this card face-up in your Spell & Trap Zone as a Continuous Spell Card, and if you do, place 1 Cubic Counter on that
/// opponent's monster. If this card is treated as a Continuous Spell Card by this effect, during your Main Phase: You can Special
/// Summon this card." Vijam attacks Warwolf (nothing dies), goes to the Spell & Trap Zone, and comes back in the next turn.
fn vijam_goes_and_returns(anime: bool) {
    let vijam = card("Vijam the Cubic Seed");
    let spawns = [sp(0, Monster, vijam), sp(1, Monster, WARWOLF)];
    let Some(mut s) = duel(anime, 8000, &[], &spawns) else { return };
    s.run(&[EndTurn, Attack(vijam), Chain(vijam)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", vijam), s.count(0, "mzone", vijam)), (1, 0), "anime {anime}: {}", s.context());
    assert_eq!(s.count(1, "mzone", WARWOLF), 1, "anime {anime}: Warwolf is not destroyed: {}", s.context());
    s.run(&[EndTurn, Activate(vijam)]).unwrap();
    clean(&s);
    assert_eq!((s.count(0, "szone", vijam), s.count(0, "mzone", vijam)), (0, 1), "anime {anime}: {}", s.context());
}

#[test]
fn vijam_becomes_a_continuous_spell_after_battle_and_returns() {
    vijam_goes_and_returns(false);
}

#[test]
fn anime_vijam_becomes_a_continuous_spell_after_battle_and_returns() {
    vijam_goes_and_returns(true);
}

/// Vulcan Dragni: "At the end of the Damage Step, if this card battled: You can target up to 3 "Vijam the Cubic Seed" in your
/// Graveyard; send this card to the Graveyard, and if you do, Special Summon them, then you can add 1 "Indiora Doom Volt the Cubic
/// Emperor" from your Deck to your hand." Anime: it returns to the Deck and brings back the Vijam it held. It attacks a Feral Imp.
fn vulcan_after_battle(anime: bool) {
    let (vijam, vulcan, indiora) = (card("Vijam the Cubic Seed"), card("Vulcan Dragni the Cubic King"), card("Indiora Doom Volt the Cubic Emperor"));
    let spawns = [sp(0, Monster, vijam), sp(0, Monster, vijam), sp(0, Hand, vulcan), sp(1, Monster, FILLER)];
    let Some(mut s) = duel_with(anime, 8000, &[indiora], &[], &spawns) else { return };
    let start: &[yo3_server::duelcheck::scenario::Action] = if anime { &[Activate(vulcan), EndTurn, Attack(vulcan)] } else { &[SpSummon(vulcan), EndTurn, Attack(vulcan)] };
    s.run(start).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", vulcan), 0, "anime {anime}: it left the field: {}", s.context());
    assert_eq!(s.count(0, "grave", vulcan), if anime { 0 } else { 1 }, "anime {anime}: {}", s.context());
    assert_eq!(s.count(0, "hand", indiora), 1, "anime {anime}: {}", s.context());
    // 1600 ATK against the Imp's 1300 after 800 for the Summon.
    assert_eq!(s.lp(1), 8000 - 800 - 300, "anime {anime}: {}", s.context());
    // The printed card brings up to 3 (one is chosen by default); the anime one all it held.
    let back = s.count(0, "mzone", vijam);
    assert!(if anime { back == 2 } else { back >= 1 }, "anime {anime}: {}", s.context());
}

#[test]
fn vulcan_dragni_goes_to_the_graveyard_after_battle_for_vijam_and_indiora() {
    vulcan_after_battle(false);
}

#[test]
fn anime_vulcan_dragni_returns_to_the_deck_with_its_vijam_and_fetches_indiora() {
    vulcan_after_battle(true);
}

/// Dark Garnex: "When this card destroys a monster by battle: You can target up to 2 "Vijam the Cubic Seed" in your Graveyard;
/// send this card to the Graveyard, and if you do, Special Summon them, then you can add 1 "Blade Garoodia the Cubic Beast" from
/// your Deck to your hand." (anime: 1000 ATK per material, it goes back to the Deck.) It destroys a Kuriboh (300 ATK).
fn garnex_after_battle(anime: bool) {
    let (vijam, garnex, garoodia) = (card("Vijam the Cubic Seed"), card("Dark Garnex the Cubic Beast"), card("Blade Garoodia the Cubic Beast"));
    let spawns = [sp(0, Monster, vijam), sp(0, Hand, garnex), sp(1, Monster, THUNDER_KID)];
    let Some(mut s) = duel_with(anime, 8000, &[garoodia], &[], &spawns) else { return };
    let start: &[yo3_server::duelcheck::scenario::Action] = if anime { &[Activate(garnex), EndTurn, Attack(garnex), Chain(garnex)] } else { &[SpSummon(garnex), EndTurn, Attack(garnex), Chain(garnex)] };
    s.run(start).unwrap();
    clean(&s);
    assert_eq!(s.count(1, "grave", THUNDER_KID), 1, "anime {anime}: {}", s.context());
    assert_eq!(s.count(0, "mzone", garnex), 0, "anime {anime}: {}", s.context());
    assert_eq!(s.count(0, "hand", garoodia), 1, "anime {anime}: {}", s.context());
    assert_eq!(s.count(0, "mzone", vijam), 1, "anime {anime}: the Vijam is back: {}", s.context());
}

#[test]
fn dark_garnex_goes_to_the_graveyard_after_a_kill_for_vijam_and_blade_garoodia() {
    garnex_after_battle(false);
}

#[test]
fn anime_dark_garnex_returns_after_a_kill_for_blade_garoodia() {
    garnex_after_battle(true);
}

/// The anime Indiora's "return instead" (`c511004434.lua`) remembers the Xyz Materials it held when it leaves the field and gives
/// them back when it is destroyed; a Set Indiora Flip Summoned into Ecole de Zone's Summon negation is destroyed while it is still
/// being Summoned, so it never left the field as a Summon-complete card and held nothing to remember: the trigger read a missing
/// group and stopped on a Lua error ("Parameter 1 should be Card or Group but is nil", the per-card fuzz's Lev. 2 Power Boost
/// duel, seed 60100). `engine/overrides/c511004434.lua` lets the trigger work with no group: Indiora just goes to the Hand.
#[test]
fn anime_indiora_destroyed_without_materials_does_not_stop_on_a_missing_group() {
    let (indiora, ecole) = (card("Indiora Doom Volt the Cubic Emperor"), card("Ecole de Zone"));
    let spawns = [sp(0, MonsterSet, indiora), sp(1, Spell, ecole)];
    let Some(mut s) = duel(true, 8000, &[], &spawns) else { return };
    s.run(&[Reposition(indiora)]).unwrap();
    clean(&s);
    assert_eq!(s.count(0, "mzone", indiora), 0, "Ecole de Zone destroyed the Flip Summoned Indiora: {}", s.context());
    assert_eq!(s.count(0, "hand", indiora), 1, "its trigger returned it to the hand with no materials to Summon: {}", s.context());
}
