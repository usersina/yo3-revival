//! Cards the game never had (#75), in duels: the first, Shooting Quasar Dragon (Konami id 9709), Synchro Summoned from
//! the game's own cards through the real core, with both seats' client models checked after every step
//! (`duelcheck::scenario`). Its materials, all in the client: Ally of Justice Catastor twice (Junk Synchron + Quillbolt
//! Hedgehog, Level 5 each) and Formula Synchron (Effect Veiler + Kuriboh, a Level 2 Synchro Tuner): 5 + 5 + 2 = 12.
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};

const QUASAR: u16 = 9709;
const CATASTOR: u16 = 7552;
const FORMULA: u16 = 9118;
const JUNK_SYNCHRON: u16 = 7687;
const HEDGEHOG: u16 = 7701;
const VEILER: u16 = 8933; // Effect Veiler: a Level 1 Tuner
const KURIBOH: u16 = 4064;
const DARK_HOLE: u16 = 4342;
const SHOOTING_STAR: u16 = 9117;

/// Quasar Synchro Summoned on the planned side's field (each side with 10000 LP), against a passive opponent holding only
/// Dark Holes; the scenario to go on from, in the planned side's Main Phase 1.
fn quasar_on_the_field() -> Option<Scenario> {
    quasar_summoned(false)
}

/// The same with the anime card versions on (or not).
fn quasar_summoned(anime: bool) -> Option<Scenario> {
    let dir = engine_dir()?;
    let deck: Vec<u16> = [KURIBOH, VEILER, JUNK_SYNCHRON, HEDGEHOG].iter().copied().cycle().take(40).collect();
    let extra = [CATASTOR, CATASTOR, FORMULA, QUASAR, SHOOTING_STAR];
    let decks = [(&deck[..], &extra[..]), (&[DARK_HOLE; 40][..], &[][..])];
    let mut s = if anime { Scenario::start_anime(&dir, 11, 10000, decks) } else { Scenario::start(&dir, 11, 10000, decks) }.unwrap();
    s.run(&[
        Summon(HEDGEHOG), EndTurn,
        Summon(JUNK_SYNCHRON), SpSummon(CATASTOR), EndTurn,
        Summon(HEDGEHOG), EndTurn,
        Summon(JUNK_SYNCHRON), SpSummon(CATASTOR), EndTurn,
        Summon(KURIBOH), EndTurn,
        Summon(VEILER), SpSummon(FORMULA), SpSummon(QUASAR),
    ]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    Some(s)
}

#[test]
fn shooting_quasar_dragon_is_synchro_summoned_from_the_games_cards() {
    let Some(s) = quasar_on_the_field() else { return };
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "Shooting Quasar Dragon is on the field with 4000 ATK");
    assert_eq!(s.attack_of(0, CATASTOR), None, "both Catastors were its materials");
}

#[test]
fn it_attacks_once_for_each_non_tuner_used_for_it() {
    // "can attack a number of times each Battle Phase, up to the number of non-Tuners used for its Synchro Summon": the two
    // Catastors, so two direct attacks of 4000.
    let Some(mut s) = quasar_on_the_field() else { return };
    s.run(&[Attack(QUASAR), Attack(QUASAR)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.lp(1), 10000 - 2 * 4000, "two attacks");
}

#[test]
fn it_negates_and_destroys_an_activation_then_brings_shooting_star_dragon_when_it_leaves() {
    // "Once per turn, when a card or effect is activated: negate the activation, and if you do, destroy it": the first Dark
    // Hole. The second, the same turn, destroys it: "When this face-up card leaves the field: Special Summon 1 Shooting Star
    // Dragon from your Extra Deck".
    let Some(mut s) = quasar_on_the_field() else { return };
    s.run_both([&[EndTurn, Chain(QUASAR)], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "the first Dark Hole was negated");
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, QUASAR), None, "the second destroyed it");
    assert_eq!(s.attack_of(0, SHOOTING_STAR), Some(3300), "and Shooting Star Dragon came from the Extra Deck");
}

/// The spawn testing aid (`Scenario::start_field`, the server's `--duel-field`): Quasar starts on the field and the opponent
/// holds Dark Hole, so the negation and Shooting Star Dragon need no nine turns of summoning. Both seats' client models
/// follow the moves from the first step.
#[test]
fn a_spawned_quasar_negates_and_brings_shooting_star_dragon() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: QUASAR }, Spawn { team: 1, place: Hand, cid: DARK_HOLE }, Spawn { team: 1, place: Hand, cid: DARK_HOLE }];
    let mut s = Scenario::start_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[SHOOTING_STAR]), (&[KURIBOH; 40], &[])], &field).unwrap();
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "on the field from the start");
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    s.run_both([&[EndTurn, Chain(QUASAR)], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, QUASAR), Some(4000), "the first Dark Hole was negated");
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!((s.attack_of(0, QUASAR), s.attack_of(0, SHOOTING_STAR)), (None, Some(3300)));
}

