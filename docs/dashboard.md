# Standalone dashboard

Implementation plan agreed on 2026-11-16. This is development work, not a public hosting guide. The existing
[panel](hosting.md#the-admin-panel-through-an-ssh-tunnel) still answers only through loopback.

## Current state

- The approved React/Vite/SWC/Tailwind dashboard is in `dashboard/`. It uses labelled sample data, not a server.
- Give BP uses typed requests, results and error codes. Its account, D-Mail, audit and operation receipt commit
  together, then its prepared state and notifications are published. Contract: `grant_contract.rs`.
- SQLite schema 3 adds `operation_receipts`. Schemas 1 and 2 upgrade transactionally. No other domain is normalized.
- Axum, OpenAPI, generated clients, public listeners, MFA, nginx and Compose remain to be implemented.

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

Use Axum and utoipa route/schema generation. Replace operation results and errors with typed structs and stable
codes incrementally. Commit generated OpenAPI and check regeneration in CI; validate actual success/error responses
against it. Generate fetch functions and TanStack Query hooks with Orval, including cookie/CSRF handling and stable
mutation request IDs. The game protocol is not an HTTP/OpenAPI API.

Connect the approved UI while access stays private. Replace production fixtures and simulated success with server
responses. Cover loading, empty results, permissions, lost connections, expired sessions and uncertain mutation
outcomes. Preserve the old panel's capabilities and security tests before removing it.

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
