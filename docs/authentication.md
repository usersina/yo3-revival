# Accounts, sign-in and the connection mod

Every game connection goes through YO3 Revival's own TLS protocol on TCP 7443, in front of the game's. The server
never picks an account by the address a client connects from, and there is no stock-client fallback. The original
game executable and `yo3.ini` stay unchanged; the game still writes its normal player files.

**Contents**

- [Accounts, sign-in and the connection mod](#accounts-sign-in-and-the-connection-mod)
  - [Accounts](#accounts)
  - [How each way of playing signs in](#how-each-way-of-playing-signs-in)
  - [Setting up a server](#setting-up-a-server)
  - [Making accounts, invites and recovery](#making-accounts-invites-and-recovery)
  - [Connections and credentials](#connections-and-credentials)
  - [Account requests](#account-requests)
  - [Passwords and throttling](#passwords-and-throttling)
  - [The connection mod and the local proxy](#the-connection-mod-and-the-local-proxy)
  - [Client evidence](#client-evidence)
  - [Retired: invitation files, handed-out keys and `/join`](#retired-invitation-files-handed-out-keys-and-join)
  - [Development checks](#development-checks)

## Accounts

An account holds up to four players with their in-game names, cards, decks and BP. There are two kinds.

**Password accounts** are how players sign in to a server: the Multiplayer tab, Host for friends, the game's own login
form and the [admin panel](guide.md#the-admin-panel). They have:

- a **User ID**: 3–16 letters or digits, unique ignoring case, never shown to other players;
- a **password**: 8–16 letters or digits, not one of a short list of common passwords and not the User ID. Both follow
  what the game's own login form lets a player type ([client internals](client-internals.md#the-games-own-login-form-34-research-r1));
- a **role**: player, moderator or admin;
- a **standing**: active, restricted or banned, with a reason and an optional end ([moderation](guide.md#moderation-and-maintenance)).
  Both refuse sign-in, by password, device or key. A ban also signs every device out and disconnects the game at once;
  a restriction lets a game already signed in play on until it logs in again;
- a **mute**, with a reason and an optional end: the player's chat lines go nowhere;
- a one-time **recovery code**, shown once at sign-up;
- up to ten **signed-in devices** ("Stay signed in").

A new password account has no players yet; the game's Select Player makes them.

**Key accounts** sign in with a random 20-character key and have no User ID or password. They are for a player's own
server and for the terminal:

- **Singleplayer** in the Desktop launcher: its local account on your own server. The launcher asks your server for
  the key at each Play (admin API, loopback only) and keeps it in the system's credential store, where the game's
  helper reads it; its private profile names the store's entry (`key_vault`). It never leaves the PC.
- **The Scripts edition on your own PC**: `add-account` makes a key account and `profile` exports its private
  profile, which `play.sh`/`play.cmd` reads. On a friend's server a Scripts player signs in with a password account
  instead (below); the key never leaves the host's PC.

Both kinds hold the same data and the same players; the admin commands find either by a player's name, and a
password account by its User ID too.

## How each way of playing signs in

| Who | How | What is kept on the player's PC |
| --- | --- | --- |
| Desktop, **Multiplayer** | User ID and password on the Multiplayer tab (operation 6 `login`); the launcher hands the helper the session in a private profile | With **Stay signed in**, a device token in the system's credential store (Windows Credential Manager, the Secret Service on Linux). Never the password |
| Desktop, **Host for friends** | Friends: as Multiplayer, with the invite link (address and identity) and an invite code the host sends. The host plays on it the same way | As Multiplayer |
| Desktop, **Singleplayer** | Your local key account on your own server | The key, in the system's credential store (entry `key:<identity>`). Only on a PC without one (a Linux PC without a Secret Service) does it stay in the launcher's private profile (`profiles/`, mode 0600 on Unix), and the launcher's log says so. A key an older launcher left in a profile moves into the store at the first start |
| **Direct start** (`yo3.exe` opened alone) | What was last played: Singleplayer's key, or Multiplayer's saved device token. With no saved sign-in, the game's own login form (User ID, password, Save Password) | `profiles/direct.json` holds the address, the identity and the credential store's entry name, never a key, token or password. On Linux the entry is copied into the Wine prefix's store, which the game's helper reads there; when that fails, the key stays in the file |
| **Scripts**, your own PC | A key profile from `add-account` and `profile` | `client.json` and `server.crt` |
| **Scripts**, a friend's server | User ID and password, once, with `play --sign-in LINK USER --profile client.json` (or `--register` with an invite code); later starts resume it. Without a saved sign-in, the game's own login form | The device token in the system's credential store (the entry Desktop uses for that server); `client.json` holds the address, the identity and that entry's name, never a key, token or password |
| The **admin panel** | A password account with the moderator or admin role, in the browser on the server's PC | A panel session cookie |

## Setting up a server

The Desktop launcher sets up Singleplayer and Host for friends by itself ([Desktop guide](launcher.md#host-friends)).
For a server on its own, follow the [command-line reference](guide.md) or the [hosting guide](hosting.md); the steps
below are the credential part.

The host makes the server's TLS identity once, with the names and addresses players will dial:

```sh
./yo3-server init-auth localhost 127.0.0.1 play.example
```

Keep `data/server.key` on the host. Players' launchers pin the identity; `data/server.crt` is public. Profiles and
private keys made by the tools are mode 0600 on Unix, and the credential directory 0700.

Then the host chooses how password accounts are made, with the `registration` [live setting](guide.md#live-settings)
(`yo3-server set registration open`), which applies at once:

| Mode | Who makes an account |
| --- | --- |
| `invite` (the default) | Anyone with an invite code from the host or a moderator |
| `open` | Anyone who reaches the server |
| `closed` | Only the host, with `add-user` |

The server requires launchers at least as new as its own release. The `server-name` and `headline` settings show in
the launcher's server list, and a higher `min-launcher VERSION` refuses
launchers older than that version, which are told to update.

Players find the server with an invite link (`yo3revival://server?address=HOST:PORT&identity=HEX&name=NAME`, which
Host for friends and `yo3-server share` make) or by typing its address in Multiplayer → **⋯** → Add a server and
comparing the identity the launcher shows with the one the host gave them. A link made for one friend also carries their
single-use invite code (`&code=ABCD-EFGH-JKLM-NPQR`): Host for friends makes one per friend, and `share --invite` prints
one; the launcher fills the code into the sign-up, and Scripts' `--register` uses it without asking.

A terminal host prints what to send, and an invite code for one sign-up that works for a week:

```sh
./yo3-server --advertise play.example share --invite
```

A Scripts player makes the account and signs in from a terminal, with the link in quotes:

```sh
./play.sh --register "yo3revival://server?address=…" Alice --profile "$PWD/client.json"
./play.sh --profile "$PWD/client.json" --game-dir "/path/to/Yu-Gi-Oh! ONLINE 3"
```

`--register` asks for the invite code (unless the link carries one), a password twice and the server's rules, prints the recovery code, then signs
in as `--sign-in SERVER USER` does: it asks for the password without echoing it, keeps the device token in the
system's credential store and writes `client.json`, which names the server, its identity and the store's entry. When
the store isn't available (a Linux PC without a Secret Service), it says so and keeps nothing: the game's own login
form asks at each start. **Delete Login Data** in the game signs this PC out. SERVER can also be an address with
`--identity HEX`.

## Making accounts, invites and recovery

These admin commands work on a running server too, through its admin API ([guide](guide.md#account-commands)):

| Command | What it does |
| --- | --- |
| `invite [--uses N] [--days D]` | An invite code (`ABCD-EFGH-JKLM-NPQR`) for N sign-ups (1 by default, 0 for any number), working for D days (for good by default). It records who issued it and which accounts used it |
| `invites` | Each invite code: whether it still works, uses left, end date, issuer and how many accounts used it |
| `add-user USER` | A password account awaiting setup. Prints a one-time setup code, valid for 24 hours. Its owner chooses a password through Desktop’s Forgot password or Scripts `--recover`, and receives their own recovery code |
| `reset-code USER` | For a player who lost both their password and recovery code, after the host or a moderator checked it is them (on Discord, for example). The code works once, for 24 hours, in the launcher's "Forgot password", and ends when the password changes |
| `set-role USER ROLE` | player, moderator or admin; finish initial password setup before promotion. Actual role changes invalidate staff cookies and outstanding reset codes |
| `share [--invite]` | What a friend needs: the address (`--advertise`, `--auth-port`), the identity and the invite link; with `--invite`, an invite code for one sign-up, for a week |
| `add-account NAME [--also B,C]` | A key account and its first players (Scripts on your own PC) |
| `profile PLAYER FILE --advertise HOST` | A key account's private profile for the Scripts helper. It never overwrites a file |
| `key WHO [--advertise HOST]` | Prints a key account's key (or gives the account one) |

Setup/reset issuance commits account state, audit and a permanent retry receipt together. The caller creates the
random code before sending it; the server keeps only its digest. `add-user` and `reset-code` print their request ID
and prepared code before sending, so a lost response can be retried with `--request-id ID --code-stdin`, feeding
the original code from a private file. No code or password belongs in command-line arguments. A retry neither
extends the expiry nor reactivates a used/replaced code. The React dashboard retains a pending code in this tab;
after confirmation it removes it on **Done**, expiry or tab closure. Receipts retain only account numbers, kind,
expiry and request ID. The response pairs that original confirmation with current availability.

First setup consumes its code and saves the password and the owner’s new recovery code atomically. Recovery
rechecks the code and credential revision after hashing, so concurrent uses, replacement and role changes cannot
reuse a stale code. Password changes and sign-out everywhere end staff cookies too.

Recovery works without email: the recovery code shown at sign-up sets a new password and is then replaced by a new
one, shown once. A reset code does the same but leaves the player's recovery code as it was. Codes are typed in any
case, with or without the dashes. Only their SHA-256 digests are kept, and their answers stay out of the server log.

The other admin commands find an account by its User ID or one of its players' names (`give-bp`, `key`,
`add-player`, `set-country`). `accounts` lists each with its User ID or "key", role, country and players.

## Connections and credentials

The server's identity is the SHA-256 of its certificate's public key (SPKI). The Desktop launcher pins it: an invite
link carries it, and a server added by address shows it first for the player to approve. Each later connection checks
the identity, the certificate's validity and the TLS handshake signature; the address may change while the identity
stays. A Scripts profile carries the identity, or names the certificate beside it (`server.crt`) and is checked
against it and its name. A self-managed certificate is valid from a day before it is made (or renewed), so a client
whose clock is behind the host's, such as a PC set to the wrong time zone, still accepts a server just set up.

Every connection starts with TLS, an 8-byte opener and one operation byte:

| Op | What | Request | Reply |
| --- | --- | --- | --- |
| 1 | Key login | protocol and DLL interface versions, a bounded launcher release string, the 20-character key | status, then a 32-byte session |
| 2 | Ticket | session, role (front, duel or HTTP) | status, then a 32-byte ticket |
| 3 | Connect | ticket, role, local duel port | status; the connection then carries the game's own frames |
| 4 | Logout | session | status |
| 5 | Discovery | — | a length and JSON: build, protocol, readiness, and for the server list its name, players online, registration mode and minimum launcher |
| 6 | Account request | a length (at most 4096) and a JSON object | a length and a JSON object ([Account requests](#account-requests)) |
| 7 | Problem report | a session token, a u32 length (at most 256 KiB) and deflated JSON (`kind`: network, relay, server-lost, game-error or report; `summary`, `launcher`, `logs`; at most 1 MiB inflated) | 0, a u8 length and the report ID (`R-XXXX-XXXX`); or 1 the session ended, 2 the account's 20 a day are sent, 3 the host turned them off, 4 not taken |

1. Rustls verifies the pinned identity (or the certificate and name of a certificate profile). TLS key-possession
   signatures are always verified. A credential-free compatibility check comes before a key login; the server refuses
   unsupported protocol and DLL interfaces before reading a key. Credentials stay out of packet logs.
2. A login (by key, password or device token) returns a random 256-bit session, valid for 12 hours. **The newest
   login wins:** an account's older session ends at once, which closes its game's connections (the game shows its own
   disconnect dialog). A crashed client never locks its owner out.
3. Each front, duel or asset connection needs a fresh random 256-bit ticket. Tickets last 30 seconds, are scoped to
   their connection role, and are consumed once. A wrong-role presentation burns the ticket too.
4. The server chooses the front account from the authenticated session. Each selected deck records that launcher's
   identity; a duel connection must match it before claiming the stock client's `player_id`. An unrelated launcher
   cannot claim it.
5. Logout ends the session, its unspent tickets and its connections. The helper logs out when the game exits if it
   signed in itself (a key or a device token); a session the Desktop launcher handed over is the launcher's, ended by
   its Sign out. Expiry also closes live connections. Restarting the server ends all sessions and tickets; devices
   stay signed in.

The session and ticket stores and simultaneous TLS connections have fixed caps (1024 sessions, 4096 tickets, 256
connections). TLS and credential handshakes have a 10-second deadline. Randomness comes from the operating system.
The game's fixed Blowfish encryption is kept inside TLS for compatibility; it is not a security boundary.

Every reply is a complete, bounded message. After a reply has been fully read and validated, failure to send the
final TLS close alert does not invalidate it. The server sends its close alert and allows up to one second for the
peer's close before dropping TCP. This avoids intermittent Windows socket resets during response cleanup.

## Account requests

Operation 6 carries one JSON object with an `"op"`. Each reply has `"ok"`, and `"error"` when it is false. Any
request must carry `"launcher"`, its version: a missing/invalid version or one older than the server release or a higher `min-launcher` gets `update_required`
with `min_launcher`.

| op | Asks for | Replies |
| --- | --- | --- |
| `register` | `user`, `password`, `rules: true`, `invite` (on an invite-only server) | `recovery_code`, shown once. The same User ID and password sent again within 15 minutes of the sign-up (its reply was lost) get a new `recovery_code` in place of the first, with no second account; another password gets `user_id_taken` and counts as a failed sign-in |
| `login` | `user`, `password`, `stay`, `device` (a label such as "Windows PC"), `device_id` (optional: the launcher's random install id, 8 to 128 characters) | `session`, `user`, and `device_token` when `stay` |
| `resume` | `device_token`, `device_id` (optional) | `session`, `user` |
| `sign_out` | `device_token`, `everywhere` | `ok`. With `everywhere`, every device and the game's session end |
| `change_password` | `user`, `password`, `new_password` | `ok`. Every device is signed out |
| `recover` | `user`, `code` (the recovery code or a reset code), `new_password` | `recovery_code` (a new one) after a recovery code; nothing after a reset code. Every device is signed out |

The errors:

| Error | When |
| --- | --- |
| `invalid_credentials` | A wrong password or an unknown User ID: one answer for both |
| `throttled` | Too many requests or failures; `retry_after` says how many seconds to wait |
| `banned`, `restricted` | With `reason`, and `until` (Unix seconds) unless it is for good |
| `maintenance` | During maintenance, for everyone but admins and moderators; with the server's `message` |
| `signed_out` | A device token that was revoked, expired or never existed |
| `invalid_recovery` | A wrong, used or expired recovery or reset code |
| `device_full` | The `games-per-device` cap: that install already runs that many games (`limit`); not for admins and moderators ([linked accounts](hosting.md#linked-accounts-and-fair-play)) |
| `invalid_user_id`, `user_id_taken` | Sign-up |
| `password_too_short`, `password_too_long`, `password_characters`, `password_too_common` | Sign-up, a password change or recovery |
| `rules_not_accepted`, `invite_required`, `invite_invalid`, `registration_closed` | Sign-up |
| `update_required` | The launcher is older than `min_launcher` |
| `bad_request`, `unknown_op`, `server_error`, `server_full` | Malformed requests; a save that failed; 1024 sessions |

A device token is 32 random bytes. The server keeps only its SHA-256 digest, a label, its creation and last-use
times and an expiry 90 days after the last use. An account keeps at most ten; the oldest goes first.

## Passwords and throttling

- **Hashing:** argon2id with m=64 MiB, t=3 and p=1, kept as a PHC string. It takes about 116 ms a hash on a Ryzen 5
  3600 core (`cargo test --release -p yo3-server --lib hash_time -- --ignored --nocapture` measures it). At most two
  hashes run at once, so a flood of logins can't exhaust memory, and an unknown User ID is checked against a dummy
  hash so it takes as long as a wrong password.
- **Per address:** a bucket of ten requests that refills one every three seconds.
- **Per User ID:** after a failed password or recovery code, the next try waits 1 second, doubling with each failure
  up to 15 minutes, and resets on success.
- The three numbers are live settings: `address-burst`, `address-refill-seconds` and `backoff-max-seconds`. There is
  no hard lockout, which an attacker could use to lock players out.
- **Logs:** the server logs each request's op, address and result, never a password, token, key or code.

## The connection mod and the local proxy

The game's helper (`yo3-launcher`, which `play.sh`/`play.cmd` and the Desktop launcher run) reads a private profile:
the server's address, name and identity (or certificate), and one credential: a key (or the name of the credential
store's entry that holds it, `key_vault`), a session the Desktop launcher signed in for, or the name of a saved device
sign-in in the credential store.

Static preflight checks the x86 game and injector, the DLL's metadata, release and interface, and the bundled file
hashes before any sign-in. The injector creates the game suspended, loads the DLL and asks its exported readiness
function to confirm all hooks initialized. Only then does the helper sign in, activate its private proxy and let the
game resume. A failed mod, identity or account check leaves the game unresumed and ends it. Direct start is the
exception: the game is already running when the helper attaches, so a refusal is shown in the game's own login dialog.

The required mod skips the original User ID/password form when the helper already has a sign-in: **ONLINE** opens
player selection. With direct start and no saved sign-in, the form stays and what the player types goes to the helper
as an operation 6 `login` (#34); the password is never written to the game's files.

The helper binds ephemeral loopback gate, front, duel and HTTP ports. The DLL only lets the game's sockets connect to
those local ports and prepends a per-launch random cookie. The local proxy requires that cookie, so another local
process can't use the proxy by finding its port. Local HTTP uses the cookie in a loopback-only path. Credentials stay
in the helper; the DLL receives only the local ports and the cookie.

The game process, the helper, the profiles and the operating-system account are trusted. This does not protect an
account from malware running as the same OS user.

The server's internal HTTP service binds loopback only. The TLS asset role cannot reach the admin API
(`/api/admin/v1/`), which also requires the private token in `data/admin.json`, even from the same machine. The Desktop
launcher and the CLI use it to manage accounts, query readiness, change settings and stop the server. Only each
operation's name is logged; every change has an audit row, without passwords or keys.

## Client evidence

The client never sends a User ID or password to the server: its login credentials stay on the front connection
object, and Delete Player's password prompt sends none either ([protocol](protocol.md#accounts)).
So the TLS transport picks the account before the original login protocol runs.

The client imports `GetPrivateProfileStringA` and `GetPrivateProfileIntA` for its configuration and
`InternetOpenUrlA` for patch/assets. The DLL replaces only the executable's import-address-table slots. Winsock is
imported by WSOCK32 ordinal: connect 4 at `0x7587D0`, send 19 at `0x7587F4`, closesocket 3 at `0x7587EC`. Both named
and ordinal imports have a Wine test executable. Socket cookies preserve the original send counts, including
partial/nonblocking sends. The DLL guards all required imports before the injector resumes the game.

The local gate uses the confirmed `0x21FE {host, port}` redirect. The server's `0x22C4` handoff is the existing shape
(`0x50F690`), with the helper's loopback duel endpoint. `0x200A {player_id}` is unchanged. The outer TLS protocol is
ours and is documented separately from Konami's reply shapes in [protocol.md](protocol.md).

## Retired: invitation files, handed-out keys and `/join`

Before the beta (v0.0.x), friends joined with what the host handed them: a Desktop invitation file (the address, the
certificate and the friend's account key in one JSON file) or a key profile, saved in the launcher as a connection.
Earlier still, a `/join` web page made accounts in a browser, and accounts could be found by the address they
connected from. The beta removed all of that (#23):

- Friends on Desktop sign in with a password account; the host sends an invite link and an invite code instead of a
  file with a key. The beta launcher neither imports invitation files nor reads a v0.0.x launcher's saved connections.
- The server has no `/join` page and no INI download; accounts found only by an address are no longer loaded (the
  server logs each one it skips), and `move-account` and the address arguments of the admin commands are gone.
- Key accounts stay for Singleplayer, the Scripts edition on your own PC and the development tools (the headless
  client and the monkeys), and keep their key. A v0.0.x key profile still plays with the Scripts helper; Scripts
  friends sign in with a password account now (`--sign-in`), so a host has no key to hand out.

Account and player ids, collections, BP, decks and social data are kept in `server.db` (SQLite,
[data and upgrades](launcher.md#your-data-and-upgrades)). Deploy the server, launcher and DLL together: old clients
are not supported. A rollback restores the previous binary and backed-up data only when the host chooses it.

## Development checks

```sh
cargo nextest run --workspace                    # tests/account_contract.rs: every account request
cargo clippy --workspace --all-targets -- -D warnings
tools/monkey/protocol-monkey.sh accounts --seed 1 --requests 1500 --par 16
tools/mod/test-dll.sh                                # private :78 / .wine-b; no game needed
export YO3_AUTH_PROFILE=/path/to/private/client.json
YO3_INSTANCE=b tools/client/vclient.sh start --trace
YO3_INSTANCE=b tools/client/vclient.sh login
```

Tests cover every account request and its errors, invite uses, devices and sign-out, the backoff, the address bucket,
the hash limit, the newest login winning, bans, the launcher version check, invalid keys and certificates, ticket
replay/expiry/role scope, logout, cross-account and cross-launcher duel claims, local cookies, the main-screen flow and
the duel handoff. The protocol monkey's `accounts` mode sends valid, mutated and garbage account requests from many
loopback addresses, then checks the server still answers a game connection.
