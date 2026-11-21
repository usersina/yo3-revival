//! Yu-Gi-Oh! ONLINE 3 private server: authenticated TLS front/duel transport and internal asset/admin HTTP.
pub mod cli;
pub mod accounts;
pub mod logins;
pub mod links;
pub mod account_ops;
pub mod settings;
pub mod config;
pub mod api;
pub mod auth;
mod transport;
pub mod admin;
pub mod panel;
pub mod staff_mfa;
pub mod dashboard;
pub mod bootstrap;
#[cfg(unix)] pub mod dashboard_socket;
pub mod recent;
pub mod store;
pub(crate) mod lock;
/// The engine's vocabulary, shared with the engine program (`yo3-ocg`); its `named!` makes `wire`'s ids too.
#[macro_use]
pub extern crate yo3_ocg as ocg;

pub mod duel;
pub mod wire;
pub mod duelcheck;
pub mod client_commands;
mod duellog;
pub mod table;
pub mod engine;
pub mod npc;
pub mod profile;
pub mod banter;
pub mod translate;
mod card_options;
pub mod front;
pub mod shop;
pub mod prizes;
mod social;
mod mail;
pub mod reports;
pub mod problems;
pub mod sources;
mod teams;
mod trade;
mod deckcheck;
mod rental;
mod events;
pub mod championship;
pub mod official;
pub mod clock;
mod log;
mod net;
mod web;
pub mod ranking;
pub mod titles;
pub mod gifts;
pub use net::IDLE_TIMEOUT;

use crate::lock::Locked;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use yo3_proto::Value;

use accounts::Directory;
use front::{Deck, Hub, STARTER_REGULATION, World};

/// The official character decks every player gets: the client's NPC decks of Yami Yugi (NPCs 120-122: "Attack of the
/// Ancients", "Miracle Magic", "Slifer the Lightning"), Jaden Yuki (147-149) and Yusei Fudo (171-173), named after
/// their duelist. The deck list sends each once, in the room its 10 decks leave.
pub const OFFICIAL_DECKS: [(u32, &str); 9] = [
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
    /// Opt-in staff JSON API on its own loopback listener. None: disabled. Hosted HTTPS uses dashboard_socket.
    pub dashboard_port: Option<u16>,
    /// Staff API socket shared with nginx (Unix only); never mounts host-token routes.
    pub dashboard_socket: Option<PathBuf>,
    /// Exact browser origin for the socket proxy (HTTPS, or HTTP localhost for local checks).
    pub dashboard_origin: Option<String>,
    /// Whether staff must use an authenticator (`staff_mfa.rs`); required unless the host chooses the transition mode.
    pub staff_mfa: staff_mfa::StaffMfaPolicy,
    /// Backup downloads through the socket proxy: off unless the host enables them (admin, recent MFA, audited).
    pub dashboard_public_backups: bool,
    pub game_dir: PathBuf,
    /// Protocol-test fixtures only: permit an empty world when no game installation is available.
    /// Normal hosting always requires game data; this is not exposed as a CLI option.
    pub allow_missing_game_data: bool,
    pub log_dir: PathBuf,
    pub packet_logging: bool,
    /// Decoded packet traces in the session log (off: no player's chat or names there; the server file's `packet_log = false`).
    pub packet_traces: bool,
    pub diagnostic_id: Option<String>,
    /// Default players for offline account administration; login never creates an account.
    pub players: Vec<String>,
    /// Testing aid: each player's LP at the start of every duel (none: the `start-lp` setting, or a hosted duel's).
    pub duel_lp: Option<u32>,
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
    /// How long a connection may stay silent before its first message, or inside a frame, before it is closed.
    pub idle_timeout: std::time::Duration,
    /// Testing aid: pretend the rules engine can't run, with this reason (none: check for real at start).
    pub engine_problem: Option<String>,
    /// Testing aid: check every duel's client view against the rules engine after each step and log an ERROR where they
    /// differ (`duelcheck.rs`; the protocol monkey's duels run with it).
    pub duel_check: bool,
    /// Testing aid: live settings (#24) set at start, by name and value, as `yo3-server set` would.
    pub settings: Vec<(String, String)>,
    /// How often a running server checks its TLS certificate (`IDENTITY_CHECK`; shorter only to test): it renews its
    /// own identity near expiry and serves a certificate that changed on disk, without a restart (#25).
    pub identity_check: std::time::Duration,
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
            dashboard_port: None, dashboard_socket: None, dashboard_origin: None,
            staff_mfa: staff_mfa::StaffMfaPolicy::Required, dashboard_public_backups: false,
            game_dir: yo3_data::default_game_dir(),
            allow_missing_game_data: true,
            log_dir: log_dir.clone(),
            packet_logging: true, packet_traces: true, diagnostic_id: None,
            players: vec!["test".into()],
            duel_lp: None,
            duel_regulation: None,
            duel_deck: vec![],
            duel_extra: vec![],
            duel_time: None,
            data_dir: Some(log_dir.join("test-auth/data")),
            idle_timeout: net::IDLE_TIMEOUT,
            engine_problem: None,
            duel_check: false,
            settings: vec![("server-name".into(), "Test server".into()), ("registration".into(), "open".into())],
            identity_check: IDENTITY_CHECK,
        }
    }
}

