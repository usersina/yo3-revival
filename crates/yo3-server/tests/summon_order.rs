//! The summon order rules (`duelcheck/summon.rs`): each rule on the stream that broke it, and all of them against the
//! duel logs Konami's own server wrote, where they must find nothing (a rule that fires there is wrong, not the log).
//!
//! The logs: `YO3_ORIGINAL_LOGS` (a folder searched for `*.dlog`), else `re/Yu-Gi-Oh! ONLINE 3 GOLDMINE/players`
//! (kept out of git). That test is skipped without them.
use std::path::{Path, PathBuf};

use yo3_server::duelcheck::summon::SummonOrder;

/// The problems the rules find in a stream of {id, [param1..param4]}.
fn problems(stream: &[(u32, [u32; 4])]) -> Vec<String> {
    let mut order = SummonOrder::default();
    stream.iter().filter_map(|&(id, p)| order.command(id, p)).collect()
}

const MATERIAL_SET: u32 = 0x2D;
const MATERIAL_RUN: u32 = 0x2F;
const TRIBUTE_SET: u32 = 0x2A;
const TRIBUTE_RUN: u32 = 0x2C;
const TUNING_SET: u32 = 0x30;
const TUNING_RUN: u32 = 0x32;
const MOVE: u32 = 0x1A;
const RUN_FUSION: u32 = 0x3A;
const RUN_SUMMON: u32 = 0x38;
const HAND: u32 = 0xB;

/// Moves a card to its owner's Graveyard (the places don't matter to the rules).
fn to_grave(uid: u32) -> (u32, [u32; 4]) {
    (MOVE, [uid, 0x16, 0x401C, 0])
}

#[test]
fn a_fusion_from_the_hand_in_konamis_order_passes() {
    // Konami's logs: MatSet, MatSet, MatRun A, Move A, MatRun B, Move B, List, RunFusion.
    let stream = [
        (MATERIAL_SET, [0, HAND, 3, 0]),
        (MATERIAL_SET, [0, HAND, 5, 0]),
        (MATERIAL_RUN, [0, HAND, 3, 0]),
        to_grave(3),
        (MATERIAL_RUN, [0, HAND, 5, 0]),
        to_grave(5),
        (1015, [2, 3, 5, 0]),
        (RUN_FUSION, [0, 81, 2, 0]),
    ];
    assert_eq!(problems(&stream), Vec::<String>::new());
}

#[test]
fn both_runs_before_the_first_move_is_issue_40() {
    // The stream that crashed the client at 0x4FC4A9 (King of the Swamp and Masaki, live 2026-11-15).
    let stream = [
        (MATERIAL_SET, [0, HAND, 3, 0]),
        (MATERIAL_SET, [0, HAND, 5, 0]),
        (MATERIAL_RUN, [0, HAND, 3, 0]),
        (MATERIAL_RUN, [0, HAND, 5, 0]),
        to_grave(3),
        to_grave(5),
    ];
    let found = problems(&stream);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("uid 3") && found[0].contains("#40"), "{found:?}");
}

#[test]
fn two_tributes_each_run_before_their_own_move() {
    let good = [
        (TRIBUTE_SET, [0, 2, 13, 0]),
        (TRIBUTE_SET, [0, 3, 23, 0]),
        (TRIBUTE_RUN, [0, 2, 13, 0]),
        to_grave(13),
        (TRIBUTE_RUN, [0, 3, 23, 0]),
        to_grave(23),
        (RUN_SUMMON, [0, 45, 0, 0]),
    ];
    assert_eq!(problems(&good), Vec::<String>::new());
    let ahead = [(TRIBUTE_RUN, [0, 2, 13, 0]), (TRIBUTE_RUN, [0, 3, 23, 0]), to_grave(13), to_grave(23)];
    assert_eq!(problems(&ahead).len(), 1);
    // A Run whose card never moves before the summon ends.
    let stuck = [(TRIBUTE_RUN, [0, 2, 13, 0]), (RUN_SUMMON, [0, 45, 0, 0])];
    assert_eq!(problems(&stuck).len(), 1);
}

