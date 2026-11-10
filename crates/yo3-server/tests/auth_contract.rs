//! The stock 0x200A names a player (0x50F690); the TLS transport must authorize that name before it seats them.
mod common;
use std::sync::{Arc, Mutex};
use common::{Server, one, req_with};
use yo3_proto::{Message, Packet, Value};
use yo3_server::auth::Identity;
use yo3_server::duel::DuelSession;
use yo3_server::front::{Deck, DuelEntry, Hub, World};

const ALICE: Identity = Identity { user_id: 1, session_id: 10 };

#[test]
fn the_front_binds_its_deck_to_the_authenticated_launcher_session() {
    let server = Server::new();
    let mut a = server.join(1001, "Alice");
    a.session.set_identity(ALICE);
    let duel = one(a.send(Packet::Message(Message::reply(0x17AC).item(Message::record().with("cpu_id", Value::U32(124))))));
    let id = duel.get("duel_id").cloned().unwrap();
    let Packet::Message(pick) = req_with(0x1F4F, &[("duel_id", id), ("update_type", Value::U32(1))]) else { unreachable!() };
    one(a.send(Packet::Message(pick.item(Message::deck().with("main", Value::u16_be_array(&[1]))))));
    assert_eq!(server.hub.lock().unwrap().duel_entry(1001).unwrap().identity, Some(ALICE));
}

#[test]
fn a_duel_cannot_claim_another_account_or_launchers_entry_on_the_same_ip() {
    for attacker in [None, Some(Identity { user_id: 2, session_id: 11 }), Some(Identity { user_id: 1, session_id: 12 })] {
        let world = Arc::new(World::new(vec![]));
        let hub = Arc::new(Mutex::new(Hub::default()));
        let ip = "127.0.0.1".parse().unwrap();
        hub.lock().unwrap().enter_duel(1001, DuelEntry { identity: Some(ALICE), peer: Some(ip), deck: Deck::default(), ..DuelEntry::default() });
        let mut thief = DuelSession::new(world.clone(), hub.clone(), 1).with_peer(ip);
        if let Some(identity) = attacker { thief = thief.with_identity(identity); }
        let enter = || Packet::Message(Message::reply(0x200A).with("player_id", Value::U32(1001)));
        assert!(thief.handle(&enter()).is_empty(), "unauthorized handoff: {attacker:?}");
        let mut owner = DuelSession::new(world, hub, 1).with_peer(ip).with_identity(ALICE);
        assert!(!owner.handle(&enter()).is_empty(), "the owner still enters");
    }
}