/// How often a running server checks its certificate: renewing happens within 30 days of expiry, so a few checks a day
/// are plenty.
pub const IDENTITY_CHECK: std::time::Duration = std::time::Duration::from_secs(6 * 3600);

/// Renews the server's own identity near expiry (the same key and names: pinned launchers keep working) and, when the
/// certificate file changed (renewed here, or replaced by the host), a TLS acceptor for it. An identity it doesn't
/// manage (`--auth-cert` and `--auth-key` naming other files) is only reloaded, never renewed.
fn check_identity(tls: &auth::TlsConfig, served: &mut Vec<u8>) -> Option<yo3_tunnel::tokio_rustls::TlsAcceptor> {
    let dir = tls.certificate.parent()?;
    let own = tls.certificate.file_name()? == "server.crt" && tls.private_key == dir.join("server.key");
    if own { auth::renew_on_start(dir); }
    let now = std::fs::read(&tls.certificate).ok()?;
    if now == *served { return None }
    match yo3_tunnel::acceptor(&tls.certificate, &tls.private_key) {
        Ok(acceptor) => { *served = now; tracing::info!("serving the new certificate (launchers keep connecting: same key)"); Some(acceptor) }
        Err(e) => { tracing::error!("the changed certificate can't be used, the old one is still served: {e:#}"); None }
    }
}

/// Maintenance (#26): while one is planned, the game's countdown is pushed every minute (it shows the time left from
/// the last push); when it starts, everyone but the staff is disconnected and the duels between players end as draws.
/// New sign-ins are refused meanwhile (`account_ops`, `transport`), and discovery says so.
async fn maintenance(world: Arc<World>, hub: Arc<Mutex<Hub>>, directory: Arc<Mutex<Directory>>, sessions: Arc<Mutex<auth::Sessions>>) {
    let mut every = tokio::time::interval(std::time::Duration::from_secs(1));
    let (mut started, mut pushed): (Option<i64>, i64) = (None, 0);
    loop {
        every.tick().await;
        let settings = world.settings.now();
        let now = logins::now();
        let Some(at) = settings.maintenance_at else { started = None; continue };
        if now < at {
            if now - pushed >= 60 {
                pushed = now;
                hub.locked().push_everyone(&front::information(&settings, world.engine_problem.is_some(), now));
            }
            continue;
        }
        if started == Some(at) { continue }
        started = Some(at);
        tracing::info!("maintenance started: everyone but admins and moderators is disconnected");
        let tables: Vec<_> = hub.locked().tables.values().cloned().collect();
        for table in tables {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| table.locked().abort()));
        }
        let staff: std::collections::HashSet<u32> = {
            let d = directory.locked();
            d.entries().filter_map(|(user, _, a)| user.and_then(|u| d.login_of(u)).filter(|(_, l)| l.staff()).map(|_| a.user_id)).collect()
        };
        sessions.locked().logout_where(|user| !staff.contains(&user));
    }
}

/// Addresses the services actually bound to.
pub struct Running {
    pub auth: SocketAddr,
    pub http: SocketAddr,
    pub dashboard: Option<SocketAddr>,
    tasks: Vec<JoinHandle<()>>,
    shutdown: tokio::sync::watch::Sender<bool>,
    hub: Arc<Mutex<Hub>>,
    /// Backed up once more when the server stops.
    data_dir: Option<PathBuf>,
}

impl Running {
    /// The launcher's own server (Singleplayer) stops itself once its owner has gone (`owner_gone` resolves: the
    /// launcher closed or crashed) and nobody has been playing for `idle`. A game still open keeps it running.
    pub fn exit_when_idle_without_owner(&mut self, owner_gone: impl Future<Output = ()> + Send + 'static, idle: std::time::Duration) {
        let (hub, shutdown) = (self.hub.clone(), self.shutdown.clone());
        self.tasks.push(tokio::spawn(async move {
            owner_gone.await;
            tracing::info!("the launcher that owns this server has gone; stopping once nobody plays for {}s", idle.as_secs());
            let step = idle.min(std::time::Duration::from_secs(5)).max(std::time::Duration::from_millis(50));
            let mut quiet_since = std::time::Instant::now();
            loop {
                tokio::time::sleep(step).await;
                let busy = { let hub = hub.locked(); hub.online() > 0 || !hub.tables.is_empty() };
                if busy { quiet_since = std::time::Instant::now(); continue }
                if quiet_since.elapsed() >= idle {
                    tracing::info!("idle with no owner: stopping");
                    shutdown.send_replace(true);
                    break;
                }
            }
        }));
    }

