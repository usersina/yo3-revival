//! Multiplayer (#30): the server list, a server's status, signing in with a User ID and password, and where a
//! "stay signed in" lives.
//!
//! - **The official list** is a JSON file signed with an ed25519 key the launcher knows ([`OFFICIAL_LIST_KEYS`]): a
//!   tampered list is refused, a server can move to a new address without a launcher update, and the key can rotate
//!   (the launcher accepts every key in the list of keys). Each entry pins the server's identity (the SHA-256 of its
//!   public key, as on the game port); a server rotating its own key lists both for a while.
//! - **Other servers** the player adds by address (trust on first use: the identity is shown once, then pinned) or by
//!   an invite link that carries the identity.
//! - **Signing in** uses the server's account requests (TLS operation 6, #23). "Stay signed in" keeps the device token
//!   in the system's credential vault, never the password, and never in a file; "Save User ID" keeps only the name.
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, bail, ensure};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The ed25519 public keys (hex) the official server list may be signed with. Empty until the project publishes a
/// list (where it is hosted is D14): then the launcher shows only the servers the player added.
pub const OFFICIAL_LIST_KEYS: &[&str] = &[];
/// Where the official list is fetched from (the release assets).
pub const OFFICIAL_LIST_URL: &str = "https://github.com/usersina/yo3-revival/releases/latest/download/servers.json";
/// The largest list the launcher reads.
const MAX_LIST: usize = 256 * 1024;

/// A server in the official list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listed {
    pub name: String,
    /// `host:port`.
    pub address: String,
    /// The name on its certificate (the host's part of the address, unless it differs).
    #[serde(default)]
    pub server_name: String,
    /// Its identities: one, or two while it rotates its key.
    pub identities: Vec<String>,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub invite_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerList {
    /// Higher with each publication: an older list never replaces a newer one in the cache.
    pub version: u64,
    pub servers: Vec<Listed>,
}

/// A list as published: its exact JSON text and the signature over those bytes.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed { list: String, signature: String }

/// A new signing key: its PKCS#8 document (keep it private, off the repository) and its public key in hex.
pub fn new_list_key() -> anyhow::Result<(Vec<u8>, String)> {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).map_err(|_| anyhow::anyhow!("key generation failed"))?;
    let pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|_| anyhow::anyhow!("invalid key"))?;
    Ok((pkcs8.as_ref().to_vec(), hex(ring::signature::KeyPair::public_key(&pair).as_ref())))
}

/// The published file for `list`, signed with the PKCS#8 key `key`.
pub fn sign_list(list: &ServerList, key: &[u8]) -> anyhow::Result<String> {
    for server in &list.servers { check_listed(server)?; }
    let pair = ring::signature::Ed25519KeyPair::from_pkcs8(key).map_err(|_| anyhow::anyhow!("not an ed25519 PKCS#8 key"))?;
    let text = serde_json::to_string(list)?;
    let signature = base64::engine::general_purpose::STANDARD.encode(pair.sign(text.as_bytes()).as_ref());
    Ok(serde_json::to_string_pretty(&Signed { list: text, signature })?)
}

/// The list in `file`, if one of `keys` (hex public keys) signed exactly its text.
pub fn verify_list(file: &[u8], keys: &[&str]) -> anyhow::Result<ServerList> {
    ensure!(file.len() <= MAX_LIST, "the server list is too large");
    let signed: Signed = serde_json::from_slice(file).context("the server list is not a signed list")?;
    let signature = base64::engine::general_purpose::STANDARD.decode(&signed.signature).context("the server list's signature is malformed")?;
    let good = keys.iter().filter_map(|k| unhex(k)).any(|key| {
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key).verify(signed.list.as_bytes(), &signature).is_ok()
    });
    ensure!(good, "the server list's signature doesn't match: it was changed, or signed with a key this launcher doesn't know");
    let list: ServerList = serde_json::from_str(&signed.list).context("the signed server list is malformed")?;
    for server in &list.servers { check_listed(server)?; }
    Ok(list)
}

