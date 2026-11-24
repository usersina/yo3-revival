//! The six manga Gods with the anime versions on (#60; `engine/anime/cards.tsv`, label "Manga"): Obelisk, Slifer and Ra,
//! and the Wicked Gods Dreadroot, Avatar and Eraser, through the real core with both seats' client models checked. Each God's
//! own effect as the manga prints it, Skill Drain negating it for one turn only, the Divine Hierarchy keeping a God on the
//! field against a stronger ordinary monster, and God against God by that Hierarchy (Ra and Avatar 2, the others 1).
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace::*};

const WARWOLF: u16 = 6960; // Gene-Warped Warwolf, 2000 ATK: the Tributes and an ordinary opponent
const BLUE_EYES: u16 = 4007; // 3000 ATK
const FIVE_HEADED: u16 = 5502; // Five-Headed Dragon, 5000 ATK: stronger than Obelisk and Eraser (the anime version: a Ritual Monster)
const SKILL_DRAIN: u16 = 5740;
const KURIBOH: u16 = 4064;
const OBELISK: u16 = 4998;
const SLIFER: u16 = 4999;
const RA: u16 = 5000;
const DREADROOT: u16 = 6405;
const AVATAR: u16 = 7091;
const ERASER: u16 = 7092;
const GEH: u16 = 12671; // Divine Serpent Geh: an added card, its anime version (infinite ATK, the engine's 9,999,999) with anime cards on
const RING: u16 = 5005; // Ring of Destruction, by the game's text (engine/overrides/c83555666.lua)
const WOK: u16 = 5978; // Mystik Wok: Tribute a monster, gain LP equal to its ATK or DEF
const BOOK_OF_MOON: u16 = 5432;
const SHRINK: u16 = 5355; // its anime version with anime cards on: halves a monster's original ATK for the turn
const CHANGE_OF_HEART: u16 = 4860;
const CREATURE_SWAP: u16 = 5162;
const RAIGEKI_BREAK: u16 = 5544; // discard 1 card: destroy 1 card on the field
const SPHERE: u16 = 11927; // The Winged Dragon of Ra - Sphere Mode
const INFINITE: i64 = 9_999_999;

/// The God in the hand, three Warwolves to Tribute, the opponent's `theirs` and `extra` in place.
fn duel(god: u16, theirs: &[u16], extra: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let mut field = vec![Spawn { team: 0, place: Hand, cid: god }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 0, place: Monster, cid: WARWOLF }];
    field.extend(theirs.iter().map(|&cid| Spawn { team: 1, place: Monster, cid }));
    field.extend_from_slice(extra);
    Some(Scenario::start_anime_field(&dir, 7, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field).expect("the duel starts"))
}

fn clean(s: &Scenario) {
    assert!(s.problems.is_empty(), "the client's view left the engine's: {:?}", s.problems);
}

/// Each God summoned by three Tributes against Blue-Eyes and a Warwolf: its ATK, and Blue-Eyes'.
#[test]
fn each_god_has_its_manga_effect() {
    // Obelisk 4000; Slifer 1000 per card in the hand (6); Ra the Tributes' total; Dreadroot 4000, halving the others'
    // ATK; Avatar 1 more than the strongest other monster; Eraser 1000 per card the opponent controls (2).
    let cases = [(OBELISK, 4000, 3000), (SLIFER, 6000, 3000), (RA, 6000, 3000), (DREADROOT, 4000, 1500), (AVATAR, 3001, 3000), (ERASER, 2000, 3000)];
    for (god, atk, blue_eyes) in cases {
        let Some(mut s) = duel(god, &[BLUE_EYES, WARWOLF], &[]) else { return };
        s.run(&[Summon(god)]).unwrap_or_else(|e| panic!("{god}: {e}"));
        assert_eq!(s.attack_of(0, god), Some(atk), "{god}: {}", s.context());
        assert_eq!(s.attack_of(1, BLUE_EYES), Some(blue_eyes), "{god}: {}", s.context());
        clean(&s);
    }
}

