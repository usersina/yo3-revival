//! A smoke test for every card the game never had (#75, `engine/added-cards.json`), new ones included without a line of
//! test code, in two tiers so it keeps up as the cards grow:
//!
//! 1. Every card's script loads and registers its effects: one duel per mode starts with every added card in a Deck
//!    (`every_added_card_loads_its_script`, seconds).
//! 2. Two turns played each way: each card starts on the field (a monster face-up, a Spell or Trap Set) with a copy in the
//!    hand, through the real core, both seats' client models checked after every step. A script that errors in the
//!    engine, or a card the client's view can't follow, fails here before anyone writes its effect scenarios
//!    (`added_cards.rs`, `added_batch.rs`). Which cards (`YO3_SMOKE`):
//!    - unset (`cargo nextest run`, CI's tests): today's rotating tenth (row % 10 == days since 1970 % 10;
//!      `YO3_SMOKE_SLICE=n` picks another), so every card plays within ten days;
//!    - `changed` (`task check:smoke`): that tenth and the cards changed since the newest release tag, the working tree
//!      included: new or changed rows of `engine/added/cards.tsv`, their own script overrides, and the cards calling a
//!      function a changed shared procedure in `engine/overrides/` defines;
//!    - `all`: every card, as `task check:added` and the release job run it in the release build;
//!    - `ROWS` (`5217-5258,5300`): those rows.
//!
//! Anime-only cards play with the anime versions on, as they only ever do; the printed ones play both ways (with anime
//! cards on, the ones with an anime version play that).
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::process::Command;
use yo3_data::added::AddedCard;
use yo3_server::duelcheck::scenario::{Action::*, Scenario, engine_dir};
use yo3_server::engine::{Spawn, SpawnPlace};

const KURIBOH: u16 = 4064;
const WARWOLF: u16 = 6960; // Gene-Warped Warwolf: an opponent's monster to battle and target

/// Tier 1: with anime cards off, every printed added card in team 0's Decks; with them on, every one. Creating a card runs
/// its script's `initial_effect`, so a script that doesn't load or errors as it registers fails the duel's start.
/// Tokens are left out: their cards' effects make them (`tests/added_tokens.rs`), no Deck holds one.
#[test]
fn every_added_card_loads_its_script() {
    let Some(dir) = engine_dir() else { return };
    let added = yo3_data::added::Added::load(&dir).expect("engine/added-cards.json");
    for anime in [false, true] {
        let cards: Vec<&AddedCard> = added.cards.iter().filter(|c| !c.token && (anime || !c.anime_only)).collect();
        let (extra, main): (Vec<&AddedCard>, Vec<&AddedCard>) = cards.iter().partition(|c| c.card().extra_deck());
        let main: Vec<u16> = main.iter().map(|c| c.cid).collect();
        let extra: Vec<u16> = extra.iter().map(|c| c.cid).collect();
        let decks = [(&main[..], &extra[..]), (&[KURIBOH; 40][..], &[][..])];
        let started = if anime { Scenario::start_anime(&dir, 3, 8000, decks) } else { Scenario::start(&dir, 3, 8000, decks) };
        let s = started.unwrap_or_else(|e| panic!("{} cards (anime cards {}): the duel didn't start: {e}", cards.len(), if anime { "on" } else { "off" }));
        assert!(s.problems.is_empty(), "the client's view left the engine's: {:?}", s.problems);
    }
}