fn check_listed(server: &Listed) -> anyhow::Result<()> {
    crate::server_address(&server.address)?;
    ensure!(!server.name.trim().is_empty() && server.name.chars().count() <= 60, "a listed server needs a name of up to 60 characters");
    ensure!(!server.identities.is_empty() && server.identities.iter().all(|i| i.len() == 64 && unhex(i).is_some()),
        "{}: each identity is a SHA-256 in hex", server.name);
    Ok(())
}

/// The official list: fetched and checked, or the last good one from the cache under `root` (no keys: none).
pub async fn official_list(root: &Path) -> Vec<Listed> {
    if OFFICIAL_LIST_KEYS.is_empty() { return vec![] }
    let cache = root.join("server-list.json");
    let cached = std::fs::read(&cache).ok().and_then(|bytes| verify_list(&bytes, OFFICIAL_LIST_KEYS).ok());
    let fetched = async {
        let client = reqwest::Client::builder().https_only(true).timeout(Duration::from_secs(10)).build()?;
        let bytes = client.get(OFFICIAL_LIST_URL).send().await?.error_for_status()?.bytes().await?;
        let list = verify_list(&bytes, OFFICIAL_LIST_KEYS)?;
        Ok::<_, anyhow::Error>((bytes, list))
    }.await;
    match (fetched, cached) {
        (Ok((bytes, list)), cached) if cached.as_ref().is_none_or(|c| list.version >= c.version) => {
            let _ = crate::private_write(&cache, &bytes, true);
            list.servers
        }
        (_, Some(cached)) => cached.servers,
        (_, None) => vec![],
    }
}

/// A server the player can pick: from the official list, or added by them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub address: String,
    pub server_name: String,
    /// The identities it may present (pinned).
    pub identities: Vec<String>,
    pub official: bool,
    #[serde(default)]
    pub description: String,
    /// The User ID last signed in with ("Save User ID"), if any.
    #[serde(default)]
    pub saved_user: Option<String>,
    /// When it was last played on (Unix seconds): the server list puts it first.
    #[serde(default)]
    pub last_used: i64,
}

impl Server {
    pub fn from_listed(listed: &Listed) -> Server {
        let server_name = if listed.server_name.is_empty() { host_of(&listed.address) } else { listed.server_name.clone() };
        Server { name: listed.name.clone(), address: listed.address.clone(), server_name, identities: listed.identities.clone(), official: true,
            description: listed.description.clone(), saved_user: None, last_used: 0 }
    }
    /// The key it is saved under: its first identity.
    pub fn key(&self) -> &str { self.identities.first().map_or("", String::as_str) }
    fn endpoints(&self) -> anyhow::Result<Vec<yo3_tunnel::Endpoint>> {
        self.identities.iter().map(|identity| yo3_tunnel::Endpoint::pinned(self.address.clone(), self.server_name.clone(), identity)).collect()
    }
}

/// The servers to show: the player's own first by last use, then the official ones they haven't used.
pub fn arrange(official: &[Listed], saved: &[Server]) -> Vec<Server> {
    let mut all: Vec<Server> = saved.to_vec();
    for listed in official {
        let server = Server::from_listed(listed);
        match all.iter_mut().find(|s| s.official && s.identities.iter().any(|i| listed.identities.contains(i))) {
            // The list wins for where an official server is now, its name and identities; the player keeps the rest.
            Some(known) => { let (user, used) = (known.saved_user.take(), known.last_used); *known = Server { saved_user: user, last_used: used, ..server } }
            None => all.push(server),
        }
    }
    all.sort_by_key(|s| (std::cmp::Reverse(s.last_used), !s.official, s.name.to_lowercase()));
    all
}

fn host_of(address: &str) -> String {
    crate::server_address(address).map(|(_, name)| name).unwrap_or_else(|_| address.to_string())
}

