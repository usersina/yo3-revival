//! The staff-only JSON API. Routes and OpenAPI come from the same registration, without constructing server state.
//! Mounted on opt-in loopback TCP or an origin-checked Unix proxy socket. No host token is an authority here.
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Json, Router, body::Body, extract::{ConnectInfo, DefaultBodyLimit, State, Path, Query, rejection::{JsonRejection, PathRejection, QueryRejection}},
    http::{HeaderMap, HeaderValue, StatusCode, header}, middleware::{self, Next}, response::{IntoResponse, Response},
    routing::get, Extension};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, openapi::{security::{ApiKey, ApiKeyValue, SecurityScheme}, OpenApi}};
use utoipa_axum::{router::{OpenApiRouter, UtoipaMethodRouterExt}, routes};

use crate::{api::{give_bp::GiveBpByIdRequest, Api, GiveBpResult, Actor, MutationError, MutationErrorCode}, lock::Locked,
    logins::Role, panel::{self, Panel, Viewer}};
use crate::api::moderation::{ModerationRequest,ModerationResponse};
use crate::api::deletion::{DeleteAccountRequest,DeleteAccountResponse};
use crate::api::server::{StopRequest,StopResponse,ServerOverview};
use crate::api::observability::{AuditQuery,AuditPage,LogsQuery,LogPage};
use crate::api::backups::{BackupList,CreateBackupRequest,CreateBackupResponse,DownloadBackupRequest};
use crate::api::settings::{SettingsRequest, SettingsResult, SettingsList};
use crate::api::card_images::CardImageSize;
use crate::api::gifts::{GiftRequest, GiftResult, GiftChoices};
use crate::api::keys::{KeyAccountRequest, KeyAccountResult};
use crate::api::credentials::{IssueAccountCodeRequest, AccountCodeResponse};
use crate::api::profiles::{AccountDetail, AccountEditRequest, AccountEditResult, CountryList};
use crate::api::invites::{CreateInviteRequest, CreateInviteResult, InviteList};
use crate::api::problem_reports::{ProblemReport, ProblemReportList, ProblemReportQuery};
use crate::api::reports::{HandleReportRequest, HandleReportResult, ReportList, ReportQuery};
use crate::api::banlist::{AccountBanlist, BanlistChoices, SetBanlistRequest};
use crate::api::championship::{CancelChampionshipRequest, ChampionshipList, ChampionshipResult, CreateChampionshipRequest, EditChampionshipRequest};
use crate::api::official::{CancelEventRequest, CreateEventRequest, EditEventRequest, EventList, EventResult};
use crate::api::codes::{BpCodeList, CreateBpCodeRequest, CreateBpCodeResult, RemoveBpCodeRequest, RemoveBpCodeResult};

#[derive(Clone)]
pub struct Dashboard { pub api: Arc<Api>, pub panel: Arc<Panel> }

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    InvalidRequest, Unauthenticated, Forbidden, InvalidOrigin, CsrfInvalid, RateLimited, NotFound,
    AlreadyExists, RequestConflict, PersistenceFailed, ReauthenticationRequired, InvalidCode, MfaEnrollmentRequired, MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType, InternalError,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError { pub code: ApiErrorCode, pub message: String }
