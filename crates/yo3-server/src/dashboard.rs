//! The staff-only JSON API. Routes and OpenAPI come from the same registration, without constructing server state.
//! Mounted only on a dedicated, opt-in loopback listener. No host token is an authority here.
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Json, Router, body::Body, extract::{ConnectInfo, DefaultBodyLimit, State, Path, rejection::{JsonRejection, PathRejection}},
    http::{HeaderMap, HeaderValue, StatusCode, header}, middleware::{self, Next}, response::{IntoResponse, Response},
    routing::get, Extension};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, openapi::{security::{ApiKey, ApiKeyValue, SecurityScheme}, OpenApi}};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{api::{give_bp::GiveBpByIdRequest, Api, GiveBpResult, Actor, MutationError, MutationErrorCode}, lock::Locked,
    logins::Role, panel::{self, Panel, Viewer}};
use crate::api::profiles::{AccountDetail, AccountEditRequest, AccountEditResult, CountryList};
use crate::api::invites::{CreateInviteRequest, CreateInviteResult, InviteList};
use crate::api::codes::{BpCodeList, CreateBpCodeRequest, CreateBpCodeResult, RemoveBpCodeRequest, RemoveBpCodeResult};

#[derive(Clone)]
pub struct Dashboard { pub api: Arc<Api>, pub panel: Arc<Panel> }

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InvalidRequest, Unauthenticated, Forbidden, InvalidOrigin, CsrfInvalid, RateLimited, NotFound,
    AlreadyExists, RequestConflict, PersistenceFailed, MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType, InternalError,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError { pub code: ApiErrorCode, pub message: String }
