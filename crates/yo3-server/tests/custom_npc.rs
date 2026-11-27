//! Custom NPCs (`engine/added/npcs.json`, carried in the added cards): Dartz in Kaiba Corporation's slot 25 (ids 195-197),
//! on the Yu-Gi-Oh! tab, with a printed deck per level and an anime deck while the server has anime cards on.
mod common;
use std::sync::Arc;
use yo3_proto::Value;
use yo3_server::front::World;

#[test]
fn dartz_joins_the_roster_with_printed_and_anime_decks() {
    let game = std::env::var_os("YO3_GAME_DIR").map(std::path::PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !game.join("package/data002.ypk").is_file() || !engine.join("added-cards.json").is_file() { return }
    let added = yo3_data::added::Added::load(&engine).unwrap();
    let mut data = yo3_data::GameData::load(&game).unwrap();
    data.cards.add(&added).unwrap();
    let mut world = World { added: Arc::new(added), ..World::new(vec![]) };
    yo3_server::add_custom_npcs(&mut world, &data);
    assert!(world.npcs.ends_with(&[195, 196, 197]), "{:?}", world.npcs);
    assert_eq!(world.npc_names.get(&195).map(String::as_str), Some("Dartz"));
    for id in [195, 196, 197] {
        assert_eq!(world.npc_decks[&id].main.len(), 40, "every card of level {id}'s deck is one of the server's");
        assert_eq!(world.npc_anime_decks[&id].main.len(), 40);
    }
    let row = |name: &str| data.cards.iter().find(|c| c.name == name).unwrap().row;
    let gigas = row("Orichalcos Gigas");
    world.settings.set("anime-cards", "off").unwrap();
    assert!(!world.npc_deck(195).unwrap().main.contains(&gigas), "printed deck with anime cards off");
    world.settings.set("anime-cards", "on").unwrap();
    assert!(world.npc_deck(195).unwrap().main.contains(&gigas), "anime deck with them on");
    // The roster (0x2711): Dartz's three levels on tab 0.
    let mut c = common::Server::with_world(world).join(1001, "Yugi");
    let roster = c.send(common::req(0x2710)).remove(0);
    let dartz: Vec<_> = roster.items.iter().filter(|i| matches!(i.get("npc_id").and_then(Value::as_u32), Some(195..=197))).collect();
    assert_eq!(dartz.len(), 3);
    assert!(dartz.iter().all(|i| i.get("npc_type").and_then(Value::as_u32) == Some(0)));
}

/// The last custom slot (99, `CUSTOM_NPC_SLOTS`): its three levels are ids 417-419 on the tab it names, as the client's
/// list takes any character on any of its three tabs (0x4BFBA0); a slot past it is refused when the added cards load.
#[test]
fn the_last_custom_slot_joins_the_roster_on_its_tab() {
    let game = std::env::var_os("YO3_GAME_DIR").map(std::path::PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !game.join("package/data002.ypk").is_file() || !engine.join("added-cards.json").is_file() { return }
    let mut added = yo3_data::added::Added::load(&engine).unwrap();
    let dartz = added.npcs.iter().find(|n| n.name == "Dartz").unwrap().clone();
    let last = *yo3_data::added::CUSTOM_NPC_SLOTS.end();
    added.npcs.push(yo3_data::added::AddedNpc { slot: last, name: "Rafael".into(), series: 2, body: 9, figure: None, ..dartz.clone() });
    assert!(yo3_data::added::check_npcs(&added.npcs).is_ok());
    assert!(yo3_data::added::check_npcs(&[yo3_data::added::AddedNpc { slot: last + 1, ..dartz }]).is_err());
    let mut data = yo3_data::GameData::load(&game).unwrap();
    data.cards.add(&added).unwrap();
    let mut world = World { added: Arc::new(added), ..World::new(vec![]) };
    yo3_server::add_custom_npcs(&mut world, &data);
    let ids = [120 + 3 * last, 121 + 3 * last, 122 + 3 * last];
    assert_eq!(ids, [417, 418, 419]);
    assert!(world.npcs.ends_with(&ids), "{:?}", world.npcs);
    let mut c = common::Server::with_world(world).join(1001, "Yugi");
    let roster = c.send(common::req(0x2710)).remove(0);
    let rafael: Vec<_> = roster.items.iter().filter(|i| i.get("npc_id").and_then(Value::as_u32).is_some_and(|id| ids.contains(&id))).collect();
    assert_eq!(rafael.len(), 3);
    assert!(rafael.iter().all(|i| i.get("npc_type").and_then(Value::as_u32) == Some(2)));
}

/// Every custom NPC's decks, as the server builds them from `engine/added/npcs.json`: each card name is one the server
/// knows (none left out), and each level's deck passes the server's deck check (`deckcheck::check`: sizes, Extra Deck
/// placement, the login banlist, cards the engine has) with anime cards off (the printed deck, no anime-only card) and
/// on (the anime deck); and each has a bot style.
#[test]
fn every_custom_npcs_decks_load_and_pass_the_deck_check_in_both_modes() {
    let game = std::env::var_os("YO3_GAME_DIR").map(std::path::PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !game.join("package/data002.ypk").is_file() || !engine.join("vendor/cardinfo.json").is_file() { return }
    // nextest runs each test in its own process
    unsafe { std::env::set_var("YO3_ENGINE_DIR", &engine) };
    let root = std::env::temp_dir().join(format!("yo3-custom-npc-decks-{}", std::process::id()));
    let mut cfg = yo3_server::Config::ephemeral(root.clone());
    cfg.game_dir = game;
    let mut world = yo3_server::game_world(&cfg).unwrap();
    let npcs = world.added.npcs.clone();
    assert!(!npcs.is_empty());
    let mut problems = vec![];
    for npc in &npcs {
        if yo3_server::npc::style(&npc.name).is_null() {
            problems.push(format!("{}: no bot style (npc.rs CHARACTERS)", npc.name));
        }
        for (id, level) in npc.ids().into_iter().zip(&npc.levels) {
            let printed = &world.npc_decks[&id];
            if printed.main.len() != level.deck.len() || printed.extra.len() != level.extra.len() {
                problems.push(format!("{} level {id}: a printed card name the server doesn't know", npc.name));
            }
            if let Some(anime) = world.npc_anime_decks.get(&id)
                && (anime.main.len() != level.anime_deck.len() || anime.extra.len() != level.anime_extra.len()) {
                problems.push(format!("{} level {id}: an anime card name the server doesn't know", npc.name));
            }
            for w in &level.anime_waiting {
                if !level.anime_deck.contains(&w.instead) && !level.anime_extra.contains(&w.instead) {
                    problems.push(format!("{} level {id}: {} waits in place of {}, which the anime deck doesn't have", npc.name, w.card, w.instead));
                }
            }
            for anime in ["off", "on"] {
                world.settings.set("anime-cards", anime).unwrap();
                let deck = world.npc_deck(id).unwrap().clone();
                let mut owned = std::collections::BTreeMap::new();
                for row in deck.cards() { *owned.entry(row).or_insert(0) += 1 }
                let regulation = world.settings.now().login_regulation;
                if let Err(refused) = yo3_server::deckcheck::check(&world, regulation, &owned, &deck) {
                    let why = yo3_server::deckcheck::explain(&world, regulation, &owned, &deck);
                    problems.push(format!("{} level {id}, anime cards {anime}: {refused:?} {why:?}", npc.name));
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