    /// Stops the server as `yo3-server stop` does (a signal handler's way in).
    pub fn stopper(&self) -> tokio::sync::watch::Sender<bool> { self.shutdown.clone() }

    /// Runs until asked to stop, then stops gracefully (#25): no new logins or connections, the duels between players
    /// end as draws (each recorded, both clients told), and a last backup. Every change is saved as it happens, so
    /// there is nothing else to write. (The game's maintenance countdown before a stop comes with #26.)
    pub async fn wait(mut self) {
        let mut stop = self.shutdown.subscribe();
        while !*stop.borrow_and_update() {
            if stop.changed().await.is_err() { break }
        }
        tracing::info!("stopping: no new logins");
        // Let the stop request's answer reach its caller before the listeners go.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for task in self.tasks.drain(..) { task.abort(); }
        // The table locks the hub to record its result: collected first, ended outside the hub's lock.
        let tables: Vec<_> = self.hub.locked().tables.values().cloned().collect();
        if !tables.is_empty() { tracing::info!("stopping: ending {} duel(s) as draws", tables.len()) }
        for table in tables {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| table.locked().abort()));
        }
        // The clients' last messages (the duels' end) leave before the process does.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if let Some(dir) = self.data_dir.take() {
            match tokio::task::spawn_blocking(move || store::back_up(&dir)).await {
                Ok(Ok(())) => tracing::info!("backed up at shutdown"),
                Ok(Err(e)) => tracing::error!("backup at shutdown failed: {e}"),
                Err(e) => tracing::error!("backup at shutdown failed: {e}"),
            }
        }
    }
}

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

/// Tokens the rules engine names differently from the client's token card: (EDOPro's name, the client's).
/// Sweet Corn's token is "Maize Token" in EDOPro's database and "Grain Corn Token" in the client's (its text).
pub const TOKEN_ALIASES: [(&str, &str); 1] = [("Maize Token", "Grain Corn Token")];

/// The client's token cards (Konami ids 3900-3999) by name, as the rules engine names them: the translator finds a
/// token without a Konami id by its name (`Translator::tokens`).
pub fn token_rows(cards: &yo3_data::CardDb) -> std::collections::HashMap<String, Vec<u16>> {
    let mut rows = std::collections::HashMap::new();
    for c in cards.iter().filter(|c| (3900..4000).contains(&c.cid)) {
        rows.entry(c.name.clone()).or_insert_with(Vec::new).push(c.row);
    }
    for (engine, client) in TOKEN_ALIASES {
        if let Some(found) = rows.get(client).cloned() {
            rows.insert(engine.to_string(), found);
        }
    }
    rows
}

/// Where each card comes from, for `/where` (#55): the booster packs, structure decks, Trade Card pools, the shop's
/// rental and the decks every player starts with.
fn card_sources(data: &yo3_data::GameData, with_extras: bool) -> Vec<sources::CardSource> {
    use sources::{CardSource, Kind};
    let cid = |row: &u16| data.cards.by_row(*row).map(|c| c.cid);
    let of = |rows: &mut dyn Iterator<Item = u16>| -> std::collections::HashSet<u16> { rows.filter_map(|r| cid(&r)).collect() };
    let mut all = vec![];
    for p in data.packs.iter().filter(|p| !p.cards.is_empty()) {
        let name = p.record.get("name_id").and_then(Value::as_u32).and_then(|id| data.text(id)).filter(|n| !n.is_empty());
        let (kind, name) = match (p.kind, name) {
            (1, Some(name)) => (Kind::Pack, name.to_string()),
            (1, None) => (Kind::Pack, format!("Booster Pack {}", p.id)),
            _ if shop::EXCHANGE_POOLS.contains(&p.id) => (Kind::TradePool, sources::trade_pool_name(p.id)),
            _ if with_extras && p.id == shop::EXTRA_POOL => (Kind::TradePool, sources::trade_pool_name(p.id)),
            _ if p.id == shop::RENTAL_POOL => (Kind::Rental, "Blister's Rental (the shop lends it)".to_string()),
            _ => continue,
        };
        all.push(CardSource { kind, name, cards: of(&mut p.cards.iter().copied()) });
    }
    // Structure decks are the shop's (deck_type 1 with art of their own); the starter and the characters' decks are free.
    for d in data.decks.iter().filter(|d| d.kind == 1 && d.record.get("deck_image").and_then(Value::as_str).is_some_and(|i| i != "noImage")) {
        let name = d.name_id().and_then(|id| data.text(id)).unwrap_or("Structure Deck").to_string();
        all.push(CardSource { kind: Kind::StructureDeck, name, cards: of(&mut d.cards()) });
    }
    if with_extras {
        for d in data.decks.iter().filter(|d| recipe_deck(data, d)) {
            let name = d.name_id().and_then(|id| data.text(id)).unwrap_or("Theme Deck").to_string();
            let name = format!("{name} (a deck recipe: the recipes command)");
            let kept = d.cards().filter(|r| data.cards.by_row(*r).is_none_or(|c| !prizes::UNOBTAINABLE.contains(&c.name.as_str())));
            all.push(CardSource { kind: Kind::StructureDeck, name, cards: of(&mut kept.collect::<Vec<_>>().into_iter()) });
        }
    }
    if let Some(d) = data.starter_deck(STARTER_REGULATION) {
        all.push(CardSource { kind: Kind::FreeDeck, name: "the Starter Deck (every player starts with it)".into(), cards: of(&mut d.cards()) });
    }
    for (npc, who) in OFFICIAL_DECKS {
        if let Some(d) = data.npc_deck(npc) {
            all.push(CardSource { kind: Kind::FreeDeck, name: format!("{who}'s deck (every player owns its cards)"), cards: of(&mut d.cards()) });
        }
    }
    all
}

