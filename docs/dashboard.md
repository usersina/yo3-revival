# Standalone dashboard

Implementation plan agreed on 2026-11-16. For deployment and HTTPS, see [Docker Compose on a VPS](docker.md). The existing
[panel](hosting.md#the-admin-panel-through-an-ssh-tunnel) still answers only through loopback.

## Current state

- The React/Vite/SWC/Tailwind dashboard in `dashboard/` opens on real staff sign-in and has separate Overview, Accounts,
  Invites & codes, News & maintenance, Moderation, Audit log, Server logs, Backups and Settings pages. It uses the local API for staff sign-in/out, account summaries/details, player edits, staff roles, password-account setup/reset codes and explicit key-account creation/disclosure, card/recipe/title gifts, Give BP, registration invites, BOOSTERPASS code management, live settings, news, maintenance, moderation, account deletion, live server status/stop, audit history, captured server logs and audited database backups.
  It never falls back to fixtures. The sample design is development-only at `/design/`; `/connected` is a
  development integration view. Neither preview is a production route. Page code loads on demand.
- Axum handles HTTP on separate sockets. Host-token operations, assets and the old HTML panel stay on `--http-port`.
  The staff JSON router has its own opt-in `--dashboard-port`, disabled by default and always bound to loopback.
  A separate Unix socket now connects the staff-only router to nginx for Compose hosting. It requires an explicit
  web origin and trusts forwarding headers only on that filesystem transport. The TCP router stays loopback-only.
- Rust handlers and DTOs generate `dashboard/api/openapi.json` through utoipa-axum. Orval generates the committed
  fetch functions, TanStack Query hooks and Zod validators in `dashboard/src/api/generated/`.
- Give BP commits balance, D-Mail, audit and operation receipt together, then publishes state and notifications.
  The dashboard targets a numeric account ID and retains an uncertain request across reloads for safe retry.
- Settings, news and maintenance commit the whole snapshot, audit and receipt before publication. The generated
  registry supplies types, bounds, choices, defaults and application times. Maintenance shows its checked status
  and absolute deadline; retries never restore an older setting, move a countdown or repeat cancellation chat.
  Save, audit, receipt and deferred-COMMIT failures publish nothing, including after a subsequent successful save.
- Creating/removing BOOSTERPASS codes commits the code change, audit and receipt together. Moderators can inspect
  codes and claims; only admins can change them. The CLI and old HTML forms use the same typed operations.
- Admins and moderators can list and create registration invites through the generated API. Creation commits the
  invite, audit and receipt together; a retry preserves the original code, issuer, limits and expiry.
- Account details project standing, mute, device/reset counts, saved player statistics and checked live presence/records,
  without credentials. Player creation, renaming and country changes commit profile/audit/receipt together. Moderators
  can rename their own or non-staff players; adding players and changing country remain admin-only.
- Role changes are admin-only and use the same profile/audit/receipt transaction. A stored staff-session revision
  invalidates old cookies on both interfaces, even if an account is demoted and promoted again without making a request. Pending setup/reset codes are invalidated too.
  Setting the same role again does not invalidate a fresh session; a retry never reapplies an older role. The React
  form requires confirmation and warns before changing your own role.
- Key-account creation and key disclosure are admin-only transactions: account, audit and receipt commit before
  any credential is returned. Account lists/details never expose keys. The React dialog confirms passwordless access;
  only its pending input/ID is retained across reloads, never the revealed key. Closing clears the credential.
- Card, recipe and title gifts use numeric player/catalogue IDs. Card/recipe D-Mail, player state, audit and receipt
  commit together before any bell, collection or present notification; titles keep their existing no-mail behavior.
  The admin-only form searches real game metadata, identifies artworks explicitly, confirms the recipient and retains
  an uncertain request across reloads. It shows catalogue failures rather than substituting sample cards.
- Moderation saves standing/mute/device changes with audit and receipt. Live disconnects have a durable intent
  before execution and an outcome afterwards. Retries never repeat them. The confirmation explains each action,
  warns before acting on your own account, enforces staff permissions and retains uncertain requests across reloads.
- SQLite schema 6 extends `operation_effects` with backup intents and known-failed outcomes beside disconnects and run-bound stops. Schemas 1–5 upgrade transactionally, preserving old effects and receipts.
  Other domains are not normalized.
- Overview includes actual engine/memory metrics, backup health, online-player links and recent problems. Memory
  bytes are decimal strings; missing and unreadable backup storage are distinct states. Account lists project the
  same effective standing/mute badges as details, with filters and numeric account-detail links.
- Staff sign in with password and an authenticator code (TOTP); see [Staff MFA and recovery](#staff-mfa-and-recovery).
  The public HTTPS/password-only testing that the user approved before MFA ends with this change: an account with no
  authenticator gets a session that can only enroll. Compose/nginx and HTTPS setup are implemented; the real VPS
  deployment/soak remain. The old private HTML panel remains available on loopback, and asks for the code too.

## Staff interface

The sign-in screen uses the project logo and a compact centered form. Connection errors offer a retry; refused
credentials stay visible without clearing the User ID, including after a session expires.

The theme button is available before sign-in and in the workspace header. **Settings → Appearance** offers
**System**, **Light** and **Dark**. The choice is saved per browser and synchronized between tabs; System follows
device changes without discarding forms. If browser storage is blocked, switching still works for that page.
Both palettes use green for confirmed actions, amber for warnings or uncertain outcomes, red for errors and
destructive controls, and blue for information. Status text continues to explain the outcome.

Shared dropdowns, text fields and setting groups use locally owned shadcn/ui components on Radix, with the same
semantic light/dark palette. Dropdowns support keyboard navigation, Escape and focus return, including inside dialogs.
Field hints and errors are linked to their inputs. Validation appears after blur or an attempted submission, clears
as the value is corrected, and focuses the first invalid input without sending the change.

Server settings are grouped into access, duels, capacity/traffic, sign-in protection, maintenance and audit/other.
Each group can collapse independently. Searching or filtering reveals matching groups; clearing filters restores
which groups you had open. Search includes readable labels, internal names and help text; group and custom-value
filters can narrow the list.
Unknown setting names appear under **Audit & other**. The server still supplies values, bounds, choices and application
times, and edits retain their existing confirmations, permissions and safe retries.

Navigation collapses at tablet widths. Account, invite, code, audit and backup tables switch to labelled
rows in narrow panels, keeping actions in view.
Searches have a clear button, account filters show the result count and an empty result offers **Clear filters**.
Long dialogs scroll their body while keeping the heading and action buttons visible. Backup download confirmations
use the same body inset as other dialogs. Nested settings and moderation forms keep a consistent field gap, and
log output aligns with its panel heading.

**Accounts → View** opens a full page at `/accounts/<id>`. The balance, player slots and standing appear first,
followed by player cards with rewards and rename controls, moderation, account access and country. Admin-only
deletion stays at the bottom. Short action dialogs keep the account underneath; closing or cancelling returns
to that page. Successful balance and profile changes refresh the visible data. Uncertain requests retain their
original retry ID, including after reload, and permissions still govern every action.

**Reports** (`/reports`) lists the players' reports from the game's chat command, Open first, with a status filter. Each
row links both players to their account pages; **Handle** and **Dismiss** open a dialog with an optional note. On an
account page, **Banlist** shows the list the account logs in with (its own choice with `/banlist`, or the server's) and
lets staff choose another or go back to the server's; it applies at the player's next login. `dashboard/e2e/reports.spec.ts` checks both.

**Events** (`/events`) lists the Stadium's official events with their kind, times, status, entrants, prize and the best
scores so far. An admin schedules one with **Schedule event** (kind, name, start and end in the browser's time zone, an
optional banlist, BP for taking part and BP for reaching a score), edits one that has not ended, or cancels it (nothing is
paid); a moderator only looks. Each change is audited. `dashboard/e2e/events.spec.ts` checks it, and `official_admin_contract.rs`
the same operations from the command line ([guide](guide.md#official-events-the-stadium)).

**Championship** (`/championship`) lists the Championships with their entry window, start, status, entrants, BP prizes and
the winner. An admin schedules one with **Schedule Championship** (name, when entries open and close and the start, in the
browser's time zone and within one day, the minimum duels and the BP prizes), edits one that has not started, or cancels it;
a moderator only looks. Each change is audited. `championship_admin_contract.rs` and `dashboard_contract.rs` check the same
operations; the page itself is checked by typecheck and lint, and `dashboard/e2e/navigation.spec.ts` and `ergonomics.spec.ts`
list it, but its own browser test is not written yet ([guide](guide.md#the-championship-the-maps-tournament-tile)).

**Give a gift → Cards** searches the host's catalogue and lists each artwork separately, with keyboard-selectable
rows and original 50×72 thumbnails. A separate **Preview** button opens the original 200×290 card and readable
2010–2012 game text without selecting or giving anything. Escape returns to the selected gift and copy count.
Copy counts are checked before confirmation, and the confirmation stays beside the submit button. **Card details**
opens Konami's current database; its wording can differ from the game's original text.

Artwork comes from the host's read-only `package/data000.ypk` (`data/mini/<cid>_<art>.png`) and
`package/data001.ypk` (`data/card/<cid>_<art>.png`), using the catalogue's exact Konami ID and artwork, never a
passcode guessed from another database. The client loader at `0x4FB090` constructs the full-image path; `yo3-data`
already decodes the corresponding artwork bits. Descriptions come from `CARD_Desc_E.bin`. No third-party host,
image download, bucket or public archive URL is needed. Missing/corrupt images show a fallback; gifting still works.

`GET /panel/api/v1/cards/{row}/{size}?v=REVISION` accepts `mini` or `full`, current admin sessions only. The catalogue
supplies the URLs and source revision. Rust retains each archive's index, bounds compressed and decoded reads to
512 KiB, checks PNG type/dimensions and returns a content ETag. Successful images alone use
`Cache-Control: private, max-age=86400`; an authenticated `If-None-Match` can return 304. Refusals and other APIs
retain `no-store`. nginx does not share-cache authenticated images. The browser loads thumbnails lazily and full
cards only when opened; the catalogue query remains fresh in memory for one minute. Replace archives while the
server is stopped, then restart: the new file timestamp/size yields new URLs, so no browser-cache clearing is needed.
This API, generated client and nginx allowlist change ship together in a full server release.

All 33 documented staff operations are available to the dashboard (image elements use the generated catalogue URLs).
Account pages use the released API without
changing its schema or generated client. Their new nginx route requires a full release; the independent UI gate
must not deliver it against an older nginx configuration. Responsive and accessibility checks are in
`dashboard/e2e/ergonomics.spec.ts` and `dashboard/e2e/auth-layout.spec.ts`; gift retry and permission checks remain
in `dashboard/e2e/connected.spec.ts`. `dashboard/e2e/account-page.spec.ts` checks direct links, history, permissions,
action dialogs and safe balance retries; production browser checks reload account URLs through real nginx.
`dashboard/e2e/theme-settings.spec.ts` checks saved/system themes, tab synchronization, blocked storage, grouped and
future settings, dark-mode contrast, narrow layouts and form preservation. Production browser checks also exercise
themes and settings under nginx's existing content-security policy. `dashboard/e2e/controls.spec.ts` covers
collapse/search, keyboard selection, mobile popup placement, inline errors, invalid-submit focus and light/dark accessibility. The shared field regression test covers programmatic resets and
screen-reader descriptions. These interface changes use the existing API.

## Independent UI delivery

Full releases still update both services. Issue [#62](https://github.com/usersina/yo3-revival/issues/62) adds a
separate path for compatible frontend changes. The input/contract and browser gates, asset image and restricted
replacement/rollback transaction are implemented. Runtime equality is checked in CI/local checks and again by the
updater. The automatic workflow reconciles current `main` after each successful Release run; manual retries use
Actions → Deploy dashboard. It retains release images plus the active/previous UI trees. The gate runs the deployed
release server image, never unreleased Rust from `main`. The footer separates server version from UI commit, and
old tabs offer an explicit reload without interrupting forms. Published v0.1.0 compatibility passed locally; the
first real workflow-token deletion probe remains to be checked on GitHub.
See [deployment checks](deployment.md#independent-dashboard-updates-62).

The 2026-11-18 UI rehearsal used the VPS's exact v0.1.0 server/dashboard digests with the new frontend assets.
Runtime equality, production browser checks and a disposable registry/Compose replacement passed, including
duplicate/stale-request refusal, rollback and retry with the same server, account data and identity.
The live planner still defers delivery because the unpublished Docker/deployer changes differ from v0.1.0;
those foundation changes must ship in a full release before independent UI delivery can start.

## Database backups

Admins create snapshots through `POST /panel/api/v1/backups`; moderators can only list them. The CLI's
`backup --request-id ID` and old HTML form use the same boundary. Audit, receipt and intent commit before the
filesystem effect. A checked SQLite copy is synced and published by directory rename; unfinished copies never
appear in the list or count toward retention. The saved outcome is completed, failed, pending or interrupted.
A retry only observes that outcome, including after restart/audit pruning or snapshot rotation; it never makes a
replacement copy. Download availability is reported separately from the historical creation outcome.

`POST /panel/api/v1/backups/download` streams the snapshot only after committing an admin access audit and receipt.
A sign-in within five minutes is required on the private interface. Public nginx/socket downloads are refused,
and the backup list reports `downloads_allowed: false` so React shows SSH copy guidance instead of download buttons. The React confirmation warns about password hashes, account data and audit history.
Only request metadata is kept across reloads, never backup bytes. The generated response type is a Blob; JSON
errors remain typed and are never saved as a database. A successful response means a download started, not proof
that the browser saved it. The legacy HTML download is audited too and capped at 64 MiB; use the new dashboard
or SSH for larger snapshots. Restore remains offline, with the server stopped and the data folder locked.

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

Give BP, invite creation, player profile/role edits, password setup/reset codes, key-account creation/disclosure, player gifts, settings/news/maintenance and
BOOSTERPASS creation/removal use this boundary. Shared identity, error and receipt helpers live in
`api/mutation.rs`; domain validation, preparation and commit stay in their operation modules. A generated-code retry
returns the original code. A removal retry returns its original result even if someone has since recreated that
code; it never deletes the replacement. Codes use canonical uppercase input, exact 16-letter/digit validation and
stable `already_exists`, `not_found` and `request_conflict` errors. These operations have no external effects to replay.
Invite creation records the actual authenticated issuer. Null limits mean unlimited sign-ups/no expiry; zero is
rejected. Listing distinguishes usable, exhausted and expired invites, includes remaining sign-ups and claims, and
reports the time at which status was checked. Expiry must fit RFC 3339; invalid stored expiry is a read error, never
an unlimited invite. Retrying a used-up or expired invite does not replenish or extend it.
An account's page lists its **linked accounts** (seen on the same PC or address, with which of the two, read-only: `linked_accounts` in the account detail, from `Directory::linked_to`; [fair play](hosting.md#linked-accounts-and-fair-play)).
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
must not be treated as a failed change: inspect the current role before issuing a new request. Password-account creation and reset-code delivery now use caller-generated random one-time codes. No password is
chosen by staff. The code travels only in the authenticated request; account storage keeps its digest and permanent
receipts keep IDs, expiry and kind. The response contains the original confirmation plus current availability
(`ready`, `expired`, `used_or_replaced`). A retry never reactivates or extends a code. The dashboard retains it in
this tab until Done, confirmed expiry or tab closure, and checks with the server after reload before showing it.
Unconfirmed requests retain their original ID/code for retry. The CLI accepts the original code through
`--code-stdin`; old HTML forms retain a hidden code. Initial setup uses the existing recovery operation to atomically
set the owner’s password and private recovery code. Staff promotion waits until setup is complete. Password
recovery rechecks code/revision after hashing; sign-out everywhere invalidates staff cookies. Other unmigrated admin operations remain separate work.

The browser retains pending IDs through session/permission/CSRF refusals too: those checks run before receipt lookup,
so a refusal cannot prove an earlier attempt did not commit. Refresh the staff session and retry the same intent.

Extend this boundary one operation at a time, fixing save failures without waiting for a schema rewrite.
For irreversible effects, persist intent first and outcome afterwards. Mixed operations
commit their stored changes with intent, then perform the effect. After a restart, show unfinished effects as
interrupted/outcome unknown, never as definitely failed, and never automatically repeat them.

Key operations use `POST /panel/api/v1/accounts/keys` with a tagged `create` (player names) or `reveal`
(numeric account ID) request. They are explicit mutations even when a key already exists, so a disclosure is audited.
Receipts hold only the account ID, request ID and key digest. A retry resolves the same account’s current players
and verifies its key against that digest before disclosure; deletion cannot recreate the account, and a changed key
requires a new request. Roles are checked again before a replay. Keys already persist in the private account database;
this does not add a second plaintext credential store. Friends should still join with password accounts and invites.

`POST /panel/api/v1/gifts` takes a tagged card/recipe/title request and stable numeric IDs. The receipt records
only IDs, quantities and a generic confirmation, never a player name. The CLI and old panel reconstruct readable
card/recipe/title labels from available metadata; missing files cannot turn an already committed receipt into failure. Retries acknowledge the saved gift without
repeating mail or presentation, even after restart or audit pruning. Cards and recipe cards must fit the stored
additional-copy limit of 255 per artwork; recipe duplicates are counted together before any change. An already-owned
recipe without cards is refused; granting an already-owned title confirms that state without adding a second item.
`GET /panel/api/v1/gifts/choices?q=…` searches game metadata and returns at most 50 results per kind with explicit
more-results flags. Neither endpoint accepts moderator or host-token authority on the staff listener.

`GET /panel/api/v1/settings` is readable by current staff. It returns a coherent registry snapshot, default/override
values as editable strings, tagged input metadata, a checked-at RFC 3339 time and maintenance deadline/status.
`POST` is admin-only and accepts tagged `set`, `reset`, `schedule_maintenance` or `cancel_maintenance` requests.
Scheduling validates the message and absolute deadline together. Settings mutations alone accept up to 256 KiB
so ten valid multilingual announcements fit; other routes retain their 16 KiB request limit. `--off` cannot be combined with scheduling options.
CLI `set`, `reset` and `maintenance` print and accept `--request-id`; existing HTML forms keep their hidden ID.
Receipts retain the original confirmation (which can contain the setting value) and deadline. Audit pruning is a
post-commit best-effort effect: failure is logged, does not undo the setting and does not repeat on receipt replay.
The existing one-second maintenance worker still handles sign-outs at the committed deadline. No client wire shape
changes. The game’s cancelled-maintenance icon still clears at the next login; cancellation chat explains that.

`POST /panel/api/v1/moderation` accepts tagged `kick`, `ban`, `restrict`, `lift`, `mute`, `unmute` and `sign_out`
requests with numeric account IDs. Current admins/moderators can act; moderators cannot act on another staff account,
including on receipt replay. Only disconnect supports key-only accounts. Reasons are optional, single-line and at
most 200 characters. Durations are positive days/minutes or null (until lifted), and must fit a date before year 10000.
The immutable result holds only account ID, operation, original expiry, request ID and a generic confirmation.
The response's `disconnect` field is separate: null when no live effect was needed, otherwise `pending`, `completed`
or `interrupted`. Completed means server sessions/tickets were ended, not that a remote window has closed.

The account change, initial audit row, receipt and pending effect commit together. Account state is published, then
the server ends live sessions while holding the account lock, then updates effect and audit outcome together.
Failure before that first commit changes nothing. Failure recording the outcome leaves the committed account change
and a pending/unknown disconnect. Startup under the data lock marks pending effects interrupted and updates their
surviving audit rows; it never executes them. Effects survive audit pruning. Even a receipt replay after a new sign-in
only reports the stored result/current outcome. A fresh disconnect requires a new, explicitly confirmed action.
The React dialog retains a known uncertain effect until Done and offers Check outcome using the original ID.

Ban/sign-out clear devices and revoke staff cookies; restriction also revokes old staff cookies so lifting it cannot
restore a previous cookie. A password verified before moderation must recheck standing and session revision before
session issuance; key admission holds the same account -> session lock order. A remembered device is not saved if
moderation/recovery invalidated its password snapshot. The CLI's seven moderation commands accept `--request-id`;
old HTML forms reuse their hidden ID. Backup operations are still separate work.

Account deletion is admin-only at `POST /panel/api/v1/accounts/delete`. The numeric account ID must match
`confirm_user_id`; the React dialog asks the admin to type it. The last administrator cannot be deleted.
Under Hub -> accounts -> database locks, the server prepares copies of accounts, friends, mail, teams, rentals,
records and events. All those domains, duel-log removal, removal of the account's earlier audit rows, the deletion
audit, receipt and any disconnect intent commit together. Only then are the copies installed, surviving team/event
members notified and sessions ended. Every persistence error propagates; no cleanup is a warning-only success.
The deletion audit keeps the User ID plus non-secret request/outcome metadata, without old account values. Receipts
keep only numeric IDs/counts, the input fingerprint and a generic confirmation. Backups and exported logs may still
contain older data. Retrying after a lost response or restart returns the old result even if the account is gone;
a new account using its old name is never deleted or disconnected by that retry. The CLI's `delete-account --yes`
accepts `--request-id`, and the old HTML panel reuses its hidden form ID. Uncertain disconnects use the same durable
outcome rules as moderation. The React dialog retains the request across reloads and can resume it without loading
the deleted account. Storage failure blocks the destructive request instead of risking an untracked retry.

`GET /panel/api/v1/server` returns a checked live snapshot: version/run identity, saved account/player counts,
current online/duel counts, uptime and save/engine health. These process values are not stored state. The disposable
API truthfully disables stop because it does not control a game server.

`POST /panel/api/v1/server/stop` requires an admin, CSRF, retry ID and the exact `run_id` from status. Audit, receipt
and stop intent commit before the shutdown signal. The outcome means the signal was requested, not that shutdown's
final save/backup has finished. Failed outcome recording stays unknown. Startup labels unfinished stop/disconnect
intents interrupted without executing them. A committed retry returns its original result even on a new process;
an uncommitted request naming an old run is refused. Receipts still outlive audit retention. The dashboard retains
uncertain input/IDs across reloads, requires disconnect confirmation and blocks sending if storage fails. The CLI
prints a retry command with both IDs: `stop --request-id ID --run-id RUN`. Desktop's host client captures the run
identity before asking it to stop, rather than signalling whatever process might replace its endpoint.

`GET /panel/api/v1/audit` reads the stored audit log for current staff. It accepts `limit` (1–200), an opaque decimal
`before` cursor and a search of up to 80 characters (actor, source, action, target or request metadata). IDs stay
strings, including above JavaScript's integer precision limit. Server-side keyset pagination avoids downloading the
whole history. Before/after/detail are JSON formatted as text for inspection; invalid stored metadata or a failed
query returns an error, never a fabricated empty history. The UI distinguishes empty search, loading and stale
results after failed refresh, and supports older/newer/newest navigation. Audit retention/deletion still applies.

`GET /panel/api/v1/logs` reads at most 500 captured lines from this process, optionally warnings/errors only.
It reports the checked time and 2,000-line retention. Restarting clears these volatile logs; verbose traces stay in
diagnostic files. Existing credential filtering runs before capture. The UI supports manual/optional five-second
refresh, line limits and copying visible lines; raw text is escaped and keyboard-scrollable. Moderators can inspect
both histories as in the old panel. Neither view invents sample rows when the API or database fails.

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
| POST | `/panel/api/v1/sign-in` | Password step. An account with an authenticator gets `code_required` and a challenge (no cookie); one without gets a session limited to enrollment (or a normal one under the `optional` transition policy). Shared sign-in throttle |
| POST | `/panel/api/v1/sign-in/code` | Challenge + authenticator or recovery code: the staff session and HttpOnly cookie |
| POST | `/panel/api/v1/sign-out` | End the current staff cookie |
| POST | `/panel/api/v1/sign-out-everywhere` | End every staff session of this account, this one too |
| POST | `/panel/api/v1/mfa/enrollment` | A fresh authenticator seed (shown once, pending): secret, `otpauth://` link for the QR code |
| POST | `/panel/api/v1/mfa/enrollment/confirm` | A valid code from the pending seed activates it; 10 recovery codes, shown once; other sessions end |
| POST | `/panel/api/v1/mfa/step-up` | A code (or recovery code) starts a five-minute window for sensitive operations |
| POST | `/panel/api/v1/mfa/recovery-codes` | Replace all recovery codes (needs the recent check) |
| GET | `/panel/api/v1/audit` | Staff history, bounded keyset pagination, string IDs and server-side search |
| GET | `/panel/api/v1/logs` | Staff captured logs, bounded count, problem filter and checked time |
| GET | `/panel/api/v1/server` | Live checked version, run identity, counts and health |
| POST | `/panel/api/v1/server/stop` | Admin-only, explicitly confirmed, run-bound graceful stop |
| GET | `/panel/api/v1/accounts` | Safe account/player summaries without credentials |
| GET | `/panel/api/v1/accounts/{user_id}` | Staff-visible profile details and checked live state; credentials excluded |
| GET | `/panel/api/v1/countries` | Country codes from the game archive, or an explicit unavailable error |
| POST | `/panel/api/v1/accounts/delete` | Admin-only confirmed deletion; atomic cleanup and a persistent receipt, with current disconnect outcome |
| POST | `/panel/api/v1/accounts/edit` | Tagged `add_player`, `rename_player`, `set_country`; stable IDs and retained retry key |
| POST | `/panel/api/v1/give-bp` | Admin-only grant to `user_id`; exact result and receipt on retry |
| GET | `/panel/api/v1/bp-codes` | Staff-visible codes, rewards, limits and claims, with RFC 3339 times where known |
| POST | `/panel/api/v1/bp-codes` | Admin-only creation; omit `code` to generate one, `uses` for unlimited accounts |
| POST | `/panel/api/v1/bp-codes/remove` | Admin-only removal by code; previously credited BP stays with players |
| GET | `/panel/api/v1/reports` | Staff-visible player reports (`status` open, handled or dismissed; `limit`), newest first, with both players' account numbers and the open count |
| POST | `/panel/api/v1/reports/handle` | Admin/moderator: `handled` or `dismissed` with an optional note; audited; the same change again answers as done, another on a closed report is a conflict |
| GET | `/panel/api/v1/banlists` | The game's banlists and the server's default (`login-regulation`) |
| POST | `/panel/api/v1/accounts/banlist` | Admin/moderator: the banlist an account logs in with from its next login, or `null` for the server's; audited |
| GET | `/panel/api/v1/invites` | Staff-visible invites, remaining uses, expiry, issuer, claims and checked status |
| POST | `/panel/api/v1/invites` | Admin/moderator creation; omit `uses` for unlimited sign-ups, `days` for no expiry |

TCP requests check the loopback peer and local Host; Unix proxy requests check the configured Host and one forwarded hop. Posts also require same-origin evidence, and authenticated posts
require the CSRF header. Host-token, restore and credential-export operations are absent from the staff router. The
existing HTML panel remains on the local management listener, sharing sessions and the sign-in throttle.
Password/recovery changes, MFA enrollment and reset, bans, restrictions, role changes and sign-out everywhere invalidate
its cookies as well as JSON API cookies. The management listener does not mount the staff API or its schema; the dashboard
listener never falls through to host-token operations, the HTML panel, raw host backup routes, health details or game assets.

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

Open `http://127.0.0.1:5173/`. Use `TestAdmin` or `TestModerator`, password `DashboardDemo123!`.
These are example accounts in a fresh temporary directory; no existing data is opened. Ctrl+C stops the example and
removes its directory. Its dashboard API uses loopback port 18080; local management/assets use a separate, automatically
assigned loopback port printed at startup. Vite's proxy targets the dashboard port; set `YO3_DASHBOARD_API` before
`npm run dev` for a scratch server on another local port. Keep development ports private.

For a normal source server, opt in with `--dashboard-port 18081`, or `dashboard_port = 18081` in its TOML file.
Omitting it leaves the API disabled; `0` assigns a free port, reported at startup. `--bind` affects only game TLS,
even when set to `0.0.0.0`. A dashboard port conflict stops startup before database initialization/migration or live
CLI discovery, rather than silently dropping the requested service. All listeners stop with the server. These
TCP options support private development. For nginx use `--dashboard-socket PATH --dashboard-origin https://HOST`
(or the corresponding TOML settings), as the [container setup](docker.md) does. Socket and TCP modes are mutually
exclusive. Invalid configuration fails before storage opens; stale socket files recover under an exclusive socket
lock, while active sockets, symlinks and unrelated files are preserved.

From `dashboard/`, `npm run api:generate` regenerates Rust's spec and the frontend. `npm run api:check` checks client
drift without writing. From the root, `cargo nextest run -p yo3-server --test dashboard_contract` checks the schema
and the actual API without Node. Frontend gates are `npm run format:check`, `npm run lint`, `npm run typecheck`,
`npm test`, `npm run build` and `npm run test:e2e` (install Chromium once with
`npx playwright install chromium --only-shell`). Browser tests use private ports 5174 and 18181 and disposable state. The second-factor browser checks (`dashboard/e2e-mfa`,
`npm run test:e2e:mfa`) run the real default policy on ports 17843 and 18381 with the cheap test password hash; the main
browser checks run the dev server's `optional` policy so their password-only sign-ins stay valid.

## Separate authority and listeners

The public Compose option supersedes D7's tunnel-only product decision for the React dashboard. On 2026-11-17 the
user explicitly approved public VPS testing before MFA; since #43 every staff account needs an authenticator.
Password authentication, the authenticator, exact origin, CSRF, live staff permissions and throttling all apply. The old
HTML panel stays private.

| Listener | Access and authority |
| --- | --- |
| Game TLS | Public TCP 7443, existing authenticated game transport |
| Local management | Loopback, unpublished; host-token CLI API, assets and existing HTML panel |
| Dashboard API | Opt-in loopback TCP or mounted Unix socket; staff-session router only. Unix mode validates the exact configured web origin and one nginx forwarding hop; no host-token authority |
| nginx | Public HTTPS; dashboard files and an explicit allowlist of dashboard API routes on one browser origin |

Restore remains an offline CLI command with the server stopped and the data folder exclusively locked. Host-token
routes are absent from the dashboard router and nginx's allowlist. Test this on the dashboard listener with a real
host token and a loopback peer. Source address alone never grants local authority.

## Staff MFA and recovery

Implemented for #43. The code is `staff_mfa.rs` (TOTP, sealing, throttling), `panel.rs` (sessions, challenges) and
`dashboard.rs` (routes); the React screens are `dashboard/src/ConnectedMfa.tsx`.

**Sign-in.** Password first (the existing throttle), then, for an account with an authenticator, a six-digit code or a
recovery code. The password step issues no cookie: it returns a five-minute challenge held in server memory, bound to
the account's credential state (a password, role, standing or MFA change since cancels it). Five wrong codes (the doubling
wait usually stops them earlier) end the challenge. The session cookie is the same `HttpOnly; Secure; SameSite=Strict;
Path=/panel` cookie as before, and the CSRF, origin and `Idempotency-Key` checks apply to the code step too.

**Policy (`--staff-mfa`, `staff_mfa` in `server.toml`, `YO3_STAFF_MFA` in Compose).**

| Policy | Account with an authenticator | Account without one |
| --- | --- | --- |
| `required` (default) | password + code | a 20-minute session that can enroll, sign out and nothing else: every other route answers `mfa_enrollment_required` |
| `optional` (transition only) | password + code, always | a normal password-only session; sensitive operations are not step-up gated for it |

Use `optional` only while staff enroll, then remove it: a password-only session is what MFA exists to prevent. The
default is the safe one, so a server that never sets it requires MFA.

**Enrollment.** The dashboard shows a QR code (drawn in the browser from the `otpauth://` link, as inline SVG: no image
service, no external request) and the key for typing it in. The seed is 160 random bits, created by the server and kept
**sealed** (AES-256-GCM, authenticated with the account) in the account's login. It stays *pending*: no code works until
a valid code from it confirms it. Confirming activates it, makes 10 recovery codes and ends the account's other
sessions (the one that enrolled carries on). The enrollment-only session is the only unauthenticated-by-MFA path to
becoming enrolled, so an attacker holding a staff password could enroll first; the host should enroll new staff promptly
after `set-role`, and an unexpected `staff-mfa-enrolled` row in the audit log means `staff-mfa-reset` and a new password.
Re-enrolling an active authenticator needs the recent check.

**The key.** `data/staff-mfa.key`: 32 random bytes in hex, created on first use, readable by the host's user only (0600).
It is not in `server.db` or its backups, so a stolen backup does not reveal a seed. Back it up with the other secrets
(`server.key`). A key that is missing or changed makes every seed unreadable: the dashboard says the authenticator could
not be checked, and the host runs `staff-mfa-reset` for each account, who then enroll again. *Why not derive it from
`server.key`?* The TLS key may be rotated or supplied from elsewhere (`--auth-key`); an independent key keeps seeds
readable through that.

**Codes.** RFC 6238: HMAC-SHA1, 6 digits, 30-second steps (what every authenticator app does), computed with `ring`'s
HMAC (no extra crate) and checked against the RFC's test vectors. The step before and after the current one are accepted
for clock skew. **Replay:** a code is accepted for a step newer than the last accepted step of that account, so each step
works once. **Recovery codes** are 16 characters (80 bits, `ABCD-EFGH-JKLM-NPQR`, typed in any case), shown once at
enrollment and when regenerated, and kept only as SHA-256 digests; each works once, and regenerating replaces them all.
Using one is audited (`staff-recovery-code-used`).

**Throttling.** Every code attempt (sign-in, enrollment, step-up, recovery) takes from the address's bucket and then waits
the account's doubling backoff after a wrong code (1 s, up to the `max-backoff` setting; a success resets it): the same
`Throttle` as password sign-in, in its own state. nginx adds an outer `limit_req` on the password, code, enrollment and
step-up routes. A wrong code answers `invalid_code` (403) and a wait `rate_limited` (429, `Retry-After`).

**Recent MFA (step-up).** Signing in with a code, or `/mfa/step-up`, starts a five-minute window in that session. These
operations need it, and answer `reauthentication_required` (the dashboard opens a "Confirm it is you" dialog) without it:

- changing an account's role
- deleting an account
- issuing a password-account setup/reset code, and creating or revealing a key account
- changing any live setting, news or maintenance
- creating or downloading a backup (the download keeps its older rule too: a sign-in within five minutes for password-only
  `optional` accounts)
- stopping the server
- replacing the authenticator and regenerating recovery codes

Moderation, gifts, BP, invites and reads are not gated.

**Revocation.** Staff cookies are bound to the account's `staff_epoch` and credential, which move on a password change or
recovery, an MFA enrollment or reset, a role change, a ban or restriction, and **Settings → Two-step sign-in → Sign out
everywhere** (`/sign-out-everywhere`). Every request rechecks that the account is still staff and in good standing.
Sessions opened by the loopback HTML panel's form are not honoured by the staff API, and an enrollment-only session is
not honoured by the HTML panel; the HTML panel's form asks for the code too when the account has an authenticator.

**Local recovery.** `yo3-server staff-mfa-reset USER` (or `Op::StaffMfaReset` on the local management API with the host
token) removes the account's authenticator and recovery codes and ends its staff sessions; it is audited as
`staff-mfa-reset` by the host. It is not a route of the staff router or of nginx, and no staff role may run it, so
recovery never depends on the public side. Use it for a lost phone with no recovery codes; the person signs in with their
password and enrolls again. A forgotten *password* is the usual `reset-code`.

## Public security requirements

- Implemented (#43): staff MFA before a usable dashboard session; see [Staff MFA and recovery](#staff-mfa-and-recovery).
  Keep passwords, seeds, codes and recovery codes out of logs (none is logged).
- Implemented: explicit staff-session revocation after password recovery, MFA enrollment and reset, bans and
  restrictions, role changes and sign-out everywhere; live role/standing checks on every request. Existing panel cookies
  are covered, not only game sessions.
- Implemented: configured public host/origin and a mounted Unix socket held by the nginx container. nginx overwrites `X-Forwarded-For` with `$remote_addr`,
  sets Host and `X-Forwarded-Proto` explicitly, and clears incoming `Forwarded`. No client-supplied forwarding chain
  is accepted. Rust trusts only that single proxy hop. Rust throttles remain authoritative for sign-in, MFA and
  recovery attempts; nginx `limit_req` is an additional outer limit.
- certbot certificates and a renewal timer with nginx reload. Document initial issuance, renewal and recovery, and
  check expiry and renewal failures before a certificate can lock staff out.
- nginx owns HSTS and the static dashboard's CSP. Rust owns API security headers. Use shared header includes per
  nginx location with `always`; do not rely on inherited `add_header` directives surviving a location override.
- Explicit allowed API prefixes and a default 404, plus normal static dashboard routes. No proxy catch-all. Verify
  forged Host and forwarding headers through the real nginx configuration, not only a Rust request fixture.
- Implemented: public backup downloads are disabled by default (nginx answers 404, and the socket router refuses them
  too). The host can enable them with `--dashboard-public-backups`; they then need an admin, an authenticator check in the
  last five minutes (even under the `optional` policy) and an audit row. Never expose offline restore or the host token.
  Review session, CSRF, proxy, brute-force and MFA recovery boundaries before publishing a port (still open: the real VPS).

## Packaging and later work

Separate server/engine and dashboard/nginx images in Compose. Persistent SQLite data, read-only user-supplied game
assets, protected keys outside the dashboard, health checks and graceful shutdown. Rehearse backup and restore.
The Compose tasks are build, start, stop, status and logs (`task --list` shows the rest). [Deployment guide](docker.md) covers certificate bootstrapping,
renewal/reload, monitoring and offline restore. The game identity and SQLite volume never reach nginx.

Normalize domains gradually behind the API, with versioned migrations, constraints and tested restore. The first
operation needs no PostgreSQL/Turso migration, generic repository framework or new authentication provider.
