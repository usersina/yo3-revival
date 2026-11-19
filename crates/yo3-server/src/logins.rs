//! Accounts with a User ID and a password (#23): what an account's login holds, how passwords and sign-in tokens are
//! kept, invite codes, recovery codes, and the throttling in front of all of it.
//!
//! - Passwords: argon2id (m = 64 MiB, t = 3, p = 1), stored as PHC strings; at most [`HASHING_AT_ONCE`] hashes run at
//!   a time, so a flood of logins can't exhaust memory.
//! - Sign-in tokens, recovery and reset codes are random and long, so they are kept as SHA-256 digests.
//! - One error text for an unknown User ID and a wrong password. No password, token or code is ever logged.
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// The longest User ID or password the game's own login form takes (`login.swf` sprite 67, #34).
pub const FORM_MAX: usize = 16;

/// What the game's login form lets a player type: letters and digits (`restrict` "A-Z a-z 0-9", #34).
fn form_typeable(text: &str) -> bool { text.bytes().all(|b| b.is_ascii_alphanumeric()) }

/// What a User ID may be: 3–16 letters or digits, as the game's login form takes them; unique ignoring case.
pub fn valid_user_id(user: &str) -> bool { (3..=FORM_MAX).contains(&user.len()) && form_typeable(user) }

/// Why a password can't be used, if it can't: 8–16 letters or digits (the game's login form takes no others), not a
/// common one, not the User ID.
pub fn password_problem(user: &str, password: &str) -> Option<&'static str> {
    let len = password.chars().count();
    if len < 8 { return Some("password_too_short") }
    if len > FORM_MAX { return Some("password_too_long") }
    if !form_typeable(password) { return Some("password_characters") }
    let lower = password.to_lowercase();
    if lower == user.to_lowercase() || COMMON.iter().any(|c| *c == lower) { return Some("password_too_common") }
    None
}

/// A short list of the most common passwords (no composition rules: length and this list do more good).
const COMMON: &[&str] = &["12345678", "123456789", "1234567890", "password", "password1", "password123", "qwertyui",
    "qwerty123", "qwertyuiop", "11111111", "00000000", "iloveyou", "abc12345", "abcd1234", "1q2w3e4r", "1qaz2wsx",
    "letmein1", "sunshine", "princess", "football", "baseball", "welcome1", "admin123", "passw0rd", "p@ssw0rd",
    "monkey12", "dragon12", "yugioh12", "yugiohyugioh", "blueeyes", "exodia12", "darkmagician", "87654321", "asdfghjk",
    "zxcvbnm1", "trustno1", "superman", "starwars", "whatever", "computer", "internet", "michael1", "shadow12"];

static FAST_HASHING: AtomicBool = AtomicBool::new(false);

/// For tests: the cheapest argon2id parameters, so a test suite doesn't spend minutes hashing.
#[doc(hidden)]
pub fn use_fast_hashing() { FAST_HASHING.store(true, Ordering::Relaxed) }

/// How many password hashes may run at once.
pub const HASHING_AT_ONCE: usize = 2;
static HASHING: LazyLock<tokio::sync::Semaphore> = LazyLock::new(|| tokio::sync::Semaphore::new(HASHING_AT_ONCE));

static RUNNING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static PEAK: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// For tests: the most password hashes that ran at the same time.
#[doc(hidden)]
pub fn hashing_peak() -> usize { PEAK.load(Ordering::Relaxed) }

/// Runs one hash, counted for [`hashing_peak`].
fn counted<T>(hash: impl FnOnce() -> T) -> T {
    let now = RUNNING.fetch_add(1, Ordering::SeqCst) + 1;
    PEAK.fetch_max(now, Ordering::SeqCst);
    let out = hash();
    RUNNING.fetch_sub(1, Ordering::SeqCst);
    out
}

fn argon2() -> argon2::Argon2<'static> {
    let params = if FAST_HASHING.load(Ordering::Relaxed) { argon2::Params::new(8, 1, 1, None) } else { argon2::Params::new(64 * 1024, 3, 1, None) };
    argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params.expect("valid argon2 parameters"))
}