/// Whether a preset deck has box art of its own (a structure deck).
fn has_art(d: &yo3_data::Deck) -> bool {
    d.record.get("deck_image").and_then(Value::as_str).is_some_and(|i| i != "noImage")
}

/// Whether a preset deck is sold as a recipe with its cards by the `recipes` chat command (#19): the 22 theme decks
/// (deck_type 2), the starter recipes besides the free one (deck_type 1 without art) and the 57 NPC decks (deck_type 3)
/// of NPCs the client's roster doesn't list, so no win can give them. The shop's own shelf can't show them: a deck
/// without `deck_image` is an invisible slot there (checked live), so only the structure decks go on it.
pub fn recipe_deck(data: &yo3_data::GameData, d: &yo3_data::Deck) -> bool {
    match d.kind {
        1 if has_art(d) => false,
        1 => data.starter_deck(STARTER_REGULATION).is_some_and(|free| free.id != d.id) && d.pbin_no().is_some(),
        2 => d.pbin_no().is_some(),
        3 => d.pbin_no().is_some() && !data.npc_ids().into_iter().any(|id| data.npc_deck(id).is_some_and(|n| n.id == d.id)),
        _ => false,
    }
}

/// The shop's structure decks (deck_type 1 with box art: SD001-SD005).
fn shelf_deck(d: &yo3_data::Deck) -> bool {
    d.kind == 1 && has_art(d)
}

/// A deck the recipes command sells: its cards without the five that stay unobtainable (`prizes::UNOBTAINABLE`; an
/// unlisted NPC's deck holds Obelisk the Tormentor). The recipe itself still lists them, as the client's file does.
fn sold_deck(data: &yo3_data::GameData, d: &yo3_data::Deck) -> shop::ShopDeck {
    let mut sold = shop_deck(data, d);
    let keep = |row: &u16| data.cards.by_row(*row).is_none_or(|c| !prizes::UNOBTAINABLE.contains(&c.name.as_str()));
    for list in [&mut sold.deck.main, &mut sold.deck.extra, &mut sold.deck.side] { list.retain(keep) }
    sold
}

fn shop_deck(data: &yo3_data::GameData, d: &yo3_data::Deck) -> shop::ShopDeck {
    let date = |k: &str| d.record.get(k).and_then(Value::as_str).unwrap_or_default();
    let name = d.name_id().and_then(|id| data.text(id)).unwrap_or("Structure Deck").to_string();
    let deck = Deck { id: d.id, name, main: d.main.clone(), extra: d.extra.clone(), side: d.side.clone() };
    shop::ShopDeck { released: date("created"), modified: date("modified"), deck, item: d.pbin_no() }
}

/// The names of the cards a player can keep through this server's routes besides the Special Pack (the booster packs,
/// structure decks, Trade Card pools, the decks the recipes command sells and the free decks), for the tests.
pub fn keepable_card_names(data: &yo3_data::GameData) -> std::collections::HashSet<String> {
    let cids: std::collections::HashSet<u16> = card_sources(data, true).iter().filter(|s| s.kind != sources::Kind::Rental).flat_map(|s| s.cards.iter().copied()).collect();
    data.cards.iter().filter(|c| cids.contains(&c.cid)).map(|c| c.name.clone()).collect()
}

/// The Special Pack's cards, the alternate artworks and the NPCs' recipes from the client's files, for the tests.
pub fn game_prizes(data: &yo3_data::GameData) -> prizes::Prizes {
    let items: Vec<u16> = data.starter_deck(STARTER_REGULATION).into_iter()
        .chain(OFFICIAL_DECKS.iter().filter_map(|&(npc, _)| data.npc_deck(npc))).filter_map(yo3_data::Deck::pbin_no).collect();
    prizes(data, &items)
}