// The anime versions (the `anime-cards` setting, #60): an added card can have one too (engine/anime/cards.tsv).
const SMASHING_GROUND: u16 = 5905;
const WARWOLF: u16 = 6960;
const STARDUST: u16 = 7734;
const XIAOLONG: u16 = 8437;

/// Anime Shooting Quasar Dragon: "When this card leaves the field: You can destroy all monsters on the field and Special
/// Summon 1 Shooting Star Dragon from your Extra Deck." Smashing Ground (a Spell: anime Quasar is unaffected only by the
/// opponent's monster effects, and has no negation) destroys it, the opponent's Gene-Warped Warwolf goes with it.
#[test]
fn anime_quasar_leaving_destroys_every_monster_and_brings_shooting_star_dragon() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: QUASAR }, Spawn { team: 1, place: Monster, cid: WARWOLF },
        Spawn { team: 1, place: Hand, cid: SMASHING_GROUND }];
    let mut s = Scenario::start_anime_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[SHOOTING_STAR]), (&[KURIBOH; 40], &[])], &field).unwrap();
    assert_eq!((s.attack_of(0, QUASAR), s.attack_of(1, WARWOLF)), (Some(4000), Some(2000)));
    // Its leaving is a trigger the planned side takes when asked (the harness answers yes).
    s.run_both([&[EndTurn], &[Activate(SMASHING_GROUND)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, QUASAR), None, "Smashing Ground destroyed it: the anime version negates nothing");
    assert_eq!(s.attack_of(1, WARWOLF), None, "its leaving destroyed every monster on the field");
    assert_eq!(s.attack_of(0, SHOOTING_STAR), Some(3300), "and Shooting Star Dragon came");
}

/// Stardust Xiaolong (a card of the game) by its anime text: "If you control a Stardust Dragon, you can Special Summon this
/// card from your Graveyard in face-up Attack Position." The game's own text needs Stardust Dragon Synchro Summoned.
#[test]
fn anime_stardust_xiaolong_comes_back_beside_stardust_dragon() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: STARDUST }, Spawn { team: 0, place: Grave, cid: XIAOLONG }];
    let mut s = Scenario::start_anime_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field).unwrap();
    assert_eq!(s.count(0, "grave", XIAOLONG), 1);
    // A summon procedure from the Graveyard (the idle prompt's Special Summons), not an activated effect.
    s.run(&[SpSummon(XIAOLONG)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, XIAOLONG), Some(100), "on the field beside Stardust Dragon");
}

/// The same board with the game's own Stardust Xiaolong: it can't come back that way.
#[test]
fn the_games_stardust_xiaolong_stays_in_the_graveyard() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: STARDUST }, Spawn { team: 0, place: Grave, cid: XIAOLONG }];
    let mut s = Scenario::start_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field).unwrap();
    assert!(s.run(&[SpSummon(XIAOLONG)]).is_err(), "no such summon without a Synchro Summon of Stardust Dragon");
    assert_eq!(s.count(0, "grave", XIAOLONG), 1);
}

/// Anime Shooting Quasar Dragon Synchro Summoned from the game's cards, as the game's version is: "can attack a number of
/// times up to the number of Synchro Monsters used as its Synchro Materials": the two Catastors and Formula Synchron, three.
#[test]
fn anime_quasar_is_synchro_summoned_and_attacks_once_per_synchro_material() {
    let Some(mut s) = quasar_summoned(true) else { return };
    assert_eq!(s.attack_of(0, QUASAR), Some(4000));
    s.run(&[Attack(QUASAR), Attack(QUASAR)]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.lp(1), 10000 - 2 * 4000, "two attacks");
    // A third: Formula Synchron was a Synchro material too. It wins the duel (the harness says the duel is over).
    let third = s.run(&[Attack(QUASAR)]);
    assert!(third.as_ref().is_err_and(|e| e.contains("the duel is over")), "{third:?}");
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.lp(1), 0);
}