/// For tests (fast hashing only): the digests of the passwords hashed, so a test can tell a request hashed none.
static HASHED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// For tests: whether `password` was hashed since the process started (only recorded with [`use_fast_hashing`]).
#[doc(hidden)]
pub fn was_hashed(password: &str) -> bool { HASHED.lock().unwrap_or_else(|e| e.into_inner()).contains(&digest(password)) }

/// Hashes a password (off the async threads, at most [`HASHING_AT_ONCE`] at a time).
pub async fn hash_password(password: String) -> anyhow::Result<String> {
    use argon2::PasswordHasher;
    if FAST_HASHING.load(Ordering::Relaxed) { HASHED.lock().unwrap_or_else(|e| e.into_inner()).push(digest(&password)) }
    let turn = HASHING.acquire().await?;
    // The turn goes with the hash: a request that gives up waiting (the 10 s handshake deadline) can't free its slot
    // while its hash still runs (#39).
    tokio::task::spawn_blocking(move || { let _turn = turn; counted(|| argon2().hash_password(password.as_bytes()).map(|h| h.to_string()).map_err(|e| anyhow::anyhow!("{e}"))) }).await?
}

/// Hashes a password on this thread (the admin commands, outside the server's async threads).
pub fn hash_password_now(password: &str) -> anyhow::Result<String> {
    use argon2::PasswordHasher;
    argon2().hash_password(password.as_bytes()).map(|h| h.to_string()).map_err(|e| anyhow::anyhow!("{e}"))
}

/// A password the host hands to a player for an account it made (they change it in the launcher): 16 letters and digits.
pub fn new_password() -> String { normalize_code(&new_code()).to_lowercase() }

/// Whether `password` matches the PHC string `hash` (a missing hash is checked against a dummy one, so an unknown
/// User ID takes as long as a wrong password).
pub async fn verify_password(password: String, hash: Option<String>) -> bool {
    use argon2::PasswordVerifier;
    let Ok(turn) = HASHING.acquire().await else { return false };
    tokio::task::spawn_blocking(move || {
        let _turn = turn;
        let (hash, real) = match hash { Some(h) => (h, true), None => (DUMMY.clone(), false) };
        counted(|| argon2().verify_password(password.as_bytes(), &*hash).is_ok()) && real
    }).await.unwrap_or(false)
}
static DUMMY: LazyLock<String> = LazyLock::new(|| {
    use argon2::PasswordHasher;
    argon2().hash_password(b"not a password anyone has").map(|h| h.to_string()).unwrap_or_default()
});

/// A random token: 32 bytes as 64 hex digits.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS randomness unavailable");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A code a person types: 16 letters and digits without look-alikes, in groups of four (`ABCD-EFGH-JKLM-NPQR`).
pub fn new_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut out = String::new();
    while out.chars().filter(|c| *c != '-').count() < 16 {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).expect("OS randomness unavailable");
        for b in bytes {
            if (b as usize) < 256 / ALPHABET.len() * ALPHABET.len() && out.chars().filter(|c| *c != '-').count() < 16 {
                if !out.is_empty() && out.chars().filter(|c| *c != '-').count() % 4 == 0 { out.push('-') }
                out.push(ALPHABET[b as usize % ALPHABET.len()] as char);
            }
        }
    }
    out
}

/// What a typed code is compared as: letters and digits only, upper case.
pub fn normalize_code(code: &str) -> String { code.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect() }

