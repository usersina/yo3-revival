//! Yu-Gi-Oh! ONLINE 3 private server: authenticated TLS front/duel transport and internal asset/admin HTTP.
pub mod cli;
pub mod accounts;
pub mod auth;
mod transport;
pub mod admin;
mod manager;
pub mod store;
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
pub use net::IDLE_TIMEOUT;

use crate::lock::Locked;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use yo3_proto::Value;

use accounts::Directory;
use front::{Deck, Hub, LOGIN_REGULATION, STARTER_REGULATION, World};

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
    pub tls: auth::TlsConfig,
    pub bind: IpAddr,
    /// The address the gate hands to clients for the front server, and the front for the duel server. None: the
    /// address each client reached, so one server answers on loopback, a LAN and a VPN at once.
    pub advertise: Option<String>,
    pub http_port: u16,
    pub game_dir: PathBuf,
    pub log_dir: PathBuf,
    pub packet_logging: bool,
    pub diagnostic_id: Option<String>,
    /// Default players for offline account administration; login never creates an account.
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
        let tls = auth::test_config(&log_dir);
        Config {
            tls,
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            advertise: None,
            http_port: 0,
            game_dir: yo3_data::default_game_dir(),
            log_dir: log_dir.clone(),
            packet_logging: true, diagnostic_id: None,
            players: vec!["test".into()],
            duel_lp: 8000,
            duel_regulation: None,
            duel_deck: vec![],
            duel_extra: vec![],
            duel_time: None,
            data_dir: Some(log_dir.join("test-auth/data")),
            regulation: None,
            idle_timeout: net::IDLE_TIMEOUT,
            engine_problem: None,
            duel_check: false,
        }
    }
}

/// Addresses the services actually bound to.
pub struct Running {
    pub auth: SocketAddr,
    pub http: SocketAddr,
    tasks: Vec<JoinHandle<()>>,
    shutdown: tokio::sync::watch::Sender<bool>,
}

impl Running {
    pub async fn wait(mut self) {
        let mut stop = self.shutdown.subscribe();
        while !*stop.borrow_and_update() {
            if stop.changed().await.is_err() { break }
        }
        // Let the management response reach its caller before the CLI shuts its runtime down.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for task in self.tasks.drain(..) { task.abort(); }
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
    let packets = Arc::new(if cfg.packet_logging { log::PacketLog::create(&cfg.log_dir)? } else { log::PacketLog::disabled()? });
    let tls = yo3_tunnel::acceptor(&cfg.tls.certificate, &cfg.tls.private_key)?;
    let authenticated = TcpListener::bind((cfg.bind, cfg.tls.port)).await?;
    let http = TcpListener::bind((Ipv4Addr::LOCALHOST, cfg.http_port)).await?;
    let (auth_addr, http_addr) = (authenticated.local_addr()?, http.local_addr()?);
    // The launcher supplies its local duel port for each front connection's handoff.
    let mut world = world(&cfg, 0, http_addr.port())?;
    world.engine_problem = cfg.engine_problem.clone().or_else(|| engine::problem(world.engine_dir.as_deref()));
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
    if let Some(dir) = &cfg.data_dir {
        store::initialize(dir)?;
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
        hub.keep_duel_logs(&dir.join("duellogs"), http_addr.port());
    }
    hub.set_round_delay(EVENT_ROUND_DELAY);
    hub.set_directory(directory.clone());
    let hub = Arc::new(Mutex::new(hub));
    let auth_task = tokio::spawn(transport::accept_loop(authenticated, tls, world.clone(), hub.clone(), directory.clone(),
        packets.clone(), quiet, heartbeats, http_addr));
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
    // The host's BP commands reach the running server through its HTTP port, with a token only the host can read.
    let admin_token = cfg.data_dir.as_ref().and_then(|dir| {
        admin::write_live(dir, http_addr.port()).map_err(|e| tracing::warn!("admin commands won't reach this server ({}): {e}", dir.display())).ok()
    });
    let (shutdown, _) = tokio::sync::watch::channel(false);
    let manager = manager::Manager { accounts: directory.clone(), hub, shutdown: shutdown.clone(), engine_problem: world.engine_problem.clone(), listen: auth_addr, diagnostic_id: cfg.diagnostic_id };
    let http_task = tokio::spawn(net::http_loop(http, cfg.game_dir.clone(), cfg.data_dir.as_ref().map(|d| d.join("duellogs")), directory, admin_token, manager));
    tracing::info!("authenticated TLS {} local management/assets HTTP {}; packets -> {}", auth_addr, http_addr, packets.path().display());
    Ok(Running { auth: auth_addr, http: http_addr, tasks: vec![auth_task, http_task, clock_task], shutdown })
}