impl ApiError {
    fn new(code: ApiErrorCode, message: impl Into<String>) -> Self { Self { code, message: message.into() } }
    fn status(&self) -> StatusCode {
        match self.code {
            ApiErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ApiErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
            ApiErrorCode::Forbidden | ApiErrorCode::InvalidOrigin | ApiErrorCode::CsrfInvalid => StatusCode::FORBIDDEN,
            ApiErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiErrorCode::NotFound => StatusCode::NOT_FOUND,
            ApiErrorCode::RequestConflict | ApiErrorCode::AlreadyExists => StatusCode::CONFLICT,
            ApiErrorCode::PersistenceFailed => StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            ApiErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ApiErrorCode::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
impl IntoResponse for ApiError { fn into_response(self) -> Response { (self.status(), Json(self)).into_response() } }
impl From<MutationError> for ApiError {
    fn from(error: MutationError) -> Self {
        Self::new(match error.code {
            MutationErrorCode::InvalidRequest => ApiErrorCode::InvalidRequest, MutationErrorCode::Forbidden => ApiErrorCode::Forbidden,
            MutationErrorCode::NotFound => ApiErrorCode::NotFound, MutationErrorCode::RequestConflict => ApiErrorCode::RequestConflict,
            MutationErrorCode::AlreadyExists => ApiErrorCode::AlreadyExists,
            MutationErrorCode::PersistenceFailed => ApiErrorCode::PersistenceFailed,
        }, error.message)
    }
}
impl From<JsonRejection> for ApiError {
    fn from(error: JsonRejection) -> Self {
        // Do not echo the request body or an internal serde error (sign-in bodies contain passwords).
        match error.status() {
            StatusCode::PAYLOAD_TOO_LARGE => Self::new(ApiErrorCode::PayloadTooLarge, "The request is too large."),
            StatusCode::UNSUPPORTED_MEDIA_TYPE => Self::new(ApiErrorCode::UnsupportedMediaType, "Send application/json."),
            _ => Self::new(ApiErrorCode::InvalidRequest, "The request does not match the API contract."),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SignInRequest { #[schema(min_length = 1, max_length = 64)] pub user: String, #[schema(write_only)] pub password: String }

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StaffSession {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32, pub user: String, pub role: Role, pub csrf_token: String,
    /// Idle expiry, refreshed by an authenticated request; capped by the absolute session lifetime.
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
impl From<Viewer> for StaffSession {
    fn from(v: Viewer) -> Self {
        Self { user_id: v.user_id, user: v.user, role: v.role, csrf_token: v.csrf,
            expires_at: chrono::DateTime::from_timestamp(v.expires, 0).expect("valid session expiry") }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignedOut { pub signed_out: bool }

/// A projection of existing account state: no keys, password hashes, device tokens or invented dates/presence.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountSummary {
    #[schema(maximum = 4294967295_u32)] pub user_id: u32, pub login: Option<String>, pub role: Role, #[schema(maximum = 4294967295_u32)] pub bp: u32, pub players: Vec<PlayerSummary>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlayerSummary { #[schema(maximum = 4294967295_u32)] pub player_id: u32, pub name: String, #[schema(maximum = 4294967295_u32)] pub slot: u32 }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountList { pub accounts: Vec<AccountSummary> }

/// Collect paths and their schemas before attaching state. Both the export binary and the real router call this.
fn documented_routes() -> OpenApiRouter<Dashboard> {
    OpenApiRouter::new().routes(routes!(staff_session)).routes(routes!(sign_in)).routes(routes!(sign_out))
        .routes(routes!(accounts)).routes(routes!(give_bp)).routes(routes!(list_bp_codes))
        .routes(routes!(create_bp_code)).routes(routes!(remove_bp_code)).routes(routes!(list_invites)).routes(routes!(create_invite))
        .routes(routes!(get_account)).routes(routes!(edit_account)).routes(routes!(country_choices))
}

pub fn openapi() -> OpenApi {
    let (_, mut spec) = documented_routes().split_for_parts();
    spec.info.title = "YO3 Revival staff API".into();
    spec.info.description = Some("Private dashboard API. Host-token operations and offline restore are not routes on this router.".into());
    let components = spec.components.get_or_insert_default();
    components.add_security_scheme("staffSession", SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(panel::COOKIE))));
    components.add_security_scheme("csrfToken", SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-CSRF-Token"))));
    spec
}

pub fn openapi_json() -> String { format!("{}\n", serde_json::to_string_pretty(&openapi()).expect("serialize OpenAPI")) }

pub fn router(state: Dashboard) -> Router {
    let (router, _) = documented_routes().split_for_parts();
    router.route("/panel/openapi.json", get(|| async { Json(openapi()) }))
        .method_not_allowed_fallback(|| async { ApiError::new(ApiErrorCode::MethodNotAllowed, "This method is not allowed.") })
        .layer(DefaultBodyLimit::max(16 * 1024))
        .route_layer(middleware::from_fn_with_state(state.clone(), guard))
        .fallback(|| async { not_found() })
        .with_state(state)
}

fn one<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    values.next().is_none().then_some(value)
}
fn cookie(headers: &HeaderMap) -> &str {
    headers.get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';'))
        .find_map(|v| v.trim().strip_prefix("yo3_panel=")).unwrap_or_default()
}
fn local_host(headers: &HeaderMap) -> bool {
    one(headers, "host").and_then(|h| h.parse::<axum::http::uri::Authority>().ok())
        .is_some_and(|h| matches!(h.host().to_ascii_lowercase().as_str(), "localhost" | "127.0.0.1" | "[::1]"))
}
fn same_origin(headers: &HeaderMap) -> bool {
    let origin = one(headers, "origin");
    let site = one(headers, "sec-fetch-site");
    let host = one(headers, "host").unwrap_or_default();
    origin.is_none_or(|o| o == format!("http://{host}") || o == format!("https://{host}"))
        && site.is_none_or(|s| s == "same-origin" || s == "none") && (origin.is_some() || site.is_some())
        && headers.get_all("origin").iter().count() <= 1 && headers.get_all("sec-fetch-site").iter().count() <= 1
}
async fn guard(State(state): State<Dashboard>, ConnectInfo(peer): ConnectInfo<SocketAddr>, mut req: axum::http::Request<Body>, next: Next) -> Response {
    let refusal = (|| -> Result<(), ApiError> {
        if !peer.ip().is_loopback() || !local_host(req.headers()) {
            return Err(ApiError::new(ApiErrorCode::Forbidden, "The dashboard API answers localhost only."))
        }
        let path = req.uri().path();
        let signing_in = path == "/panel/api/v1/sign-in";
        let viewer = if signing_in || path == "/panel/openapi.json" { None } else {
            Some(state.panel.session(cookie(req.headers()), &state.api).ok_or_else(||
                ApiError::new(ApiErrorCode::Unauthenticated, "Your session has ended. Sign in again."))?)
        };
        if req.method() == axum::http::Method::POST {
            if !same_origin(req.headers()) { return Err(ApiError::new(ApiErrorCode::InvalidOrigin, "Use the dashboard's own page.")) }
            if !one(req.headers(), "idempotency-key").is_some_and(crate::api::mutation::valid_request_id) {
                return Err(ApiError::new(ApiErrorCode::InvalidRequest, "Supply one valid Idempotency-Key header."))
            }
            if let Some(v) = &viewer && !one(req.headers(), "x-csrf-token").is_some_and(|t| crate::admin::same(t, &v.csrf)) {
                return Err(ApiError::new(ApiErrorCode::CsrfInvalid, "Reload the dashboard before trying again."))
            }
        }
        if let Some(viewer) = viewer { req.extensions_mut().insert(viewer); }
        Ok(())
    })();
    let mut response = match refusal { Ok(()) => next.run(req).await, Err(e) => e.into_response() };
    secure(&mut response);
    response
}

fn secure(response: &mut Response) {
    for (name, value) in panel::security_headers() {
        response.headers_mut().insert(axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(), HeaderValue::from_str(&value).unwrap());
    }
}

#[utoipa::path(get, path = "/panel/api/v1/session", operation_id = "getStaffSession", tag = "Staff",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = StaffSession),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn staff_session(Extension(viewer): Extension<Viewer>) -> Json<StaffSession> { Json(viewer.into()) }

#[utoipa::path(post, path = "/panel/api/v1/sign-in", operation_id = "signInStaff", tag = "Staff", security(),
    request_body = SignInRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$",
        description = "One ID per sign-in attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "Success", body = StaffSession), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many sign-in attempts", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn sign_in(State(state): State<Dashboard>, ConnectInfo(peer): ConnectInfo<SocketAddr>, body: Result<Json<SignInRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let Json(body) = body?;
    if body.user.is_empty() || body.user.len() > 64 {
        return Err(ApiError::new(ApiErrorCode::InvalidRequest, "Enter a User ID of at most 64 characters."))
    }
    match panel::authenticate(&body.user, body.password, peer.ip(), &state.panel, &state.api).await {
        Ok((token, viewer)) => {
            let mut response = Json(StaffSession::from(viewer)).into_response();
            response.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&panel::session_cookie(&token, false)).unwrap());
            Ok(response)
        }
        Err(panel::SignInError::Wrong) => Err(ApiError::new(ApiErrorCode::Unauthenticated, "Wrong User ID or password, or this account can't use the admin panel.")),
        Err(panel::SignInError::Throttled(wait)) => {
            let mut response = ApiError::new(ApiErrorCode::RateLimited, format!("Too many tries. Wait {wait} seconds before trying again.")).into_response();
            response.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from_str(&wait.to_string()).unwrap());
            Ok(response)
        }
    }
}

#[utoipa::path(post, path = "/panel/api/v1/sign-out", operation_id = "signOutStaff", tag = "Staff",
    security(("staffSession" = [], "csrfToken" = [])),
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$")),
    responses((status = 200, description = "Success", body = SignedOut), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn sign_out(State(state): State<Dashboard>, headers: HeaderMap) -> Response {
    state.panel.close(cookie(&headers));
    let mut response = Json(SignedOut { signed_out: true }).into_response();
    response.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&panel::session_cookie("", true)).unwrap());
    response
}

#[utoipa::path(get, path = "/panel/api/v1/accounts", operation_id = "listAccounts", tag = "Accounts",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = AccountList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn accounts(State(state): State<Dashboard>) -> Json<AccountList> {
    let d = state.api.accounts.locked();
    Json(AccountList { accounts: d.entries().map(|(user, _, a)| AccountSummary {
        user_id: a.user_id, login: user.map(str::to_string), role: d.login_by_id(a.user_id).map_or(Role::Player, |l| l.role), bp: a.dpoint,
        players: a.players.iter().map(|p| PlayerSummary { player_id: p.player_id, name: p.name.clone(), slot: p.index }).collect(),
    }).collect() })
}

#[utoipa::path(post, path = "/panel/api/v1/give-bp", operation_id = "giveBp", tag = "Accounts",
    security(("staffSession" = [], "csrfToken" = [])), request_body = GiveBpByIdRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$",
        description = "Create once per intended grant; reuse on every retry, including after a lost response.")),
    responses((status = 200, description = "Success", body = GiveBpResult), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError), (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 409, description = "Request ID conflict", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError), (status = 503, description = "Persistence unavailable; retry the same request ID", body = ApiError)))]
