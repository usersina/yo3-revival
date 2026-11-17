//! `CLIENT_COMMANDS` (src/client_commands.rs) against its two sources: the client's own dispatch tables, decoded from
//! `yo3.exe` (skipped without the game), and the duel logs Konami's server wrote (`YO3_ORIGINAL_LOGS`, else
//! `re/Yu-Gi-Oh! ONLINE 3 GOLDMINE/players`; skipped without them). A command the client handles or Konami sent that the
//! table lacks fails here, as does a handler or count that changed.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use yo3_server::client_commands::{CLIENT_COMMANDS, Use};

#[test]
fn the_table_is_ordered_once_per_id_and_every_reason_says_something() {
    let ids: Vec<u32> = CLIENT_COMMANDS.iter().map(|c| c.id).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids, sorted, "ordered by id, each once");
    for c in CLIENT_COMMANDS {
        match c.usage {
            Use::NotSent(why) | Use::Undecided(why) => assert!(why.len() > 20, "{:#x}: a reason with its evidence", c.id),
            Use::NoHandler => assert!(c.handlers.is_empty(), "{:#x}: no handler, yet {:?}", c.id, c.handlers),
            Use::Sent(_) => assert!(!c.handlers.is_empty(), "{:#x}: sent, but nothing in the client handles it", c.id),
        }
    }
}

/// The game's executable mapped by its sections (image base 0x400000), to read the dispatch tables at their addresses.
struct Image {
    bytes: Vec<u8>,
    sections: Vec<(u32, u32, u32)>,
}

impl Image {
    fn open(path: &Path) -> Option<Image> {
        let bytes = std::fs::read(path).ok()?;
        let u16_at = |at: usize| u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) as usize;
        let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let pe = u32_at(0x3C) as usize;
        let (count, optional) = (u16_at(pe + 6), u16_at(pe + 20));
        let table = pe + 24 + optional;
        let sections = (0..count).map(|i| table + 40 * i).map(|s| (u32_at(s + 12), u32_at(s + 16).max(u32_at(s + 8)), u32_at(s + 20))).collect();
        Some(Image { bytes, sections })
    }

    fn at(&self, va: u32) -> usize {
        let rva = va - 0x40_0000;
        let &(start, _, raw) = self.sections.iter().find(|(start, size, _)| (*start..start + size).contains(&rva)).expect("an address in a section");
        (raw + rva - start) as usize
    }

    fn u8(&self, va: u32) -> u32 {
        self.bytes[self.at(va)] as u32
    }

    fn u32(&self, va: u32) -> u32 {
        let at = self.at(va);
        u32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap())
    }

    /// The ids a switch sends anywhere but its default: `ids` from `first`, through an index table when there is one.
    fn cases(&self, first: u32, count: u32, index: Option<u32>, jumps: u32, default: u32) -> Vec<u32> {
        (0..count)
            .filter(|&i| {
                let slot = index.map_or(i, |table| self.u8(table + i));
                self.u32(jumps + 4 * slot) != default
            })
            .map(|i| first + i)
            .collect()
    }
}

