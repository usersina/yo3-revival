//! The generated dashboard contract is collected from its actual routes, without state or a running server.
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yo3_server::{api::{self, Op}, Config, Running, logins};

const PASSWORD: &str = "LongPassphrase1";
const KEY: &str = "dashboard-grant-00000001";

#[test]
fn committed_openapi_matches_the_router_without_starting_a_server() {
    let generated: Value = serde_json::from_str(&yo3_server::dashboard::openapi_json()).unwrap();
    let committed: Value = serde_json::from_str(include_str!("../../../dashboard/api/openapi.json")).unwrap();
    assert_eq!(generated, committed, "run npm run api:generate in dashboard/");
    let paths = generated["paths"].as_object().unwrap();
    let expected = [
        ("/panel/api/v1/session", "get", "getStaffSession"),
        ("/panel/api/v1/sign-in", "post", "signInStaff"),
        ("/panel/api/v1/sign-out", "post", "signOutStaff"),
        ("/panel/api/v1/accounts", "get", "listAccounts"),
        ("/panel/api/v1/give-bp", "post", "giveBp"),
        ("/panel/api/v1/bp-codes", "get", "listBpCodes"),
        ("/panel/api/v1/bp-codes", "post", "createBpCode"),
        ("/panel/api/v1/bp-codes/remove", "post", "removeBpCode"),
        ("/panel/api/v1/invites", "get", "listInvites"),
        ("/panel/api/v1/invites", "post", "createInvite"),
        ("/panel/api/v1/accounts/{user_id}", "get", "getAccount"),
        ("/panel/api/v1/accounts/edit", "post", "editAccount"),
        ("/panel/api/v1/countries", "get", "listCountries"),
        ("/panel/api/v1/accounts/credentials", "post", "issueAccountCode"),
        ("/panel/api/v1/accounts/keys", "post", "manageAccountKey"),
        ("/panel/api/v1/gifts", "post", "givePlayerGift"),
        ("/panel/api/v1/settings", "get", "listSettings"),
        ("/panel/api/v1/settings", "post", "changeSettings"),
        ("/panel/api/v1/moderation", "post", "moderateAccount"),
        ("/panel/api/v1/gifts/choices", "get", "listGiftChoices"),
    ];
    assert_eq!(paths.len(), 17);
    for (path, method, id) in expected {
        let op = &generated["paths"][path][method];
        assert_eq!(op["operationId"], id);
        for status in ["400", "401", "403", "405", "413", "415", "500"] {
            assert_eq!(op["responses"][status]["content"]["application/json"]["schema"]["$ref"], "#/components/schemas/ApiError", "{path} {status}");
        }
        if method == "post" {
            assert!(op["parameters"].as_array().unwrap().iter().any(|p| p["name"] == "Idempotency-Key" && p["in"] == "header" && p["required"] == true));
        }
    }
    assert_eq!(generated["components"]["schemas"]["StaffSession"]["properties"]["expires_at"]["format"], "date-time");
}

