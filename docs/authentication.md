# Accounts, sign-in and the connection mod

The revival launcher is the only supported login path. The server accepts game traffic over TLS on TCP 7443;
there is no IP-based login or stock-client fallback. The original game executable and `yo3.ini` stay unchanged;
the game still writes its normal player files.

**Contents**

- [Accounts, sign-in and the connection mod](#accounts-sign-in-and-the-connection-mod)
  - [Accounts](#accounts)
  - [Setup](#setup)
  - [Making accounts, invites and recovery](#making-accounts-invites-and-recovery)
  - [Connections and credentials](#connections-and-credentials)
  - [Account requests](#account-requests)
  - [Passwords and throttling](#passwords-and-throttling)
  - [Client evidence](#client-evidence)
  - [Existing accounts and rollout](#existing-accounts-and-rollout)
  - [Development checks](#development-checks)

## Accounts

An account holds up to four players with their in-game names, cards, decks and BP. There are two kinds:

- **Password accounts** (the beta, #23). They have:
  - a **User ID**: 3–16 letters, digits, `_` or `-`, unique ignoring case, never shown to other players;
  - a **password**: 8–128 characters, not one of a short list of common passwords and not the User ID;
  - a **role**: player, moderator or admin;
  - a **standing**: active, restricted or banned, with a reason and an optional end. A ban keeps the player out;
    a restriction lets them sign in and limits what they can do in the game (#26);
  - a one-time **recovery code**, shown once at sign-up;
  - up to ten **signed-in devices** ("stay signed in").
  A new password account has no players yet; the game's Select Player makes them.
- **Key accounts**. A 20-character key signs in. The launcher's own Singleplayer account uses one, and so do the
  Scripts edition's private profiles until the launcher's sign-in form (#32) replaces them.

The address a client connects from never selects an account.

## Setup

The [graphical application](launcher.md) handles server startup, live account management and single-file private
invitations. For complete platform-specific commands, server startup and firewall setup, follow the
[command-line reference](guide.md). The commands below illustrate the credential flow from an extracted Linux bundle.

The host initializes a certificate with the DNS names/IPs friends will connect to, once:

```sh
./yo3-server init-auth localhost 127.0.0.1 play.example
```

Then it chooses how accounts are made, with `--registration` when the server starts:

| Mode | Who makes an account |
| --- | --- |
| `invite` (the default) | Anyone with an invite code from the host or a moderator |
| `open` | Anyone who reaches the server |
| `closed` | Only the host, with `add-user` |

`--name` sets the server's name in the launcher's server list and `--min-launcher VERSION` refuses launchers older
than that version, which are told to update.

A key account still works as before: `add-account Alice` makes it and `profile Alice Alice.json --advertise
play.example` exports its private profile. Send `Alice.json` privately to Alice with the public `data/server.crt`,
saved next to the profile as `server.crt`. Keep `data/server.key` on the host. Profiles and server private keys
created by the tools use mode 0600 on Unix; initialization makes the credential directory private (0700).

The player runs the launcher from the bundle:

```sh
./yo3-launcher --profile Alice.json --game-dir "/path/to/Yu-Gi-Oh! ONLINE 3"
```

On Windows use `yo3-launcher.exe` and the Windows game-folder path. On Linux install Wine and use the Wine game
folder; `--wine-prefix /path/to/prefix` selects a non-default prefix. The injector and DLL are read from the
launcher's directory, or `--injector` and `--dll` can name the development builds.

The required mod skips the original username/password form: **ONLINE** opens player selection. The normal
connection and error handlers remain in use. Static preflight checks the x86 game/injector, DLL
metadata/release/interface and bundled file hashes before account authentication. The injector creates the game
suspended, loads the DLL and asks its exported readiness function to confirm all hooks initialized. Only then does
the launcher authenticate, activate its private proxy and permit the game to resume. A failed mod, identity or
account check leaves the game unresumed and terminates it.

## Making accounts, invites and recovery

These admin commands work on a running server too (through its loopback admin port and token, like the BP ones):

| Command | What it does |
| --- | --- |
| `invite [--uses N] [--days D]` | An invite code (`ABCD-EFGH-JKLM-NPQR`) for N sign-ups (1 by default, 0 for any number), working for D days (for good by default). It records who issued it and which accounts used it |
| `invites` | Each invite code: whether it still works, uses left, end date, issuer and how many accounts used it |
| `add-user USER` | A password account made by the host. Prints its first password and recovery code to hand to the player, who changes the password in the launcher |
| `reset-code USER` | For a player who lost both their password and recovery code, after the host or a moderator checked it is them (on Discord, for example). The code works once, for 24 hours, in the launcher's "Forgot password", and ends when the password changes |
| `set-role USER ROLE` | player, moderator or admin |

Recovery works without email: the recovery code shown at sign-up sets a new password and is then replaced by a new
one, shown once. A reset code does the same but leaves the player's recovery code as it was. Codes are typed in any
case, with or without the dashes. Only their SHA-256 digests are kept, and their answers stay out of the server log.

The other admin commands find an account by its User ID or one of its players' names (`give-bp`, `key`,
`add-player`, `set-country`). Run `accounts` to list them with their User ID, role, country and players.

## Connections and credentials

The graphical application's address discovery is credential-free. Its first-use confirmation explicitly approves
the observed identity; invitation import supplies the expected identity without another file-selection step.
The approved SHA-256 SPKI identity, certificate validity and TLS handshake signature are verified on subsequent
connections. Its address may change while retaining that identity. Certificate-only CLI profiles use the supplied
trust anchor and hostname checks.

Every connection starts with TLS, an 8-byte opener and one operation byte:

| Op | What | Request | Reply |
| --- | --- | --- | --- |
| 1 | Key login | protocol and DLL interface versions, the 20-character key | status, then a 32-byte session |
| 2 | Ticket | session, role (front, duel or HTTP) | status, then a 32-byte ticket |
| 3 | Connect | ticket, role, local duel port | status; the connection then carries the game's own frames |
| 4 | Logout | session | status |
| 5 | Discovery | — | a length and JSON: build, protocol, readiness, and for the server list its name, players online, registration mode and minimum launcher |
| 6 | Account request | a length (at most 4096) and a JSON object | a length and a JSON object ([Account requests](#account-requests)) |

1. Rustls verifies the approved server identity (or the supplied certificate/name for certificate-only profiles).
   TLS key-possession signatures are always verified. A credential-free compatibility check precedes a key login;
   the server rejects unsupported protocol/DLL interfaces before reading a key. Credentials stay outside packet logs.
2. A login (by key, password or device) returns a random 256-bit session, valid for 12 hours. **The newest login
   wins:** an account's older session ends at once, which closes its game's connections (the game shows its own
   disconnect dialog). A crashed client never locks its owner out.
3. Each front, duel or asset connection needs a fresh random 256-bit ticket. Tickets last 30 seconds, are scoped
   to their connection role, and are consumed once. A wrong-role presentation burns the ticket too.
4. The server chooses the front account from the authenticated identity. Each selected deck records that launcher
   identity; a duel connection must match it before claiming the stock client's `player_id`. An unrelated launcher
   cannot claim it.
5. Logout invalidates the session, unspent tickets and active connections. The launcher logs out when the game exits.
   Expiry also closes live connections. Restarting the server invalidates all sessions and tickets; devices stay
   signed in.

The session/ticket stores and simultaneous TLS connections have fixed caps (1024 sessions, 4096 tickets, 256
connections). TLS and credential handshakes have a 10-second deadline. Randomness comes from the operating system.
The fixed Blowfish encryption is retained inside TLS for game compatibility; it is not a security boundary.

Every reply is a complete, bounded message. After a reply has been fully read and validated, failure to send the
final TLS close alert does not invalidate it. The server sends its close alert and allows up to one second for the
peer's close before dropping TCP. This avoids intermittent Windows socket resets during response cleanup.

The launcher binds ephemeral loopback gate/front/duel/HTTP ports. The DLL only permits the game's socket connections
to those local ports and prepends a per-launch random cookie. The local proxy requires that cookie, so another
local process cannot use the proxy merely by finding its port. Local HTTP uses the cookie in a loopback-only path.
Credentials remain in the launcher; the DLL receives only local ports and the cookie.

The game process, launcher, credential files and operating-system account are trusted. This does not protect an
account from malware running as the same OS user.

## Account requests

Operation 6 carries one JSON object with an `"op"`. Each reply has `"ok"`, and `"error"` when it is false. Any
request may carry `"launcher"`, its version: one older than the server's `--min-launcher` gets `update_required`
with `min_launcher`.

| op | Asks for | Replies |
| --- | --- | --- |
| `register` | `user`, `password`, `rules: true`, `invite` (on an invite-only server) | `recovery_code`, shown once |
| `login` | `user`, `password`, `stay`, `device` (a label such as "Windows PC") | `session`, `user`, and `device_token` when `stay` |
| `resume` | `device_token` | `session`, `user` |
| `sign_out` | `device_token`, `everywhere` | `ok`. With `everywhere`, every device and the game's session end |
| `change_password` | `user`, `password`, `new_password` | `ok`. Every device is signed out |
| `recover` | `user`, `code` (the recovery code or a reset code), `new_password` | `recovery_code` (a new one) after a recovery code; nothing after a reset code. Every device is signed out |

The errors:

| Error | When |
| --- | --- |
| `invalid_credentials` | A wrong password or an unknown User ID: one answer for both |
| `throttled` | Too many requests or failures; `retry_after` says how many seconds to wait |
| `banned` | With `reason`, and `until` (Unix seconds) unless it is for good |
| `signed_out` | A device token that was revoked, expired or never existed |
| `invalid_recovery` | A wrong, used or expired recovery or reset code |
| `invalid_user_id`, `user_id_taken` | Sign-up |
| `password_too_short`, `password_too_long`, `password_too_common` | Sign-up, a password change or recovery |
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
- **Per User ID:** after a failed password or recovery code, the next try waits 1 second, doubling with each
  failure up to 15 minutes, and resets on success. There is no hard lockout, which an attacker could use to lock
  players out.
- **Logs:** the server logs each request's op, address and result, never a password, token or code.

## Client evidence

The client imports `GetPrivateProfileStringA` and `GetPrivateProfileIntA` for its configuration and
`InternetOpenUrlA` for patch/assets. The DLL replaces only the executable's import-address-table slots. Winsock
is imported by WSOCK32 ordinal: connect 4 at `0x7587D0`, send 19 at `0x7587F4`, closesocket 3 at `0x7587EC`.
Both named and ordinal imports have a Wine test executable. Socket cookies preserve the original send counts,
including partial/nonblocking sends. The DLL guards all required imports before the injector resumes the game.

The local gate uses the confirmed `0x21FE {host, port}` redirect. The server's `0x22C4` handoff is the existing
shape (`0x50F690`), with this launcher's loopback duel endpoint. `0x200A {player_id}` is unchanged. The outer TLS
protocol is ours and is documented separately from Konami's reply shapes in [protocol.md](protocol.md).

## Existing accounts and rollout

Account/player ids, collections, BP, decks and social data are kept in `server.db` (SQLite,
[data and upgrades](launcher.md#your-data-and-upgrades)). Accounts found only by the address they connected from
are no longer loaded (the server logs each one it skips): they had no way in since the TLS transport, and #23
removed what was left of that login, with `move-account` and address arguments to the admin commands. Key accounts
keep their key.

The internal HTTP service binds loopback only; the TLS asset role cannot reach `/admin` or `/manager`. The desktop
uses the private local admin token to list/create accounts, add players, export invitations, query readiness and
shut down its server. Credential-bearing bodies and replies are not logged.

Deploy the server, launcher and DLL together. Old clients are intentionally unsupported. A rollback restores the
previous binary and backed-up data only when the host deliberately chooses it.

## Development checks

```sh
cargo nextest run --workspace                    # tests/account_contract.rs: every account request
cargo clippy --workspace --all-targets -- -D warnings
tools/monkey/protocol-monkey.sh accounts --seed 1 --requests 1500 --par 16
tools/mod/test.sh                                # private :78 / .wine-b; no game needed
export YO3_AUTH_PROFILE=/path/to/private/client.json
YO3_INSTANCE=b tools/client/vclient.sh start --trace
YO3_INSTANCE=b tools/client/vclient.sh login
```

Tests cover every account request and its errors, invite uses, devices and sign-out, the backoff, the address
bucket, the hash limit, the newest login winning, bans, the launcher version check, invalid keys and certificates,
ticket replay/expiry/role scope, logout, cross-account and cross-launcher duel claims, local cookies, the main-screen
flow and the duel handoff. The protocol monkey's `accounts` mode sends valid, mutated and garbage account requests
from many loopback addresses, then checks the server still answers a game connection.
