# Dashboard UI preview

A standalone admin dashboard for YO3 Revival, built with React, TypeScript, Vite’s React SWC plugin, and Tailwind CSS.

This first slice is for reviewing the interface. **It uses sample data and never connects to a game server.** Changes
live in memory until you reload or choose **Reset preview**. The role selector previews administrator and moderator
controls; it is not authentication.

## Run

Use Node.js 22.12 or newer:

```sh
cd dashboard
npm ci
npm run dev
```

Open <http://127.0.0.1:5173>. The development server binds only to loopback and refuses to replace a busy port.

## Try it

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

Browser tests use a separate loopback server on port 5174. They check navigation, account changes, dialog focus,
moderator controls, 320px/390px phone layouts, navigation scroll/focus, and WCAG A/AA accessibility rules. `npm run preview` serves the production build
locally on port 4173 for review. It is not a production web server.

## Boundaries

`src/data.ts` holds fixtures; `src/store.tsx` owns the in-memory preview. Reusable controls are in `src/components/`,
and each area has a page under `src/pages/`. Routing uses normal URLs, including account detail links.

The Rust server and embedded panel are unchanged. Real sessions, permissions, API transport, the complete settings
registry, card/recipe gifts, and Docker packaging follow after the UI is approved. Production permissions will be
enforced by Rust, regardless of which controls the frontend shows.

The [launcher mark](../crates/yo3-desktop/assets/branding/README.md) and Barlow fonts are reused from the existing app.
The fonts’ license is included in [public/fonts/OFL.txt](public/fonts/OFL.txt). The minimatch override applies a
patched release to the development linter’s dependency; build, lint and interaction checks exercise it.