async fn give_bp(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<GiveBpByIdRequest>, JsonRejection>) -> Result<Json<GiveBpResult>, ApiError> {
    let Json(request) = body?;
    let key = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.give_bp_to_account(&request, &key, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/bp-codes", operation_id = "listBpCodes", tag = "BOOSTERPASS",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = BpCodeList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn list_bp_codes(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<BpCodeList>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.list_bp_codes(Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The code list could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/bp-codes", operation_id = "createBpCode", tag = "BOOSTERPASS",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CreateBpCodeRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change. Retain it on an uncertain outcome; retries return the original code.")),
    responses((status = 200, description = "Committed code or original retry result", body = CreateBpCodeResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 409, description = "Request ID conflict or code already exists", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn create_bp_code(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<CreateBpCodeRequest>, JsonRejection>) -> Result<Json<CreateBpCodeResult>, ApiError> {
    let Json(request) = body?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.create_bp_code(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/bp-codes/remove", operation_id = "removeBpCode", tag = "BOOSTERPASS",
    security(("staffSession" = [], "csrfToken" = [])), request_body = RemoveBpCodeRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per removal. Retain it on an uncertain outcome; do not generate a new one on retry.")),
    responses((status = 200, description = "Committed removal or original retry result", body = RemoveBpCodeResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 409, description = "Request ID conflict", body = ApiError), (status = 404, description = "Code does not exist", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn remove_bp_code(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<RemoveBpCodeRequest>, JsonRejection>) -> Result<Json<RemoveBpCodeResult>, ApiError> {
    let Json(request) = body?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.remove_bp_code(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/invites", operation_id = "listInvites", tag = "Invites",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = InviteList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored invite could not be read", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn list_invites(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<InviteList>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.list_invites(Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The invite list could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/invites", operation_id = "createInvite", tag = "Invites",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CreateInviteRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change. Retain it on an uncertain outcome; retries return the original invite and expiry.")),
    responses((status = 200, description = "Committed invite or original retry result", body = CreateInviteResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 409, description = "Request ID conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn create_invite(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<CreateInviteRequest>, JsonRejection>) -> Result<Json<CreateInviteResult>, ApiError> {
    let Json(request) = body?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.create_invite(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/accounts/{user_id}", operation_id = "getAccount", tag = "Accounts",
    params(("user_id" = u32, Path, maximum = 4294967295_u32, description = "Stable account number")),
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = AccountDetail),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 404, description = "Account not found", body = ApiError), (status = 503, description = "Stored account could not be read", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn get_account(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, path: Result<Path<u32>, PathRejection>) -> Result<Json<AccountDetail>, ApiError> {
    let Path(user_id) = path.map_err(|_| ApiError::new(ApiErrorCode::InvalidRequest, "Choose a valid account number."))?;
    tokio::task::spawn_blocking(move || state.api.account_detail(user_id, Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "Account details could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/countries", operation_id = "listCountries", tag = "Accounts",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = CountryList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Country choices unavailable", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn country_choices(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<CountryList>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.account_countries(Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "Country choices could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/accounts/edit", operation_id = "editAccount", tag = "Accounts",
    security(("staffSession" = [], "csrfToken" = [])), request_body = AccountEditRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change. Retain it on an uncertain outcome; retries never reapply a committed edit.")),
    responses((status = 200, description = "Committed edit or original retry result", body = AccountEditResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account or player not found", body = ApiError), (status = 409, description = "Request ID conflict, duplicate name or full account", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn edit_account(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<AccountEditRequest>, JsonRejection>) -> Result<Json<AccountEditResult>, ApiError> {
    let Json(request) = body?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.edit_account(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

pub(crate) fn not_found() -> Response {
    let mut response = ApiError::new(ApiErrorCode::NotFound, "No such dashboard API route.").into_response();
    secure(&mut response);
    response
}