struct Browser { live: Running, admin: yo3_server::admin::Live, endpoint: yo3_tunnel::Endpoint, cookie: String, csrf: String }
struct Answer { status: u16, headers: String, body: Value }
impl Browser {
    async fn new(name: &str) -> Self {
        Self::with_dashboard(name, true).await
    }
    async fn with_dashboard(name: &str, enabled: bool) -> Self {
        logins::use_fast_hashing();
        let dir = std::env::temp_dir().join(format!("yo3-dashboard-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut cfg = Config::ephemeral(dir);
        cfg.dashboard_port = enabled.then_some(0);
        let data = cfg.data_dir.clone().unwrap();
        cfg.game_dir = data.join("no-game-files-needed");
        let cert = cfg.tls.certificate.clone();
        let live = yo3_server::start(cfg).await.unwrap();
        let endpoint = yo3_tunnel::Endpoint::new(live.auth.to_string(), "localhost".into(), &cert).unwrap();
        let admin = yo3_server::admin::running(&data).unwrap();
        for (user, role) in [("Pegasus", "admin"), ("Croquet", "moderator"), ("Yugi", "player")] {
            let result = endpoint.account(&json!({ "op": "register", "user": user, "password": PASSWORD, "rules": true })).await.unwrap();
            assert_eq!(result["ok"], true);
            let token = admin.clone();
            tokio::task::spawn_blocking(move || api::call(&token, &Op::SetRole { user: user.into(), role: role.into() })).await.unwrap().unwrap();
        }
        Self { live, admin, endpoint, cookie: String::new(), csrf: String::new() }
    }
    async fn raw(&self, method: &str, path: &str, body: &str, extra: &str) -> Answer {
        self.raw_on(self.live.dashboard.unwrap_or(self.live.http), method, path, body, extra).await
    }
    async fn raw_on(&self, addr: std::net::SocketAddr, method: &str, path: &str, body: &str, extra: &str) -> Answer {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let raw = format!("{method} {path} HTTP/1.1\r\nHost: localhost:{}\r\nOrigin: http://localhost:{}\r\nCookie: {}\r\nX-CSRF-Token: {}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", addr.port(), addr.port(), self.cookie, self.csrf, body.len());
        stream.write_all(raw.as_bytes()).await.unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).await.unwrap();
        let (head, body) = answer.split_once("\r\n\r\n").unwrap();
        let status = head.lines().next().unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
        Answer { status, headers: head.into(), body: serde_json::from_str(body).unwrap_or_else(|_| json!({ "raw": body })) }
    }
    async fn call(&self, method: &str, path: &str, body: Value) -> Answer {
        self.raw(method, path, &body.to_string(), &format!("Idempotency-Key: {KEY}\r\n")).await
    }
    async fn sign_in(&mut self, user: &str) {
        let a = self.call("POST", "/panel/api/v1/sign-in", json!({ "user": user, "password": PASSWORD })).await;
        assert_eq!(a.status, 200, "{}", a.body);
        self.cookie = a.headers.lines().find_map(|l| l.to_ascii_lowercase().starts_with("set-cookie:").then(|| l.split_once(':').unwrap().1.trim().split(';').next().unwrap().to_string())).unwrap();
        self.csrf = a.body["csrf_token"].as_str().unwrap().to_string();
        for flag in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/panel"] { assert!(a.headers.contains(flag)); }
    }
}

#[tokio::test]
async fn management_does_not_mount_dashboard_routes_by_default() {
    let b = Browser::with_dashboard("management-only", false).await;
    assert!(b.live.dashboard.is_none());
    for (method, path, body) in [
        ("GET", "/panel/api/v1/session", json!(null)),
        ("GET", "/panel/api/v1/accounts", json!(null)),
        ("POST", "/panel/api/v1/sign-in", json!({ "user": "Pegasus", "password": PASSWORD })),
        ("GET", "/panel/openapi.json", json!(null)),
    ] {
        let response = b.call(method, path, body).await;
        assert_eq!(response.status, 404, "{path} belongs only to the dedicated dashboard listener");
    }
    let admin = b.admin.clone();
    let status = tokio::task::spawn_blocking(move || api::call(&admin, &Op::Status)).await.unwrap().unwrap();
    assert!(status.is_object(), "local host administration remains available");
}

#[tokio::test]
async fn real_listeners_keep_host_authority_assets_and_staff_routes_separate() {
    let mut b = Browser::new("listeners").await;
    let dashboard = b.live.dashboard.expect("dashboard explicitly enabled");
    assert_ne!(dashboard, b.live.http);
    assert!(dashboard.ip().is_loopback());
    assert_eq!(b.admin.port, b.live.http.port(), "CLI discovery must never name the staff port");
    let token = format!("Authorization: Bearer {}\r\n", b.admin.token);
    assert_eq!(b.raw("GET", "/panel/api/v1/accounts", "", &token).await.status, 401);
    for signed_in in [false, true] {
        if signed_in { b.sign_in("Pegasus").await; }
        for (method, path, body) in [
            ("POST", "/api/admin/v1/status", json!({ "op": "status" })),
            ("POST", "/api/admin/v1/give-bp", json!({ "op": "give-bp", "who": "Yugi", "amount": 99 })),
            ("POST", "/api/admin/v1/key", json!({ "op": "key", "who": "Yugi" })),
            ("POST", "/api/admin/v1/restore", json!({ "which": "latest" })),
            ("POST", "/panel/api/v1/restore", json!({ "which": "latest" })),
            ("GET", "/panel/backups", json!(null)),
            ("GET", "/panel/", json!(null)),
            ("GET", "/health", json!(null)),
            ("GET", "/yo3/yo3.idx", json!(null)),
            ("GET", "/web/v3/bpass/index/", json!(null)),
        ] {
            let reply = b.raw(method, path, &body.to_string(), &token).await;
            assert_eq!((reply.status, reply.body["code"].as_str()), (404, Some("not_found")), "{path}; staff cookie: {signed_in}");
            assert!(reply.headers.to_ascii_lowercase().contains("cache-control: no-store"));
        }
    }
    for path in ["/panel/api/v1/session", "/panel/api/v1/accounts", "/panel/openapi.json"] {
        assert_eq!(b.raw_on(b.live.http, "GET", path, "", &token).await.status, 404);
    }
    assert_eq!(b.raw_on(b.live.http, "POST", "/panel/api/v1/sign-in",
        &json!({ "user": "Pegasus", "password": PASSWORD }).to_string(), &format!("Idempotency-Key: {KEY}\r\n")).await.status, 404);
    assert_eq!(b.call("GET", "/panel/api/v1/accounts", json!(null)).await.status, 200);
    // Local commands still work when a staff listener exists, and stop closes all three listeners.
    let local = b.live.http;
    let auth = b.live.auth;
    let admin = b.admin.clone();
    tokio::task::spawn_blocking(move || {
        api::call(&admin, &Op::GiveBp { who: "Yugi".into(), amount: 17 }).unwrap();
        api::call(&admin, &Op::Shutdown).unwrap();
    }).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), b.live.wait()).await.unwrap();
    for addr in [local, auth, dashboard] {
        assert!(tokio::net::TcpStream::connect(addr).await.is_err(), "{addr} closed at shutdown");
    }
}

