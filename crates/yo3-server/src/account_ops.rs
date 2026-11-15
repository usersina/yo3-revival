//! Operation 6 of the TLS protocol: the account requests (#23). Each request is one JSON object with an "op"; each
//! reply has "ok", and "error" when it isn't (one of the codes below, with "retry_after" seconds when throttled).
//!
//! | op | asks for | replies |
//! | --- | --- | --- |
//! | register | user, password, invite (when the server needs one), rules: true | recovery_code |
//! | login | user, password, stay, device | session, device_token (when stay), user |
//! | resume | device_token | session, user |
//! | sign_out | device_token, everywhere | — |
//! | change_password | user, password, new_password | — (every device is signed out) |
//! | recover | user, code (the recovery code or a moderator's reset code), new_password | recovery_code (a new one) |
//!
//! Every request may carry "launcher" (its version): an older one than the server's `min-launcher` setting gets
//! `update_required`. The game's own connections still use the session through tickets (operations 2 and 3).
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

use serde_json::{Value as Json, json};

use crate::accounts::Directory;
use crate::auth::Sessions;
use crate::lock::Locked;
use crate::logins::{self, Device, Login, Registration, Throttle};
use crate::settings::Settings;

pub struct Context<'a> {
    pub accounts: &'a Mutex<Directory>,
    pub sessions: &'a Mutex<Sessions>,
    pub throttle: &'a Mutex<Throttle>,
    /// The live settings at this request (#24): registration, the oldest launcher, the throttle's limits.
    pub settings: &'a Settings,
    pub peer: IpAddr,
}

fn error(code: &str) -> Json { json!({ "ok": false, "error": code }) }
fn waiting(wait: std::time::Duration) -> Json { json!({ "ok": false, "error": "throttled", "retry_after": wait.as_secs().max(1) }) }
fn text<'a>(request: &'a Json, field: &str) -> &'a str { request[field].as_str().unwrap_or_default() }
fn hex(token: &[u8]) -> String { token.iter().map(|b| format!("{b:02x}")).collect() }

/// Answers one account request. Never logs passwords, tokens or codes.
pub async fn handle(request: &Json, ctx: &Context<'_>) -> Json {
    let op = text(request, "op");
    let allowed = { let mut throttle = ctx.throttle.locked(); throttle.set_limits(ctx.settings.limits); throttle.address(ctx.peer, Instant::now()) };
    if let Err(wait) = allowed {
        tracing::warn!("[account] {} {op}: throttled", ctx.peer);
        return waiting(wait);
    }
    if let (Some(need), Some(have)) = (&ctx.settings.min_launcher, request["launcher"].as_str())
        && !logins::version_at_least(have, need) { return json!({ "ok": false, "error": "update_required", "min_launcher": need }) }
    let reply = match op {
        "register" => register(request, ctx).await,
        "login" => login(request, ctx).await,
        "resume" => resume(request, ctx),
        "sign_out" => sign_out(request, ctx),
        "change_password" => change_password(request, ctx).await,
        "recover" => recover(request, ctx).await,
        _ => error("unknown_op"),
    };
    tracing::info!("[account] {} {op}: {}", ctx.peer, reply["error"].as_str().unwrap_or("ok"));
    reply
}

async fn register(request: &Json, ctx: &Context<'_>) -> Json {
    let (user, password) = (text(request, "user").trim(), text(request, "password"));
    if !logins::valid_user_id(user) { return error("invalid_user_id") }
    if let Some(problem) = logins::password_problem(user, password) { return error(problem) }
    if request["rules"].as_bool() != Some(true) { return error("rules_not_accepted") }
    if ctx.settings.registration == Registration::Closed { return error("registration_closed") }
    if ctx.accounts.locked().login_of(user).is_some() { return error("user_id_taken") }
    let Ok(hash) = logins::hash_password(password.to_owned()).await else { return error("server_error") };
    let recovery = logins::new_code();
    let login = Login::new(user, hash, logins::digest(&logins::normalize_code(&recovery)));
    let invite = request["invite"].as_str().filter(|c| !c.trim().is_empty());
    match ctx.accounts.locked().register(login, invite, ctx.settings.registration) {
        Ok(_) => json!({ "ok": true, "user": user, "recovery_code": recovery }),
        Err(code) if ["user_id_taken", "invite_required", "invite_invalid", "registration_closed"].contains(&code.as_str()) => error(&code),
        Err(_) => error("server_error"),
    }
}

/// Checks `user`'s password, with the backoff after failures; the account's user id when it matches.
async fn check_password(user: &str, password: &str, ctx: &Context<'_>) -> Result<(u32, Login), Json> {
    if let Some(wait) = ctx.throttle.locked().user_wait(user, Instant::now()) { return Err(waiting(wait)) }
    let found = ctx.accounts.locked().login_of(user);
    let hash = found.as_ref().map(|(_, l)| l.password.clone());
    if !logins::verify_password(password.to_owned(), hash).await {
        ctx.throttle.locked().failed(user, Instant::now());
        return Err(error("invalid_credentials"));
    }
    ctx.throttle.locked().succeeded(user);
    found.ok_or_else(|| error("invalid_credentials"))
}