#[test]
fn a_synchro_tunes_its_tuner_first() {
    // Our translator tunes both at the first move, the Tuner first: allowed.
    let ours = [
        (TUNING_SET, [0, 2, 2, 0]),
        (TUNING_SET, [0, 1, 1, 0]),
        (TUNING_RUN, [0, 2, 2, 0x100 | 3]),
        (TUNING_RUN, [0, 1, 1, 4]),
        to_grave(1),
        to_grave(2),
        (1015, [2, 2, 1, 0]),
        (RUN_FUSION, [0, 90, 2 << 16 | 2, 0]),
    ];
    assert_eq!(problems(&ours), Vec::<String>::new());
    // The non-Tuner tuned first, then the Tuner (starting the sums again): the Synchro animation crashed (live).
    let late = [(TUNING_RUN, [0, 1, 1, 4]), (TUNING_RUN, [0, 2, 2, 0x100 | 3])];
    assert_eq!(problems(&late).len(), 2, "{:?}", problems(&late));
}

#[test]
fn an_animation_needs_its_material_list() {
    let tuned = [(TUNING_RUN, [0, 2, 2, 0x100 | 3]), (TUNING_RUN, [0, 1, 1, 4])];
    let mut none = tuned.to_vec();
    none.push((RUN_FUSION, [0, 90, 2 << 16 | 2, 0]));
    assert!(problems(&none).iter().any(|p| p.contains("without a material list")), "{:?}", problems(&none));
    let mut short = tuned.to_vec();
    short.extend([(1015, [1, 2, 0, 0]), (RUN_FUSION, [0, 90, 2 << 16 | 2, 0])]);
    assert!(problems(&short).iter().any(|p| p.contains("names 2 materials")), "{:?}", problems(&short));
    // Four materials: 1015 gives three, 1016 the fourth.
    let four = [(1015, [4, 1, 2, 3]), (1016, [4, 0, 0, 0]), (RUN_FUSION, [0, 90, 4, 0])];
    assert_eq!(problems(&four), Vec::<String>::new());
    // A Synchro animation with no Tuner.
    let untuned = [(1015, [2, 1, 2, 0]), (RUN_FUSION, [0, 90, 2 << 16 | 2, 0])];
    assert!(problems(&untuned).iter().any(|p| p.contains("no Tuner")), "{:?}", problems(&untuned));
}

fn logs_dir() -> PathBuf {
    std::env::var_os("YO3_ORIGINAL_LOGS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/Yu-Gi-Oh! ONLINE 3 GOLDMINE/players"))
}

fn dlogs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            dlogs(&path, out);
        } else if path.extension().is_some_and(|e| e == "dlog") {
            out.push(path);
        }
    }
}

/// A log's command stream {id, [param1..param4]} (crates/yo3-server/src/duellog.rs has the layout); None for a file
/// that isn't a duel log.
fn stream_of(bytes: &[u8]) -> Option<Vec<(u32, [u32; 4])>> {
    if bytes.len() < 360 || bytes[..2] != [0xE7, 0x09] {
        return None;
    }
    let body: Vec<u8> = bytes[360..].as_chunks::<2>().0.iter().flat_map(|p| std::iter::repeat_n(p[0], p[1] as usize)).collect();
    let u32_at = |at: usize| body.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let start = 80 + 2 * 0x21D0;
    let count = u32_at(start)? as usize;
    (0..count)
        .map(|i| {
            let at = start + 4 + 24 * i;
            Some((u32_at(at)?, [u32_at(at + 8)?, u32_at(at + 12)?, u32_at(at + 16)?, u32_at(at + 20)?]))
        })
        .collect()
}

#[test]
fn konamis_own_duel_logs_keep_every_rule() {
    let mut files = vec![];
    dlogs(&logs_dir(), &mut files);
    if files.is_empty() {
        eprintln!("no original duel logs; skipping");
        return;
    }
    let (mut duels, mut summons, mut found) = (0, 0, vec![]);
    for path in &files {
        let Some(stream) = stream_of(&std::fs::read(path).unwrap()) else { continue };
        duels += 1;
        summons += stream.iter().filter(|(id, _)| matches!(*id, RUN_FUSION | RUN_SUMMON | 0x39)).count();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        found.extend(problems(&stream).into_iter().map(|p| format!("{name}: {p}")));
    }
    assert!(duels >= 200, "only {duels} duel logs read");
    assert!(found.is_empty(), "{} problem(s) in Konami's own logs ({duels} duels, {summons} summons):\n{}", found.len(), found.join("\n"));
}