#[tokio::test]
async fn dashboard_bind_failure_stops_startup_before_opening_the_database() {
    let blocked = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = blocked.local_addr().unwrap().port();
    let root = std::env::temp_dir().join(format!("yo3-dashboard-bind-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut cfg = Config::ephemeral(root.clone());
    cfg.dashboard_port = Some(port);
    // Config::ephemeral seeds a fixture database for TLS tests; use a separate unopened data folder here.
    cfg.data_dir = Some(root.join("unopened-data"));
    let data = cfg.data_dir.clone().unwrap();
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("existing-data"), b"preserve").unwrap();
    let error = match yo3_server::start(cfg.clone()).await { Ok(_) => panic!("a requested dashboard must not silently disappear"), Err(e) => e.to_string() };
    assert!(error.contains("dashboard") && error.contains(&port.to_string()), "{error}");
    assert!(!data.join("server.db").exists(), "failed bind must not migrate or initialize state");
    assert!(!data.join("admin.json").exists(), "failed startup must not publish live management discovery");
    assert_eq!(std::fs::read(data.join("existing-data")).unwrap(), b"preserve");
    drop(blocked);
    cfg.bind = "0.0.0.0".parse().unwrap();
    let running = yo3_server::start(cfg).await.unwrap();
    assert!(running.auth.ip().is_unspecified());
    assert!(running.http.ip().is_loopback(), "the game's bind never exposes local management");
    assert!(running.dashboard.unwrap().ip().is_loopback(), "nor the unfinished dashboard API");
    running.stopper().send_replace(true);
    running.wait().await;
    yo3_server::store::close(&data);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn typed_staff_api_rejects_host_authority_and_replays_a_grant_without_paying_twice() {
    let mut b = Browser::new("grant").await;
    let token = format!("Authorization: Bearer {}\r\n", b.admin.token);
    let a = b.raw("GET", "/panel/api/v1/accounts", "", &token).await;
    assert_eq!((a.status, a.body["code"].as_str()), (401, Some("unauthenticated")));
    assert_eq!(b.call("POST", "/panel/api/v1/restore", json!({})).await.status, 404);
    b.sign_in("Pegasus").await;
    let session = b.call("GET", "/panel/api/v1/session", json!(null)).await;
    assert_eq!(session.body["user"], "Pegasus");
    assert!(chrono::DateTime::parse_from_rfc3339(session.body["expires_at"].as_str().unwrap()).is_ok());
    let accounts = b.call("GET", "/panel/api/v1/accounts", json!(null)).await;
    assert_eq!(accounts.status, 200);
    let yugi = accounts.body["accounts"].as_array().unwrap().iter().find(|a| a["login"] == "Yugi").unwrap();
    let before = yugi["bp"].as_u64().unwrap();
    let grant = json!({ "user_id": yugi["user_id"], "amount": 100 });
    let first = b.call("POST", "/panel/api/v1/give-bp", grant.clone()).await;
    assert_eq!(first.status, 200, "{}", first.body);
    assert_eq!(first.body["bp"], before + 100);
    let again = b.call("POST", "/panel/api/v1/give-bp", grant).await;
    assert_eq!(again.body, first.body);
    let conflict = b.call("POST", "/panel/api/v1/give-bp", json!({ "user_id": yugi["user_id"], "amount": 101 })).await;
    assert_eq!((conflict.status, conflict.body["code"].as_str()), (409, Some("request_conflict")));
    let invalid = b.raw("POST", "/panel/api/v1/give-bp", "{", &format!("Idempotency-Key: {KEY}\r\n")).await;
    assert_eq!((invalid.status, invalid.body["code"].as_str()), (400, Some("invalid_request")));
    let absent = b.raw("POST", "/panel/api/v1/give-bp", r#"{"user_id":1004,"amount":100}"#, "").await;
    assert_eq!(absent.status, 400);
    let denied = b.raw("POST", "/panel/api/v1/give-bp", "{}", "Sec-Fetch-Site: cross-site\r\n").await;
    assert_eq!(denied.status, 403);
    for header in ["content-security-policy", "cache-control: no-store", "x-content-type-options: nosniff"] {
        assert!(first.headers.to_ascii_lowercase().contains(header));
    }
    assert_eq!(b.call("POST", "/panel/api/v1/sign-out", json!({})).await.status, 200);
    assert_eq!(b.call("GET", "/panel/api/v1/accounts", json!(null)).await.status, 401);
}

#[tokio::test]
async fn moderator_permissions_and_demotion_are_checked_on_the_server() {
    let mut b = Browser::new("roles").await;
    let refused = b.call("POST", "/panel/api/v1/sign-in", json!({ "user": "Yugi", "password": PASSWORD })).await;
    assert_eq!((refused.status, refused.body["code"].as_str()), (401, Some("unauthenticated")));
    b.sign_in("Croquet").await;
    assert_eq!(b.call("GET", "/panel/api/v1/accounts", json!(null)).await.status, 200);
    let denied = b.call("POST", "/panel/api/v1/give-bp", json!({ "user_id": 1004, "amount": 100 })).await;
    assert_eq!((denied.status, denied.body["code"].as_str()), (403, Some("forbidden")));
    let admin = b.admin.clone();
    tokio::task::spawn_blocking(move || api::call(&admin, &Op::SetRole { user: "Croquet".into(), role: "player".into() })).await.unwrap().unwrap();
    assert_eq!(b.call("GET", "/panel/api/v1/accounts", json!(null)).await.status, 401);
}

#[tokio::test]
async fn staff_codes_use_typed_results_retry_conflicts_and_server_side_permissions() {
    let mut b = Browser::new("codes").await;
    let token = format!("Authorization: Bearer {}\r\n", b.admin.token);
    assert_eq!(b.raw("GET", "/panel/api/v1/bp-codes", "", &token).await.status, 401);
    b.sign_in("Pegasus").await;
    let request = json!({ "bp": 300, "uses": 2 });
    let first = b.call("POST", "/panel/api/v1/bp-codes", request.clone()).await;
    assert_eq!(first.status, 200, "{}", first.body);
    let result: yo3_server::api::codes::CreateBpCodeResult = serde_json::from_value(first.body.clone()).unwrap();
    assert_eq!(result.code.len(), 16);
    assert_eq!(result.request_id, KEY);
    assert_eq!(b.call("POST", "/panel/api/v1/bp-codes", request).await.body, first.body);
    let changed = b.call("POST", "/panel/api/v1/bp-codes", json!({ "bp": 301 })).await;
    assert_eq!((changed.status, changed.body["code"].as_str()), (409, Some("request_conflict")));
    let duplicate = b.raw("POST", "/panel/api/v1/bp-codes", &json!({ "code": result.code, "bp": 300 }).to_string(),
        "Idempotency-Key: duplicate-code-000000001\r\n").await;
    assert_eq!((duplicate.status, duplicate.body["code"].as_str()), (409, Some("already_exists")));
    let malformed = b.raw("POST", "/panel/api/v1/bp-codes", &json!({ "bp": 300, "role": "admin" }).to_string(),
        "Idempotency-Key: invalid-code-000000001\r\n").await;
    assert_eq!(malformed.status, 400);
    b.sign_in("Croquet").await;
    let list = b.call("GET", "/panel/api/v1/bp-codes", json!(null)).await;
    assert_eq!(list.status, 200);
    let parsed: yo3_server::api::codes::BpCodeList = serde_json::from_value(list.body.clone()).unwrap();
    assert_eq!(parsed.codes.len(), 1);
    assert_eq!(parsed.codes[0].bp, 300);
    assert!(!list.body.to_string().contains(&b.admin.token));
    for (path, body) in [("/panel/api/v1/bp-codes", json!({ "code": result.code, "bp": 300 })),
        ("/panel/api/v1/bp-codes/remove", json!({ "code": result.code }))] {
        assert_eq!(b.call("POST", path, body).await.status, 403);
    }
    b.sign_in("Pegasus").await;
    let body = json!({ "code": result.code.to_lowercase() }).to_string();
    let headers = "Idempotency-Key: remove-code-000000001\r\n";
    let removed = b.raw("POST", "/panel/api/v1/bp-codes/remove", &body, headers).await;
    assert_eq!(removed.status, 200);
    assert_eq!(removed.body["removed"], result.code);
    assert_eq!(b.raw("POST", "/panel/api/v1/bp-codes/remove", &body, headers).await.body, removed.body);
    assert_eq!(b.call("GET", "/panel/api/v1/bp-codes", json!(null)).await.body["codes"], json!([]));
    assert_eq!(b.raw_on(b.live.http, "GET", "/panel/api/v1/bp-codes", "", &token).await.status, 404);
}

#[tokio::test]
async fn forged_forwarding_headers_never_turn_a_remote_peer_or_host_into_local_authority() {
    use axum::{body::{Body, to_bytes}, extract::ConnectInfo, http::Request};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;
    let api = api::Api::new(Arc::new(Mutex::new(yo3_server::accounts::Directory::new(vec![]))),
        Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap()), None, Default::default(), "server");
    let router = yo3_server::dashboard::router(yo3_server::dashboard::Dashboard {
        api: Arc::new(api), panel: Arc::new(yo3_server::panel::Panel::new()),
    });
    for (peer, host) in [("203.0.113.1:40000", "localhost"), ("127.0.0.1:40000", "attacker.example")] {
        let request = Request::builder().uri("/panel/api/v1/accounts").header("Host", host)
            .header("X-Forwarded-For", "127.0.0.1").header("Forwarded", "for=127.0.0.1;host=localhost")
            .extension(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap())).body(Body::empty()).unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), 403);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let error: yo3_server::dashboard::ApiError = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(serde_json::to_value(error).unwrap()["code"], "forbidden");
    }
    for path in ["/api/admin/v1/status", "/api/admin/v1/key", "/api/admin/v1/restore"] {
        let response = router.clone().oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), 404, "host-only route {path} must not exist in the staff router");
    }
}

