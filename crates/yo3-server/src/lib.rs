//! Yu-Gi-Oh! ONLINE 3 private server: gate, front, duel and patch-HTTP services.
pub mod accounts;
mod store;
pub mod duel;
mod table;
pub mod engine;
pub mod translate;
pub mod front;
pub mod shop;
pub mod clock;
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
use translate::CardStats;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: IpAddr,
    /// The address the gate hands to clients for the front server, and the front for the duel server. None: the
    /// address each client reached, so one server answers on loopback, a LAN and a VPN at once.
    pub advertise: Option<String>,
    pub gate_port: u16,
    pub front_port: u16,
    /// Where clients open their duel connection (DuelServerConnection); only logged so far.
    pub duel_port: u16,
    pub http_port: u16,
    pub game_dir: PathBuf,
    pub log_dir: PathBuf,
    /// Players every new account starts with (one account per client address).
    pub players: Vec<String>,
    /// Each player's LP at the start of a duel (8000; lower only to test).
    pub duel_lp: u32,
    /// Testing aid: every player's main deck in a duel, as CARD_Prop rows repeated to 40 cards (empty: their own).
    pub duel_deck: Vec<u16>,
    /// Where accounts and duel records are kept (none: in memory only).
    pub data_dir: Option<PathBuf>,
}

impl Config {
    /// Localhost on OS-assigned ports; for tests.
    pub fn ephemeral(log_dir: PathBuf) -> Config {
        Config {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            advertise: None,
            gate_port: 0,
            front_port: 0,
            duel_port: 0,
            http_port: 0,
            game_dir: yo3_data::default_game_dir(),
            log_dir,
            players: vec!["test".into()],
            duel_lp: 8000,
            duel_deck: vec![],
            data_dir: None,
        }
    }
}

/// Addresses the services actually bound to.
pub struct Running {
    pub gate: SocketAddr,
    pub front: SocketAddr,
    pub duel: SocketAddr,
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

/// Banlist ids, card rows and NPCs from the client's own files, or ids 1-20 and nothing else if the
/// game is absent.
fn world(cfg: &Config, duel_port: u16) -> World {
    let duel_server = (cfg.advertise.clone().unwrap_or_default(), duel_port);
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
            cards: data
                .cards
                .iter()
                .map(|c| {
                    let stats = CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race };
                    (c.row, stats)
                })
                .collect(),
            engine_dir: engine::Engine::dir(),
            start_lp: cfg.duel_lp,
            test_deck: (!cfg.duel_deck.is_empty()).then(|| cfg.duel_deck.clone()),
            npcs: data.npc_ids(),
            npc_decks: data
                .npc_ids()
                .into_iter()
                .filter_map(|id| {
                    let d = data.npc_deck(id)?;
                    Some((id, Deck { id: d.id, name: String::new(), main: d.main.clone(), extra: d.extra.clone(), side: d.side.clone() }))
                })
                .collect(),
            duel_server,
            // Booster packs (pack_type 1), their cards read as rarity tiers.
            packs: data
                .packs
                .iter()
                .filter(|p| p.kind == 1 && !p.cards.is_empty())
                .map(|p| {
                    let date = |k: &str| p.record.get(k).and_then(Value::as_str).unwrap_or_default();
                    shop::ShopPack { id: p.id, released: date("created"), modified: date("modified"), tiers: shop::tiers(&p.cards) }
                })
                .collect(),
            // Structure decks: the deck_type 1 presets with their own art (SD001-SD005; the others are starters).
            structure_decks: data
                .decks
                .iter()
                .filter(|d| d.kind == 1 && d.record.get("deck_image").and_then(Value::as_str).is_some_and(|i| i != "noImage"))
                .map(|d| {
                    let date = |k: &str| d.record.get(k).and_then(Value::as_str).unwrap_or_default();
                    let name = d.name_id().and_then(|id| data.text(id)).unwrap_or("Structure Deck").to_string();
                    let deck = Deck { id: d.id, name, main: d.main.clone(), extra: d.extra.clone(), side: d.side.clone() };
                    shop::ShopDeck { released: date("created"), modified: date("modified"), deck }
                })
                .collect(),
            ..World::new(data.regulations.iter().map(|r| r.id as u16).collect())
        },
        Err(e) => {
            tracing::warn!("game data not loaded ({e}); using regulation ids 1-20");
            World { duel_server, ..World::new((1..=20).collect()) }
        }
    }
}

