# Security review

The beta's security review (#39, 2026-11-16): what was reviewed, what was fixed, and what is known and still open.
It looks at the server and the launcher as an attacker on the internet sees them and as a malicious player sees
them. The player-facing summary of what is kept is the [rules and privacy page](rules-privacy.md).

Deployment update (2026-11-17, v0.1.0): public Compose/nginx testing now uses staff passwords, exact origin/host,
CSRF and socket-only proxy authority; MFA is explicitly deferred. Host-token management stays local and public backup
downloads are disabled. [Deployment boundaries](dashboard.md#separate-authority-and-listeners). The historical review
below is not a completed review of the new local Ansible setup/release image workflow or a real VPS deployment.

**Contents**

- [What was reviewed](#what-was-reviewed)
- [Fixed](#fixed)
- [Open: needs the owner](#open-needs-the-owner)
- [Known and accepted](#known-and-accepted)
- [Checked and fine](#checked-and-fine)

## What was reviewed

| Area | Code |
| --- | --- |
| TLS account requests and their throttle | `transport.rs`, `account_ops.rs`, `logins.rs`, `auth.rs`, `accounts.rs`, `yo3-tunnel` |
| The game protocol from an authenticated client: sizes, other players' ids, trade, mail, chat, shop, BP, rentals, teams | `front.rs`, `trade.rs`, `mail.rs`, `shop.rs`, `social.rs`, `rental.rs`, `teams.rs`, `duel.rs`, `yo3-proto` |
| The loopback HTTP port: admin API, panel session, CSRF and roles, `/web` pages, duel logs | `net.rs`, `api.rs`, `panel.rs`, `web.rs`, `admin.rs` |
| The launcher: local proxy and cookie, credential vault, updater, `yo3revival://` invite links, Report a problem | `yo3-launcher`, `yo3-desktop` (`updates`, `multiplayer.rs`, `reports.rs`, `bug_*`, `privileged.rs`) |
| Secrets and personal data in logs | `log.rs`, `recent.rs`, `yo3-tunnel/src/diagnostics.rs`, `tracing` calls |

This was a code review. Each fix has a test that failed before it. The first review ran no live client or VM; the
fixes of the patch check, the request pace and the outbox were checked on the private headless client (2026-11-16).

## Fixed

| Severity | Finding | Fix | Test |
| --- | --- | --- | --- |
| High | A normal Play relayed the game's patch check (`yo3/yo3.idx`) and patch files (`ygoda/patch/…`) to the server. The stock patcher writes a patch archive's entries into the game folder under their own names and replaces `yo3.exe` ([client internals](client-internals.md#the-patcher)), so any server a player joined could run code on their PC. | The launcher answers the index with the game's own `yo3.idx` on every Play, and carries only `web/`, `duellogs/` and `app/` to the server. Live (2026-11-16, headless client): a normal Play reached the map with no HTTP request at the server. | `yo3-launcher` `proxy.rs` `the_patch_check_gets_the_games_own_index_and_no_server_reaches_the_patcher` |
| Medium | A connection had no request rate: it could repeat requests that save (ignore, unfriend, team changes) under the hub lock as fast as it liked. A player who never read their socket grew their outbox with every push. | Each front and duel connection has a token bucket (`request-burst` 200, `requests-per-second` 20, live settings): past the burst requests wait, none is dropped; a 16 KiB frame costs one more. Real play peaks near 32 a second, briefly. Past `outbox-max` (4096) waiting messages, or a write stalled for 60 s, the connection ends. The game's gate is answered by the launcher itself, and the TLS handshake already has per-address limits. Checked: the protocol monkey's 120 s soak cut no connection (its 2 canary stalls happen with the limits out of reach too), and an Auto Duel between two headless clients played to its result. | `net.rs` `a_connection_past_its_burst_is_paced_not_cut`, `a_client_that_does_not_read_is_disconnected_when_its_outbox_fills`, `a_write_stalled_for_a_minute_ends_the_connection` |
| Medium | No operation deleted an account: a player who asked to leave stayed in the database, with their User ID and audit rows. | `delete-account` (API, CLI with `--yes`, the panel's **Delete account** with the User ID typed out), an admin's alone: the account, its players and everything kept for them go, its sessions and devices end. The audit log keeps one row, who deleted which User ID and when ([rules and privacy](rules-privacy.md#deleting-your-account)). | `deletion_contract.rs`, `panel_contract.rs` `an_admin_deletes_an_account_only_by_typing_its_user_id` |
| Low | The admin audit log, which names players and moderation reasons, was kept for ever. | The live setting `audit-keep-days` (default 365: a year of moderation history still shows a repeat offender; 0 keeps every row) prunes older rows at start, daily and when it changes. | `settings_contract.rs` `the_audit_log_keeps_rows_for_audit_keep_days_pruned_at_start_and_when_set` |
| Medium | A 1 MiB chat line was copied to everyone in the room, and to the queue of every player who wasn't reading. Control characters reached other players' chat windows. | A chat line is cut to 200 bytes. Control characters become spaces. | `chat_contract.rs` `a_chat_line_is_cut_to_a_line_and_loses_control_characters` |
| Medium | A password change or an account recovery signed out devices but left game sessions open for up to 12 hours. Someone who had taken the account stayed in. | Both end every session of the account. | `account_contract.rs` `a_new_password_ends_the_sessions…`, `a_recovered_account_ends_the_sessions…` |
| Medium | Sign-up hashed the password (64 MiB argon2id) before checking the invite. Sign-ups without an invite could keep both hashing slots busy and slow everyone's sign-in. | The User ID and the invite are checked first. | `account_contract.rs` `a_sign_up_without_a_usable_invite_hashes_no_password` |
| Medium | Per-address limits counted each IPv6 address on its own, so one /64 had billions of allowances. Past 100,000 addresses every request also rescanned the whole map. | Allowances belong to the IPv4 address or the IPv6 /64, for both handshakes and account requests. Full buckets are pruned at most once a second. | `logins.rs` `an_ipv6_network_shares_one_allowance`, `full_buckets_are_dropped_and_pruning_waits_a_second` |
| Medium | A muted player could still send D-Mail, which also reaches offline players. | A mute covers D-Mail. | `moderation_contract.rs` `a_muted_player_sends_no_d_mail_either` |
| Medium | A lent card still counted in the lender's own duel decks, so it was in two decks at once. | The deck check takes lent cards out, as trades already did. | `trade.rs` `a_lent_card_is_not_the_lenders_to_play…` |
| Low | A mailbox had no limit. One player could fill another's, and the game fetches all of it at once. | At most 100 player mails wait for one player. The host's own mail always arrives. | `mail_contract.rs` `a_full_mailbox_takes_no_more_player_mail` |
| Low | A team name had no length limit. It is stored, then sent in every team info, invitation, mail and search entry. | At most 32 characters. | `team_contract.rs` `a_team_name_is_a_name_not_a_page` |
| Low | Any string, up to about 4 KB, could become a key in the per-User ID backoff map, and an unknown name in `login` cost a full hash. | Names that can't be a User ID are refused without a hash or a backoff entry. | `account_contract.rs` `a_long_user_id_is_refused…`, `logins.rs` `a_name_that_is_no_user_id_keeps_no_backoff` |
| Low | A request that gave up at the 10-second deadline freed its hashing slot while its hash kept running, so more than two hashes could run at once. | The slot is held until the hash ends. | `account_contract.rs` `at_most_two_passwords_are_hashed_at_once` (unchanged) |
| Low | Host for friends turned off packet files but not packet traces, so friends' chat lines reached the host's session logs. Report a problem exports those logs. | `--no-packet-trace` too. | `hosting.rs` `the_hosted_server_has_its_own_data…` |
| Low | A malicious server's page behind the game's old website links reached the player's browser as sent, on the launcher's loopback origin, with the launch cookie in its address. | The proxy keeps only the status and body, and serves them as HTML under `Content-Security-Policy: sandbox`. | `yo3-launcher` `a_servers_web_page_reaches_the_browser_sandboxed…` |
| Info | The launcher's HTTP role allowed paths by prefix, so `/web/../api/…` passed the allowlist. The local server routes by prefix too, so it reached nothing. | `..` and `\` are refused. | `transport.rs` `launchers_reach_the_games_files_logs_and_web_pages_only` |

## Open: needs the owner

Ordered by severity. Each needs a decision or a larger change than this review should make.

1. **Medium: updates are verified only by a digest from the same GitHub response.** Anyone who can publish a release,
   such as a stolen maintainer token, can ship an update. Proposed: a detached ed25519 signature checked against a
   key built into the launcher, as the signed server list already does. This needs key handling decided.
2. **Medium: the launch cookie appears in a browser command line.** The game's old website links open
   `http://127.0.0.1:PORT/session/<cookie>/web/…` (`tools/mod/web-links-logic.h`). Under Wine another local user
   can read it in `/proc`, and it lands in browser history. Proposed: serve `web/` on the proxy without the cookie
   (it is public), or use a one-time path.
3. **Low–medium: invite links add a server without asking.** A pasted `yo3revival://server?…&name=Official…` link is
   added and selected at once, with any name. Its identity is the attacker's own, so pinning raises no warning, and
   the next sign-in sends the password there. Proposed: the trust screen with address and fingerprint, names capped
   and cleaned, and unlisted servers marked "not verified".
4. **Low–medium: BP can be farmed.** A loss pays 30 BP and a win 100, even for a duel left at once. Two players
   (one account is enough) can farm without limit. This needs a decision: a minimum number of turns, nothing for two
   players of one account, or a daily cap.
5. **Low–medium: the per-User ID backoff lets anyone lock a player out.** One wrong password every 15 minutes keeps
   a User ID waiting, and recovery uses the same backoff. Proposed: key it on User ID and network, with a higher
   ceiling across networks.
6. **Low: open registration has no account cap.** With `registration = open`, accounts can be made without limit,
   and several directory lookups are linear. The beta runs invite-only, so this matters when registration opens
   (#24). Proposed: a `max-accounts` setting and a per-network daily sign-up limit.
7. **Low: one account can hold many connections.** There is no per-user connection cap, and an authenticated front
    connection has no idle timeout after its first message.
8. **Low: other protocol points.**
    - BOOSTERPASS codes can be guessed without a throttle, so hosts should make long codes.
    - Avatar parts and titles a player sends are not checked against what they own.
    - The shop's rental takes the same card row more than once.
    - Invitations (trade, rental, team, friend) never expire, and ignore lists don't stop them.
    - A whisper keeps the `chat_type` the client sent, including the one NPC and moderator notes use.
    - The pack RNG is seeded from the clock at connection.
9. **Low: launcher, local attacker.**
    - The elevated helper runs `yo3-revival` from the user-writable bundle folder.
    - The game session token stays in `profiles/multiplayer.json` (0600) after the game ends.
    - Windows loopback ports are bound without `SO_EXCLUSIVEADDRUSE`.
    - The signed server list has no expiry, and it is read whole before its size check.
10. **Info: behind a reverse proxy every player looks like loopback.** Handshake limits are skipped for loopback and
    account requests share one bucket. The hosting guide's setup has no reverse proxy, so it is unaffected.

## Known and accepted

- **Duel logs are readable by any signed-in player who knows a log's name.** Names are
  `<end time>-<duel id>-<owner>.dlog`, so they can be guessed. A log is the replay the game itself offers in
  Duel Log. The newest 1,000 are kept.
- **Packet files and traces are on by default** for the Scripts edition and development, and older runs' packet
  files are never deleted. The hosted server's `server.toml` and Host for friends turn both off.
- **The admin API and panel trust this PC.** They answer only on loopback, and the API needs the token in
  `data/admin.json`. Anyone with a shell on the host as the server's user is the host.
- **Recovery and reset codes are compared as SHA-256 digests with `==`.** The digest of a guess reveals nothing
  about the stored code.

## Checked and fine

- **TLS and authentication**
  - A 10-second deadline covers the TLS handshake and every account request.
  - Frames are capped at 1 MiB, op 6 at 4 KiB, and HTTP heads at 8 KiB.
  - `max-connections` and per-address handshake limits are checked before TLS.
  - Tokens are 256 bits from the OS RNG. Recovery and invite codes are 80 bits, kept as digests.
  - A wrong password and an unknown User ID get the same answer in the same time (a dummy hash).
  - Tickets are single-use and expire in 30 seconds. At most two passwords are hashed at once.
- **The game protocol**
  - The server owns players and decks, and checks ownership on every player, trade, rental and shop request.
  - Purchases charge and grant under one lock, with checked arithmetic.
  - Trades re-check both offers when they run, and refuse trading with yourself.
  - Duel results come from the rules engine, never the client.
  - Hidden cards go out without their identity.
  - Deck, form and mail texts are capped. Decoding caps depth and allocations.
- **Admin API and panel**
  - Loopback only, and the token is compared in constant time.
  - The panel needs a staff password account and goes through the launcher's throttle.
  - The cookie is `HttpOnly; Secure; SameSite=Strict`, and every form needs a CSRF token plus a same-origin check.
  - The CSP allows no inline script. `X-Frame-Options: DENY` is set, and the Host header must be local.
  - Moderators can't act on staff accounts, and every change is audited.
  - `/web` pages never echo the request. Launchers reach only the game's files, duel logs and `/web`.
- **The launcher**
  - The proxy binds `127.0.0.1` and checks a 32-byte cookie in constant time.
  - The keyring holds device tokens only, never passwords.
  - Updates download over HTTPS from fixed GitHub hosts, are hashed while streaming, and are never a downgrade.
  - Archive extraction refuses links, `..`, absolute paths and reserved names.
  - Report a problem uploads nothing: it writes a local ZIP with home paths and IPs masked, for the player to look at.
  - No shell is ever built from server text.
- **Logs**
  - Account requests log the address, operation and result code, never a password, token or code.
  - Session logs pass through `redact_secrets`.
  - The panel's log view masks credentials.
  - Session logs are pruned after 14 days or past 100 MB.