impl ApiError {
    fn new(code: ApiErrorCode, message: impl Into<String>) -> Self { Self { code, message: message.into() } }
    fn status(&self) -> StatusCode {
        match self.code {
            ApiErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ApiErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
            ApiErrorCode::ReauthenticationRequired | ApiErrorCode::InvalidCode | ApiErrorCode::MfaEnrollmentRequired | ApiErrorCode::Forbidden | ApiErrorCode::InvalidOrigin | ApiErrorCode::CsrfInvalid => StatusCode::FORBIDDEN,
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
    /// The account has an active authenticator (TOTP).
    pub mfa_enabled: bool,
    /// The server requires an authenticator of every staff account (the default policy).
    pub mfa_required: bool,
    /// An enrollment-only session: every route but enrollment, sign-out and this session answers `mfa_enrollment_required`.
    pub limited: bool,
    /// When the authenticator's recent check ends; none: sensitive operations ask for a code first.
    pub mfa_recent_until: Option<chrono::DateTime<chrono::Utc>>,
    /// Unused recovery codes left.
    #[schema(maximum = 100)] pub recovery_codes_remaining: u32,
}
impl From<Viewer> for StaffSession {
    fn from(v: Viewer) -> Self {
        let recent = v.mfa_recent_until();
        Self { user_id: v.user_id, user: v.user, role: v.role, csrf_token: v.csrf,
            expires_at: chrono::DateTime::from_timestamp(v.expires, 0).expect("valid session expiry"),
            mfa_enabled: v.mfa_enabled, mfa_required: v.policy == crate::staff_mfa::StaffMfaPolicy::Required, limited: v.limited,
            mfa_recent_until: recent.and_then(|t| chrono::DateTime::from_timestamp(t, 0)), recovery_codes_remaining: v.recovery_left }
    }
}

/// The result of the password step: a code is still owed (`challenge`), or a session was opened (see its `limited`).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignInOutcome {
    pub code_required: bool,
    /// Present with `code_required`: send it back with the code to `/sign-in/code` within five minutes.
    pub challenge: Option<String>,
    pub session: Option<StaffSession>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SignInCodeRequest {
    #[schema(min_length = 64, max_length = 64)] pub challenge: String,
    /// An authenticator code (six digits) or an unused recovery code.
    #[schema(write_only, min_length = 6, max_length = 32)] pub code: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CodeRequest {
    /// An authenticator code (six digits) or an unused recovery code.
    #[schema(write_only, min_length = 6, max_length = 32)] pub code: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyRequest {}

/// The seed of a new authenticator, shown to its owner once. The QR code is the `otpauth_uri`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaEnrollmentStart { pub secret: String, pub otpauth_uri: String, pub account: String, pub issuer: String }

/// An authenticator was activated. The recovery codes are shown once and are kept only as digests.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MfaEnrolled { pub recovery_codes: Vec<String>, pub session: StaffSession }

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoveryCodes { pub recovery_codes: Vec<String> }

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignedOut { pub signed_out: bool }

/// A projection of existing account state: no keys, password hashes, device tokens or invented dates/presence.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountSummary {
    pub standing:Option<crate::api::profiles::AccountStanding>,pub muted:Option<crate::api::profiles::AccountMute>,
    #[schema(maximum = 4294967295_u32)] pub user_id: u32, pub login: Option<String>, pub role: Role, #[schema(maximum = 4294967295_u32)] pub bp: u32, pub players: Vec<PlayerSummary>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlayerSummary { #[schema(maximum = 4294967295_u32)] pub player_id: u32, pub name: String, #[schema(maximum = 4294967295_u32)] pub slot: u32 }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountList { pub accounts: Vec<AccountSummary> }

/// Collect paths and their schemas before attaching state. Both the export binary and the real router call this.
fn documented_routes() -> OpenApiRouter<Dashboard> {
    OpenApiRouter::new().routes(routes!(staff_session)).routes(routes!(sign_in)).routes(routes!(sign_in_code)).routes(routes!(sign_out)).routes(routes!(sign_out_everywhere))
        .routes(routes!(start_enrollment)).routes(routes!(confirm_enrollment)).routes(routes!(step_up)).routes(routes!(regenerate_recovery_codes))
        .routes(routes!(accounts)).routes(routes!(give_bp)).routes(routes!(list_bp_codes))
        .routes(routes!(create_bp_code)).routes(routes!(remove_bp_code)).routes(routes!(list_invites)).routes(routes!(create_invite))
        .routes(routes!(get_account)).routes(routes!(edit_account)).routes(routes!(country_choices)).routes(routes!(issue_account_code)).routes(routes!(key_account)).routes(routes!(give_gift)).routes(routes!(gift_choices)).routes(routes!(card_image)).routes(routes!(moderate)).routes(routes!(delete_account)).routes(routes!(server_status)).routes(routes!(audit_history)).routes(routes!(server_logs)).routes(routes!(backups)).routes(routes!(create_backup)).routes(routes!(download_backup)).routes(routes!(stop_server)).routes(routes!(live_settings)).routes(routes!(change_settings).layer(DefaultBodyLimit::max(256 * 1024))).routes(routes!(list_reports)).routes(routes!(handle_report)).routes(routes!(list_problem_reports)).routes(routes!(get_problem_report)).routes(routes!(banlist_choices)).routes(routes!(set_banlist)).routes(routes!(list_events)).routes(routes!(create_event)).routes(routes!(edit_event)).routes(routes!(cancel_event)).routes(routes!(list_championships)).routes(routes!(create_championship)).routes(routes!(edit_championship)).routes(routes!(cancel_championship))
}

pub fn openapi() -> OpenApi {
    let (_, mut spec) = documented_routes().split_for_parts();
    spec.info.title = "YO3 Revival staff API".into();
    spec.info.description = Some("Staff dashboard API. Host-token operations and offline restore are not routes on this router.".into());
    let components = spec.components.get_or_insert_default();
    components.schemas.insert("CardImageSize".into(), <CardImageSize as utoipa::PartialSchema>::schema());
    components.add_security_scheme("staffSession", SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(panel::COOKIE))));
    components.add_security_scheme("csrfToken", SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-CSRF-Token"))));
    spec
}

pub fn openapi_json() -> String { format!("{}\n", serde_json::to_string_pretty(&openapi()).expect("serialize OpenAPI")) }

/// The socket is a filesystem capability held only by nginx. Forwarded identity is accepted on that transport
/// alone; the TCP router never trusts it. No proxy request gains local management authority.
#[derive(Clone)]
pub struct ProxyOrigin { origin: String, authority: String, scheme: String, backups: bool }
impl ProxyOrigin {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        let uri: axum::http::Uri = value.parse()?;
        let authority = uri.authority().ok_or_else(|| anyhow::anyhow!("dashboard-origin needs a host"))?;
        let scheme = uri.scheme_str().unwrap_or_default();
        anyhow::ensure!(!authority.as_str().contains('@') && uri.path_and_query().is_none_or(|p|p.as_str()=="/")
            && !value.ends_with('/') && (scheme=="https" || scheme=="http" && matches!(authority.host(),"localhost"|"127.0.0.1"|"[::1]")),
            "dashboard-origin must be an exact HTTPS origin without a path (HTTP localhost is allowed for local checks)");
        Ok(Self { origin:value.into(), authority:authority.as_str().into(), scheme:scheme.into(), backups:false })
    }
    /// Lets backups be downloaded through this transport (the host's `--dashboard-public-backups`; off by default).
    pub fn public_backups(mut self, allowed: bool) -> Self { self.backups = allowed; self }
    fn peer(&self, headers:&HeaderMap) -> Result<std::net::IpAddr,ApiError> {
        if one(headers,"host")!=Some(self.authority.as_str()) || one(headers,"x-forwarded-proto")!=Some(self.scheme.as_str())
            || headers.contains_key("forwarded") {
            return Err(ApiError::new(ApiErrorCode::Forbidden,"Use the configured dashboard address through nginx."))
        }
        one(headers,"x-forwarded-for").and_then(|p|p.parse().ok()).ok_or_else(||ApiError::new(ApiErrorCode::Forbidden,"One proxy client address is required."))
    }
}
pub fn proxy_router(state:Dashboard, origin:ProxyOrigin)->Router { build_router(state).layer(Extension(origin)) }
pub fn router(state: Dashboard) -> Router { build_router(state) }
fn build_router(state: Dashboard) -> Router {
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
/// What an enrollment-only session may reach.
const ENROLLMENT_ONLY: [&str; 5] = ["/panel/api/v1/session", "/panel/api/v1/sign-out", "/panel/api/v1/mfa/enrollment", "/panel/api/v1/mfa/enrollment/confirm", "/panel/api/v1/sign-out-everywhere"];

async fn guard(State(state): State<Dashboard>, ConnectInfo(peer): ConnectInfo<SocketAddr>, mut req: axum::http::Request<Body>, next: Next) -> Response {
    let refusal = (|| -> Result<(), ApiError> {
        let proxy = req.extensions().get::<ProxyOrigin>().cloned();
        if let Some(origin) = &proxy {
            let client=origin.peer(req.headers())?;
            req.extensions_mut().insert(ConnectInfo(SocketAddr::new(client,0)));
        } else if !peer.ip().is_loopback() || !local_host(req.headers()) {
            return Err(ApiError::new(ApiErrorCode::Forbidden, "The dashboard API answers localhost only."))
        }
        let path = req.uri().path();
        if proxy.as_ref().is_some_and(|p| !p.backups) && path=="/panel/api/v1/backups/download" {
            return Err(ApiError::new(ApiErrorCode::Forbidden,"Backup downloads are disabled on the proxy. Copy a backup through SSH, or use the private dashboard port."))
        }
        let signing_in = path == "/panel/api/v1/sign-in" || path == "/panel/api/v1/sign-in/code";
        let viewer = if signing_in || path == "/panel/openapi.json" { None } else {
            Some(state.panel.session(cookie(req.headers()), &state.api).ok_or_else(||
                ApiError::new(ApiErrorCode::Unauthenticated, "Your session has ended. Sign in again."))?)
        };
        if let Some(v) = &viewer && v.limited && !ENROLLMENT_ONLY.contains(&path) {
            return Err(ApiError::new(ApiErrorCode::MfaEnrollmentRequired, "Set up your authenticator app before using the dashboard."))
        }
        if req.method() == axum::http::Method::POST {
            let origin_ok = if let Some(origin)=&proxy {
                one(req.headers(),"origin")==Some(origin.origin.as_str())
                    && one(req.headers(),"sec-fetch-site").is_none_or(|s|s=="same-origin"||s=="none")
                    && req.headers().get_all("sec-fetch-site").iter().count()<=1
            } else { same_origin(req.headers()) };
            if !origin_ok { return Err(ApiError::new(ApiErrorCode::InvalidOrigin, "Use the dashboard's own page.")) }
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

#[derive(Clone)]
struct PrivateCardImage;

fn secure(response: &mut Response) {
    let image = response.extensions().get::<PrivateCardImage>().is_some();
    for (name, value) in panel::security_headers() {
        response.headers_mut().insert(axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(), HeaderValue::from_str(&value).unwrap());
    }
    if image { response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=86400")); }
}

#[utoipa::path(get, path = "/panel/api/v1/session", operation_id = "getStaffSession", tag = "Staff",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = StaffSession),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn staff_session(Extension(viewer): Extension<Viewer>) -> Json<StaffSession> { Json(viewer.into()) }

#[utoipa::path(post, path = "/panel/api/v1/sign-in", operation_id = "signInStaff", tag = "Staff", security(()),
    request_body = SignInRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$",
        description = "One ID per sign-in attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "A code is owed (code_required and a challenge), or a session was opened: an enrollment-only one (limited) for an account with no authenticator", body = SignInOutcome), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Wrong User ID or password", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many sign-in attempts", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn sign_in(State(state): State<Dashboard>, ConnectInfo(peer): ConnectInfo<SocketAddr>, body: Result<Json<SignInRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let Json(body) = body?;
    if body.user.is_empty() || body.user.len() > 64 {
        return Err(ApiError::new(ApiErrorCode::InvalidRequest, "Enter a User ID of at most 64 characters."))
    }
    match panel::begin_sign_in(&body.user, body.password, peer.ip(), &state.panel, &state.api).await {
        Ok(panel::Next::Code(challenge)) => Ok(Json(SignInOutcome { code_required: true, challenge: Some(challenge), session: None }).into_response()),
        Ok(panel::Next::Session(token, viewer)) => {
            let mut response = Json(SignInOutcome { code_required: false, challenge: None, session: Some(StaffSession::from(viewer)) }).into_response();
            response.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&panel::session_cookie(&token, false)).unwrap());
            Ok(response)
        }
        Err(panel::SignInError::Wrong | panel::SignInError::CodeNeeded) => Err(ApiError::new(ApiErrorCode::Unauthenticated, "Wrong User ID or password, or this account can't use the admin panel.")),
        Err(panel::SignInError::Throttled(wait)) => Ok(throttled(wait)),
    }
}

fn throttled(wait: u64) -> Response {
    let mut response = ApiError::new(ApiErrorCode::RateLimited, format!("Too many tries. Wait {wait} seconds before trying again.")).into_response();
    response.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from_str(&wait.to_string()).unwrap());
    response
}

/// A refused code, as the API says it.
fn code_error(error: crate::staff_mfa::CodeError) -> Response {
    use crate::staff_mfa::CodeError;
    match error {
        CodeError::Invalid => ApiError::new(ApiErrorCode::InvalidCode, "That code is not right, or was already used. Check your authenticator's clock, or use a recovery code.").into_response(),
        CodeError::Throttled(wait) => throttled(wait),
        CodeError::Unavailable => ApiError::new(ApiErrorCode::Forbidden, "This account has no active authenticator.").into_response(),
        CodeError::Failed(why) => { tracing::error!("[staff-mfa] {why}"); ApiError::new(ApiErrorCode::InternalError, "The authenticator could not be checked. Ask the host to check the server log.").into_response() }
    }
}

#[utoipa::path(post, path = "/panel/api/v1/sign-in/code", operation_id = "verifyStaffCode", tag = "Staff", security(()),
    request_body = SignInCodeRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$",
        description = "One ID per attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "The password and the code were right: a session was opened", body = StaffSession), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "The challenge expired or was used up: sign in again", body = ApiError),
        (status = 403, description = "The code is not right, or was already used (invalid_code)", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many attempts", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn sign_in_code(State(state): State<Dashboard>, ConnectInfo(peer): ConnectInfo<SocketAddr>, body: Result<Json<SignInCodeRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let Json(body) = body?;
    if body.challenge.len() != 64 || body.code.is_empty() || body.code.len() > 32 {
        return Err(ApiError::new(ApiErrorCode::InvalidRequest, "Send the sign-in challenge and one code."))
    }
    let expired = || ApiError::new(ApiErrorCode::Unauthenticated, "Your sign-in expired. Enter your password again.");
    let (user_id, bound) = state.panel.challenge_of(&body.challenge).ok_or_else(expired)?;
    let outcome = {
        let (panel, api) = (state.panel.clone(), state.api.clone());
        let code = body.code.clone();
        tokio::task::spawn_blocking(move || panel.use_code(&api, user_id, peer.ip(), &code)).await
            .map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The code could not be checked."))?
    };
    let used = match outcome {
        Ok(used) => used,
        Err(crate::staff_mfa::CodeError::Invalid) => { state.panel.challenge_missed(&body.challenge); return Ok(code_error(crate::staff_mfa::CodeError::Invalid)) }
        Err(e) => return Ok(code_error(e)),
    };
    state.panel.challenge_done(&body.challenge);
    // The password was right a moment ago; the account must still be exactly as it was (no change of password, role or standing).
    let login = state.api.accounts.locked().login_by_id(user_id).cloned().filter(|l| panel::credential_of(l) == bound).ok_or_else(expired)?;
    if let crate::staff_mfa::Used::Recovery { left } = used {
        state.api.note(&login.user, "dashboard", "staff-recovery-code-used", &format!("{left} left"));
    }
    let opening = panel::Opening { mfa_at: Some(crate::logins::now()), ..Default::default() };
    let (token, viewer) = state.panel.open_staff(user_id, &login.user, bound, opening, &state.api).ok_or_else(expired)?;
    state.api.note(&login.user, "dashboard", "staff-sign-in", &login.user);
    let mut response = Json(StaffSession::from(viewer)).into_response();
    response.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&panel::session_cookie(&token, false)).unwrap());
    Ok(response)
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

#[utoipa::path(post, path = "/panel/api/v1/mfa/enrollment", operation_id = "startMfaEnrollment", tag = "Staff",
    security(("staffSession" = [], "csrfToken" = [])),
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "A fresh seed, pending until a valid code confirms it. Shown to its owner once.", body = MfaEnrollmentStart), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden, or the code is not right (invalid_code), or a recent authenticator code is required (reauthentication_required)", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many attempts", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn start_enrollment(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<MfaEnrollmentStart>, ApiError> {
    // Replacing an authenticator that is active needs the old one's recent code.
    if viewer.mfa_enabled && !viewer.sensitive_ok() { return Err(needs_code()) }
    let (secret, otpauth_uri, account) = tokio::task::spawn_blocking(move || {
        let started = state.panel.begin_enrollment(&state.api, viewer.user_id)?;
        state.api.note(&viewer.user, "dashboard", "staff-mfa-enrollment-started", &viewer.user);
        Ok::<_, String>(started)
    }).await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "Enrollment could not start."))?
        .map_err(|e| { tracing::error!("[staff-mfa] {e}"); ApiError::new(ApiErrorCode::PersistenceFailed, "Enrollment could not be saved. Ask the host to check the server log.") })?;
    Ok(Json(MfaEnrollmentStart { secret, otpauth_uri, account, issuer: "YO3 Revival".into() }))
}

