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
    ];
    assert_eq!(paths.len(), expected.len());
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
        logins::use_fast_hashing();
        let dir = std::env::temp_dir().join(format!("yo3-dashboard-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = Config::ephemeral(dir);
        let data = cfg.data_dir.clone().unwrap();
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
        let addr = self.live.http;
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
    let old_panel = b.call("GET", "/panel/", json!(null)).await;
    assert_eq!(old_panel.status, 303);
    b.sign_in("Croquet").await;
    let admin = b.admin.clone();
    let reset = tokio::task::spawn_blocking(move || api::call(&admin, &Op::ResetCode { user: "Croquet".into() })).await.unwrap().unwrap();
    let recovered = b.endpoint.account(&json!({ "op": "recover", "user": "Croquet", "code": reset["code"],
        "new_password": "RecoveryPass123" })).await.unwrap();
    assert_eq!(recovered["ok"], true, "{recovered}");
    assert_eq!(b.call("GET", "/panel/api/v1/session", json!(null)).await.status, 401);
    assert_eq!(b.call("GET", "/panel/", json!(null)).await.status, 303);
}
