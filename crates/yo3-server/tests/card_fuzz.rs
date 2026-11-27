//! The per-card fuzz (tiered testing, docs/added-cards.md "Test it"): each chosen added card plays a few self-played duels
//! from its player's opening hand (in the Extra Deck for a Fusion or Synchro), anime versions off and, when it has one,
//! on, with the duel check's client model checked after every step as in every self-played duel. A duel that fails (the
//! client's view leaving the engine's, a script error, an engine answer slower than the self-play allows) fails the test.
//!
//! It also reports, without failing, what a scenario test should look at (`target/tmp/card-fuzz.json`, and a summary on
//! stderr): each card's effects a player activates (from its script, `duelcheck::promises`) that never activated, and
//! those that resolved and never once did something they declare (a Special Summon that never came).
//!
//! Which cards: `YO3_FUZZ_ROWS=5206-5300` (rows of engine/added/cards.tsv), or `YO3_FUZZ_CARDS=11173,5180` (the ids the
//! client knows), else every 15th added card. `YO3_FUZZ_DUELS=n` duels per card and mode (3). `task check:fuzz`.
use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value as Json, json};
use yo3_server::duelcheck::promises::{self, OBSERVABLE, Tally};
use yo3_server::duelcheck::scenario::engine_dir;
use yo3_server::duelcheck::selfplay;

