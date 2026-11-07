//! Golden tests against the user's own Yu-Gi-Oh! ONLINE 3 install (YO3_GAME_DIR, or the
//! default Wine path). They are skipped when the game is not installed.
use std::path::PathBuf;

use yo3_data::{GameData, Ypk, default_game_dir};
use yo3_proto::Message;

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    dir.join("package/data002.ypk").is_file().then_some(dir)
}

macro_rules! need_game {
    () => {
        match game_dir() {
            Some(d) => d,
            None => {
                eprintln!("game not installed; skipping");
                return;
            }
        }
    };
}

#[test]
fn ypk_lists_and_reads_entries() {
    let dir = need_game!();
    let ypk = Ypk::open(dir.join("package/data002.ypk")).unwrap();
    let prop = ypk.entries().iter().find(|e| e.name == "data/bin#/CARD_Prop.bin").expect("CARD_Prop listed");
    assert_eq!(prop.size, 38672);
    assert_eq!(ypk.read("data/bin#/CARD_Prop.bin").unwrap().unwrap().len(), 38672);
    assert!(ypk.read("data/does/not/exist").unwrap().is_none());
}

#[test]
fn every_data_file_round_trips_byte_for_byte() {
    let dir = need_game!();
    let ypk = Ypk::open(dir.join("package/data002.ypk")).unwrap();
    let mut checked = 0;
    for e in ypk.entries().iter().filter(|e| e.name.ends_with(".y3d") || e.name.ends_with(".y3a") || e.name.ends_with(".y3r")) {
        let blob = ypk.read(&e.name).unwrap().unwrap();
        let n = yo3_proto::frame_len(blob[..4].try_into().unwrap());
        let plain = yo3_proto::decrypt(&blob[4..4 + n]).unwrap();
        let (msg, used) = Message::decode(&plain).unwrap();
        assert_eq!(&plain[..used], &msg.encode()[..], "{}", e.name);
        assert!(plain[used..].iter().all(|&b| b == 0), "{}: trailing bytes", e.name);
        checked += 1;
    }
    assert_eq!(checked, 163 + 82 + 20);
}

#[test]
fn card_database_uses_konami_card_ids() {
    let dir = need_game!();
    let data = GameData::load(&dir).unwrap();
    assert_eq!(data.cards.len(), 4824);
    let bewd = data.cards.by_cid(4007).unwrap();
    assert_eq!((bewd.name.as_str(), bewd.atk, bewd.def), ("Blue-Eyes White Dragon", 3000, 2500));
    let dm = data.cards.by_cid(4041).unwrap();
    assert_eq!((dm.name.as_str(), dm.atk, dm.def), ("Dark Magician", 2500, 2100));
    assert_eq!(data.cards.by_cid(3901).unwrap().name, "Kuriboh Token");
    assert_eq!(data.cards.by_row(bewd.row).unwrap().cid, 4007);
    // Level, attribute (1 LIGHT, 2 DARK) and race (1 Dragon, 2 Spellcaster in Konami's order).
    assert_eq!((bewd.level, bewd.attribute, bewd.race), (8, 1, 1));
    assert_eq!((dm.level, dm.attribute), (7, 2));
}

#[test]
fn regulations_carry_the_banlists() {
    let dir = need_game!();
    let data = GameData::load(&dir).unwrap();
    let ids: Vec<u32> = data.regulations.iter().map(|r| r.id).collect();
    assert_eq!(ids, (1..=20).collect::<Vec<_>>());
    let first = &data.regulations[0];
    assert_eq!(first.kind, 1);
    let forbidden: Vec<&str> = first.forbidden().iter().filter_map(|&cid| data.cards.by_cid(cid)).map(|c| c.name.as_str()).collect();
    assert!(forbidden.contains(&"Victory Dragon"), "{forbidden:?}");
    assert!(forbidden.contains(&"Chaos Emperor Dragon - Envoy of the End"));
}