#[test]
fn the_handlers_are_the_clients_own_dispatch_tables() {
    let exe = yo3_data::default_game_dir().join("yo3.exe");
    let Some(image) = Image::open(&exe) else {
        eprintln!("skipped: no game at {}", exe.display());
        return;
    };
    // Each component's switch (docs/protocol.md, "Components"): first id, count, index table, jump table, default.
    let mut handled: BTreeMap<u32, BTreeSet<&str>> = BTreeMap::new();
    let mut add = |component: &'static str, ids: Vec<u32>| {
        for id in ids {
            handled.entry(id).or_default().insert(component);
        }
    };
    add("DuelScene", image.cases(1, 0x3F, Some(0x515294), 0x515240, 0x515229));
    add("DuelScene", vec![1505]); // 0x5142CA: `je` on 0x5E1
    add("DuelCardManager", image.cases(0x12, 0x18, None, 0x50007C, 0x4FFEFE));
    add("DuelSummon", image.cases(0x2A, 9, Some(0x516920), 0x516914, 0x51690A));
    add("DuelCutin", image.cases(6, 0x48, Some(0x505E44), 0x505E20, 0x505E18));
    add("DuelBattle", image.cases(4, 11, None, 0x4F8C9C, 0x4F8C90));
    // The pop: ids 2-0x37 through an index table, 1001-1027 through a jump table, and 1000, 1500, 1504, 1506 by
    // comparisons (0x522B1D, 0x522B46, 0x522FBC).
    add("the pop", image.cases(2, 0x36, Some(0x5232BC), 0x5232A4, 0x523080));
    add("the pop", image.cases(1001, 0x1B, None, 0x523238, 0x523080));
    add("the pop", vec![1000, 1500, 1504, 1506]);
    // Chains of comparisons, read once: DuelCockpit 0x502EB0 (5, 0xF, 0x10), DuelFieldManager 0x50A690 (4, 0x26),
    // DuelChain 0x501560 (0x33, 0x34), and the rock-paper-scissors scene (1501-1503, docs/protocol.md "Pre-duel").
    add("DuelCockpit", vec![5, 0xF, 0x10]);
    add("DuelFieldManager", vec![4, 0x26]);
    add("DuelChain", vec![0x33, 0x34]);
    add("the rock-paper-scissors scene", vec![1501, 1502, 1503]);

    let mut wrong = vec![];
    for (id, components) in &handled {
        match CLIENT_COMMANDS.iter().find(|c| c.id == *id) {
            None => wrong.push(format!("{id:#x}: handled by {components:?}, not in the table")),
            Some(c) if c.handlers.iter().copied().collect::<BTreeSet<_>>() != *components => {
                wrong.push(format!("{id:#x}: the table says {:?}, the client {components:?}", c.handlers));
            }
            Some(_) => {}
        }
    }
    for c in CLIENT_COMMANDS.iter().filter(|c| !c.handlers.is_empty() && !handled.contains_key(&c.id)) {
        wrong.push(format!("{:#x}: the table says {:?}, but no dispatch table has it", c.id, c.handlers));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn logs_dir() -> PathBuf {
    std::env::var_os("YO3_ORIGINAL_LOGS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/Yu-Gi-Oh! ONLINE 3 GOLDMINE/players"))
}

fn dlogs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            dlogs(&path, out);
        } else if path.extension().is_some_and(|e| e == "dlog") {
            out.push(path);
        }
    }
}

/// A log's command ids (crates/yo3-server/src/duellog.rs has the layout; tests/summon_order.rs reads the same).
fn ids_of(bytes: &[u8]) -> Option<Vec<u32>> {
    if bytes.len() < 360 || bytes[..2] != [0xE7, 0x09] {
        return None;
    }
    let body: Vec<u8> = bytes[360..].as_chunks::<2>().0.iter().flat_map(|p| std::iter::repeat_n(p[0], p[1] as usize)).collect();
    let u32_at = |at: usize| body.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let start = 80 + 2 * 0x21D0;
    (0..u32_at(start)? as usize).map(|i| u32_at(start + 4 + 24 * i)).collect()
}

#[test]
fn konamis_commands_are_all_in_the_table_as_often_as_it_says() {
    let mut files = vec![];
    dlogs(&logs_dir(), &mut files);
    if files.is_empty() {
        eprintln!("no original duel logs; skipping");
        return;
    }
    let mut counts: BTreeMap<u32, u32> = BTreeMap::new();
    let mut duels = 0;
    for path in &files {
        let Some(ids) = ids_of(&std::fs::read(path).unwrap()) else { continue };
        duels += 1;
        for id in ids {
            *counts.entry(id).or_default() += 1;
        }
    }
    assert!(duels >= 200, "only {duels} duel logs read");
    let mut wrong = vec![];
    for (id, n) in &counts {
        match CLIENT_COMMANDS.iter().find(|c| c.id == *id) {
            None => wrong.push(format!("{id:#x}: Konami sent it {n} times, not in the table")),
            Some(c) if c.konami != *n => wrong.push(format!("{id:#x}: Konami sent it {n} times, the table says {}", c.konami)),
            Some(_) => {}
        }
    }
    for c in CLIENT_COMMANDS.iter().filter(|c| c.konami > 0 && !counts.contains_key(&c.id)) {
        wrong.push(format!("{:#x}: the table says Konami sent it {} times, the logs never", c.id, c.konami));
    }
    assert!(wrong.is_empty(), "{duels} duels:\n{}", wrong.join("\n"));
}