#[tokio::test]
async fn invalid_json_csrf_headers_methods_and_payload_sizes_return_declared_errors() {
    let mut b = Browser::new("errors").await;
    let huge = "x".repeat(17 * 1024);
    let large = b.raw("POST", "/panel/api/v1/sign-in", &huge, &format!("Idempotency-Key: {KEY}\r\n")).await;
    assert_eq!((large.status, large.body["code"].as_str()), (413, Some("payload_too_large")));
    b.sign_in("Pegasus").await;
    let method = b.call("DELETE", "/panel/api/v1/accounts", json!({})).await;
    assert_eq!((method.status, method.body["code"].as_str()), (405, Some("method_not_allowed")));
    let duplicate = b.raw("POST", "/panel/api/v1/give-bp", "{}", &format!("Idempotency-Key: {KEY}\r\nIdempotency-Key: {KEY}\r\n")).await;
    assert_eq!(duplicate.status, 400);
    let unknown = b.call("POST", "/panel/api/v1/give-bp", json!({ "user_id": 1, "amount": 10, "role": "admin" })).await;
    assert_eq!(unknown.status, 400, "unknown request fields never become authority");
    b.csrf = "wrong".into();
    let csrf = b.call("POST", "/panel/api/v1/give-bp", json!({ "user_id": 1, "amount": 10 })).await;
    assert_eq!((csrf.status, csrf.body["code"].as_str()), (403, Some("csrf_invalid")));
    let spec = b.raw("GET", "/panel/openapi.json", "", "").await;
    assert_eq!(spec.body, serde_json::from_str::<Value>(&yo3_server::dashboard::openapi_json()).unwrap());
}