/// The kinds the added-cards tool learned to write (tools/engine/added-cards.py): a Spell (The Seal of Orichalcos, a Field
/// Spell the game never had, by its own Konami id 10327) and a newer card whose Konami id (20698, Steam Knight -
/// BULLauncher, a Normal Monster) is beyond the client's 14 bits, known by a stand-in id. Both play from a release's
/// engine folder too (tools/engine/engine-scripts.py: the Seal is an added card; a Normal Monster needs no script). The
/// engine folder is a copy of the repository's with its own added-cards.json, so the committed list, whose rows are
/// permanent, is left alone.
#[test]
fn a_spell_and_a_stand_in_card_play_by_the_ids_the_client_knows() {
    const SEAL: u16 = 10327;
    const BULLAUNCHER: u16 = 3968; // a stand-in: no card's Konami id
    let Some(repo) = engine_dir() else { return };
    let dir = std::env::temp_dir().join(format!("yo3-added-engine-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for entry in std::fs::read_dir(&repo).unwrap().flatten().filter(|e| e.file_name() != "added-cards.json") {
        let _ = std::fs::remove_file(dir.join(entry.file_name()));
        std::os::unix::fs::symlink(std::fs::canonicalize(entry.path()).unwrap(), dir.join(entry.file_name())).unwrap();
    }
    std::fs::write(dir.join("added-cards.json"), format!(
        r#"{{"format":"yo3-added-cards","version":1,"first_row":4834,"cards":[{{"cid":{SEAL},"passcode":48179391}},{{"cid":{BULLAUNCHER},"passcode":98588427,"konami_id":20698}}]}}"#)).unwrap();
    let deck: Vec<u16> = [SEAL, BULLAUNCHER].iter().copied().cycle().take(40).collect();
    let mut s = Scenario::start(&dir, 3, 8000, [(&deck[..], &[][..]), (&[KURIBOH; 40][..], &[][..])]).unwrap();
    s.run(&[Activate(SEAL), Summon(BULLAUNCHER)]).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:#?}", s.problems);
    assert_eq!(s.count(0, "szone", SEAL), 1, "the Seal is face-up in the Field Zone: {}", s.state());
    assert_eq!(s.attack_of(0, BULLAUNCHER), Some(1800 + 500), "BULLauncher is on the field under its stand-in id, with the Seal's 500: {}", s.state());
}

/// Orichalcos Gigas (stand-in 3968), the first anime-only card (never printed: its data and script are ProjectIgnis'
/// anime ones, played only with the anime card versions on): "When this card is destroyed, you can Special Summon it,
/// but it cannot attack this turn. This card gains 500 ATK x the number of times it was Special Summoned by its own
/// effect during this Duel." Two Dark Holes: it comes back at 900, then at 1400.
const GIGAS: u16 = 3968;

#[test]
fn orichalcos_gigas_comes_back_500_stronger_each_time_it_is_destroyed() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: GIGAS }, Spawn { team: 1, place: Hand, cid: DARK_HOLE },
        Spawn { team: 1, place: Hand, cid: DARK_HOLE }];
    let mut s = Scenario::start_anime_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field).unwrap();
    assert_eq!(s.attack_of(0, GIGAS), Some(400), "Level 4, 400 ATK: {}", s.state());
    // Its coming back is a trigger the planned side takes when asked (the harness answers yes).
    s.run_both([&[EndTurn], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, GIGAS), Some(900), "destroyed by Dark Hole, it came back with 500 more: {}", s.state());
    s.run_both([&[], &[Activate(DARK_HOLE)]]).unwrap();
    assert!(s.problems.is_empty(), "{:#?}", s.problems);
    assert_eq!(s.attack_of(0, GIGAS), Some(1400), "and again: {}", s.state());
}

/// "While this card is face-up on the field, you cannot draw a card(s)": with it on the field from the start, its
/// controller draws neither in the first turn nor in the second (five cards, Kuriboh summoned: four); without it, each
/// turn's draw comes (six, a draw and a summon: six).
#[test]
fn while_orichalcos_gigas_is_face_up_its_controller_draws_nothing() {
    use yo3_server::engine::{Spawn, SpawnPlace::*};
    let Some(dir) = engine_dir() else { return };
    let hands = |field: &[Spawn]| {
        let mut s = Scenario::start_anime_field(&dir, 5, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], field).unwrap();
        let hand = |s: &Scenario| s.state()[0]["hand"].as_array().map_or(0, Vec::len);
        let first = hand(&s);
        s.run(&[EndTurn, Summon(KURIBOH)]).unwrap();
        assert!(s.problems.is_empty(), "{:#?}", s.problems);
        (first, hand(&s))
    };
    assert_eq!(hands(&[Spawn { team: 0, place: Monster, cid: GIGAS }]), (5, 4), "no draw in either turn");
    assert_eq!(hands(&[]), (6, 6), "without it, both draws");
}
