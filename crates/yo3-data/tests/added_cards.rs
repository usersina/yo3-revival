//! #75 against the real game files (skipped unless the game is installed, YO3_GAME_DIR or the default Wine path):
//! `engine/added-cards.json`'s rows extend every card file, the game's own rows stay as they were, and the new rows are
//! found by Konami id, by name, by text and in both display orders.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use yo3_data::added::{Added, extend_files};
use yo3_data::{CardDb, Ypk, default_game_dir};

fn game() -> Option<Ypk> {
    let dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(default_game_dir);
    let path = dir.join("package/data002.ypk");
    if !path.is_file() { eprintln!("game not installed; skipping"); return None }
    Some(Ypk::open(path).unwrap())
}

fn added() -> Added {
    Added::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engine")).unwrap()
}

fn u16s(b: &[u8]) -> Vec<u16> { b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect() }

#[test]
fn the_added_cards_extend_the_games_files_and_leave_its_cards_alone() {
    let Some(ypk) = game() else { return };
    let added = added();
    assert!(!added.is_empty(), "engine/added-cards.json lists the first card");
    let read = |n: &str| ypk.read(n).unwrap();
    let out: HashMap<String, Vec<u8>> = extend_files(read, &added).unwrap().into_iter().collect();
    let file = |n: &str| out.get(&format!("data/bin#/{n}.bin")).cloned().unwrap_or_else(|| ypk.read(&format!("data/bin#/{n}.bin")).unwrap().unwrap());
    let original = CardDb::parse(&file_orig(&ypk, "CARD_Prop"), &file_orig(&ypk, "CARD_Indx_E"), &file_orig(&ypk, "CARD_Name_E"), &file_orig(&ypk, "CARD_Desc_E"));
    let extended = CardDb::parse(&file("CARD_Prop"), &file("CARD_Indx_E"), &file("CARD_Name_E"), &file("CARD_Desc_E"));
    assert_eq!(extended.len(), original.len() + added.cards.len());
    for card in original.iter() {
        let same = extended.by_row(card.row).unwrap();
        assert_eq!((same.words, &same.name, &same.desc), (card.words, &card.name, &card.desc), "row {} changed", card.row);
    }
    let quasar = extended.by_cid(9709).expect("Shooting Quasar Dragon by its Konami id");
    assert_eq!((quasar.row, quasar.name.as_str(), quasar.atk, quasar.def, quasar.level, quasar.kind), (4834, "Shooting Quasar Dragon", 4000, 4000, 12, 18));
    assert!(quasar.extra_deck() && quasar.desc.starts_with("1 Tuner Synchro Monster + 2+ non-Tuner Synchro Monsters\r"));
    // The id lookup: base + i -> row, as 0x536FF0 builds it.
    let base = original.iter().map(|c| c.cid).min().unwrap();
    assert_eq!(u16s(&file("CARD_IntID"))[usize::from(9709 - base)], 4834);
    // Every language's index, names and texts; the readings' index and names.
    for lang in ["E", "F", "G", "I", "J", "K", "S", "R"] {
        let index = file(&format!("CARD_Indx_{lang}"));
        assert_eq!(index.len(), (4834 + added.cards.len() + 1) * 8, "CARD_Indx_{lang}");
        let lang_db = CardDb::parse(&file("CARD_Prop"), &index, &file(&format!("CARD_Name_{lang}")),
            &if lang == "R" { vec![] } else { file(&format!("CARD_Desc_{lang}")) });
        assert_eq!(lang_db.by_row(4834).map(|c| c.name.as_str()), Some("Shooting Quasar Dragon"), "{lang}");
    }
    // Both display orders hold the row once; Sort2 (alphabetical) between "Shooting Star Dragon"'s neighbours.
    for lang in ["E", "F", "G", "I", "J", "K", "S"] {
        for sort in ["CARD_Sort_", "CARD_Sort2_"] {
            let rows = u16s(&file(&format!("{sort}{lang}")));
            assert_eq!(rows.iter().filter(|&&r| r == 4834).count(), 1, "{sort}{lang}");
        }
    }
    let sort2 = u16s(&file("CARD_Sort2_E"));
    let at = sort2.iter().position(|&r| r == 4834).unwrap();
    let name = |i: usize| extended.by_row(sort2[i]).unwrap().name.to_lowercase();
    assert!(name(at - 1) <= "shooting quasar dragon".into() && name(at + 1) > "shooting quasar dragon".into(), "{} < Quasar < {}", name(at - 1), name(at + 1));
}

fn file_orig(ypk: &Ypk, n: &str) -> Vec<u8> { ypk.read(&format!("data/bin#/{n}.bin")).unwrap().unwrap() }

/// A banlist file as the game reads it (0x52DB30, 0x51A930): its four lists and the copies of an unlisted card.
fn banlist(blob: &[u8]) -> yo3_data::Regulation {
    let rule = yo3_proto::decode_file(blob).unwrap();
    let list = |k: &str| rule.get(k).and_then(yo3_proto::Value::as_bytes).map(|b| b.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect()).unwrap_or_default();
    yo3_data::Regulation { id: 0, kind: 0, status: 0, name_id: 0, lists: ["card0", "card1", "card2", "card3"].map(list),
        default_limit: rule.get("addition_card").and_then(yo3_proto::Value::as_u32).unwrap_or(3), record: rule }
}

#[test]
fn with_anime_cards_off_every_banlist_forbids_the_anime_only_cards() {
    // The server serves the added cards with `anime_only_banned` while anime cards are off: the launcher then writes each
    // of the game's 20 banlists (Unlimited too) with the anime-only cards allowed no copy, so Edit Deck marks them and the
    // game's own deck check refuses them. The printed added cards and the game's own keep their limits. On, nothing.
    let Some(ypk) = game() else { return };
    let added = added();
    let read = |n: &str| ypk.read(n).unwrap();
    let anime_only: Vec<u16> = added.cards.iter().filter(|c| c.anime_only && !c.token).map(|c| c.cid).collect();
    let printed: Vec<u16> = added.cards.iter().filter(|c| !c.anime_only && !c.token).map(|c| c.cid).collect();
    assert!(!anime_only.is_empty() && !printed.is_empty());
    let on = Added::parse(&added.render_for(true)).unwrap();
    assert!(!on.anime_only_banned && extend_files(read, &on).unwrap().iter().all(|(n, _)| !n.starts_with("data/regulation/")));
    let off = Added::parse(&added.render_for(false)).unwrap();
    assert!(off.anime_only_banned && off.hash() != on.hash(), "another folder of files for the launcher");
    let out: HashMap<String, Vec<u8>> = extend_files(read, &off).unwrap().into_iter().collect();
    for id in 1..=20 {
        let name = format!("data/regulation/{id}.y3r");
        let game = banlist(&ypk.read(&name).unwrap().expect("the game's banlist"));
        let ours = out.get(&name).map_or_else(|| game.clone(), |b| banlist(b));
        for &cid in &anime_only {
            assert_eq!(ours.limit(cid), 0, "banlist {id}: anime-only card {cid}");
        }
        for &cid in printed.iter().chain(&[4007, 4342, 4966]) {
            assert_eq!(ours.limit(cid), game.limit(cid), "banlist {id}: card {cid} keeps its limit");
        }
    }
    assert!(out.contains_key("data/regulation/3.y3r"), "Unlimited, which forbids nothing of its own, is written too");
}