#[tokio::test]
async fn password_changes_and_recovery_end_existing_html_and_json_staff_cookies() {
    let mut b = Browser::new("credentials").await;
    b.sign_in("Pegasus").await;
    let changed = b.endpoint.account(&json!({ "op": "change_password", "user": "Pegasus", "password": PASSWORD,
        "new_password": "OtherLongPass1" })).await.unwrap();
    assert_eq!(changed["ok"], true, "{changed}");
    assert_eq!(b.call("GET", "/panel/api/v1/session", json!(null)).await.status, 401);
    // The legacy panel and JSON API share the same cookie and validation.
    let old_panel = b.raw_on(b.live.http, "GET", "/panel/", "", "").await;
    assert_eq!(old_panel.status, 303);
    b.sign_in("Croquet").await;
    let admin = b.admin.clone();
    let reset = tokio::task::spawn_blocking(move || api::call(&admin, &Op::ResetCode { user: "Croquet".into(), code: logins::new_code().into() })).await.unwrap().unwrap();
    let recovered = b.endpoint.account(&json!({ "op": "recover", "user": "Croquet", "code": reset["code"],
        "new_password": "RecoveryPass123" })).await.unwrap();
    assert_eq!(recovered["ok"], true, "{recovered}");
    assert_eq!(b.call("GET", "/panel/api/v1/session", json!(null)).await.status, 401);
    assert_eq!(b.raw_on(b.live.http, "GET", "/panel/", "", "").await.status, 303);
}

#[tokio::test]
async fn staff_invites_are_typed_receipted_and_usable_for_real_registration() {
    use yo3_server::api::invites::{CreateInviteResult, InviteList, InviteState};
    let mut b = Browser::new("invites").await;
    let token = format!("Authorization: Bearer {}\r\n", b.admin.token);
    assert_eq!(b.raw("GET", "/panel/api/v1/invites", "", &token).await.status, 401);
    b.sign_in("Croquet").await;
    let empty = b.call("GET", "/panel/api/v1/invites", json!(null)).await;
    assert_eq!(empty.status, 200);
    assert!(serde_json::from_value::<InviteList>(empty.body).unwrap().invites.is_empty());
    let body = json!({ "uses": 1, "days": 7 });
    let first = b.call("POST", "/panel/api/v1/invites", body.clone()).await;
    assert_eq!(first.status, 200, "{}", first.body);
    let result: CreateInviteResult = serde_json::from_value(first.body.clone()).unwrap();
    assert_eq!(b.call("GET", "/panel/api/v1/invites", json!(null)).await.body["invites"][0]["issued_by"], "Croquet");
    assert_eq!(result.request_id, KEY);
    assert!(result.expires_at.is_some());
    assert_eq!(b.call("POST", "/panel/api/v1/invites", body.clone()).await.body, first.body);
    let conflict = b.call("POST", "/panel/api/v1/invites", json!({ "uses": 2, "days": 7 })).await;
    assert_eq!((conflict.status, conflict.body["code"].as_str()), (409, Some("request_conflict")));
    for invalid in [json!({ "uses": 0 }), json!({ "days": 0 }), json!({ "issued_by": "Pegasus" }), json!({ "days": 4294967295_u32 })] {
        let refused = b.raw("POST", "/panel/api/v1/invites", &invalid.to_string(), "Idempotency-Key: invalid-invite-00000001\r\n").await;
        assert_eq!(refused.status, 400);
    }
    let admin = b.admin.clone();
    tokio::task::spawn_blocking(move || api::call(&admin, &Op::Set { name: "registration".into(), value: "invite".into() })).await.unwrap().unwrap();
    let registered = b.endpoint.account(&json!({ "op": "register", "user": "Joey", "password": PASSWORD, "invite": result.code, "rules": true })).await.unwrap();
    assert_eq!(registered["ok"], true, "{registered}");
    let list: InviteList = serde_json::from_value(b.call("GET", "/panel/api/v1/invites", json!(null)).await.body).unwrap();
    assert_eq!(list.invites[0].state, InviteState::Exhausted);
    assert_eq!(list.invites[0].uses, Some(0));
    assert_eq!(list.invites[0].claims.len(), 1);
    assert_eq!(b.call("POST", "/panel/api/v1/invites", body).await.body, first.body, "retry must not replenish the invite");
    let refused = b.endpoint.account(&json!({ "op": "register", "user": "Mai", "password": PASSWORD, "invite": result.code, "rules": true })).await.unwrap();
    assert_eq!(refused["error"], "invite_invalid");
    assert_eq!(b.raw_on(b.live.http, "GET", "/panel/api/v1/invites", "", &token).await.status, 404);
    b.sign_in("Pegasus").await;
    assert_eq!(b.call("POST", "/panel/api/v1/invites", json!({})).await.status, 200, "admin and moderator have separate receipt scopes");
}

