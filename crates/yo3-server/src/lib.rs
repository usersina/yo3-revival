//! Yu-Gi-Oh! ONLINE 3 private server: gate, front and patch-HTTP services.
pub mod front;
mod log;
mod net;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use yo3_proto::{Message, Value};

use front::{Account, FrontSession, World};

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

/// Banlist ids from the client's own regulation files, or ids 1-20 if the game is absent.
fn world(cfg: &Config) -> World {
    match yo3_data::GameData::load(&cfg.game_dir) {
        Ok(data) => World::new(data.regulations.iter().map(|r| r.id as u16).collect()),
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
        tokio::spawn(net::accept_loop(gate, "gate", packets, move || {
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
        tokio::spawn(net::accept_loop(front, "front", packets.clone(), move || {
            let mut session = FrontSession::new(Account::test(), world.clone());
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