/// The SHA-256 digest a token or code is kept as.
pub fn digest(secret: &str) -> String {
    ring::digest::digest(&ring::digest::SHA256, secret.as_bytes()).as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Seconds since the epoch.
pub fn now() -> i64 { chrono::Utc::now().timestamp() }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role { #[default] Player, Moderator, Admin }

/// An account's standing. A restriction or ban with an end time is over once that time has passed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Standing {
    #[default]
    Active,
    Restricted { reason: String, until: Option<i64> },
    Banned { reason: String, until: Option<i64> },
}

/// One "stay signed in": a device's token (as a digest) and when it was made and last used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device { pub token: String, pub label: String, pub created: i64, pub last_used: i64, pub expires: i64 }

/// How long a device stays signed in without being used; each use moves it on.
pub const DEVICE_TTL: i64 = 90 * 24 * 3600;
/// How many devices an account keeps signed in; the oldest goes first.
pub const MAX_DEVICES: usize = 10;
/// How long a moderator's reset code works.
pub const RESET_TTL: i64 = 24 * 3600;

/// What a password account has besides its players: its User ID, password, role, standing, recovery code and devices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Login {
    /// The User ID as typed at sign-up.
    pub user: String,
    /// The password, as a PHC string.
    pub password: String,
    #[serde(default)]
    pub role: Role,
    /// Permanently revokes staff cookies when a role changes, even across a later promotion back.
    #[serde(default)]
    pub staff_epoch: u64,
    #[serde(default)]
    pub standing: Standing,
    /// The one-time recovery code's digest (a new one is made each time it is used).
    pub recovery: String,
    /// A moderator's reset code: its digest and when it stops working.
    #[serde(default)]
    pub reset: Option<(String, i64)>,
    #[serde(default)]
    pub devices: Vec<Device>,
    /// A moderator's mute (#26): the player's chat lines go nowhere until it ends.
    #[serde(default)]
    pub muted: Option<Mute>,
    /// The staff dashboard's authenticator (TOTP) and recovery codes (`staff_mfa.rs`); none until enrolled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mfa: Option<StaffMfa>,
}

/// A staff account's second factor. The seeds are sealed with the server's `staff-mfa.key`, never kept in the clear.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffMfa {
    /// The active TOTP seed, sealed (empty: no authenticator is active yet).
    #[serde(default)]
    pub seed: String,
    /// A seed shown for enrollment that no valid code has confirmed yet, sealed.
    #[serde(default)]
    pub pending: String,
    /// The newest 30-second step a code was accepted for: a step is used once.
    #[serde(default)]
    pub last_step: u64,
    /// Digests of the unused recovery codes; each works once.
    #[serde(default)]
    pub recovery: Vec<String>,
    #[serde(default)]
    pub enrolled_at: i64,
}

impl StaffMfa { pub fn active(&self) -> bool { !self.seed.is_empty() } }

/// A mute: why, and until when (Unix seconds; none: until lifted).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mute { pub reason: String, pub until: Option<i64> }

impl Login {
    /// A new password account: User ID `user`, its password's PHC hash and its recovery code's digest.
    pub fn new(user: &str, password: String, recovery: String) -> Login {
        Login { user: user.into(), password, role: Role::default(), staff_epoch: 0, standing: Standing::default(), recovery, reset: None, devices: vec![], muted: None, mfa: None }
    }

    /// Why a sign-in is refused, if it is (#26): a ban or a restriction still running at `now`. A ban also ended the
    /// player's sessions when it was made; a restriction lets a game already signed in play on, and refuses the next
    /// sign-in, the game's own ONLINE included ("This User ID is currently restricted.").
    pub fn refusal(&self, now: i64) -> Option<serde_json::Value> {
        let (error, reason, until) = match &self.standing {
            Standing::Active => return None,
            Standing::Restricted { reason, until } => ("restricted", reason, until),
            Standing::Banned { reason, until } => ("banned", reason, until),
        };
        if until.is_some_and(|t| t <= now) { return None }
        Some(serde_json::json!({ "ok": false, "error": error, "reason": reason, "until": until }))
    }

    /// The mute still running at `now`, if any.
    pub fn mute(&self, now: i64) -> Option<&Mute> {
        self.muted.as_ref().filter(|m| m.until.is_none_or(|t| t > now))
    }

    /// Whether the role may come in during maintenance.
    pub fn staff(&self) -> bool { matches!(self.role, Role::Admin | Role::Moderator) }
}

/// How new accounts are made on a server.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Registration { Open, #[default] Invite, Closed }

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "player" => Ok(Role::Player), "moderator" => Ok(Role::Moderator), "admin" => Ok(Role::Admin),
            _ => Err(format!("{s:?} is not a role: player, moderator or admin")),
        }
    }
}

