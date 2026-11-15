//! Live settings (#24) on a running server: a change sent as `yo3-server set` sends it (the loopback admin port and its
//! token) applies without a restart, at the moment its setting says, and is saved. The banlist's "next login" is in
//! `front_contract.rs` and the LP and clocks' "next duel" in `pvp_contract.rs`.
use std::path::PathBuf;

use serde_json::json;
use yo3_server::admin;
use yo3_server::api::{self, Op};
use yo3_server::{Config, Running, start};
use yo3_tunnel::Endpoint;

struct Server { endpoint: Endpoint, data: PathBuf, _running: Running }

async fn server(test: &str) -> Server {
    yo3_server::logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-settings-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config::ephemeral(dir);
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    let endpoint = Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap();
    Server { endpoint, data, _running: running }
}

impl Server {
    /// What the host's command prints, sent to the running server as the CLI does.
    async fn admin(&self, op: Op) -> Result<String, String> {
        let live = admin::running(&self.data).expect("the server's admin port");
        tokio::task::spawn_blocking(move || api::call(&live, &op).map(|r| r["message"].as_str().unwrap_or_default().to_string())).await.unwrap()
    }
    async fn set(&self, name: &str, value: &str) -> String {
        self.admin(Op::Set { name: name.into(), value: value.into() }).await.unwrap()
    }
    async fn account(&self, request: serde_json::Value) -> serde_json::Value { self.endpoint.account(&request).await.unwrap() }
    async fn register(&self, user: &str) -> serde_json::Value {
        self.account(json!({ "op": "register", "user": user, "password": "a long passphrase", "rules": true })).await
    }
    /// The settings as saved in the database.
    fn saved(&self) -> std::sync::Arc<yo3_server::settings::Settings> {
        yo3_server::settings::Live::open(Some(&self.data.join("settings.json")), vec![]).unwrap().now()
    }
}

#[tokio::test]
async fn the_registration_mode_and_the_server_list_change_at_once_and_are_saved() {
    let s = server("at-once").await;
    assert_eq!(s.register("Yugi").await["ok"], true, "open on a test server");
    assert_eq!(s.set("registration", "closed").await, "registration is now closed (applies at once)");
    assert_eq!(s.register("Kaiba").await["error"], "registration_closed", "the next request, no restart");
    s.set("server-name", "Kaiba Land").await;
    s.set("headline", "Season 16 is on").await;
    let info = s.endpoint.info().await.unwrap();
    assert_eq!((info.name.as_str(), info.headline.as_str(), info.registration.as_str()), ("Kaiba Land", "Season 16 is on", "closed"));
    let saved = s.saved();
    assert_eq!((saved.server_name.as_str(), saved.registration), ("Kaiba Land", yo3_server::logins::Registration::Closed));
    // Back to the default, the same way.
    assert!(s.admin(Op::Reset { name: "server-name".into() }).await.unwrap().starts_with("server-name is now Yu-Gi-Oh! ONLINE 3 server"));
    assert_eq!(s.endpoint.info().await.unwrap().name, "Yu-Gi-Oh! ONLINE 3 server");
}

#[tokio::test]
async fn the_throttle_limits_change_at_once() {
    let s = server("limits").await;
    s.set("address-burst", "2").await;
    for _ in 0..2 { assert_eq!(s.account(json!({ "op": "nothing" })).await["error"], "unknown_op") }
    assert_eq!(s.account(json!({ "op": "nothing" })).await["error"], "throttled", "the third of a burst of two");
}

#[tokio::test]
async fn a_wrong_value_is_refused_and_changes_nothing() {
    let s = server("refused").await;
    let refused = s.admin(Op::Set { name: "start-lp".into(), value: "0".into() }).await.unwrap_err();
    assert!(refused.contains("100 to 99999"), "{refused}");
    assert!(s.admin(Op::Set { name: "nothing".into(), value: "1".into() }).await.unwrap_err().contains("no setting"));
    assert_eq!(s.saved().start_lp, 8000);
    let listed = s.admin(Op::Settings).await.unwrap();
    assert!(listed.contains("start-lp\t8000\t(default; applies at the next duel)"), "{listed}");
    assert!(listed.contains("registration\topen\t(set; applies at once)"), "{listed}");
}