#[utoipa::path(post, path = "/panel/api/v1/mfa/enrollment/confirm", operation_id = "confirmMfaEnrollment", tag = "Staff",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CodeRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "Activated. The recovery codes are shown once; the session carries on and every other session of the account ended.", body = MfaEnrolled), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden, or the code is not right (invalid_code), or a recent authenticator code is required (reauthentication_required)", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many attempts", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn confirm_enrollment(State(state): State<Dashboard>, ConnectInfo(peer): ConnectInfo<SocketAddr>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<CodeRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let Json(body) = body?;
    if body.code.len() > 32 { return Err(ApiError::new(ApiErrorCode::InvalidRequest, "Send the six digits your authenticator shows.")) }
    let state2 = state.clone();
    let codes = tokio::task::spawn_blocking(move || state2.panel.confirm_enrollment(&state2.api, viewer.user_id, peer.ip(), &body.code)).await
        .map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The code could not be checked."))?;
    let codes = match codes { Ok(codes) => codes, Err(e) => return Ok(code_error(e)) };
    state.api.note(&viewer.user, "dashboard", "staff-mfa-enrolled", &viewer.user);
    let session = state.panel.rebind(cookie(&headers), &state.api, Some(crate::logins::now()))
        .ok_or_else(|| ApiError::new(ApiErrorCode::Unauthenticated, "Your session has ended. Sign in again."))?;
    Ok(Json(MfaEnrolled { recovery_codes: codes, session: session.into() }).into_response())
}