impl Registration {
    pub fn as_str(self) -> &'static str { match self { Registration::Open => "open", Registration::Invite => "invite", Registration::Closed => "closed" } }
}

/// An invite code the host or a moderator handed out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invite {
    pub code: String,
    /// How many more accounts it may make; `None` for any number.
    pub uses: Option<u32>,
    /// When it stops working.
    pub expires: Option<i64>,
    /// Who made it (for tracing abuse).
    pub issued_by: String,
    /// The accounts it made.
    #[serde(default)]
    pub used_by: Vec<u32>,
}

impl Invite {
    pub fn usable(&self, now: i64) -> bool { self.uses != Some(0) && self.expires.is_none_or(|t| t > now) }
}

/// Throttling in front of the account operations: a token bucket per address, and per User ID a backoff after failed
/// passwords or codes (1 s, doubling to 15 minutes by default, reset by a success). No lockout: waiting always works.
#[derive(Default)]
pub struct Throttle { addresses: Buckets, users: HashMap<String, (u32, Instant)>, users_pruned: Option<Instant>, limits: Limits }

/// The network an allowance belongs to: an IPv4 address, or an IPv6 address's /64. A home or a server gets a whole
/// /64, so rotating addresses inside it is not a new allowance each time (security review, #39).
pub fn network(addr: IpAddr) -> IpAddr {
    match addr {
        IpAddr::V4(_) => addr,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => v4.into(),
            None => { let s = v6.segments(); std::net::Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0).into() }
        },
    }
}

/// Beyond this many networks the buckets that have filled up again are dropped (at most once a second).
pub const BUCKETS_PRUNED_ABOVE: usize = 100_000;

/// Token buckets per [`network`]: the account requests' throttle and the TLS listener's handshake allowance (#27).
#[derive(Default)]
pub struct Buckets { map: HashMap<IpAddr, (f64, Instant)>, pruned: Option<Instant> }

impl Buckets {
    /// Takes one from `addr`'s bucket (`burst` deep, `per_second` back each second); how long to wait if it is empty.
    pub fn take(&mut self, addr: IpAddr, now: Instant, burst: f64, per_second: f64) -> Result<(), Duration> {
        // A full bucket is the same as none: those go, but not on every request (a flood of new addresses would
        // otherwise make each one scan the whole map).
        if self.map.len() > BUCKETS_PRUNED_ABOVE && self.pruned.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(1)) {
            self.pruned = Some(now);
            self.map.retain(|_, (tokens, at)| *tokens + now.saturating_duration_since(*at).as_secs_f64() * per_second < burst);
        }
        let (tokens, at) = self.map.entry(network(addr)).or_insert((burst, now));
        *tokens = (*tokens + now.saturating_duration_since(*at).as_secs_f64() * per_second).min(burst);
        *at = now;
        if *tokens >= 1.0 { *tokens -= 1.0; Ok(()) } else { Err(Duration::from_secs_f64((1.0 - *tokens) / per_second)) }
    }

    /// How many networks have a bucket now.
    pub fn len(&self) -> usize { self.map.len() }

    pub fn is_empty(&self) -> bool { self.map.is_empty() }
}

/// The throttle's limits, live settings (#24): the burst an address may make, how many seconds bring back one request,
/// and the longest backoff after failures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits { pub burst: u32, pub refill_seconds: u32, pub max_backoff_seconds: u32 }

impl Default for Limits {
    fn default() -> Self { Limits { burst: 10, refill_seconds: 3, max_backoff_seconds: 15 * 60 } }
}

impl Throttle {
    /// The limits from now on (allowances already taken stay taken).
    pub fn set_limits(&mut self, limits: Limits) { self.limits = limits }

    /// Takes one request from `addr`'s allowance; how long to wait if there is none left.
    pub fn address(&mut self, addr: IpAddr, now: Instant) -> Result<(), Duration> {
        self.addresses.take(addr, now, f64::from(self.limits.burst.max(1)), 1.0 / f64::from(self.limits.refill_seconds.max(1)))
    }

