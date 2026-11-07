//! Main Menu screens (MainGamen's menu, 0x46FA60): what each asks for when it opens.
mod common;

use common::{Server, one, req, req_with};
use yo3_proto::{Message, Value};

fn int(m: &Message, k: &str) -> Option<u32> {
    m.get(k).and_then(Value::as_u32)
}

#[test]
fn the_rental_card_list_opens_on_its_three_replies() {
    // RentalRecord (0x481A30) builds its screen only after the whole chain (0x482250): 0x2C34 -> 0x2D6C (also dataset
    // slot 8), then 0x2C35 {rental_kind 0, base_id} -> 0x2D6E (the borrowed cards), then 0x2C35 {rental_kind 1} ->
    // 0x2D6E (the cards lent). The lists are count-bounded, so empty ones are safe; an error reply never opens it.
    let mut c = Server::new().join(1001, "Yugi");
    assert_eq!(one(c.send(req(0x2C34))).request(), Some(0x2D6C));
    for kind in [0, 1] {
        let r = one(c.send(req_with(0x2C35, &[("rental_kind", Value::U32(kind)), ("base_id", Value::U32(0))])));
        assert_eq!(r.request(), Some(0x2D6E), "rental_kind {kind}");
    }
}

#[test]
fn the_duel_log_gets_its_championships_and_server_logs() {
    // DuelLogWindow (0x499750) asks 0x1793 when it opens and queues the server log tab behind its reply 0x177B (the
    // championship list, items {year, season, event_id}). Server Duel Logs asks 0x19C8 {pname, offset} -> 0x19C9
    // items {end_time, match_type, dlv_1, dlv_2, pname_1, pname_2, dlog_comment, duel_id, filename, dlogURL}
    // (0x49ADF0); the server keeps no replays, so none are listed.
    let mut c = Server::new().join(1001, "Yugi");
    assert_eq!(one(c.send(req(0x1793))).request(), Some(0x177B));
    let r = one(c.send(req_with(0x19C8, &[("pname", Value::text("")), ("offset", Value::U32(0))])));
    assert_eq!((r.request(), r.items.len()), (Some(0x19C9), 0));
}

#[test]
fn a_boosterpass_code_gets_an_answer() {
    // BpInputWindow sends 0x20D0 {dpcode} for a 16-character code and waits for 0x23F0 (0x40F7B0; there is no error
    // reply). dpauth_result 0 adds dpGetPoints; anything else shows the "ng" frame. Konami's codes are gone.
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req_with(0x20D0, &[("dpcode", Value::text("ABCDEFGHIJKLMNOP"))])));
    assert_eq!(r.request(), Some(0x23F0));
    assert!(int(&r, "dpauth_result").is_some_and(|n| n != 0));
}

#[test]
fn my_profile_and_player_information_count_the_duels_against_players() {
    // My Profile (0x1F49 -> 0x2260, ProfileGet 0x4705D0) reads n_win, n_lose, n_duelall... from the player's record;
    // Player Information reads n_win and n_lose from the search entry (0x4B7350). Duels against players count;
    // practice against NPCs doesn't (**guess**).
    let server = Server::new();
    for (opponent, result) in [(1002, 1), (1002, 1), (1002, 2), (1003, 3), (124, 1)] {
        server.hub.lock().unwrap().record_result(1001, opponent, result);
    }
    let mut yugi = server.join(1001, "Yugi");
    let me = one(yugi.send(req(0x1F49))).items.remove(0);
    assert_eq!([int(&me, "n_win"), int(&me, "n_lose"), int(&me, "n_draw"), int(&me, "n_duelall")], [Some(2), Some(1), Some(1), Some(4)]);
    let mut kaiba = server.join(1002, "Kaiba");
    let found = kaiba.send(req_with(0x2CF0, &[("search_flag", Value::U32(1)), ("base_point", Value::U32(0))]));
    let yugi_row = found.iter().flat_map(|m| &m.items).find(|i| int(i, "player_id") == Some(1001)).expect("Yugi listed");
    assert_eq!((int(yugi_row, "n_win"), int(yugi_row, "n_lose")), (Some(2), Some(1)));
}