/// The Special Pack's cards, the alternate artworks and the NPCs' recipes (`prizes::Prizes`), from the client's files.
fn prizes(data: &yo3_data::GameData, official_items: &[u16]) -> prizes::Prizes {
    use std::collections::HashSet;
    // Where a card had a route in the original: the sources without the theme decks, which this server sells too.
    let routed: HashSet<u16> = card_sources(data, false).iter().flat_map(|s| s.cards.iter().copied()).collect();
    let mut special_rows = prizes::special_cards(data.cards.iter(), &routed);
    // The rental-only cards no route of this server lets a player keep (Blister's extra pool, the recipes command, the
    // NPCs' recipes give only the recipe): in the Special Pack too, so every card but the five that stay out can be kept.
    let kept: HashSet<u16> = card_sources(data, true).iter().filter(|s| s.kind != sources::Kind::Rental).flat_map(|s| s.cards.iter().copied()).collect();
    let mut rental_only = prizes::special_cards(data.cards.iter(), &kept);
    rental_only.retain(|c| !special_rows.iter().any(|s| s.cid == c.cid));
    special_rows.extend(rental_only);
    special_rows.sort_by_key(|c| c.row);
    // The rows the packs and the free and structure decks give: any other artwork of their cards is an alternate.
    let mut routed_rows: HashSet<u16> = data.packs.iter().filter(|p| p.kind == 1).flat_map(|p| p.cards.iter().copied()).collect();
    routed_rows.extend(data.starter_deck(STARTER_REGULATION).into_iter().flat_map(|d| d.cards()));
    routed_rows.extend(OFFICIAL_DECKS.iter().filter_map(|&(npc, _)| data.npc_deck(npc)).flat_map(|d| d.cards()));
    routed_rows.extend(data.decks.iter().filter(|d| d.kind == 1 && has_art(d)).flat_map(|d| d.cards()));
    let odds = std::env::var("YO3_ALT_ART_ODDS").ok().and_then(|v| v.parse().ok()).unwrap_or(shop::ALT_ART_ODDS);
    let npc_recipes = data.npc_ids().into_iter().filter_map(|id| {
        let deck = data.npc_deck(id)?;
        let item = deck.pbin_no().filter(|i| !official_items.contains(i))?;
        let title = data.npc_deck_title(id).unwrap_or_default().trim().to_string();
        Some((id, prizes::NpcRecipe { item, npc: data.npc_name(id)?.to_string(), deck: title }))
    }).collect();
    prizes::Prizes {
        special: prizes::special_tiers(&special_rows),
        arts: shop::Arts { alts: prizes::alternate_arts(data.cards.iter(), &routed_rows), odds },
        npc_recipes,
    }
}

