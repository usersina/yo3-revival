//! The map's event facilities: Official Event (MenuSelect 21), Championship (20) and User event
//! (22). Official Event opens TournamentScene, which asks for the event list first (0x177A, sent at
//! 0x4D0BB0) and scans it for an event with event_type 10 and status 12; with none it takes its
//! no-event path (event_id 0). 0x177C would be the error reply (dialog 0x4F7940). User event opens
//! UserEventScene, which first asks for the player's own event (0x339A, sent at 0x4D9BB0).
mod common;

use common::{Server, one, req};
use yo3_proto::Value;

#[test]
fn the_event_list_is_empty() {
    // 0x177B answers both the event list (0x177A) and the event search (0x1789 {search_flag, pname,
    // status, pack_type, base_point, front_num, back_num}, sent by UserEventScene at 0x4D9E37).
    let mut c = Server::new().join(1001, "Yugi");
    for request in [0x177A, 0x1789] {
        let r = one(c.send(req(request)));
        assert_eq!(r.request(), Some(0x177B), "{request:#x}");
        // UserEventScene's list (0x4D66A0) reads the total, like player search (tracer: 0x4D670D).
        assert_eq!(r.get("result_number"), Some(&Value::U32(0)));
        assert!(r.items.is_empty());
    }
}

#[test]
fn the_player_holds_no_user_event() {
    // 0x33FE: the scene keeps event_id only while remain is non-zero (0x4D9BB0); 0x33FF = error.
    let mut c = Server::new().join(1001, "Yugi");
    let r = one(c.send(req(0x339A)));
    assert_eq!(r.request(), Some(0x33FE));
    assert_eq!(r.get("remain"), Some(&Value::U32(0)));
    assert_eq!(r.get("event_id"), Some(&Value::U32(0)));
}