#[utoipa::path(post, path = "/panel/api/v1/mfa/step-up", operation_id = "confirmMfaStepUp", tag = "Staff",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CodeRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "The authenticator was checked: sensitive operations are allowed for five minutes.", body = StaffSession), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden, or the code is not right (invalid_code), or a recent authenticator code is required (reauthentication_required)", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many attempts", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn step_up(State(state): State<Dashboard>, ConnectInfo(peer): ConnectInfo<SocketAddr>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<CodeRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let Json(body) = body?;
    if body.code.is_empty() || body.code.len() > 32 { return Err(ApiError::new(ApiErrorCode::InvalidRequest, "Send an authenticator code or a recovery code.")) }
    let state2 = state.clone();
    let used = tokio::task::spawn_blocking(move || state2.panel.use_code(&state2.api, viewer.user_id, peer.ip(), &body.code)).await
        .map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The code could not be checked."))?;
    match used { Ok(crate::staff_mfa::Used::Recovery { left }) => state.api.note(&viewer.user, "dashboard", "staff-recovery-code-used", &format!("{left} left")), Ok(_) => {}, Err(e) => return Ok(code_error(e)) }
    state.panel.stamp_mfa(cookie(&headers));
    let session = state.panel.session(cookie(&headers), &state.api).ok_or_else(|| ApiError::new(ApiErrorCode::Unauthenticated, "Your session has ended. Sign in again."))?;
    Ok(Json(StaffSession::from(session)).into_response())
}

#[utoipa::path(post, path = "/panel/api/v1/mfa/recovery-codes", operation_id = "regenerateRecoveryCodes", tag = "Staff",
    security(("staffSession" = [], "csrfToken" = [])), request_body = EmptyRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "New recovery codes, shown once; the old ones no longer work.", body = RecoveryCodes), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden, or the code is not right (invalid_code), or a recent authenticator code is required (reauthentication_required)", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many attempts", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn regenerate_recovery_codes(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<EmptyRequest>, JsonRejection>) -> Result<Json<RecoveryCodes>, ApiError> {
    let _ = body?;
    if !viewer.mfa_enabled { return Err(ApiError::new(ApiErrorCode::Forbidden, "Set up an authenticator first.")) }
    sensitive(&viewer)?;
    tokio::task::spawn_blocking(move || {
        let codes = state.panel.regenerate_recovery(&state.api, viewer.user_id)?;
        state.api.note(&viewer.user, "dashboard", "staff-recovery-codes-regenerated", &viewer.user);
        Ok::<_, String>(codes)
    }).await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The codes could not be made."))?
        .map(|recovery_codes| Json(RecoveryCodes { recovery_codes }))
        .map_err(|e| { tracing::error!("[staff-mfa] {e}"); ApiError::new(ApiErrorCode::PersistenceFailed, "The codes could not be saved. Try again.") })
}

