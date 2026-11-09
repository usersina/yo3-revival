//! Yu-Gi-Oh! ONLINE 3 private server: gate, front, duel and patch-HTTP services.
pub mod accounts;
pub mod admin;
mod store;
pub(crate) mod lock;
pub mod duel;
pub mod duelcheck;
mod duellog;
mod table;
pub mod engine;
pub mod translate;
mod card_options;
pub mod front;
pub mod shop;
mod social;
mod mail;
mod teams;
mod trade;
mod deckcheck;
mod rental;
mod events;
pub mod clock;
mod log;
mod net;
mod join;
pub use net::IDLE_TIMEOUT;

use crate::lock::Locked;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use yo3_proto::{Message, Value};

use accounts::Directory;
use front::{Deck, FrontSession, Hub, LOGIN_REGULATION, STARTER_REGULATION, World};

/// The official character decks every player gets: the client's NPC decks of Yami Yugi (NPCs 120-122: "Attack of the
/// Ancients", "Miracle Magic", "Slifer the Lightning"), Jaden Yuki (147-149) and Yusei Fudo (171-173), named after
/// their duelist. The deck list sends each once, in the room its 10 decks leave.
const OFFICIAL_DECKS: [(u32, &str); 9] = [
    (121, "Yugi"),
    (147, "Jaden"),
    (171, "Yusei"),
    (120, "Yugi"),
    (148, "Jaden"),
    (172, "Yusei"),
    (122, "Yugi"),
    (149, "Jaden"),
    (173, "Yusei"),
];
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
    /// Serve the join page (`/join`) on the HTTP port; off for a server open to the internet (`--no-join`).
    pub join: bool,
    pub game_dir: PathBuf,
    pub log_dir: PathBuf,
    /// Players every new account starts with (one account per client address).
    pub players: Vec<String>,
    /// Each player's LP at the start of a duel (8000; lower only to test).
    pub duel_lp: u32,
    /// Testing aid: every duel's deck is checked against this regulation, not the duel's own.
    pub duel_regulation: Option<u32>,
    /// Testing aid: every player's main deck in a duel, as CARD_Prop rows repeated to 40 cards (empty: their own).
    pub duel_deck: Vec<u16>,
    /// Testing aid: every player's extra deck in a duel, as CARD_Prop rows (empty: their own).
    pub duel_extra: Vec<u16>,
    /// Testing aid: every duel between players' turn and total clocks in seconds, 0 for off (none: the duel's own).
    pub duel_time: Option<(u32, u32)>,
    /// Where accounts and duel records are kept (none: in memory only).
    pub data_dir: Option<PathBuf>,
    /// The banlist players log in with (Auto Duel and NPC duels), as a regulation id or name ("unlimited"); none:
    /// regulation 1.
    pub regulation: Option<String>,
    /// How long a connection may stay silent before it is closed (the client heartbeats every 30 s).
    pub idle_timeout: std::time::Duration,
    /// Testing aid: pretend the rules engine can't run, with this reason (none: check for real at start).
    pub engine_problem: Option<String>,
    /// Testing aid: check every duel's client view against the rules engine after each step and log an ERROR where they
    /// differ (`duelcheck.rs`; the protocol monkey's duels run with it).
    pub duel_check: bool,
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
            join: true,
            game_dir: yo3_data::default_game_dir(),
            log_dir,
            players: vec!["test".into()],
            duel_lp: 8000,
            duel_regulation: None,
            duel_deck: vec![],
            duel_extra: vec![],
            duel_time: None,
            data_dir: None,
            regulation: None,
            idle_timeout: net::IDLE_TIMEOUT,
            engine_problem: None,
            duel_check: false,
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
/// A regulation by its id or its name (ignoring case), among the client's (id, name) pairs; else what there is.
pub fn regulation_by(arg: &str, known: &[(u32, String)]) -> Result<u32, String> {
    let found = match arg.trim().parse::<u32>() {
        Ok(id) => known.iter().find(|(k, _)| *k == id),
        Err(_) => known.iter().find(|(_, name)| name.eq_ignore_ascii_case(arg.trim())),
    };
    found.map(|(id, _)| *id).ok_or_else(|| {
        let choices: Vec<String> = known.iter().map(|(id, name)| format!("{id} \"{name}\"")).collect();
        format!("no regulation {arg:?}; the client has {}", choices.join(", "))
    })
}