/// A server an invite link names: `yo3revival://server?address=HOST:PORT&identity=HEX&name=NAME`. The link carries
/// the identity, so no first-use question follows.
pub fn invite_link(link: &str) -> anyhow::Result<Server> {
    let rest = link.trim().strip_prefix("yo3revival://server?").context("an invite link starts with yo3revival://server?")?;
    let field = |key: &str| rest.split('&').find_map(|p| p.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| decode(v)));
    let address = field("address").context("the invite link has no address")?;
    let identity = field("identity").context("the invite link has no identity")?.to_ascii_lowercase();
    ensure!(identity.len() == 64 && unhex(&identity).is_some(), "the invite link's identity is malformed");
    let (address, server_name) = crate::server_address(&address)?;
    let name = field("name").filter(|n| !n.trim().is_empty()).unwrap_or_else(|| server_name.clone());
    Ok(Server { name, address, server_name, identities: vec![identity], official: false, description: String::new(), saved_user: None, last_used: 0 })
}

/// The invite link for `server`.
pub fn link_for(server: &Server) -> String {
    let encode = |s: &str| s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~:".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>();
    format!("yo3revival://server?address={}&identity={}&name={}", encode(&server.address), server.key(), encode(&server.name))
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) { out.push(v); i += 3; continue }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What a server's row shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Online { online: Option<u32>, max: Option<u32>, registration: String, news: Vec<yo3_tunnel::News>, ping: Duration },
    Maintenance,
    NeedsUpdate { min: String },
    /// It answered with an identity it isn't pinned to.
    IdentityChanged,
    Offline,
}

/// Asks `server` how it is (credential-free), against its pinned identities; one it doesn't present is reported.
pub async fn status(server: &Server) -> Status {
    let started = Instant::now();
    let Ok(endpoints) = server.endpoints() else { return Status::Offline };
    for endpoint in endpoints {
        if let Ok(info) = endpoint.info().await {
            if info.maintenance { return Status::Maintenance }
            if let Some(min) = &info.min_launcher && !yo3_server::logins::version_at_least(yo3_tunnel::BUILD_VERSION, min) {
                return Status::NeedsUpdate { min: min.clone() };
            }
            return Status::Online { online: info.online, max: info.max_online, registration: info.registration, news: info.news, ping: started.elapsed() };
        }
    }
    // None of its identities answered: is it down, or a different key at its address?
    match yo3_tunnel::discover(&server.address, &server.server_name).await {
        Ok(seen) if !server.identities.contains(&seen.identity) => Status::IdentityChanged,
        _ => Status::Offline,
    }
}

/// A server first met at `address`: its name and identity, to show before trusting it (trust on first use).
pub async fn first_contact(address: &str) -> anyhow::Result<Server> {
    let (address, server_name) = crate::server_address(address)?;
    let seen = yo3_tunnel::discover(&address, &server_name).await.context("the server didn't answer")?;
    let name = if seen.info.name.trim().is_empty() { server_name.clone() } else { seen.info.name.clone() };
    Ok(Server { name, address, server_name, identities: vec![seen.identity], official: false, description: String::new(), saved_user: None, last_used: 0 })
}

/// Why a sign-in, sign-up or recovery didn't work, as one sentence for the player.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// A wrong password and an unknown User ID look the same.
    WrongUserOrPassword,
    TooManyTries { wait_seconds: u64 },
    Banned { reason: String, until: Option<i64> },
    /// A restriction: sign-in refused for a while (#26).
    Restricted { reason: String, until: Option<i64> },
    /// The server's own words, when it gave some.
    Maintenance { message: String },
    Full,
    NeedsUpdate { min: String },
    IdentityChanged,
    Unreachable,
    /// A stay-signed-in that was revoked (a new password, Sign out everywhere, a ban) or expired.
    SignInAgain,
    /// At Play, the sign-in's session had ended: a kick or a ban, or the account signed in elsewhere (#30).
    SessionEnded,
    InviteNeeded,
    InviteInvalid,
    RegistrationClosed,
    UserIdTaken,
    UserIdInvalid,
    PasswordTooShort,
    PasswordTooLong,
    PasswordTooCommon,
    PasswordsDiffer,
    RulesNotAccepted,
    RecoveryInvalid,
    Other(String),
}

/// What the player can do about a problem, besides typing again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remedy { Retry, Update, SignInAgain, RemoveServer }

