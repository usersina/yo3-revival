# Roadmap

Goal: play Yu-Gi-Oh! ONLINE 3 with friends on our own server. Each milestone ends in something checked against the
real client (`tools/client/vclient.sh`), not just against tests. Tick boxes in the same commit as the work.

**Contents**

- [Roadmap](#roadmap)
  - [Status](#status)
  - [▶ Overnight plan (2026-11-07): everything left, end to end](#-overnight-plan-2026-11-07-everything-left-end-to-end)
  - [▶ Windows VM test (2026-11-08): what a real Windows client turned up](#-windows-vm-test-2026-11-08-what-a-real-windows-client-turned-up)
  - [▶ Robustness review (2026-11-08): what three code reviews found, and the fixes](#-robustness-review-2026-11-08-what-three-code-reviews-found-and-the-fixes)
  - [▶ Monkey testing (2026-11-08): two monkeys, what they covered and found](#-monkey-testing-2026-11-08-two-monkeys-what-they-covered-and-found)
  - [✅ Screenshot session (2026-11-08): two things seen playing NPC duels](#-screenshot-session-2026-11-08-two-things-seen-playing-npc-duels)
  - [▶ Duel check (2026-11-09): the client's view checked against the engine](#-duel-check-2026-11-09-the-clients-view-checked-against-the-engine)
  - [✅ M0: Research and tooling](#-m0-research-and-tooling)
  - [✅ M1: Stable world](#-m1-stable-world)
  - [✅ M2: An instrumented client](#-m2-an-instrumented-client)
  - [▶ M3: A social world](#-m3-a-social-world)
  - [M4: Accounts, collection and economy](#m4-accounts-collection-and-economy)
  - [✅ M5: Duels](#-m5-duels)
  - [▶ M6: Play with friends](#-m6-play-with-friends)
  - [Open questions and guesses (revisit)](#open-questions-and-guesses-revisit)
    - [Settled by the client's own code (no longer guesses)](#settled-by-the-clients-own-code-no-longer-guesses)
  - [Leads (outside the code)](#leads-outside-the-code)

## Status

Platform scope: **64-bit Windows 10/11** and **Linux with Wine**. **Windows 7 support is out of scope for now**,
for both Desktop and Scripts. Current compatibility and native Windows verification work targets Windows 10/11.

Launcher icon: the Duel Disk / Monster Reborn badge is embedded in the native window and Windows executable, and drawn
in the app itself (sidebar, Normal header, About).
Linux and Windows release builds pass; live X11 icon pixels and all nine embedded Windows icon sizes match the assets.
Desktop archives also carry `yo3-revival.ico` and `yo3-revival.png` for custom shortcuts, checked byte-for-byte in
both platform archives. The in-game login news no longer includes the deck-onboarding notice; a front contract
checks its removal alongside the existing news-format and engine-warning coverage.

v0.0.12 release preparation (2026-11-13):

- [x] The workspace package version and the local lockfile entries are `0.0.12`. The changelog describes Edit Deck's
  saves reaching the server, the Official tab, and the fixes for the janken stall, the game closing when the host goes
  away, an opponent who never connects, an unknown NPC and each server's own local decks. All workspace tests pass;
  workspace Clippy is clean; documentation links pass.
- [x] Windows 11 VM (build 22000, the Desktop bundle built from this commit, private writable game copy): the game
  started with every new hook's byte check passing; Edit Deck's overwrite save reached the server (`0x1F4F`,
  `update_type` 1, no `duel_id`); Load Deck's Official tab listed the nine character decks and CONCEALERS DECK; the
  first server claimed the original Documents folder (`revival-server.txt`, the existing decks kept); an NPC duel
  against Kaiba ran a summon, then the server was killed mid-duel: "You have been disconnected from the Duel Server.",
  then "…from the server.", then the title screen, with no crash (no Application Error event) and Desktop reporting
  the stopped local connection. No process was left over.

v0.0.11 release preparation (2026-11-13):

- [x] The workspace package version and the local lockfile entries are `0.0.11`. The changelog describes NPC duels
  hiding the NPC's face-down cards. All workspace tests pass; workspace Clippy is clean; documentation links pass. The
  change was checked live against Pegasus with the duel check (see the v0.0.10 section's fixed item below).

v0.0.10 release preparation (2026-11-13):

- [x] The workspace package version and the local lockfile entries are `0.0.10`. The changelog describes the native
  rules engine (no Node.js), the duel bot, the NPCs' characters, the heroes' Heart of the Cards and the NPCs' lines.
- [x] All workspace tests pass; workspace Clippy is clean; documentation links pass; the packaged-update test passes
  on the Linux Desktop archive.
- [x] Linux, from the v0.0.10 bundles: the Scripts server run by its path from another directory (found its engine
  after the fix below); a 3000 LP NPC duel against Pegasus with the duel check, 18 turns, no error, his lines (hand
  reads, a Set named, a heavy hit), Threatening Roar and a negated attack from the bot; its 563 engine calls replayed
  exactly with the bundle's engine; an Auto Duel between two accounts (Swords of Concealing Light, Sets, phases), both
  views in step; Desktop starts and checks.
- [x] Windows 11 VM, from the Desktop zip: unpacked into a scratch folder with its own app data, "Ready to play"
  (the engine check loads its card data), Play hosted locally and opened the game; an NPC duel against Pegasus with
  the Yusei deck (Max Warrior's battle effect, his Toons, his hand read in the chat); closing the game and the app left
  no process, listener or error line.
- [x] The first release run failed its tests before publishing: Cargo 1.100 (the workflow's stable Rust) runs
  integration tests from `target/debug/build/<package>/<hash>/out/`, and the server's tests looked for the engine
  program only one directory above (`target/debug/deps/` until 1.99). It now looks up to four directories up; the
  workspace tests and Clippy (`-D warnings`) pass with Rust 1.100 as well as 1.98. CI runs the tests only for a new
  version, which is why the native engine's first CI test run was the release's.
- [x] Found on the way (fixed after the release, 2026-11-13): in NPC duels the client was sent the NPC's cards unhidden,
  so the attack-target window and the card panel showed his face-down monsters. NPC duels now hide them as player
  duels do (`duel::npc_translator`; the duel check learns their rows from a translator that hides nothing). Live: a
  card back with no stats in both, the card itself once flipped in battle, his cut-ins unchanged.
- [x] Not reproduced, watched. Found on the way, not new in this release: a monster's switch to Defense Position at the end of the Battle Phase (Goblin Attack Force) stays drawn upright on
  its owner's screen, though the duel check agrees with the engine. Not reproduced on 2026-11-13 in five tries (an NPC
  duel with `--duel-deck 307`, and an Auto Duel through the opponent's turn, a discard window right after the switch,
  a manual switch back to Attack and a second Goblin): the switch showed sideways every time. The v0.0.10 case came
  after a battle with Pegasus' Toons and a Lei Lei trade; the trigger is still unknown. A sixth try the same day, with
  Kaiba's Call of the Haunted going off at the end of the Battle Phase, also showed it sideways, and the UI probe read
  the client's own card going from `up ATK` to `up DEF` (a pending switch stays upright until the phase really ends).

v0.0.9 release preparation (2026-11-12):

- [x] The workspace package version and the local lockfile entries are `0.0.9`. The changelog describes the
  presentation from Konami's logs, the card check and sweeps with their fixes, choices returned to the player
  (two-effect cards, unlabeled options, sums), replays with every card, the stricter deck check, and the pinned
  engine data (the release workflow now checks the bundle's own engine).
- [x] All workspace tests pass; workspace Clippy is clean; documentation links pass. The full card sweep (4,684
  cards) and the real-deck sweep (2,270 duels) pass; the presentation commands, the sum window and cancelling were
  checked live in the client on Linux.

v0.0.8 release preparation (2026-11-12):

- [x] The workspace package version and the local lockfile entries are `0.0.8`. The changelog describes the new
  Desktop look, live BP and the gift D-Mail, BOOSTERPASS codes in the app, coloured logs and duel logs in SQLite.
- [x] All 776 workspace tests pass; workspace Clippy is clean on Linux and the Windows target; documentation links
  pass. The v0.0.8 Desktop bundle passed natively on Windows 11 (every page, Play to the map, clean close) and the
  earlier full retest covered Linux Scripts/Desktop and a native Windows duel with a mid-duel BP gift.

v0.0.7 release preparation (2026-11-12):

- [x] The workspace package version and all seven local lockfile entries are `0.0.7`. The changelog describes the
  new Play flow, recovery, startup fixes and GitHub bug reports, with Desktop/Scripts update links.
- [x] All 761 workspace tests pass, including 240 self-played duels and the original Yusei-deck checks. Strict
  workspace Clippy, Python helper checks and documentation links pass. DLL/injector and automatic-loader checks
  pass in an isolated Wine prefix on a private display.
- [x] Release CI exposed a timeout fixture that assumed Node could finish cold startup within 500 ms. Injecting
  a one-second Node delay reproduced the missing PID file. A minimal waiting process now records its PID before
  `exec`, preserving the timeout and reaping assertions; 20 repeated runs pass with the delayed Node on PATH.
- The release workflow builds all four bundles and requires the packaged Desktop activation check before publishing.
  Real GitHub publishing and native Windows verification of the new Desktop UI were left as player checks; both were
  done later (releases publish from CI; the v0.0.8 Desktop bundle passed natively on Windows 11, above).

Desktop UI refinement (2026-11-12):

- [x] Both modes use one dynamic **Play** button with checking/starting progress and contextual retry. Choosing a
  game folder starts verification automatically; pasted edits wait briefly. Verification leaves preferences and direct
  launch unchanged. Successful checks save one at a time; stale results cannot approve a newer folder. Play rechecks
  critical requirements, and imported invitations automatically verify the server, compatibility and account.
- [x] Field errors appear beside the affected input, with consistent colors for progress, success, required actions
  and failures. Contextual guidance covers play, joining, hosting, activity and updates. **Help & support** groups
  log export and recovery; Add/Edit server group connection actions, dialogs retain visible confirmations, and
  unchanged forms hide Save controls. Notices retain a short title and visible cause, including filenames, failure
  stages and available native codes from the error chain. Background checks preserve the original launch failure.
- [x] Update notes render formatted Markdown in a bounded scroll area, with restart warnings and confirmation buttons
  always visible. A geometry regression guards table layout against infinite bounds. Real release notes, long notes,
  tables and link errors each remain usable through 60 scroll actions at 900×640 on a private display.
- [x] Direct launch uses a checkbox with contextual repair, installed-state feedback and prerequisite explanations.
  Removal remains possible after failed validation or a removed connection; enabling requires verified settings and
  a running local server or saved remote connection. Normal/Advanced stay text-only with hover help and fit the
  sidebar. Busy server/account selectors show their saved values without offering ignored interactions.
- [x] Missing required files appear together as exact paths, with a short conditional antivirus/exclusion hint.
  A direct Desktop download action replaces the recovery dialog. Error notices show their cause/code directly, with
  a copy icon for the full report and no details expander. Direct-launch repair states which
  loader/configuration it restores.
  Missing-path collection runs in background checks, ignores stale results and distinguishes absent files from
  existing invalid files. Copied errors and exported diagnostics preserve the complete list and original failure.
  Local Play/hosting recheck the server checksum, packaged Node, engine anchors and bounded sidecar startup even
  when hosting is already live (since v0.0.10: the `yo3-engine` program and its data files, with a bounded start). Four focused Desktop backend checks and 11 engine checks pass, including deleted
  imports, multiple missing files, non-regular files and timeout cleanup. Individual card scripts are not exhaustively
  verified, and remote hosts' later file changes are not monitored by Desktop.
- [x] Normal logs use a compact panel, group consecutive repeated messages and hide Wine `fixme` output with a count.
  Errors, native codes and filenames remain visible. Advanced Activity retains technical output. Focused tests verify
  full errors survive dismissal and ZIP export, including output filtered from the display; following and paused views
  remain covered. Both modes were inspected with a sample crash and recorded diagnostic lines.
- [x] Short logs and release notes fit their contents; Normal's footer and connection-editor actions follow the
  content. Removed empty feedback/save-row gaps, centred mode labels and tightened About spacing. Inspected 21
  private-display states, including minimum-size forms, dialogs, errors and empty/long logs. Native scrolling checks
  confirm that scrolling up stops following, returning to the bottom resumes it, and error-page footers remain reachable.
- [x] Twenty-nine new regression checks bring focused UI coverage to 67 checks: 60 flow checks, six error presentation
  checks and one Markdown geometry check. The three direct-launch regressions pass; nine UI states were inspected
  at 900×640. Four additional missing-file cache regressions pass; actual two-file deletion lists and direct download
  controls were inspected in both modes at the same size. Repeat selection of the same invalid folder remains covered. Eight error
  cases were inspected at 900×640; actual missing-helper failures are visible in both modes. Desktop lint and
  Linux/Windows builds pass; private-display checks also cover automatic verification, forms/dialogs, account creation
  and shutdown. Rendering a native crash error does not establish a crash fix;
  native Windows GUI verification of this refinement was open then; the v0.0.8 Desktop bundle later passed natively on
  Windows 11 (every page, Play to the map, clean close).

Local game-data recovery (2026-11-12):

- [x] Both modes offer **Help & support → Reset local game data… → Back up & reset** with a reviewed local
  folder and explicit scope. The default moves only `options.y3o` and cached stock logins into a dated backup,
  preserving the entire player tree, including local decks, `.y3p` selection and `revival-starter.initialized`.
  An explicit full reset moves the original client folder to `backup/data`; server accounts/cards/BP and Revival
  connections stay separate. Completion keeps an **Open backup folder** action, with restoration steps in the guide.
- [x] The actual Windows Documents location is resolved through the same API as the client, including redirects.
  Linux queries that API in the selected Wine prefix using a read-only injector flag. Running games, stale
  confirmations, protected folder overlaps and symlink/reparse entries are refused. Partial failures roll back;
  a failed rollback retains the backup and reports its recovery path without overwriting newly appeared files.
- [x] Focused TDD: 16 Linux backend/platform checks, eight UI flow checks and 13 native Windows filesystem checks
  pass. Native discovery and running-game refusal pass with a disposable process. Private Wine GUI checks complete
  both reset scopes and restore their backups with identical player-file hashes; a simulated external game produces
  an actionable visible error with no changed files. Linux/Windows release builds, focused Desktop Clippy and doc
  links pass. Real user Documents were not reset; native Windows GUI confirmation/reset remains a separate check.

Windows startup investigation (2026-11-12):

- [x] Reproduced intermittent authentication resets on native Windows 11 with unmodified v0.0.6: eight explicit
  socket-10054 failures in 40 attempts, separate from the reported native game crash. The client now accepts a fully
  validated reply despite a later close-alert failure; the server gives the peer a bounded opportunity to close.
  Truncation, credential rejection and certificate/compatibility checks remain enforced. Focused TLS regressions
  went red/green. The patched client passed 60/60 against the original server; the original client passed 60/60
  against the patched server. Combined checks passed 199/200, including a clean repeat of 100; one discovery
  handshake hit the unchanged 10-second deadline, with no socket reset. The final rebuilt binaries passed another
  50/50. The cause of that timeout remains open; new diagnostics identify the stalled connection/response stage.
  Native regressions pass: 50 repeated framing/rejection checks, two deterministic cleanup-failure cases and three
  timeout-stage cases. Successful-reply fixtures use orderly TLS closure because immediate TCP resets can discard
  unread response bytes on Windows; cleanup failure is injected after validation in the focused regression.
- [x] Desktop startup timeout now stops/reaps its owned unfinished child, preserves diagnostics and allows retry.
  Stop waits for the captured server endpoint to close and any owned child to exit, including servers attached
  after reopening Desktop. Real child/manager regressions cover retry and the acknowledgement-before-exit race.
  Native closed-port/attached regressions and nine actual starts pass, including retry after timeout. Windows takes
  about two seconds to report loopback refusal, so shutdown probes allow up to three seconds within the remaining
  deadline. The deliberate startup timeout took 13 seconds including blocking status probes; that polling limit
  is not a strict wall-clock deadline. All private test processes and listeners were stopped afterward.
- [ ] The Discord recording's native game exit `0xc0000005` is a separate failure. No faulting address is present
  in the submitted logs. Repeated authentication success does not establish that this access violation is fixed;
  seven fresh native Windows 11 launches with the patched binaries reached the main map and exited normally, each
  monitored through exit by a temporary debugger with unique capture files. A crash dump/faulting address from a
  failing machine is still needed. Debugger attachment can change timing, so these runs do not rule out the crash.

v0.0.6 release preparation (2026-11-11):

- [x] The shared workspace package version and all seven local lockfile entries are `0.0.6`; external dependencies
  are unchanged. Release notes include the log viewer fixes below and the package-version alignment.

Desktop log viewer (2026-11-11):

- [x] Normal and Advanced logs fill their panels, placing the scrollbar on the right. Views open at the newest line
  and follow output only at the bottom; scrolling up preserves the older viewport and returning down resumes following.
  Unchanged polls no longer force scrolling. Activity's explicit Pause control remains independent of following.
- [x] Four regressions cover unchanged polls, following in both modes during startup, pause/resume and reopening or
  changing the log view. All 594 workspace tests pass (one packaged-window release gate remains skipped), strict
  Clippy and documentation links pass. Private Xvfb checks confirm both modes open/follow at the bottom, keep the same
  older rows visible during appends and resume following after scrolling down; their scrollbars sit at the right edge.

v0.0.5 release preparation (2026-11-11):

- [x] Release notes cover Normal/Advanced mode, Desktop/Scripts editions, Windows startup repairs, protected-monster
  fixes and automatic diagnostic capture with optional personal filtering at export. Setup and support guides use
  the current UI. Dated entries below retain the evidence and limitations from their original checks.
- [x] All seven crates inherit shared versions, package metadata, dependencies and lints from the root workspace.
  Linux and Windows resolved dependency/feature graphs match the pre-refactor graphs exactly; `Cargo.lock` is
  unchanged. Four test fixtures were made portable after native Windows failures, preserving their assertions.
- [x] Current verification: 590 workspace tests pass, including the 240-duel check; the one packaged-window test
  stays reserved for the release gate. Strict Clippy passes for Linux and Windows, both release builds pass,
  DLL/loader harnesses pass under Wine, and documentation/helper/edition checks pass.
- [x] Native Windows 11: 111 tests across 16 test binaries pass, including all 13 engine contracts with bundled
  dependencies. The actual Desktop validates the game, starts its isolated server, confirms DLL readiness and
  authenticates through player selection to the main map. Linux Desktop/Wine also reaches the main map.
  Test processes were stopped and the VM's original game Documents restored with matching hashes.
- [x] The first release attempt passed all 590 tests and lints, then failed linking a synthetic mod fixture:
  CI's MinGW runtime exceeded its fixed 12 KiB code reservation. The hook section now follows the actual aligned
  code end while preserving guarded RVAs and the callback relocation. A regression reproduces the failure with
  larger code at both image bases, checks contiguous PE sections, and rejects code overlapping the first hook.
  Both larger fixtures pass the real DLL contract under Wine; all four archives build and the packaged Desktop
  updater passes locally. Release CI runs the layout check before bundling; no production DLL or game-hook behavior changes.
- [x] The next CI attempt built all four archives, then its real-window gate caught missing X11 runtime libraries.
  An isolated Ubuntu 26.04 reproduction failed loading Xcursor/XInput. Installing the complete Iced X11 runtime
  set makes the same packaged-updater test pass; CI and the Linux guides now list those dependencies explicitly.
- [x] Published [v0.0.5](https://github.com/usersina/yo3-revival/releases/tag/v0.0.5) from `7007ccd` after the
  [release workflow](https://github.com/usersina/yo3-revival/actions/runs/43074266729) passed all 590 tests, lints,
  PE layout checks, four bundles and the actual packaged-updater startup gate. All four uploaded asset digests
  match the release-note SHA-256 checksums; the latest-release endpoint returns v0.0.5 and its tag names that commit.
- [ ] Native Windows 10, protected-folder direct-start installation, runtime reinstallation and separate-machine
  network checks remain open; Windows 11 evidence does not close those items.

Startup feedback (2026-11-11):

- [x] Automatic bounded diagnostics replace the capture checkbox: normal game launches retain client stdout/stderr,
      mod messages and decoded nested server packets; engine stderr also reaches retained diagnostics in Scripts mode.
      Full exports are the default, with optional best-effort personal filtering applied only to the ZIP. Credentials
      remain scrubbed in every mode. Ordinary viewers hide verbose payloads; advanced searches and full exports retain
      them. Expensive duel validation/replay stays under `--duel-check`. Direct-start game stdout and a remote host's
      server logs still require separate collection; these limits are explicit in the support guide.
- [x] Normal hosting refuses missing/corrupt game archives and empty card/deck data before player-storage migration
      or accepting players. Only explicit in-process protocol fixtures retain the empty-world path. CLI regressions
      preserve existing files and the server key. Live A/B: absent data refuses startup; the complete installation gives
      the same scratch account 218 distinct cards, 498 total. This addresses a reproduced path relevant to
      [#4](https://github.com/usersina/yo3-revival/issues/4); the reporter's exact cause remains unconfirmed.
- [x] Desktop validation and script startup check the supported v1.153 executable's four existing hook signatures.
      A separate 32-bit runtime probe uses the game's VC90 manifest and checks DirectX/VC90 dependencies without
      invoking the game or its WINMM loader. Wine regressions cover missing runtimes, guarded hooks, Unicode paths,
      both manifest/no-manifest fixtures and direct start. The direct fixture includes synthetic guarded hook sites.
- [x] Native startup failures retain their stage, real Win32 error where available and child exit status. Plain
      repair instructions lead; details/export retain the diagnostics. Bounded stderr discards cut first lines before
      credential redaction, protecting even truncated account-key suffixes. Executable fingerprint and mod version/ABI
      identify the components being tested.
- [x] Private Desktop checks: missing DirectX fails validation, disables Start and creates no player storage;
      valid setup starts its scratch server, reaches the real game's main screen and opens the deck editor with the
      selected starter and all 498 cards. The Windows direct-start helper also passes under Wine.
- [x] 582 workspace tests pass at bounded concurrency (one archive/window-dependent update test is skipped),
      including the 240-duel check; strict Clippy, Linux/Windows release builds, native/Wine mod and loader harnesses,
      documentation links and offline-edition tests pass. An engine-identity timeout during concurrent builds did
      not repeat in either bounded full-suite run. The Windows CLI also passes its runtime probe under Wine.
- [x] Release CI explicitly runs the packaged Desktop updater test after bundling and before publishing, on owned
      Xvfb with fresh application storage. The shared `tools/test-packaged-update.sh` helper fails if no test matches.
      Local proof: the current unpublished `v0.0.5` archive opened and activated successfully; a checksum-consistent
      fixture whose launcher exits immediately failed the same command and remained inactive. No updater version
      checks were relaxed; native Windows testing remains separate. CI setup, lints/tests and packaging run only
      before publishing a new version; other pushes stop after the changelog check.
- [ ] Native Windows 10, installing direct start in protected folders and reinstalling missing runtimes still need
      platform evidence. Windows 11 standard-user checks below used a writable private game copy, with Defender active;
      they do not establish reputation with every security product. The app does not change antivirus/firewall policy.
- [x] Native Windows 11 build 22000.2538: reproduced the supported game's runtime check passing with a normal
      drive path but failing with Rust's canonical extended-path prefix. Compatible native path handling now covers
      runtime probes, injection and direct-loader configuration. The Windows path regression went red/green with
      spaces/Japanese text, and the actual Desktop validation passed; Linux workspace tests and strict Clippy pass.
- [x] Native Windows ASLR: observed the supported game at `0x006D0000`; the old DLL rejected it. Code, data,
      vtables and the absolute duel-update callback now use the actual image base, retaining byte guards. Linked
      full-size/relocated fixtures are valid Windows PE images (the old section-gap fixture was accepted only by Wine).
      The same native fixture failed with the old DLL and passed with the fix at randomized bases; Wine's forced
      `0x600000` fixture also passes. The native client reaches login/map/deck editing with 498 cards. Standard-user
      game launch is still being checked separately; no ASLR/security policy was disabled.
- [x] Native Windows standard-user startup: reproduced the original game's UAC request followed by injection
      error 5. Scoped `RunAsInvoker` on the injector/game tree fixes normal Desktop Start without elevating the app or
      server. The Windows regression went red/green; live token checks show every component non-elevated. A fresh
      player received its starter automatically and entered matchmaking before ever opening Deck Edit.
- [x] Native Windows diagnostic I/O: a second duel's dependency fingerprint hit the 30-second engine-call deadline
      during archive extraction. Separate native probes measured 61.4 seconds and 7.5 seconds for valid identities.
      Fingerprinting now has its own bounded two-minute budget; a delayed-fixture regression went red/green and still
      proves that stalled duel steps retain their shorter deadline. Recording errors continue to fail the report without
      being mislabeled as a proven engine/model mismatch.
- [x] Native Windows 11 completion: Normal Start authenticates and prepares a fresh starter; Scripts wrappers host
      with their bundled engine and launch the real client. A complete NPC duel ended with a battle win, all five native
      checkpoints matching, no checker errors and all 59 recorded calls replaying exactly. The earlier PvP recording's
      31 calls also replay exactly. Four native engine contracts cover opening, complete self-play, battle move reasons
      and field stats; native Rust identity checks pass for absolute and relative engine paths.
- [x] The rebuilt Windows package passed native updater activation; the deliberately broken update remained
      inactive. Both archive component inventories verify. Native current/all-retained exports contain 7/21 entries,
      pass ZIP integrity and credential/home-path checks. A missing Desktop DLL disables Start with repair guidance;
      restoring it permits revalidation. Linux workspace: 583 tests, strict Clippy and docs checks pass.
- [x] Initial Windows window sizing: the old 1080×760 window hid its footer on the 1280×800 VM. A centered
      1000×640 default now keeps Version, GitHub and Help visible without maximizing; native DWM bounds fit the
      1280×752 work area. Desktop state tests and strict Clippy pass; Iced's own sizing/position controls are used.

v0.0.4 release preparation (2026-11-10):

- [x] README and illustrated launcher guide lead with release downloads and the current four-page UI. Fresh Windows
      screenshots under Wine show v0.0.4, automatic starter setup, settings, independent hosting and diagnostic export.
      Obsolete browser-join/login-field pictures are removed; all four GUI images ship in both application bundles.
- [x] The old-release upgrade path explains complete backups, selecting the existing data folder before hosting,
      transactional SQLite import, preserved accounts/credentials, private invitations and the single public TLS port.
      CLI instructions, issue-help links and report forms point to the appropriate GUI guidance.
- [x] 523 workspace tests, strict Clippy, four headless-helper tests and documentation checks pass. Both archives build;
      component hashes, archive integrity and every bundled guide link/image are checked. The Windows GUI under Wine
      started its own scratch server and launched the real game; a fresh player received a selected 40-card starter.
      Close game kept hosting alive. Native Windows UAC/protected-folder and separate-machine checks remain open.
- [x] Published [v0.0.4](https://github.com/usersina/yo3-revival/releases/tag/v0.0.4) from `d0f482f` after approval;
      the [release workflow](https://github.com/usersina/yo3-revival/actions/runs/42868619870) passed. Downloaded both
      public archives and verified release-note SHA-256 checksums, integrity, component manifests, guide links and
      screenshots. The latest-release endpoint returns v0.0.4; its tag still points to the verified release commit.

Launcher scope and consistency (2026-11-10):

- [x] Four pages: Play, My server, Activity and Settings. Server selection/Add server share one section; connection
      editing is inline. Headings/dividers replace nested panels; hosting status, logs and modals retain useful boundaries.
      Native help tooltips explain accounts, keys, sharing, interfaces, game installation and direct launch. Wine prefix
      controls appear only on Linux; private :90 checked Linux at 900×650 and the actual Windows build under Wine.
- [x] Hosting accounts/invitations are separate from the Play account. Per-section Save/Cancel cannot apply unrelated
      drafts; Start uses saved configuration. Live: an unsaved port change did not affect the listener, and creating,
      selecting and exporting UXGuest left Play on UXHost. Advanced hosting Save/Cancel stay visible while scrolling.
- [x] Complete game/component readiness, first-setup Continue, actionable setup help and visible direct-launch state.
      Missing/updated owned loaders can be repaired without replacing foreign mods. Bundled helpers must exist and match
      their release manifest; no developer fallback for a packaged direct helper. Regression tests first reproduced that
      fallback. Live: missing loader repair, changed-DLL fail-fast, repaired original-executable login and managed Play
      reached the game main screen; cleanup confirmed unchanged executable/INI hashes.
- [x] Saved connection discovery/trust/test/edit/removal pass live. Loopback/listener addresses are not offered to
      friends; local play remains available. Local process status does not claim remote reachability. Firewall automation
      and manual certificate controls are removed from the GUI; OS guidance and automatic identity maintenance remain.
- [x] One current/all-retained log-export dialog with scope/size/date preview. The final Windows GUI exported a private
      invitation and both ZIP scopes through native dialogs; ZIP integrity, different inventories, credential exclusions
      and home-path redaction passed. Hosting survived manager exit/reopen and Windows GUI reattachment. The final
      Windows bundle also started its own server and launched the real game to main as UXHost; Close game kept hosting.
- [x] 518 workspace tests (including 240 duels), desktop flow regressions, Clippy, workspace check, four headless-helper
      tests and documentation checks pass. Linux/Windows bundles build; actual staged-window activation succeeds on both
      Linux and Windows under Wine. Local test tags are not published releases.
- [ ] Native Windows protected-folder/UAC behavior, friends on separate machines and the first published GUI-to-GUI
      automatic update still need platform/release verification. Firewall mutation is outside the launcher's scope.

Launcher refinement and updates (2026-11-10, `feat/launcher-ux-updates`):

- [x] Retain Iced with standard widgets, Lucide SVG icons/notices, grouped primary actions and collapsed advanced
      options. Private display :89 verified the normal layout and 900×650 minimum; the overview screenshot is refreshed.
      Navigation stays usable while jobs run; modal guards and account/connection refreshes preserve unfinished input.
- [x] Visible version/build, GitHub link and asynchronous startup/manual release checks. Stable semantic versions,
      platform assets, bounded metadata and HTTPS origins are checked; development builds are never auto-downgraded.
      Live GitHub lookup reports v0.0.3; no test release was published. Offline/rate-limit/slow/oversized responses have tests.
- [x] Explicit review → Download & restart, cancellable streaming progress and SHA-256 verification. Complete bundles
      are unpacked into private version folders with path/link/size checks and component manifests; original files and
      user data stay in place. SQLite records the active/previous version only after the new window acknowledges readiness.
      Failed, missing or mismatched readiness cannot activate; verified installs can be retried and shortcuts route forward.
- [x] Both local `v0.1.1+local` review bundles pass actual packaged window/activation checks on private :89 (Linux and
      Windows under Wine). These are local test versions, not published releases. Managed Play reached the real game's main
      screen as UXYugi and Close game ended its authenticated session. One cold Wine start under build load timed out before
      mod readiness; retry reached main, and the failure remained visible in Activity. Default displays/data were untouched.
- [x] Workspace check/Clippy, the full contract/240-duel suite, focused updater/UI regressions, four headless-helper
      tests and documentation links pass. Archive tests cover changed hashes, traversal, Windows aliases, links, retries,
      original data preservation and failed startup. Both bundles include the icon and dependency license notices.
- [ ] Native Windows update/UAC behavior and a future published GUI release's full GitHub download still need
      platform/release verification. Live release discovery, bounded download fixtures and full local bundle activation
      are covered separately; Wine does not establish native Windows firewall behavior.

Launcher usability phase (2026-11-10, implemented on `feat/desktop-launcher`):

- [x] Address-first discovery and remembered identities; existing invitation/profile credentials preserved. Private
      display :88 showed discovery and first-use identity confirmation by IP with no certificate file; metadata contracts
      cover expiry, key changes, same-key renewal, wrong names and bounded responses.
- [x] Shared build/protocol/DLL requirements, preflight and DLL readiness before game login. Static preflight rejects
      wrong/missing/changed files before authentication; injector readiness precedes account login and game resume.
      Live: private :88 game reached the main screen as UsabilityYugi after readiness interface 2 was acknowledged.
- [x] Reversible direct-start loader; `yo3.exe` and INI hashes stay unchanged. Private :88 reached the main screen
      from the original executable, and a missing DLL showed its expected path, accepted a matching file and required
      restart before login. Named/ordinal imports, Unicode paths, stale readiness and foreign-mod refusal are covered.
      Gate callback `0x4699B0` reads only host/port;
      login dispatcher `0x469C00` uses error codes and `0x46D050` selects fixed local UI strings. A custom requirements
      dialog/file picker needs local code; do not invent a server message field.
- [x] Advanced interface binding (new GUI setups default all IPv4), one public port, retained local-only upgrades and
      optional firewall elevation (removed in the scope refinement below). Live: selected eno1 listener and local game main screen on :88; key retained during
      automatic identity maintenance. Windows helper cross-checks and pure command/argument tests pass; native UAC and
      real firewall mutation remain untested, so the user's host policy stays untouched.
- [x] Session-indexed diagnostic logs, bounded retention and redacted current/all-retained ZIP exports. Private
      checks verified continued server logging after manager exit, reattachment, different current/all inventories and
      valid ZIPs without saved account keys, private keys or home paths. Wine helpers use a cross-platform heartbeat.
- [x] 493 Rust tests and four headless-helper tests pass; workspace check/Clippy and documentation checks are clean.
      Both review bundles build, their component hashes match, and game files/credentials are absent. The packaged
      launcher reached the real client's main screen on private :88; offline-server and missing-DLL paths blocked login.
      The Windows GUI exported both current and retained-history ZIPs through its native file picker under Wine; CRCs,
      exclusions and redaction passed. Loader removal restored the original game folder's executable/INI hashes.
- [ ] Native Windows UAC/protected-folder installation and separate friends' machines
      still need platform/user-driven verification. Firewall mutation is no longer an application feature. Wine and cross-compilation do not establish those results.

Next DLL candidates, after launcher usability:

- [x] Skip the unused original login form: **ONLINE** opens player selection through the same native connection
      and reply handlers. The mod guards the one-shot `StartLogin` call at `0x46964B`, keeps saved fields untouched,
      and leaves the original executable/Flash files unchanged. The faster flow exposed premature local gate EOF
      (null logger at `0x535210`); the proxy now retains that socket until client close or launcher shutdown.
      Shared C/32-bit Wine contracts and a proxy lifetime regression protect this path. Fresh Documents data reached
      player selection, Back/re-entry and the main screen on private Xvfb without a form click; starter setup still works.
      Direct launch of the original executable also reached the main screen using the installed loader and Windows
      helper under Wine; executable/INI hashes stayed unchanged. All 525 workspace tests and strict Clippy pass.
      Native Windows gameplay was verified later (Windows 11 native completion, above: a complete NPC duel).
- [x] Initialize a legal starting deck after player login only for a fresh local profile; preserve existing
      files and selections. The first trunk reply supplies legal candidates after collection/rentals; the required mod
      uses native validation, serialization and cache reload, then sets an empty default selection. Any existing deck
      or saved selection blocks setup, even an invalid one; a durable per-player marker prevents regeneration after manual deletion. Contract tests and
      the portable DLL policy protect ordering, ownership/banlists, repeat login and preservation. A fresh private Wine
      player reached a live NPC duel without opening Edit Deck; the selected starter had 40 cards.
      Live checks also preserve a custom selection and an unfinished deck, isolate the second player, and retain an
      intentional deletion after reconnect even with an empty selection. All 523 workspace tests pass; check/Clippy,
      DLL/loader contracts and Linux/Windows cross-builds pass. Native Windows gameplay was verified later (Windows 11,
      above: a fresh player received its starter and entered matchmaking).
- [x] Sync editor saves without requiring a duel selection (2026-11-13). The connection DLL (`tools/mod/deck-upload.h`)
      redirects the editor save's call to `0x415610` (`0x415A4D`, inside `0x4159B0`) and sends the record it makes as
      Select Deck does, `0x1F4F {update_type 1}` with the deck and no `duel_id`; the server keeps it unchecked and
      unanswered (the editor reloads its list on `0x22CA` and opens a dialog on `0x22CB`). Live on two headless clients
      of one account: New and Save → overwrite reached the server, and the other machine's Load Deck listed and loaded
      the deck, also after a server restart. A machine that already has a deck of that name keeps its own copy.
- [x] Diagnose the client black-screen wait when the server hangs, then add bounded failure feedback (2026-11-13). A
      dropped connection crashed the game: every failed receive logs through the client's network error logger
      (`0x82D214`), which this build never creates (`0xC0000005` at `0x535210`, live with the server killed mid-duel;
      the "black screen" was the crash). The connection DLL now fills it with a logger that writes nothing
      (`tools/mod/netlog.h`): live, "You have been disconnected from the Duel Server.", OK, "You have been
      disconnected from the server.", OK, the title screen. A host gone without closing (asleep, its network lost)
      now drops too: the launcher's connections use TCP keepalive (45 s, then 4 probes 10 s apart; not checked
      live, it needs a dead network path). A server process that hangs but keeps its connections open still leaves
      the duel waiting with no message (live with SIGSTOP); the client has no timeout of its own.
- [x] Local profile collisions for identically named players on different servers (2026-11-13). Local decks, `.y3p`
      and the starter marker live in `Documents/Yu-Gi-Oh! ONLINE 3/players/<name>/` (`tools/mod/starter.h`
      `starter_local_path`), and every server's players start as "Duelist". The client looks Documents up only with
      `SHGetFolderPathW/A(CSIDL_PERSONAL)` then appends the folder name (`0x52B93F` and its narrow twin); the DLL
      answers those per server (`tools/mod/documents.h`): the launcher passes `YO3_SERVER_ID` (16 hex digits of the
      host's key, `Endpoint::server_id`, kept across certificate renewals and address changes; the direct loader gets it
      in its ready file). The first server played on claims the original folder (`revival-server.txt`), others get
      `Documents/YO3 Revival/servers/<id>/` with `options.y3o` copied. Live (isolated prefix and Documents copy): server
      A claimed the original folder and kept its decks, server B got a new folder with only its starter deck, and A
      logged in again to the original. Desktop's reset and backup still act on the original folder only.

Desktop manager (2026-11-10): `yo3-desktop` supplies an Iced `yo3-revival` application with game-folder setup,
local play, saved remote connections, independent server controls, live account creation/add-player/export,
single-file private invitations, certificate updates and activity logs. Server state uses SQLite with a transactional
one-time JSON import; preferences and connections use a separate launcher database. Workspace tests cover
migration retry/preservation, live manager operations, certificate identity and game-control lifecycle.
On private Xvfb :88, the GUI started its server, created two accounts while running and launched the real client to
the main screen. The GUI also joined a second server as RemotePlayer while its own server stayed hosted. Keeping
hosting across manager exit/reopen passes after separating the server's process group. Unicode game and DLL paths
have injector regression checks. Windows cross-build and GUI checks under Wine succeed, including the embedded
font, native invitation picker/import, saved connection and TLS connection check. Native Windows GUI testing passed
later on Windows 11 (above); separate friends' machines remain open. The Linux bundle also starts its server with the
bundled engine (Node and the sidecar then, `yo3-engine` since v0.0.10) from outside the checkout; both platform
archives exclude private credentials, databases and game files.

Desktop bug reporting (2026-11-12):

- [x] One review dialog publishes an issue and linked text diagnostics through the registered GitHub App, with device
      sign-in, automatic version/system/error evidence, optional filtering and a visible public-sharing notice.
- [x] OS credential storage and token refresh; private SQLite receipts, restart recovery, cancellation and concurrent
      posting guards prevent blind duplicate uploads/issues. Oversized evidence fails before upload.
- [x] 83 focused regressions pass; Desktop Clippy is clean. Native Iced layout checks cover sign-in, review, progress,
      uncertain results and recovery at 900×640 and 1000×700. Linux and Windows release builds pass.
- [ ] Player verification of real GitHub sign-in and issue/Gist publishing, including native Windows credential storage.
      HTTP/vault checks used mocks; no player diagnostics were uploaded during development.

Headless display fix (2026-11-10): cold Xvfb startup releases captured pipes immediately, keeps diagnostics in
its instance log and reuses an existing display. Regression tests cover cold startup, reuse and failed startup;
they run in the release checks without an X server or game. Live: the UI monkey's capture path on private `:88`
returned in 1.02 s, then reused that display in 0.19 s, with no timeout.

Authenticated launcher milestone (2026-11-10): the server accepts game connections only over validated TLS. The
required DLL redirects the original client through private launcher endpoints without changing its binary or INI.
Two real clients on one IP logged in to different accounts, matched, reached the field and displayed the win/loss
results when the turn clock expired. Both downloaded their duel logs through TLS. Named/ordinal Wine hook tests pass; a failed injection prevents game startup.
Regression coverage includes ticket expiry/replay/roles, certificate/name failures, logout and cross-launcher duel
claims. The full suite passed 454 tests; clippy is clean. A 200-plan authenticated fuzz run, including restart,
reported no failures. Linux and Windows bundles build; the Windows launcher authenticated under Wine.
Setup and migration: [authentication.md](authentication.md). This completes the login milestone of
[issue #1](https://github.com/usersina/yo3-revival/issues/1); visual mods remain separate work.

Documentation review (2026-11-10): all ten tracked Markdown files checked for navigation and current setup.
README routes joining, local play and hosting to complete platform-specific commands. Corrected certificate setup,
source paths, backup coverage and stale milestone/protocol descriptions. A fresh Linux bundle authenticated its
launcher on alternate ports; 454 tests and clippy passed. Local links, anchors, image paths and table structure pass.

Review fixes (2026-11-10), each with a failing regression test before the implementation:

- [x] Trade confirmations belong to the current offers: an offer change clears both approvals, and final answers
      before both Ready marks are refused. Tests: `trade_contract` `changing_an_offer_withdraws_every_confirmation`,
      `a_final_answer_before_both_ready_marks_is_refused`. Live TCP: the stale answer was refused, both collections
      stayed unchanged, then two fresh approvals executed the changed offer once.
- [x] A refused player deletion leaves friends, ignores, waiting mail, rentals and team membership intact. Eligibility
      is checked against the current accounts before cleanup. Tests: `player_contract`
      `a_refused_last_player_deletion_preserves_their_social_data`, `a_team_leader_cannot_be_deleted_even_with_another_player`.
      Live TCP: the last-player deletion was refused and fresh friend, ignore, mail and rental replies were unchanged.
- [x] Key-only accounts use the same saved collections as address accounts: trades complete, rental offers are
      checked for ownership, and trunk requests refresh cards changed by another session. Three red-to-green tests
      in `trade_contract` and `rental_contract`. Live TCP: two `/join` accounts traded, an older session refreshed its
      trunk, and a rental refused an unowned card before accepting an owned one.
- [x] Duel ids do not grant membership: outsiders cannot upload a deck, cancel another pair's match with an invalid
      deck, or receive its duel-server handoff. Three red-to-green tests in `regulation_contract`. Live TCP: outsider
      uploads were refused without cancellation or handoff; both matched players still entered normally.
- [x] Hosted duels retain their chosen banlist after acceptance, in the match push, handoff and server deck check.
      Tests cover both a restricted duel under an unlimited login and the reverse. Live TCP: both clients received
      the chosen regulation in the match and handoff; the same starter was refused under banlist 4, accepted under 3.

Server-hardening verification (before the launcher migration): 451 workspace tests passed, clippy clean, and no
broken documentation links. The traced stock client on private display `:78` logged in, opened Edit Deck, selected a deck for Seto Kaiba, received
`0x22CA` and the `0x22C4` handoff, and reached turn 2 without an exception. The handoff's host, port and match type
were found by the tracer. This was an opening-turn check; no full manual duel was played in this fix session.

As of 2026-11-10 the server covers the client's game features: M0 to M2 (research, a stable world, an instrumented
client) and M5 (duels) are done, M3 (social) and M6 (play with friends) are done apart from checking on real friends'
machines. M4 (accounts, collection and economy) is implemented; the connection DLL sends Edit Deck's saves to the
server, so decks follow a player without a duel. Two duel problems found in the screenshot session are settled (one fixed, one hardened and watched). What is left is
tracked as `[ ]` boxes in the sections below and in "Open questions and guesses".

| Where                                                                                                                   | What it is                                                               |
| ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| The dated sections (overnight plan, Windows VM test, robustness review, monkey testing, screenshot session, duel check) | What each work session did and found                                     |
| M0 to M6                                                                                                                | The milestones, each ending in something checked against the real client |
| Open questions and guesses                                                                                              | Every value we picked without client evidence, to revisit                |
| Leads                                                                                                                   | Ideas outside the code                                                   |

The dated work sessions below are historical snapshots. Their old raw ports, IP-based accounts and join-page
instructions were superseded by the TLS launcher migration in M4; use [the guide](guide.md) for current setup.

## ▶ Overnight plan (2026-11-07): everything left, end to end

Goal: finish what the client can use, then hand over for the real friend test. Each slice goes the usual way
(evidence, failing test, code, live check on the headless clients, docs, its own commit fast-forwarded into `main`),
most valuable first. Research for the later slices runs ahead in parallel (read-only).

1. [x] Duels: an extra deck summon seen live (Synchro, materials picked in the card list); [x] the Number, Type and Attribute declarations
       (dialog kinds 11, 5, 7) and card name declarations (the card list's searchable kind `0x4C`); [x] options that
       are a card's own strings (the prompt table's 400–495 labels)
2. [x] Duels: a spectator through a match's later duels (live, no change needed); settled without code: chain
       chances outside timely ones (EDOPro's "always chain" asks with no card able to respond, which this client's
       Activation Message option has no setting for) and a server-side deck check (done later, see M5; the client checks sizes,
       ownership and the banlist itself; only a modified client gets past it, out of scope for friends)
3. [x] NPC experience (`npcbin`): wins against an NPC unlock its higher levels; every NPC enabled (`premium`), on
       its series tab, the four gated ones willing to duel
4. [x] D-Mail between players, kept for players offline
5. [x] Main Menu screens: Card Album, Rental Card List, My Profile, Duel Log, Leaderboard, BOOSTERPASS, each with
       what the server knows (collection, records, duels played)
6. [x] Teams (create, join, leave, members, team chat, Create Duel's Team limit) and user events (create, join, a
       knockout of event duels)
7. [x] Player search criteria: language, manner level, status, event, chat room, country (the account's setting)
8. [x] Shop: protectors and avatar items for BP; card trade and card rental between players
9. [x] A Windows server build (cross-compiled) in the release bundle, with the friends' guide updated
10. [x] Final: full test run (250 tests, clippy clean), every flow seen live during the night on the final code paths
        (and a last Auto Duel match and single duel, which caught the 0x1785 re-queue), the release bundles rebuilt
        (Linux and Windows); [ ] then the real friend test (M6 exit)

Not possible in the stock game protocol: logins with a user id and password (the client never sends
credentials; the required revival launcher now authenticates accounts over TLS; the delete dialog's password prompt is local too). (Corrected later: the client does have Create Player and
Delete on Select Player, `0x1F4A`/`0x1F4B`, now answered.)

## ▶ Windows VM test (2026-11-08): what a real Windows client turned up

The user played the real client natively on a Windows 11 VM (its own address, its own account) against a headless
client, through the Linux release bundle's `run.sh`. Findings, in the order they came:

- [x] `yo3.ini`: only the gate line was changed, so the patch check went to Konami and the game never connected. The
      friends' guide now says all five lines change, how to edit a file in Program Files, and a browser check.
- [x] The host's firewall (firewalld's `libvirt` zone) blocked the VM; friends over the VPN (trusted zone) aren't
      affected. Noted in AGENTS.md.
- [x] No banlist by default (the user's choice); `--regulation` picks one.
- [x] The required mod prepares a legal local starting deck after login automatically (2026-11-10), and
      the login news describes this behavior.
- [x] When the rules engine can't run (then: Node.js missing or older than 22.18 or the engine not fetched; since
      v0.0.10: the `yo3-engine` program or its data files missing, or the program not running), the server logs an ERROR
      at start, the login news and the join page say duels are unavailable, and the launchers (`run.sh`, `run.cmd`,
      `engine/fetch.sh`) stop before starting a server whose duels would all end as draws.
- [x] Attacks and activations from any field slot but the first were ignored and the duel stalled: the client sends
      a field card's command as (zone slot, index 0).
- [x] Set Spells/Traps lay sideways like set monsters (ocgcore's face-down defense position).
- [x] No way back from an attack's or a card's target, or a chain list: cancelable lists now need no pick (kind 1),
      and the list's own script then takes a right-click as "decide with nothing picked", which cancels (its OK
      button never shows; a click on a card still decides at once when one is allowed). Live: right-click withdrew an
      attack and the monster could still attack.
- [x] The opponent's Set Spell/Trap showed blank white once activated: it now moves onto its own slot with its row.
      Live: Spellbinding Circle and Magic Drain showed their art on the other client.
- [x] The Graveyard pile showed a card back on both clients, even with cards moved in face up: a pile draws its top
      card's face only when that card's table entry has the flag of command 1000's uid top byte. Graveyard and
      banished cards now carry it. Live: both Graveyards showed their top card on both clients, also after a card left
      from under the top, and hovering one previews it.
- [x] An attack position monster offered "to Attack Position" too: a position change now offers only the entry that
      fits (to Defence in attack, to Attack in face-up defense, Flip Summon face down). Live: Giant Orc in attack
      position offered only "Change to Defence Position".
- [x] An attack on an empty field asked nothing (EDOPro asks only when a monster could be attacked instead): a direct
      attack now asks the original's "Attack your opponent directly?" first (**guess**: the original asked every
      time). Live: No went back to the battle commands (the question came again on the next click), Yes dealt 2700.
- [x] The Graveyard couldn't be opened. Now Confirm (the menu's bit 11, which highlights a pile) sits on both
      players' Graveyards and banished piles and the player's own extra deck, and opens a view-only card list under
      the original's own titles ("Opponent's Graveyard", "Your Extra Deck", ...); a right-click closes it and gives
      the field back. Live on both clients.
- [x] Card target lists had no message: EDOPro's HINTMSG_* hints for destroy, equip, face-up (Spellbinding Circle's
      target), discard, return, Set, control, show and position changes now read as the client's own prompts. Live:
      "Select card to destroy." (MST), "Select a monster to equip." (Gravity Axe).
- [x] Found while checking: an opponent's Set card in a target list drew nothing and couldn't be clicked (MST couldn't
      hit a Set card; an attack on a Set monster would have been stuck the same way). Such lists now open as the
      client's card-back kind (`0x4A`). Live: MST listed and destroyed a Set Spellbinding Circle. **Limitation**: that
      kind shows every card of the list as a back and can't be backed out of, so in a list of face-up and Set cards
      the player can't tell them apart.
- [x] Found while checking: a pile's commands were looked up at index 0 only, so an extra deck summon or Graveyard
      effect of any card but the pile's first was unreachable. They now sit at index 0, and with several cards able the
      card list asks which (the original's "Select the monster(s) to Special Summon from your Extra Deck.").
- [x] Found while checking: tokens never showed (Scapegoat's Sheep Tokens were invisible, so they couldn't be seen,
      attacked knowingly or used). EDOPro's tokens have no Konami id; the engine now names them and the server shows
      the client's own token card of that name. Live: four coloured Sheep Tokens on both clients; one destroyed by an
      attack shattered and left the field.
- [x] Testing aid: `--duel-time 0,0` turns the clocks of duels between players off (live checks run slower than
      180 s a turn).
- [x] Checked live in three more duels between the headless clients (to the end at 4000 LP): Pot of Greed, Change of
      Heart on a Set monster (it moved to the taker's side; its menu offered only Flip Summon), Man-Eater Bug's flip
      effect, Book of Moon, an attack on a Set monster (the card-back list), Dark Hole, Black Rose Dragon's wipe with a
      chained Spellbinding Circle, Monster Reborn from either Graveyard, Mirror Force, Witch of the Black Forest's
      deck search, a three-link chain, and WINNER / LOSER with the records on both clients.
- [x] Cards an effect reveals (EDOPro's MSG_CONFIRM_CARDS, e.g. Witch of the Black Forest's "show it to your opponent")
      weren't shown to the opponent: they are now, through HandShow (`0x13`): a hand card shows at once, a searched
      card when it reaches the hand. Live: the new card flipped to its face in the opponent's hand row and the info
      panel named it (recorded frames). Revealed Set cards and Deck tops still aren't shown (open).
- [x] A target list of face-up and Set cards opened as the card-back kind `0x4A` (every card a back, no way out): it now
      opens as kind 4 with per-card table flags, so face-up cards keep their faces, Set cards are backs and a right-click
      backs out. Live: Giant Orc's attack list showed a back and a face; right-click withdrew it; a click on the back
      attacked the Set card.
- [x] A monster whose position an effect changes right after its summon (Stumbling) stayed upright on its owner's
      screen. Reproduced live on the VM round (2026-11-08, Yugi headless against the VM): Goblin Elite Attack Force and
      Giant Orc, summoned under the VM player's Stumbling, stood upright on Yugi's screen (and stayed so through later
      changes: Enemy Controller to attack, Goblin's own change to defense) while the VM showed them right. Their batch
      was the summon move (attack), RunSummon `0x38`, ChainSet, ChainRun and `0x1C` to defense, in one burst; `0x1C`
      waits only for the card manager, so it ran while the summoner's summon effect played (`0x4FB7F0` keeps a pending
      effect mode per card, `+0xDC`). Now, within a batch, the summon move carries the position its card turns to
      (`settle_positions`), so no turn overlaps the effect. Test: `a_monster_turned_right_after_its_summon_lands_in_its_new_position`.
      Then the same round showed more: the VM player's own monsters (Lily, Reflect Bounder, Giant Orc) turned by their
      own Stumbling stayed upright on the VM too, even with a question splitting the batch, while Yugi saw them turn.
      So a player's own summoned monster ignores in-place turns on its owner's screen, whatever the timing; a card that
      arrives by a move is drawn in its position. A monster's turn now also moves it onto its own slot in the new
      position (with its row) before the 0x1C. Test: `a_monster_that_changes_position_moves_onto_its_own_slot`.
      Live check pending.
- [x] The summon effect played at the leftmost monster zone, not the summoned monster's: RunSummon / RunSpSummon's
      first value is the card's place (`player | zone << 1`, `0x514E5A`), not only the player.
- [x] The swords over the possible attackers stayed after a Battle Phase with no attack: BattleEnd now goes whenever a
      Battle Phase ends (`0x4F8E40` takes them down).
- [x] A direct attack showed no sword (the user, VM round): BattleAttack's target zone `0xFFFFFFFF` plays nothing, while
      any zone from 5 plays the direct cut-in and the strike `0xCB` (`0x4F8430` state 3). Direct attacks now go at the
      defending player, zone 16. Live check pending; an attack on a monster plays the cut-in and strike already.

## ▶ Robustness review (2026-11-08): what three code reviews found, and the fixes

Three reviews of the server looked for what one client (or one bug) could do to everyone. Each item was checked against
the code first, then fixed test first (`crates/yo3-server/tests/robustness_contract.rs` and the unit tests beside the
code); the evidence is the test named in each line.

P0, crashes and stalls:

- [x] 1 Nesting depth: `Message::decode` stops at 16 levels (`Error::TooDeep`); one 1 MB frame of 200,000 nested
      messages overflowed the stack and aborted the process before login. Tests: `codec.rs`
      `deeply_nested_messages_are_refused_not_a_stack_overflow`, `robustness_contract` `wire::a_crafted_…`. Live: the
      crafted frame sent to the running server, which went on answering a login.
- [x] 2 Poisoned locks: every hub, table and directory lock goes through `lock.rs` (`Locked::locked`, recovering from a
      panic). The clock pass runs in `spawn_blocking` (a panic in a pass is logged, not the end of the clock) and a table
      that panicked, under its lock or in its tick, ends its duel as a draw and goes. Tests: `duel.rs`
      `a_poisoned_hub_…`, `a_table_that_panicked_…`.
- [x] 3 Engine failure: when the engine process (the Node sidecar then, `yo3-engine` since v0.0.10) dies, hangs or
      can't start, both seats (and the NPC duel's player) get a draw
      end (command 2 {3, 0}), the match is reported once as a draw and the engine is dropped. Tests:
      `an_engine_that_dies_mid_duel_…`, `…cannot_open_the_duel…`, `…in_an_npc_duel…` (a proxy sidecar that dies on
      cue).
- [x] 4 Blocking: engine calls time out after 30 s (`engine.rs`, a reader thread; the hung engine is killed, then 3
      applies); connection handlers run under `block_in_place`, the clock on a blocking thread, and a table whose lock
      is busy is skipped (`try_lock`). Tests: `engine.rs` `a_sidecar_that_never_answers_…`, `duel.rs`
      `a_table_that_is_busy_is_skipped_…`.
- [x] 5 Prompts with no answer: `MSG_SELECT_SUM` is answered (the smallest set of cards that makes the sum, exact or
      "at least with none to spare", counting a card's two levels; the client's card list for it is not made: it takes the
      automatic answer), and a client's card name declaration with no dialog is answered; the other prompt types all
      had answers. A prompt that comes back identical 8 times, or more than 10,000 answers between two client choices,
      ends the duel as in 3. Tests: `translate_contract` `a_sum_prompt_is_answered_…`, `robustness_contract`
      `a_prompt_the_core_keeps_refusing_…`, `the_same_goes_for_an_npc_duel`.
- [x] 6 A player leaving before the engine started (janken, turn order, deck select, between a match's duels after a
      draw) now sends the other seat command 2 {1 won, 14 disconnected} and its result screen (`0x2009`) with the records
      unchanged; only a duel that began is counted. Tests: `a_player_leaving_before_the_duel_started_…`,
      `…in_the_turn_order_choice_…`.
- [x] 6b (2026-11-13) Live, that wasn't enough: the janken screen ignores command 2 and `duel_end` 1, so the player
      whose opponent left at the janken stayed there for good (a hand picked got no answer). Its `0x2009` now has
      `duel_end` 4 (`front::duel_left`): "Bob has disconnected.", OK, the WINNER panel with the records unchanged, Next,
      the map. The turn order choice is the same scene (`0x518631`) but wasn't checked live; a player arriving after its
      match ended still gets `duel_end` 1 (not reproduced live). Test: `an_opponent_leaving_at_the_janken_…`.
- [x] 7 A duel connection arriving after its match ended gets no table: it is told (command 2) and closed
      (`Hub::ended`). Test: `a_second_player_arriving_after_the_first_left_…`.
- [x] 8 `Hub::leave` ends a match of the leaving player that has no table (the other player is sent out as at a
      refused deck, `0x2009` duel_end 3, and an event's round goes on with them as the winner). Test:
      `a_matched_player_whose_client_dies_before_the_duel_frees_the_match`. Not done: removing an offline entrant from an
      event's list on leave (the round start already forfeits a player who is gone).
- [x] 9 The turn order (0xD) is taken once per janken, at the table and against the CPU (a second 0xD, or a janken
      after it, opened another engine process each time). Test: `the_turn_order_is_taken_once`.

P1, data integrity and stuck state:

- [x] 10 A side file that doesn't parse (friends, mail, teams, rentals, records, events) is renamed
      `NAME.corrupt-<time>` and logged as an error, and the server starts empty; the start-up backup skips files that don't
      parse (so repeated bad starts never rotate the last good copy out). `accounts.json` still stops the start with the
      parse error, as before. Tests: `store.rs`, `a_corrupt_side_file_is_kept_aside_…`.
- [x] 11 `next_player` and `next_user` are saved with the accounts; deleting a player also clears them from the friend,
      ignore and request lists, waiting mail, rentals (as lender or borrower), team invites and the team. Tests:
      `accounts.rs` `player_ids_are_never_reused_…`, `player_contract` `a_deleted_player_leaves_the_friends_…`.
- [x] 12 Team invitations (made and received) end when the player disconnects. Test: `team_contract`
      `a_team_invitation_ends_with_a_player_who_disconnects`.
- [x] 13 Trade and rental: a player in either scene can't invite or be invited, an acceptance refuses (a "no" to the
      inviter, `0x2BDC`/`0x2D55`) when either side is in a scene now, and ending a scene removes every one of the
      player's. Tests in `trade_contract` and `rental_contract`.
- [x] 14 A trade's four updates are one `Directory::update_players` (one save, all or none). Test: `accounts.rs`
      `several_players_change_together_or_not_at_all`.
- [x] 15 A purchase debits the stored BP under the directory lock (`Directory::charge`). Test: `accounts.rs`
      `a_charge_takes_from_the_stored_bp_…` (the race itself can't be made in a single thread).
- [x] 16 The engine is dropped when a duel is reported, abandoned, left or timed out (also covered by 3 and 6).

P1, network hygiene:

- [x] 17 Accept loops back off (50 ms doubling to 2 s) after an error; a connection silent for 180 s is closed (the
      client heartbeats every 30 s; `Config::idle_timeout`) and so is one that stalls inside a frame; a frame's buffer
      grows 16 KiB at a time as its bytes arrive (at most the 1 MiB frame cap per connection). The HTTP endpoint gives
      a request line 10 s. Test: `wire::a_connection_that_goes_silent_…`. Not done: a limit on the number of connections.
- [x] 18 The packet log writes at most 2048 bytes of a packet in hex (plus `raw_len`), 256 characters of a text, 64
      fields and 32 items per message. Test: `log.rs` `a_huge_packet_is_logged_in_part`.
- [x] 19 `0x1F4F`: any update that names a duel passes the deck check; update_type 1 is refused as before, any other is
      ignored (nothing kept, answer `0x22CA`) so a Card Swap can't bring a deck the check refuses (it only takes a deck of
      the same cards in the match anyway). Each deck list is cut at 100 rows and a deck name at 64 bytes. Tests:
      `regulation_contract` `the_deck_check_applies_to_every_update_that_names_a_duel`, `a_decks_row_counts_are_capped`.
- [x] 20 A failed `0x1F4C` takes the old registration offline, and until a player is selected only the login and
      Select Player requests (`0x1F47`-`0x1F4C`) and the heartbeat are answered. Test:
      `a_failed_reselect_drops_the_old_registration_…`.
- [x] 21 A deck entry remembers the address of the front connection that brought it and a duel connection from another
      address that names the player is closed (`DuelSession::with_peer`). Test: `a_duel_entry_is_claimed_only_from_…`.
      Friends behind one NAT share an address, but each entry still belongs to its own player id.
- [x] 22 Caps: an avatar blob 256 bytes (else `0x2CF6`), speech lines and comments 200 bytes, a mail's title 128 and
      body 2048, a form's fields (user event, hosted duel, NPC duel rule) 64 fields of 256 bytes, one unfinished event per
      host (`0x33F5` for a second) and 100 on the server, one hosted duel per player, 32 rule sets per session, 256 card
      list picks. Tests: `what_a_client_can_make_the_server_keep_is_capped`, `mail_contract`,
      `translate.rs` `the_picks_of_a_card_list_are_capped`.
- [x] 23 `price * count` is `checked_mul` (refused with `0x1C25`); the language and manner level shifts in player search
      are `checked_shr`. Tests: `shop_contract`, `search_contract`.

Skipped as the review allowed: mail deleted before the client fetched it; accounts made for any address (how friends
join). Live (release build, two headless clients, scratch data directory): the crafted nested frame left the server
answering; an Auto Duel between the two clients went from the match through Select Deck and the janken to the result
screens and the evaluation dialog (Chiron's direct attack at 1000 LP); a trade between two more accounts (raw protocol
from `127.0.0.2` and `127.0.0.3`, the real clients being busy in the duel) invited, accepted, refused a second scene
for the inviter (`0x2BD8` 106), both OK, both yes, `0x2BCA` and the trunks. Not done live: the trade scene on the real
client (unchanged by this work), a sidecar that really hangs (covered by the engine test).

## ▶ Monkey testing (2026-11-08): two monkeys, what they covered and found

Tools: `crates/yo3-monkey` (protocol monkey, run through `tools/monkey/protocol-monkey.sh`) and
`tools/monkey/ui-monkey.py` (the two real headless clients); how to run them is in AGENTS.md (Commands).

Protocol monkey (a supervised server on a scratch data dir; plans from a seed and the packet logs' real requests,
replayed in order and shuffled, fields mutated, random and garbage frames, drips, reconnects, plans as the clients of
four accounts, floods, HTTP junk; a canary logging in and listing players; rss/fds/threads/data-dir sampling; panic and
ERROR scan; Auto Duel pairs and NPC duels played with random commands, picks, dialog results, phase buttons, leaving at
random moments; SIGKILL and restart on the same data):

- fuzz seeds 1-1500: 49,464 messages; http-only 400 plans; 1500 client addresses (`addrs`); a flood of 400 connections
  every 12 s in every soak.
- soaks (24 or 20 connections plus duel pairs): 300 s x2 (about 0.9 M messages each), 600 s multi-account (79,423 plans,
  3.1 M messages, 99 duel pairs, 58 duel results), 2400 s (131,160 plans, 5.1 M messages, 237 duel pairs, 81 results),
  180 s with 7 SIGKILLs (1.2 M messages). Duel-only: 40 pairs (71 duel connections, 54 began, 33 results, 4,207 random
  actions). Peak server rss 205 MB, 455 fds, which fall back to 14 fds; canary p99 under 175 ms (a 24-connection load).
- Result: no panic, no ERROR, no death, no stall, no fd leak, no corrupt data file after crashes, clean restarts.

UI monkey: 40 min (seed 3): 4,845 actions (clicks, random clicks, right clicks, drags, keys, typing), 6 Auto Duel rounds,
12 duel connections, 8 duel results, clients killed mid-duel and logged in again; no crash, no dialog, no server
trouble, no stall. A second pass of 20 min (seed 4): 2,249 actions, 3 Auto Duel rounds, 6 duel results, no failure. The
Documents folder was restored and verified identical after each run.

Findings:

- [x] 1 The packet log grew without limit (2.6 GB in five minutes of a client that only sends). It now rotates at
      128 MB and keeps four files per run. Test: `log.rs` `the_packet_log_rotates_and_keeps_only_its_newest_files`.
- Monkey-side, not server bugs: Delete Player and Create Player (`0x1F4A/B`) changed the account the canary logged in
  as (a deleted Kaiba: no reply to `0x1F4C`, as designed), so the monkey leaves them out; a canary refused while the
  monkey itself kills the server is expected.
- Historical issue, resolved by required TLS profiles: one account per client address meant new addresses kept making accounts (about 2 KB of file and 40 KB
  of memory each; 1500 addresses were fine). Only a spoofing client could do this over TCP. Rotation is per run: a
  server restarted often keeps four files each time.
- Open, client (crash fixed 2026-11-13, see "Diagnose the client black-screen wait"): with the server frozen (SIGSTOP)
  the client shows no "no response" dialog within 60 s; the UI monkey's OCR check for that dialog was not exercised live because the server answered every
  request. OCR itself reads the client's text (tested on the Duel Menu).
- Real-client requests the server answers with nothing: duel `0x200A` (the first seat waits for the second, by design)
  and `0x200C` (duel messages that only produce commands). Nothing else was UNHANDLED.

## ✅ Screenshot session (2026-11-08): two things seen playing NPC duels

Found while playing Kaiba Corporation duels on the headless client for the README's screenshots (Elemental Heroes
and Miracle Magic official decks). Both are traced below.

- [x] Magical Hats (a Trap, chained to an attack) "stalled" the duel: after "Yes", the target pick works, then a
      list of 17 Spell/Traps shows; picking one only marked it and no OK button showed. Traced live (2026-11-08):
      the engine asks MSG_SELECT_CARD min 2 max 2, the list is right (kind 6, max 2) and the second pick decides at
      once; the hint never says two picks are needed. Fixed by naming the count on every list that needs two or more
      picks ("Select 2 card(s)."). See "Card lists" in the protocol notes.
- [x] A card's art went wrong once (Mystical Space Typhoon with Call of the Haunted, then a Normal Summon of
      Elemental Hero Sparkman showing the Typhoon's art). Not reproduced: the move streams of this chain replay
      consistently in a contract test and the packet logs of the session show no move whose row differs from its
      table entry. One way the mirror can go wrong was closed anyway: a card leaving a hand, Graveyard or banished
      pile was taken at the engine's sequence without checking its id; it is now taken by its id when another card
      sits there (a warning logs "mirror out of step" if it ever happens). If the art goes wrong again, look for
      that line in the server log.

## ▶ Duel check (2026-11-09): the client's view checked against the engine

After the client crash at `0x516185` (an engine hand shuffle the server didn't follow put the client's hand out of step,
and a card left it by the wrong index; see the crash history in
[client-internals](client-internals.md#crash-history-and-the-rule-each-one-taught)), a program now looks for that class
of bug instead of a list of cases:

- **A model of the client** (`crates/yo3-server/src/duelcheck.rs`): the hand lists, field slots, pile counts, card
  table and each card's place word, as DuelCardManager keeps them after the commands the server sends, built only from
  the client's code (each rule cites its address; see "Moves out of a hand go by index" and the paragraphs after it in
  [protocol](protocol.md)). It notes the client going wrong as it happens (a hand card taken out by an index holding
  another card, a HandShuffle with an empty index) and compares every hand, slot and pile with the engine's real field
  (the engine's read-only `state` op) after each engine step.
- **Self-play** (`duelcheck::selfplay`, `tests/duel_check.rs`): two seeded random players (any summon, Set, activation,
  attack, chain, pick or answer the engine offers) duel through the engine and a translator, decks drawn from a pool of
  searches, discards, hand and deck churn, control changes, swaps, flips, hats, tokens, Synchro and Xyz Summons; one
  duel in three as against a player (the opponent's hidden cards without rows), every fourth starting at 2000 LP. 240
  duels run in `cargo nextest run --workspace` (`YO3_CHECK_DUELS=n` per test for more); 400 more seeds were played
  while fixing. `tests/duelcheck_model.rs` checks the model alone.
- **In the server** (`--duel-check`, a testing aid): every NPC duel and table checks each step and logs `ERROR duel
check` with the difference (once per duel). The protocol monkey's `duel` and `soak` runs start the server with it.
  Proved by a server built with a translator broken on purpose (the opponent's draws dropped): ERRORs in every table and
  NPC duel, reported by the monkey as failures. The GUI's detailed diagnostics also enables it on the next server start.

What it found (covered by `translate_contract.rs` and the dedicated regressions below):

- [x] Preserve counters on control changes (Breaker/Snatch Steal, confirmed on both running clients) and dynamic
      stats on field moves/swaps (engine-discovered contract regression). Counters remain on their card object;
      moving it must not erase them or reset displayed values to printed ones.

- [x] Expanded validation (2026-11-10): independently query LP, current/base monster stats and counters; compare
      turn/phase events with translated commands. Unknown messages fail, while known unsupported mechanics are
      explicitly recorded as `[YO3 GAP]` rather than silently treated as covered. New regressions prove detection
      of dropped LP/stat/counter/turn/phase changes.
- [x] Actual-client snapshots: the required DLL independently reads slots, visibility, ordered hand UIDs, pile
      counts, cached LP/stats and per-card counters after card/battle/chain activity settles. Shared schema vectors
      and a synthetic-memory mutation harness run in workspace tests; 32-bit Wine tests also check hook guards.
      The report requires an acknowledgement for every checkpoint; absent, duplicate, superseded, unreadable and
      timed-out snapshots fail validation. This establishes cached state/visibility, not every pixel or legal menu.
- [x] Deterministic engine recordings under `--duel-check`: final deck order, seed, every input/answer and state
      query, plus SHA-256 fingerprints of sidecar/core/bindings/scripts/database/card IDs and the Node version (since
      v0.0.10 the `yo3-engine` identity: the program, its core commit, scripts, database and card ids).
      Existing bounded diagnostic sessions hold chunked recordings. `duel-replay.py` verifies exact replay with
      matching dependencies and rejects incomplete recordings. This replays the engine, not game UI actions.
- [x] Live proof on two isolated Wine clients: 35 native checkpoints through Fortress protection and
      Debris/Fortress/Quillbolt → Stardust; another 15 through Breaker's counter, Snatch Steal and a lethal direct
      attack to the result screen, including the final 0-LP checkpoint. No missing/failed acknowledgements,
      engine/model errors or crashes in those runs. All 61 engine calls of the counter duel replay exactly from
      the redacted indexed server session. Workspace: 547 passed, 1 skipped; check, strict Clippy, release build,
      mod/loader harnesses, helper tests and documentation links pass. Executable/INI hashes stayed unchanged.
- [ ] Complete menu/effect coverage and automatic native UI replay/minimization. Known reveal/reversal/disabled-zone
      gaps remain visible; neither the random corpus nor matching state hashes proves all card interactions.

- [x] Yusei battle protection (2026-11-10): Fortress Warrior and Shield Wing survived in the core but vanished in
      the client, leaving occupied, targetable slots. MSG_BATTLE's destroy flag precedes limited protection;
      BattleRun no longer hides from that provisional flag. Confirmed destruction moves now shatter through
      CardVanish. The model now follows BattleRun's hide bits, closing the gap that missed this issue.
      Real-core regressions cover both views, face-up/Set Fortress Warrior, Shield Wing's two protections and later
      destruction, plus legal Debris/Fortress/Quillbolt → Stardust and correctly rejected Debris/Max Warrior.
      The three original Yusei decks are loaded from installed game data for 24 seeded checks across both seats
      (28 battles), alongside the ordinary 240 mixed-deck checks. All 533 workspace tests and strict Clippy pass.
      Live on two private Wine clients: Regained Bonds PvP finished at 0 LP with Shield Wing still visible after
      protected attacks; a controlled duel kept Fortress Warrior visible with no battle damage, then used it with
      Debris Dragon and Quillbolt to summon Stardust. Both views showed all three materials in the graveyard and
      Stardust's subsequent battle destruction of Debris correctly. Both duels reached the result screen at 0 LP,
      with no duel-check differences or client crashes.
      Detailed GUI diagnostics also starts the hosted server with `--duel-check` on its next start.

- [x] Magical Hats (MSG_SHUFFLE_SET_CARD 36) ignored: the client showed the cards under the wrong hats. ocgcore-wasm also
      misreads the message; the engine now reads the shuffled zones back. CardSwap `0x1B` trades the cards.
- [x] Creature Swap (MSG_SWAP 55) ignored: each monster stayed on its old side. CardSwap `0x1B`; **confirmed live**.
- [x] A token whose control changed (Snatch Steal on a Sheep Token) vanished instead of moving.
- [x] Xyz materials read as moving into the Extra Deck (ocgcore-wasm gives the Xyz Monster's location), and a detached
      one as leaving its monster's zone (which would clear the monster's slot on the client). Now off the field with
      CardVanish and moved on from an empty slot (shape; see "Xyz materials" in protocol).
- [x] Exchange of the Spirit (MSG_SWAP_GRAVE_DECK 35) ignored: both piles kept their old counts.
- [x] (Before the checker, the crash itself: MSG_SHUFFLE_HAND 33 ignored, and moves out of a hand named the engine's
      index; fixed in ac0afd8.)

The engine message types the translator leaves alone, and why (the audit):

- No card changes place: hints, waits, relations (CREATE/RELEASE_RELATION; targets and equips are followed, as hover
  markers), coin and dice, chain negations and disables, field zones disabled, MSG_SET (the move that follows does the
  work), RPS results, AI names.
- Only the Deck's or Extra Deck's order changes, which the client never sees (the Deck is a count; cards leaving the
  Deck or Extra Deck are found by id): SHUFFLE_DECK 32, REFRESH_DECK 34, REVERSE_DECK 37 and DECK_TOP 38 (Convulsion of
  Nature: the face-up Deck's top card isn't shown, **open**), SHUFFLE_EXTRA 39, CONFIRM_DECKTOP 30 and
  CONFIRM_EXTRATOP 42 (reveals not shown, **open**).
- [ ] Places change, not followed (never seen in 640 self-played duels with this pool): REMOVE_CARDS 190 (cards taken
      out of the duel), RELOAD_FIELD 162 (the whole field anew), TAG_SWAP 161 (tag duels, not played here), an Xyz
      material attached from a hand or pile (logged).
- [x] Equips and lasting targets (EQUIP 93, CARD_TARGET 96, CANCEL_TARGET 97) shown as hover markers between the
      cards (1017, see "Card relations" in protocol); the duel check holds them to occupied slots.
- [x] Hovering a Spell/Trap shows its markers too (checked live 2026-11-13, `--duel-deck 807,581`: hovering the equipped
      Axe of Despair drew the yellow ∞ on Vorse Raider, and the other way round). The hover pick (`0x50B510`) gives a
      Spell/Trap slot as `zone << 8` like a monster's; the earlier failure was before the relations' places were fixed.
- [ ] Not seen live: Magical Hats, Exchange of the Spirit and Xyz materials on the client (Creature Swap was). The
      client lacks the Xyz Monsters tried, so Xyz may not come up at all.

## ✅ M0: Research and tooling

- [x] Feasibility study ([research.md](research.md))
- [x] Wire crypto and message format; the client's data files decode with the same codec
- [x] Headless client on Xvfb with a virtual mouse; the client's own debug trace captured
- [x] Annotated disassembly, RTTI map, reply-dispatch router, Ghidra headless export
- [x] GhidraMCP (headless) registered for the project

## ✅ M1: Stable world

- [x] Gate redirect, version check, auth, login, player list, player select
- [x] World entry (`0x1FA4` → `9000`)
- [x] Every main-screen poll answered; news list; player search; regulations
- [x] Rust server (`crates/`), test-first, at parity on the real client
- **Exit:** a cold start reaches a stable main screen with no crash and no timeout dialog (met 2026-11-06)

## ✅ M2: An instrumented client

Stop finding missing fields one crash at a time.

- [x] Field-read tracer: a 32-bit DLL (mingw-w64) hooking the one lookup behind every field getter
      (`std::map::find` at `0x41A980`) and list-item reads (`0x410D60`). It logs the reply being parsed, field,
      getter type, found/missing, value type (from RTTI), the asking function and top/sub level.
- [x] Loaded by `vclient.sh start --trace` through an injector, so the game folder is never modified
- [x] `tools/tracer/summarize.py`: per reply, the fields read and the ones missing, with the asking function
- [x] First catch: `0x2775` (its reply later corrected to `0x2778`), `0x2CF7.result_number`, and the read-but-absent
      fields in `docs/protocol.md`
- [x] Code hits: `YO3_TRACE_HITS=0x…,0x…` logs when chosen client addresses run (int3 and a vectored handler); it
      found the startup step that kept the map locked
- [x] UI probe (2026-11-13): `vclient.sh ui` prints the open screens and dialogs (the client's component tree), the
      UI texts just looked up, the game's calls into its Flash screens (NPC talk, dialogs, janken) and the duel field
      as the client holds it, so most checks need no screenshot
      ([client internals](client-internals.md#the-ui-probe); every tool's view and blind spots:
      [development](development.md#which-tool-sees-what))
- Standing practice (moved to AGENTS.md): name functions in GhidraMCP as we understand them
- **Exit:** one session prints every field each reply was asked for, with the missing ones flagged (met 2026-11-06)

## ▶ M3: A social world

- [x] Answer `0x2775` (next present: `0x2778` = none, which closes PresentWindow and unlocks the map and menu)
- [x] Chat rooms (the chat panel's top-left button) and chat messages: room list, enter, leave, room chat
      with echo, whispers
- [x] Sessions registry: who is online, in which room (`Hub`); sessions leave on disconnect
- [x] Accounts support up to four players each. The original address-based login and self-service `/join` page
      are retired; the host now creates accounts and exports private TLS launcher profiles (M4).
      `tests/join.rs` protects the removed page returning 404.
- [x] Two clients at once: `YO3_INSTANCE=b` with its own display and reflinked Wine prefix
- [x] Player search returns the players online, filtered by name, level or id; both clients list each other
      (needed the friend and black list pushes, `0x251C`/`0x252A`)
- [x] Map locations: the map unlocks, and each of its six facilities opens and returns to the map (rooms are the
      chat rooms above; the DUEL panel belongs to duels, M5)
  - [x] Shop: opens to the Shop Menu with empty shelves and returns to the map (rental records pushed at entry)
  - [x] NPC duel: opens (Kaiba Corporation), says there are no valid decks, returns to the map (the trunk now has
        `cards`/`noTrade` arrays, all zero)
  - [x] Character settings: Avatar Settings opens with a default avatar and returns to the map (`avatar1` needs
        41 entries); saving changes is now handled (see Avatar Settings below)
  - [x] Official Event (Stadium) and Championship (tournament map): open, report no valid decks, return
  - [x] User event (Satellite Plaza): events created, listed, joined, started and run as a knockout
- [x] Friends and the ignore list: friend requests through the bell, accepting or declining, withdrawing, removing,
      the ignore list, both lists at login and on request, friends' online state pushed as they come and go, kept in
      `server.db` (the friends document, once `friends.json`); player search's friend, ignore and team tabs. Live: Yugi and Kaiba became friends, stayed so
      after a restart, and Yugi removed Kaiba
- [x] Main Menu's Friend List and Ignore List views: their search reply comes before their list, so the rows draw.
      Live: Kaiba listed with the friend icon, online and after logging out
- [x] D-Mail: Write Mail to a friend (`0x2580` → `0x2584`), the recipient's bell (`0x2581`), the fetch (`0x2583` →
      `0x2588`, `0x2582` → `0x2586`); mail waits in `server.db` (once `data/mail.json`) for players offline. Live: Yugi to Kaiba
- [x] Teams: form, invite, answer, leave, remove, authority, disband; the team in records, search and Player
      Information; team chat, team mail and Create Duel's Team limit; kept in `server.db` (once `data/teams.json`). Live: Millennium formed,
      Kaiba invited and joined, team chat, Set as subleader
- [x] Player search criteria: language (from the login), status (duelling, hosting by rule), chat room, manner level
      (from the manner ratings), events (nobody). Live: the waiting tab listed only the host
- [x] Manner ratings: after a duel between players the server pushes `0x2CEC {eval_id}`, DuelResult opens "Please
      evaluate NAME." (`dlg_manner.swf`), the answer `0x2CED {eval_id, eval_data}` (-10..10) is kept in the accounts
      and the average sets `evallevel` (search entries, Player Information, the player records). The client has no
      ratings for trades. Live: Yugi rated Kaiba +10, Kaiba rated Yugi -10; each saw the other's face icon change
- [x] Country: no request carries the client's country, so it is a setting per account (`yo3-server set-country`, kept in
      the accounts in `server.db`; default from the login's language), sent in the login item, player records, search entries (and
      the search's country criterion) and the bracket. Live: Player Information showed the UK flag, then Japan's
- [x] Main menu screens: Rental Card List (`0x2C34`/`0x2C35` chain), My Profile (wins and losses against players),
      Duel Log (`0x1793`, `0x19C8`), BOOSTERPASS (codes refused); Card Album, Leaderboard (a Konami
      web page) and Duel Options need nothing. Live: each opened; Team is with teams
- [x] Local duel logs: the client keeps no log by itself: after a duel against a player it downloads the `dlogURL`
      of the `0x2009` result (`0x512070`; never against an NPC), a `.dlog` the server builds (`duellog.rs`: header,
      both players, the spectator's command stream). Latest Duel Logs lists it and Play replays the duel. Live: Yugi
      and Kaiba's duel, listed, replayed to "Save this duel log?"; a match keeps its last duel only
- [x] Saving character settings: Avatar Settings' Confirm sends `0x2CEF` {avatar1, speech00–22, comment00–01,
      title} → `0x2CF5`; kept with the account and sent back in the player record. Live: purple hair after a restart
- [x] User events: Create, the list and details, Participate and leave, the lobby and its Event Menu (Start, Cancel),
      the event's duels paired round by round through the match push. Live: "Friday Cup", Yugi beat Kaiba in it
- [x] User events, the lobby: the bracket's names, faces and advances (`0x32FF` blocks, `0x32FD` entries), the round
      banner and status, a wait between rounds with the TIME countdown, the winner; open events kept across restarts
      (`server.db`, once `events.json`; a running event ends at a restart). Live with three players, a bye and a final. Ready/Standby:
      the client builds no such request (docs/protocol.md), so none exists to answer
- **Exit:** two headless clients log in as different players, find each other and chat (met 2026-11-06:
  Yugi and Kaiba chat in the Lobby; the room list shows the other player)

## M4: Accounts, collection and economy

- [x] Per-connection authentication through the launcher and connection DLL. TLS 7443 is the sole game listener;
      IP-based account selection and the HTTP join page are retired. Session capabilities last 12 hours and role-
      scoped tickets last 30 seconds, are single-use, and bind front decks to the same launcher's duel connection.
      Logout revokes tickets and active connections. Tests: `auth_flow`, `auth_contract`, `auth::tests`, launcher
      `proxy` and local-cookie tests; native/Wine tests cover the real WSOCK32 ordinal imports and named imports.
- [x] Existing account ids, cards, BP and other state keep their JSON schema. `profile PLAYER FILE --advertise HOST`
      exports a private profile using its current key (or assigns a key to an address-only account). The host creates
      accounts with `add-account`; there is no legacy-client fallback. See the authenticated setup guide.
- [x] Player creation and deletion: Select Player's Create Player and Delete (`0x1F4A {pindex, pname}` → `0x2262`/`0x2263`,
      `0x1F4B {player_id}` → `0x2264`/`0x2265`; `CharMake.cpp`'s `0x2CF5/6` is only the appearance editor), within four
      slots kept with the account. Live: Joey made in slot 3 (name, then Avatar Settings, then the main screen), deleted
      with its password prompt (local, nothing sent), the slot empty again
- [x] Trunk contents: the card collection (the starter deck's cards and the player's bought cards, `0x1FA4` → `9000`)
- [x] Deck list, deck editor saves (`HolderSelect.cpp` "deck list")
  - [x] Every player owns a starter deck's cards and gets it in the deck list (`0x1F4E` → `0x22C8`, tag-6
        items); Edit Deck saves it locally and the NPC duel accepts it
  - [x] Official character decks: every player also gets Yami Yugi's, Jaden's and Yusei's three decks each (the
        client's own NPC decks 120-122, 147-149, 171-173) and owns their cards. Since 2026-11-13 they are on Load Deck's
        Official tab, with the starter's recipe and the bought structure decks: the server sets their recipes' items
        (`pbin_no`) in the player's own record, and the tab reads the client's own `data/recipe/` files (`0x43D8D0`).
        The deck list (`0x22C8`) keeps only the starter and the player's own decks. Live: the tab listed the nine and
        CONCEALERS DECK; Yugi's "Miracle Magic" loaded with its 42 + 3 cards and saved as the player's deck.
  - [x] Deleting a deck (`0x1F50` → `0x22CC`); the stock editor saves only local files, so the connection DLL sends
        its saves itself (below)
  - [x] Decks on the server, so they follow a player to another machine: a duel's Select Deck sends the whole deck
        (`0x1F4F` update_type 1), and the connection DLL sends Edit Deck's saves the same way without a `duel_id`; the
        server keeps them per player and lists them in `0x22C8`. Live: a deck used in a duel came back after its local
        file was deleted; a deck saved in Edit Deck appeared in the other machine's Load Deck
- [x] Shop: every booster pack of the client's pack files on the shelves (`0x1C22`), bought with BOOSTERPASS Points
      (`0x1B68` → `0x1C24`), opened as presents (`0x2775` → `0x2777` with the cards); BP and bought cards kept per
      account and player in the accounts (`server.db`, once `data/accounts.json`). Live: three Collection Pack vol.1 opened card by card, and after a
      server restart the shop showed 4700 BP and the pack 21.4% collected
- [x] BP from duels: each counted result pays into the account (win 100, draw 50, loss 30), which the login replies
      and every purchase re-read. Live: a lost NPC duel took 5000 to 5030, shown at the next login; a pack then left 4930
- [x] Structure decks: SD001–SD005 (the art-carrying `deck_type` 1 recipes) on the STRUCTURE DECK tab, bought as
      type-1 presents with the deck id; the deck's cards join the collection. Live: GADGET BOX (deck 502) for 500 BP,
      shown as its 40-card grid, then 41 cards (with its extra deck) in the account
- [x] Bought structure decks in the deck list (`0x22C8`), under their names from the client's text table
      (`data/text/txttbl_E.bin`, now read by `yo3-data`), kept per player. Live: GADGET BOX was saved locally as
      "GADGET BOX.y3d", listed in Load Deck and loaded with its 40 + 1 cards
- [x] Protectors and avatar items for BP: their shelves from the item catalogue, presents type 7 and 6, owned ids kept
      with the player in the record's pbin. Live: P-01K and Domino: Red bought
- [x] Card trade between players: invitation, the trade scene's offers, OKs and confirmation; ownership checked and
      the cards moved in the accounts; a new trunk after. Live: Tainted Wisdom for Windstorm of Etaqua
- [x] Card rental between players: invitation, the rental scene (conditions, cards, Confirm), the records `0x2D6C` the
      client counts its trunk from, the Rental Card List, terms of 1/3/7/14 days that end by themselves, kept in
      `server.db` (once `data/rentals.json`). Live: Yugi lent two cards to Kaiba, both trunks changed, a restart kept it and the term's end
      gave the cards back with both online. Not seen live: a Deck rental (three lists); the client has no early return
- [x] Presents between players: not possible, the client has no way to send one. PresentWindow only asks for the next
      present (`0x2775`); no screen builds a send request (docs/protocol.md, "No presents between players"). Cards
      move between players by trade and rental, both done
- **Exit:** create a player, open packs, build a deck, restart the server, everything is still there

## ✅ M5: Duels

- [x] The main screen's DUEL panel: the menu bar's duel button opens the Duel Menu (`0x17FC` counts); Auto Duel asks
      for a match (`0x1798`), and the server pairs two players waiting for the same rule format (push `0x1978` with
      the opponent and the duel). Live: two clients matched and saw each other
- [x] Create Duel: a hosted duel shows on its host in player search (Duel Conditions, Send Duel Request); the
      offer reaches the host, who accepts or declines; passwords, a busy host and a withdrawn offer are handled; the
      duel starts with the host's LP. Live: Kaiba challenged Yugi's hosted duel and both reached the janken
- [x] Match Mode: best of three on one duel connection (1500 starts the next duel's janken, `0x2009` carries the
      match result); Auto Duel's Both matches either rule format. Live: Kaiba won a hosted match 2–0
- [x] Duel clocks between players: the timers message (`0x14`) drives each side's turn and total clocks; the server
      runs them while the engine waits on a player and ends the duel when one runs out (reason 11). Live: in an Auto
      Duel Kaiba's clock ran down from 180 while Yugi's stood still, and an idle player lost: "Yugi has ran out of
      time.", then the match went on to its next duel
- [x] Watching duels: Player Information's Watch Duel on a duelling player (mode 4): the front sends the duel
      server and both players (`0x1838` → `0x1839`), the duel server catches the spectator up from seat 0's side
      (janken, setup, the duel so far, both hands hidden) and then sends every event, and at the match's end 1505
      with the result. Live: Joey watched Yugi and Kaiba's Auto Duel and went back to the map when Kaiba quit, and
      watched a whole Match Mode match (the pre-duel screen between duels, the second duel, the map at the end)
- [x] Card Swap between a match's duels: a player with a side deck gets 1504 (DeckReplace), the swapped deck comes
      back through the front (`0x1F4F` update_type 2) and the duel connection's `0xE` starts the next janken. Live:
      Yugi swapped Danipon in for Crass Clown and duel 2 was dealt from the swapped deck
- [x] Create Duel's Opponent's Limits: Friend lets only the host's friends challenge (the client hides the button,
      the server refuses with 201); Team accepts only the host's team members. Live: Joey had no Send Duel Request, Kaiba
      did and his request reached Yugi
- [x] NPC roster and duel start: 25 named NPCs (ids 120–194 from the NPC deck presets; unnamed 195–197 omitted) listed by name with their
      levels; picking one runs `0x1784` → `0x17AC` → Select Deck (`0x1F4F`) → `0x17D4`, and the duel screen opens
      with both players. The client builds the NPC duel metadata (DuelManager `0x50ECF0`); the authenticated duel server runs the rules engine
- [x] Duel server handoff: after `0x17D4` the server pushes `0x22C4` (`duelData {host, port}` + `rule`), the
      client connects through the launcher's loopback duel proxy and TLS role 2 and sends `0x200A {player_id, autoSet}`
- [x] Duel connection mapped (both directions' `duelRequestName`s, the engine command pump) and the janken played
      against the server as the CPU (commands 1500–1503): the duel field opens, Turn 1
- [x] Duel opening: setup `0x15` with both decks' sizes, duel start, LP 8000 (`0xF`), the first player's Draw Phase
      (5); the engine commands are mapped to the components that handle them (docs/protocol.md)
- [x] Cards: both shuffled decks go into the engine's card table (command 1000) and each player draws five (1000 +
      the card move `0x1A`); the opening hand shows, the CPU's face down
- [x] Player actions mapped: phases, the command map `0x12`, the input wait 4, the menu's 14 command bits and the
      client's card command (`duelRequestName` 2), confirmed live with a test build
- [x] Rules engine: EDOPro's core (rule MR2), one process per duel; first the Node sidecar (ocgcore-wasm in
      `engine/duelcore.ts`), since v0.0.10 the native `yo3-engine` (`crates/yo3-engine`, below), JSON lines, cards as
      Konami ids (4,702 of the client's 4,771 map to passcodes through YGOProDeck's `konami_id`; the rest look like tokens); `engine/fetch.sh` fetches the core, ProjectIgnis' scripts and database
- [x] Card check (2026-11-12, `tests/card_data.rs`): every client card loads in the engine as the client shows it.
      It found three faults, now fixed: Barrel Dragon mapped to an artwork passcode the database lacks, so it had no
      data or script; the second prints of Dark Magician (5165) and Polymerization (4867), under Konami ids YGOProDeck
      doesn't list, never reached the engine and vanished from the deck (now found by name, each keeping its id on an
      artwork passcode of its own); Red-Eyes Darkness Metal Dragon's script is in `pre-errata/`. The 12 game
      exclusives EDOPro lacks are refused by the deck check
- [x] Card sweep (2026-11-12, `tests/card_sweep.rs`, `YO3_CARD_SWEEP=all`): every card the engine plays (4,684)
      self-played in both decks with the duel check. The first full run failed 74 cards, now all fixed: the duel
      check read the wrapper's counters the wrong way round (it gives `{count: type}`); zone prompts named only the
      player's own zones (Give and Take, Black Garden) and one zone where two were asked (Ground Collapse); a counter
      removal across cards took all from the first (Cloudian - Altus); a sum with `max` 0 (no bound) picked nothing
      (Machina Fortress); tokens with Konami ids (Doomsday Token) never reached the client; the CPU named cards the
      effect's filter refuses (Prohibition, Mind Crush). Default runs sweep 24 cards
- [x] Round trip in the duel check (2026-11-12): every answer the engine offers must be one the client's windows can
      give (each command map click, pile list pick, chain chance, card choice, question, option), and a choice the
      server makes for the player is counted. It found a card offering two effects at once could only use the first
      (now the card list asks which), and options without client labels were taken as the first for the player (now
      the card list shows the option's card once per option), and sums are the player's in the card list too. Zone
      choices stay automatic (the client's own "auto
      place" option, on in every saved player config of the 2010–2012 data). The full sweep with it: 4,684 cards, no
      failure; choices made for the player: zones, two chain orders, three card orders, ten disabled zones
- [x] Konami's own duel logs (2026-11-12, 227 from a 2010–2012 client folder, `tools/re/dlog.py`): the log file's
      header and body fields, replays with every card shown, card table Konami ids, move kinds, surrender 12,
      summon places with face bits, ChainRun's place; and what the original showed that we didn't: LP changes shown
      with their number and sound, the Set and activation effects, Field Spell backgrounds, the avatars' speech
      (draw, summon, activation, Set, turn end, damage). See "Presentation from Konami's logs" in protocol.md for
      what is still open (CardBreak, music). Live (NPC duel against Kaiba, `--duel-deck 1408,550,557,120`):
      Necrovalley turned the field into its canyon (back after the Battle Phase's own board), Gemini Elf's summon and
      Trap Hole's Set played the avatar's cut-ins, Kaiba shouted his damage line, "-1240" counted his LP down from
      7000 to 5100 (applied once), and the duel went on
- [x] Engine data pinned (2026-11-12): `engine/fetch.sh` installs the card scripts and database at pinned commits and
      the Konami ids from `engine/card-ids.json`; `tests/engine_cards.rs` checks the pins and, without game files,
      that every card of the client's era loads; CI runs that on the release bundle's engine. It found the bundles
      shipped only `official/` scripts and trimmed the artwork ids, which lost Red-Eyes Darkness Metal Dragon and
      Barrel Dragon in v0.0.8; `release.sh` now ships `pre-errata/`, `pre-release/` and the pinned ids whole
- [x] Real-deck sweep (2026-11-12, `tests/original_decks.rs`): the 227 deck pairs of Konami's own logs, self-played
      with the duel check. It found that a deck with a Fusion or Synchro Monster in its main deck split the client's
      view from the engine (which moves the card to the Extra Deck): the deck check now refuses such decks (the card
      kind is CARD_Prop's second word, bits 3-7). All 681 duels (3 seeds) pass
- [x] The heroes draw what they need (2026-11-13): Yami Yugi, Yugi Muto, Jaden Yuki and Yusei Fudo have the Heart of
      the Cards, a hidden skill (`crates/yo3-engine/src/skills.lua`, given by `npc.rs` through `new`'s `skills`):
      before each of their draws they pick which card of their Deck is on top, and the bot picks by playing each
      candidate ahead. Moving a card within the Deck sends no message, so the client simply sees the draw; the duel
      check with the skill on the opponent's side (`YO3_CHECK_SKILLS=1`) keeps all 240 duels in step. In copies the
      bot now activates what it can after summoning, so a Spell drawn or held counts. A hero beats the same bot with
      the same deck 17 of 20 (11 of 20 before that fix). Live: Yami Yugi's first turn put Summoned Skull and a second
      monster on the field and hit for 3,800
- [x] A server started from another directory finds its rules engine (2026-11-13): `engine/` beside the program
      after `engine/` under the current directory (`YO3_ENGINE_DIR` still first). The v0.0.10 Scripts bundle's
      `yo3-server`, run by its path from elsewhere, had no engine and ended every duel as a draw
- [x] Bot and banter polish (2026-11-13): a decision has a time budget (700 ms, then 300 ms more for a try under
      way; doing nothing more is always tried first) and one ready copy per thread. Seed 9001's late turns (five
      monsters a side, each engine step up to 80 ms as the core checks every Extra Deck summon) took up to 2.7 s a
      decision, now 1.1 s; 30 ms on average. It still won 34 of 40 against the old CPU with mixed decks, all 40 at
      once. The duel replay no longer replays `think` (its answer depends on time). An NPC's taunt no longer uses the
      turn's two reactions (Yami Yugi's winning direct attack went unsaid)
- [x] The NPCs talk (2026-11-13): each NPC has its own lines (`crates/yo3-server/src/npc_lines.txt`, ours, in each
      character's voice) in the duel's chat window, since the client's own speech for NPCs is fixed. `banter.rs`
      reads the engine's messages: the duel's start and end, its turns now and then, a summon of 2400+ ATK (named,
      once per card), its direct attacks, 2000+ damage it takes, a hero's draw (Heart of the Cards), and Pegasus'
      Millennium Eye naming a card in your hand (the engine's state) or the card you just Set. At most two lines a
      turn and one of a kind; each is sent when the client should be showing its moment (a pace per message, 1.5
      times `pace()` and a 3 s opening, measured live). Live: Pegasus named the Dark Mirror Force in our hand and the
      one we Set as it landed; Yami Yugi's win line showed with YOU LOSE
- [x] The duel bot looks through the opponent's turn (2026-11-13): a copy plays the option, the rest of the bot's turn
      by its rules, then the opponent's next turn with only its public cards (face-up monsters attack, nothing from the
      hand, no chains), and is judged when the bot's next turn begins. The two most promising idle actions are also
      tried with each follow-up action (Tuner, then Synchro). Four copies are kept in step with the duel and rebuilt in
      the background, so an NPC thinks in about 8 ms an answer (at most a quarter of a second) when it plays alone.
      Against the first version: 30 of 40 mirror duels won; against the old placeholder: 38 of 40. Each NPC has a
      character (`crates/yo3-server/src/npc.rs`: Kaiba and Jack attack hard, Syrus and Luna hold back) and Pegasus'
      Millennium Eye sees your face-down monsters and Set cards. Seeing alone tied the fair bot (10 of 20); assuming
      every Set card springs made it timid (16 of 40), so his copies still expect only your public cards
- [x] A duel bot for the NPCs (2026-11-13): `yo3-engine`'s `think` answers the NPC's prompts. At a decision that
      matters (summon, activate, Set, chain, a card pick, yes/no) it tries each option in a copy of the duel (the
      core rebuilt from the same seed and every answer so far, one copy per thread), plays the action out, and keeps
      the option worth most by LP, cards, board strength and what the opponent's next battle would cost; attacks,
      positions, places and costs follow rules. It plays fair: in a copy its opponent never responds, it values the
      position only by what a player could see, and it never simulates an attack into a face-down monster. Against
      the old placeholder CPU (`tests/npc_bot.rs`, the client's 75 NPC decks): 35 of 40 mirror duels won, 31 of 40
      with mixed decks; about 50 ms an answer on average. Live: Kaiba Ritual Summoned Blue-Eyes White Dragon with
      Advanced Ritual Art on turn 1, set two cards, and attacked over our Blade Rabbit. It also found a loop the old
      CPU had (finishing a pick list with nothing picked cancelled Panther Warrior's attack, declared again forever),
      now fixed in both
- [x] A native rules engine, no Node.js (2026-11-13, issue #3): `yo3-engine` (`crates/yo3-engine`, AGPL like the core)
      compiles EDOPro's core and Lua from the commits ocgcore-wasm 0.1.6 was built from and answers the server's
      JSON lines exactly as `engine/duelcore.ts` did, so the translator is unchanged. `tools/engine/parity.py` ran both
      engines request by request: the duel check's duels (480, twice the usual seeds), the full card sweep (4,684
      cards) and the 227 real-deck pairs gave the same answers. The only difference is the order within a run of card
      and player hints, which follows the memory allocator (the core walks those effects by address). The bundles drop
      Node.js and ocgcore-wasm; the Windows build is one self-contained `.exe` and answers byte for byte like Linux's (Wine).
      Desktop 0.0.9 and older check an update for the sidecar's files, so the bundles keep short notes at those paths
- [x] Translator, first part: the engine drives the duel session; its draws, turns and phases become engine commands,
      the client's idle prompt a command map of the legal commands, the client's card commands its responses; the CPU
      ends its turns at once (a placeholder bot, later replaced: see the placeholder CPU below) and other prompts are answered with defaults
- [x] Card moves: the engine's MOVE becomes a card-table entry plus `0x1A`; place prompts are answered for the client
      (first free zone). Live: a Normal Summon from the hand
- [x] Phase buttons: Main Phase prompts enable BP and EP as the engine allows (command 4 mode 2 + other flags); a
      pressed button ends the phase. Live: summon, EP, the CPU's turn, our Turn 3 with BP and EP lit
- [x] Battle: the battle prompt offers the attackers (Attack) and M2/EP; damage, costs and recovery change the LP
      shown. Live: a direct attack, 8000 → 5700
- [x] A placeholder CPU: it Normal Summons (or Sets) the first monster it may, attacks with everything, never chains;
      other prompts get simple answers (first cards, yes, first option), also for the client's prompts without a window.
      Two such AIs play a full engine duel to a win (test). Live: the CPU summoned, attacked and destroyed our monster
      (LP 8000 → 7350; it went to the graveyard pile). Since replaced by the duel bot (`crates/yo3-engine/src/bot/`,
      look-ahead in copies of the duel) with each NPC's style in `npc.rs`
- [x] Field stats: a monster entering a monster zone gets its record (data commands `0x3E9`–`0x3EB`) with CARD_Prop's
      ATK/DEF, level, attribute and race; the field label and card info show them
- [x] The duel's end on screen: command 2 {result, reason} plays the win/lose/draw movie with its reason; the front
      push `0x2009` (duel data, tag 7) ends the match, so the result screen shows the record against this opponent and
      Next returns to the map. Live: "YOU LOSE", LOSER/WINNER panels, back at Kaiba Corporation
- [x] Players kept across server restarts (accounts and records in `data/`, M6)
- [x] The client's own card choices: targets, tributes, attack targets and other card selections open the card list
      (command `0x37` and its data commands) with a message from the client's duel prompts. Live: an attack target
- [x] Yes/no and effect questions: the Yes/No dialog (command `0x36` kind 2) with the card's name. Live: Marauding
      Captain's optional effect, then its Special Summon choice. Testing aid `--duel-deck ROWS`
- [x] Positions, options and chains: the position dialog (kind 8), the option list (kind 4) for options the client has
      labels for (coin calls, card types), and the chain question (kind 3, with the cards) then the card list. Live:
      Marauding Captain's Special Summon in Defence, Time Wizard's coin call, two Trap Holes chained to the CPU's summon
- [x] Declarations: types, attributes and numbers in their dialogs, card names in the card list's searchable kind.
      Live: Abyssal Designator (DARK, Fiend), Prohibition (Giant Orc), Star Blast (1000 LP)
- [x] Options that are a card's own strings: EDOPro's `code << 20 | n` paired with the prompt table's per-card labels
      (110 options of 44 cards). Live: Card Shuffle's "Your Deck" / "Opponent's Deck"
- [x] Places and chains outside timely chances, settled: the client has no place window (the server takes the first
      free zone), and EDOPro's "always chain" (asking when no card can respond) has no setting in the client
- [x] Position changes and flips on the field: command `0x1C` (CardFlipTurn) for MSG_POS_CHANGE and flip summons; the
      command ids named after Konami's view types (Master Duel's `DuelViewType`). Live: Giant Orc to defense after
      attacking, a Set Trap Hole turning face-up
- [x] Battle and summon animations: BattleInit/Attack/Run/End (`0xA`–`0xE`, figures and destruction per player)
      and RunSummon/RunSpSummon (`0x38`/`0x39`). Live: the NPC's attack cut-in, the hit, the destroyed card going,
      summon sparkles
- [x] Chain link animations: ChainSet `0x33` {player, zone, link} at each MSG_CHAINING and ChainRun `0x34` {_, _,
      link} at each MSG_CHAIN_SOLVING. Live: two Trap Holes chained to the CPU's summon showed links 1 and 2, the
      chain banner (`duel_chain.swf`), and each number fading as its link resolved
- [x] Confirmed destruction: CardVanish `0x1F` {player, zone | index << 8} before the card's move, for MSG_MOVE's
      reason DESTROY, including battle (the engine reads the reason back, since ocgcore-wasm's parse, whose shape it keeps, drops it). CardBreak `0x20`
      only hides a card's label. Live: Trap Hole shattered Sangan, then the duel went on
- [x] Stat changes from effects: the engine reports the monster zones after each engine step when they changed, and
      a face-up monster's record (`0x3E9`–`0x3EB`) goes again, which the client re-reads after every command. Live: Axe
      of Despair took Giant Orc's label and card info to ATK 3200 (in blue)
- [x] Attack targets: the original used dialogs too (its dialog texts have the target list, "Attack your opponent
      directly?" and the replay question; the client has no field target step), so the card list stays, and EDOPro's
      direct-attack and replay questions read as the client's own (225, 223). Live: Jinzo #7's attack asked "Attack
      your opponent directly?"
- [x] Counters: EDOPro's counters on a field card become `0x27` in the client's own counter kinds (the icon on
      the card's label, "Spell Counters 1" in the card info); a card leaving the field drops its counts first. Live:
      Breaker the Magical Warrior's Spell Counter
- [x] Deck piles on the field: DeckReset (`0x16`) for each deck and extra deck after the opening's LP; extra decks
      join the card table so their summons move from the pile. Live: deck piles of 34 and 35 counting down with the
      draws, and a one-card extra deck pile on both clients
- [x] Extra deck summons: commands on extra deck, Graveyard and banished cards at their pile; Synchro and Fusion
      materials one at a time in the card list. Live: Junk Synchron and Giant Orc into Ancient Fairy Dragon. Testing
      aid `--duel-extra ROWS`
- [x] NPC experience (`npcbin`) so higher levels unlock: wins, losses, draws and experience per NPC level id from
      the duel records; `need_ex` 100/200/300, the series tabs, `premium` and the gated NPCs' pbin bits. `0x17D4` only
      signals ready. Live: beating Seto Kaiba opened his level 2 and showed "Wins 1 / Losses 0"
- [x] Online 3's duel stream is Konami's engine family but renumbered (turn change 6, LP set `0xF`; docs/protocol.md)
- [x] Regulations enforced: Select Deck's deck is checked for sizes, cards owned (rentals counted) and the duel's
      regulation (`deckcheck.rs`), refused with `0x22CB`. Live (testing aid `--duel-regulation 4`): both Auto Duel
      players saw "You have been disconnected from the Duel Server." and the map; a legal deck reached the janken. A
      stock client checks first, so only a modified one gets refused. The refused player's opponent gets a non-result `0x2009` (`duel_end` 3) and leaves for the map, live
- [x] PvP on the duel connection: both matched clients share one table (`table.rs`): the janken between their hands,
      the winner's turn order, one engine with a translator per client (each sees itself as player 0), each prompt to
      the client it asks, the opponent's hidden cards without rows, a player leaving loses. Results reach both front
      connections. Live: Yugi against Kaiba, Giant Orc's direct attack, WINNER / LOSER screens, both back on the map
- [x] Players' records written back (`server.db`, once `data/records.json`, M6)
- **Exit:** two clients play a full duel with real decks to a finish (met 2026-11-07: two headless clients matched
  through Auto Duel and played their starter decks to a win)

## ▶ M6: Play with friends

- [x] LAN/VPN hosting: `--bind 0.0.0.0`, a certificate covering the reachable name/IP, a private profile per account
      exported with `--advertise HOST`, and TCP 7443 through the firewall. Gate/front/duel/asset traffic uses TLS.
      The earlier raw LAN-versus-loopback duel check predates this transport; new-launcher remote-machine checks remain open.
- [x] Accounts and duel records kept in `data/` (then `accounts.json` and `records.json`; now `server.db`, the JSON
      files only imported once), backed up at each start; new
      players' names made unique (Duelist, Duelist2…)
- [x] Admin commands: `accounts`, `add-player WHO NAME` (add a player to an existing account), `rename`
- [x] Linux and Windows bundles (`tools/release.sh`): server, TLS launcher, connection DLL/injector, complete
      rules engine (with Node then; `yo3-engine` since v0.0.10), `run.sh` / `run.cmd` and
      [the setup guide](guide.md). No game files or credentials ship.
      Windows executables are cross-compiled with mingw-w64; the new Windows launcher authenticated under Wine.
- [x] Chain questions follow the player's Activation Message option (Manual / Semi Auto Cancel / Auto Cancel): the
      server sends a level per chance (3 after an event, 1 at a phase, 0 when nothing fits). Live: no question at the
      CPU's phases, the question at its summon
- **Exit:** friends connect from their own machines and duel (the network path is verified on one machine: a client
  through the LAN address against one through loopback, 2026-11-07; real friends' machines not tried yet)

## Open questions and guesses (revisit)

| Item                                                                      | Current choice                                                                                                                                                                                                                                                                                                                                                                                               | Why it's uncertain                                                                                                                                                                    |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starter deck and collection                                               | the first preset of type 1 that regulation 1 allows (preset 2); one copy of each of its cards                                                                                                                                                                                                                                                                                                                | The original let new players choose; a real collection comes with the economy                                                                                                         |
| Trunk (`9000`) contents                                                   | one record: `cards` (the starter deck's), `noTrade` all zero                                                                                                                                                                                                                                                                                                                                                 | Other fields later screens may read are unknown                                                                                                                                       |
| `regulations` list                                                        | all 20 ids                                                                                                                                                                                                                                                                                                                                                                                                   | Which banlists the UI should offer                                                                                                                                                    |
| `0x271A/0x2AFB/0x2AFD/0x27D8`                                             | minimal replies                                                                                                                                                                                                                                                                                                                                                                                              | Meanings unknown; they only stop timeouts                                                                                                                                             |
| `avatar1` slots                                                           | 41 zero entries (a default male avatar); the length is the client's own (settled)                                                                                                                                                                                                                                                                                                                            | Which slot holds which part category is not mapped (`0x42CA60` copies slots 0, 1, 2, 4–10 into separate fields, `0x429210` slots 17–40; 11–16 unread there)                           |
| Whisper `chat_type` values                                                | any type with `to_name` is a whisper                                                                                                                                                                                                                                                                                                                                                                         | Only room chat (type 1) was seen live                                                                                                                                                 |
| Player search paging                                                      | skip `base_point`, return at most `front_num`                                                                                                                                                                                                                                                                                                                                                                | Only page 1 (`0, 100`) was seen                                                                                                                                                       |
| Player search by name                                                     | case-insensitive substring                                                                                                                                                                                                                                                                                                                                                                                   | Konami's matching rule is unknown                                                                                                                                                     |
| Friend and black list pushes                                              | sent once, right after `9000`                                                                                                                                                                                                                                                                                                                                                                                | The original timing is unknown; the client only needs them before it lists a player                                                                                                   |
| Shop lists besides packs, `perk_num`, rental terms                        | shop lists populated; unused fields 0                                                                                                                                                                                                                                                                                                                                                                        | Packs, structure decks, protectors and avatar items are stocked; `perk_num`'s meaning is unknown                                                                                      |
| Pack prices and contents                                                  | 100 BP a pack, at most 10 per purchase; 5 cards: 4 of the pack list's first ascending run, 1 of a later run (each run half as likely as the one before), read as rarity tiers                                                                                                                                                                                                                                | The original's prices, pack size and odds are unknown; the runs match the OCG's rarity split (44/18/9/9)                                                                              |
| Starting BP                                                               | 5000 per account (`START_BP`)                                                                                                                                                                                                                                                                                                                                                                                | The original's amounts are unknown                                                                                                                                                    |
| Structure decks                                                           | SD001–SD005 only (the starters 1–3 have no art); 500 BP each, at most 3 per purchase, sale id 20000 + deck id; the owned-items bit (`pbin_no`) the client sets isn't kept by the server                                                                                                                                                                                                                      | The original's prices are unknown; nothing read so far depends on the bit for decks                                                                                                   |
| BP from duels                                                             | win 100, draw 50, loss 30 per counted result (NPC duels and duels between players; a match counts once)                                                                                                                                                                                                                                                                                                      | The original's duels paid no BP (no reply after a duel sets it); the client shows the new total at its next login or purchase                                                         |
| Pack presents                                                             | one per pack, `present_id` counting from 1 per session, `level`/`form` 0, `item_img` and `end_time` empty; the cards join the collection at purchase                                                                                                                                                                                                                                                         | Only the fields the window reads; presents aren't kept across reconnects (the cards are)                                                                                              |
| Event lists (`0x177B`), own user event (`0x33FE`)                         | official events: none; user events as created; paging by `base_point`/`front_num` like player search                                                                                                                                                                                                                                                                                                         | No official events are held                                                                                                                                                           |
| User events                                                               | a knockout paired in entry order; lobby status 0/4/6/7/8 as the round goes; a 20 s wait between rounds; the bracket's blocks above 8 seats built the same way (not seen live); a restart ends a running event                                                                                                                                                                                                | How the original drew and ran its tournaments is unknown (the statuses and texts are the client's)                                                                                    |
| Rental records push (`0x2D6C`)                                            | sent after `9000`, when a rental starts and when one ends (only the rentals not over)                                                                                                                                                                                                                                                                                                                        | The original timing is unknown; the shop only needs it before it opens                                                                                                                |
| Card rental                                                               | the ready request `0x2C3E` is answered `0x2D68` (the client reads no reply: settled); error codes 504/503 are never read by the rental handler (settled; every error shows "Rental Cancelled."); terms 1, 3, 7, 14 days (the dialog's); a No keeps the scene with `trade_result` 0 (settled, `0x41CAB1`); a rental ends exactly at its term, checked each second; a lender can't put up cards they have lent | The original's rental rules (list limits, early return: the client has none) are unknown                                                                                              |
| Country                                                                   | the account's setting, else GB/JP/DE/FR/IT/ES/KR/CN from the login's language                                                                                                                                                                                                                                                                                                                                | The client has no language-to-country table (it derives `lang` from the Windows language, `0x40DE50`); `country` is server data                                                       |
| NPC roster `need_ex`                                                      | 100/200/300 (the tabs follow the series: settled by the NPC name texts)                                                                                                                                                                                                                                                                                                                                      | No client file or script holds the thresholds; it can't all be 0 (`0x4BFDC4`)                                                                                                         |
| Teams                                                                     | `level` 1 and scores 0 for every team; handing over the leadership makes the old leader a member; the founding time in Japan time                                                                                                                                                                                                                                                                            | No team scores or levels exist without the original's events                                                                                                                          |
| Search status bits                                                        | `0x1000` duelling, `0x800` hosting, `0x200` hosting Single, `0x400` hosting Match                                                                                                                                                                                                                                                                                                                            | Read from the Detailed Search texts                                                                                                                                                   |
| Manner level                                                              | the average of a player's ratings (-10..10) mapped onto 0–4, (average + 10) / 5 rounded; 2 with no ratings; `evaltotal` the sum, 0 when negative                                                                                                                                                                                                                                                             | The original's thresholds and `evaltotal`'s sign are unknown                                                                                                                          |
| Trade end                                                                 | a refused trade sends `0x2BCA {trade_result 0, error_code 503}` ("Trade Cancelled", the scene closes: settled by `0x424F71` and seen live on both clients, with a later trade completing); the trunk is pushed after a trade                                                                                                                                                                                 | How the original refreshed the trunk is unknown                                                                                                                                       |
| Item prices                                                               | protectors 200 BP, avatar items 100 BP, one of each                                                                                                                                                                                                                                                                                                                                                          | The original prices are unknown                                                                                                                                                       |
| Avatar items for sale                                                     | the catalogue's parts with sub 0 (178 male, 148 female)                                                                                                                                                                                                                                                                                                                                                      | Read as "to acquire" next to the default (2) and starting (1) parts                                                                                                                   |
| Wins and losses                                                           | My Profile and Player Information count duels against players only, not NPCs                                                                                                                                                                                                                                                                                                                                 | The original's counting is unknown                                                                                                                                                    |
| NPC experience                                                            | 100 a win, 50 a draw, 10 a loss, per NPC level id                                                                                                                                                                                                                                                                                                                                                            | No request reports it; the original's amounts are unknown                                                                                                                             |
| `premium`                                                                 | 1 for every account (all NPCs, Form Team, Create Event)                                                                                                                                                                                                                                                                                                                                                      | The original sold it (Duelist Premium); a friends' server gives it to all                                                                                                             |
| `0x17D4`                                                                  | answered `0x17D5`, nothing kept                                                                                                                                                                                                                                                                                                                                                                              | Sent once the duel object is ready; may also carry results later                                                                                                                      |
| `0x22C4` push                                                             | right after `0x17D5`; `rule` = the fields of the duel's `0x17AC` item                                                                                                                                                                                                                                                                                                                                        | The original timing and rule contents are unknown; only `match_type` is read so far                                                                                                   |
| Server duel messages                                                      | request `0x200D`                                                                                                                                                                                                                                                                                                                                                                                             | The pump reads only `duelRequestName` (and needs the id non-negative)                                                                                                                 |
| CPU janken                                                                | random hand; the CPU goes first when it wins                                                                                                                                                                                                                                                                                                                                                                 | The original CPU's choices are unknown                                                                                                                                                |
| Card table entries (1000) | param3 the Konami id; flag byte 1 for some public cards (stored, no consumer found); uids 1.. in table order | Settled by Konami's logs (param3 the Konami id in all 53,586 entries); the flag byte's use is unverified |
| Hidden cards                                                              | against another player, the opponent's deck, hand, extra deck and face-down cards go with row 0 and face-down monsters get no field record; NPC duels the same for the NPC's cards (`duel::npc_translator`, v0.0.11)                                                                                                                                                                                                                                 | Online 2 used placeholders, whose value is unknown; face-up position changes reveal the card through `Translator::turn`; the original hidden-card encoding remains unknown            |
| Automatic matching                                                        | the first two players waiting for the same `match_type`, Both (6) with either and two Both players in a Single duel; the level choice is ignored; the second player's `0x1978` goes a second after its `0x1799`                                                                                                                                                                                              | Konami's matching rules are unknown; the delay assumes the agent reads one message per step                                                                                           |
| Match Mode                                                                | best of three; every duel opens with a janken; a drawn duel counts for neither; the records count the match once                                                                                                                                                                                                                                                                                             | Online 3's rules for who goes first in later duels, draws and the side deck are unknown                                                                                               |
| Hosted LP and `--duel-lp`                                                 | the testing aid wins over a hosted `lp_start` when it isn't 8000                                                                                                                                                                                                                                                                                                                                             | A testing convenience, not the original's                                                                                                                                             |
| Duel Menu counts                                                          | `duel_playing` = 2 × matched duels, `duel_ready` = hosted duels, `duel_auto` = players waiting                                                                                                                                                                                                                                                                                                               | Whether they count players or duels is unknown                                                                                                                                        |
| Hosted duels                                                              | `duel_status` 1 while hosting, 3 while in a duel between players; Team-only duels take nobody (no teams); `0x17C0` answered `0x17C1`                                                                                                                                                                                                                                                                         | The original values and the limits' rules are unknown                                                                                                                                 |
| A player leaving                                                          | the other wins (command 2 {1, 14 "disconnected"}) and both records count it                                                                                                                                                                                                                                                                                                                                  | The original's handling (time-outs, `0x2724` heartbeats) is unknown                                                                                                                   |
| Choice messages                                                           | EDOPro's hint mapped to the client's text for 7 hints; none otherwise                                                                                                                                                                                                                                                                                                                                        | The original server's message per choice is unknown                                                                                                                                   |
| Yes/no questions                                                          | 'Activate the effect of "X"?' for optional effects, 'Use the effect of "X"?' for other questions                                                                                                                                                                                                                                                                                                             | The cards' own question strings (EDOPro's) aren't in the client                                                                                                                       |
| Position dialog                                                           | button 0 face-up attack (face-down if only that), button 1 face-up defense when allowed, else face-down; the flag shows the face-up defense picture                                                                                                                                                                                                                                                          | The dialog has two buttons; ocgcore allows any mix of four positions                                                                                                                  |
| ChainRun `param1`/`param2`, ChainSet's place for a card off the field map | the link's ChainSet player and zone; the zone it is in (hand 11, Graveyard 14, banished 15), else the field's centre (16) | Settled by Konami's logs (ChainRun repeats its ChainSet; zones 11, 14, 15 and rarely 16) |
| Confirmed destruction                                                     | CardVanish (`0x1F`, the shatter) before the move; CardBreak (`0x20`) never sent                                                                                                                                                                                                                                                                                                                              | Which of the two the original sent, and when, is unknown; CardVanish is the one with an effect                                                                                        |
| MSG_MOVE's reason                                                         | read back by a query at the card's new place after each processing step                                                                                                                                                                                                                                                                                                                                      | A card that moves again within the same step reports its later reason                                                                                                                 |
| Stat changes                                                              | level, ATK, DEF, base ATK, base DEF from the engine; race and attribute stay CARD_Prop's                                                                                                                                                                                                                                                                                                                     | The client's race and attribute numbering isn't ocgcore's; type-changing effects are rare in the card pool                                                                            |
| Duel clocks                                                               | turn clock full again at each of the player's turns, both clocks running while the engine waits on that player (also during client animations), a time-out 5 s after 0; Auto Duel 180 s / 600 s, hosted duels their `time_turn` / `time_total` (0: off), NPC duels none                                                                                                                                      | The original's clock rules and Auto Duel's limits are unknown; the clocks follow Create Duel's own defaults                                                                           |
| Counter kinds                                                             | EDOPro ids mapped by name to the client's kinds (texts 200 + kind); Spellstone as "Magical Stone", Des as "Death", unnamed ones as 38 "Counter"; Mechanical (23) unused                                                                                                                                                                                                                                      | The original's kind per card is unknown; EDOPro's and Konami's names differ for a few                                                                                                 |
| Card Swap                                                                 | offered (1504) only to a player whose deck has a side deck; no time limit (the clock slot 4 the screen counts down also drives the janken's); a swapped deck with other cards is refused                                                                                                                                                                                                                     | Who the original offered it to, and its time limit, are unknown                                                                                                                       |
| Friends                                                                   | a request needs the other player online with no request waiting for them; no limit on friends (110 "The Player List is full." never sent); declining tells the requester; the team tab lists nobody                                                                                                                                                                                                          | The original's limits and offline requests are unknown                                                                                                                                |
| Watching                                                                  | from seat 0's side (the first player to enter); the decisive janken round only; NPC duels can't be watched                                                                                                                                                                                                                                                                                                   | The original's spectator view and opening are unknown                                                                                                                                 |
| Login banlist                                                             | regulation 3 ("unlimited", no banlist; the user's choice) unless `--regulation` names another                                                                                                                                                                                                                                                                                                                | Which list the original used for Auto Duel at each date is unknown; the client calls the login one "Latest Regulation"                                                                |
| Declarations                                                              | several types at once and numbers outside the number dialog (min or step above 15 units, max above 255) answered automatically; the CPU names a card the filter allows (the engine's `fits`)                                                                                                                                                                                                                 | The client's dialogs pick one type and a range of numbers; the original server's handling of the rest is unknown                                                                      |
| Material choice                                                           | Synchro and Fusion materials one at a time in the card list (EDOPro's order: the Tuner first); Fusion materials under "Select monsters to use as Fusion Material." (92), Synchro materials without a message                                                                                                                                                                                                 | The client has no Synchro material text; how the original asked is unknown                                                                                                            |
| Option labels                                                             | EDOPro's system strings 60/61 and 70–72 mapped to the client's labels; a card's own strings paired by hand with the prompt table's per-card labels (`card_options.rs`), else the card list (the option's card once per option, in order)                                                                                                                                                                   | The original server's label per card option is unknown                                                                                                                                |
| Chain chances                                                             | level 3 after an event, 1 at a phase or step, 0 when no card fits; messages 7/8 (Spell/Trap zone)/269                                                                                                                                                                                                                                                                                                        | Client side settled (`0x4C5560`: bands below 2, 2–3, 4+; Activation Message decides below 4); which windows the original judged low priority, and its message choice, are Konami-only |
| Rules                                                                     | EDOPro's `MODE_MR2`                                                                                                                                                                                                                                                                                                                                                                                          | Online 3 (2010) ran Master Rule 2; EDOPro's preset may differ in details                                                                                                              |
| CPU play                                                                  | the duel bot (`crates/yo3-engine/src/bot`): looks ahead within its turn, rules for attacks and positions                                                                                                                                                                                                                                                                                                     | The original NPCs' AI is unknown                                                                                                                                                      |
| Duel end reason | EDOPro's reason mapped to the client's texts (LP 0, deck out, surrender 12, Exodia...; others "won with a card") | Konami's logs: 1 LP 0, 12 surrender, 11 time out, and 100 with param3 1 (12 duels, meaning open) |
| Duel's end push `0x2009`                                                  | sent when the engine ends the duel; `duel_end` 1 (normal: no effect) or 3 (cancelled before it began); no `score_<id>`; `dlogURL` for duels between players                                                                                                                                                                                                                                                  | The original timing is unknown. Values settled by `0x515BD3`/`0x518631`: 2, 3 dialog `0x2918`; 4 "has disconnected" (we don't send it); 5 opens the result                            |
| Result screen counts                                                      | `win`/`lose`/`draw` = totals against this opponent including this duel, `*_add` = this duel's change; kept in memory                                                                                                                                                                                                                                                                                         | The client only shows them ("Wins 1 (+001)")                                                                                                                                          |
| Duel port                                                                 | launcher-selected loopback port; remote TLS role 2 on 7443                                                                                                                                                                                                                                                                                                                                                   | The original's port is unknown; the client takes whatever `duelData` says                                                                                                             |
| Accounts                                                                  | authenticated TLS key login, single-use front/duel tickets, launcher-bound duel entries                                                                                                                                                                                                                                                                                                                      | The launcher authenticates first; the required mod skips the stock login form. Shared IPs no longer select accounts                                                                   |
| Uploaded decks                                                            | the decks Select Deck and Edit Deck sent, ten per player by name, listed after the starter deck                                                                                                                                                                                                                                                                                                              | The client has no other upload; the original server's deck storage is unknown                                                                                                         |
| Deck check | sizes, Fusion and Synchro Monsters only in the Extra Deck, ownership per Konami id, the duel's regulation, no card the rules engine lacks, refused with `0x22CB` (client text `0x2919`: settled, `0x50ECF0` compares only `0x22CA`/`0x22CB` and never uses `error_code`) | The rules of the check are ours |
| Player names                                                              | the starting names (`--players`, default Duelist) made unique with a number; players make more at Select Player (at most 16 characters, unique ignoring case: **guess**), the host can still `add-player` / `rename`                                                                                                                                                                                         | The original's name rules are unknown; a team's leader can't be deleted and an account keeps one player (ours)                                                                        |
| CardSwap `0x1B`                                                           | each of the two words carries the face and position of the card arriving there (MSG_SWAP, Magical Hats)                                                                                                                                                                                                                                                                                                      | Read in the code (`0x4FF000` case `0x1B`); live only for Creature Swap, whose two monsters were both face-up attack                                                                   |
| Magical Hats' new places                                                  | read back from the duel after the step (the engine's `places`), matched by id                                                                                                                                                                                                                                                                                                                               | The core's message lists only old places (the core's writing order as remembered, and ocgcore-wasm's pairing); the self-played duels agree with the engine, the client never saw it   |
| Xyz materials                                                             | CardVanish as one becomes a material, a move from an empty field slot as it leaves                                                                                                                                                                                                                                                                                                                           | How Online 3 showed materials is unknown; the client lacks the Xyz Monsters tried                                                                                                     |

### Settled by the client's own code (no longer guesses)

Evidence for each is in docs/protocol.md: `pbin` 512 bytes (`0x47E490`); `avatar1` length 41; `0x21FC lang`, chat room
`region`/`status`/`type`, `0x2458.lang`, `PlayerType001/002`, `join_id`/`duel_id` (opaque echoes), ChainRun
`param1`/`param2`; `bgm`, `icon`/`lefttime`/`maintenance`, `npc_id` (omitted is right); menu bits 6/7 for Set (the icons
and `0x520F00`); card list kinds and marks (`0x4C72D0`, `CardAttr`); the NPC tabs; the `0x17C1` and `0x2D68` ids (no
reader). `cpuUse`/`notRelease` (`0x27D9`) are real but only drive the main screen's completion percentages: left out
(every card counts) because the key type isn't verified.

## Leads (outside the code)

- Old Online 3 duel logs (`players\<name>\duellogs\`) would be official duel recordings.
- The DuelEvolution Discord reportedly has Online 3 traffic captures (deferred).
- archive.org's `yo3setup_20110728.exe` is three weeks newer than our client; check its protocol version.