    /// How long `user` must still wait after failures, if at all.
    pub fn user_wait(&self, user: &str, now: Instant) -> Option<Duration> {
        self.users.get(&user.to_lowercase()).and_then(|(_, until)| until.checked_duration_since(now)).filter(|d| !d.is_zero())
    }

    /// A wrong password or code for `user`: the next try waits twice as long as the last (from 1 s, at most 15 min).
    pub fn failed(&mut self, user: &str, now: Instant) {
        // A name no account can have is not kept: no backoff protects it, and anyone could fill the map (#39).
        if !valid_user_id(user) { return }
        if self.users.len() > BUCKETS_PRUNED_ABOVE && self.users_pruned.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(1)) {
            self.users_pruned = Some(now);
            self.users.retain(|_, (_, until)| *until > now);
        }
        let (count, until) = self.users.entry(user.to_lowercase()).or_insert((0, now));
        *count = count.saturating_add(1);
        let wait = Duration::from_secs(1u64 << (*count - 1).min(20)).min(Duration::from_secs(self.limits.max_backoff_seconds.max(1).into()));
        *until = now + wait;
    }

    pub fn succeeded(&mut self, user: &str) { self.users.remove(&user.to_lowercase()); }
}

/// Whether launcher version `have` is at least `need` (dotted numbers, a leading "v" and any "-suffix" ignored).
pub fn version_at_least(have: &str, need: &str) -> bool {
    yo3_tunnel::version_at_least(have,need)
}

#[cfg(test)]
mod tests {
    /// The production parameters' cost on this machine (#23 aims for 100–300 ms): `cargo test --release -p yo3-server
    /// --lib hash_time -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement, not a check"]
    fn hash_time() {
        let started = std::time::Instant::now();
        for _ in 0..5 { super::hash_password_now("correct horse battery").unwrap(); }
        println!("argon2id m=64 MiB t=3 p=1: {:?} a hash", started.elapsed() / 5);
    }

    use super::*;

    #[test]
    fn user_ids_and_passwords_follow_the_rules() {
        assert!(valid_user_id("YugiM1") && !valid_user_id("yu") && !valid_user_id("a".repeat(17).as_str()) && !valid_user_id("yugi muto"));
        assert!(!valid_user_id("Yugi_M") && !valid_user_id("Yugi-M"), "the game's login form can't type _ or -");
        assert_eq!(password_problem("yugi", "short"), Some("password_too_short"));
        assert_eq!(password_problem("yugi", &"a1".repeat(9)), Some("password_too_long"), "the form takes 16 characters");
        assert_eq!(password_problem("yugi", "heart of the cards"), Some("password_too_long"));
        assert_eq!(password_problem("yugi", "heart of cards"), Some("password_characters"), "no spaces in the form");
        assert_eq!(password_problem("yugi", "Kuriboh!42"), Some("password_characters"));
        assert_eq!(password_problem("yugi", "Password"), Some("password_too_common"));
        assert_eq!(password_problem("yugimuto", "YugiMuto"), Some("password_too_common"), "not the User ID either");
        assert_eq!(password_problem("yugi", "HeartOfCards16"), None, "no composition rules");
    }

    #[test]
    fn codes_are_typeable_and_compare_loosely() {
        let code = new_code();
        assert_eq!(code.len(), 19);
        assert!(code.split('-').all(|g| g.len() == 4));
        assert!(!code.contains(['0', 'O', '1', 'I']));
        assert_eq!(normalize_code(&code.to_lowercase().replace('-', " ")), normalize_code(&code));
        assert_ne!(new_token(), new_token());
    }