impl Problem {
    /// The server's answer to an account request, as a problem.
    pub fn from_reply(reply: &Value) -> Problem {
        match reply["error"].as_str().unwrap_or_default() {
            "invalid_credentials" => Problem::WrongUserOrPassword,
            "throttled" => Problem::TooManyTries { wait_seconds: reply["retry_after"].as_u64().unwrap_or(1) },
            "banned" => Problem::Banned { reason: reply["reason"].as_str().unwrap_or_default().to_string(), until: reply["until"].as_i64() },
            "restricted" => Problem::Restricted { reason: reply["reason"].as_str().unwrap_or_default().to_string(), until: reply["until"].as_i64() },
            "maintenance" => Problem::Maintenance { message: reply["message"].as_str().unwrap_or_default().to_string() },
            "server_full" => Problem::Full,
            "update_required" => Problem::NeedsUpdate { min: reply["min_launcher"].as_str().unwrap_or_default().to_string() },
            "signed_out" => Problem::SignInAgain,
            "invite_required" => Problem::InviteNeeded,
            "invite_invalid" => Problem::InviteInvalid,
            "registration_closed" => Problem::RegistrationClosed,
            "user_id_taken" => Problem::UserIdTaken,
            "invalid_user_id" => Problem::UserIdInvalid,
            "password_too_short" => Problem::PasswordTooShort,
            "password_too_long" => Problem::PasswordTooLong,
            "password_too_common" => Problem::PasswordTooCommon,
            "rules_not_accepted" => Problem::RulesNotAccepted,
            "invalid_recovery" => Problem::RecoveryInvalid,
            other => Problem::Other(other.to_string()),
        }
    }

    pub fn sentence(&self) -> String {
        match self {
            Problem::WrongUserOrPassword => "The User ID or password is wrong.".into(),
            Problem::TooManyTries { wait_seconds } => format!("Too many attempts. Try again in {}.", wait(*wait_seconds)),
            Problem::Banned { reason, until } => standing("banned", reason, *until, "for good"),
            Problem::Restricted { reason, until } => standing("restricted", reason, *until, "until a moderator lifts it"),
            Problem::Maintenance { message } if !message.trim().is_empty() => format!("The server is in maintenance: {}", message.trim()),
            Problem::Maintenance { .. } => "The server is in maintenance. Try again later.".into(),
            Problem::Full => "The server is full.".into(),
            Problem::NeedsUpdate { min } => format!("This server needs YO3 Revival {min} or newer."),
            Problem::IdentityChanged => "This server's identity changed. It may not be the server you added: don't sign in.".into(),
            Problem::Unreachable => "We can't reach the server. Check your internet connection, or try again later.".into(),
            Problem::SignInAgain => "Please sign in again.".into(),
            Problem::SessionEnded => "Your sign-in ended: the server restarted, a moderator ended it, or this account signed in somewhere else.".into(),
            Problem::InviteNeeded => "This server needs an invite code to make an account.".into(),
            Problem::InviteInvalid => "That invite code doesn't work: it may be used up or expired.".into(),
            Problem::RegistrationClosed => "This server isn't taking new accounts.".into(),
            Problem::UserIdTaken => "That User ID is taken.".into(),
            Problem::UserIdInvalid => "A User ID is 3 to 16 letters, digits, _ or -.".into(),
            Problem::PasswordTooShort => "A password has at least 8 characters.".into(),
            Problem::PasswordTooLong => "A password has at most 128 characters.".into(),
            Problem::PasswordTooCommon => "That password is too common, or the same as the User ID.".into(),
            Problem::PasswordsDiffer => "The two passwords differ.".into(),
            Problem::RulesNotAccepted => "Accept the server's rules to make an account.".into(),
            Problem::RecoveryInvalid => "That code doesn't work: check it, or ask the server's moderators for a reset code.".into(),
            Problem::Other(code) => format!("The server refused ({code})."),
        }
    }

    pub fn remedy(&self) -> Option<Remedy> {
        match self {
            Problem::Maintenance { .. } | Problem::Full | Problem::Unreachable => Some(Remedy::Retry),
            Problem::NeedsUpdate { .. } => Some(Remedy::Update),
            Problem::SignInAgain | Problem::SessionEnded => Some(Remedy::SignInAgain),
            Problem::IdentityChanged => Some(Remedy::RemoveServer),
            _ => None,
        }
    }
}