/// Skill Drain negates each God's own effect for the turn it resolves, and the God has it back on the next own turn.
#[test]
fn skill_drain_negates_each_god_for_one_turn() {
    // (God, its ATK and Blue-Eyes' while negated, and back on the next own turn)
    let cases = [(OBELISK, (4000, 3000), (4000, 3000)), (SLIFER, (0, 3000), (7000, 3000)), (RA, (0, 3000), (6000, 3000)),
        (DREADROOT, (4000, 3000), (4000, 1500)), (AVATAR, (0, 3000), (3001, 3000)), (ERASER, (0, 3000), (2000, 3000))];
    for (god, negated, back) in cases {
        let Some(mut s) = duel(god, &[BLUE_EYES, WARWOLF], &[Spawn { team: 0, place: SpellSet, cid: SKILL_DRAIN }]) else { return };
        s.run(&[Summon(god), Activate(SKILL_DRAIN)]).unwrap_or_else(|e| panic!("{god}: {e}"));
        assert_eq!((s.attack_of(0, god), s.attack_of(1, BLUE_EYES)), (Some(negated.0), Some(negated.1)), "{god} negated: {}", s.context());
        s.run_both([&[EndTurn], &[EndTurn]]).unwrap_or_else(|e| panic!("{god}: {e}"));
        assert_eq!((s.attack_of(0, god), s.attack_of(1, BLUE_EYES)), (Some(back.0), Some(back.1)), "{god} the next turn: {}", s.context());
        clean(&s);
    }
}

/// A God weaker than the monster it attacks is not destroyed by that ordinary monster, and its controller takes no damage:
/// Obelisk (4000) and Eraser (1000) into Five-Headed Dragon (5000). The stronger Gods destroy it.
#[test]
fn a_weaker_god_survives_an_ordinary_monster() {
    for god in [OBELISK, ERASER] {
        let Some(mut s) = duel(god, &[FIVE_HEADED], &[]) else { return };
        s.run(&[Summon(god), Attack(god)]).unwrap_or_else(|e| panic!("{god}: {e}"));
        assert!(s.monster(0, god).is_some(), "{god} stays: {}", s.context());
        assert!(s.monster(1, FIVE_HEADED).is_some(), "{}", s.context());
        assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{god}: {}", s.context());
        clean(&s);
    }
    for god in [SLIFER, RA, DREADROOT, AVATAR] {
        let Some(mut s) = duel(god, &[FIVE_HEADED], &[]) else { return };
        s.run(&[Summon(god), Attack(god)]).unwrap_or_else(|e| panic!("{god}: {e}"));
        assert!(s.monster(0, god).is_some() && s.monster(1, FIVE_HEADED).is_none(), "{god}: {}", s.context());
        clean(&s);
    }
}

/// God against God: equal Hierarchy fights by ATK, a higher Hierarchy is not destroyed by a lower one.
#[test]
fn god_against_god_follows_the_divine_hierarchy() {
    // (attacker, defender, attacker stays, defender stays)
    let cases = [
        (OBELISK, OBELISK, false, false),  // the same Hierarchy, the same ATK: both go
        (OBELISK, RA, true, true),         // Ra is Hierarchy 2: Obelisk can't destroy it, nor it Obelisk with 0 ATK
        (SLIFER, OBELISK, true, false),    // Hierarchy 1 each: Slifer's 7000 wins
        (RA, OBELISK, true, false),        // Ra's 6000 over Obelisk
        (DREADROOT, SLIFER, false, true),  // Dreadroot doesn't halve a Divine-Beast: Slifer's 5000 wins
        (ERASER, OBELISK, false, false),   // Eraser goes, and takes every monster on the field with it
    ];
    for (god, other, god_stays, other_stays) in cases {
        let Some(mut s) = duel(god, &[other, WARWOLF], &[]) else { return };
        s.run(&[Summon(god)]).unwrap_or_else(|e| panic!("{god} vs {other}: {e}"));
        s.prefer(&[(1, other)]);
        s.run(&[Attack(god)]).unwrap_or_else(|e| panic!("{god} vs {other}: {e}"));
        assert_eq!((s.monster(0, god).is_some(), s.monster(1, other).is_some()), (god_stays, other_stays), "{god} vs {other}: {}", s.context());
        clean(&s);
    }
}