/// Tier 2: the chosen cards (`YO3_SMOKE`, the module's comment) start on the field and play two turns each way.
#[test]
fn the_chosen_added_cards_start_on_the_field_and_play_two_turns_without_an_error() {
    let Some(dir) = engine_dir() else { return };
    let added = yo3_data::added::Added::load(&dir).expect("engine/added-cards.json");
    let cards: Vec<&AddedCard> = added.cards.iter().filter(|c| !c.token).collect();
    let chosen = chosen(&dir, &cards);
    let cards: Vec<&AddedCard> = cards.into_iter().filter(|c| chosen.contains(&c.row)).collect();
    eprintln!("smoke: {} added card(s) play two turns", cards.len());
    let mut failures = vec![];
    // Each card as printed (anime cards off), and each printed card again with anime cards on, when it may play its anime
    // version (engine/anime-cards.json); an anime-only card only ever plays with them on.
    let runs = cards.iter().map(|&card| (card, card.anime_only)).chain(cards.iter().filter(|c| !c.anime_only).map(|&card| (card, true)));
    for (card, anime) in runs {
        let c = card.card();
        let place = match c.kind {
            13 | 14 => SpawnPlace::SpellSet, // a Spell or Trap: Set, as a player would put it down
            _ => SpawnPlace::Monster,
        };
        let extra = c.extra_deck();
        let field = [Spawn { team: 0, place, cid: card.cid }, Spawn { team: 0, place: SpawnPlace::Hand, cid: card.cid },
            Spawn { team: 1, place: SpawnPlace::Monster, cid: WARWOLF }];
        let started = if anime { Scenario::start_anime_field(&dir, 3, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field) }
            else { Scenario::start_field(&dir, 3, 8000, [(&[KURIBOH; 40], &[]), (&[KURIBOH; 40], &[])], &field) };
        let what = format!("{} (row {}, id {}{}{})", card.name, card.row, card.cid, if extra { ", Extra Deck" } else { "" },
            if anime && !card.anime_only { ", anime cards on" } else { "" });
        let mut s = match started {
            Ok(s) => s,
            Err(e) => { failures.push(format!("{what}: the duel didn't start: {e}")); continue }
        };
        if let Err(e) = s.run_both([&[EndTurn, EndTurn], &[EndTurn, EndTurn]]) {
            // A duel an added card ends early (an LP or deck effect) is no error of its own.
            if !e.contains("the duel is over") {
                failures.push(format!("{what}: {e}"));
                continue;
            }
        }
        if !s.problems.is_empty() {
            failures.push(format!("{what}: the client's view left the engine's: {:?}", s.problems));
        }
    }
    assert!(failures.is_empty(), "{} added card(s) failed:\n{}", failures.len(), failures.join("\n\n"));
}

/// The rows tier 2 plays, by `YO3_SMOKE`.
fn chosen(dir: &Path, cards: &[&AddedCard]) -> BTreeSet<u16> {
    let all = || cards.iter().map(|c| c.row).collect();
    let mode = std::env::var("YO3_SMOKE").unwrap_or_default();
    let with_changed = match mode.trim() {
        "all" => return all(),
        "changed" => true,
        "" => false,
        rows => return parse_rows(rows),
    };
    let slice = std::env::var("YO3_SMOKE_SLICE").ok().and_then(|s| s.parse::<u16>().ok()).unwrap_or_else(|| {
        let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() / 86_400);
        (days % 10) as u16
    }) % 10;
    let mut rows: BTreeSet<u16> = cards.iter().map(|c| c.row).filter(|r| r % 10 == slice).collect();
    let sample = rows.len();
    if !with_changed {
        eprintln!("smoke: today's tenth, {sample} card(s) (rows ending in {slice}); YO3_SMOKE=changed or all for more");
        return rows;
    }
    match changed(dir, cards) {
        Ok((base, changed)) => {
            eprintln!("smoke: {} card(s) changed since {base}, {sample} in today's tenth (rows ending in {slice})", changed.len());
            rows.extend(changed);
        }
        Err(e) => eprintln!("smoke: the changed cards are unknown ({e}): {sample} in today's tenth (rows ending in {slice})"),
    }
    rows
}

/// `5217-5258,5300` as rows.
fn parse_rows(spec: &str) -> BTreeSet<u16> {
    let mut rows = BTreeSet::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let bad = || panic!("YO3_SMOKE: {part:?} is not a row or a range of rows (all, 5217-5258,5300)");
        match part.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (a.trim().parse::<u16>().unwrap_or_else(|_| bad()), b.trim().parse::<u16>().unwrap_or_else(|_| bad()));
                rows.extend(a..=b);
            }
            None => { rows.insert(part.parse().unwrap_or_else(|_| bad())); }
        }
    }
    rows
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The rows changed since the newest release tag, the working tree included: (the tag, the rows).
fn changed(dir: &Path, cards: &[&AddedCard]) -> Result<(String, BTreeSet<u16>), String> {
    let base = git(dir, &["describe", "--tags", "--abbrev=0", "--match", "v*", "HEAD"])?.trim().to_string();
    let files = git(dir, &["diff", "--name-only", &base, "--", "."])?;
    let old = git(dir, &["show", &format!("{base}:./added/cards.tsv")]).unwrap_or_default();
    let new = std::fs::read_to_string(dir.join("added/cards.tsv")).map_err(|e| format!("added/cards.tsv: {e}"))?;
    let new_files = git(dir, &["ls-files", "--others", "--exclude-standard", "--", "overrides"])?;
    let files: Vec<&str> = files.lines().chain(new_files.lines()).collect();
    let mut rows = BTreeSet::new();
    // New or changed rows of the table, by column name (a column added to the table changes no row).
    rows.extend(changed_rows(&table(&old), &table(&new)));
    let by_passcode: HashMap<u32, u16> = cards.iter().map(|c| (c.passcode, c.row)).collect();
    let mut functions: Vec<String> = vec![];
    for file in &files {
        let Some(name) = Path::new(file).file_name().and_then(|n| n.to_str()) else { continue };
        if !file.contains("overrides/") || !name.ends_with(".lua") {
            continue;
        }
        if let Some(row) = name.strip_prefix('c').and_then(|n| n.strip_suffix(".lua")).and_then(|n| n.parse::<u32>().ok()).and_then(|p| by_passcode.get(&p)) {
            rows.insert(*row);
        } else if !name.starts_with('c') {
            // A shared procedure: the functions it defines, which the cards' scripts call.
            let text = std::fs::read_to_string(dir.join("overrides").join(name)).unwrap_or_default();
            functions.extend(defined_functions(&text));
        }
    }
    if !functions.is_empty() {
        let scripts = scripts(dir);
        for c in cards {
            let Some(path) = scripts.get(&c.passcode) else { continue };
            let text = std::fs::read_to_string(path).unwrap_or_default();
            if functions.iter().any(|f| text.contains(f.as_str())) {
                rows.insert(c.row);
            }
        }
    }
    Ok((base, rows))
}