fn world(cfg: &Config, duel_port: u16, http_port: u16) -> anyhow::Result<World> {
    let duel_server = (cfg.advertise.clone().unwrap_or_default(), duel_port);
    let world = match yo3_data::GameData::load(&cfg.game_dir) {
        Ok(data) => {
            let official_items: Vec<u16> = data.starter_deck(STARTER_REGULATION).into_iter()
                .chain(OFFICIAL_DECKS.iter().filter_map(|&(npc, _)| data.npc_deck(npc))).filter_map(yo3_data::Deck::pbin_no).collect();
            let mut world = World {
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
            // Every player's Official tab in Load Deck: the starter preset's recipe and the official decks' (their
            // pbin_no items, catalogue kinds 5 and 8).
            official_items: official_items.clone(),
            cards: data
                .cards
                .iter()
                .map(|c| {
                    let stats = CardStats { cid: c.cid, atk: c.atk, def: c.def, level: c.level, attribute: c.attribute, race: c.race };
                    (c.row, stats)
                })
                .collect(),
            tuners: data.cards.iter().filter(|c| c.tuner()).map(|c| c.cid).collect(),
            card_kinds: data.cards.iter().map(|c| (c.cid, c.kind)).collect(),
            titles: Arc::new(titles::TitleData::from_game(&data)),
            token_rows: token_rows(&data.cards),
            card_names: data.cards.iter().fold(std::collections::HashMap::new(), |mut m, c| {
                m.entry(c.cid).or_insert_with(|| c.name.clone());
                m
            }),
            extra_deck_rows: data.cards.iter().filter(|c| c.extra_deck()).map(|c| c.row).collect(),
            engine_dir: engine::Engine::dir(),
            test_lp: cfg.duel_lp,
            test_regulation: cfg.duel_regulation,
            test_deck: (!cfg.duel_deck.is_empty()).then(|| cfg.duel_deck.clone()),
            test_extra: (!cfg.duel_extra.is_empty()).then(|| cfg.duel_extra.clone()),
            test_times: cfg.duel_time,
            duel_check: cfg.duel_check,
            npcs: data.npc_ids(),
            npc_names: data.npc_ids().into_iter().filter_map(|id| Some((id, data.npc_name(id)?.to_string()))).collect(),
            npc_styles: data.npc_ids().into_iter().map(|id| (id, npc::style(data.npc_name(id).unwrap_or_default()))).collect(),
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
            structure_decks: data.decks.iter().filter(|d| shelf_deck(d)).map(|d| shop_deck(&data, d)).collect(),
            recipe_decks: data.decks.iter().filter(|d| recipe_deck(&data, d)).map(|d| sold_deck(&data, d)).collect(),
            exchange_pools: data
                .packs
                .iter()
                .filter(|p| (shop::EXCHANGE_POOLS.contains(&p.id) || p.id == shop::EXTRA_POOL) && !p.cards.is_empty())
                .map(|p| {
                    let date = |k: &str| p.record.get(k).and_then(Value::as_str).unwrap_or_default();
                    shop::ExchangePool { id: p.id, released: date("created"), modified: date("modified"), cards: p.cards.clone() }
                })
                .collect(),
            rental_pool: data.packs.iter().filter(|p| p.id == shop::RENTAL_POOL).flat_map(|p| p.cards.iter().copied()).collect(),
            protectors: data.protectors().iter().map(|e| e.id).collect(),
            avatar_items: data.avatar_items_for_sale().iter().map(|e| (e.id, e.b)).collect(),
            free_items: data.free_items().iter().map(|e| e.id).collect(),
            banlists: data.regulations.iter().map(|r| (r.id, data.text(r.name_id).unwrap_or_default().to_string())).collect(),
            regulations: data.regulations.iter().map(|r| (r.id, r.clone())).collect(),
            card_sources: card_sources(&data, true),
            prizes: Arc::new(prizes(&data, &official_items)),
            ..World::new(data.regulations.iter().map(|r| r.id as u16).collect())
        };
            // The card profiles (#60), over the game's own cards.
            let base = profile::CardView { cards: world.cards.clone(), card_kinds: world.card_kinds.clone(),
                extra_deck_rows: world.extra_deck_rows.clone(), tuners: world.tuners.clone(), ..Default::default() };
            world.profiles = profile::Profiles::load(&base, data.cards.iter(), world.engine_dir.as_deref());
            world
        }
        Err(e) => {
            if !cfg.allow_missing_game_data {
                anyhow::bail!("Cannot load the host's game data from {}: {e}. Select a complete Yu-Gi-Oh! ONLINE 3 v1.153 installation with --game-dir (Desktop: Game setup). Hosting was not started; existing accounts and decks were preserved.", cfg.game_dir.display());
            }
            tracing::warn!("game data not loaded ({e}); using regulation ids 1-20");
            World { duel_server, http_port, ..World::new((1..=20).collect()) }
        }
    };
    if !cfg.allow_missing_game_data && (world.cards.is_empty() || (world.starter_deck.is_none() && world.official_decks.is_empty())) {
        anyhow::bail!("The host's game data has no usable cards or starting decks. Select a complete Yu-Gi-Oh! ONLINE 3 v1.153 installation with --game-dir (Desktop: Game setup). Hosting was not started; existing accounts and decks were preserved.");
    }
    Ok(world)
}

/// How long a user event waits between two of its rounds: the winners get through the result screens to the lobby.
const EVENT_ROUND_DELAY: std::time::Duration = std::time::Duration::from_secs(20);
/// How long a Championship's entry draw shows "Selecting players for entry..." before the lots are drawn (**guess**).
const CHAMPIONSHIP_DRAW_DELAY: std::time::Duration = std::time::Duration::from_secs(8);