#[tokio::test]
async fn staff_account_detail_and_edits_use_stable_ids_without_credentials() {
    let mut b = Browser::new("profiles").await;
    b.sign_in("Pegasus").await;
    let list = b.call("GET", "/panel/api/v1/accounts", json!(null)).await;
    let user = list.body["accounts"].as_array().unwrap().iter().find(|a| a["login"] == "Yugi").unwrap()["user_id"].as_u64().unwrap();
    let path = format!("/panel/api/v1/accounts/{user}");
    let detail = b.call("GET", &path, json!(null)).await;
    assert_eq!(detail.status, 200);
    assert_eq!(detail.body["user_id"], user);
    assert_eq!(detail.body["players"], json!([]));
    for forbidden in ["password", "key", "recovery", "devices", "token"] { assert!(detail.body.get(forbidden).is_none()); }
    let edit = "/panel/api/v1/accounts/edit";
    let first = b.call("POST", edit, json!({ "kind": "add_player", "user_id": user, "name": "Yugi" })).await;
    assert_eq!(first.status, 200, "{}", first.body);
    assert_eq!(b.call("POST", edit, json!({ "kind": "add_player", "user_id": user, "name": "Yugi" })).await.body, first.body);
    let player = first.body["player_id"].as_u64().unwrap();
    b.sign_in("Croquet").await;
    let rename = json!({ "kind": "rename_player", "user_id": user, "player_id": player, "name": "Atem" });
    let renamed = b.call("POST", edit, rename.clone()).await;
    assert_eq!(renamed.status, 200, "{}", renamed.body);
    assert_eq!(b.call("POST", edit, rename).await.body, renamed.body);
    assert_eq!(b.call("GET", &path, json!(null)).await.body["players"][0]["name"], "Atem");
    for kind in [json!({ "kind": "add_player", "user_id": user, "name": "Joey" }), json!({ "kind": "set_country", "user_id": user, "country": null })] {
        assert_eq!(b.raw("POST", edit, &kind.to_string(), "Idempotency-Key: mod-admin-edit-000001\r\n").await.status, 403);
    }
    assert_eq!(b.call("GET", "/panel/api/v1/accounts/not-an-id", json!(null)).await.status, 400);
    assert_eq!(b.call("GET", "/panel/api/v1/accounts/4294967295", json!(null)).await.status, 404);
    let countries = b.call("GET", "/panel/api/v1/countries", json!(null)).await;
    assert_eq!(countries.status, 503, "no game assets means explicit failure, never invented choices");
    b.sign_in("Pegasus").await;
    let malformed = json!({ "kind": "rename_player", "user_id": user, "player_id": player, "name": "Joey", "role": "admin" });
    assert_eq!(b.raw("POST", edit, &malformed.to_string(), "Idempotency-Key: malformed-edit-000001\r\n").await.status, 400);
    let reset = b.raw("POST", edit, &json!({ "kind": "set_country", "user_id": user }).to_string(), "Idempotency-Key: country-edit-00000001\r\n").await;
    assert_eq!(reset.status, 200);
    assert_eq!(b.call("GET", &path, json!(null)).await.body["country"], json!(null));
    let token = format!("Authorization: Bearer {}\r\n", b.admin.token);
    assert_eq!(b.raw_on(b.live.http, "GET", &path, "", &token).await.status, 404);
}

#[tokio::test]
async fn role_edits_are_admin_only_and_revoke_staff_cookies_on_both_listeners() {
    let mut b = Browser::new("role-edits").await;
    b.sign_in("Pegasus").await;
    let list = b.call("GET", "/panel/api/v1/accounts", json!(null)).await;
    let user = list.body["accounts"].as_array().unwrap().iter().find(|a| a["login"] == "Yugi").unwrap()["user_id"].clone();
    let req = json!({ "kind": "set_role", "user_id": user, "role": "moderator" });
    let response = b.call("POST", "/panel/api/v1/accounts/edit", req.clone()).await;
    assert_eq!(response.status, 200, "{}", response.body);
    assert_eq!(b.call("POST", "/panel/api/v1/accounts/edit", req).await.body, response.body);
    assert_eq!(b.call("POST", "/panel/api/v1/accounts/edit", json!({ "kind": "set_role", "user_id": user, "role": "admin" })).await.status, 409);
    assert_eq!(b.call("POST", "/panel/api/v1/accounts/edit", json!({ "kind": "set_role", "user_id": user, "role": "superuser" })).await.status, 400);
    b.sign_in("Yugi").await;
    assert_eq!(b.call("POST", "/panel/api/v1/accounts/edit", json!({ "kind": "set_role", "user_id": user, "role": "admin" })).await.status, 403);
    for role in ["player", "moderator"] {
        let admin = b.admin.clone();
        tokio::task::spawn_blocking(move || api::call(&admin, &Op::SetRole { user: "Yugi".into(), role: role.into() })).await.unwrap().unwrap();
    }
    assert_eq!(b.call("GET", "/panel/api/v1/session", json!(null)).await.status, 401);
    assert_eq!(b.raw_on(b.live.http, "GET", "/panel/", "", "").await.status, 303, "legacy panel rejects the same stale staff cookie");
}

#[tokio::test]
async fn provisioned_accounts_choose_their_own_password_and_code_retries_do_not_reactivate_them() {
    use yo3_server::api::credentials::{AccountCodeResponse, AccountCodeState};
    let mut b = Browser::new("setup-codes").await; b.sign_in("Pegasus").await;
    let path = "/panel/api/v1/accounts/credentials";
    let code = "ABCD-EFGH-JKLM-NPQR";
    let request = json!({ "kind": "create_account", "user": "NewOwner", "code": code });
    let token = format!("Authorization: Bearer {}\r\n", b.admin.token);
    assert_eq!(b.raw_on(b.live.http, "POST", path, &request.to_string(), &token).await.status, 404);
    let first = b.call("POST", path, request.clone()).await; assert_eq!(first.status, 200, "{}", first.body);
    let confirmation: AccountCodeResponse = serde_json::from_value(first.body.clone()).unwrap(); assert_eq!(confirmation.state, AccountCodeState::Ready);
    assert!(!first.body.to_string().contains(code)); assert!(first.body.get("code").is_none());
    let user = confirmation.confirmation.user_id;
    assert_eq!(b.call("GET", &format!("/panel/api/v1/accounts/{user}"), json!(null)).await.body["password_ready"], false);
    let login = b.endpoint.account(&json!({ "op": "login", "user": "newowner", "password": "anything" })).await.unwrap(); assert_eq!(login["ok"], false);
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await; // the failed sign-in also throttles recovery
    let recovered = b.endpoint.account(&json!({ "op": "recover", "user": "newowner", "code": code, "new_password": "ChosenPassword1" })).await.unwrap();
    assert_eq!(recovered["ok"], true, "{recovered}"); assert!(recovered["recovery_code"].is_string(), "first setup must give the owner a private recovery code");
    let retry: AccountCodeResponse = serde_json::from_value(b.call("POST", path, request).await.body).unwrap();
    assert_eq!(retry.confirmation, confirmation.confirmation); assert_eq!(retry.state, AccountCodeState::UsedOrReplaced);
    let signed = b.endpoint.account(&json!({ "op": "login", "user": "newowner", "password": "ChosenPassword1" })).await.unwrap(); assert_eq!(signed["ok"], true);
    let refused = b.endpoint.account(&json!({ "op": "recover", "user": "newowner", "code": code, "new_password": "StolenPassword1" })).await.unwrap(); assert_eq!(refused["error"], "invalid_recovery");
    assert_eq!(b.call("POST", path, json!({ "kind": "create_account", "user": "Someone", "code": code, "role": "admin" })).await.status, 400);
    b.sign_in("Croquet").await;
    assert_eq!(b.call("POST", path, json!({ "kind": "create_account", "user": "Someone", "code": code })).await.status, 403);
    let admin_id = b.call("GET", "/panel/api/v1/accounts", json!(null)).await.body["accounts"].as_array().unwrap().iter().find(|a| a["login"] == "Pegasus").unwrap()["user_id"].clone();
    assert_eq!(b.call("POST", path, json!({ "kind": "reset_password", "user_id": admin_id, "code": code })).await.status, 403);
    let reset = b.call("POST", path, json!({ "kind": "reset_password", "user_id": user, "code": "QRST-UVWX-YZ23-4567" })).await; assert_eq!(reset.status, 200);
    assert_eq!(b.call("POST", path, json!({ "kind": "reset_password", "user_id": u32::MAX, "code": code })).await.status, 409, "request IDs share one namespace for the actor");
}

