//! The account monkey: TLS operation 6 (#23) under valid, mutated and garbage requests from many local addresses.
//!
//! Each request comes from its own address in 127.0.0.0/8 (the server's per-address bucket would else answer most of
//! them "throttled"), and every answer must be a JSON object with "ok". A few flows are checked end to end: an account
//! made by the monkey signs in, keeps a device, resumes and signs out.
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use serde_json::{Value as Json, json};
use yo3_tunnel::Endpoint;

use crate::wire::Rng;

const PASSWORD: &str = "MonkeyPass1";

/// What the monkey saw: answers by error code, and what went wrong.
#[derive(Default)]
pub struct Stats {
    pub sent: usize,
    pub answers: HashMap<String, usize>,
    pub problems: Vec<String>,
}

/// The `i`th request's address: 127.2.0.1 onwards, 250 to a block.
fn address(i: u64) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(127, 2 + (i / 62_500) as u8, ((i / 250) % 250) as u8, (i % 250) as u8 + 1))
}

fn text(rng: &mut Rng) -> Json {
    match rng.below(7) {
        0 => json!(""),
        1 => json!("x".repeat(rng.range(1, 3000) as usize)),
        2 => json!(rng.range(0, u32::MAX)),
        3 => json!(null),
        4 => json!({ "nested": [1, 2, 3] }),
        5 => json!(String::from_utf8_lossy(&rng.blob(1, 40))),
        _ => json!(format!("user{}", rng.below(50))),
    }
}

/// A request with random values in the fields each op reads.
fn mutated(rng: &mut Rng) -> Json {
    let op = *rng.pick(&["register", "login", "resume", "sign_out", "change_password", "recover", "status", "", "LOGIN"]);
    let mut request = json!({ "op": op });
    for field in ["user", "password", "new_password", "invite", "code", "device_token", "device", "launcher", "rules", "stay", "everywhere"] {
        if rng.chance(0.6) { request[field] = text(rng); }
    }
    if rng.chance(0.3) { request["op"] = text(rng); }
    request
}

/// A body that is not a request: garbage, a JSON value of the wrong kind, a length that lies, or nothing at all.
fn garbage(rng: &mut Rng) -> Vec<u8> {
    let body: Vec<u8> = match rng.below(4) {
        0 => rng.blob(0, 300),
        1 => b"[\"login\"]".to_vec(),
        2 => b"{\"op\": \"login\", ".to_vec(),
        _ => json!("just a string").to_string().into_bytes(),
    };
    match rng.below(4) {
        0 => [&(body.len() as u16).to_be_bytes()[..], &body].concat(),
        1 => [&u16::MAX.to_be_bytes()[..], &body].concat(),
        2 => [&((body.len() as u16).saturating_add(10)).to_be_bytes()[..], &body].concat(),
        _ => body.into_iter().take(1).collect(),
    }
}

/// Every answer is an object with a boolean "ok", and "error" when it is false.
fn check(answer: &Json, what: &str, stats: &Mutex<Stats>) {
    let mut s = stats.lock().unwrap_or_else(|e| e.into_inner());
    let well_formed = answer.is_object() && answer["ok"].is_boolean() && (answer["ok"] == true || answer["error"].is_string());
    if !well_formed { s.problems.push(format!("{what}: a malformed answer {answer}")) }
    *s.answers.entry(answer["error"].as_str().unwrap_or("ok").to_string()).or_default() += 1;
}

/// One end-to-end flow from `from`: a new account signs in, keeps a device, resumes and signs out again.
async fn flow(endpoint: &Endpoint, user: &str, stats: &Mutex<Stats>) -> Result<(), String> {
    let ask = |request: Json| async move { endpoint.account(&request).await.map_err(|e| format!("{e:#}")) };
    let made = ask(json!({ "op": "register", "user": user, "password": PASSWORD, "rules": true })).await?;
    check(&made, "register", stats);
    if made["ok"] != true { return Err(format!("register {user}: {made}")) }
    let signed = ask(json!({ "op": "login", "user": user, "password": PASSWORD, "stay": true, "device": "monkey" })).await?;
    check(&signed, "login", stats);
    let token = signed["device_token"].as_str().ok_or(format!("login {user}: {signed}"))?.to_string();
    let resumed = ask(json!({ "op": "resume", "device_token": token })).await?;
    if resumed["session"].as_str().map(str::len) != Some(64) { return Err(format!("resume {user}: {resumed}")) }
    ask(json!({ "op": "sign_out", "device_token": token })).await?;
    let after = ask(json!({ "op": "resume", "device_token": token })).await?;
    if after["error"] != "signed_out" { return Err(format!("resume after sign-out {user}: {after}")) }
    Ok(())
}

/// Sends `requests` account requests, `par` at once; a tenth are full flows, the rest mutated or garbage.
pub async fn run(endpoint: &Endpoint, seed: u64, requests: u64, par: u64, stats: Arc<Mutex<Stats>>) {
    let mut i = 0;
    while i < requests {
        let batch: Vec<_> = (i..(i + par).min(requests)).map(|k| {
            let endpoint = endpoint.clone().bound_to(address(k));
            let stats = stats.clone();
            async move {
                let stats = &*stats;
                let mut rng = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(k));
                let kind = rng.below(10);
                let outcome = match kind {
                    0 => flow(&endpoint, &format!("m{seed}x{k}"), stats).await,
                    1..=2 => endpoint.raw(6, &garbage(&mut rng)).await.map(|_| ()).or_else(|e| {
                        // The server may close at once on a lying length: that is the right answer, not a failure.
                        if format!("{e:#}").contains("timed out") { Err(format!("garbage: {e:#}")) } else { Ok(()) }
                    }),
                    _ => {
                        let request = mutated(&mut rng);
                        let body = request.to_string().into_bytes();
                        if body.len() > yo3_tunnel::MAX_ACCOUNT_REQUEST {
                            // Over the limit: the server must close without answering.
                            match endpoint.raw(6, &[&(body.len() as u16).to_be_bytes()[..], &body].concat()).await {
                                Ok(reply) if reply.is_empty() => Ok(()),
                                Ok(reply) => Err(format!("an oversized request got an answer: {}", String::from_utf8_lossy(&reply))),
                                Err(e) => Err(format!("oversized: {e:#}")),
                            }
                        } else {
                            match endpoint.account(&request).await {
                                Ok(answer) => { check(&answer, "mutated", stats); Ok(()) }
                                Err(e) => Err(format!("mutated: {e:#}")),
                            }
                        }
                    }
                };
                let mut s = stats.lock().unwrap_or_else(|e| e.into_inner());
                s.sent += 1;
                if let Err(problem) = outcome { s.problems.push(format!("request {k}: {problem}")) }
            }
        }).collect();
        let n = batch.len() as u64;
        futures_join(batch).await;
        i += n;
    }
}

async fn futures_join<F: std::future::Future<Output = ()> + Send + 'static>(all: Vec<F>) {
    let tasks: Vec<_> = all.into_iter().map(tokio::spawn).collect();
    for task in tasks { let _ = task.await; }
}