/// A session for `user_id` (ending its older one) and the reply around it.
fn start_session(user_id: u32, login: &Login, ctx: &Context<'_>) -> Result<Json, Json> {
    if let Some(refused) = login.refusal(logins::now()) { return Err(refused) }
    // During maintenance only the staff comes in (#26); the others see the server's message.
    if ctx.settings.in_maintenance(logins::now()) && !login.staff() {
        return Err(json!({ "ok": false, "error": "maintenance", "message": ctx.settings.maintenance_message }));
    }
    if !crate::store::healthy() { return Err(error("server_error")) }
    let session = ctx.sessions.locked().login(user_id, Instant::now()).ok_or_else(|| error("server_full"))?;
    Ok(json!({ "ok": true, "session": hex(&session), "user": login.user }))
}

async fn login(request: &Json, ctx: &Context<'_>) -> Json {
    let (user, password) = (text(request, "user").trim(), text(request, "password"));
    let (user_id, login) = match check_password(user, password, ctx).await { Ok(found) => found, Err(reply) => return reply };
    let mut reply = match start_session(user_id, &login, ctx) { Ok(reply) => reply, Err(reply) => return reply };
    if request["stay"].as_bool() == Some(true) {
        let token = logins::new_token();
        let now = logins::now();
        let label: String = text(request, "device").chars().filter(|c| !c.is_control()).take(40).collect();
        let device = Device { token: logins::digest(&token), label, created: now, last_used: now, expires: now + logins::DEVICE_TTL };
        if ctx.accounts.locked().add_device(user_id, device).is_ok() { reply["device_token"] = json!(token); }
    }
    reply
}

fn resume(request: &Json, ctx: &Context<'_>) -> Json {
    let digest = logins::digest(text(request, "device_token"));
    let now = logins::now();
    let mut accounts = ctx.accounts.locked();
    let Some(user_id) = accounts.device_user(&digest, now) else { return error("signed_out") };
    let Some(login) = accounts.login_by_id(user_id).cloned() else { return error("signed_out") };
    if login.refusal(now).is_none() && accounts.touch_device(user_id, &digest, now).is_err() { return error("server_error") }
    drop(accounts);
    start_session(user_id, &login, ctx).unwrap_or_else(|reply| reply)
}

fn sign_out(request: &Json, ctx: &Context<'_>) -> Json {
    let digest = logins::digest(text(request, "device_token"));
    let mut accounts = ctx.accounts.locked();
    let Some(user_id) = accounts.device_user(&digest, i64::MIN) else { return json!({ "ok": true }) };
    let done = if request["everywhere"].as_bool() == Some(true) {
        let done = accounts.revoke_devices(user_id).map(|()| true);
        drop(accounts);
        ctx.sessions.locked().logout_user(user_id);
        done
    } else { accounts.revoke_device(&digest) };
    match done { Ok(_) => json!({ "ok": true }), Err(_) => error("server_error") }
}

async fn change_password(request: &Json, ctx: &Context<'_>) -> Json {
    let user = text(request, "user").trim();
    let (user_id, login) = match check_password(user, text(request, "password"), ctx).await { Ok(found) => found, Err(reply) => return reply };
    let new = text(request, "new_password");
    if let Some(problem) = logins::password_problem(&login.user, new) { return error(problem) }
    let Ok(hash) = logins::hash_password(new.to_owned()).await else { return error("server_error") };
    match ctx.accounts.locked().set_password(user_id, hash) { Ok(()) => json!({ "ok": true }), Err(_) => error("server_error") }
}

async fn recover(request: &Json, ctx: &Context<'_>) -> Json {
    let user = text(request, "user").trim();
    if let Some(wait) = ctx.throttle.locked().user_wait(user, Instant::now()) { return waiting(wait) }
    let code = logins::digest(&logins::normalize_code(text(request, "code")));
    let found = ctx.accounts.locked().login_of(user);
    let now = logins::now();
    let by = found.as_ref().and_then(|(_, l)| if l.recovery == code { Some("recovery") }
        else if l.reset.as_ref().is_some_and(|(r, until)| *r == code && *until > now) { Some("reset") } else { None });
    let (Some((user_id, login)), Some(by)) = (found, by) else {
        ctx.throttle.locked().failed(user, Instant::now());
        return error("invalid_recovery");
    };
    ctx.throttle.locked().succeeded(user);
    let new = text(request, "new_password");
    if let Some(problem) = logins::password_problem(&login.user, new) { return error(problem) }
    let Ok(hash) = logins::hash_password(new.to_owned()).await else { return error("server_error") };
    let recovery = logins::new_code();
    let mut accounts = ctx.accounts.locked();
    if accounts.set_password(user_id, hash).is_err() { return error("server_error") }
    // A used recovery code is replaced; a reset code leaves the recovery code as it was (set_password ended the reset).
    if by == "recovery" {
        if accounts.set_recovery(user_id, logins::digest(&logins::normalize_code(&recovery))).is_err() { return error("server_error") }
        return json!({ "ok": true, "recovery_code": recovery });
    }
    json!({ "ok": true })
}