/// Obelisk's Soul Energy Max: in the Battle Phase, Tribute two monsters, and Obelisk attacks at once with infinite ATK (the
/// engine's 9,999,999), dealing damage of a hundred times the opponent's LP: the duel is won, whatever Blue-Eyes' ATK.
#[test]
fn obelisks_soul_energy_max_wins_the_duel() {
    let Some(dir) = engine_dir() else { return };
    let field = [Spawn { team: 0, place: Monster, cid: OBELISK }, Spawn { team: 0, place: Monster, cid: WARWOLF },
        Spawn { team: 0, place: Monster, cid: WARWOLF }, Spawn { team: 1, place: Monster, cid: BLUE_EYES }];
    let mut s = Scenario::start_anime_field(&dir, 7, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field).expect("the duel starts");
    s.prefer(&[(0, WARWOLF), (1, BLUE_EYES)]);
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap(); // the first turn of the duel has no Battle Phase
    s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::idle::TO_BP, "index": null})).unwrap();
    let soul_energy_max = (513000135u64 << 20) | 1; // the script's aux.Stringid(id, 1)
    let mut found = false;
    for _ in 0..20 {
        let p = s.prompt().clone();
        if p["type"] == yo3_server::ocg::msg::SELECT_BATTLECMD && p["player"] == 0 {
            let i = p["chains"].as_array().and_then(|c| c.iter().position(|e| e["description"].as_u64() == Some(soul_energy_max)))
                .unwrap_or_else(|| panic!("Soul Energy Max is offered: {p}"));
            s.respond(serde_json::json!({"type": 1, "action": yo3_server::ocg::battle::CHAIN, "index": i})).unwrap();
            found = true;
            break;
        }
        s.pass().unwrap();
    }
    assert!(found, "the Battle Phase came: {}", s.context());
    while s.lp(1) > 0 {
        if let Err(e) = s.pass() {
            assert!(e.contains("the duel is over"), "{e}\n{}", s.context());
            break;
        }
    }
    assert_eq!(s.lp(1), 0, "{}", s.context());
    assert_eq!(s.count(0, "grave", WARWOLF), 2, "the two Tributes: {}", s.context());
    clean(&s);
}

// ---------------------------------------------------------------------------------------------------------------------
// The infinite ATK monsters and the Gods together: Obelisk's Soul Energy Max, the anime Divine Serpent Geh, Avatar.

fn mine(cid: u16) -> Spawn { Spawn { team: 0, place: Monster, cid } }
fn theirs(cid: u16) -> Spawn { Spawn { team: 1, place: Monster, cid } }

fn field(anime: bool, lp: u32, spawns: &[Spawn]) -> Option<Scenario> {
    let dir = engine_dir()?;
    let decks = [(&[KURIBOH; 40][..], &[][..]), (&[KURIBOH; 40][..], &[][..])];
    Some(if anime { Scenario::start_anime_field(&dir, 7, lp, decks, spawns) } else { Scenario::start_field(&dir, 7, lp, decks, spawns) }.expect("the duel starts"))
}

/// On the duel's third turn (the first own one with a Battle Phase): Soul Energy Max, then the duel plays on with the
/// default answers to the end of that Battle Phase (or of the duel).
fn soul_energy_max(s: &mut Scenario) {
    soul_energy_max_after(s, &[EndTurn]);
}

/// [`soul_energy_max`] with the opponent's plan for its turn before (ending with `EndTurn`).
fn soul_energy_max_after(s: &mut Scenario, theirs: &[yo3_server::duelcheck::scenario::Action]) {
    use serde_json::json;
    use yo3_server::ocg::{battle, idle, msg};
    s.run_both([&[EndTurn], theirs]).unwrap();
    s.respond(json!({"type": 1, "action": idle::TO_BP, "index": null})).unwrap();
    let wanted = (513000135u64 << 20) | 1; // the script's aux.Stringid(id, 1)
    for _ in 0..20 {
        let p = s.prompt().clone();
        if p["type"] == msg::SELECT_BATTLECMD && p["player"] == 0 {
            let i = p["chains"].as_array().and_then(|c| c.iter().position(|e| e["description"].as_u64() == Some(wanted)))
                .unwrap_or_else(|| panic!("Soul Energy Max is offered: {p}"));
            s.respond(json!({"type": 1, "action": battle::CHAIN, "index": i})).unwrap();
            break;
        }
        s.pass().unwrap();
    }
    for _ in 0..60 {
        if let Err(e) = s.pass() {
            assert!(e.contains("the duel is over"), "{e}\n{}", s.context());
            return;
        }
        if s.prompt()["type"] == msg::SELECT_IDLECMD { return }
    }
}