pub async fn start(cfg: Config) -> anyhow::Result<Running> {
    let packets = Arc::new(log::PacketLog::create(&cfg.log_dir)?);
    let gate = TcpListener::bind((cfg.bind, cfg.gate_port)).await?;
    let front = TcpListener::bind((cfg.bind, cfg.front_port)).await?;
    let duel = TcpListener::bind((cfg.bind, cfg.duel_port)).await?;
    let http = TcpListener::bind((cfg.bind, cfg.http_port)).await?;
    let addrs = (gate.local_addr()?, front.local_addr()?, duel.local_addr()?, http.local_addr()?);
    let front_port = addrs.1.port();
    let world = Arc::new(world(&cfg, addrs.2.port()));
    match &world.engine_dir {
        Some(dir) => tracing::info!("rules engine: {}", dir.display()),
        None => tracing::warn!("rules engine not fetched (run engine/fetch.sh): duels open but can't be played"),
    }

    let gate_task = {
        let (packets, advertise) = (packets.clone(), cfg.advertise.clone());
        tokio::spawn(net::accept_loop(gate, "gate", packets, move |_peer, local: SocketAddr, _outbox| {
            let advertise = advertise.clone().unwrap_or_else(|| local.ip().to_string());
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
    if let Some(dir) = &cfg.data_dir {
        store::back_up(dir)?;
    }
    let directory = Arc::new(Mutex::new(match &cfg.data_dir {
        Some(dir) => Directory::open(cfg.players.clone(), &dir.join("accounts.json"))?,
        None => Directory::new(cfg.players.clone()),
    }));
    // Locks are taken hub first, then accounts (duel results pay BP), never the other way.
    let mut hub = match &cfg.data_dir {
        Some(dir) => Hub::with_records(&dir.join("records.json")),
        None => Hub::default(),
    };
    hub.set_directory(directory.clone());
    let hub = Arc::new(Mutex::new(hub));
    let front_task = {
        let (world, hub) = (world.clone(), hub.clone());
        tokio::spawn(net::accept_loop(front, "front", packets.clone(), move |peer: SocketAddr, local: SocketAddr, outbox| {
            let account = directory.lock().expect("directory lock").account_for(peer.ip());
            let mut session = FrontSession::new(account, world.clone(), hub.clone(), outbox);
            session.set_reached_at(local.ip());
            session.set_directory(directory.clone());
            move |p: &yo3_proto::Packet| session.handle(p)
        }))
    };
    // Duel clocks between players run out on the server's side: checked once a second.
    let clock_task = {
        let hub = hub.clone();
        tokio::spawn(async move {
            let mut every = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                every.tick().await;
                duel::tick_tables(&hub, std::time::Instant::now());
            }
        })
    };
    // The duel connection (DuelManager 0x50F690): a client duels the server as the CPU, or another client at a table.
    let duel_task = tokio::spawn(net::accept_loop(duel, "duel", packets.clone(), move |_peer, _local, outbox| {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        let mut session = duel::DuelSession::new(world.clone(), hub.clone(), seed).with_outbox(outbox);
        move |p: &yo3_proto::Packet| session.handle(p)
    }));
    let http_task = tokio::spawn(net::http_loop(http, cfg.game_dir.clone()));

    tracing::info!(
        "gate {} front {} duel {} http {} (advertise {}); packets -> {}",
        addrs.0,
        addrs.1,
        addrs.2,
        addrs.3,
        cfg.advertise.as_deref().unwrap_or("the address each client reached"),
        packets.path().display()
    );
    let tasks = vec![gate_task, front_task, duel_task, http_task, clock_task];
    Ok(Running { gate: addrs.0, front: addrs.1, duel: addrs.2, http: addrs.3, tasks })
}