fn world(cfg: &Config, duel_port: u16, http_port: u16) -> anyhow::Result<World> {
    let duel_server = (cfg.advertise.clone().unwrap_or_default(), duel_port);
    let world = match yo3_data::GameData::load(&cfg.game_dir) {
        Ok(data) => World {
            card_rows: data.cards.iter().map(|c| c.row as usize).max().unwrap_or(0),
            // The first deck_type 1 preset (they look like starter decks) the login regulation allows.
            starter_deck: data.starter_deck(STARTER_REGULATION).map(|d| Deck {
                id: 1,
                name: "Starter Deck".into(),
                main: d.main.clone(),
                extra: d.extra.clone(),
                side: d.side.clone(),
            }),
            official_decks: OFFICIAL_DECKS
                .iter()
                .filter_map(|&(npc, who)| {
                    let d = data.npc_deck(npc)?;
                    let title = data.npc_deck_title(npc).unwrap_or_default();
                    let name = format!("{who} - {title}");
                    Some(Deck { id: d.id, name, main: d.main.clone(), extra: d.extra.clone(), side: d.side.clone() })
                })
                .collect(),
            cards: data
                .cards
                .iter()
                .map(|c| {
                    let stats = CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race };
                    (c.row, stats)
                })
                .collect(),
            token_rows: data.cards.iter().filter(|c| (3900..4000).contains(&c.cid)).fold(std::collections::HashMap::new(), |mut m, c| {
                m.entry(c.name.clone()).or_insert_with(Vec::new).push(c.row);
                m
            }),
            engine_dir: engine::Engine::dir(),
            start_lp: cfg.duel_lp,
            test_regulation: cfg.duel_regulation,
            test_deck: (!cfg.duel_deck.is_empty()).then(|| cfg.duel_deck.clone()),
            test_extra: (!cfg.duel_extra.is_empty()).then(|| cfg.duel_extra.clone()),
            test_times: cfg.duel_time,
            duel_check: cfg.duel_check,
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
            http_port,
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
            protectors: data.protectors().iter().map(|e| e.id).collect(),
            avatar_items: data.avatar_items_for_sale().iter().map(|e| (e.id, e.b)).collect(),
            login_regulation: match &cfg.regulation {
                Some(arg) => {
                    let known: Vec<(u32, String)> =
                        data.regulations.iter().map(|r| (r.id, data.text(r.name_id).unwrap_or_default().to_string())).collect();
                    regulation_by(arg, &known).map_err(anyhow::Error::msg)?
                }
                None => LOGIN_REGULATION,
            },
            regulations: data.regulations.iter().map(|r| (r.id, r.clone())).collect(),
            ..World::new(data.regulations.iter().map(|r| r.id as u16).collect())
        },
        Err(e) => {
            tracing::warn!("game data not loaded ({e}); using regulation ids 1-20");
            let login_regulation = match cfg.regulation.as_deref().map(str::parse::<u32>) {
                Some(Ok(id)) => id,
                Some(Err(_)) => anyhow::bail!("--regulation by name needs the game's data"),
                None => LOGIN_REGULATION,
            };
            World { duel_server, http_port, login_regulation, ..World::new((1..=20).collect()) }
        }
    };
    tracing::info!("players log in with regulation {}", world.login_regulation);
    Ok(world)
}

/// How long a user event waits between two of its rounds: the winners get through the result screens to the lobby.
const EVENT_ROUND_DELAY: std::time::Duration = std::time::Duration::from_secs(20);

