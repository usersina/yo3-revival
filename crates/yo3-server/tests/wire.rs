//! `wire::Cmd`: the commands both the server and the duel check's client model read and write are packed and unpacked
//! in one place, so the two sides can't drift apart (#51's BattleAttack with no target was one such packing).
use yo3_server::wire::{Cmd, DIRECT_TARGET, NO_TARGET, OPEN, Place, Target, cmd, dialog};

fn samples() -> Vec<Cmd> {
    let place = |player, zone, index, face_up, defense| Place { player, zone, index, face_up, defense };
    vec![
        Cmd::BattleInit,
        Cmd::BattleSelect,
        Cmd::BattleEnd,
        Cmd::BattleAttack { player: 1, zone: 2, target: Target::Monster { player: 0, zone: 4 } },
        Cmd::BattleAttack { player: 0, zone: 0, target: Target::Direct { player: 1 } },
        Cmd::BattleAttack { player: 0, zone: 3, target: Target::NotYet { player: 1 } },
        Cmd::BattleRun { damage: [800, 0], flags: 2 << 8 },
        Cmd::Move { uid: 0x1234, kind: 3, from: place(0, 0xB, 2, false, false), to: place(0, 4, 0, true, true) },
        Cmd::Card { uid: 77, open: true, row: 1449, place: 1 | 0xE << 8 | 3 << 16 },
        Cmd::Card { uid: 78, open: false, row: 0, place: 0xD << 8 },
        Cmd::Dialog { kind: dialog::POSITION, message: 1, param: 77 | 1 << 16 },
    ]
}

#[test]
fn every_typed_command_decodes_to_what_was_encoded() {
    for c in samples() {
        let (id, params) = c.params();
        assert_eq!(Cmd::decode(id, params), Some(c), "{id:#x} {params:x?}");
    }
}

#[test]
fn the_packing_is_the_clients() {
    // DuelBattle (0x4F8A90): {attacker's player | zone << 8, target's player, target's zone}; a direct attack's zone 16,
    // none yet 0xFFFFFFFF. Konami's logs, before every BattleSelect: 0xC {513, 0, -1}.
    assert_eq!(Cmd::decode(cmd::BATTLE_ATTACK, [513, 0, u32::MAX, 0]), Some(Cmd::BattleAttack { player: 1, zone: 2, target: Target::NotYet { player: 0 } }));
    assert_eq!(Cmd::BattleAttack { player: 0, zone: 1, target: Target::Direct { player: 1 } }.params(), (0xC, [1 << 8, 1, DIRECT_TARGET, 0]));
    assert_eq!(Cmd::BattleAttack { player: 0, zone: 1, target: Target::NotYet { player: 1 } }.params().1[2], NO_TARGET);
    // Moves: player | zone << 1 | index << 6, face up bit 14, defense bit 15 (0x5000E0); the kind in uid's high half.
    let to = Place { player: 1, zone: 4, index: 0, face_up: true, defense: true };
    assert_eq!(Cmd::Move { uid: 5, kind: 2, from: Place { player: 1, zone: 0xB, index: 1, ..Place::default() }, to }.params(),
        (0x1A, [5 | 2 << 16, 1 | 0xB << 1 | 1 << 6, 1 | 4 << 1 | 1 << 14 | 1 << 15, 0]));
    // A card table entry: the open-pile flag in the uid's top byte (0x521DF0).
    assert_eq!(Cmd::Card { uid: 9, open: true, row: 3, place: 0x20E01 }.params(), (1000, [9 | OPEN, 3, 0, 0x20E01]));
    // Anything else is no typed command.
    assert_eq!(Cmd::decode(cmd::PHASE, [0; 4]), None);
}