#[utoipa::path(post, path = "/panel/api/v1/sign-out-everywhere", operation_id = "signOutEverywhere", tag = "Staff",
    security(("staffSession" = [], "csrfToken" = [])), request_body = EmptyRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per attempt. Authentication does not create a durable operation receipt.")),
    responses((status = 200, description = "Every staff session of this account ended, this one too.", body = SignedOut), (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError),
        (status = 403, description = "Forbidden, or the code is not right (invalid_code), or a recent authenticator code is required (reauthentication_required)", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError),
        (status = 415, description = "JSON required", body = ApiError), (status = 429, description = "Too many attempts", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn sign_out_everywhere(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<EmptyRequest>, JsonRejection>) -> Result<Response, ApiError> {
    let _ = body?;
    let state2 = state.clone();
    tokio::task::spawn_blocking(move || {
        state2.api.accounts.locked().update_login(viewer.user_id, |login| {
            login.staff_epoch = login.staff_epoch.checked_add(1).ok_or("session revision exhausted")?;
            Ok(())
        })
    }).await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The sessions could not be ended."))?
        .map_err(|_| ApiError::new(ApiErrorCode::PersistenceFailed, "The sessions could not be ended. Try again."))?;
    state.api.note(&viewer.user, "dashboard", "staff-sign-out-everywhere", &viewer.user);
    let mut response = Json(SignedOut { signed_out: true }).into_response();
    response.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&panel::session_cookie("", true)).unwrap());
    Ok(response)
}

/// A sensitive operation asks for an authenticator check in the last five minutes (see docs/dashboard.md).
fn needs_code() -> ApiError { ApiError::new(ApiErrorCode::ReauthenticationRequired, "Confirm with your authenticator code to do this.") }
fn sensitive(viewer: &Viewer) -> Result<(), ApiError> { if viewer.sensitive_ok() { Ok(()) } else { Err(needs_code()) } }