pub async fn start(cfg: Config) -> anyhow::Result<Running> {
    let packets = Arc::new(log::PacketLog::create(&cfg.log_dir)?);
    let gate = TcpListener::bind((cfg.bind, cfg.gate_port)).await?;
    let front = TcpListener::bind((cfg.bind, cfg.front_port)).await?;
    let duel = TcpListener::bind((cfg.bind, cfg.duel_port)).await?;
    let http = TcpListener::bind((cfg.bind, cfg.http_port)).await?;
    let addrs = (gate.local_addr()?, front.local_addr()?, duel.local_addr()?, http.local_addr()?);
    let front_port = addrs.1.port();
    let mut world = world(&cfg, addrs.2.port(), addrs.3.port())?;
    world.engine_problem = cfg.engine_problem.clone().or_else(|| engine::problem(world.engine_dir.as_deref()));
    let duels_unavailable = world.engine_problem.is_some();
    if cfg.duel_check {
        tracing::info!("duel check on: every duel's client view is compared with the rules engine after each step");
    }
    let world = Arc::new(world);
    match (&world.engine_problem, &world.engine_dir) {
        (Some(why), _) => tracing::error!("DUELS WON'T WORK: {why}. Duels will end at once as draws; players are told in the news and on the join page."),
        (None, Some(dir)) => tracing::info!("rules engine: {}", dir.display()),
        (None, None) => {}
    }

    let quiet = net::Idle { limit: cfg.idle_timeout, heartbeats: false };
    let heartbeats = net::Idle { limit: cfg.idle_timeout, heartbeats: true };
    let gate_task = {
        let (packets, advertise) = (packets.clone(), cfg.advertise.clone());
        tokio::spawn(net::accept_loop(gate, "gate", packets, quiet, move |_peer, local: SocketAddr, _outbox| {
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
    if let Some(dir) = &cfg.data_dir {
        hub.keep_social(&dir.join("friends.json"));
        hub.keep_mail(&dir.join("mail.json"));
        hub.keep_teams(&dir.join("teams.json"));
        hub.keep_events(&dir.join("events.json"));
        hub.keep_rentals(&dir.join("rentals.json"));
        hub.keep_duel_logs(&dir.join("duellogs"), addrs.3.port());
    }
    hub.set_round_delay(EVENT_ROUND_DELAY);
    hub.set_directory(directory.clone());
    let hub = Arc::new(Mutex::new(hub));
    let directory_http = directory.clone();
    let front_task = {
        let (world, hub) = (world.clone(), hub.clone());
        tokio::spawn(net::accept_loop(front, "front", packets.clone(), quiet, move |peer: SocketAddr, local: SocketAddr, outbox| {
            let account = directory.locked().login(peer.ip(), std::time::Instant::now());
            let mut session = FrontSession::new(account, world.clone(), hub.clone(), outbox);
            session.set_reached_at(local.ip());
            session.set_peer(peer.ip());
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
                // On a blocking thread (a table's engine call may take a while), and a panic in one pass is logged,
                // not the end of the clock.
                let hub = hub.clone();
                let pass = tokio::task::spawn_blocking(move || {
                    duel::tick_tables(&hub, std::time::Instant::now());
                    let mut hub = hub.locked();
                    hub.tick_events(std::time::Instant::now());
                    hub.tick_rentals(chrono::Utc::now().timestamp());
                });
                if let Err(e) = pass.await {
                    tracing::error!("clock pass failed: {e}");
                }
            }
        })
    };
    // The duel connection (DuelManager 0x50F690): a client duels the server as the CPU, or another client at a table.
    let duel_task = tokio::spawn(net::accept_loop(duel, "duel", packets.clone(), heartbeats, move |peer, _local, outbox| {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        let mut session = duel::DuelSession::new(world.clone(), hub.clone(), seed).with_outbox(outbox).with_peer(peer.ip());
        move |p: &yo3_proto::Packet| session.handle(p)
    }));
    let join_info = cfg.join.then(|| join::JoinInfo { gate_port: addrs.0.port(), http_port: addrs.3.port(), advertise: cfg.advertise.clone(), duels_unavailable });
    // The host's BP commands reach the running server through its HTTP port, with a token only the host can read.
    let admin_token = cfg.data_dir.as_ref().and_then(|dir| {
        admin::write_live(dir, addrs.3.port()).map_err(|e| tracing::warn!("admin commands won't reach this server ({}): {e}", dir.display())).ok()
    });
    let http_task = tokio::spawn(net::http_loop(http, cfg.game_dir.clone(), cfg.data_dir.as_ref().map(|d| d.join("duellogs")), directory_http, join_info, admin_token));

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
