//! Yu-Gi-Oh! ONLINE 3 private server: gate, front and patch-HTTP services.
pub mod accounts;
pub mod front;
mod log;
mod net;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use yo3_proto::{Message, Value};

use accounts::Directory;
use front::{Deck, FrontSession, Hub, LOGIN_REGULATION, World};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: IpAddr,
    /// Front-server address the gate hands to clients.
    pub advertise: String,
    pub gate_port: u16,
    pub front_port: u16,
    pub http_port: u16,
    pub game_dir: PathBuf,
    pub log_dir: PathBuf,
    /// Players every new account starts with (one account per client address).
    pub players: Vec<String>,
}

impl Config {
    /// Localhost on OS-assigned ports; for tests.
    pub fn ephemeral(log_dir: PathBuf) -> Config {
        Config {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            advertise: "127.0.0.1".into(),
            gate_port: 0,
            front_port: 0,
            http_port: 0,
            game_dir: yo3_data::default_game_dir(),
            log_dir,
            players: vec!["test".into()],
        }
    }
}

/// Addresses the services actually bound to.
pub struct Running {
    pub gate: SocketAddr,
    pub front: SocketAddr,
    pub http: SocketAddr,
    tasks: Vec<JoinHandle<()>>,
}

impl Running {
    pub async fn wait(self) {
        for t in self.tasks {
            let _ = t.await;
        }
    }
}

/// Banlist ids and card rows from the client's own files, or ids 1-20 and no cards if the game is absent.
fn world(cfg: &Config) -> World {
    match yo3_data::GameData::load(&cfg.game_dir) {
        Ok(data) => World {
            card_rows: data.cards.iter().map(|c| c.row as usize).max().unwrap_or(0),
            // The first deck_type 1 preset (they look like starter decks) the login regulation allows.
            starter_deck: data.starter_deck(LOGIN_REGULATION).map(|d| Deck {
                id: 1,
                name: "Starter Deck".into(),
                main: d.main.clone(),
                extra: d.extra.clone(),
                side: d.side.clone(),
            }),
            npcs: data.npc_ids(),
            ..World::new(data.regulations.iter().map(|r| r.id as u16).collect())
        },
        Err(e) => {
            tracing::warn!("game data not loaded ({e}); using regulation ids 1-20");
            World::new((1..=20).collect())
        }
    }
}

pub async fn start(cfg: Config) -> anyhow::Result<Running> {
    let packets = Arc::new(log::PacketLog::create(&cfg.log_dir)?);
    let world = Arc::new(world(&cfg));
    let gate = TcpListener::bind((cfg.bind, cfg.gate_port)).await?;
    let front = TcpListener::bind((cfg.bind, cfg.front_port)).await?;
    let http = TcpListener::bind((cfg.bind, cfg.http_port)).await?;
    let addrs = (gate.local_addr()?, front.local_addr()?, http.local_addr()?);
    let front_port = addrs.1.port();

    let gate_task = {
        let (packets, advertise) = (packets.clone(), cfg.advertise.clone());
        tokio::spawn(net::accept_loop(gate, "gate", packets, move |_peer, _outbox| {
            let advertise = advertise.clone();
            let mut sent = false;
            move |_: &yo3_proto::Packet| {
                if std::mem::replace(&mut sent, true) {
                    return vec![];
                }
                vec![
                    Message::reply(0x21FE)
                        .with("host", Value::text(&advertise))
                        .with("port", Value::U32(front_port as u32)),
                ]
            }
        }))
    };
    let front_task = {
        let world = world.clone();
        let hub = Arc::new(Mutex::new(Hub::default()));
        let directory = Mutex::new(Directory::new(cfg.players.clone()));
        tokio::spawn(net::accept_loop(front, "front", packets.clone(), move |peer: SocketAddr, outbox| {
            let account = directory.lock().expect("directory lock").account_for(peer.ip());
            let mut session = FrontSession::new(account, world.clone(), hub.clone(), outbox);
            move |p: &yo3_proto::Packet| session.handle(p)
        }))
    };
    let http_task = tokio::spawn(net::http_loop(http, cfg.game_dir.clone()));

    tracing::info!(
        "gate {} front {} http {} (advertise {}); packets -> {}",
        addrs.0,
        addrs.1,
        addrs.2,
        cfg.advertise,
        packets.path().display()
    );
    Ok(Running { gate: addrs.0, front: addrs.1, http: addrs.2, tasks: vec![gate_task, front_task, http_task] })
}