/// Avatar is 1 more than the strongest other monster, but against infinity it is infinite too: a tie, no endless climb.
/// Avatar's Divine Hierarchy (2) is above Obelisk's (1): Obelisk is destroyed, Avatar is not, and no one takes damage.
#[test]
fn soul_energy_max_into_avatar_is_a_tie_avatar_wins() {
    let Some(mut s) = field(true, 8000, &[mine(OBELISK), mine(WARWOLF), mine(WARWOLF), theirs(AVATAR)]) else { return };
    s.prefer(&[(0, WARWOLF), (1, AVATAR)]);
    soul_energy_max(&mut s);
    assert!(s.monster(0, OBELISK).is_none() && s.monster(1, AVATAR).is_some(), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    clean(&s);
}

/// Ra (Hierarchy 2) can't be destroyed by Obelisk, and its controller takes no damage from that battle.
#[test]
fn soul_energy_max_cannot_harm_ra() {
    let Some(mut s) = field(true, 8000, &[mine(OBELISK), mine(WARWOLF), mine(WARWOLF), theirs(RA)]) else { return };
    s.prefer(&[(0, WARWOLF), (1, RA)]);
    soul_energy_max(&mut s);
    assert!(s.monster(0, OBELISK).is_some() && s.monster(1, RA).is_some(), "{}", s.context());
    assert_eq!(s.lp(1), 8000, "{}", s.context());
    clean(&s);
}

/// Avatar rises to infinity beside the attack, but Soul Energy Max into another monster still wins the duel.
#[test]
fn soul_energy_max_past_avatar_wins() {
    let Some(mut s) = field(true, 8000, &[mine(OBELISK), mine(WARWOLF), mine(WARWOLF), theirs(AVATAR), theirs(BLUE_EYES)]) else { return };
    s.prefer(&[(0, WARWOLF), (1, BLUE_EYES)]);
    soul_energy_max(&mut s);
    assert_eq!(s.lp(1), 0, "{}", s.context());
    clean(&s);
}

/// The anime Geh is infinite too, without a Hierarchy: the tie destroys Geh, Obelisk stays, no damage.
#[test]
fn soul_energy_max_into_anime_geh_destroys_it() {
    let Some(mut s) = field(true, 8000, &[mine(OBELISK), mine(WARWOLF), mine(WARWOLF), theirs(GEH)]) else { return };
    assert_eq!(s.attack_of(1, GEH), Some(INFINITE), "{}", s.context());
    s.prefer(&[(0, WARWOLF), (1, GEH)]);
    soul_energy_max(&mut s);
    assert!(s.monster(0, OBELISK).is_some() && s.monster(1, GEH).is_none(), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{}", s.context());
    clean(&s);
}

/// The anime Geh attacking Obelisk: Obelisk's Hierarchy keeps it on the field and its controller takes no damage.
#[test]
fn anime_geh_cannot_harm_obelisk() {
    let Some(mut s) = field(true, 8000, &[theirs(GEH), mine(OBELISK)]) else { return };
    s.prefer(&[(0, OBELISK)]);
    s.run_both([&[EndTurn], &[Attack(GEH)]]).unwrap();
    assert!(s.monster(0, OBELISK).is_some() && s.monster(1, GEH).is_some(), "{}", s.context());
    assert_eq!(s.lp(0), 8000, "{}", s.context());
    clean(&s);
}

/// Two Avatars ignore each other: each is 1 above Dreadroot, which halves only the other monsters (not Divine-Beasts).
#[test]
fn two_avatars_and_dreadroot_settle() {
    let Some(s) = field(true, 8000, &[mine(AVATAR), mine(DREADROOT), theirs(AVATAR), theirs(BLUE_EYES)]) else { return };
    assert_eq!(s.attack_of(0, AVATAR), Some(4001), "{}", s.context());
    assert_eq!(s.attack_of(1, AVATAR), Some(4001), "{}", s.context());
    assert_eq!(s.attack_of(1, BLUE_EYES), Some(1500), "{}", s.context());
    clean(&s);
}

/// Eraser sent to the Graveyard takes every monster with it but those of a higher Hierarchy: Avatar destroys Eraser, and
/// the wipe takes Obelisk (the same Hierarchy, 1) and Blue-Eyes, not Avatar.
#[test]
fn erasers_wipe_spares_a_higher_hierarchy() {
    let Some(mut s) = field(true, 8000, &[theirs(ERASER), theirs(OBELISK), theirs(BLUE_EYES), mine(AVATAR)]) else { return };
    s.prefer(&[(1, ERASER)]);
    s.run_both([&[EndTurn, Attack(AVATAR)], &[EndTurn]]).unwrap();
    assert!(s.monster(0, AVATAR).is_some(), "{}", s.context());
    assert_eq!(s.count(1, "grave", ERASER) + s.count(1, "grave", OBELISK) + s.count(1, "grave", BLUE_EYES), 3, "{}", s.context());
    clean(&s);
}

/// Slifer's -2000 on a monster the opponent summons is a monster effect of Hierarchy 1: it doesn't touch Avatar (2).
#[test]
fn slifers_effect_does_not_touch_avatar() {
    let Some(mut s) = field(true, 8000, &[mine(SLIFER), Spawn { team: 1, place: Hand, cid: AVATAR }, theirs(WARWOLF), theirs(WARWOLF), theirs(WARWOLF)]) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run_both([&[EndTurn], &[Summon(AVATAR)]]).unwrap();
    assert_eq!(s.attack_of(1, AVATAR), s.attack_of(0, SLIFER).map(|a| a + 1), "{}", s.context());
    clean(&s);
}

/// Ring of Destruction can't destroy a God ("unaffected by Spell/Trap effects that would make this card leave the
/// field"), so no one takes damage: Obelisk, and Avatar risen to infinity by the anime Geh.
#[test]
fn ring_of_destruction_cannot_destroy_a_god() {
    for (god, extra) in [(OBELISK, None), (AVATAR, Some(mine(GEH)))] {
        let mut spawns = vec![Spawn { team: 0, place: SpellSet, cid: RING }, theirs(god)];
        spawns.extend(extra);
        let Some(mut s) = field(true, 8000, &spawns) else { return };
        s.prefer(&[(1, god)]);
        s.run_both([&[EndTurn, Activate(RING)], &[EndTurn]]).unwrap();
        assert_eq!(s.count(0, "grave", RING), 1, "Ring of Destruction resolved: {}", s.context());
        assert!(s.monster(1, god).is_some(), "{god}: {}", s.context());
        assert_eq!((s.lp(0), s.lp(1)), (8000, 8000), "{god}: {}", s.context());
        clean(&s);
    }
}

/// Ring of Destruction on Divine Serpent Geh: the printed card "cannot be targeted by card effects"; the anime one is
/// destroyed and both players take its infinite ATK at once: a draw (its "you cannot lose" ends with it).
#[test]
fn ring_of_destruction_on_geh() {
    let Some(mut s) = field(false, 8000, &[Spawn { team: 0, place: SpellSet, cid: RING }, theirs(GEH)]) else { return };
    s.prefer(&[(1, GEH)]);
    let printed = s.run_both([&[EndTurn, Activate(RING)], &[EndTurn]]);
    assert!(printed.is_err_and(|e| e.contains("never became possible")), "printed Geh can't be targeted");
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: SpellSet, cid: RING }, theirs(GEH)]) else { return };
    s.prefer(&[(1, GEH)]);
    let anime = s.run_both([&[EndTurn, Activate(RING)], &[EndTurn]]);
    assert!(anime.is_err_and(|e| e.contains("the duel is over")), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (0, 0), "a draw: {}", s.context());
    clean(&s);
}

