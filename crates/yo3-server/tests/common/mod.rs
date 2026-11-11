//! A server with several clients online, for contract tests that need more than one player.
#![allow(dead_code, reason = "Each integration-test binary uses a different subset of the shared protocol helpers.")]
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{Message, Packet, Value};
use yo3_server::accounts::Directory;
use yo3_server::front::{Account, FrontSession, Hub, Player, World};

pub fn req(id: u32) -> Packet {
    Packet::Message(Message::reply(id))
}

pub fn req_with(id: u32, fields: &[(&str, Value)]) -> Packet {
    Packet::Message(fields.iter().fold(Message::reply(id), |m, (k, v)| m.with(k, v.clone())))
}

pub fn one(replies: Vec<Message>) -> Message {
    assert_eq!(replies.len(), 1, "expected one reply, got {replies:?}");
    replies.into_iter().next().unwrap()
}

pub struct Client {
    pub session: FrontSession,
    pub inbox: UnboundedReceiver<Message>,
}

impl Client {
    pub fn send(&mut self, p: Packet) -> Vec<Message> {
        self.session.handle(&p)
    }

    pub fn pushed(&mut self) -> Vec<Message> {
        std::iter::from_fn(|| self.inbox.try_recv().ok()).collect()
    }
}

pub struct Server {
    pub world: Arc<World>,
    pub hub: Arc<Mutex<Hub>>,
}

impl Server {
    pub fn new() -> Server {
        Server::with_world(World::new((1..=20).collect()))
    }

    pub fn with_world(world: World) -> Server {
        Server { world: Arc::new(world), hub: Arc::new(Mutex::new(Hub::default())) }
    }

    /// A client that logged in and selected `player_id`.
    pub fn join(&self, player_id: u32, name: &str) -> Client {
        self.join_as(Player::new(player_id, name))
    }

    /// A client of an account with `dpoint` BOOSTERPASS Points.
    pub fn join_with_bp(&self, player_id: u32, name: &str, dpoint: u32) -> Client {
        self.join_account(Account { players: vec![Player::new(player_id, name)], dpoint, ..Account::test() })
    }

    pub fn join_as(&self, player: Player) -> Client {
        self.join_account(Account { players: vec![player], ..Account::test() })
    }

    /// A client of the account at `addr` in `directory` (made on first contact), as the server's front makes them,
    /// logged in as its first player.
    pub fn join_from(&self, directory: &Arc<Mutex<Directory>>, addr: std::net::IpAddr) -> Client {
        let account = directory.lock().unwrap().account_for(addr);
        self.join_with(account, Some(directory.clone()))
    }

    /// An existing account, including one found by key only (as /join and add-account create).
    pub fn join_saved(&self, directory: &Arc<Mutex<Directory>>, user_id: u32) -> Client {
        let account = directory.lock().unwrap().account_of(user_id).expect("saved account");
        self.join_with(account, Some(directory.clone()))
    }

    fn join_account(&self, account: Account) -> Client {
        self.join_with(account, None)
    }

    fn join_with(&self, account: Account, directory: Option<Arc<Mutex<Directory>>>) -> Client {
        let player_id = account.players[0].player_id;
        let (tx, inbox) = unbounded_channel();
        let mut session = FrontSession::new(account, self.world.clone(), self.hub.clone(), tx);
        if let Some(d) = directory {
            session.set_directory(d);
        }
        let mut c = Client { session, inbox };
        c.send(Packet::Version("2010.03.23 skelton".into()));
        c.send(req_with(0x1F4C, &[("player_id", Value::U32(player_id))]));
        c.send(req_with(0x1FA4, &[("player_id", Value::U32(player_id))]));
        c
    }
}
