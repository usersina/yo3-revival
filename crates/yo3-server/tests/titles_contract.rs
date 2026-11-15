//! Titles a player earns (`crate::titles`, catalogue kind 10): Avatar Settings lists the owned ones (their pbin bits in
//! the player's own record), so earning one adds it to the player's items, kept with the account, and a chat line says
//! so (`0x2455`, chat type 0).
mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use common::{Server, one, req};
use yo3_proto::Value;
use yo3_server::accounts::Directory;
use yo3_server::front::Ended;
use yo3_server::titles::{DuelStats, TitleData};

fn titles() -> TitleData {
    let names = [("BRONZE DUELIST", 250), ("SUMMONER OF EXODIA", 488), ("ASPIRING DUELIST", 175), ("KING OF DOMINO CITY", 160)];
    TitleData {
        by_name: names.iter().map(|(n, id)| (n.to_string(), *id)).collect(),
        npcs: [(120, "Yami Yugi".to_string()), (121, "Yami Yugi".to_string()), (122, "Yami Yugi".to_string())].into(),
        ..TitleData::default()
    }
}

#[test]
fn a_duels_end_earns_titles_kept_with_the_account_and_announced() {
    let dir = std::env::temp_dir().join(format!("yo3-titles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let directory = Arc::new(Mutex::new(Directory::open(vec!["Yugi".into()], &dir.join("accounts.json")).unwrap()));
    let s = Server::new();
    s.hub.lock().unwrap().set_directory(directory.clone());
    s.hub.lock().unwrap().set_titles(Arc::new(titles()));
    let mut c = s.join_from(&directory, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    let me = one(c.send(req(0x2CEE))).items.remove(0);
    let player_id = me.get("player_id").and_then(Value::as_u32).unwrap();
    let owns = |c: &mut common::Client, bit: usize| {
        let me = one(c.send(req(0x2CEE))).items.remove(0);
        me.get("pbin").and_then(Value::as_bytes).is_some_and(|b| b[bit >> 3] & 1 << (bit & 7) != 0)
    };
    // A loss to Yami Yugi (level id 120): his defeat title; a win by Exodia: SUMMONER OF EXODIA.
    let lost = Ended { player_id, opponent: 120, npc: true, duel_id: 1, match_type: 1, result: 2, replay: None, stats: None, start_lp: 8000 };
    s.hub.lock().unwrap().finish_duel(&lost);
    let exodia = DuelStats { win_reason: Some(0x10), ..DuelStats::default() };
    let won = Ended { player_id, opponent: 121, npc: true, duel_id: 2, match_type: 1, result: 1, replay: None, stats: Some(exodia), start_lp: 8000 };
    s.hub.lock().unwrap().finish_duel(&won);
    let lines: Vec<String> = c.pushed().iter().filter(|m| m.request() == Some(0x2455))
        .filter_map(|m| m.items.first()?.get("chat_str")?.as_str().map(|s| s.to_string())).collect();
    assert!(lines.iter().any(|l| l.contains("ASPIRING DUELIST")) && lines.iter().any(|l| l.contains("SUMMONER OF EXODIA")), "{lines:?}");
    assert!(owns(&mut c, 175) && owns(&mut c, 488));
    assert!(!owns(&mut c, 250), "two duels are not ten");
    // Kept with the account, counters too.
    let fresh = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    let p = fresh.player(player_id).unwrap();
    assert!(p.items.contains(&175) && p.items.contains(&488));
    assert_eq!(p.counters.get("duels"), Some(&2));
    // No title twice: the same win again says nothing new.
    let again = Ended { duel_id: 3, ..won };
    assert!(!s.hub.lock().unwrap().award_titles(player_id, Some((&DuelStats { win_reason: Some(0x10), ..DuelStats::default() }, 1, 8000))).contains(&488));
    drop(again);
    let _ = std::fs::remove_dir_all(&dir);
}