/// The cards a card's text names in quotes ("Toon World", "Destiny HERO"), as (Konami id, Extra Deck): a card of that
/// name, else up to four whose names hold it (an archetype). Its fuzz duels hold them too, so its conditions can come up.
fn partners(desc: &str, names: &[(String, u16, bool)], own: u16) -> Vec<(u16, bool)> {
    let mut out: Vec<(u16, bool)> = vec![];
    for quoted in desc.split('"').skip(1).step_by(2).filter(|q| q.len() > 2) {
        let exact: Vec<(u16, bool)> = names.iter().filter(|(n, c, _)| n == quoted && *c != own).map(|(_, c, e)| (*c, *e)).take(1).collect();
        let found = if exact.is_empty() {
            names.iter().filter(|(n, c, _)| n.contains(quoted) && *c != own).map(|(_, c, e)| (*c, *e)).take(4).collect()
        } else {
            exact
        };
        for p in found {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out.truncate(8);
    out
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// The script the engine plays for `code`, as cards.rs finds it: ours first, else ProjectIgnis'.
fn script(dir: &Path, code: u64) -> String {
    let name = format!("c{code}.lua");
    std::iter::once(dir.join("overrides").join(&name))
        .chain(["official", "pre-errata", "pre-release", "unofficial"].iter().map(|f| dir.join("vendor/CardScripts").join(f).join(&name)))
        .find_map(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default()
}

#[test]
fn every_chosen_card_plays_and_its_effects_are_reported() {
    let Some(dir) = engine_dir() else { return };
    let added = yo3_data::added::Added::load(&dir).expect("engine/added-cards.json");
    let anime_map: Json = serde_json::from_str(&std::fs::read_to_string(dir.join("anime-cards.json")).unwrap()).unwrap();
    let anime_of: BTreeMap<u16, u64> = anime_map["cards"].as_array().unwrap().iter()
        .filter(|c| c["status"] == "mapped")
        .filter_map(|c| Some((c["cid"].as_u64()? as u16, c["anime"].as_u64()?)))
        .collect();
    // A card whose anime version sits in the other Deck (Five-Headed Dragon: a Fusion Monster, a Main Deck Ritual
    // Monster in the anime): with anime cards on, a partner goes where the anime version does, as the card profile
    // tells the client (#60).
    let anime_extra: BTreeMap<u16, bool> = anime_map["cards"].as_array().unwrap().iter()
        .filter(|c| c["status"] == "mapped")
        .filter_map(|c| {
            let deck = c["profile"].as_array()?.iter().filter_map(Json::as_str).find(|p| p.starts_with("deck:"))?;
            Some((c["cid"].as_u64()? as u16, deck.ends_with("-> Fusion)")))
        })
        .collect();
    let rows = std::env::var("YO3_FUZZ_ROWS").ok().and_then(|r| {
        let (a, b) = r.split_once('-')?;
        Some(a.trim().parse::<u16>().ok()?..=b.trim().parse::<u16>().ok()?)
    });
    let ids: Vec<u16> = std::env::var("YO3_FUZZ_CARDS").ok().map(|v| v.split(',').filter_map(|c| c.trim().parse().ok()).collect()).unwrap_or_default();
    let cards: Vec<_> = added.cards.iter().filter(|c| !c.token).enumerate()
        .filter(|(i, c)| if let Some(r) = &rows { r.contains(&c.row) } else if !ids.is_empty() { ids.contains(&c.cid) } else { i % 15 == 0 })
        .map(|(_, c)| c).collect();
    assert!(!cards.is_empty(), "no card chosen");
    let duels = env_u64("YO3_FUZZ_DUELS", 3);
    // The cards a player can hold, by name: the game's (when it is installed) and the added ones.
    let mut names: Vec<(String, u16, bool)> = added.cards.iter().filter(|c| !c.token).map(|c| (c.name.clone(), c.cid, c.card().extra_deck())).collect();
    let game_dir = std::env::var_os("YO3_GAME_DIR").map(std::path::PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    if game_dir.join("package/data002.ypk").is_file() {
        let game = yo3_data::GameData::load(&game_dir).expect("the client's data loads");
        names.extend(game.cards.iter().filter(|c| c.kind != 10).map(|c| (c.name.clone(), c.cid, c.extra_deck())));
    }

    let mut failures = vec![];
    let mut report = vec![];
    let mut pools = BTreeMap::new();
    for card in &cards {
        let modes: Vec<bool> = if card.anime_only { vec![true] } else if anime_of.contains_key(&card.cid) { vec![false, true] } else { vec![false] };
        let partners = partners(&card.desc, &names, card.cid);
        for anime in modes {
            let partners: Vec<(u16, bool)> = partners.iter().map(|&(p, e)| (p, if anime { anime_extra.get(&p).copied().unwrap_or(e) } else { e })).collect();
            let pool = pools.entry(anime).or_insert_with(|| selfplay::added_pool(&dir, anime, 8).expect("the added cards"));
            let code = if anime && !card.anime_only { anime_of[&card.cid] } else { card.passcode as u64 };
            let mut tallies: BTreeMap<(u16, u64), Tally> = BTreeMap::new();
            for n in 0..duels {
                let seed = 7000 + card.row as u64 * 10 + n;
                let (mut main, mut extra) = selfplay::anime_deck(pool, seed * 2);
                main.truncate(40 - 2 * partners.iter().filter(|p| !p.1).count().min(8));
                for &(p, in_extra) in &partners {
                    if in_extra { extra.extend([p; 2]) } else { main.extend([p; 2]) }
                }
                if card.card().extra_deck() {
                    extra.retain(|&c| c != card.cid);
                    extra.extend([card.cid; 2]);
                } else {
                    main.retain(|&c| c != card.cid);
                    main.extend([card.cid; 3]);
                }
                let (b_main, b_extra) = selfplay::anime_deck(pool, seed * 2 + 1);
                let decks = [(&main[..], &extra[..]), (&b_main[..], &b_extra[..])];
                match selfplay::play_card(&dir, seed, decks, anime, card.cid) {
                    Ok(played) => {
                        for ((_, description), t) in played.effects.iter().filter(|((cid, _), _)| *cid == card.cid) {
                            let mine = tallies.entry((card.cid, promises::effect_number(*description))).or_default();
                            mine.activated += t.activated;
                            mine.resolved += t.resolved;
                            mine.seen |= t.seen;
                        }
                    }
                    Err(e) => failures.push(format!("{} ({}, anime {anime}), seed {seed}: {e}", card.name, card.cid)),
                }
            }
            let declared = promises::declared(&script(&dir, code), code);
            let mut never = vec![];
            let mut unmet = vec![];
            for d in &declared {
                let t = tallies.get(&(card.cid, promises::effect_number(d.description))).cloned().unwrap_or_default();
                if t.activated == 0 {
                    never.push(promises::effect_number(d.description));
                } else if t.resolved >= 2 && d.categories & OBSERVABLE & !t.seen != 0 {
                    unmet.push(json!({"effect": promises::effect_number(d.description), "never": promises::names(d.categories & OBSERVABLE & !t.seen),
                        "resolved": t.resolved}));
                }
            }
            let activated: u32 = tallies.values().map(|t| t.activated).sum();
            report.push(json!({"row": card.row, "cid": card.cid, "name": card.name, "anime": anime, "code": code, "activated": activated,
                "effects": declared.len(), "never_activated": never, "unmet": unmet,
                "spell_trap": matches!(card.card().kind, 13 | 14), "partners": partners.iter().map(|p| p.0).collect::<Vec<_>>()}));
        }
    }
    let flagged: Vec<&Json> = report.iter().filter(|r| (r["activated"] == 0 && (r["effects"] != 0 || r["spell_trap"] == true))
        || !r["never_activated"].as_array().unwrap().is_empty()
        || !r["unmet"].as_array().unwrap().is_empty()).collect();
    for r in &flagged {
        eprintln!("{} ({}, anime {}): activated {}; effects never activated {:?}; promised, never done {}", r["name"], r["cid"], r["anime"],
            r["activated"], r["never_activated"], r["unmet"]);
    }
    eprintln!("{} card-modes fuzzed, {} flagged for a scenario test", report.len(), flagged.len());
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("card-fuzz.json");
    std::fs::write(&out, serde_json::to_string_pretty(&json!({"cards": report})).unwrap()).unwrap();
    assert!(failures.is_empty(), "{} duel(s) failed:\n{}", failures.len(), failures.join("\n\n"));
}
