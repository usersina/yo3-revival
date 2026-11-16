# Standalone dashboard

Implementation plan agreed on 2026-11-16. This is development work, not a public hosting guide. The existing
[panel](hosting.md#the-admin-panel-through-an-ssh-tunnel) still answers only through loopback.

## Current state

- The approved React/Vite/SWC/Tailwind dashboard is in `dashboard/`. The main pages remain a labelled design preview.
  `/connected` uses the real local API for staff sign-in/out, accounts and Give BP. It never falls back to fixtures.
- Axum handles local HTTP. The staff JSON router is separate from host-token operations; both still use the existing
  loopback listener. There is no public listener or network binding option in this slice.
- Rust handlers and DTOs generate `dashboard/api/openapi.json` through utoipa-axum. Orval generates the committed
  fetch functions, TanStack Query hooks and Zod validators in `dashboard/src/api/generated/`.
- Give BP commits balance, D-Mail, audit and operation receipt together, then publishes state and notifications.
  The dashboard targets a numeric account ID and retains an uncertain request across reloads for safe retry.
- SQLite schema 3 adds `operation_receipts`. Schemas 1 and 2 upgrade transactionally. Other domains are not normalized.
- Remaining work: the other dashboard operations, separate public/private listeners, staff MFA, nginx and Compose.

## Operation boundary

Reuse the existing admin operations and permissions. SQLite/rusqlite stay. The dashboard, CLI and game protocol
share business operations; the dashboard never reads the database or holds the host token. Live duels and
connections remain in memory. Maintenance settings are persisted.

Prepare copies without changing live state. Preserve hub -> accounts -> store lock order. Commit stored changes,
associated mail, audit and receipt in one transaction; only afterwards publish the copies and notify clients.
Store bookkeeping also waits for commit. A failed commit must not leak through a subsequent successful save.

Receipt keys use a stable authenticated identity: `host` for every host-token/offline CLI call, independent of
`--by`, and `staff:<account number>` for a staff session. Recheck permissions before returning a receipt. A matching
request ID and input returns the original result; different input gives a conflict. Successful receipts do not
expire or follow `audit-keep-days`. They keep the input fingerprint and a minimal result, not player names or
credentials. Clients retain an ID for retries and generate a new one only for a new intended change.

Give BP is the first operation. Extend this boundary one operation at a time, fixing save failures without waiting
for a schema rewrite. For irreversible effects, persist intent first and outcome afterwards. Mixed operations
commit their stored changes with intent, then perform the effect. After a restart, show unfinished effects as
interrupted/outcome unknown, never as definitely failed, and never automatically repeat them.

## API and dashboard

Rust is the source of truth. `crates/yo3-server/src/dashboard.rs` registers annotated handlers with `OpenApiRouter`;
its `split_for_parts()` collects the schema without server state, configuration, game files or a database. The
`openapi` binary writes it to stdout, and the local server exposes the same document at `/panel/openapi.json`.

From `dashboard/`, `npm run api:generate` exports Rust's document and runs pinned Orval. It generates request/result/error
models, native-fetch functions, TanStack Query hooks and Zod validators. Explicit operation IDs keep hook names stable
when Rust functions are renamed. All declared error responses use `ApiError` with stable codes. Mutations require an
`Idempotency-Key` argument; the shared mutator supplies the session's CSRF header and same-origin credentials. The
HttpOnly session cookie never becomes frontend data. Sign-in/out require a request ID but do not create durable grant
receipts. Only Give BP currently has the atomic mutation boundary.

The Rust drift test compares the collected document with the committed file under `cargo nextest`, without Node.
`npm run api:check` independently runs Orval's non-writing outdated check. Both gates run before release publishing,
with dashboard lint, typecheck, unit/browser tests and build. Generated files carry a do-not-edit header and are
excluded from lint/formatting, but remain typechecked. Never edit generated output or handwritten copies of API DTOs.
Use tagged serde enums for payload variants, `u32` (bounded in the schema) or string IDs, and RFC 3339 date-time strings.
Zod validators check fixtures and actual browser-test responses; the mutator also validates error bodies, including
unexpected proxy failures. The game protocol is not an HTTP/OpenAPI API.

Connected routes in this slice:

| Method | Path | Result |
| --- | --- | --- |
| GET | `/panel/api/v1/session` | Staff identity, CSRF token and RFC 3339 idle expiry |
| POST | `/panel/api/v1/sign-in` | Staff session and HttpOnly cookie; shared sign-in throttle |
| POST | `/panel/api/v1/sign-out` | End the current staff cookie |
| GET | `/panel/api/v1/accounts` | Safe account/player summaries without credentials |
| POST | `/panel/api/v1/give-bp` | Admin-only grant to `user_id`; exact result and receipt on retry |

Every route checks the loopback peer and local Host. Posts also require same-origin evidence, and authenticated posts
require the CSRF header. Host-token, restore and credential-export operations are absent from the staff router. The
existing HTML panel remains available, sharing sessions and the sign-in throttle. Password/recovery changes invalidate
its cookies as well as JSON API cookies. Full sign-out-everywhere/MFA revocation remains part of the public gate.

For a disposable local server and the connected page, see [local development](#local-development).

Connect the approved UI while access stays private. Replace production fixtures and simulated success with server
responses. Cover loading, empty results, permissions, lost connections, expired sessions and uncertain mutation
outcomes. Preserve the old panel's capabilities and security tests before removing it.

## Local development

In a source checkout, with Rust and Node.js 22.12 or newer:

```sh
# Terminal 1, repository root: disposable local data and accounts, no game files required.
cargo run -p yo3-server --example dashboard_dev -- 18080
# Terminal 2:
cd dashboard
npm ci
npm run dev
```

Open `http://127.0.0.1:5173/connected`. Use `TestAdmin` or `TestModerator`, password `DashboardDemo123!`.
These are example accounts in a fresh temporary directory; no existing data is opened. Ctrl+C stops the example and
removes its directory. Vite's proxy targets loopback port 18080; set `YO3_DASHBOARD_API` before `npm run dev` for a
scratch server on another local port. Keep both ports private.

From `dashboard/`, `npm run api:generate` regenerates Rust's spec and the frontend. `npm run api:check` checks client
drift without writing. From the root, `cargo nextest run -p yo3-server --test dashboard_contract` checks the schema
and the actual API without Node. Frontend gates are `npm run format:check`, `npm run lint`, `npm run typecheck`,
`npm test`, `npm run build` and `npm run test:e2e` (install Chromium once with
`npx playwright install chromium --only-shell`). Browser tests use private ports 5174 and 18181 and disposable state.

## Separate authority and listeners

The planned public management option supersedes D7's tunnel-only product decision, not the existing implementation's
safety checks. Public access stays disabled until the gate below is implemented and verified (#43, #47).

| Listener | Access and authority |
| --- | --- |
| Game TLS | Public TCP 7443, existing authenticated game transport |
| Local management | Loopback, unpublished; host-token CLI API |
| Dashboard API | Private Compose network; staff-session router only, never host-token authority even from 127.0.0.1 |
| nginx | Public HTTPS; dashboard files and an explicit allowlist of dashboard API routes on one browser origin |

Restore remains an offline CLI command with the server stopped and the data folder exclusively locked. Host-token
routes are absent from the dashboard router and nginx's allowlist. Test this on the dashboard listener with a real
host token and a loopback peer. Source address alone never grants local authority.

## Public access gate

- Staff MFA before usable dashboard sessions. Include enrollment, protected TOTP seeds, replay checks, throttling,
  hashed single-use recovery codes and local administrative recovery. Require recent MFA for sensitive actions.
- Explicit staff-session revocation after password recovery, MFA reset and sign-out everywhere; live role/standing
  checks. Test existing panel cookies, not only game sessions. Keep passwords, factors and recovery codes out of logs.
- Configured public host/origin and a specific trusted proxy. nginx overwrites `X-Forwarded-For` with `$remote_addr`,
  sets Host and `X-Forwarded-Proto` explicitly, and clears incoming `Forwarded`. No client-supplied forwarding chain
  is accepted. Rust trusts only that single proxy hop. Rust throttles remain authoritative for sign-in, MFA and
  recovery attempts; nginx `limit_req` is an additional outer limit.
- certbot certificates and a renewal timer with nginx reload. Document initial issuance, renewal and recovery, and
  check expiry and renewal failures before a certificate can lock staff out.
- nginx owns HSTS and the static dashboard's CSP. Rust owns API security headers. Use shared header includes per
  nginx location with `always`; do not rely on inherited `add_header` directives surviving a location override.
- Explicit allowed API prefixes and a default 404, plus normal static dashboard routes. No proxy catch-all. Verify
  forged Host and forwarding headers through the real nginx configuration, not only a Rust request fixture.
- Keep public backup downloads disabled by default. If enabled: admin permission, recent MFA and auditing. Never
  expose offline restore or the host token. Review session, CSRF, proxy, brute-force and MFA recovery boundaries before
  publishing a port.

## Packaging and later work

Separate server/engine and dashboard/nginx images in Compose. Persistent SQLite data, read-only user-supplied game
assets, protected keys outside the dashboard, health checks and graceful shutdown. Rehearse backup and restore.
Taskfile stays a convenience wrapper for build, up, down, status and logs.

Normalize domains gradually behind the API, with versioned migrations, constraints and tested restore. The first
operation needs no PostgreSQL/Turso migration, generic repository framework or new authentication provider.
