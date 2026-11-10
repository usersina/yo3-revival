//! Typed, token-authenticated desktop operations on the running server. Never exposed through the game tunnel.
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use crate::{accounts::Directory, admin::{Live, Response}, front::Hub, lock::Locked};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManagerOp {
    Status,
    Accounts,
    CreateAccount { name: String },
    AddPlayer { who: String, name: String },
    ExportAccount { who: String },
    Shutdown,
}

pub fn manage(live: &Live, op: &ManagerOp) -> Result<Value, String> {
    let call = || -> anyhow::Result<Value> {
        let mut stream = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], live.port)), Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let body = serde_json::to_vec(op)?;
        write!(stream, "POST /manager HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", live.token, body.len())?;
        stream.write_all(&body)?;
        let mut response = String::new();
        stream.take(1024 * 1024).read_to_string(&mut response)?;
        let (head, body) = response.split_once("\r\n\r\n").ok_or_else(|| anyhow::anyhow!("invalid manager response"))?;
        let value: Value = serde_json::from_str(body)?;
        anyhow::ensure!(head.starts_with("HTTP/1.1 200"), "{}", value["error"].as_str().unwrap_or("management request refused"));
        Ok(value)
    };
    call().map_err(|e| e.to_string())
}

#[derive(Clone)]
pub(crate) struct Manager {
    pub accounts: Arc<Mutex<Directory>>,
    pub hub: Arc<Mutex<Hub>>,
    pub shutdown: tokio::sync::watch::Sender<bool>,
    pub engine_problem: Option<String>,
    pub listen: SocketAddr,
}

impl Manager {
    pub fn handle(&self, request: &str, peer: IpAddr, token: Option<&str>) -> Response {
        let reply = |status, value: Value| Response { status, content_type: "application/json", headers: vec![], body: value.to_string().into_bytes() };
        let (head, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let given = head.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("authorization").then(|| value.trim().strip_prefix("Bearer ")).flatten()
        });
        if !peer.is_loopback() || !matches!((token, given), (Some(a), Some(b)) if crate::admin::same(a, b)) {
            return reply("403 Forbidden", json!({"error":"local administrator authentication required"}));
        }
        if !head.starts_with("POST /manager HTTP/1.") {
            return reply("405 Method Not Allowed", json!({"error":"use POST"}));
        }
        let op: ManagerOp = match serde_json::from_str(body) {
            Ok(op) => op, Err(_) => return reply("400 Bad Request", json!({"error":"invalid management request"})),
        };
        let result = self.apply(&op);
        // Only the operation name is logged: export replies contain account credentials.
        let name = match op { ManagerOp::Status => "status", ManagerOp::Accounts => "accounts", ManagerOp::CreateAccount { .. } => "create-account",
            ManagerOp::AddPlayer { .. } => "add-player", ManagerOp::ExportAccount { .. } => "export-account", ManagerOp::Shutdown => "shutdown" };
        if !matches!(op, ManagerOp::Status | ManagerOp::Accounts) {
            tracing::info!("[manager] {name} {}", if result.is_ok() { "ok" } else { "refused" });
        }
        match result { Ok(value) => reply("200 OK", value), Err(error) => reply("400 Bad Request", json!({"error":error})) }
    }

    fn apply(&self, op: &ManagerOp) -> Result<Value, String> {
        if matches!(op, ManagerOp::Status) {
            let hub = self.hub.locked();
            return Ok(json!({"ready":self.engine_problem.is_none(), "engine_problem":self.engine_problem,
                "listen":self.listen.to_string(), "release":yo3_tunnel::BUILD_VERSION,
                "players":hub.manager_players(), "online":hub.online(), "duels":hub.tables.len()}));
        }
        if matches!(op, ManagerOp::Shutdown) {
            self.shutdown.send_replace(true);
            return Ok(json!({"stopping":true}));
        }
        let mut accounts = self.accounts.locked();
        match op {
            ManagerOp::Accounts => Ok(Value::Array(accounts.entries().map(|(_, _, account)| json!({
                "user_id":account.user_id, "bp":account.dpoint,
                "players":account.players.iter().map(|p| json!({"player_id":p.player_id,"name":p.name,"slot":p.index})).collect::<Vec<_>>()
            })).collect())),
            ManagerOp::CreateAccount { name } => {
                accounts.atomic(|d| d.add_account(std::slice::from_ref(name)))?;
                Ok(json!({"created":name}))
            }
            ManagerOp::AddPlayer { who, name } => {
                let id = accounts.user_of(who).ok_or("account not found")?;
                accounts.atomic(|d| d.add_player_to(id, name))?;
                Ok(json!({"created":name}))
            }
            ManagerOp::ExportAccount { who } => Ok(json!({"key":accounts.atomic(|d| d.key_of(who))?})),
            ManagerOp::Status | ManagerOp::Shutdown => unreachable!(),
        }
    }
}
