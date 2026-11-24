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