/// A ban or a restriction as one sentence: what, until when (local time), and the moderator's reason.
fn standing(what: &str, reason: &str, until: Option<i64>, forever: &str) -> String {
    let until = until.and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map_or(forever.to_string(), |t| format!("until {}", t.with_timezone(&chrono::Local).format("%d %B %Y, %H:%M")));
    if reason.trim().is_empty() { format!("This account is {what} {until}.") } else { format!("This account is {what} {until}: {}", reason.trim()) }
}

fn wait(seconds: u64) -> String {
    match seconds {
        0..=59 => format!("{} second{}", seconds.max(1), if seconds == 1 { "" } else { "s" }),
        _ => { let minutes = seconds.div_ceil(60); format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" }) }
    }
}

/// A sign-in: the session for the game's connections, and the device token when the player stays signed in.
#[derive(Clone)]
pub struct SignedIn { pub user: String, pub session: yo3_tunnel::Token, pub device_token: Option<String> }

impl std::fmt::Debug for SignedIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "SignedIn({})", self.user) }
}

/// One account request to `server`, through whichever of its identities answers.
async fn ask(server: &Server, request: Value) -> Result<Value, Problem> {
    let mut request = request;
    request["launcher"] = json!(yo3_tunnel::BUILD_VERSION.trim_start_matches('v'));
    let mut reached = false;
    for endpoint in server.endpoints().map_err(|_| Problem::Unreachable)? {
        match endpoint.account(&request).await {
            Ok(reply) if reply["ok"] == true => return Ok(reply),
            Ok(reply) => return Err(Problem::from_reply(&reply)),
            Err(_) => reached |= endpoint.info().await.is_ok(),
        }
    }
    if !reached && matches!(status(server).await, Status::IdentityChanged) { return Err(Problem::IdentityChanged) }
    Err(Problem::Unreachable)
}

fn session_of(reply: &Value) -> Result<yo3_tunnel::Token, Problem> {
    reply["session"].as_str().and_then(unhex).and_then(|b| b.try_into().ok()).ok_or_else(|| Problem::Other("no session".into()))
}

pub async fn sign_in(server: &Server, user: &str, password: &str, stay: bool) -> Result<SignedIn, Problem> {
    let label = format!("{} PC", if cfg!(windows) { "Windows" } else if cfg!(target_os = "macos") { "Mac" } else { "Linux" });
    let reply = ask(server, json!({ "op": "login", "user": user, "password": password, "stay": stay, "device": label })).await?;
    Ok(SignedIn { user: reply["user"].as_str().unwrap_or(user).to_string(), session: session_of(&reply)?, device_token: reply["device_token"].as_str().map(str::to_owned) })
}

pub async fn resume(server: &Server, device_token: &str) -> Result<SignedIn, Problem> {
    let reply = ask(server, json!({ "op": "resume", "device_token": device_token })).await?;
    Ok(SignedIn { user: reply["user"].as_str().unwrap_or_default().to_string(), session: session_of(&reply)?, device_token: Some(device_token.to_owned()) })
}

/// Whether a sign-in's session still plays: it ends when a moderator kicks or bans the account, or the account signs
/// in elsewhere (the newest wins). Asks for a ticket, as the game's helper does: refused is `SignInAgain`, no answer
/// `Unreachable`.
pub async fn session_alive(server: &Server, session: &yo3_tunnel::Token) -> Result<(), Problem> {
    for endpoint in server.endpoints().map_err(|_| Problem::Unreachable)? {
        match endpoint.ticket(session, yo3_tunnel::Role::Http).await {
            Ok(_) => return Ok(()),
            Err(e) if e.downcast_ref::<yo3_tunnel::Refused>().is_some() => return Err(Problem::SignInAgain),
            Err(_) => {}
        }
    }
    Err(Problem::Unreachable)
}