#[test]
fn decks_and_packs_reference_card_rows() {
    let dir = need_game!();
    let data = GameData::load(&dir).unwrap();
    assert_eq!(data.decks.len(), 163);
    assert_eq!(data.packs.len(), 82);
    let deck = data.decks.iter().find(|d| d.id == 1).unwrap();
    assert_eq!(deck.main.len(), 40);
    assert_eq!(data.cards.by_row(deck.main[0]).unwrap().name, "Malevolent Nuzzler");
    let pack = data.packs.iter().find(|p| p.id == 1).unwrap();
    assert!(pack.cards.iter().all(|&row| data.cards.by_row(row).is_some()));
}

#[test]
fn the_starter_deck_is_the_first_preset_the_regulation_allows() {
    let dir = need_game!();
    let data = GameData::load(&dir).unwrap();
    let cids = |d: &yo3_data::Deck| d.cards().map(|row| data.cards.by_row(row).unwrap().cid).collect::<Vec<_>>();
    let first = data.regulations.iter().find(|r| r.id == 1).unwrap();
    // Preset 1 holds a card regulation 1 forbids (4966); preset 2 is the first legal one of type 1.
    assert!(!first.allows(cids(data.decks.iter().find(|d| d.id == 1).unwrap())));
    let starter = data.starter_deck(1).unwrap();
    assert_eq!((starter.id, starter.kind), (2, 1));
    assert!(first.allows(cids(starter)));
}

#[test]
fn npc_ids_come_from_the_npc_preset_decks() {
    let dir = need_game!();
    let data = GameData::load(&dir).unwrap();
    // 25 NPCs, three levels each: Yami Yugi (120-122) to Kalin Kessler (192-194). A 26th has decks (195-197) but no
    // name, and its row on the NPC screen would be blank, so it is left out. Deck 19 is titled "Attack of the
    // Ancients", text 0x5FB4 + 2 * 120.
    assert_eq!(data.npc_ids(), (120..=194).collect::<Vec<_>>());
    assert_eq!(data.npc_deck(120).map(|d| d.id), Some(19));
}

#[test]
fn the_text_table_names_packs_and_structure_decks() {
    // data/text/txttbl_E.bin: the UI texts behind name_id / discription_id.
    let dir = need_game!();
    let data = GameData::load(&dir).unwrap();
    assert_eq!(data.text(23000), Some("Pack 1"));
    assert_eq!(data.text(24114), Some("GADGET BOX"));
    assert_eq!(data.text(24106), Some("Equip to Win!"));
    assert_eq!(data.text(99_999), None);
}

#[test]
fn the_item_catalogue_names_avatar_items_and_protectors() {
    // data/extra/yo3pbininfo.bin: a u32 count (4096), then {u16 id, u8 sub, u8 kind, u16 a, u16 b} per pbin id. Avatar
    // items are kind 1 (male) and 2 (female), b their name text, sub 2 the default part, 1 the parts a new player can
    // pick, 0 the ones to acquire; protectors are kind 11 (2304 "No Protector"), a their name text.
    let dir = need_game!();
    let data = GameData::load(&dir).unwrap();
    assert_eq!(data.catalogue.len(), 4096);
    let long_hair = data.catalogue[1];
    assert_eq!((long_hair.id, long_hair.sub, long_hair.kind, long_hair.b), (1, 1, 1, 26001));
    assert_eq!(data.text(long_hair.b as u32), Some("Long Hair"));
    let for_sale = data.avatar_items_for_sale();
    assert_eq!(for_sale.len(), 178 + 148);
    assert!(for_sale.iter().all(|e| matches!(e.kind, 1 | 2) && e.sub == 0 && e.id < 1274));
    let protectors = data.protectors();
    assert_eq!(protectors.len(), 196);
    assert_eq!((protectors[0].id, data.text(protectors[0].a as u32)), (2305, Some("P-01K")));
}
