# YO3 Revival: play and host from one window

Open **yo3-revival.exe** on Windows or **yo3-revival** on Linux. This is the graphical launcher and server manager;
there is no need to run a batch file, shell script, or Cargo command. Extract the whole application bundle first.
Published releases through v0.0.3 predate this application; these instructions apply to bundles built from current source.

**Contents**

- [First launch](#first-launch)
- [Play on your server](#play-on-your-server)
- [Join another server](#join-another-server)
- [Host friends](#host-friends)
- [Accounts and invitations](#accounts-and-invitations)
- [Activity and closing the app](#activity-and-closing-the-app)
- [Your data and upgrades](#your-data-and-upgrades)
- [Troubleshooting](#troubleshooting)
- [Getting the original game](#getting-the-original-game)

## First launch

Install your own copy of **Yu-Gi-Oh! ONLINE 3 ver. 1.153**. Linux players also need Wine. The application bundle
includes our server, rules engine, connection DLL and launcher, but no game files.

In **Settings**, check the detected game folder or press **Browse**. Choose the folder containing `yo3.exe`,
`yo3.idx` and `package/`, then **Save settings**. On Linux you can select a different Wine prefix too.
The application checks the game data before starting. It leaves `yo3.exe` and `yo3.ini` unchanged.
Component preflight checks the DLL's interface/release, 32-bit architecture and release manifest before sending an
account key. The game stays suspended until the injected mod acknowledges readiness and authentication succeeds.

## Play on your server

On **Play**, select **My server** and press **Play**. The application creates your local identity/account on first
use, starts your server and authenticates the game. The server starts in local-only mode by default.
You can also start it separately from **My server → Start server**.

In the game, press **ONLINE**, dismiss the original login form and choose your player. Its username/password fields
are unused; the launcher already authenticated your account. Open **Main Menu → Edit Deck** once to save your starting
decks, then visit **Kaiba Corporation** for a computer opponent or use **Auto Duel** with a friend.

## Join another server

Hosting and playing are independent: you can leave your own server running while you play on someone else's.

The easiest route is **Play → Import invitation**. Select the private invitation from the host, then **Save connection**.
It includes the address, account access key and public certificate in one file. Existing `client.json` profiles work
with their adjacent `server.crt` too.

Alternatively, choose **Add by address** and enter:

- A name for the connection.
- The host's DNS name or IP, optionally with a port (default **7443**).
- The account access key supplied by the host.

Press **Discover server**. The application observes the host's identity without sending account credentials and shows
its release, protocol and required connection-mod interface. Compare the identity code with the host, then approve
it and save the connection. The stored identity is checked on later connections. An invitation already carries the
expected identity, so its normal import flow needs no separate certificate file. `yo3://host:port` links work too.

Manual certificate import is under **Advanced**. Original certificate-only profiles/invitations still import with
their adjacent certificate; the application derives the trusted identity from it and preserves the account key.

Save the connection, select it from the server list, and use **Test connection** or **Play**. Saved connections keep
separate credentials even when accounts use the same host or router. Pinned connections verify the approved public
key, certificate lifetime and TLS signature; certificate-only command-line profiles retain hostname/CA verification.
An address alone does not grant an account or administration access.

## Host friends

Before starting a new hosted server, open **Settings**:

1. Enable **Allow friends to connect to my server**.
2. Set the address friends will reach, such as `192.168.1.50:7443` or your VPN/DNS address.
3. Save, then use **My server → Start server**.

Allow the chosen TCP port through the host firewall and route/forward it through your VPN/router if necessary.
The application does not open firewall ports or configure your router automatically. Its internal admin service
listens only on loopback; never forward that port.

If the address changes after you created the identity: stop your server, save the new address, then choose
**Update certificate for saved address** and confirm. This keeps the private key and account data, backs up the previous
certificate and issues one covering the saved name/IP. Export updated invitations afterward.

**My server** shows the local server's status, connected players and duel count. Those controls always manage your
own server, including while a remote connection is selected on Play.

## Accounts and invitations

With your server running, open **Accounts**. You can create an account, add up to four players to it, select the account
for local play, and **Export private invitation**. Changes go through the running server and commit to SQLite;
you do not need to disconnect players to add a friend.

Give each friend their own account and send its invitation privately. An invitation contains an account key, so do
not post it publicly or attach it to a bug report. Export refuses to overwrite an existing file; choose a new filename.

## Activity and closing the app

**Activity** shows either your server's logs or launcher/game diagnostics. Filter the text, pause scrolling, copy
it, or export it. The display keeps a bounded tail instead of loading an unlimited log into memory. Account keys,
TLS session tokens and private server keys are excluded from normal diagnostic output.

**Close game** ends the authenticated game session. It leaves hosting active. Closing the application asks whether
to keep hosting or stop the server; an active game closes in either case. Reopening the manager reconnects to its
running local server. **Stop server** disconnects players, so finish their duels first.

## Your data and upgrades

The default application folder is `%LOCALAPPDATA%\YO3 Revival` on Windows and
`${XDG_DATA_HOME:-~/.local/share}/yo3-revival` on Linux. **Settings → Open application data** opens it.

- `launcher.db` holds preferences and saved connections, including their private account credentials.
- `server/server.db` holds the hosted server's persistent game state.
- `server/server.crt` and `server/server.key` are its public certificate and private TLS key.
- `logs/` holds diagnostic logs; `profiles/` contains private launcher profiles.

To use an existing server's data, stop that server and select its data folder in Settings before starting the manager's
server. The upgrade imports accounts, collections, BP, decks, records, social data, mail, teams, events and rentals into
SQLite in one transaction. IDs and credentials remain intact. Original JSON files are retained and are **not** used as
live storage after import. A failed import reports an error and can be retried after repairing its source.

The first SQLite schema preserves the existing domain serializers as transactional documents. It changes storage,
not the client protocol. Live account administration rolls back its memory changes if persistence fails.

At startup the server makes consistent SQLite backups under its data folder's `backups/`. Separately back up the
entire data folder securely: automatic database backups do not include `server.key`. Keep players' local
`Documents/Yu-Gi-Oh! ONLINE 3` too, because editor-only deck saves stay there.

Restore a database backup only with the server stopped. A migrated directory with a missing database reports an error
rather than silently reimporting stale JSON. Returning to an older server requires the original pre-upgrade JSON
snapshot; it cannot read SQLite or retain subsequent database-only changes.

## Troubleshooting

- **Incomplete game:** choose a full supported installation rather than an installer or a shortcut.
- **Server executable/engine missing:** extract the entire application bundle, including `engine/` and its Node runtime.
- **Cannot connect:** check the address/port, that the host is running and its firewall/VPN permits the connection.
- **Identity error:** confirm changes with the host or import their updated invitation. Automatic certificate renewal
  with the same identity does not require approving a new key. An expired certificate still needs renewal by the host.
- **Authentication refused:** the key belongs to an account on that specific server; import the correct invitation.
- **Game startup fails:** see **Activity → Launcher & game**. The supported client and required DLL must match.
- **Account name refused:** use a unique player name, 1–16 characters, and at most four players per account.
- **Long session disconnected:** launcher sessions last 12 hours; close and launch the game again.
- **Windows warnings:** the executables and injection DLL are unsigned. See the
  [Windows notes](guide.md#windows-smartscreen-and-antivirus-warnings).

The [command-line guide](guide.md) remains available for developers and unattended hosting, and
[authentication.md](authentication.md) explains the TLS/launcher design.

## Getting the original game

Bring a supported original installation; the application does not download or distribute the game.
A third-party archive reference is the [MyAbandonware game listing](https://www.myabandonware.com/game/yu-gi-oh-online-3-duel-accelerator-qy4).
This reference stays in documentation, separate from the launcher's Play and hosting actions.
