# Standalone dashboard

Implementation plan agreed on 2026-11-16. This is development work, not a public hosting guide. The existing
[panel](hosting.md#the-admin-panel-through-an-ssh-tunnel) still answers only through loopback.

## Current state

- The approved React/Vite/SWC/Tailwind dashboard is in `dashboard/`. The main pages remain a labelled design preview.
  `/connected` uses the real local API for staff sign-in/out, account summaries/details, player edits and staff roles, Give BP, registration invites and BOOSTERPASS code management.
  It never falls back to fixtures.
- Axum handles HTTP on separate sockets. Host-token operations, assets and the old HTML panel stay on `--http-port`.
  The staff JSON router has its own opt-in `--dashboard-port`, disabled by default and always bound to loopback.
  There is no public dashboard binding option before the MFA/proxy gate.
- Rust handlers and DTOs generate `dashboard/api/openapi.json` through utoipa-axum. Orval generates the committed
  fetch functions, TanStack Query hooks and Zod validators in `dashboard/src/api/generated/`.
- Give BP commits balance, D-Mail, audit and operation receipt together, then publishes state and notifications.
  The dashboard targets a numeric account ID and retains an uncertain request across reloads for safe retry.
- Creating/removing BOOSTERPASS codes commits the code change, audit and receipt together. Moderators can inspect
  codes and claims; only admins can change them. The CLI and old HTML forms use the same typed operations.
- Admins and moderators can list and create registration invites through the generated API. Creation commits the
  invite, audit and receipt together; a retry preserves the original code, issuer, limits and expiry.
- Account details project standing, mute, device/reset counts, saved player statistics and checked live presence/records,
  without credentials. Player creation, renaming and country changes commit profile/audit/receipt together. Moderators
  can rename their own or non-staff players; adding players and changing country remain admin-only.
- Role changes are admin-only and use the same profile/audit/receipt transaction. A stored staff-session revision
  invalidates old cookies on both interfaces, even if an account is demoted and promoted again without making a request.
  Setting the same role again does not invalidate a fresh session; a retry never reapplies an older role. The React
  form requires confirmation and warns before changing your own role.
- SQLite schema 3 adds `operation_receipts`. Schemas 1 and 2 upgrade transactionally. Other domains are not normalized.
- Remaining work: the other dashboard operations, staff MFA, the public-access/proxy gate, nginx and Compose.

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
login credentials. A generated reward code is retained so a retry can retrieve it. Clients retain an ID for retries
and generate a new one only for a new intended change. IDs share a namespace across operations for each actor.

Give BP, invite creation, player profile/role edits and BOOSTERPASS creation/removal use this boundary. Shared identity, error and receipt helpers live in
`api/mutation.rs`; domain validation, preparation and commit stay in their operation modules. A generated-code retry
returns the original code. A removal retry returns its original result even if someone has since recreated that
code; it never deletes the replacement. Codes use canonical uppercase input, exact 16-letter/digit validation and
stable `already_exists`, `not_found` and `request_conflict` errors. These operations have no external effects to replay.
Invite creation records the actual authenticated issuer. Null limits mean unlimited sign-ups/no expiry; zero is
rejected. Listing distinguishes usable, exhausted and expired invites, includes remaining sign-ups and claims, and
reports the time at which status was checked. Expiry must fit RFC 3339; invalid stored expiry is a read error, never
an unlimited invite. Retrying a used-up or expired invite does not replenish or extend it.
Profile edits target numeric account/player IDs in the dashboard. The CLI and old panel use the same operations;
name-based retries look up receipts before resolving an old name. Receipts keep only IDs and a generic result, not
player names. Renames, new player slots and country changes take effect in the game after signing in again; no new
wire push is invented. The maximum is four players and names are 1–16 printable characters, as in the existing
player-creation checks. Country flags are read from `package/data002.ypk`; no built-in allowlist substitutes for missing
assets. Automatic remains available without that file. Unknown presence/records are null when no hub is running;
collection counts explicitly exclude the starter deck. Deleted/unauthorized account details are hidden on refresh.
Role edits use `kind: set_role` and a typed player/moderator/admin value on the same endpoint. Only password accounts
can have a staff role. CLI `set-role USER ROLE --request-id ID` and the old HTML form share this transaction.
A self-demotion removes management permissions; another admin or the host must restore them. A lost response
must not be treated as a failed change: inspect the current role before issuing a new request. Account creation
and reset-code delivery still need a safe retry design that keeps credentials out of permanent receipts.

The browser retains pending IDs through session/permission/CSRF refusals too: those checks run before receipt lookup,
so a refusal cannot prove an earlier attempt did not commit. Refresh the staff session and retry the same intent.

Extend this boundary one operation at a time, fixing save failures without waiting for a schema rewrite.
For irreversible effects, persist intent first and outcome afterwards. Mixed operations
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
receipts. Give BP, invite creation, player edits and code creation/removal currently have the atomic mutation boundary.

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
| GET | `/panel/api/v1/accounts/{user_id}` | Staff-visible profile details and checked live state; credentials excluded |
| GET | `/panel/api/v1/countries` | Country codes from the game archive, or an explicit unavailable error |
| POST | `/panel/api/v1/accounts/edit` | Tagged `add_player`, `rename_player`, `set_country`; stable IDs and retained retry key |
| POST | `/panel/api/v1/give-bp` | Admin-only grant to `user_id`; exact result and receipt on retry |
| GET | `/panel/api/v1/bp-codes` | Staff-visible codes, rewards, limits and claims, with RFC 3339 times where known |
| POST | `/panel/api/v1/bp-codes` | Admin-only creation; omit `code` to generate one, `uses` for unlimited accounts |
| POST | `/panel/api/v1/bp-codes/remove` | Admin-only removal by code; previously credited BP stays with players |
| GET | `/panel/api/v1/invites` | Staff-visible invites, remaining uses, expiry, issuer, claims and checked status |
| POST | `/panel/api/v1/invites` | Admin/moderator creation; omit `uses` for unlimited sign-ups, `days` for no expiry |

Every route checks the loopback peer and local Host. Posts also require same-origin evidence, and authenticated posts
require the CSRF header. Host-token, restore and credential-export operations are absent from the staff router. The
existing HTML panel remains on the local management listener, sharing sessions and the sign-in throttle.
Password/recovery changes invalidate its cookies as well as JSON API cookies. Full sign-out-everywhere/MFA revocation
remains part of the public gate. The management listener does not mount the staff API or its schema; the dashboard
listener never falls through to host-token operations, the HTML panel, backups, health details or game assets.

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
removes its directory. Its dashboard API uses loopback port 18080; local management/assets use a separate, automatically
assigned loopback port printed at startup. Vite's proxy targets the dashboard port; set `YO3_DASHBOARD_API` before
`npm run dev` for a scratch server on another local port. Keep development ports private.

For a normal source server, opt in with `--dashboard-port 18081`, or `dashboard_port = 18081` in its TOML file.
Omitting it leaves the API disabled; `0` assigns a free port, reported at startup. `--bind` affects only game TLS,
even when set to `0.0.0.0`. A dashboard port conflict stops startup before database initialization/migration or live
CLI discovery, rather than silently dropping the requested service. All listeners stop with the server. These
options support private development only; do not proxy this unfinished API publicly.

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
| Local management | Loopback, unpublished; host-token CLI API, assets and existing HTML panel |
| Dashboard API | Separate loopback socket today, disabled by default; private Compose network only after the public-access gate; staff-session router only, never host-token authority even from 127.0.0.1 |
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
