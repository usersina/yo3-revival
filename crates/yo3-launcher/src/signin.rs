//! The Scripts edition's sign-in (#23): `yo3-launcher --sign-in SERVER USER --profile FILE` signs in with a User ID and
//! password once, keeps the device token in the system's credential store (the same entry Desktop's "Stay signed in"
//! uses for that server) and writes a profile without a secret: where the server is, its identity, and the store entry.
//! `play` with that profile resumes the saved sign-in; without one, the game's own login form asks (#34). A host shares
//! the server's invite link (`yo3-server share`) and an invite code, never an account key.
use std::path::Path;
use anyhow::Context;
use yo3_tunnel::invite::Invite;
use yo3_tunnel::{Endpoint, Token};
use crate::vault::Vault;

/// The store entry of a server's saved sign-in: by its identity, as Desktop names it.
pub fn vault_account(identity: &str) -> String { format!("server:{identity}") }

/// The device's name in the account's list of signed-in devices.
pub fn device_label() -> String { format!("{} PC (Scripts)", if cfg!(windows) { "Windows" } else { "Linux" }) }

/// The server SERVER names: an invite link, or an address with its identity (`--identity`).
pub fn server(server: &str, identity: Option<&str>) -> anyhow::Result<Invite> {
    if server.trim().starts_with("yo3revival:") {
        let invite = Invite::parse(server)?;
        anyhow::ensure!(identity.is_none_or(|i| i.trim().eq_ignore_ascii_case(&invite.identity)), "the invite link names another identity than --identity");
        return Ok(invite)
    }
    let identity = identity.context("give the server's invite link, or its address with --identity (the host's `yo3-server share` prints both)")?;
    Invite::new(server, identity, "")
}

/// The connection to `invite`'s server, pinned to its identity.
pub fn endpoint(invite: &Invite) -> anyhow::Result<Endpoint> { Endpoint::pinned(invite.address.clone(), invite.server_name.clone(), &invite.identity) }

/// Why the credential store can't keep a sign-in here, if it can't (no Secret Service on a headless Linux, a locked
/// one). It only reads.
pub fn vault_problem(vault: &dyn Vault) -> Option<String> {
    vault.get("availability").err().map(|e| format!("{e:#}"))
}

/// An account request the server refused: its error code and the sentence for the player.
#[derive(Debug)]
pub struct Refused { pub code: String, pub sentence: String }

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.sentence) }
}
impl std::error::Error for Refused {}

/// The sentence for an account request's error (Desktop's `multiplayer::Problem` says the same).
pub fn sentence(reply: &serde_json::Value) -> String {
    let text = |key: &str| reply[key].as_str().unwrap_or_default().trim().to_string();
    let reason = |what: &str| { let r = text("reason"); if r.is_empty() { format!("This account is {what}.") } else { format!("This account is {what}: {r}") } };
    match reply["error"].as_str().unwrap_or("unknown") {
        "invalid_credentials" => "The User ID or password is wrong.".into(),
        "throttled" => format!("Too many attempts. Try again in {} seconds.", reply["retry_after"].as_u64().unwrap_or(1)),
        "banned" => reason("banned"),
        "restricted" => reason("restricted"),
        "maintenance" => { let m = text("message"); if m.is_empty() { "The server is under maintenance.".into() } else { format!("The server is under maintenance: {m}") } }
        "server_full" => "The server is full. Try again later.".into(),
        "update_required" => format!("This server needs YO3 Revival {} or newer. Download the new Scripts edition.", text("min_launcher")),
        "signed_out" => "That sign-in has ended. Sign in again.".into(),
        "invite_required" => "This server needs an invite code from its host.".into(),
        "invite_invalid" => "That invite code doesn't work: it may be used up or expired. Ask the host for another.".into(),
        "registration_closed" => "This server doesn't take new accounts. Ask its host.".into(),
        "user_id_taken" => "That User ID is taken. Choose another.".into(),
        "invalid_user_id" => "A User ID is 3 to 16 letters or digits.".into(),
        "password_too_short" => "The password is too short: 8 to 16 letters or digits.".into(),
        "password_too_long" => "The password is too long: 8 to 16 letters or digits.".into(),
        "password_too_common" => "That password is too common. Choose another.".into(),
        "password_characters" => "A password is 8 to 16 letters or digits (the game's own login form takes no others).".into(),
        "rules_not_accepted" => "Accept the server's rules to make an account.".into(),
        code => format!("The server refused ({code})."),
    }
}

