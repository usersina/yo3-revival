# Dashboard development

A standalone admin dashboard for YO3 Revival, built with React, TypeScript, Vite’s React SWC plugin, and Tailwind CSS.

The main pages are a **labelled design preview** with sample data. Changes live in memory until reload or
**Reset preview**. The role selector is not authentication.

The separate **[/connected](http://127.0.0.1:5173/connected)** page uses generated API hooks for real staff sign-in,
account details and player creation/rename/country edits, confirmed staff role changes, one-time password-account setup/reset codes, Give BP, registration invites (list/create, limits, expiry, issuer and claims) and BOOSTERPASS codes
(list, create, remove and claims). It never substitutes sample data for a
failed connection. This is local development work;
public hosting, the remaining management pages, MFA and Compose are still pending.

## Run

Use Node.js 22.12 or newer:

```sh
cd dashboard
npm ci
npm run dev
```

Open <http://127.0.0.1:5173>. The development server binds only to loopback and refuses to replace a busy port.

## Connect to a disposable server

From the repository root, run this in another terminal:

```sh
cargo run -p yo3-server --example dashboard_dev -- 18080
```

It creates fresh temporary data, identity and accounts, listens only on loopback, and needs no game files. It never
opens the normal server's `data/`. Open **Connect to a local server** in the preview, or visit `/connected`. Sign in
as `TestAdmin` or `TestModerator` with `DashboardDemo123!`. These are disposable example credentials, not production
accounts. Ctrl+C stops the example and removes its temporary directory.

The Vite development proxy targets `http://127.0.0.1:18080`. Set `YO3_DASHBOARD_API` before `npm run dev` to use a
scratch server on another local port. Use a real staff game account on that server. Keep development ports private.
The example prints a separate, automatically assigned local management port. The server's existing HTML panel
remains at `/panel/` on that port. A normal source server enables the JSON API with `--dashboard-port PORT` (or
`dashboard_port` in its TOML file); it is disabled by default and always binds to loopback, independently of `--bind`.

Give BP targets the stable numeric account ID. An unconfirmed grant or code change keeps its ID and payload in
per-staff browser session storage, survives a reload, and can be resumed safely. A retry never creates another grant,
mail, generated code or audit entry; a removal retry does not remove a replacement code. Mutation hooks do not retry
automatically. The session cookie is HttpOnly; CSRF tokens stay in memory and
passwords, cookies and tokens are not saved in the pending grant.

## Generate the API client

Rust endpoint annotations and DTOs in [dashboard.rs](../crates/yo3-server/src/dashboard.rs) are the source of truth.
From `dashboard/`:

```sh
npm run api:generate
npm run api:check
```

The first command runs the pure Rust `openapi` binary, writes `api/openapi.json`, then runs pinned Orval to generate
fetch functions, TanStack Query hooks, types and Zod validators. No running server, database or game is required.
The server exposes that same document at `/panel/openapi.json`. Commit the spec and generated files together with
the Rust change; never hand-edit `src/api/generated/`. The shared mutator is `src/api/transport.ts`.

`cargo nextest run -p yo3-server --test dashboard_contract` checks Rust/schema drift without Node. `npm run api:check`
uses Orval's non-writing outdated check. Both run before publishing a release, alongside dashboard checks.
Operation IDs are explicit; declared errors have stable codes; each generated mutation requires its request ID.

## Try the design preview

- Browse the overview, search and filter accounts, and open account details.
- Give an account BP, then find the change in the audit log.
- Create invites and BOOSTERPASS codes; copy a code or remove a reward code.
- Review and confirm a moderation action.
- Write and preview an announcement, or schedule and cancel maintenance.
- Edit live settings. Unsaved drafts survive navigation; save or discard explicitly.
- Filter and export sample logs. Add a sample backup entry; database downloads require a real server connection.
- Switch the preview role or resize to a phone viewport.

## Check

```sh
npm run format:check
npm run typecheck
npm test
npm run lint
npm run build
npx playwright install chromium --only-shell  # once, for browser tests
npm run test:e2e
```

`npm run lint:fix` removes unused imports and applies safe lint and Prettier fixes. Unused variables and parameters
are errors; prefix an intentionally unused parameter with `_`. `npm run format` formats source, styles, tests,
configuration and this guide. Generated files and bundled assets are excluded.

Open `dashboard/` as the folder in VS Code or Cursor to use its checked-in editor settings: Prettier formats on
save, and ESLint applies its fixes on save. The folder recommends the Prettier and ESLint extensions; install them
if your editor does not already have them. These settings do not change the Rust workspace’s formatter.

Browser tests use Vite on port 5174 and a disposable Rust server on 18181. Both refuse busy ports. They check navigation, account changes, dialog focus,
moderator controls, 320px/390px phone layouts, navigation scroll/focus, WCAG A/AA accessibility rules, real staff sign-in, Zod validation of actual responses, expired cookies, failed refreshes, duplicate-code refusals and lost grant/code responses followed by reload/retry. `npm run preview` serves the production build
locally on port 4173 for review. It is not a production web server.

## Boundaries

`src/data.ts` holds fixtures; `src/store.tsx` owns the in-memory preview. Reusable controls are in `src/components/`,
and each area has a page under `src/pages/`. Routing uses normal URLs, including account detail links.

The HTTP services use Axum on separate sockets. Existing HTML pages share their sign-in throttle and staff sessions
with the new JSON routes; the host-token API stays on the management socket and cannot be reached on the dashboard
socket, even with a valid token. Rust enforces staff roles and CSRF regardless of which controls
React displays. Password/recovery and actual role changes invalidate staff cookies. Role changes permanently
invalidate old cookies even if the role is later restored; the form warns before self-demotion. The public access gate still needs MFA enrollment/recovery, public-origin/proxy checks and the reviewed nginx configuration. See the
[accepted plan](../docs/dashboard.md).

The [launcher mark](../crates/yo3-desktop/assets/branding/README.md) and Barlow fonts are reused from the existing app.
The fonts’ license is included in [public/fonts/OFL.txt](public/fonts/OFL.txt). The minimatch override applies a
patched release to the development linter’s dependency; build, lint and interaction checks exercise it.

Setup/reset requests use a browser-generated code and retain it only in this tab for safe retry. The staff API
returns confirmation/availability, never the code. Done or confirmed expiry clears the retained code. Reload requires
a server confirmation before sharing. Passwords are chosen by the player through Desktop Forgot password or
Scripts `yo3-launcher --recover`; staff never choose or see them.