/// Mystik Wok on Divine Serpent Geh: the printed card's ATK and DEF are 0, so Wok can't use it; the anime one gives its
/// infinite ATK as LP. Past 99,999 the client shows 99,999 (it keeps no more) and stays there after a Warwolf's 2000: the
/// engine gives the LP after each change above 99,999, where the client alone would count down from 99,999 to 97,999.
#[test]
fn mystik_wok_on_geh() {
    let Some(mut s) = field(false, 8000, &[Spawn { team: 0, place: Hand, cid: WOK }, mine(GEH)]) else { return };
    s.prefer(&[(0, GEH)]);
    assert!(s.run(&[Activate(WOK)]).is_err_and(|e| e.contains("never became possible")), "printed Geh has no ATK to give");
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: WOK }, mine(GEH), theirs(WARWOLF)]) else { return };
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), 8000 + INFINITE, "{}", s.context());
    s.run_both([&[EndTurn], &[Attack(WARWOLF)]]).unwrap();
    assert_eq!(s.lp(0), 8000 + INFINITE - 2000, "a direct attack: {}", s.context());
    clean(&s);
}

/// Three Mystik Woks on three anime Gehs: 30,007,997 LP. The opponent's anime Geh then attacks directly for "a hundred
/// times your LP", past the core's 32 bits: it used to wrap into a gain of a billion LP. Now it takes all of it.
#[test]
fn infinite_damage_past_32_bits_still_wins() {
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: WOK }, Spawn { team: 0, place: Hand, cid: WOK },
        Spawn { team: 0, place: Hand, cid: WOK }, mine(GEH), mine(GEH), mine(GEH), theirs(GEH)]) else { return };
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK), Activate(WOK), Activate(WOK)]).unwrap();
    assert!(s.lp(0) > 30_000_000, "{}", s.context());
    let r = s.run_both([&[EndTurn], &[Attack(GEH)]]);
    assert!(r.is_err_and(|e| e.contains("the duel is over")), "{}", s.context());
    assert_eq!(s.lp(0), 0, "{}", s.context());
    clean(&s);
}