#[utoipa::path(get, path = "/panel/api/v1/accounts", operation_id = "listAccounts", tag = "Accounts",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = AccountList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Account metadata unavailable", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn accounts(State(state): State<Dashboard>) -> Result<Json<AccountList>,ApiError> {
    let d=state.api.accounts.locked();let now=crate::logins::now();
    let accounts=d.entries().map(|(user,_,a)|{
        let login=d.login_by_id(a.user_id);let (standing,muted)=crate::api::profiles::badges(login,now)?;
        Ok(AccountSummary{standing,muted,user_id:a.user_id,login:user.map(str::to_string),role:login.map_or(Role::Player,|l|l.role),bp:a.dpoint,
            players:a.players.iter().map(|p|PlayerSummary{player_id:p.player_id,name:p.name.clone(),slot:p.index}).collect()})
    }).collect::<Result<Vec<_>,MutationError>>()?;
    Ok(Json(AccountList{accounts}))
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
    if matches!(request, crate::api::profiles::AccountEditRequest::SetRole { .. }) { sensitive(&viewer)? }
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.edit_account(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/accounts/credentials", operation_id = "issueAccountCode", tag = "Accounts",
    security(("staffSession" = [], "csrfToken" = [])), request_body = IssueAccountCodeRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and the same generated code on retry. Codes never appear in durable receipts.")),
    responses((status = 200, description = "Immutable issuance confirmation and current code availability; never the code", body = AccountCodeResponse),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account not found", body = ApiError), (status = 409, description = "User ID taken or retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn issue_account_code(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<IssueAccountCodeRequest>, JsonRejection>) -> Result<Json<AccountCodeResponse>, ApiError> {
    let Json(request) = body?;
    sensitive(&viewer)?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.issue_account_code(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID and code."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/accounts/keys", operation_id = "manageAccountKey", tag = "Accounts",
    security(("staffSession" = [], "csrfToken" = [])), request_body = KeyAccountRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. A key is returned only after its audit is committed; never stored in a receipt.")),
    responses((status = 200, description = "Committed key account creation or explicit key disclosure. Handle as a private credential.", body = KeyAccountResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account not found", body = ApiError), (status = 409, description = "Player name taken or retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn key_account(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<KeyAccountRequest>, JsonRejection>) -> Result<Json<KeyAccountResult>, ApiError> {
    let Json(request) = body?;
    sensitive(&viewer)?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.key_account(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/gifts", operation_id = "givePlayerGift", tag = "Accounts",
    security(("staffSession" = [], "csrfToken" = [])), request_body = GiftRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. Retries never give duplicate cards or repeat mail or game notifications.")),
    responses((status = 200, description = "Committed player gift or original retry result", body = GiftResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account not found", body = ApiError), (status = 409, description = "Recipe already owned or retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn give_gift(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<GiftRequest>, JsonRejection>) -> Result<Json<GiftResult>, ApiError> {
    let Json(request) = body?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.give_gift(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/moderation", operation_id = "moderateAccount", tag = "Moderation",
    security(("staffSession" = [], "csrfToken" = [])), request_body = ModerationRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. A retry observes the original saved outcome.")),
    responses((status = 200, description = "Committed moderation plus current disconnect outcome", body = ModerationResponse),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account not found", body = ApiError), (status = 409, description = "Recipe already owned or retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn moderate(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<ModerationRequest>, JsonRejection>) -> Result<Json<ModerationResponse>, ApiError> {
    let Json(request) = body?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.moderate(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/accounts/delete", operation_id = "deleteAccount", tag = "Accounts",
    security(("staffSession" = [], "csrfToken" = [])), request_body = DeleteAccountRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. A retry observes the original saved outcome.")),
    responses((status = 200, description = "Committed account deletion plus current disconnect outcome", body = DeleteAccountResponse),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account not found", body = ApiError), (status = 409, description = "Retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn delete_account(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<DeleteAccountRequest>, JsonRejection>) -> Result<Json<DeleteAccountResponse>, ApiError> {
    let Json(request) = body?;
    sensitive(&viewer)?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.delete_account(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[derive(Deserialize)]
struct CardRevision { v: String }
#[derive(ToSchema)]
#[allow(dead_code, reason="binary response schema")]
#[schema(value_type=String, format=Binary)]
struct CardPng(String);
#[utoipa::path(get, path="/panel/api/v1/cards/{row}/{size}", operation_id="getCardImage", tag="Gifts",
    security(("staffSession"=[])), params(("row"=u16, Path), ("size"=CardImageSize, Path),
        ("v"=String, Query, description="Archive revision from the gift catalogue"),
        ("If-None-Match"=Option<String>, Header, nullable=false, description="Previously received ETag")),
    responses((status=200, description="Original game PNG; private browser cache for one day", body=CardPng, content_type="image/png"),
        (status=304, description="Image unchanged; authentication and permissions rechecked"),
        (status=400, description="Invalid request", body=ApiError), (status=401, description="Staff sign-in required", body=ApiError),
        (status=403, description="Forbidden", body=ApiError), (status=404, description="Image or revision unavailable", body=ApiError),
        (status=405, description="Method not allowed", body=ApiError), (status=413, description="Request too large", body=ApiError),
        (status=415, description="JSON required", body=ApiError), (status=500, description="Internal error", body=ApiError)))]
async fn card_image(State(state):State<Dashboard>, Extension(viewer):Extension<Viewer>, headers:HeaderMap,
    path:Result<Path<(u16,CardImageSize)>,PathRejection>, query:Result<Query<CardRevision>,QueryRejection>)->Result<Response,ApiError> {
    let Path((row,size))=path.map_err(|_|ApiError::new(ApiErrorCode::InvalidRequest,"Choose a card row and mini or full image."))?;
    let Query(query)=query.map_err(|_|ApiError::new(ApiErrorCode::InvalidRequest,"An image revision is required."))?;
    if query.v.len()!=64 { return Err(ApiError::new(ApiErrorCode::NotFound,"This image revision is unavailable.")) }
    let image=tokio::task::spawn_blocking(move ||state.api.card_image(row,size,&query.v,Actor::Staff { user_id:viewer.user_id }))
        .await.map_err(|_|ApiError::new(ApiErrorCode::InternalError,"The image could not be loaded."))??;
    let unchanged=headers.get_all(header::IF_NONE_MATCH).iter().filter_map(|v|v.to_str().ok()).flat_map(|v|v.split(','))
        .any(|v|v.trim()=="*" || v.trim().strip_prefix("W/").unwrap_or(v.trim())==image.etag);
    let mut response=if unchanged { StatusCode::NOT_MODIFIED.into_response() } else { image.bytes.into_response() };
    response.headers_mut().insert(header::CONTENT_TYPE,HeaderValue::from_static("image/png"));
    response.headers_mut().insert(header::ETAG,HeaderValue::from_str(&image.etag).expect("hex ETag"));
    response.extensions_mut().insert(PrivateCardImage);
    Ok(response)
}

#[derive(Default, Deserialize)]
struct GiftSearch { #[serde(default)] q: String }
#[utoipa::path(get, path = "/panel/api/v1/gifts/choices", operation_id = "listGiftChoices", tag = "Accounts",
    params(("q" = Option<String>, Query, max_length = 80, description = "Search game metadata by name or ID; at most 50 results per kind")),
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = GiftChoices),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Gift choices unavailable", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn gift_choices(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, query: Result<Query<GiftSearch>, QueryRejection>) -> Result<Json<GiftChoices>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::new(ApiErrorCode::InvalidRequest, "Use a single search term."))?;
    tokio::task::spawn_blocking(move || state.api.gift_choices(&query.q, Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "Gift choices could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/settings", operation_id = "changeSettings", tag = "Settings",
    security(("staffSession" = [], "csrfToken" = [])), request_body = SettingsRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. Retries return the original result without restoring old settings or moving a countdown.")),
    responses((status = 200, description = "Committed settings or original retry result", body = SettingsResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account not found", body = ApiError), (status = 409, description = "Recipe already owned or retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn change_settings(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<SettingsRequest>, JsonRejection>) -> Result<Json<SettingsResult>, ApiError> {
    let Json(request) = body?;
    sensitive(&viewer)?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.change_settings(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/settings", operation_id = "listSettings", tag = "Settings",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = SettingsList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Settings unavailable", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn live_settings(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<SettingsList>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.live_settings(Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "Settings could not be read."))?.map(Json).map_err(Into::into)
}

pub(crate) fn not_found() -> Response {
    let mut response = ApiError::new(ApiErrorCode::NotFound, "No such dashboard API route.").into_response();
    secure(&mut response);
    response
}

#[utoipa::path(get, path = "/panel/api/v1/server", operation_id = "getServerStatus", tag = "Server",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = ServerOverview),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn server_status(State(state): State<Dashboard>) -> Result<Json<ServerOverview>,ApiError> {
    tokio::task::spawn_blocking(move||state.api.overview()).await.map(Json).map_err(|_|ApiError::new(ApiErrorCode::InternalError,"The server status could not be read."))
}

#[utoipa::path(post, path = "/panel/api/v1/server/stop", operation_id = "stopServer", tag = "Server",
    security(("staffSession" = [], "csrfToken" = [])), request_body = StopRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. A retry observes the original saved outcome.")),
    responses((status = 200, description = "Committed stop request plus current signal outcome", body = StopResponse),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Account not found", body = ApiError), (status = 409, description = "Retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn stop_server(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, headers: HeaderMap,
    body: Result<Json<StopRequest>, JsonRejection>) -> Result<Json<StopResponse>, ApiError> {
    let Json(request) = body?;
    sensitive(&viewer)?;
    let id = one(&headers, "idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move || state.api.request_stop(&request, &id, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The result could not be confirmed. Retry with the same request ID."))?
        .map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/audit", operation_id = "getAuditHistory", tag = "Audit",
    params(("limit" = Option<u32>, Query, minimum=1, maximum=200), ("before" = Option<String>, Query, min_length=1, max_length=19, pattern="^[1-9][0-9]*$", description="Decimal ID returned by next_before; never a JS number"), ("q" = Option<String>, Query, max_length=80)),
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = AuditPage),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 404, description = "No persistent history", body = ApiError), (status = 503, description = "History unavailable", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn audit_history(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, query:Result<Query<AuditQuery>,QueryRejection>) -> Result<Json<AuditPage>,ApiError> {
    let Query(query)=query.map_err(|_|ApiError::new(ApiErrorCode::InvalidRequest,"Invalid history query parameters."))?;
    tokio::task::spawn_blocking(move||state.api.read_audit(&query,Actor::Staff{user_id:viewer.user_id})).await
        .map_err(|_|ApiError::new(ApiErrorCode::InternalError,"The history could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/logs", operation_id = "getServerLogs", tag = "Logs",
    params(("limit" = Option<u32>, Query, minimum=1, maximum=500), ("problems" = Option<bool>, Query, description="Only warnings and errors")),
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = LogPage),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 404, description = "No persistent history", body = ApiError), (status = 503, description = "History unavailable", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn server_logs(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, query:Result<Query<LogsQuery>,QueryRejection>) -> Result<Json<LogPage>,ApiError> {
    let Query(query)=query.map_err(|_|ApiError::new(ApiErrorCode::InvalidRequest,"Invalid history query parameters."))?;
    tokio::task::spawn_blocking(move||state.api.read_logs(&query,Actor::Staff{user_id:viewer.user_id})).await
        .map_err(|_|ApiError::new(ApiErrorCode::InternalError,"The history could not be read."))?.map(Json).map_err(Into::into)
}

#[derive(ToSchema)]
#[allow(dead_code, reason="schema-only wrapper for the streamed SQLite response")]
#[schema(value_type=String,format=Binary)]
struct BackupFile(String);

#[utoipa::path(get, path = "/panel/api/v1/backups", operation_id = "listBackups", tag = "Backups",
    security(("staffSession" = [])), responses((status = 200, description = "Success", body = BackupList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 405, description = "Method not allowed", body = ApiError), (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 404, description = "No backups", body = ApiError), (status = 503, description = "Backup storage unavailable", body = ApiError), (status = 500, description = "Internal error; mutation outcome may be unknown", body = ApiError)))]
async fn backups(State(state):State<Dashboard>,Extension(viewer):Extension<Viewer>,proxy:Option<Extension<ProxyOrigin>>)->Result<Json<BackupList>,ApiError>{
    let mut list=tokio::task::spawn_blocking(move||state.api.read_backups(Actor::Staff{user_id:viewer.user_id})).await.map_err(|_|ApiError::new(ApiErrorCode::InternalError,"The backup list could not be read."))??;
    list.downloads_allowed=proxy.is_none();Ok(Json(list))
}

#[utoipa::path(post, path = "/panel/api/v1/backups", operation_id = "createBackup", tag = "Backups",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CreateBackupRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. A retry observes the original saved outcome.")),
    responses((status = 200, description = "Saved snapshot intent and current outcome", body = CreateBackupResponse),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Backup unavailable", body = ApiError), (status = 409, description = "Retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn create_backup(State(state):State<Dashboard>,Extension(viewer):Extension<Viewer>,headers:HeaderMap,body:Result<Json<CreateBackupRequest>,JsonRejection>)->Result<Json<CreateBackupResponse>,ApiError>{
    let Json(request)=body?;sensitive(&viewer)?;let id=one(&headers,"idempotency-key").unwrap_or_default().to_string();
    tokio::task::spawn_blocking(move||state.api.create_backup(&request,&id,Actor::Staff{user_id:viewer.user_id},"dashboard")).await.map_err(|_|ApiError::new(ApiErrorCode::InternalError,"The backup outcome is unknown. Retry the same request."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/backups/download", operation_id = "downloadBackup", tag = "Backups",
    security(("staffSession" = [], "csrfToken" = [])), request_body = DownloadBackupRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "Retain this ID and input on retry. A retry observes the original saved outcome.")),
    responses((status = 200, description = "Audited snapshot stream. Includes sensitive account data; keep it private.", body = BackupFile, content_type="application/octet-stream"),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Backup unavailable", body = ApiError), (status = 409, description = "Retry input conflict", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError),
        (status = 503, description = "Commit could not be confirmed; retry the same request", body = ApiError), (status = 500, description = "Internal error; outcome may be unknown", body = ApiError)))]
async fn download_backup(State(state):State<Dashboard>,Extension(viewer):Extension<Viewer>,proxy:Option<Extension<ProxyOrigin>>,headers:HeaderMap,body:Result<Json<DownloadBackupRequest>,JsonRejection>)->Result<Response,ApiError>{
    // Through the public proxy (when the host enables it) only an authenticator check in the last five minutes counts.
    let strict=proxy.is_some();
    if strict && !(viewer.mfa_enabled && viewer.sensitive_ok()) || !strict && !viewer.step_up_ok(){return Err(needs_code())}
    let Json(request)=body?;let id=one(&headers,"idempotency-key").unwrap_or_default().to_string();
    let snapshot=tokio::task::spawn_blocking(move||state.api.open_backup_download(&request,&id,Actor::Staff{user_id:viewer.user_id},"dashboard")).await
        .map_err(|_|ApiError::new(ApiErrorCode::InternalError,"The snapshot download could not be opened."))??;
    let mut response=Response::new(Body::from_stream(tokio_util::io::ReaderStream::new(tokio::fs::File::from_std(snapshot.file))));
    response.headers_mut().insert(header::CONTENT_TYPE,HeaderValue::from_static("application/octet-stream"));
    response.headers_mut().insert(header::CONTENT_LENGTH,HeaderValue::from_str(&snapshot.bytes.to_string()).unwrap());
    response.headers_mut().insert(header::CONTENT_DISPOSITION,HeaderValue::from_str(&format!("attachment; filename=\"{}\"",snapshot.filename)).unwrap());
    Ok(response)
}

#[utoipa::path(get, path = "/panel/api/v1/reports", operation_id = "listReports", tag = "Reports", params(ReportQuery),
    security(("staffSession" = [])), responses((status = 200, description = "Player reports, newest first", body = ReportList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn list_reports(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, query: Result<Query<ReportQuery>, QueryRejection>) -> Result<Json<ReportList>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::new(ApiErrorCode::InvalidRequest, "Use a status of open, handled or dismissed."))?;
    tokio::task::spawn_blocking(move || state.api.list_reports(&query, Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The reports could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/problem-reports", operation_id = "listProblemReports", tag = "Problem reports", params(ProblemReportQuery),
    security(("staffSession" = [])), responses((status = 200, description = "Logs players' launchers sent, newest first", body = ProblemReportList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn list_problem_reports(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, query: Result<Query<ProblemReportQuery>, QueryRejection>) -> Result<Json<ProblemReportList>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::new(ApiErrorCode::InvalidRequest, "Invalid problem report query parameters."))?;
    tokio::task::spawn_blocking(move || state.api.list_problem_reports(&query, Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The problem reports could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/problem-reports/{id}", operation_id = "getProblemReport", tag = "Problem reports",
    params(("id" = String, Path, min_length = 11, max_length = 11, pattern = "^[Rr]-[A-Za-z0-9]{4}-[A-Za-z0-9]{4}$", description = "A report ID such as R-7K3Q-M2PA, in any case")),
    security(("staffSession" = [])), responses((status = 200, description = "One report with the server's lines and the launcher's log", body = ProblemReport),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn get_problem_report(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, Path(id): Path<String>) -> Result<Json<ProblemReport>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.read_problem_report(&id, Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The problem report could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/reports/handle", operation_id = "handleReport", tag = "Reports",
    security(("staffSession" = [], "csrfToken" = [])), request_body = HandleReportRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; marking the same report the same way again answers as done.")),
    responses((status = 200, description = "The report, handled or dismissed", body = HandleReportResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn handle_report(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<HandleReportRequest>, JsonRejection>) -> Result<Json<HandleReportResult>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.handle_report(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The report could not be updated."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/banlists", operation_id = "listBanlists", tag = "Accounts",
    security(("staffSession" = [])), responses((status = 200, description = "The game's banlists and the server's default", body = BanlistChoices),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn banlist_choices(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<BanlistChoices>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.banlist_choices(Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The banlists could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/accounts/banlist", operation_id = "setAccountBanlist", tag = "Accounts",
    security(("staffSession" = [], "csrfToken" = [])), request_body = SetBanlistRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; the change is the same when repeated.")),
    responses((status = 200, description = "The banlist the account logs in with from its next login", body = AccountBanlist),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn set_banlist(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<SetBanlistRequest>, JsonRejection>) -> Result<Json<AccountBanlist>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.set_banlist(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The banlist could not be changed."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/events", operation_id = "listEvents", tag = "Events",
    security(("staffSession" = [])), responses((status = 200, description = "The Stadium's official events with their standings, newest first", body = EventList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn list_events(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<EventList>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.list_events(Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The events could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/events/create", operation_id = "createEvent", tag = "Events",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CreateEventRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; the same change again is answered as done.")),
    responses((status = 200, description = "The event, scheduled (an identical one made again is answered as done)", body = EventResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn create_event(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<CreateEventRequest>, JsonRejection>) -> Result<Json<EventResult>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.create_event(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The event could not be scheduled."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/events/edit", operation_id = "editEvent", tag = "Events",
    security(("staffSession" = [], "csrfToken" = [])), request_body = EditEventRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; the same change again is answered as done.")),
    responses((status = 200, description = "The event, changed", body = EventResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn edit_event(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<EditEventRequest>, JsonRejection>) -> Result<Json<EventResult>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.edit_event(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The event could not be changed."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/events/cancel", operation_id = "cancelEvent", tag = "Events",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CancelEventRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; the same change again is answered as done.")),
    responses((status = 200, description = "The event, cancelled", body = EventResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn cancel_event(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<CancelEventRequest>, JsonRejection>) -> Result<Json<EventResult>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.cancel_event(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The event could not be cancelled."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(get, path = "/panel/api/v1/championships", operation_id = "listChampionships", tag = "Championships",
    security(("staffSession" = [])), responses((status = 200, description = "The Championships with their entrants and results, newest first", body = ChampionshipList),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn list_championships(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>) -> Result<Json<ChampionshipList>, ApiError> {
    tokio::task::spawn_blocking(move || state.api.list_championships(Actor::Staff { user_id: viewer.user_id }))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The Championships could not be read."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/championships/create", operation_id = "createChampionship", tag = "Championships",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CreateChampionshipRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; the same change again is answered as done.")),
    responses((status = 200, description = "The Championship, scheduled (an identical one made again is answered as done)", body = ChampionshipResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn create_championship(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<CreateChampionshipRequest>, JsonRejection>) -> Result<Json<ChampionshipResult>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.create_championship(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The Championship could not be scheduled."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/championships/edit", operation_id = "editChampionship", tag = "Championships",
    security(("staffSession" = [], "csrfToken" = [])), request_body = EditChampionshipRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; the same change again is answered as done.")),
    responses((status = 200, description = "The Championship, changed", body = ChampionshipResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn edit_championship(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<EditChampionshipRequest>, JsonRejection>) -> Result<Json<ChampionshipResult>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.edit_championship(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The Championship could not be changed."))?.map(Json).map_err(Into::into)
}

#[utoipa::path(post, path = "/panel/api/v1/championships/cancel", operation_id = "cancelChampionship", tag = "Championships",
    security(("staffSession" = [], "csrfToken" = [])), request_body = CancelChampionshipRequest,
    params(("Idempotency-Key" = String, Header, min_length = 16, max_length = 128, pattern = "^[A-Za-z0-9_-]+$", description = "One ID per change; the same change again is answered as done.")),
    responses((status = 200, description = "The Championship, cancelled", body = ChampionshipResult),
        (status = 400, description = "Invalid request", body = ApiError), (status = 401, description = "Staff sign-in required", body = ApiError), (status = 403, description = "Forbidden", body = ApiError),
        (status = 404, description = "Not found", body = ApiError), (status = 405, description = "Method not allowed", body = ApiError), (status = 409, description = "Already handled", body = ApiError),
        (status = 413, description = "Request too large", body = ApiError), (status = 415, description = "JSON required", body = ApiError), (status = 503, description = "Stored data could not be read or written", body = ApiError), (status = 500, description = "Internal error", body = ApiError)))]
async fn cancel_championship(State(state): State<Dashboard>, Extension(viewer): Extension<Viewer>, body: Result<Json<CancelChampionshipRequest>, JsonRejection>) -> Result<Json<ChampionshipResult>, ApiError> {
    let Json(request) = body?;
    tokio::task::spawn_blocking(move || state.api.cancel_championship(&request, Actor::Staff { user_id: viewer.user_id }, "dashboard"))
        .await.map_err(|_| ApiError::new(ApiErrorCode::InternalError, "The Championship could not be cancelled."))?.map(Json).map_err(Into::into)
}
