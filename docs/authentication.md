# Authenticated launcher and connection mod

The revival launcher is the only supported login path. The server accepts game traffic over TLS on TCP 7443;
there is no IP-based login or stock-client fallback. The original game executable and `yo3.ini` stay unchanged;
the game still writes its normal player files.

**Contents**

- [Authenticated launcher and connection mod](#authenticated-launcher-and-connection-mod)
  - [Setup](#setup)
  - [Connections and credentials](#connections-and-credentials)
  - [Client evidence](#client-evidence)
  - [Existing accounts and rollout](#existing-accounts-and-rollout)
  - [Development checks](#development-checks)

## Setup

The [graphical application](launcher.md) handles server startup, live account management and single-file private
invitations. It also imports existing profiles with their adjacent certificate. The command-line flow below remains
available; it uses the same TLS authentication and connection DLL.

For complete platform-specific commands, server startup and firewall setup, follow the
[host and player guide](https://github.com/usersina/yo3-revival/blob/main/docs/guide.md). The commands below illustrate
the credential flow from an extracted Linux bundle.

The host initializes a certificate with the DNS names/IPs friends will connect to, makes an account, and exports a
private profile while the server is stopped:

```sh
./yo3-server init-auth localhost 127.0.0.1 play.example
./yo3-server add-account Alice
./yo3-server profile Alice Alice.json --advertise play.example
```

Send `Alice.json` privately to Alice, together with the public `data/server.crt`, saved next to the profile as
`server.crt`. Keep `data/server.key` on the host. The profile contains the server address, expected certificate name,
public certificate filename, and the account's secret key. Profiles and server private keys created by the tools
use mode 0600 on Unix; initialization makes the credential directory private (0700), including existing backups.
On Windows keep them in a folder private to the intended user.

The player runs the launcher from the bundle:

```sh
./yo3-launcher --profile Alice.json --game-dir "/path/to/Yu-Gi-Oh! ONLINE 3"
```

On Windows use `yo3-launcher.exe` and the Windows game-folder path. On Linux install Wine and use the Wine game
folder; `--wine-prefix /path/to/prefix` selects a non-default prefix. The injector and DLL are read from the
launcher's directory, or `--injector` and `--dll` can name the development builds.

The game still shows its original login form. Its username/password fields are unused. Static preflight checks
the x86 game/injector, DLL metadata/release/interface and bundled file hashes before account authentication.
The injector creates the game suspended, loads the DLL and asks its exported readiness function to confirm all
hooks initialized. Only then does the launcher authenticate, activate its private proxy and permit the game to
resume. A failed mod, identity or account check leaves the game unresumed and terminates it. Direct stock launches
still require the separately installed loader; the current raw TLS endpoint cannot authenticate them.

## Connections and credentials

The graphical application's address discovery is credential-free. Its first-use confirmation explicitly approves
the observed identity; invitation import supplies the expected identity without another file-selection step.
The approved SHA-256 SPKI identity, certificate validity and TLS handshake signature are verified on subsequent
connections. Its address may change while retaining that identity. Legacy certificate-only CLI profiles below
continue to use the supplied trust anchor and hostname checks; application exports now also include `identity`.

1. Rustls verifies the approved server identity (or the supplied certificate/name for certificate-only CLI profiles).
   TLS key-possession signatures are always verified. A credential-free compatibility check precedes login; the
   server rejects unsupported protocol/DLL interfaces before reading an account key. Credentials stay outside packet logs.
2. Login returns a random 256-bit session capability, valid for 12 hours. Separate launcher logins have separate
   identities, including two logins to the same account. The launcher never passes keys or tokens on command lines.
3. Each front, duel or asset connection needs a fresh random 256-bit ticket. Tickets last 30 seconds, are scoped
   to their connection role, and are consumed once. A wrong-role presentation burns the ticket too.
4. The server chooses the front account from the authenticated identity. It never consults source-IP patch-key
   associations. Each selected deck records that launcher identity; a duel connection must match it before
   claiming the stock client's `player_id`. An unrelated launcher cannot claim even the same account's entry.
5. Logout invalidates the session, unspent tickets and active connections. The launcher logs out when the game exits.
   Expiry also closes live connections. Restarting the server invalidates all sessions and tickets.

The session/ticket stores and simultaneous TLS connections have fixed caps (1024 sessions, 4096 tickets, 256
connections). TLS and credential handshakes have a 10-second deadline. Randomness comes from the operating system.
The fixed Blowfish encryption is retained inside TLS for game compatibility; it is not a security boundary.

The launcher binds ephemeral loopback gate/front/duel/HTTP ports. The DLL only permits the game's socket connections
to those local ports and prepends a per-launch random cookie. The local proxy requires that cookie, so another
local process cannot use the proxy merely by finding its port. Local HTTP uses the cookie in a loopback-only path.
The account credential remains in the launcher; the DLL receives only local ports and the cookie.

The game process, launcher, credential files and operating-system account are trusted. This does not protect an
account from malware running as the same OS user. It does separate remote accounts sharing a public IP.

## Client evidence

The client imports `GetPrivateProfileStringA` and `GetPrivateProfileIntA` for its configuration and
`InternetOpenUrlA` for patch/assets. The DLL replaces only the executable's import-address-table slots. Winsock
is imported by WSOCK32 ordinal: connect 4 at `0x7587D0`, send 19 at `0x7587F4`, closesocket 3 at `0x7587EC`.
Both named and ordinal imports have a Wine test executable. Socket cookies preserve the original send counts,
including partial/nonblocking sends. The DLL guards all required imports before the injector resumes the game.

The local gate uses the confirmed `0x21FE {host, port}` redirect. The server's `0x22C4` handoff is the existing
shape (`0x50F690`), with this launcher's loopback duel endpoint. `0x200A {player_id}` is unchanged. The outer TLS
protocol is ours and is documented separately from Konami's reply shapes in
[protocol.md](https://github.com/usersina/yo3-revival/blob/main/docs/protocol.md).

## Existing accounts and rollout

Account/player ids, collections, BP, decks and social data retain their document formats inside `server.db` (SQLite).
The first start imports the old JSON files atomically and leaves those originals untouched; later writes use SQLite.
See [data and upgrades](launcher.md#your-data-and-upgrades) before restoring or downgrading. Exporting a profile
for an existing player reuses its key, or adds a key to an address-only account; no player data is moved. The old
patch URL no longer selects an account. The HTTP `/join` page and INI downloads are removed, so an IP cannot reveal
another account's key. The internal HTTP service binds loopback only; the TLS asset role cannot reach `/admin` or
`/manager`. The desktop uses the private local admin token to list/create accounts, add players, export invitations,
query readiness and shut down its server. Credential-bearing bodies and replies are not logged.

Deploy the server, launcher and DLL together. All friends need a private profile and the public certificate. Old
clients are intentionally unsupported. A rollback restores the previous binary and backed-up data only when the
host deliberately chooses it; this implementation has no downgrade switch.

## Development checks

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
tools/mod/test.sh                                # private :78 / .wine-b; no game needed
export YO3_AUTH_PROFILE=/path/to/private/client.json
YO3_INSTANCE=b tools/client/vclient.sh start --trace
YO3_INSTANCE=b tools/client/vclient.sh login
```

Tests cover shared IPs, invalid keys and certificates, wrong certificate names, ticket replay/expiry/role scope,
logout, cross-account and cross-launcher duel claims, local cookies, the main-screen flow and the duel handoff.
The protocol monkey runs through the same TLS launcher proxy; its restart refreshes the local cookie.