pub async fn sign_out(server: &Server, device_token: &str, everywhere: bool) -> Result<(), Problem> {
    ask(server, json!({ "op": "sign_out", "device_token": device_token, "everywhere": everywhere })).await.map(|_| ())
}

/// Settings → Accounts' Sign out everywhere: every device token of the account is revoked and its game sessions end.
/// A sign-in that didn't stay has no device token to ask with, so the password signs in once more for one.
pub async fn sign_out_everywhere(server: &Server, device_token: Option<&str>, user: &str, password: &str) -> Result<(), Problem> {
    let token = match device_token {
        Some(token) => token.to_owned(),
        None => sign_in(server, user, password, true).await?.device_token.ok_or_else(|| Problem::Other("no device token".into()))?,
    };
    sign_out(server, &token, true).await
}

/// Settings → Accounts' Change password: every device is signed out by the server, this one too.
pub async fn change_password(server: &Server, user: &str, password: &str, new: &str, again: &str) -> Result<(), Problem> {
    if new != again { return Err(Problem::PasswordsDiffer) }
    ask(server, json!({ "op": "change_password", "user": user, "password": password, "new_password": new })).await.map(|_| ())
}

/// A new account: its one-time recovery code.
pub async fn register(server: &Server, user: &str, password: &str, again: &str, invite: &str, rules: bool) -> Result<String, Problem> {
    if password != again { return Err(Problem::PasswordsDiffer) }
    if !rules { return Err(Problem::RulesNotAccepted) }
    let reply = ask(server, json!({ "op": "register", "user": user, "password": password, "invite": invite, "rules": true })).await?;
    reply["recovery_code"].as_str().map(str::to_owned).ok_or_else(|| Problem::Other("no recovery code".into()))
}

/// A new password from a recovery or reset code: the new recovery code, after a recovery code.
pub async fn recover(server: &Server, user: &str, code: &str, password: &str, again: &str) -> Result<Option<String>, Problem> {
    if password != again { return Err(Problem::PasswordsDiffer) }
    let reply = ask(server, json!({ "op": "recover", "user": user, "code": code, "new_password": password })).await?;
    Ok(reply["recovery_code"].as_str().map(str::to_owned))
}

/// Where a stay-signed-in's device token is kept: the system's credential vault, by server and User ID.
pub trait Vault: Send + Sync {
    fn get(&self, account: &str) -> anyhow::Result<Option<String>>;
    fn set(&self, account: &str, secret: &str) -> anyhow::Result<()>;
    fn delete(&self, account: &str) -> anyhow::Result<()>;
}

const VAULT_SERVICE: &str = "yo3-revival.servers";

/// The account a server's sign-in is kept under (the token itself is the secret).
pub fn vault_account(server: &Server) -> String { format!("server:{}", server.key()) }

/// The operating system's vault (Windows Credential Manager, the Secret Service on Linux).
pub struct SystemVault;

impl Vault for SystemVault {
    fn get(&self, account: &str) -> anyhow::Result<Option<String>> {
        match keyring::Entry::new(VAULT_SERVICE, account).and_then(|e| e.get_password()) {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            // Backend errors may carry the stored bytes: never in a message.
            Err(_) => bail!("the system's credential store can't be read"),
        }
    }
    fn set(&self, account: &str, secret: &str) -> anyhow::Result<()> {
        keyring::Entry::new(VAULT_SERVICE, account).and_then(|e| e.set_password(secret)).map_err(|_| anyhow::anyhow!("the system's credential store can't be written"))
    }
    fn delete(&self, account: &str) -> anyhow::Result<()> {
        match keyring::Entry::new(VAULT_SERVICE, account).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => bail!("the system's credential store can't be written"),
        }
    }
}

/// Whether the vault works here; why not, if it doesn't (headless Linux, a locked Secret Service). It only reads (an
/// entry that isn't there): checking writes nothing to the player's store.
pub fn vault_problem(vault: &dyn Vault) -> Option<String> {
    match vault.get("availability") {
        Ok(_) => None,
        Err(_) => Some("Stay signed in needs the system's credential store, which isn't available here.".into()),
    }
}

