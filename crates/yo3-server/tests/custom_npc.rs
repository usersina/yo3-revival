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

/// The last custom slot (42, `CUSTOM_NPC_SLOTS`): its three levels are ids 246-248 on the tab it names, as the client's
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
    assert!(world.npcs.ends_with(&[246, 247, 248]), "{:?}", world.npcs);
    let mut c = common::Server::with_world(world).join(1001, "Yugi");
    let roster = c.send(common::req(0x2710)).remove(0);
    let rafael: Vec<_> = roster.items.iter().filter(|i| matches!(i.get("npc_id").and_then(Value::as_u32), Some(246..=248))).collect();
    assert_eq!(rafael.len(), 3);
    assert!(rafael.iter().all(|i| i.get("npc_type").and_then(Value::as_u32) == Some(2)));
}