pub async fn start(cfg: Config) -> anyhow::Result<Running> {
    anyhow::ensure!(cfg.dashboard_socket.is_none() || cfg.dashboard_port.is_none(), "Choose a dashboard socket or port, not both");
    anyhow::ensure!(cfg.dashboard_socket.is_some() == cfg.dashboard_origin.is_some(), "dashboard-socket and dashboard-origin must be set together");
    let proxy_origin = cfg.dashboard_origin.as_deref().map(dashboard::ProxyOrigin::parse).transpose()?.map(|o| o.public_backups(cfg.dashboard_public_backups));
    #[cfg(not(unix))] anyhow::ensure!(cfg.dashboard_socket.is_none(), "dashboard-socket requires Unix; use dashboard-port on Windows");
    #[cfg(unix)] let dashboard_socket = cfg.dashboard_socket.as_deref().map(dashboard_socket::Bound::bind).transpose()?;
    let packets = Arc::new(if cfg.packet_logging { log::PacketLog::create(&cfg.log_dir)? } else { log::PacketLog::disabled()? }.with_traces(cfg.packet_traces));
    let mut served = std::fs::read(&cfg.tls.certificate)?;
    let tls = Arc::new(std::sync::RwLock::new(yo3_tunnel::acceptor(&cfg.tls.certificate, &cfg.tls.private_key)?));
    let authenticated = TcpListener::bind((cfg.bind, cfg.tls.port)).await?;
    let http = TcpListener::bind((Ipv4Addr::LOCALHOST, cfg.http_port)).await?;
    // Bind all requested listeners before opening/migrating storage or starting background tasks. The game's bind
    // address never changes management TCP. Hosted HTTPS uses a separate filesystem-only staff socket.
    let dashboard = match cfg.dashboard_port {
        Some(port) => Some(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await
            .map_err(|e| anyhow::anyhow!("Cannot bind dashboard API at 127.0.0.1:{port}: {e}"))?),
        None => None,
    };
    let dashboard_addr = dashboard.as_ref().map(TcpListener::local_addr).transpose()?;
    let (auth_addr, http_addr) = (authenticated.local_addr()?, http.local_addr()?);
    // The launcher supplies its local duel port for each front connection's handoff. The game's data comes first: a
    // server that can't load it stops before it opens (or makes) the database.
    let mut world = world(&cfg, 0, http_addr.port())?;
    if let Some(dir) = &cfg.data_dir {
        store::initialize(dir)?;
        store::interrupt_effects(dir)?;
        store::back_up(dir)?;
    }
    // The live settings (#24), saved in the database, read with the game's banlist names.
    let settings = settings::Live::open(cfg.data_dir.as_ref().map(|d| d.join("settings.json")).as_deref(), world.banlists.clone())?;
    for (name, value) in &cfg.settings {
        settings.set(name, value).map_err(anyhow::Error::msg)?;
    }
    world.settings = Arc::new(settings);
    // The audit log keeps `audit-keep-days` (#39): pruned now and then daily.
    let audit_task = cfg.data_dir.clone().map(|dir| {
        let settings = world.settings.clone();
        tokio::spawn(async move {
            let day = std::time::Duration::from_secs(24 * 3600);
            let mut every = tokio::time::interval_at(tokio::time::Instant::now() + day, day);
            loop {
                every.tick().await;
                let (dir, days) = (dir.clone(), settings.now().audit_keep_days);
                match tokio::task::spawn_blocking(move || store::prune_audit(&dir, days)).await {
                    Ok(Ok(n)) if n > 0 => tracing::info!("audit log: {n} row(s) older than {days} days removed (audit-keep-days)"),
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => tracing::error!("audit log not pruned: {e}"),
                    Err(e) => tracing::error!("audit log not pruned: {e}"),
                }
            }
        })
    });
    if let Some(dir) = &cfg.data_dir {
        let n = store::prune_audit(dir, world.settings.now().audit_keep_days)?;
        if n > 0 { tracing::info!("audit log: {n} row(s) older than audit-keep-days removed") }
    }
    tracing::info!("players log in with regulation {}", world.settings.now().login_regulation);
    world.engine_problem = cfg.engine_problem.clone().or_else(|| engine::problem(world.engine_dir.as_deref(), &engine::program(world.engine_dir.as_deref().unwrap_or(std::path::Path::new("engine")))));
    if cfg.duel_check {
        tracing::info!("duel check on: every duel's client view is compared with the rules engine after each step");
    }
    let world = Arc::new(world);
    match (&world.engine_problem, &world.engine_dir) {
        (Some(why), _) => tracing::error!("DUELS WON'T WORK: {why}. Duels will end at once as draws; players are told in the news."),
        (None, Some(dir)) => tracing::info!("rules engine: {}", dir.display()),
        (None, None) => {}
    }

    let idle = net::Idle { limit: cfg.idle_timeout };
    // A backup every hour while running (and one at each start and stop).
    let backup_task = cfg.data_dir.clone().map(|dir| tokio::spawn(async move {
        let mut every = tokio::time::interval_at(tokio::time::Instant::now() + store::BACKUP_EVERY, store::BACKUP_EVERY);
        loop {
            every.tick().await;
            let dir = dir.clone();
            match tokio::task::spawn_blocking(move || store::back_up(&dir)).await {
                Ok(Ok(())) => tracing::info!("hourly backup taken"),
                Ok(Err(e)) => tracing::error!("hourly backup failed: {e}"),
                Err(e) => tracing::error!("hourly backup failed: {e}"),
            }
        }
    }));
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
        store::report_leftovers(dir);
        hub.keep_social(&dir.join("friends.json"));
        hub.keep_mail(&dir.join("mail.json"));
        hub.keep_teams(&dir.join("teams.json"));
        hub.keep_events(&dir.join("events.json"));
        hub.keep_official(dir);
        hub.keep_championships(dir);
        hub.keep_rentals(&dir.join("rentals.json"));
        hub.keep_duel_logs(dir, http_addr.port());
        hub.reports = reports::Reports::at(dir);
    }
    hub.set_round_delay(EVENT_ROUND_DELAY);
    hub.set_draw_delay(CHAMPIONSHIP_DRAW_DELAY);
    hub.set_directory(directory.clone());
    hub.set_titles(world.titles.clone());
    hub.set_prizes(world.prizes.clone());
    hub.set_settings(world.settings.clone());
    let hub = Arc::new(Mutex::new(hub));
    // The certificate is checked while running too, not only at start (#25).
    let identity_task = {
        let (tls, config, every) = (tls.clone(), cfg.tls.clone(), cfg.identity_check);
        tokio::spawn(async move {
            let mut ticks = tokio::time::interval_at(tokio::time::Instant::now() + every, every);
            loop {
                ticks.tick().await;
                let (config, mut known) = (config.clone(), std::mem::take(&mut served));
                let checked = tokio::task::spawn_blocking(move || { let next = check_identity(&config, &mut known); (next, known) }).await;
                let Ok((next, known)) = checked else { continue };
                served = known;
                if let Some(next) = next { *tls.write().unwrap_or_else(|e| e.into_inner()) = next; }
            }
        })
    };
    // The signed-in sessions: the TLS gate makes them, the admin API ends them (a kick, a ban, maintenance).
    let sessions = Arc::new(Mutex::new(auth::Sessions::default()));
    let auth_task = tokio::spawn(transport::accept_loop(authenticated, tls, world.clone(), hub.clone(), directory.clone(),
        packets.clone(), idle, http_addr, sessions.clone(), Arc::new(problems::Problems::new(cfg.data_dir.as_deref()))));
    let maintenance_task = tokio::spawn(maintenance(world.clone(), hub.clone(), directory.clone(), sessions.clone()));
    // Duel clocks between players run out on the server's side: checked once a second.
    let clock_task = {
        let (hub, world) = (hub.clone(), world.clone());
        tokio::spawn(async move {
            let mut every = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                every.tick().await;
                // On a blocking thread (a table's engine call may take a while), and a panic in one pass is logged,
                // not the end of the clock.
                let (hub, world) = (hub.clone(), world.clone());
                let pass = tokio::task::spawn_blocking(move || {
                    duel::tick_tables(&hub, std::time::Instant::now());
                    let mut hub = hub.locked();
                    hub.tick_matches(std::time::Instant::now());
                    hub.match_waiting();
                    hub.match_computers(&world);
                    hub.tick_events(std::time::Instant::now());
                    hub.tick_official(chrono::Utc::now().timestamp());
                    hub.tick_championships(chrono::Utc::now().timestamp(), std::time::Instant::now());
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
    let running_hub = hub.clone();
    let mut api = api::Api::new(directory.clone(), world.settings.clone(), cfg.data_dir.clone(), cfg.game_dir.clone(), "server");
    api.hub = Some(hub);
    api.sessions = Some(sessions);
    api.shutdown = Some(shutdown.clone());
    api.status = api::Status { engine_problem: world.engine_problem.clone(), listen: Some(auth_addr), diagnostic_id: cfg.diagnostic_id, started: Some(std::time::Instant::now()) };
    api.titles = Some(world.titles.clone());
    api.world = Some(world.clone());
    let api = Arc::new(api);
    let panel = Arc::new(panel::Panel::with_policy(cfg.staff_mfa));
    let dashboard_task = dashboard.map(|listener| tokio::spawn(net::dashboard_loop(listener, dashboard::Dashboard { api: api.clone(), panel: panel.clone() })));
    #[cfg(unix)] let socket_task = dashboard_socket.map(|listener| tokio::spawn(dashboard_socket::serve(listener,
        dashboard::Dashboard { api: api.clone(), panel: panel.clone() }, proxy_origin.expect("validated socket origin"))));
    #[cfg(not(unix))] let _ = proxy_origin;
    let http_task = tokio::spawn(net::http_loop(http, cfg.game_dir.clone(), cfg.data_dir.clone(), admin_token, api, panel));
    tracing::info!("authenticated TLS {} local management/assets HTTP {}; packets -> {}", auth_addr, http_addr, packets.path().display());
    if let Some(addr) = dashboard_addr { tracing::info!("staff dashboard API {addr} (loopback only; no host-token authority)"); }
    let mut tasks = vec![auth_task, http_task, clock_task, identity_task, maintenance_task];
    tasks.extend(dashboard_task);
    #[cfg(unix)] tasks.extend(socket_task);
    tasks.extend(backup_task);
    tasks.extend(audit_task);
    Ok(Running { auth: auth_addr, http: http_addr, dashboard: dashboard_addr, tasks, shutdown, hub: running_hub, data_dir: cfg.data_dir.clone() })
}