/// A vault in memory: for tests, and where the system's isn't available (nothing outlives the launcher).
#[derive(Default)]
pub struct MemoryVault(std::sync::Mutex<std::collections::HashMap<String, String>>);

impl Vault for MemoryVault {
    fn get(&self, account: &str) -> anyhow::Result<Option<String>> { Ok(self.0.lock().unwrap().get(account).cloned()) }
    fn set(&self, account: &str, secret: &str) -> anyhow::Result<()> { self.0.lock().unwrap().insert(account.into(), secret.into()); Ok(()) }
    fn delete(&self, account: &str) -> anyhow::Result<()> { self.0.lock().unwrap().remove(account); Ok(()) }
}

fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }
fn unhex(text: &str) -> Option<Vec<u8>> {
    text.len().is_multiple_of(2).then_some(())?;
    (0..text.len() / 2).map(|i| u8::from_str_radix(text.get(i * 2..i * 2 + 2)?, 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listed(name: &str, identity: char) -> Listed {
        Listed { name: name.into(), address: format!("{}.example:7443", name.to_lowercase()), server_name: String::new(),
            identities: vec![identity.to_string().repeat(64)], region: "EU".into(), description: String::new(), invite_only: true }
    }

    #[test]
    fn a_signed_list_is_read_and_a_tampered_one_refused() {
        let (key, public) = new_list_key().unwrap();
        let list = ServerList { version: 3, servers: vec![listed("Beta", 'a')] };
        let file = sign_list(&list, &key).unwrap();
        assert_eq!(verify_list(file.as_bytes(), &[&public]).unwrap(), list);
        // A changed address, signature or a list signed by another key: refused.
        let tampered = file.replace("beta.example", "evil.example");
        assert!(verify_list(tampered.as_bytes(), &[&public]).unwrap_err().to_string().contains("doesn't match"));
        let (other, _) = new_list_key().unwrap();
        assert!(verify_list(sign_list(&list, &other).unwrap().as_bytes(), &[&public]).is_err());
        assert!(verify_list(b"{\"servers\": []}", &[&public]).is_err(), "an unsigned list");
        assert!(verify_list(file.as_bytes(), &[]).is_err(), "no keys: nothing is trusted");
    }

    #[test]
    fn a_key_rotation_is_accepted_while_the_launcher_knows_both_keys() {
        let (old, old_public) = new_list_key().unwrap();
        let (new, new_public) = new_list_key().unwrap();
        let list = ServerList { version: 4, servers: vec![Listed { identities: vec!["a".repeat(64), "b".repeat(64)], ..listed("Beta", 'a') }] };
        for key in [&old, &new] { assert!(verify_list(sign_list(&list, key).unwrap().as_bytes(), &[&old_public, &new_public]).is_ok()) }
        // A launcher that dropped the old key no longer reads old lists.
        assert!(verify_list(sign_list(&list, &old).unwrap().as_bytes(), &[&new_public]).is_err());
        // The server's own key rotation: both identities are accepted for it.
        assert_eq!(Server::from_listed(&list.servers[0]).identities.len(), 2);
    }

    #[test]
    fn a_listed_server_needs_a_name_an_address_and_identities() {
        let (key, _) = new_list_key().unwrap();
        for bad in [Listed { name: " ".into(), ..listed("Beta", 'a') }, Listed { identities: vec![], ..listed("Beta", 'a') },
            Listed { identities: vec!["xyz".into()], ..listed("Beta", 'a') }, Listed { address: "".into(), ..listed("Beta", 'a') }] {
            assert!(sign_list(&ServerList { version: 1, servers: vec![bad] }, &key).is_err());
        }
    }

    #[test]
    fn the_last_used_server_comes_first_and_the_list_moves_official_ones() {
        let official = [listed("Beta", 'a'), listed("Second", 'b')];
        let moved = Listed { address: "new.example:7443".into(), ..listed("Beta", 'a') };
        let mine = Server { last_used: 50, ..invite_link(&format!("yo3revival://server?address=friend.example:7443&identity={}&name=Friends", "c".repeat(64))).unwrap() };
        let used = Server { last_used: 90, saved_user: Some("yugi".into()), ..Server::from_listed(&official[0]) };
        let shown = arrange(&[moved, official[1].clone()], &[mine, used]);
        let names: Vec<&str> = shown.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Beta", "Friends", "Second"]);
        assert_eq!((shown[0].address.as_str(), shown[0].saved_user.as_deref()), ("new.example:7443", Some("yugi")), "the list's address, the player's User ID");
    }

    #[test]
    fn an_invite_link_carries_the_address_identity_and_name() {
        let identity = "d".repeat(64);
        let server = invite_link(&format!("yo3revival://server?address=play.example%3A7443&identity={identity}&name=Kaiba%20Land")).unwrap();
        assert_eq!((server.name.as_str(), server.address.as_str(), server.server_name.as_str()), ("Kaiba Land", "play.example:7443", "play.example"));
        assert_eq!(invite_link(&link_for(&server)).unwrap(), server, "a link round-trips");
        assert!(invite_link("https://example.com").is_err());
        assert!(invite_link("yo3revival://server?address=play.example").is_err(), "no identity: not an invite");
    }

    #[test]
    fn every_problem_is_one_sentence_and_some_have_a_remedy() {
        let reply = |error: &str, extra: Value| { let mut r = json!({ "ok": false, "error": error }); for (k, v) in extra.as_object().unwrap() { r[k] = v.clone() } r };
        let cases = [
            (reply("invalid_credentials", json!({})), "The User ID or password is wrong.", None),
            (reply("throttled", json!({ "retry_after": 120 })), "Too many attempts. Try again in 2 minutes.", None),
            (reply("throttled", json!({ "retry_after": 1 })), "Too many attempts. Try again in 1 second.", None),
            (reply("banned", json!({ "reason": "cheating", "until": null })), "This account is banned for good: cheating", None),
            (reply("maintenance", json!({})), "The server is in maintenance. Try again later.", Some(Remedy::Retry)),
            (reply("maintenance", json!({ "message": "Back at 10:00" })), "The server is in maintenance: Back at 10:00", Some(Remedy::Retry)),
            (reply("restricted", json!({ "reason": "spam", "until": null })), "This account is restricted until a moderator lifts it: spam", None),
            (reply("server_full", json!({})), "The server is full.", Some(Remedy::Retry)),
            (reply("update_required", json!({ "min_launcher": "0.2.0" })), "This server needs YO3 Revival 0.2.0 or newer.", Some(Remedy::Update)),
            (reply("signed_out", json!({})), "Please sign in again.", Some(Remedy::SignInAgain)),
            (reply("invite_required", json!({})), "This server needs an invite code to make an account.", None),
            (reply("user_id_taken", json!({})), "That User ID is taken.", None),
        ];
        for (reply, sentence, remedy) in cases {
            let problem = Problem::from_reply(&reply);
            assert_eq!((problem.sentence().as_str(), problem.remedy()), (sentence, remedy), "{reply}");
        }
        assert!(Problem::Banned { reason: "x".into(), until: Some(2_000_000_000) }.sentence().contains("until"));
        assert_eq!(Problem::IdentityChanged.remedy(), Some(Remedy::RemoveServer));
        assert_eq!(Problem::Unreachable.remedy(), Some(Remedy::Retry));
        for problem in [Problem::PasswordsDiffer, Problem::RulesNotAccepted, Problem::RecoveryInvalid, Problem::Other("x".into())] {
            let sentence = problem.sentence();
            assert!(sentence.ends_with('.') || sentence.ends_with(')'), "{sentence}");
        }
    }

    #[test]
    fn the_memory_vault_keeps_a_token_per_server() {
        let vault = MemoryVault::default();
        let server = Server::from_listed(&listed("Beta", 'a'));
        assert_eq!(vault.get(&vault_account(&server)).unwrap(), None);
        vault.set(&vault_account(&server), "token").unwrap();
        assert_eq!(vault.get(&vault_account(&server)).unwrap().as_deref(), Some("token"));
        vault.delete(&vault_account(&server)).unwrap();
        assert_eq!(vault.get(&vault_account(&server)).unwrap(), None);
        assert_eq!(vault_problem(&vault), None);
    }
}