/// Soul Energy Max deals 100 times the LP, not the infinite ATK: a player who gained 9,999,999 from Mystik Wok on the
/// anime Geh still loses everything.
#[test]
fn soul_energy_max_takes_even_lp_from_mystik_wok() {
    let Some(mut s) = field(true, 8000, &[mine(OBELISK), mine(WARWOLF), mine(WARWOLF), Spawn { team: 1, place: Hand, cid: WOK }, theirs(GEH)]) else { return };
    s.prefer(&[(0, WARWOLF), (1, GEH)]);
    soul_energy_max_after(&mut s, &[Activate(WOK), EndTurn]);
    assert_eq!(s.lp(1), 0, "{}", s.context());
    clean(&s);
}

/// Damage that is an infinite ATK is infinite (`crates/yo3-engine/src/guards.lua`): Avatar risen to infinity beside the
/// anime Geh attacks a player directly, and the 10,007,999 LP Mystik Wok gave them are all gone.
#[test]
fn avatar_at_infinity_takes_all_lp() {
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: WOK }, mine(GEH), theirs(AVATAR), theirs(GEH)]) else { return };
    s.prefer(&[(0, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    assert_eq!(s.lp(0), 8000 + INFINITE, "{}", s.context());
    let r = s.run_both([&[EndTurn], &[Attack(AVATAR)]]);
    assert!(r.is_err_and(|e| e.contains("the duel is over")), "{}", s.context());
    assert_eq!(s.lp(0), 0, "{}", s.context());
    clean(&s);
}

/// Ring of Destruction on the anime Geh deals its infinite ATK to both players: both lose, a draw, even with the
/// 10,007,999 LP Mystik Wok gave one of them.
#[test]
fn ring_of_destruction_on_geh_is_a_draw_whatever_the_lp() {
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: WOK }, mine(GEH), Spawn { team: 0, place: SpellSet, cid: RING }, theirs(GEH)]) else { return };
    s.prefer(&[(0, GEH), (1, GEH)]);
    s.run(&[Activate(WOK)]).unwrap();
    let r = s.run_both([&[EndTurn, Activate(RING)], &[EndTurn]]);
    assert!(r.is_err_and(|e| e.contains("the duel is over")), "{}", s.context());
    assert_eq!((s.lp(0), s.lp(1)), (0, 0), "{}", s.context());
    clean(&s);
}