#[tokio::test]
async fn signing_out_everywhere_revokes_both_management_cookies() {
    let mut b = Browser::new("all-staff-sessions").await; b.sign_in("Croquet").await;
    let admin = b.admin.clone(); tokio::task::spawn_blocking(move || api::call(&admin, &Op::SignOut { who: "Croquet".into() })).await.unwrap().unwrap();
    assert_eq!(b.call("GET", "/panel/api/v1/session", json!(null)).await.status, 401);
    assert_eq!(b.raw_on(b.live.http, "GET", "/panel/", "", "").await.status, 303);
}

#[tokio::test]
async fn key_disclosure_is_explicit_admin_only_audited_and_absent_from_account_projections() {
    let mut b = Browser::new("keys").await;
    b.sign_in("Pegasus").await;
    let request = json!({ "kind": "create", "names": ["KeyTester"] });
    let first = b.call("POST", "/panel/api/v1/accounts/keys", request.clone()).await;
    assert_eq!(first.status, 200, "{}", first.body);
    let key: yo3_server::api::keys::KeyAccountResult = serde_json::from_value(first.body.clone()).unwrap();
    assert_eq!(key.key.expose().len(), yo3_server::accounts::KEY_LEN);
    assert!(first.headers.to_ascii_lowercase().contains("cache-control: no-store"));
    assert_eq!(b.call("POST", "/panel/api/v1/accounts/keys", request).await.body, first.body);
    for path in ["/panel/api/v1/accounts".into(), format!("/panel/api/v1/accounts/{}", key.user_id)] {
        let response = b.call("GET", &path, json!(null)).await;
        assert_eq!(response.status, 200); assert!(!response.body.to_string().contains(key.key.expose()));
    }
    let reveal = json!({ "kind": "reveal", "user_id": key.user_id });
    let shown = b.raw("POST", "/panel/api/v1/accounts/keys", &reveal.to_string(), "Idempotency-Key: separate-key-reveal-001\r\n").await;
    assert_eq!(shown.body["key"], first.body["key"]);
    let get = b.call("GET", "/panel/api/v1/accounts/keys", json!(null)).await;
    assert_ne!(get.status, 200);
    b.sign_in("Croquet").await;
    let denied = b.call("POST", "/panel/api/v1/accounts/keys", reveal).await;
    assert_eq!((denied.status, denied.body["code"].as_str()), (403, Some("forbidden")));
}

#[tokio::test]
async fn gifts_fail_with_typed_errors_without_metadata_and_never_accept_host_or_moderator_authority() {
    let mut b = Browser::new("gifts").await;
    let token = format!("Authorization: Bearer {}\r\n", b.admin.token);
    assert_eq!(b.raw("GET", "/panel/api/v1/gifts/choices", "", &token).await.status,401);
    b.sign_in("Pegasus").await;
    let error = b.call("GET", "/panel/api/v1/gifts/choices", json!(null)).await;
    assert_eq!((error.status,error.body["code"].as_str()),(503,Some("persistence_failed")));
    let bad = b.call("GET", &format!("/panel/api/v1/gifts/choices?q={}", "x".repeat(81)), json!(null)).await;
    assert_eq!((bad.status,bad.body["code"].as_str()),(400,Some("invalid_request")));
    let admin = b.admin.clone();
    tokio::task::spawn_blocking(move || api::call(&admin, &Op::AddPlayer { who: "Yugi".into(), name: "GiftTarget".into() })).await.unwrap().unwrap();
    let accounts = b.call("GET", "/panel/api/v1/accounts", json!(null)).await;
    let player = accounts.body["accounts"].as_array().unwrap().iter().find(|a| a["login"] == "Yugi").unwrap()["players"][0]["player_id"].clone();
    let request = json!({"kind":"cards","player_id":player,"row":816,"count":1});
    let error = b.call("POST", "/panel/api/v1/gifts", request.clone()).await;
    assert_eq!(error.status,503);
    b.sign_in("Croquet").await;
    assert_eq!(b.call("GET", "/panel/api/v1/gifts/choices", json!(null)).await.status,403);
    assert_eq!(b.call("POST", "/panel/api/v1/gifts", request).await.status,403);
}