type Table = HashMap<u16, HashMap<String, String>>;

/// The rows of `new` that `old` lacks or holds with another value in a column (a column added to the table, empty,
/// changes no row).
fn changed_rows(old: &Table, new: &Table) -> BTreeSet<u16> {
    let differs = |was: &HashMap<String, String>, now: &HashMap<String, String>| {
        now.iter().any(|(k, v)| was.get(k).map_or(!v.is_empty(), |w| w != v)) || was.iter().any(|(k, w)| !w.is_empty() && !now.contains_key(k))
    };
    new.iter().filter(|(row, now)| old.get(row).is_none_or(|was| differs(was, now))).map(|(row, _)| *row).collect()
}

/// `engine/added/cards.tsv` as row → (column → value).
fn table(text: &str) -> Table {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
    lines.filter_map(|l| {
        let fields: Vec<&str> = l.split('\t').collect();
        let row = fields.first()?.parse().ok()?;
        Some((row, header.iter().zip(fields.iter().chain(std::iter::repeat(&""))).map(|(k, v)| (k.to_string(), v.to_string())).collect()))
    }).collect()
}

/// `function Auxiliary.AddEquipProcedure(` → `aux.AddEquipProcedure` and `Auxiliary.AddEquipProcedure` (scripts call
/// both), `function Synchro.AddProcedure(` → `Synchro.AddProcedure`; local and plain globals are left out.
fn defined_functions(text: &str) -> Vec<String> {
    let mut out = vec![];
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("function ") else { continue };
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.' || *c == ':').collect();
        let Some((table, function)) = name.split_once(['.', ':']) else { continue };
        if table == "Auxiliary" || table == "aux" {
            out.push(format!("aux.{function}"));
            out.push(format!("Auxiliary.{function}"));
        } else {
            out.push(name.clone());
        }
    }
    out
}

/// Each script's file by passcode: ours (`overrides/`) before ProjectIgnis' (`vendor/CardScripts/*/`), as the engine loads
/// them.
fn scripts(dir: &Path) -> HashMap<u32, std::path::PathBuf> {
    let mut out = HashMap::new();
    let mut folders = vec![dir.join("overrides")];
    if let Ok(entries) = std::fs::read_dir(dir.join("vendor/CardScripts")) {
        folders.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    for folder in folders {
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        for path in entries.flatten().map(|e| e.path()) {
            let passcode = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_prefix('c')).and_then(|n| n.strip_suffix(".lua"))
                .and_then(|n| n.parse::<u32>().ok());
            if let Some(p) = passcode {
                out.entry(p).or_insert(path);
            }
        }
    }
    out
}

#[test]
fn smoke_rows_and_shared_functions_parse() {
    assert_eq!(parse_rows("5217-5219, 5300"), BTreeSet::from([5217, 5218, 5219, 5300]));
    let f = defined_functions("function Auxiliary.AddEquipProcedure(c)\nlocal function helper()\nfunction Synchro.AddProcedure(c,...)\n");
    assert_eq!(f, ["aux.AddEquipProcedure", "Auxiliary.AddEquipProcedure", "Synchro.AddProcedure"]);
    // A column added changes no row; a changed or new row does.
    let old = table("row\tname\n1\tA\n2\tB\n");
    let new = table("row\ttoken\tname\n1\t\tA\n2\t\tC\n3\t\tD\n");
    assert_eq!(changed_rows(&old, &new), BTreeSet::from([2, 3]));
}