/// Book of Moon turns a God face-down, but face-down it is still a God (the Hierarchy is the card's, as against Skill
/// Drain): Raigeki Break, a Spell/Trap effect that would make it leave the field, doesn't destroy it. And "other cards'
/// effects only apply to this card for 1 turn": it is face-up in Attack Position again from the end of that turn.
#[test]
fn book_of_moon_turns_a_god_face_down_for_the_turn() {
    use yo3_server::ocg::pos;
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: BOOK_OF_MOON }, Spawn { team: 0, place: SpellSet, cid: RAIGEKI_BREAK },
        theirs(OBELISK)]) else { return };
    s.prefer(&[(1, OBELISK)]);
    s.run(&[Activate(BOOK_OF_MOON)]).unwrap();
    let position = |s: &Scenario| s.monster(1, OBELISK).map(|c| c["position"].as_u64().unwrap_or(0));
    assert_eq!(position(&s), Some(pos::FACEDOWN_DEFENSE), "{}", s.context());
    s.run(&[Activate(RAIGEKI_BREAK)]).unwrap();
    assert_eq!(s.count(0, "grave", RAIGEKI_BREAK), 1, "Raigeki Break resolved: {}", s.context());
    assert_eq!(position(&s), Some(pos::FACEDOWN_DEFENSE), "face-down, still not destroyed: {}", s.context());
    s.run_both([&[EndTurn], &[]]).unwrap();
    assert_eq!(position(&s), Some(pos::FACEUP_ATTACK), "face-up again from the end of that turn: {}", s.context());
    clean(&s);
}

/// Shrink halves a God's ATK for the turn it resolves (other cards' effects apply to it for 1 turn anyway).
#[test]
fn shrink_on_a_god_lasts_the_turn() {
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: SHRINK }, theirs(OBELISK)]) else { return };
    s.prefer(&[(1, OBELISK)]);
    s.run(&[Activate(SHRINK)]).unwrap();
    assert_eq!(s.attack_of(1, OBELISK), Some(2000), "{}", s.context());
    s.run_both([&[EndTurn], &[EndTurn]]).unwrap();
    assert_eq!(s.attack_of(1, OBELISK), Some(4000), "{}", s.context());
    clean(&s);
}

/// "Control of this card cannot switch": Change of Heart and Creature Swap can't take a God, so with a God as the
/// opponent's only monster neither can be activated.
#[test]
fn a_gods_control_cannot_switch() {
    for card in [CHANGE_OF_HEART, CREATURE_SWAP] {
        let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: card }, mine(WARWOLF), theirs(OBELISK)]) else { return };
        s.prefer(&[(1, OBELISK), (0, WARWOLF)]);
        let r = s.run(&[Activate(card)]);
        assert!(r.is_err_and(|e| e.contains("never became possible")), "{card}: {}", s.context());
        assert!(s.monster(1, OBELISK).is_some(), "{}", s.context());
    }
}

/// "Your opponent cannot Tribute this card": Sphere Mode, Summoned with three of the opponent's monsters, can't use their
/// Slifer; beside three Warwolves it takes those.
#[test]
fn sphere_mode_cannot_tribute_the_opponents_god() {
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: SPHERE }, theirs(SLIFER), theirs(WARWOLF), theirs(WARWOLF)]) else { return };
    s.prefer(&[(1, SLIFER), (1, WARWOLF)]);
    assert!(s.run(&[Summon(SPHERE)]).is_err_and(|e| e.contains("never became possible")), "{}", s.context());
    let Some(mut s) = field(true, 8000, &[Spawn { team: 0, place: Hand, cid: SPHERE }, theirs(SLIFER), theirs(WARWOLF), theirs(WARWOLF), theirs(WARWOLF)]) else { return };
    s.prefer(&[(1, WARWOLF)]);
    s.run(&[Summon(SPHERE)]).unwrap();
    assert!(s.monster(1, SPHERE).is_some() && s.monster(1, SLIFER).is_some(), "{}", s.context());
    assert_eq!(s.count(1, "grave", WARWOLF), 3, "{}", s.context());
    clean(&s);
}
