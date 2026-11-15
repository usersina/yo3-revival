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

/// What a User ID may be: 3–16 letters, digits, `_` or `-`; unique ignoring case.
pub fn valid_user_id(user: &str) -> bool {
    (3..=16).contains(&user.len()) && user.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Why a password can't be used, if it can't: 8–128 characters, not a common one, not the User ID.
pub fn password_problem(user: &str, password: &str) -> Option<&'static str> {
    let len = password.chars().count();
    if len < 8 { return Some("password_too_short") }
    if len > 128 { return Some("password_too_long") }
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

/// Hashes a password (off the async threads, at most [`HASHING_AT_ONCE`] at a time).
pub async fn hash_password(password: String) -> anyhow::Result<String> {
    use argon2::PasswordHasher;
    let _turn = HASHING.acquire().await?;
    tokio::task::spawn_blocking(move || counted(|| argon2().hash_password(password.as_bytes()).map(|h| h.to_string()).map_err(|e| anyhow::anyhow!("{e}")))).await?
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
    let Ok(_turn) = HASHING.acquire().await else { return false };
    tokio::task::spawn_blocking(move || {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// A mute: why, and until when (Unix seconds; none: until lifted).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mute { pub reason: String, pub until: Option<i64> }

impl Login {
    /// A new password account: User ID `user`, its password's PHC hash and its recovery code's digest.
    pub fn new(user: &str, password: String, recovery: String) -> Login {
        Login { user: user.into(), password, role: Role::default(), standing: Standing::default(), recovery, reset: None, devices: vec![], muted: None }
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
pub struct Throttle { addresses: HashMap<IpAddr, (f64, Instant)>, users: HashMap<String, (u32, Instant)>, limits: Limits }

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
        let bucket = f64::from(self.limits.burst.max(1));
        let refill = 1.0 / f64::from(self.limits.refill_seconds.max(1));
        if self.addresses.len() > 100_000 { self.addresses.retain(|_, (tokens, at)| *tokens < bucket && now.duration_since(*at) < Duration::from_secs(3600)) }
        let (tokens, at) = self.addresses.entry(addr).or_insert((bucket, now));
        *tokens = (*tokens + now.duration_since(*at).as_secs_f64() * refill).min(bucket);
        *at = now;
        if *tokens >= 1.0 { *tokens -= 1.0; Ok(()) } else { Err(Duration::from_secs_f64((1.0 - *tokens) / refill)) }
    }

    /// How long `user` must still wait after failures, if at all.
    pub fn user_wait(&self, user: &str, now: Instant) -> Option<Duration> {
        self.users.get(&user.to_lowercase()).and_then(|(_, until)| until.checked_duration_since(now)).filter(|d| !d.is_zero())
    }

    /// A wrong password or code for `user`: the next try waits twice as long as the last (from 1 s, at most 15 min).
    pub fn failed(&mut self, user: &str, now: Instant) {
        if self.users.len() > 100_000 { self.users.retain(|_, (_, until)| *until > now) }
        let (count, until) = self.users.entry(user.to_lowercase()).or_insert((0, now));
        *count = count.saturating_add(1);
        let wait = Duration::from_secs(1u64 << (*count - 1).min(20)).min(Duration::from_secs(self.limits.max_backoff_seconds.max(1).into()));
        *until = now + wait;
    }

    pub fn succeeded(&mut self, user: &str) { self.users.remove(&user.to_lowercase()); }
}

/// Whether launcher version `have` is at least `need` (dotted numbers, a leading "v" and any "-suffix" ignored).
pub fn version_at_least(have: &str, need: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> { v.trim_start_matches('v').split('-').next().unwrap_or("").split('.').map(|n| n.parse().unwrap_or(0)).collect() };
    let (have, need) = (parse(have), parse(need));
    (0..have.len().max(need.len())).map(|i| (have.get(i).copied().unwrap_or(0), need.get(i).copied().unwrap_or(0)))
        .find(|(h, n)| h != n).is_none_or(|(h, n)| h > n)
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
        assert!(valid_user_id("Yugi_M-1") && !valid_user_id("yu") && !valid_user_id("a".repeat(17).as_str()) && !valid_user_id("yugi muto"));
        assert_eq!(password_problem("yugi", "short"), Some("password_too_short"));
        assert_eq!(password_problem("yugi", "Password"), Some("password_too_common"));
        assert_eq!(password_problem("yugimuto", "YugiMuto"), Some("password_too_common"), "not the User ID either");
        assert_eq!(password_problem("yugi", "heart of the cards"), None, "spaces and length, no composition rules");
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