    #[tokio::test]
    async fn a_password_verifies_against_its_hash_and_nothing_else() {
        use_fast_hashing();
        let hash = hash_password("heart of the cards".into()).await.unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("heart of the cards".into(), Some(hash.clone())).await);
        assert!(!verify_password("heart of the card".into(), Some(hash)).await);
        assert!(!verify_password("anything".into(), None).await, "an unknown User ID never matches");
    }

    #[test]
    fn an_ipv6_network_shares_one_allowance() {
        // Security review (#39): every address of one /64 draws on the same bucket; IPv4-mapped is the IPv4 address.
        let (mut t, now) = (Throttle::default(), Instant::now());
        let a: IpAddr = "2001:db8:1:2::1".parse().unwrap();
        for i in 0..10u16 { assert!(t.address(format!("2001:db8:1:2::{:x}", i + 1).parse().unwrap(), now).is_ok()) }
        assert!(t.address("2001:db8:1:2:ffff::9".parse().unwrap(), now).is_err(), "another address of the same /64 waits");
        assert!(t.address("2001:db8:1:3::1".parse().unwrap(), now).is_ok(), "the next /64 has its own");
        assert_eq!(network(a), "2001:db8:1:2::".parse::<IpAddr>().unwrap());
        assert_eq!(network("::ffff:198.51.100.7".parse().unwrap()), "198.51.100.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn full_buckets_are_dropped_and_pruning_waits_a_second() {
        let (mut b, now) = (Buckets::default(), Instant::now());
        for i in 0..=BUCKETS_PRUNED_ABOVE as u32 { b.take(IpAddr::from(i.to_be_bytes()), now, 10.0, 1.0).unwrap() }
        let later = now + Duration::from_secs(5);
        b.take(IpAddr::from([255, 0, 0, 1]), later, 10.0, 1.0).unwrap();
        assert!(b.len() < 10, "every refilled bucket went: {}", b.len());
        for i in 0..=BUCKETS_PRUNED_ABOVE as u32 { b.take(IpAddr::from(i.to_be_bytes()), later, 10.0, 1.0).unwrap() }
        let at = b.len();
        b.take(IpAddr::from([255, 0, 0, 2]), later + Duration::from_secs(20), 10.0, 1.0).unwrap();
        assert_eq!(b.len(), 1, "pruned again a second later: only the newest is left");
        assert!(at > BUCKETS_PRUNED_ABOVE, "not pruned twice within the same second");
    }

    #[test]
    fn a_name_that_is_no_user_id_keeps_no_backoff() {
        let (mut t, now) = (Throttle::default(), Instant::now());
        t.failed(&"y".repeat(4000), now);
        t.failed("Yugi", now);
        assert_eq!(t.users.len(), 1);
    }

    #[test]
    fn throttling_backs_off_and_forgives_on_success() {
        let mut t = Throttle::default();
        let now = Instant::now();
        let addr = IpAddr::from([10, 0, 0, 1]);
        for _ in 0..10 { assert!(t.address(addr, now).is_ok()) }
        assert!(t.address(addr, now).is_err(), "the burst is used up");
        assert!(t.address(addr, now + Duration::from_secs(4)).is_ok(), "and comes back");
        t.failed("Yugi", now);
        assert_eq!(t.user_wait("yugi", now), Some(Duration::from_secs(1)));
        t.failed("YUGI", now);
        assert_eq!(t.user_wait("Yugi", now), Some(Duration::from_secs(2)));
        for _ in 0..20 { t.failed("yugi", now) }
        assert_eq!(t.user_wait("yugi", now), Some(Duration::from_secs(15 * 60)));
        t.succeeded("Yugi");
        assert_eq!(t.user_wait("yugi", now), None);
    }

    #[test]
    fn launcher_versions_compare_as_numbers() {
        assert!(version_at_least("0.2.10", "0.2.9"));
        assert!(version_at_least("v0.3.0-dev", "0.3.0"));
        assert!(!version_at_least("0.2.9", "0.2.10"));
        assert!(version_at_least("1.0", "1.0.0"));
    }

    #[test]
    fn a_ban_refuses_until_it_ends() {
        let mut login = Login::new("Yugi", String::new(), String::new());
        assert!(login.refusal(100).is_none());
        login.standing = Standing::Banned { reason: "cheating".into(), until: Some(200) };
        assert_eq!(login.refusal(100).unwrap()["error"], "banned");
        assert!(login.refusal(200).is_none(), "over once its time has passed");
    }
}