#[tokio::test]
async fn settings_routes_are_staff_readable_admin_writable_and_require_stable_request_ids() {
    let mut b = Browser::new("settings-api").await;
    let path = "/panel/api/v1/settings";
    assert_eq!(b.call("GET",path,json!(null)).await.status,401);
    b.sign_in("Croquet").await;
    let list = b.call("GET",path,json!(null)).await;
    assert_eq!(list.status,200);
    assert!(list.body["checked_at"].as_str().is_some_and(|t| chrono::DateTime::parse_from_rfc3339(t).is_ok()));
    let settings = list.body["settings"].as_array().unwrap();
    assert_eq!(settings.len(),yo3_server::settings::REGISTRY.len());
    assert!(settings.iter().any(|s| s["name"] == "start-lp" && s["input"]["max"] == 99999));
    let request = json!({ "kind":"set","name":"headline","value":"Welcome from the API" });
    assert_eq!(b.call("POST",path,request.clone()).await.status,403);
    b.sign_in("Pegasus").await;
    assert_eq!(b.raw("POST",path,&request.to_string(),"").await.status,400);
    let result = b.call("POST",path,request.clone()).await;
    assert_eq!(result.status,200,"{}",result.body);
    assert_eq!(b.call("POST",path,request).await.body,result.body);
    assert_eq!(b.call("POST",path,json!({ "kind":"reset","name":"headline" })).await.status,409);
    let list = b.call("GET",path,json!(null)).await;
    assert!(list.body["settings"].as_array().unwrap().iter().any(|s|s["name"]=="headline" && s["overridden"]==true && s["value"]=="Welcome from the API"));
    let token = format!("Authorization: Bearer {}\r\n",b.admin.token);
    assert_eq!(b.raw_on(b.live.http,"GET",path,"",&token).await.status,404);
    let anonymous = Browser { cookie:String::new(),csrf:String::new(),..b };
    assert_eq!(anonymous.raw("POST",path,"{}",&token).await.status,401,"host token cannot authorize dashboard mutations");
}

#[tokio::test]
async fn settings_accept_all_ten_valid_unicode_announcements_with_a_bounded_payload() {
    let mut b=Browser::new("news-payload").await; b.sign_in("Pegasus").await;
    let entries: Vec<_>=(0..10).map(|_|json!({"title":"界".repeat(128),"body":"界".repeat(2048)})).collect();
    let body=json!({"kind":"set","name":"news","value":serde_json::to_string(&entries).unwrap()});
    let result=b.call("POST","/panel/api/v1/settings",body).await;
    assert_eq!(result.status,200,"{}",result.body);
    let huge=json!({"kind":"set","name":"news","value":"x".repeat(257*1024)});
    assert_eq!(b.raw("POST","/panel/api/v1/settings",&huge.to_string(),"Idempotency-Key: oversized-news-request\r\n").await.status,413);
}

#[tokio::test]
async fn moderation_uses_stable_ids_role_rules_and_observable_disconnect_outcomes() {
    let mut b=Browser::new("moderation-api").await;b.sign_in("Croquet").await;
    let accounts=b.call("GET","/panel/api/v1/accounts",json!(null)).await.body;
    let id=|name:&str|accounts["accounts"].as_array().unwrap().iter().find(|a|a["login"]==name).unwrap()["user_id"].as_u64().unwrap();
    let yugi=id("Yugi");let pegasus=id("Pegasus");let path="/panel/api/v1/moderation";
    assert_eq!(b.call("POST",path,json!({"kind":"kick","user_id":pegasus})).await.status,403);
    let request=json!({"kind":"ban","user_id":yugi,"reason":"Spam","days":1});
    let first=b.call("POST",path,request.clone()).await;
    assert_eq!(first.status,200,"{}",first.body);assert_eq!(first.body["disconnect"],"completed");
    assert!(chrono::DateTime::parse_from_rfc3339(first.body["result"]["until"].as_str().unwrap()).is_ok());
    assert_eq!(b.call("POST",path,request).await.body,first.body);
    assert_eq!(b.call("POST",path,json!({"kind":"kick","user_id":yugi})).await.status,409);
    let lifted=b.raw("POST",path,&json!({"kind":"lift","user_id":yugi}).to_string(),"Idempotency-Key: dashboard-lift-00001\r\n").await;
    assert_eq!(lifted.status,200);assert_eq!(lifted.body["disconnect"],Value::Null);
    assert_eq!(b.call("GET",&format!("/panel/api/v1/accounts/{yugi}"),json!(null)).await.body["standing"]["kind"],"active");
    let old=b.call("POST",path,json!({"kind":"ban","user_id":yugi,"reason":"Spam","days":1})).await;
    assert_eq!(old.body,first.body,"replay confirms the old operation; it cannot ban Yugi again");
    assert_eq!(b.call("GET",&format!("/panel/api/v1/accounts/{yugi}"),json!(null)).await.body["standing"]["kind"],"active");
}

#[tokio::test]
async fn restricting_then_lifting_staff_without_a_request_in_between_cannot_restore_old_cookies() {
    let mut b=Browser::new("restriction-epoch").await;b.sign_in("Pegasus").await;
    let live=b.admin.clone();tokio::task::spawn_blocking(move|| {
        api::call(&live,&Op::Restrict{who:"Pegasus".into(),reason:"Review".into(),days:None}).unwrap();
        api::call(&live,&Op::Lift{who:"Pegasus".into()}).unwrap();
    }).await.unwrap();
    assert_eq!(b.call("GET","/panel/api/v1/session",json!(null)).await.status,401);
}