async fn ask(endpoint: &Endpoint, mut request: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    request["launcher"] = serde_json::json!(yo3_tunnel::BUILD_VERSION.trim_start_matches('v'));
    let reply = endpoint.account(&request).await.context("the server can't be reached")?;
    if reply["ok"] != true {
        return Err(Refused { code: reply["error"].as_str().unwrap_or("unknown").to_string(), sentence: sentence(&reply) }.into())
    }
    Ok(reply)
}

/// A new account: its one-time recovery code. `invite` is the host's invite code ("" when the server takes anyone).
pub async fn register(endpoint: &Endpoint, user: &str, password: &str, invite: &str, rules: bool) -> anyhow::Result<String> {
    let reply = ask(endpoint, serde_json::json!({ "op": "register", "user": user, "password": password, "invite": invite.trim(), "rules": rules })).await?;
    reply["recovery_code"].as_str().map(str::to_owned).context("the server's reply has no recovery code")
}

/// How a sign-in ended here.
#[derive(Debug, PartialEq, Eq)]
pub enum Kept {
    /// The device token is in the credential store: `play` resumes it.
    Saved,
    /// The store can't keep it (why): `play` shows the game's own login form instead.
    NotKept(String),
}

/// Signs in with `user` and `password` and keeps the device token in `vault` under the server's entry. The session the
/// sign-in also gives ends here: `play` starts its own from the token.
pub async fn sign_in(endpoint: &Endpoint, invite: &Invite, user: &str, password: &str, vault: &dyn Vault) -> anyhow::Result<Kept> {
    let problem = vault_problem(vault);
    let reply = ask(endpoint, serde_json::json!({ "op": "login", "user": user, "password": password, "stay": problem.is_none(),
        "device": device_label() })).await?;
    if let Some(session) = reply["session"].as_str().and_then(unhex) { let _ = endpoint.logout(&session).await; }
    if let Some(problem) = problem { return Ok(Kept::NotKept(problem)) }
    let token = reply["device_token"].as_str().context("the server kept no sign-in for this device")?;
    match vault.set(&vault_account(&invite.identity), token) {
        Ok(()) => Ok(Kept::Saved),
        Err(error) => Ok(Kept::NotKept(format!("{error:#}"))),
    }
}

fn unhex(text: &str) -> Option<Token> {
    if text.len() != 64 { return None }
    let bytes: Option<Vec<u8>> = (0..32).map(|i| u8::from_str_radix(text.get(i * 2..i * 2 + 2)?, 16).ok()).collect();
    bytes?.try_into().ok()
}

/// Writes the profile `play` reads: the server, its identity and the store entry; no key, no session, no password.
/// An existing profile is replaced, unless it holds an account key (that one may be the only copy).
pub fn write_profile(path: &Path, invite: &Invite) -> anyhow::Result<()> {
    if let Ok(bytes) = std::fs::read(path) {
        let old: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        anyhow::ensure!(old["key"].as_str().is_none_or(str::is_empty), "{} holds an account key: choose another file for this sign-in", path.display());
    }
    let body = serde_json::json!({ "server": invite.address, "server_name": invite.server_name, "identity": invite.identity,
        "name": invite.name, "vault": vault_account(&invite.identity) });
    let mut options = std::fs::OpenOptions::new(); options.write(true).create(true).truncate(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    use std::io::Write;
    options.open(path).with_context(|| format!("write {}", path.display()))?.write_all(body.to_string().as_bytes())?;
    Ok(())
}
