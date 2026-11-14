# Protocol catalog

The client (`yo3.exe`) is the spec. Every row cites its evidence: a client address, a line of the client's own
trace (`File.cpp Lnnn`), a recorded packet, or the **field tracer** (`vclient.sh start --trace`, then
`tools/tracer/summarize.py`), which lists every field the client actually looked up in each reply. **Status** is one
of:

- **confirmed**: the shape is proven, and so is the meaning where one is given
- **shape**: the client accepts the shape we send, but the meaning or contents are unknown
- **guess**: we picked a value; revisit
- **open**: not answered yet

The contract tests in `crates/yo3-server/tests/front_contract.rs` enforce the confirmed shapes.

**Contents**

- [Protocol catalog](#protocol-catalog)
  - [Authenticated transport](#authenticated-transport)
  - [Wire format](#wire-format)
  - [Gate (TCP 5730)](#gate-tcp-5730)
  - [Front (TCP 1213)](#front-tcp-1213)
    - [Login (dispatcher `0x469C00`, `LoginWindow.cpp`)](#login-dispatcher-0x469c00-loginwindowcpp)
    - [Main screen (MainGamen polls `0x472EDC…`, global dispatcher `0x4F4A00`)](#main-screen-maingamen-polls-0x472edc-global-dispatcher-0x4f4a00)
    - [Info and chat windows](#info-and-chat-windows)
    - [Chat (ChatWindow dispatcher `0x435950`)](#chat-chatwindow-dispatcher-0x435950)
    - [Friends and the ignore list (community agent `0x4A4EE0`)](#friends-and-the-ignore-list-community-agent-0x4a4ee0)
    - [D-Mail (MailWindow `0x4AC6B0`, `dmail_main.swf`)](#d-mail-mailwindow-0x4ac6b0-dmail_mainswf)
    - [Teams (the community agent's modes 5–12, Team window `0x4BBEC0`, `main_teaminfo.swf`)](#teams-the-community-agents-modes-512-team-window-0x4bbec0-main_teaminfoswf)
    - [Card trade between players (agent step `0x4A91D0`, trade scene `CardTrade.cpp`, handler `0x4248F0`)](#card-trade-between-players-agent-step-0x4a91d0-trade-scene-cardtradecpp-handler-0x4248f0)
    - [Card rental between players (agent steps `0x4A9D80` and `0x4AA430`, rental scene `CardRental.cpp`, handler `0x41C7E0`)](#card-rental-between-players-agent-steps-0x4a9d80-and-0x4aa430-rental-scene-cardrentalcpp-handler-0x41c7e0)
    - [Country (flags)](#country-flags)
    - [Manner ratings (`dlg_manner.swf`, DuelResult `0x512A40`, dialog `0x4C2A10`/`0x4C3F20`)](#manner-ratings-dlg_mannerswf-duelresult-0x512a40-dialog-0x4c2a100x4c3f20)
    - [Main Menu screens (MainGamen's menu `0x46FA60`)](#main-menu-screens-maingamens-menu-0x46fa60)
    - [Shop (`shopmain.cpp`, handler `0x48AA60`)](#shop-shopmaincpp-handler-0x48aa60)
    - [Events (TournamentScene, UserEventScene)](#events-tournamentscene-usereventscene)
    - [Decks (deck editor `DeckEditTest`, handler `0x43FC60`)](#decks-deck-editor-deckedittest-handler-0x43fc60)
    - [NPC duels and duel start](#npc-duels-and-duel-start)
    - [Duels between players (automatic matching)](#duels-between-players-automatic-matching)
  - [Duel connection](#duel-connection)
    - [Duel log files (`0x512070`, `0x498700`, `0x498B10`, `duellog.rs`)](#duel-log-files-0x512070-0x498700-0x498b10-duellogrs)
  - [Accounts](#accounts)
  - [Fields the client reads that we leave out](#fields-the-client-reads-that-we-leave-out)
  - [Requests with a timeout](#requests-with-a-timeout)
  - [Families seen in the client, not mapped yet](#families-seen-in-the-client-not-mapped-yet)

## Authenticated transport

The server exposes TLS on TCP 7443. The launcher and DLL are required; there is no raw gate/front/duel listener.
The numbered gate/front headings below retain their historical port names for the client's original protocol.
Today the gate runs locally in the launcher and front/duel frames are carried inside TLS. **Status: confirmed live**
with two accounts on one IP, private displays :78/:79, named and ordinal DLL tests, and `auth_flow`/launcher `proxy`.

The outer handshake is our own protocol, not inferred Konami fields. Inside validated TLS each connection starts
with ASCII `YO3TLS02` and a one-byte operation. Numbers are big-endian; tokens are opaque 32-byte random values.
The compiled protocol version is 2 and the required DLL interface is 2. Old transports are refused.

| Operation | Payload after operation                              | Reply                                                    | Meaning                                                                                                    |
| --------- | ---------------------------------------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 1 login   | u16 protocol, u16 DLL interface, 20-byte account key | status byte, then session token on success               | incompatible versions refused before reading the key; session lasts 12 hours                               |
| 2 ticket  | session token, role byte                             | status byte, then ticket on success                      | role 1 front, 2 duel, 3 assets; ticket lasts 30 seconds                                                    |
| 3 connect | ticket, role, u16 launcher-local duel port           | status byte, then stock game frames (or HTTP for role 3) | consume ticket once, including failed wrong-role attempts; a front needs a nonzero local duel port         |
| 4 logout  | session token                                        | status byte                                              | invalidate session, queued tickets and live connections; retry is harmless                                 |
| 5 info    | none                                                 | u16 byte length, UTF-8 JSON (1–4096 bytes)               | credential-free build, commit, protocol, DLL interface, game version, readiness and generic engine problem |

`tests/metadata_contract.rs` verifies discovery without an account, compatibility independent of patch-release labels,
wrong host names/keys, certificate renewal preserving the identity, expired certificates and bounded metadata.
Normal account endpoints verify an invitation/user-approved SHA-256 hash of the certificate's SPKI, its lifetime and
the TLS handshake signature. Address discovery proves possession of the observed key but is **untrusted** until the
user approves it or compares it to the host's invitation. It can issue only the credential-free info operation and
does not save trust. Certificate-only CLI profiles retain their existing CA/name verification behavior.

The local injector's own startup protocol is `YO3_MOD_READY` (LF/CRLF accepted) on stdout after the required mod's
`Yo3ModReady` confirms DLL interface 2; the game stays suspended. The parent authenticates and writes `resume\n`
to stdin only after that acknowledgement. EOF/failure kills the suspended child. `.yo3` PE metadata carries the
mod interface, release and commit, while `bundle.json` inventories release hashes. Tests: launcher `components`,
`desktop_control`, `tools/mod/test.sh`; this is our bootstrap protocol, not a game reply shape.

Opt-in direct launch uses a game-local WINMM proxy (the seven imports in `tools/mod/loader.c`), leaving the executable
unchanged. Its IAT wrappers defer process/UI/network work until a relevant INI/WinInet/connect call, outside DllMain.
The user's private registry-selected config pins the connection DLL hash and private profile path. A Windows helper
attaches to the verified game PID/executable, prepares local proxies, then publishes a bounded readiness JSON with
helper/game PID, ABI, ports and fresh cookie. The loader verifies it, loads the DLL, checks `Yo3ModReady`, and
atomically acknowledges that cookie. Only then does the helper authenticate and publish `authenticated`; game exit
revokes the session. Stale/mismatched acknowledgements and existing readiness files are refused. Managed injection
skips the automatic bootstrap. Tests: launcher `attach`, desktop `direct_install`, `tools/mod/test-loader.sh` and
the live fixture `tools/mod/test-direct.sh`. These fields belong to our private bootstrap, never stock login replies.

Status 0 succeeds, 1 refuses. Handshakes time out after 10 seconds. Authentication bytes never enter the game packet
logger. The asset role permits public patch/app/duel-log routes only; `/admin` is inaccessible through TLS. Internal
HTTP listens on loopback. New keys and tokens use OS cryptographic randomness. Store/connection caps are documented
in [authentication.md](authentication.md).

The DLL overrides the executable's INI API imports and guards its Winsock connect/send/close imports. The real
WSOCK32 slots use ordinals 4/19/3 (`0x7587D0`/`0x7587F4`/`0x7587EC`); `InternetOpenUrlA` redirects asset downloads.
Every local socket begins with a per-launch cookie, stripped before any game frame. Local HTTP uses a cookie path.
The local `0x21FE` redirect and server `0x22C4` handoff (`0x50F690`) name this launcher's loopback ports. The
`0x200A {player_id}` entry must match the authenticated identity attached when its front selected the deck.

## Wire format

- Frame: `u32 BE (len(body) ^ 0x77777777)` + body.
- Body: Blowfish-ECB, key `"12345678"`, each 32-bit word little-endian (`BlowfishLE`), zero-padded to 8 bytes.
- Message: `u8 tag | u16 nfields | (u8 namelen, name, u8 vtype, value)… | u16 nitems | items…`, big-endian.
  - Value types:

    | vtype | Kind   | Encoding                                                     |
    | ----- | ------ | ------------------------------------------------------------ |
    | 5 / 6 | u32    | 4 bytes                                                      |
    | 7 / 9 | text   | `u32 len` + bytes; the client stops reading at the first NUL |
    | 8     | binary | `u32 len` + bytes; NULs are kept                             |

  - Tags: replies and requests use 4, list records use 3. The message ID is the `request` field.
  - The client builds each item as an object chosen by its tag (factory `0x531B90`, a jump table on tags 0–10):
    3, 5 and 7 make a plain dataset, 4 a request object, **6 a `YO3Deck`** (`0x414240`), 8 and 9 other types,
    10 the chat item. Code that treats an item as a deck needs tag 6; a tag-3 deck aborts the client (`0x413F63`).
- Every connection opens with a bare version string: `00 12 "2010.03.23 skelton"`.
- The client's `.y3d/.y3a/.y3r` files use the same frame and message layout.
- Client-side failures:
  - An unanswered poll shows "There is no response from the server. (_decimal id_)". For example, `(8,306)` is
    `0x2072`.
  - A missing field that the client reads without a NULL check is a crash: Wine's "Unhandled page fault" at the
    reading address.
- Formatted date/time strings sent to the client are **JST** (UTC+9). Duel-log `end_time` is a Unix timestamp
  rendered in the client's local time zone.

## Gate (TCP 5730)

| Request | Reply    | Fields                                 | Evidence                    | Status    |
| ------- | -------- | -------------------------------------- | --------------------------- | --------- |
| opener  | `0x21FE` | `host` text, `port` u32 (front server) | `LoginWindow.cpp L303/L314` | confirmed |

The launcher also gives the connection DLL the server's id (`YO3_SERVER_ID`), which picks the game's local data folder
(see the roadmap's "Local profile collisions" and `tools/mod/documents.h`); nothing of it reaches the server.

The local gate keeps its socket open after the redirect until the client closes it or the launcher proxy stops.
Immediate EOF races the native receive thread during automatic login and reaches a null error logger at
`0x535210` (`0x82D214` is null). The native redirect handler sets its disconnect-suppression flag only at
`0x469AF7`. Status: **confirmed live**; `tests/proxy.rs` protects the socket lifetime and cleanup.

Game framing has a 180 s idle limit before the first decoded message, while waiting for bytes inside a frame,
and between duel messages (`net::Idle`). Once it has spoken, the front connection may remain quiet: real-client
logs include map-idle gaps of 650 s and 3000 s. Duel connections heartbeat `0x2724` every 30 s. These are server
limits, separate from TLS handshake deadlines and launcher-session expiry. Messages nest at most 16 levels deep
(the client's own go about 3); a deeper one is an undecodable packet (`Error::TooDeep`).

## Front (TCP 1213)

Until a player is selected (`0x1F4C`), the front answers only the login and Select Player requests (`0x1F47`–`0x1F4C`)
and the heartbeat; every other request is ignored (nothing in the client sends one earlier: seen in the packet logs). A
`0x1F4C` for a player the account doesn't have takes the old selection offline (no reply, as before).

### Login (dispatcher `0x469C00`, `LoginWindow.cpp`)

The required mod skips the unused username/password form after **ONLINE**, starting the same native gate connection
at `0x46B5FF…0x46B62C` in place of the one-shot `StartLogin` call at `0x46964B`. The wire requests, authenticated
account identity and player-selection handler stay unchanged. See [the native flow](client-internals.md#skipping-the-stock-login-form).
Status: **confirmed (revival behavior)**.

The stock request named "auth" carries no credential: the opener is a version string, `0x1F47` is bare and
`0x1F48` carries only `lang` (live packets). Identity is established by the authenticated outer transport before
`FrontSession` is constructed. The fixed Blowfish key provides game compatibility; TLS protects the connection.

| Request                                                                                            | Reply                                     | Fields                                                                                                                                                                                                                                                                                                                                                                                                                                        | Evidence                                                                                                                                                                                                                                                                                                  | Status                                                                                                                                     |
| -------------------------------------------------------------------------------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| opener                                                                                             | `0x219F`                                  | none                                                                                                                                                                                                                                                                                                                                                                                                                                          | "front server : version valid." L370                                                                                                                                                                                                                                                                      | confirmed                                                                                                                                  |
| `0x1F47` auth                                                                                      | `0x219A`                                  | `regulation_id`, `user_id`, `uname`, `dpoint`                                                                                                                                                                                                                                                                                                                                                                                                 | "user auth success" L395                                                                                                                                                                                                                                                                                  | confirmed                                                                                                                                  |
| `0x1F48` login `{lang}`                                                                            | `0x2201`                                  | top: `regulation_id`; item[0]: `user_id`, `uname`, `dpoint`, `premium` (copied into UserData by `0x51B570`; non-zero opens every NPC, `0x4BF520`, and Form Team and Create Event)                                                                                                                                                                                                                                                             | L449, `uid : [1001]` L457; live: all 25 NPCs enabled with `premium` 1                                                                                                                                                                                                                                     | confirmed; `premium` 1 for everyone is our choice                                                                                          |
| `0x1F49` player list                                                                               | `0x2260`                                  | items = player records (below)                                                                                                                                                                                                                                                                                                                                                                                                                | "MyPlayerList" L509, loader `0x51C260`                                                                                                                                                                                                                                                                    | confirmed                                                                                                                                  |
| `0x1F4C` select `{player_id}`                                                                      | `0x2266`                                  | none                                                                                                                                                                                                                                                                                                                                                                                                                                          | "my player use success" L577                                                                                                                                                                                                                                                                              | confirmed                                                                                                                                  |
| `0x1F4A` create `{pindex, pname}` (an empty slot's Create Player: name dialog, builder `0x4F2D50`) | `0x2262` (`0x2263`)                       | none                                                                                                                                                                                                                                                                                                                                                                                                                                          | "my player sub success" `LoginWindow.cpp L608`: the client asks `0x1F49` again, finds the new player and selects it (`0x1F4C`), then opens Avatar Settings; `0x2263` shows `$10525` ("This Player Name is already in use. Please create another.") and returns to the avatar screen; live: Joey in slot 3 | confirmed; name rules (unique ignoring case, 1-16 characters, no control characters) and the full-account/slot-taken errors **guess**      |
| `0x1F4B` delete `{player_id}` (Delete, after a warning and a password prompt; builder `0x4F2EB0`)  | `0x2264` (`0x2265`)                       | none                                                                                                                                                                                                                                                                                                                                                                                                                                          | "my player unsub success" L626: the client lists again (`0x1F49`) and shows "Player Deleted."; `0x2265` is "Login Error!!"; live: the slot is empty again, and the client removed the player's local deck file (its profile folder stays)                                                                 | confirmed; we refuse (`0x2265`) a player the account lacks, its last player and a team's leader (**guess**), a member leaves their team    |
| `0x1FA4` trunk `{player_id}`                                                                       | `9000` (`0x2328`)                         | exactly one record; item[0] is copied wholesale into `UserData+0x8C8`, and the global dispatcher keeps the reply in slot 18. The record holds `cards` and `noTrade` (binary): owned and untradeable count per card, one byte at index row − 1 (CARD_Prop rows, row 0 a dummy, so 4,833 bytes). `0x410DB0` reads them bounds-checked; the collection builder `0x4F6340` writes into both unchecked (crash at `0x4F6A82`, opening the NPC duel) | "trunk data success" L666, `0x46AFA8`, `0x410DB0`                                                                                                                                                                                                                                                         | confirmed; `cards` = one copy per card of the starter deck (**guess**: a real collection comes with the economy); other contents **guess** |
| (server push after `9000`)                                                                         | `0x251C` friend list, `0x252A` black list | items `{friend_id, friend_name, login_flag}` / `{black_id, black_name, login_flag}` (`0x475A20` modes 2 and 3); see "Friends and the ignore list"                                                                                                                                                                                                                                                                                             | the global dispatcher stores them in dataset slots 6 and 7; the client never asks, and listing any player crashes without them (`0x45E734`)                                                                                                                                                               | shape; push timing **guess**                                                                                                               |
| (server push after `9000`)                                                                         | `0x2D6C` rental records                   | one item per rental not over that the player lent or borrowed (fields in "Card rental" below); the shop checks `from_type` for 1 (we send 0)                                                                                                                                                                                                                                                                                                  | the global dispatcher stores it in slot 8 (`0x4F5FBA`); opening the shop reads slot 8 unchecked (`0x48F0F6`); the collection builder `0x4F6340` reads it. `RentalRecord`/`CardRental` ask for it with `0x2C34` (bare), but not before the shop opens                                                      | live; push timing **guess**                                                                                                                |
| (server push after rental records on the first trunk following player selection)                   | `0x22C8` automatic starting decks         | Existing tag-6 deck records, legal starter then official alternatives, at most ten; omitted without eligible candidates. See [Decks](#decks-deck-editor-deckedittest-handler-0x43fc60).                                                                                                                                                                                                                                                       | required mod post-dispatch hook `0x4F44BD`, native check `0x51AA80`, serializer `0x52C320`; fresh-player NPC duel without editor                                                                                                                                                                          | confirmed (revival behavior)                                                                                                               |

Deleting a player checks current account ownership, the last-player rule and team leadership before changing any
associated data. A refused `0x1F4B` (`0x2265`, `LoginWindow 0x469C00`) keeps friends, ignores, waiting mail, rentals and
team membership intact. Tests: `a_refused_last_player_deletion_preserves_their_social_data`,
`a_team_leader_cannot_be_deleted_even_with_another_player`.

**Player record** (read by `0x46C1E4` and `0x51C260`):

- `pname`, `pindex`, `player_id`, `dlv`, `title`, `n_win`, `n_lose`, `n_draw`, `team_level`, `team_name`,
  `team_authority`.
- `avatar1`: **binary**, BE u16 entries copied into PlayerData+`0x28`. Sent as text, it crashes at `0x46C750`. Character
  settings read entries 17–40 through a checked index (`0x429210`; with only 17 entries the CRT aborts from
  `0x42923F`), so we send 41 zero entries (82 bytes): a default male avatar. 41 is the client's own length (its update `0x2CEF` sends 82
  bytes; CharMake reads entries 0–40 through checked indexes), settled. `0x42CA60` copies entries 0, 1, 2, 4–10 into ten
  separate selection fields and `0x429210` entries 17–40 into a second table; 11–16 are not read there. The catalogue
  (`yo3pbininfo.bin` kinds 1 and 2) has nine part categories per sex in its `a` field (hair, face mark, nose, eyes,
  mouth, top, hair back, bottom, glasses); which slot holds which is not mapped, and all zero is what the client starts
  from.
- `pbin`: **binary**, required. The main screen reads byte `0x4B` at `0x4727DE`; without it the client crashes.
  We send 512 bytes: the deck editor's protector loop (`0x47E490`) tests `pbin[id >> 3] & (1 << (id & 7))` for ids
  1 to 4096 (the count in `yo3pbininfo.bin`) with no length check (`0x51C260`), so 4096 bits = 512 bytes is exactly the
  size read (confirmed).
- `speech00`–`speech22` and `comment00`–`comment01`: text.

### Main screen (MainGamen polls `0x472EDC…`, global dispatcher `0x4F4A00`)

MainGamen's constructor (`0x46D8A0`) queues startup steps that run one after another, each waiting for its reply
(`obj+0xE8 = request id`): `0x472780` (Online 2 lists), `0x472DA0` (`0x2072`), `0x4731A0` (`0x27D8`), `0x4736C0`
(`0x2CEE`), `0x472F70` (`0x2583`), `0x4733C0` (`0x271A`, `0x2710`), `0x473830` (`0x2AFB`), `0x473970` (`0x2AFD`),
`0x473BE0` (a window if dataset slot 13 has items), `0x473AE0` (presents, below), `0x4725B0` (unlock), `0x473AB0`.
The login also pushes one `0x23F0 {dpauth_result 0, …, premium_expire}` after `0x2201`: only that reply (result 0)
fills the client's `premium` and `premium_expire` (global case `0x4F5349`), which the eagle at the top left parses
(`0x485C00`); empty, it showed "valid for -20767 day(s)" and the client crashed at `0x485CA1`. Live with the push: the
eagle shows "Duelist premium, Valid until 2037/12/31" and "valid for 4069 day(s)".
Until the unlock step runs, the map's facilities, the menu and the duel button stay disabled (`0x472230(0x3F)`), so
clicks do nothing. Every reply also passes through the global front dispatcher; `python3 tools/re/dispatch.py
0x4f4a4f <id>` shows the case it lands in.

| Request                                                                                                                                                                                                                                                                                                        | Reply (error)                                           | Fields                                                                                                             | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                      | Status                                                                                                                                                                                                               |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `0x2072` regulations                                                                                                                                                                                                                                                                                           | `0x2398` (`0x2399`)                                     | `regulations`: binary u16 BE regulation ids                                                                        | read unchecked at `0x4F545B`                                                                                                                                                                                                                                                                                                                                                                                                                  | confirmed field; list contents **guess** (all 20)                                                                                                                                                                    |
| `0x24C4 {friend_list, black_list}` (u32 BE player ids): Yu-Gi-Oh! Online 2's takeover, the first step `0x472780`. While `pbin[0x4B]` bit 0 is clear it sets the bit in its own copy, reads the player's local lists (`friends.y3l`, `blacks.yo3l`, `0x52CEF0`) and, if either has anyone, sends this and waits | `0x2530` (`0x2531`)                                     | none read: the step notes `yo2_takeover` 1 in its local save itself (`0x472D3C`)                                   | live: a player with a friend sent `0x24C4 {friend_list 000003f8}`; unanswered, the startup never reached the unlock and only the chat answered clicks (a player's report). The server sets `pbin[0x4B]` bit 0, so the step is skipped (traced: `0x472832` inside the step no longer runs, `0x4725B0` does), and answers `0x24C4` with `0x2530` anyway                                                                                         | confirmed                                                                                                                                                                                                            |
| `0x27D8`                                                                                                                                                                                                                                                                                                       | `0x27D9` (`0x27DA`)                                     | one record (copied into a 0xC4-byte object)                                                                        | item[0] read unchecked at `0x4F5BE9`                                                                                                                                                                                                                                                                                                                                                                                                          | shape                                                                                                                                                                                                                |
| `0x2CEE` player refresh `{player_id, type}`                                                                                                                                                                                                                                                                    | `0x2CF3` (`0x2CF4`)                                     | item = the selected player's record; its `player_id` must match                                                    | `0x47378A`, `0x4F5FC2`                                                                                                                                                                                                                                                                                                                                                                                                                        | confirmed                                                                                                                                                                                                            |
| `0x2583`                                                                                                                                                                                                                                                                                                       | `0x2588` (`0x2589`)                                     | `count` u32: D-Mail waiting (see D-Mail); a count sets flag `0x1000`, the bell's mail icon                         | `0x4730B0`                                                                                                                                                                                                                                                                                                                                                                                                                                    | confirmed                                                                                                                                                                                                            |
| `0x271A`                                                                                                                                                                                                                                                                                                       | `0x271B` (`0x271C`)                                     | none                                                                                                               | `0x47352F`                                                                                                                                                                                                                                                                                                                                                                                                                                    | shape                                                                                                                                                                                                                |
| `0x2710` NPC roster                                                                                                                                                                                                                                                                                            | `0x2711` (`0x2712`)                                     | items `{npc_type, npc_id, need_ex}`, three per NPC (one per level), kept in slot 4; see "NPC duels and duel start" | `0x473646`, `0x4BEB40`; live: 25 NPCs on three tabs; the NPC names are text `0x5208 + 2n`: 0–8 Yu-Gi-Oh!, 9–16 GX, 17–24 5D's, matching the tab texts 15504–15506                                                                                                                                                                                                                                                                             | confirmed (the tabs follow the series); `need_ex` 100/200/300 is a server choice: no client file or script holds the thresholds, it only feeds the stars and the friendship bar (`0x4BFDC4`, a divisor, so non-zero) |
| `0x2AFB`                                                                                                                                                                                                                                                                                                       | `0x2B13` (`0x2B14`)                                     | none                                                                                                               | `0x4738FA`                                                                                                                                                                                                                                                                                                                                                                                                                                    | shape                                                                                                                                                                                                                |
| `0x2AFD`                                                                                                                                                                                                                                                                                                       | `0x2B17` (`0x2B18`)                                     | none; the global case reads `member_id`, `authority` if present                                                    | `0x473A3A`, `0x4F5E0E`                                                                                                                                                                                                                                                                                                                                                                                                                        | shape (team?)                                                                                                                                                                                                        |
| `0x2775` next present (bare)                                                                                                                                                                                                                                                                                   | `0x2778` none (`0x2777`/`0x2779` would carry a present) | none                                                                                                               | builder `0x4F3A00`. The `0x2201` reply sets flag `0x80` (so does a `0x2774` push), so startup step `0x473AE0` opens PresentWindow, which sends `0x2775` and holds the main screen locked until it closes. Its switch (`0x476E74`): `0x2777`/`0x2779` "PRESENT content get success", `0x2778`/`0x277A` "content get error", which retries while `+0x58` > 0 and then closes (`0x4783C9`); its `0x2774` case reads `present_count` and drops it | confirmed (live: the facilities unlock after `0x2778`)                                                                                                                                                               |

**Banlists.** The client ships 20 regulations (`data/regulation/*.y3r`, named in the text table): 1 "Beta
Forbidden/Limited", 2 "Pre-Season", 3 "unlimited" (no lists: every card 3 times), 4–15 the OCG lists of 2004-03 to
2009-09 (which also limit the card pool to the cards out by then: `addition_card` 0 and the pool as `card3`), 16–20
"Season 12" to "Season 16". The client's deck check (`0x51AB00`) before Auto Duel and NPC duels
uses the login's `regulation_id` (`0x219A`, `0x2201`), and a hosted duel uses the host's choice; the server repeats it
at Select Deck (`0x1F4F`, below: sizes, cards owned, the duel's regulation), so a modified client gets no further. The server sends the
one its `--regulation` setting names (an id, or a name such as "Season 16"; default 20, "Season 16", since 2026-11-14:
issue #15. "unlimited", 3, was the default from 2026-11-08). Konami's server sent its current list: the 19 players'
saved settings of 2010–12 (`.y3p` `duelMaking.regulation`) mostly leave Create Duel on 0, "Latest Regulation", and Auto
Duel's settings have no banlist at all. Desktop's Hosting → Overview → **Banlist** passes `--regulation`. Under Season
16 the starter deck is legal; the nine character decks are not (Heavy Storm, Premature Burial, Snatch Steal...). The starter is the first preset regulation 1 allows; other regulations can reject it. Live: the Starter Deck breaks regulation 4 (six cards
newer than March 2004); with `--regulation 4` Kaiba Corporation answered "There are no valid Decks available.", with
`--regulation unlimited` the same deck reached the janken. Create Duel → Customize → Regulations shows the login
regulation as "Latest Regulation" (with "unlimited": no Forbidden cards, "Cards not listed are Unlimited.") and Select
lists the others by name with their Forbidden / Limited / Semi-Limited tabs (an OCG list: Limited, Semi-Limited and
"Any", "Cards not listed are Forbidden."). The same screen offers Wait Time Per Turn (Standard, Any, 2, 3, 5 minutes)
and Total Wait Time (Standard, Any, 5, 10, 20 minutes): Standard is the 180 s / 600 s the clocks use.

### Info and chat windows

| Request                                                                                                                                                                                   | Reply (error)       | Fields                                                                                                                | Evidence                                                                                                              | Status                                                                                                                                                                                                                                                                                     |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `0x1F41` info list                                                                                                                                                                        | `0x21FC`            | item[0] `{lang}` whose items are `{starttime "%Y-%m-%d %H:%M:%S" JST, title, body}`                                   | "information recv success!" `InfoWindow.cpp L1547`, `0x458A64`                                                        | confirmed; `lang` is read as a string and discarded by both readers (`0x458410`, `0x44E130`), so "1" is harmless (settled). When the rules engine can't run (`World::engine_problem`, checked at start) one more entry, "Duels are unavailable on this server", is appended (our own text) |
| `0x2CEF` player update (Avatar Settings' Confirm, built by `0x51C960`): top-level `avatar1` (binary, 82 bytes live), `speech00`–`speech22`, `comment00`–`comment01` (text), `title` (int) | `0x2CF5` (`0x2CF6`) | none ("avatar update success", CharMake `0x42AC70`); the client then reads its player again                           | live: Confirm sent it with the new hair colour in `avatar1`; after a restart the player record brought it back        | confirmed; we answer `0x2CF6` when `avatar1` isn't binary                                                                                                                                                                                                                                  |
| `0x2CF0` player search (below)                                                                                                                                                            | `0x2CF7` (`0x2CF8`) | `result_number` u32: the total, the client shows pages of 100 (`/100 + 1`); items = players `{player_id, pname, dlv}` | InfoWindow `0x458410` (`0x2CF7` case), item reader `0x475A20`; `RESPONSE_PLAYER_SEARCH_SUCCESS` `ChatWindow.cpp L651` | confirmed (fields); paging and name matching **guess**                                                                                                                                                                                                                                     |
| `0x2724` heartbeat                                                                                                                                                                        | none                | none                                                                                                                  | builders `0x4F17DF` (every 30 s) and `0x4F45A5` (after 120 s); no reply is checked                                    | confirmed                                                                                                                                                                                                                                                                                  |

Revival's default info list contains the welcome entry only; the deck-onboarding notice has been removed.
The optional engine-unavailable warning is appended using the same record shape.

**Player search** (`0x2CF0`, built by InfoWindow `0x45F390`):

- Request: `search_flag`, `search_type`, `base_point`, `front_num`, `back_num`, plus the criteria the flags name.
  The default search (no criteria) sends `{search_flag 1, search_type 1, base_point 0, front_num 100, back_num 0}`
  (captured).
- `search_flag` bits, from the builder and the Detailed Search dialog: `0x1` everyone, `0x2` name (`pname`),
  `0x4` language (`lang`), `0x8` country (`country` text), `0x10` level (`min_level` + `max_level`), `0x100`
  manner level (`eval_level`), status (`0x200`/`0x400`/`0x800`/`0x1000`: duel in progress, waiting for a single
  duel or a match), community (`0x40` friends, `0x80` ignore list, `0x20` team, `0x20000` chat room), event
  (`0x2000`/`0x4000`/`0x8000`). The preset tabs above the list send fixed sets of these bits (live: "players
  currently waiting for a Duel" `0xE00`, the chat room `0x20000`, events `0xE000`, friends `0x40`, team `0x20`).
  The Detailed Search builder (`0x45F84A`) sets `0x1000` for its first status box, `0x800` without it, `0x200` for
  "Wait for Single" and `0x400` for "Wait for match": read as a duel in progress, waiting for any duel, for a Single
  Mode one and for a Match Mode one (**guess** from the texts). `lang` is a mask, bit i language i (J, E, G, F, I, S,
  K, C; `0x74E863`), against the language each client gives in its login (`0x1F48 {lang}`, live 1). `eval_level` is a
  mask of manner levels 0–4 (`0x45F786`); `country` an ISO code from `countrytbl.bin`.
- Live: the dialog's name search sends `{search_flag 2, pname "kai", search_type 1, base_point 0, front_num 100,
back_num 0}`.
- `0x4B6AF0` looks one player up: `{search_flag 0x40000, player_id, search_type 0}`, and reads item[0]'s
  `player_id`, `pname`, `duel_id`, `duel_status` (and `friend_id`/`black_id`/`member_id`) through `0x4B6060`.
- Reply items, read by `0x475A20` in player mode (ECX = 0): `player_id` (int; 0 is skipped), `pname`, `dlv`,
  `duel_id`, `event_score`, `duel_status`, `tprank`, `duel_match_type`, `world_evid`, `official_evid`, `user_evid`,
  `userevent_remain`. The handler skips the searcher's own `player_id`, so a server may list everyone.
- We filter on name (case-insensitive substring, a **guess**), level range, player id, language, the status bits
  (duelling: in a match; waiting: hosting a Create Duel of that rule), the community bits (`0x40` the searcher's
  friends, `0x80` their ignore list, `0x20` their team, `0x20000` their chat room), the manner level (the player's own,
  from the ratings below: 2, the middle, while nobody rated them; entries carry `evallevel`) and the event bits (nobody is in an
  event). The country criterion (`0x8`, `country` an ISO code) matches the flag each player shows (see "Country" below).
  Paging is a **guess**: skip `base_point`, return at most `front_num`.
  Live: with Kaiba hosting, the waiting tab listed only him with the hosting icon.

### Chat (ChatWindow dispatcher `0x435950`)

| Request                    | Reply (error)                              | Fields                                                                                                                                                            | Evidence                                                                                               | Status                                                                                                                                                                                                                                                   |
| -------------------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `0x2135` room list         | `0x2458` (`0x2459`)                        | top `lang`; items = rooms `{id, name, capacity, num, region, status, type}` (read by `0x434D20`); `id` ≥ 1                                                        | "chat room list success!" `ChatWindow.cpp L339`; the room chooser lists them with `[num/capacity]`     | confirmed; `type` (+0x30) is tested `== 0` at `0x4364F0`: a room is listed only when it is 0 (required); `region` (+0x28) and `status` (+0x2C) are only copied between vector slots, never read; `lang` is read once at `0x4362FF` and dropped (settled) |
| `0x2136` enter `{room_id}` | `0x245A {room_id}` (`0x245B {error_code}`) | the client ignores `room_id` < 1                                                                                                                                  | "In>>Lobby" printed, tab renamed                                                                       | confirmed                                                                                                                                                                                                                                                |
| `0x2137` leave             | `0x245C`                                   | none                                                                                                                                                              | "chat room exit"                                                                                       | shape                                                                                                                                                                                                                                                    |
| `0x2134` send              | `0x2456` (`0x2457`)                        | fields sit in **one item (tag 10)**: `chat_str`, `chat_type`, and `to_id` or `to_name`. Room chat = `chat_type` 1 with `to_id` = the room id; whisper = `to_name` | captured packets; builders `0x4F3E00` (to_id) and `0x4F4030` (to_name)                                 | confirmed (room); whisper shape                                                                                                                                                                                                                          |
| (server push)              | `0x2455`                                   | items `{from_id, pname, chat_type, chat_str}`; the handler also checks `friend_id`/`member_id`                                                                    | two clients see each other's lines; the client shows nothing on `0x2456`, so the sender needs the echo | confirmed                                                                                                                                                                                                                                                |

`0x2455`'s `chat_type` picks the tab (`0x435FD0`: 0 → tab 7, 1 → 0, 2 → 1, 4 → 2, 3 → 3, 5 → 4, else 5) and the
name's colour (`0x435DAF`, table at `0x7F67C0`: chat type 0 red `#F00000`; the player's own line `#00E8FF`; a team
member green, a friend orange, anyone else `#F0F0F0`). The duel screen's chat window shows chat types 0 and 1; 2 to 7
did not appear there (live probe, 2026-11-13). An NPC's lines use chat type 0, so the NPC's name is red like the
opponent's name plate, and its `npc_id` as `from_id` (never a player's id): **confirmed** live, Pegasus and Yami Yugi.

### Friends and the ignore list (community agent `0x4A4EE0`)

PlayerInfoWindow (`0x4B6060`) looks the player up in the friend list (dataset slot 6, items by `friend_id`) and the
ignore list (slot 7, by `black_id`), matching the player's id as text (`0x531EB0`), and offers "Send Friend Request" /
"Add to Ignore List" (or their opposites). Its buttons start the community agent (`0x4B9D00` → `0x4A4EE0`), whose
mode picks a state: 0 send a friend request (`0x4A50D0`, then `0x4A5390` waits for the answer), 1 answer one
(`0x4A5B70`), 2 remove a friend (`0x4A5880`), 3 / 4 add to / remove from the ignore list (`0x4A6290`), 5 and up teams.
The global dispatcher keeps both lists: `0x251C` and `0x252A` replace them (the left panel's lists redraw, InfoWindow
`0x458410`), and the replies below add or remove one entry by themselves. Errors carry `error_code` (`0x43D4F0`).

| Request                                                                  | Reply (error)                                                                                                                                                                                           | Fields                                                                                                                                                   | Evidence                                                                  | Status                            |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | --------------------------------- |
| `0x24B9 {to_id}` friend request (top level)                              | `0x251E` (`0x251F {error_code}`: 106 "This player cannot receive another request.", 110 "The Player List is full.", 112 " The opponent has left the lobby.", else "A transmission error has occurred.") | none                                                                                                                                                     | `0x4A50D0`, switch `0x4A522E`; live packet `{request 0x24B9, to_id 1002}` | confirmed (request); replies code |
| (push to the other player)                                               | `0x24BA {from_id, from_name}`                                                                                                                                                                           | kept in slot 24, sets flag `0x100`: the bell's notification, whose answer runs mode 1 (`0x450460` case 5) with this message: a dialog naming `from_name` | dispatcher `0x4F54D7`, `0x4A5B70`                                         | code                              |
| `0x24BB {to_id: from_id, answer}` (2 accept, 1 decline)                  | `0x2522 {answer, friend_id, friend_name}` (`0x2523 {error_code}`)                                                                                                                                       | `answer` 2 adds `{friend_id, friend_name, login_flag 1}` to the list (dispatcher `0x4F55AA`) and the agent says so; else the flag `0x100` clears         | `0x4A5F5D`, `0x4A5B70` state 1                                            | code                              |
| (push to the requester)                                                  | `0x24BC {answer, friend_id, friend_name}`                                                                                                                                                               | the same list update; the waiting agent (`0x4A5390`) shows accepted or declined                                                                          | `0x4F55AA`, `0x4A5390`                                                    | code                              |
| `0x24BD` cancel the request (bare; the waiting dialog's button)          | `0x2524` (`0x2525 {error_code}`)                                                                                                                                                                        | the other player gets `0x24BE` (their answer dialog says the request was withdrawn; flag `0x100` clears); `0x24C0` clears it too, as an error            | `0x4A578E`, `0x4A5B70`                                                    | code                              |
| `0x24BF {to_id}` remove a friend                                         | `0x2528 {to_id}` (`0x2529`)                                                                                                                                                                             | the dispatcher drops the entry whose `friend_id` matches `to_id`                                                                                         | `0x4A5A60`, `0x4F57CE`                                                    | code                              |
| `0x24C2 {to_id}` / `0x24C3 {to_id}` add to / remove from the ignore list | `0x252C {to_id, to_name}` / `0x252E {to_id}` (`0x252D {error_code}`)                                                                                                                                    | `0x252C` adds `{black_id, black_name, login_flag}`, `0x252E` drops it                                                                                    | `0x4A6290`, `0x4F58DD`, `0x4F5A54`                                        | code                              |
| (login push)                                                             | `0x251C` friends `{friend_id, friend_name, login_flag}`, `0x252A` ignored `{black_id, black_name, login_flag}`                                                                                          | `login_flag` 1 = online (the dispatcher's own entries use 1); both lists are read unchecked when listing a player, so they are always sent               | `0x475A20` modes 2 and 3, `0x45E734`                                      | shape                             |
| `0x24B8` / `0x24C1` (bare)                                               | `0x251C` / `0x252A`                                                                                                                                                                                     | the left panel's friend and ignore list views ask for their list (`0x4F2FB0`, `0x4F3090`, sent by `0x459690` / `0x459760`)                               | tracer: `friend_id`, `friend_name`, `login_flag` found (`0x475A20`)       | confirmed                         |

Main Menu → Friend List opens the panel's list view (tab 9, `+0xCE`, `0x459690`): it sends `0x24B8` and a search
`0x2CF0 {search_flag 0x40, search_type 1}` with no paging (`0x45B920`, repeated every 60 s), which marks a reply
pending (`+0xD7`) and the tab as a list (`+0xD4`). A `0x251C` fills the tab and refreshes the Flash list
(`0x45BED0`: `setDuelInfoCount`, `DuelInfoUpdate`), whose rows come one by one from `setDuelInfo` (`0x459BA0`), which
draws nothing while a reply is pending; a `0x2CF7` clears the pending reply but, on a `+0xD4` tab, redraws nothing. So
the server answers the view's search with `0x2CF7` and then `0x251C`; Main Menu → Ignore List (tab 10, `0x45BA70`,
`search_flag 0x80`) the same with `0x252A`. The team view (tab 11, `0x45BBD0`, `0x20`) sets `+0xD5` instead, so its
`0x2CF7` redraws the tab itself. The search tab's "Display online friends." preset (`search_flag 0x40` with paging) is
an ordinary search and lists the online friends, each with the friend icon. Live: Friend List showed Kaiba with the
friend icon, and kept him after he logged out (`login_flag` 0 changes nothing visible in the row); Ignore List showed
"Ignore List(0)".

The server keeps friendships (both ways) and ignore lists, with the players' names, in the hub and saves them in
`server.db` (the friends document, once `friends.json`) next to the accounts. A request needs the other player online and with no request waiting (106 else,
112 when offline); answering, cancelling and removing reach the other player as the pushes above, and whenever a player
logs in or out their online friends get their friend list again (`0x251C`) with the new `login_flag`. Live: Yugi's
Send Friend Request showed "Sending Friend Request to Kaiba...", Kaiba's bar showed a friends icon whose dialog read
"You have received a Friend Request from Yugi. Add to Friend List?"; Yes gave both "… has been added to your Friend
List."; after a server restart Kaiba still had the friend icon in Yugi's search, "Display online friends." listed him
but not Joey, and Player Information offered "Remove from Friend List", whose Yes sent `0x24BF` and emptied the tab.

### D-Mail (MailWindow `0x4AC6B0`, `dmail_main.swf`)

The client keeps its mail in its own files (`players\<pname>\mail\<box>\<mail_id>.y3m`) and reads and deletes it
there; the server only holds a mail until its recipient's window fetches it. All fields are top level.

| Request                                                                                             | Reply (error)                                                                                                                                                               | Fields                                                                                                                                                                                                                        | Evidence                | Status    |
| --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | --------- |
| `0x2583` (bare; the window opening and the main screen's startup step `0x4730B0`)                   | `0x2588 {count}` (`0x2589`)                                                                                                                                                 | the mail waiting                                                                                                                                                                                                              | live                    | confirmed |
| `0x2582 {count}` (when the count isn't 0)                                                           | `0x2586` items (`0x2587 {error_code}`)                                                                                                                                      | `mail_id`, `mail_type` (0 a player, 3 a team), `from_id`, `from_name`, `to_id`, `to_name`, `title`, `body`, `send_date` "YYYY-MM-DD HH:MM:SS", `team_id`, `team_name` (`0x4AD3C0`); processing it clears the bell's mail flag | live                    | confirmed |
| `0x2580 {mail_type, title, body, to_id}` (Send D-mail, `0x4AE8E0`; subject 16 characters, body 255) | `0x2584 {mail_id}` (`0x2585 {error_code}`: 0 and 201 silent, 300 "This Player is currently offline.", 301 "There are no Players with this name.", 700 a transmission error) | the recipient is picked from the friend list (slot 6), offline friends too; team mail (type 3) has no `to_id` and carries `team_id`, `team_name`: every other member gets it                                                  | live; team mail by test | confirmed |
| (push) `0x2581` (bare)                                                                              | —                                                                                                                                                                           | a mail came: flag `0x1000`, the mail icon on the bell, which opens the window                                                                                                                                                 | live                    | confirmed |

The server keeps waiting mail in `server.db` (once `data/mail.json`) with the last mail id, so ids go on across restarts (the client
files mail by id), and drops a mail once fetched. Live: Yugi wrote to Kaiba from Write Mail (the friends button listed
Kaiba); Send D-mail got `0x2584 {mail_id 1}`, Kaiba's bar showed the mail icon, and his D-Mail window listed "Yugi /
Rematch? / 2026-11-07" with the body.

A host's BP gift (`Hub::mail_bp_gift`) posts a mail from "Host" (`from_id` 0) titled "BP received" to each player of
the account and pushes `0x2581`. The global case (`0x4F5565`) only sets the connection's flag `0x1000`, with no fields
and no window, so it is safe on any screen. Live (2026-11-12): given mid-duel against Kaiba, the bar's mail icon
appeared and the duel went on; the D-Mail window then listed the mail. Reply on it does nothing (no such player).
The game's own BOOSTERPASS result box is not used: it belongs to the window the player opens (`0x40F7B0`).

### Teams (the community agent's modes 5–12, Team window `0x4BBEC0`, `main_teaminfo.swf`)

The global dispatcher keeps the team info `0x2B13` whole in slot 17 (non-null = in a team: the chat's Team tab,
`0x436CA0`; Send to Team in D-Mail) and the members `0x2B17` in slot 9 (the player's own row gives their authority,
front `+0xE4`). After every team push (`0x2B07`–`0x2B0B`) and after forming or joining, the agent asks for both again
(`0x4A9100`). Authority: 0 member, 1 subleader, 2 leader (`0x4B6510`; Player Information shows Invite to Team to a
subleader or leader for a player without a team, Remove from Team for a lower authority, the authority buttons to the
leader). All request fields are top level; some ids come as text (live: `0x2B00`'s `to_id` "1001", `team_id` "1").

| Request                                                                                                         | Reply (error: the next id, `{error_code}`)                                                                                                                                                                                                                     | Pushes                                                                                                                        | Evidence                                                 | Status                                                                                |
| --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `0x2AFB` (bare; the Team window, the main screen's startup)                                                     | `0x2B13 {team_id, team_name, level, leader_name, created, season_score, score}`, or `0x2B14 {error_code 404}`, which clears slot 17 (`0x4F5CCA`): no team. A bare `0x2B13` (ours until 2026-11-08) made an empty team: the chat's Team tab showed for everyone | —                                                                                                                             | live                                                     | confirmed; `level` 1 (above 1 opens team titles and protectors), scores 0 (**guess**) |
| `0x2AFD` (bare)                                                                                                 | `0x2B17` items `{member_id, member_name, login_flag, authority}`, the player's own row among them (without it the Team window stays blank), or `0x2B18` without a team (changes nothing)                                                                       | —                                                                                                                             | live                                                     | confirmed                                                                             |
| Form Team `0x2AF9 {team_name}` (`0x4A6490`; needs `premium`)                                                    | `0x2B0F {team_name}` "You have formed the team" (`0x2B10`: 409 the name is taken, 808 bad characters)                                                                                                                                                          | —                                                                                                                             | live: "You have formed the team "Millennium"."           | confirmed                                                                             |
| Invite `0x2AFE {to_id}` (`0x4A6D20`)                                                                            | `0x2B19`, "Inviting %s to the team" waits (`0x2B1A`: 106 "This player cannot receive another request." for a player in a team or invited; else "A transmission error has occurred.")                                                                           | `0x2AFF {from_id, from_name, team_id, team_name}` to the player: their bell, "%s has invited you to the team "%s". Accept?"   | live                                                     | confirmed                                                                             |
| Cancel the invitation `0x2B02` (bare)                                                                           | `0x2B1F`                                                                                                                                                                                                                                                       | `0x2B03` to the player (the invitation was cancelled)                                                                         | code                                                     | shape                                                                                 |
| Answer `0x2B00 {to_id: the inviter, team_id, answer: 2 yes, 1 no}` (`0x4A76F0`)                                 | `0x2B1D`                                                                                                                                                                                                                                                       | `0x2B01 {answer}` to the inviter ("%s has joined the team "%s"." for 2, else declined), `0x2B08 {member_name}` to the members | live: Yugi saw "Kaiba has joined the team "Millennium"." | confirmed                                                                             |
| Disband `0x2AFA {player_id}` (the leader, `0x4A6880`)                                                           | `0x2B11`                                                                                                                                                                                                                                                       | `0x2B07 {team_name}` to the others (clears their slots 17 and 9)                                                              | test                                                     | shape                                                                                 |
| Remove from Team `0x2B05 {to_id}` (`0x4A7F50`)                                                                  | `0x2B25`                                                                                                                                                                                                                                                       | `0x2B0A {admin_name, member_name}` to everyone, the one removed too (its text has a third `%s` the client fills with nothing) | test                                                     | shape                                                                                 |
| Leave `0x2B04` (bare, not the leader, `0x4A8230`)                                                               | `0x2B23`                                                                                                                                                                                                                                                       | `0x2B09 {member_name}` to the others                                                                                          | test                                                     | shape                                                                                 |
| Authority `0x2B06 {to_id, authority}` (the leader, `0x4A84E0`): 1 subleader, 0 back to member, 2 the leadership | `0x2B27`                                                                                                                                                                                                                                                       | `0x2B0B {authority, to_name}` to everyone (dropped by a client without slot 17)                                               | live: "Set Kaiba as subleader?", Kaiba's bell            | confirmed; the old leader becomes a member on a hand-over (**guess**)                 |

Elsewhere: the player's record carries `team_level` (1 in a team, below 1 none), `team_name`, `team_authority` (the
icon frame, `0x46C1D0`); search entries carry `team_id` (0 none), `team_name`, `team_authority` (Player Information,
`0x4B6AF0`/`0x4B7350`); the search's team tab (`search_flag 0x20`) lists the player's team; chat type 3 goes to the
team (`to_id` = slot 17's `team_id`, `0x439D20`); D-Mail type 3 goes to every other member with the team's
`team_id`/`team_name`; Create Duel's Opponent's Limits → Team takes requests only from the host's team. Teams are kept
in `server.db` (once `data/teams.json`). Live: Yugi formed Millennium in the Team window (Lv1, founded, leader Yugi), invited Kaiba from
Player Information, Kaiba accepted from his bell, Kaiba's "gg team" reached Yugi's Team tab, and after a server
restart Player Information showed Kaiba in Millennium with Remove from Team and Set as subleader.

**Protectors and avatar items** come from the item catalogue `data/extra/yo3pbininfo.bin` (a u32 count, 4096, then
`{u16 id, u8 sub, u8 kind, u16 a, u16 b}` per pbin id): kind 1 and 2 avatar items (male, female; `b` the name text;
`sub` 2 the default part, 1 the parts any new player can pick, 0 the ones to acquire), kind 11 protectors (`a` the
name text, `b` "Card Protector"). Buying one is `0x1B68 {sale_id, item_num 1}` → `0x1C24 {dpoint}` like a pack (sale
ids 30000 + the pbin id), then the present: `type` 7 for a protector (PresentWindow case 7, "Protector") and 6 for an
avatar item with `name_id` its name text (the shop's case: "You have acquired %s!"), `item` the pbin id as a
big-endian u16. The client sets the pbin bit itself; the server keeps the owned ids with the player and sets their
bits in every record's `pbin` (the deck editor's protector list `0x47E490` and Avatar Settings read them). A second
purchase of one fails (`0x1C25`). Live: P-01K for 200 BP showed "P-01K Card Protector"; Domino: Red for 100 BP
showed "You have acquired Domino: Red!".

### Card trade between players (agent step `0x4A91D0`, trade scene `CardTrade.cpp`, handler `0x4248F0`)

Player Information's Send Trade Request invites; the other player answers from their bell (the global dispatcher
keeps the invitation in slot 25, flag `0x400`); a yes opens the trade scene on both, which lists the player's trunk
and the two offers. The scene's replies and pushes (ids from its jump table at `0x425140`, named by its log lines):

| Request                                                                            | Reply (error: next id, `{error_code}`)                                                                                                    | Pushes                                                                                                                                                                                                       | Evidence                                                     | Status                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `0x2BC0 {to_id}`                                                                   | `0x2BD7 {pname}` "Sending Trade Request to %s..." (`0x2BD8`: 106 "This player cannot receive another request.")                           | `0x2BC1 {from_id, from_name}` to the player: "You have received a Trade Request from %s. Accept?"                                                                                                            | live                                                         | confirmed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| cancel `0x2BC4`                                                                    | `0x2BDD`                                                                                                                                  | `0x2BC5` to the player                                                                                                                                                                                       | research                                                     | shape                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| answer `0x2BC2 {to_id (text, live), answer: 2 yes, 1 no}`                          | `0x2BDB` (OFFER_REPLY_SUCCESS, in the scene)                                                                                              | `0x2BC3 {answer}` to the inviter (OFFER_NOTICE): 2 opens its scene                                                                                                                                           | live                                                         | confirmed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ready `0x2BD5` (each scene on opening)                                             | `0x2BED`                                                                                                                                  | once both: `0x2BD6` (NOTICE_START) to both                                                                                                                                                                   | live                                                         | confirmed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| offer `0x2BC6 {main_list}` (the cards dragged from the trunk, big-endian u16 rows) | `0x2BE1` "card list send ok" (`0x2BE2`: 502 offer player illegal, 503 trade error, 504 card number illegal, 505 status, 700 server error) | `0x2BCF {main_list}` to the partner (NOTICE_OTHER, read unchecked)                                                                                                                                           | live: Tainted Wisdom `main_list 0011`                        | confirmed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `0x2BC7` (the partner's scene has the offer)                                       | `0x2BE3`                                                                                                                                  | `0x2BCE` (NOTICE_OWN) and `0x2BD0` (NOTICE_CHANGE_ARRIVE) to the offerer                                                                                                                                     | live                                                         | confirmed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Ready to Trade. `0x2BC8 {status: 1 on, 0 off}`                                     | `0x2BE5`                                                                                                                                  | `0x2BD1 {status}` to the partner; both on: `0x2BD2` (ALLREADY) to both, which shows "Execute Trade?"                                                                                                         | live                                                         | confirmed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| "Execute Trade?" `0x2BC9 {answer: 2 yes, 1 no}` (`0x427640`)                       | `0x2BE7`                                                                                                                                  | once both answered: `0x2BCA {error_code, own_status, other_status, trade_result}` to both (LAST_CONFIRM_NOTICE; `trade_result` 1 "Trade Completed"), then a new trunk `9000` (no request asks for one there) | live: Yes sent 2; "Trade Completed", the cards changed sides | confirmed. `0x2BCA` (`0x424F71`): `trade_result` 1 plays "TradeSucess"; otherwise with `error_code` 0 the scene stays with both Ready marks on (`0x427730`), with `error_code` non-zero `0x427400` shows `$14313` "Trade Cancelled" and leaves (no other use of that text). So a No sends a non-zero `error_code` (503, `TRADE_ERROR`; the client only tests for 0). Live (2026-11-08): Yugi said Yes and Kaiba No; both screens showed "Trade Cancelled" with OK, then closed back to the main screen, the cards unchanged, and a second trade opened fresh and completed ("Trade Completed"). `own_status`/`other_status` are read and unused. The Yes/No buttons are 1/0 in the dialog and `answer` 2/1 (No inferred) |
| leave `0x2BCB`                                                                     | `0x2BE9`                                                                                                                                  | `0x2BCD` to the partner: "The trade partner has cancelled." (also when a player goes offline)                                                                                                                | research, test                                               | shape                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

The server checks each offer against what the player owns (their starter deck's cards, the ones bought or got, less
the ones given away) and moves both offers when both said yes, in the accounts (`server.db`, once `data/accounts.json`: `cards` and
`given` per player). A session refreshes its player's collection from there on each trunk request, so the partner's
next shop visit shows it too. Live: Yugi's Tainted Wisdom for Kaiba's Windstorm of Etaqua.
Saved collections are looked up by player id across every account, including `/join` and `add-account` accounts with
a key and no stored address. This applies to trade execution, rental offer validation and every trunk refresh
(`9000`, collection builder `0x4F6340`). Tests: `two_key_only_accounts_can_complete_a_trade`,
`a_key_only_accounts_trunk_refreshes_its_saved_collection`, `a_key_only_lenders_offer_is_checked_against_their_saved_cards`.

Confirmation is tied to the current offers (**server choice**, protecting the sequence above): changing either offer
clears both Ready marks and both final answers. `0x2BC9` before both scenes and Ready marks are ready is refused with
`0x2BE8 {error_code 503}` (the error twin in CardTrade's handler `0x4248F0`; global dispatcher `0x4F6057` reads nothing).
Tests: `a_final_answer_before_both_ready_marks_is_refused`, `changing_an_offer_withdraws_every_confirmation`.

One scene at a time (a **server choice**): a player in a trade or rental scene can neither invite nor be invited, and an
answer that finds either side in a scene by now is sent to the inviter as a "no" (`0x2BC3 {answer 1}`, `0x2C27` for
rentals) with `0x2BDC` (`0x2D55`) `{error_code 106}` for the answerer; leaving a scene ends every scene of the player.

### Card rental between players (agent steps `0x4A9D80` and `0x4AA430`, rental scene `CardRental.cpp`, handler `0x41C7E0`)

Player Information's Send Rental Request (text 12267) invites; the other player answers from their bell (the global
dispatcher keeps the invitation, flag `0x800`); a yes opens the rental scene on both. **The player who sent the
request is the lender**: the scene is opened with `for_rent` 1 for them (`0x4AA3BB`) and 0 for the borrower
(`0x4AA936`), and SetUp (`cardrental.swf`) locks the borrower's controls. The ids follow the trade scene's pattern
(requests by even ids, replies `0x2D50`..., pushes odd) and are named by the handler's log lines:

| Request                                                                                                                                                                                   | Reply (error: next id, `{error_code}`)                                                                                                                                                                                                                                                                      | Pushes                                                                                                                 | Evidence                                                                                                      | Status                                                                                                                                                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `0x2C24 {to_id}` (`0x4A9E1C`)                                                                                                                                                             | `0x2D50 {pname}` "Sending Rental Request to %s..." (`0x2D51`: 106 "This player cannot receive another request.")                                                                                                                                                                                            | `0x2C25 {from_id, from_name}` to the player: "You have received a Rental Request from %s. Accept?"                     | live                                                                                                          | confirmed                                                                                                                                                                                                                                                                |
| cancel `0x2C28` (`0x4AA2B9`)                                                                                                                                                              | `0x2D56` (`0x2D57`)                                                                                                                                                                                                                                                                                         | `0x2C29` to the player (the global dispatcher clears flag `0x800`, as `0x2C37` does)                                   | code                                                                                                          | shape                                                                                                                                                                                                                                                                    |
| answer `0x2C26 {to_id (text, live), answer: 2 yes, 1 no}` (`0x4AA611`)                                                                                                                    | `0x2D54` (`0x2D55`)                                                                                                                                                                                                                                                                                         | `0x2C27 {answer}` to the lender: 2 opens its scene                                                                     | live                                                                                                          | confirmed                                                                                                                                                                                                                                                                |
| ready `0x2C3E` (each scene on opening, `0x41C6AF`)                                                                                                                                        | none read: the handler compares `0x2D5A`–`0x2D61`, `0x2D64`–`0x2D67`, `0x2D6A`/`0x2D6B` only (`0x2D62`, `0x2D63`, `0x2D68`, `0x2D69` fall to its default) and the global dispatcher sends `0x2D68` to its empty arm (`0x4F6057`); `0x2D68` is the free slot in the reply numbering, so the id is immaterial | once both: `0x2C3F` (NOTICE_START) to both                                                                             | live                                                                                                          | confirmed (no reply is read; no "no response" timer exists for it, see "Requests with a timeout")                                                                                                                                                                        |
| conditions `0x2C2B {type 0 Free / 1 Deck, days}` (the lender's Rental Conditions dialog, `0x41ED74`)                                                                                      | `0x2D5C` (`0x2D5D`)                                                                                                                                                                                                                                                                                         | `0x2C39 {type, days}` to the borrower (`SetType`)                                                                      | live: 3 days, Free sent `days 3`, `type 0` (the term is the number of days: 1, 3, 7 or 14, texts 14212-14215) | confirmed                                                                                                                                                                                                                                                                |
| cards `0x2C2A {main_list, extra_list, side_list}` (`0x4211F1`; a Free rental sends `main_list` only, the cards as big-endian u16 CARD_Prop rows; double-clicking a trunk card puts it up) | `0x2D5A` (`0x2D5B`)                                                                                                                                                                                                                                                                                         | `0x2C38 {main_list, extra_list, side_list}` to the borrower                                                            | live                                                                                                          | confirmed; `0x2D5B` only logs; no `error_code` is read by the rental handler (the arms `0x2D5D`, `0x2D5F`, `0x2D61`, `0x2D65`, `0x2D67` all show `$14225` "Rental Cancelled." through `0x41D46B`), so 504 is unread (settled)                                            |
| the partner's scene has the cards or conditions `0x2C2C` (`0x41EFC6`)                                                                                                                     | `0x2D5E` (`0x2D5F`)                                                                                                                                                                                                                                                                                         | `0x2C3A` to the partner (unlocks the lender's next send, `+0x8A`)                                                      | live                                                                                                          | confirmed                                                                                                                                                                                                                                                                |
| the lender's Confirm `0x2C2D` (`0x421586`)                                                                                                                                                | `0x2D60` (`0x2D61`)                                                                                                                                                                                                                                                                                         | `0x2C2E` to the borrower: "Borrow these cards?"                                                                        | live                                                                                                          | confirmed                                                                                                                                                                                                                                                                |
| the lender takes it back `0x2C31` (`0x421646`)                                                                                                                                            | `0x2D66` (`0x2D67`)                                                                                                                                                                                                                                                                                         | `0x2C32` to the borrower                                                                                               | code                                                                                                          | shape                                                                                                                                                                                                                                                                    |
| the borrower's answer `0x2C2F {answer: 2 yes, 1 no}` (`0x421704`)                                                                                                                         | `0x2D64` (`0x2D65`)                                                                                                                                                                                                                                                                                         | `0x2C30 {trade_result: 1 done, 0 not}` to both: 1 "Rental Completed."; 0 "Rental Cancelled." and the scene stays       | live: Yes sent 2, "Rental Completed." on both                                                                 | confirmed: `0x2C30` (`0x41CAB1`) shows "Rental Completed." only for `trade_result` 1; anything else "Rental Cancelled." (`$14225`), clears `+0x8A`/`+0xA0` and the scene stays, which is what we send for a No (settled; the No's `answer` 1 is inferred as the trade's) |
| exit `0x2C33` (ExitRental)                                                                                                                                                                | `0x2D6A` (`0x2D6B`)                                                                                                                                                                                                                                                                                         | `0x2C37` to the partner: "The Borrower has cancelled." / "The Lender has cancelled." (also when a player goes offline) | code                                                                                                          | shape                                                                                                                                                                                                                                                                    |

**Records and collections.** The client counts the cards itself. The collection builder `0x4F6340` copies the trunk
(`9000`), then, for every item of the records `0x2D6C` (slot 8): with `to_id` = the player and `confirm_to` 0 it adds the
item's `main`/`extra`/`side` cards (and to `noTrade`: a borrowed card can't be traded), and with `from_id` = the player and
`confirm_from` 0 it takes them away (flags: `0x1D` Edit Deck and the duel screens, `9` the trade and rental scenes and the
shop, `1` the plain trunk). So the server **moves no cards between collections**: the trunk `9000` stays what each owns, and
a rental is one record per rental, pushed to both players (`0x2D6C`) when it starts, when it ends and at login, and sent for
`0x2C34`. Record fields: `from_id`, `from_name`, `from_type` (0; the shop's own rentals use 1), `to_id`, `to_name`,
`rent_end` (text "%Y-%m-%d %H:%M:%S", Japan time), `deck_type` (0 Free, 1 Deck), `confirm_from`, `confirm_to` (0 while it
runs, 1 once over) and `main`/`extra`/`side` (big-endian u16 rows). We send only the rentals not over in `0x2D6C`; the
history `0x2D6E` has the over ones, greyed. The server keeps the rentals in `server.db` (once `data/rentals.json`), checks the terms once a
second and, when one ends, pushes the new records to both (live: Kaiba's Edit Deck trunk went from 42 back to 40, the two
cards 000), and it stops a lender from putting up or trading cards they have lent (`0x2D5B`, `0x2BE2`: owned less
lent). Live: Yugi lent Tainted Wisdom and Windstorm of Etaqua to Kaiba for 3 days; Yugi's rental scene trunk showed them as
000, Kaiba's Edit Deck listed them (42 cards, a blue badge) and both Rental Card Lists had the row; a server restart
kept it and an end time moved to the past ended it while both were online.

Not seen: a Deck rental (`type 1`: three lists). **No early return** (issue #13): the Borrowed tab's script
(`rentalrecord.swf`, frame 3) wires `borrow_btn0` (`BorrowDeckSave`) and `borrow_btn1` (`BorrowReturan` with the row's
index) and the C++ registers no callbacks for them (RentalRecord's Accept `0x481D70`, slot 1 of the vtable at
`0x7A7A54`), but the shipped movie never places either button (nor `btn_main`, AdjustRental): only `btn_main01` is in
its library. Live (2026-11-14): Bravo's Borrowed tab listed Alpha's rental, with no button below it. So a rental ends at
its term. The server already takes an early return, `0x7A14 {rental_id}` from the borrower (every record carries our
`rental_id`, which the client ignores; contract `the_borrower_can_return_a_rental_early`), ending it as at its term;
what the client lacks is a button to send it (see the roadmap, #13). The "Lender's / Borrower's Rental List is full" texts (14205/14206) have no code we know of.

### Country (flags)

The client reads `country` (text, an ISO code it looks up in `data/text/countrytbl.bin`: 200 two-letter codes, ids 1-200,
`AL`... `HN`; `0x51BC80` returns the id, -1 for a code it doesn't have) in the login item (`0x2201`, the leaderboard link
`0x46FC14`), the player records (`0x2260`, Player Information `0x4B7350`, a duel's `vsPlayerData` `0x4A2D30`/`0x4A4460`), the
search entries (`0x2CF7`) and the bracket's seats (`0x32FD`, `0x4CEBE0`). No request carries the client's country, so the host
sets one per account (`yo3-server set-country ADDRESS CODE`, `countries` lists the codes, `auto` clears it), kept in
the accounts (`server.db`); without a setting it follows the language the client gives at login (`0x1F48 {lang}`): Japanese JP,
English GB, German DE, French FR, Italian IT, Spanish ES, Korean KR, Chinese CN (**guess**: the client has no language-to-country table. It derives `lang` itself in `0x40DE50` from the Windows primary language id: ja 0, de 2, fr 3, it 4, es 5, ko 6, zh 7, else 1, which confirms our numbering; `country` is always server data and `countrytbl.bin` is only an id lookup). A missing or unknown `country`
shows the Germany flag (what every player showed until 2026-11-08). The search's country criterion (`0x8`) compares the code.
Live: Player Information showed the UK flag for an English client, the Japanese flag once the account was set to JP.

### Manner ratings (`dlg_manner.swf`, DuelResult `0x512A40`, dialog `0x4C2A10`/`0x4C3F20`)

| Message                                                                                                 | Fields                                                                                                                                                                                      | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Status                                    | FIXME:                               |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- | ------------------------------------ |
| push `0x2CEC` (after the duel's `0x2009`, to each player of a duel between players, at the match's end) | `eval_id`                                                                                                                                                                                   | the global dispatcher keeps it in slot 22 and sets front flag `0x40` (`0x4F6024`); DuelResult state 0 (`0x512A40`) sees the flag once the duel's result is known (MainGamen `+0x820`), clears it and opens the dialog "Please evaluate NAME." with the **opponent's name** (the client's own). The `0x2CEC` arms of CardRental (`0x41D164`) and CardTrade (`0x425114`) only set a flag: a trade never opens the dialog, so the client has no trade ratings | live: both players got the dialog         | confirmed                            |
| `0x2CED {eval_id, eval_data}`                                                                           | `eval_data` is the chosen button's value from the table at `0x790004`: -10, -5, 0, 5, 10 (sent as a u32, so -10 arrives as 4294967286); top row (five stars) +10, bottom row (one star) -10 | `0x4C3F20`; live                                                                                                                                                                                                                                                                                                                                                                                                                                           | confirmed                                 |                                      |
| reply `0x2CF1` (error `0x2CF2 {error_code}`)                                                            | none                                                                                                                                                                                        | DuelResult state 2 (`0x512A40`) waits for an id above `0x2CF0` and below `0x2CF3`; the global dispatcher ignores both                                                                                                                                                                                                                                                                                                                                      | live: the dialog closed, no "no response" | confirmed (`error_code` 0 **guess**) |

The server keeps the evaluations it asked for (`eval_id` -> rater, player rated; ours count up from 1): an answer counts once, from
the rater, with one of the five values. Each player's ratings (count and sum) are in the accounts (`server.db`)
(`eval_num`, `eval_total`). `evallevel` (0-4) is the average mapped onto the buttons' scale, (average + 10) / 5 rounded
(a **guess**: the original's thresholds are unknown), 2 without ratings. It is in the search entries (Player
Information's face icon: live, a blue smile for Kaiba after a +10, a dead red face for Yugi after a -10 and the star
icons between) and, with `evalnum` and `evaltotal` (the sum, 0 when negative: **guess**, only the level is read), in the
player records.

### Main Menu screens (MainGamen's menu `0x46FA60`)

| Screen                                                         | Request                                                                              | Reply (error)                                                                            | Fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Evidence                                                                                                                                                                                      | Status                                                                                               |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Rental Card List (RentalRecord `0x481A30`, `rentalrecord.swf`) | `0x2C34` (bare), then `0x2C35 {rental_kind 0, base_id}` and `0x2C35 {rental_kind 1}` | `0x2D6C` (`0x2D6D`), `0x2D6E` (`0x2D6F`) twice                                           | the screen is built only after all three (`0x482250`); `0x2D6C` also replaces slot 8. Kind **0 is the Lent tab, kind 1 the Borrowed one** (the first `0x2D6E` is kept in the vector at `+0x78`, which `GetLoanCount` `0x482FE0` counts, the second at `+0x90`, `GetBorrowCount` `0x482970`). Borrowed rows `{from_name, from_type, rent_end, deck_type, confirm_to}` (`0x482A90`), lent rows `{to_name, rent_end, deck_type, confirm_from}` (`0x483100`), a row's cards `deck_type`, `main`/`extra`/`side` (`0x483620`); a flag of 1 greys the row ("over"). Without the replies the screen stays black                                                                                                                            | live: Kaiba's Borrowed tab listed Yugi, "2026-11-11 19:03:35", Free, and the two cards; Yugi's Lent tab listed Kaiba; after the term the row went grey                                        | confirmed                                                                                            |
| My Profile (`profile.swf`)                                     | `0x1F49` (bare)                                                                      | `0x2260`                                                                                 | the item whose `pindex` is the player's; ProfileGet (`0x4705D0`) reads `pname`, `title`, `dlv`, `tp`, `n_win`, `n_lose`, `dscore`, `n_duelall`, `turns`, `n_a_win/lose/draw`, `n_nogame`, `isurrender`, `usurrender`, `lpleft`, `n_m_win/lose`, `n_s_win/lose`, `sidedeck`, `sacrifice1..3`, `fusion`, `synchro`, `gravesummon`, `spsummon`, `itrap`, `utrap`, `imagic`, `umagic`, `iefmon`, `uefmon`, `reversalwin`, `reversallose` (absent = 0). We send `n_win`, `n_lose`, `n_draw`, `n_duelall` from the duel records against players (also in the search entries, which Player Information's "Number of Wins" reads)                                                                                                          | live: Wins 4, Losses 2                                                                                                                                                                        | confirmed; NPC duels not counted (**guess**)                                                         |
| Duel Log (DuelLogWindow `0x499750`, `duel_log.swf`)            | `0x1793` (bare) on opening; Server Duel Logs `0x19C8 {pname, offset}`                | `0x177B` (`0x177C`), `0x19C9` (`0x1790`)                                                 | `0x177B` is the championship list, items `{year, season, event_id}`; the server log tab queues behind it. `0x19C9` items `{end_time (epoch), match_type, dlv_1, dlv_2, pname_1, pname_2, dlog_comment, duel_id, filename, dlogURL}` (`0x49ADF0`) would be replays to download; we keep none. **Latest Duel Logs** lists the `*.dlog` files of `players\<name>\duellogs\myduels\` (`0x49A950`: each loaded by `0x524CA0`, whose header fields are the `0x19C9` item's; the row prints `end_time` through `_localtime32`, so the client's own time zone, `dlv_1`/`dlv_2` as `Lv%d`, `pname_1 V.S. pname_2`); Duel Log Collection is `favorites\`. Those files are **downloaded by the client after each duel**, see "Duel log files" | live: both lists empty without logs; with them Latest Duel Logs lists the duel                                                                                                                | confirmed                                                                                            |
| BOOSTERPASS (BpInputWindow, `boosterpass.swf`)                 | `0x20D0 {dpcode}` (a 16-character code)                                              | `0x23F0 {dpauth_result, dpPoints, dpGetPoints, premium, premium_expire}`, no error reply | 0 adds `dpGetPoints` and shows the points, then "Acquired Duelist premium! Valid until" `premium_expire` (text, JST, `%4d-%2d-%2d %2d:%2d:%2d`, `0x485B50`; without it `localtime` gives null and the client crashes at `0x485CA1`); anything else "Incorrect BOOSTERPASS CODE.". The global dispatcher keeps `premium` from it (`0x4F5349`), so it goes with 1                                                                                                                                                                                                                                                                                                                                                                    | live: a host code showed "BOOSTERPASS Point(s) 10000 + 2500", "Acquired Duelist premium!", "Valid until 2037/12/31 15:59" (the PC's time zone); an unknown code "Incorrect BOOSTERPASS CODE." | confirmed; codes are the host's (`add-code`), once per account, `premium_expire` 2037-12-31 23:59:59 |
| Leaderboard                                                    | none                                                                                 | —                                                                                        | ShellExecute on Konami's dead ranking page with the login item's `country` (`0x46FC14`), which we send                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | code                                                                                                                                                                                          | nothing to serve                                                                                     |
| DUEL OPTIONS                                                   | none                                                                                 | —                                                                                        | local options file                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | code                                                                                                                                                                                          | nothing to serve                                                                                     |

### Shop (`shopmain.cpp`, handler `0x48AA60`)

The map's top-left facility. Opening it re-requests the trunk (`0x1FA4`), and each reply below asks for the next, in
this order; the last one clears `isLoading`. Success and error replies are pairs (`0x1C20`/`0x1C21`, …); none of
them has a global case. Every list case checks its item count (`0x410D90`) before reading. The shop's BP
(BOOSTERPASS Points, `user_bp`) is the account's `dpoint` from the login replies.

| Request                                                                       | Reply (error)       | Fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Evidence                                                                                                                                                          | Status                             |
| ----------------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| `0x1B66` text (bare, sent from the `9000` case, `0x48BEDD`)                   | `0x1C20` (`0x1C21`) | `text`: the shopkeeper's line. The case also reads `evalnum`, `evaltotal`, `evallevel`, but from the player record; they pick a stock line (`$33003`–`$33007`) when `text` is empty                                                                                                                                                                                                                                                                                                                                                                                                                         | "shop text success" L286                                                                                                                                          | confirmed                          |
| `0x1B67` items (bare, `0x48ACC5`)                                             | `0x1C22` (`0x1C23`) | items `{item_type, item_id, item_cost, item_max, sale_id, sale, item_trade, modified, rel_time}` (`0x485E50`; dates are text); `item_type` < 3 go on the shelves, others are kept only when `item_param` isn't owned (a bit set at the player data's `+0x68`). The kept list (`+0xD4`, 0xD8 bytes each) fills the Purchase screen (`0x48E0A0`): kinds 0 and 3 call the Flash `AddPack(id, name, comment, image, date, cost, max, trade)` (`0x48C790`) with the name, description, image and date from the client's own pack file `item_id`, 1 and 4 `AddStruct`, 6 `AddProtector`, 10 and 11 `AddEventPack` | tracer: all nine found; live: three packs on the Booster Packs shelf with their own art, then "Pack 1", "A rearranged pack comprising of season 1 Cards.", 100 BP | confirmed                          |
| `0x1B6F` (bare, `0x48AE5F`) | `0x1C26` (`0x1C27`) | Trade Card (Blister's exchange): items `{item_id, item_cost, item_trade, item_type, modified, rel_time, sale_id}`. The client loads its own pack file `item_id` and lists one card per row of it, each costing `item_cost` of the player's cards ("Cards Required:"); `item_trade` 1 would mark the received card no-trade. We list the client's exchange pools, pack files 264–268 (see "Card exchange"), at 10 cards each, `item_type` 0 | "shop exchange item success" L360; live: 2,612 cards, alphabetical, each "Cards 10" | confirmed; cost 10 (**guess**), `item_type` 0 (**guess**) |
| `0x1B6B` (bare, `0x48B3B8`)                                                   | `0x1C32` (`0x1C33`) | Purchase Protectors: items like the packs' (`0x485E50`) with `item_type` 6 (AddProtector `0x48D520`) and `item_id` the protector's pbin id (2305–2512, the catalogue's kind 11; the client leaves out the ones the player's pbin owns, images `data/protector/p%04d.png`)                                                                                                                                                                                                                                                                                                                                   | "shop protector item success" L409; live: ten protectors a page with their art                                                                                    | confirmed; 200 BP each (**guess**) |
| `0x1B6D` (bare, `0x48B4C3`) | `0x1C2E` (`0x1C2F`) | items; an empty list leaves Purchase Special Packs disabled (`_root.spac_btn_disable`). Kept empty: see "Special Packs" | "shop event item success" case; the client moves on to `0x1B72` | shape (empty, decided) |
| `0x1B72` (bare, `0x48B64D`)                                                   | `0x1C36`            | `perk_num` u32, compared with 500 to show `_root.expack_del`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | tracer: found at `0x48B6CD`                                                                                                                                       | shape (value **guess** 0)          |
| `0x2C40` rental terms (bare, `0x48B777`) | `0x2D75` (`0x2D76`) | `rental_cardnum`, `rental_point`, `rental_term` u32: "You can borrow %d cards for %dBP for %d days." (text 15081); the pick list must hold exactly `rental_cardnum` cards, so 0 is an offer nobody can confirm. We offer 5 cards for 100 BP for 7 days (0 without the client's rental pool) | read at `0x48C20E`; the case then sends `0x1B69`; live: "You can borrow 5 cards for 100BP for 7 days." | confirmed; values **guess** |
| `0x1B69` (bare, `0x48C391`)                                                   | `0x1C2A` (`0x1C2B`) | Purchase Avatar Items: items with `item_type` above 2 and `item_param` the item's pbin id, which the client leaves out once owned, then the packs' fields; `0x496A50` sorts them into four tabs by id (tables `0x78D2E8`, `0x7A83B0`) and by gender, skipping unknown ids. We sell the catalogue's parts to acquire (kinds 1 and 2, sub 0: 178 male, 148 female); clears `isLoading`                                                                                                                                                                                                                        | "shop avatar item success"; live: the Male and Female lists by tab, a preview on the avatar                                                                       | confirmed; 100 BP each (**guess**) |
| `0x1B68` buy `{sale_id, item_num, end_time}` (live: `end_time` the text `""`) | `0x1C24` (`0x1C25`) | `dpoint`: the BP left, shown as `user_bp`; a Special Pack's purchase adds a fourth field, `ex_pack` (-1 or the regular pack picked in its chooser), never sent while the Special Packs list is empty; then the client asks for presents (`0x2775`, builder `0x4F3A00`, through `0x47C4A0`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | "shop buy succes"; live: the purchase dialog's "BP After Purchase", then the pack opening                                                                         | confirmed                          |
| `0x1B6A` exchange `{sale_id, item_id, kaban}` | `0x1C28` (`0x1C29`) | `sale_id` the pool, `item_id` the picked card's CARD_Prop row, `kaban` the cards given as big-endian u16 rows (live: 264, 2, 20 bytes). `0x1C28` has no fields; its case asks for presents (`0x47C4A0`), and the card comes as a type-0 present named after the booster pack it first came in. `0x1C29` shows an error dialog (`0x493D30`) | "shop exchange succes"; live: Armored Starfish NEW!, the trunk 498 → 489 after the shop was entered again | confirmed |
| `0x2C41` shop rental `{main_list}` (big-endian u16 rows) | `0x2D77` (`0x2D78`) | `dpoint`: the BP left (the client sets its BP from it). The client then asks for the rental records (`0x2C34`) itself; the rental's record has `from_type` 1 and `from_id` 0, shown as "`<Shop>`" (text 14266). A new rental from the shop replaces the one under way (texts 15082, 15084) | live: "Borrow these cards for 100BP for 7 days. OK?", "Rental Completed.", BP 6000 → 5900, Rental Record "`<Shop>`", 2026-11-22, Free | confirmed |

**Card exchange.** Blister's Trade Card lists the cards of each `0x1C26` item's pack file. The client has five pack
files made for it: 264–268 (`pack_type` 4, no art or name) with 625, 470, 483, 474 and 560 cards, disjoint, which are
exactly the cards that first came in booster packs 1–14, 15–23, 24–32, 33–40 and 41–48. The server lists those five,
at 10 cards each (the original costs are unknown). The player picks a card, the trade screen asks for exactly
`item_cost` trunk cards (dragged, double-clicked or Auto Select), and "Trade selected cards with ... OK?" sends
`0x1B6A`. The server checks the cards against the stored player under the accounts' lock (lent cards can't be given,
borrowed ones aren't the player's), takes them and adds the picked card. The present window titles a one-card present
"Card" (text 68) with the pack's name under it: the pool's own file has none (live, "Attack"), so the present names the
first booster pack holding the card ("Pack 5").

**Shop rental.** Rental lists the client's pack file 10000 (`pack_type` 19, 4,292 cards) to pick from; the server sets
only the offer (`0x2D75`) and checks the picks (`0x2C41`): exactly `rental_cardnum` cards, all from that pool, and the
BP. The cards are lent like a rental between players (`rental.rs`): the records give them to the borrower until the
term ends, they count in the borrower's decks, and they can't be traded or exchanged away.

**Special Packs.** The client's texts make them event prizes: a present ("Right to purchase Special Pack", 15125) that
unlocks a pack at Blister's for BP until a date, earned by event wins, streaks and stars. The client has names and art
for about 100 of them but not their contents ("For pack content, check the official website", 28000–28007), and the
server holds no events. Any contents would be invented, so the list stays empty and the button disabled (decided
2026-11-14, issue #8).

**No presents between players.** PresentWindow (`PresentWindow.cpp`, `0x476240`-`0x4784F0`) only ever builds `0x2775`
(`0x4F3A00`, "give me the next present"): it receives presents and sends none. No other screen has a send-present
request: the client's text table has no "send present" string (the "gift" lines are NPC speech), and the player menus
offer only D-Mail (no attachment), friend request, trade (cards) and rental. So presents come from the server alone
(bought packs, decks, protectors and avatar items, and anything a prize would be); cards move between players only
through trade and rental. Nothing to answer.

**Bought packs arrive as presents.** After `0x1C24` the client asks for the next present (`0x2775`) until there is
none (`0x2778`), then reloads the shelves (`0x1B67`…). PresentWindow (`0x476DE0`, cases `0x2777`/`0x2779`) reads
`present_id`, `type`, `level`, `form`, `item_img`, `end_time`, `item` and, for `type` 0 or 9 (a pack), `pack_id`, whose
name and description it takes from its own pack file. `item` is binary: big-endian u16 CARD_Prop rows
(EasyMultiPlayDataBinary's `0x407ED0`), here the pack's cards, which the window shows one by one (marked NEW! when the
trunk has none) with a SKIP button. The client doesn't add them to its trunk itself (the shop's `0x2777` case acts
only on `type` 6): the server keeps them, and the next trunk request (`0x1FA4`) brings them. Live: Pack 1 opened
showing "Power of Kaishin" NEW!, and SKIP returned to the Shop Menu.

**Structure decks.** A shop item of kind 1 or 4 goes to the STRUCTURE DECK tab: `AddStruct` (`0x48CE40`) looks up the
preset deck `item_id` in the client's own recipe files (`0x52DC10`) for its `name_id`, `discription_id`,
`deck_image` and `modified`. The recipes of `deck_type` 1 with their own art are the structure decks SD001–SD005
(decks 4, 5, 6, 501, 502; decks 1–3 are art-less starters). Bought, each comes as a present of `type` 1
(PresentWindow case 1, `0x4777D3`): the deck id is the first u16 of `item` (`0x410AD0`, index 0), the deck is looked up
again (`0x52DC10`) and handed to the deck routines `0x414410`, `0x415310`, `0x414BC0`, `0x419050`, `0x4188D0`; the window
shows the deck's name, its description when the reply's `desc` text is `"-1"`, and sets the deck's `pbin_no` bit in
the player's owned-items bitmap (player data `+0x68`). The server sells each structure deck under sale id 20000 + its
deck id (pack ids overlap deck ids) and adds the deck's cards to the collection. Live: the tab showed SD001–SD005 with
their box art, newest first; GADGET BOX (sale id 20502) cost 500 BP and came as a grid of its 40 cards, each NEW!,
with its name and description, then OK walked through the cards and closed the window (no local deck file is
written).

**Where the client's BP comes from.** The client keeps its BP at UserData `+0x76C`, which the shop and the avatar
shop (`0x4963E0`) check before they send a purchase ("You do not have enough BOOSTERPASS Points."). Four replies set
it: the login's `dpoint` (`0x469C00`), the purchase `0x1C24` and the rental `0x2D77` (both shop cases read `dpoint`),
and BpInputWindow's `0x23F0 {dpauth_result, dpPoints, dpGetPoints, premium, premium_expire}`, the BOOSTERPASS code
window, which adds the points got (`0x40F7B0`; the global case `0x4F5349` keeps only `premium`). Nothing after a duel
touches it, so the original's duels earned no BP. The server pays BP for duels anyway (each result counted by the
hub: win 100, draw 50, loss 30; **guess**, a server choice), keeps it with the account, and re-reads it for the
login replies and each purchase (charged against the stored account, so BP given since the login can be spent).

**Revival's BP push** (Revival only, no original counterpart). When an account's BP changes outside a purchase (a duel
result, the host's give-bp), the server pushes `0x7A11 {revival_bp}` to that account's online players
(`Hub::push_bp`). The client ignores the id: the global dispatcher (`0x4F4A00`) has no case for it and returns through
its default (`0x4F6057`). The connection DLL, which wraps that dispatcher call (`0x4F44BD`), sets UserData `+0x76C`
(UserData from its singleton getter `0x40A500`, no arguments) to `revival_bp` (`tools/mod/bp.h`). Screens read the
value when drawn, so an open screen shows it on its next visit. Live (2026-11-12): with the DLL, the shop opened after
a gift shows the new total without a login; with an older DLL the push was ignored on the main screen and in the shop
and the client kept working. Status: confirmed.

Each pack file's card list is ascending runs: 44, 18, 9, 9 for an 80-card pack, the OCG split of Normal, Rare, Super
and Ultra Rares. The server reads the runs as rarity tiers (**guess**) and puts four cards of the first tier and one of
a higher one in a pack (**guess**: the original's pack size and odds are unknown); a pack of one run gives five of it.

### Events (TournamentScene, UserEventScene)

The map's Official Event tile (`MenuSelect` 21) opens TournamentScene, Championship (20) a 64-player tournament map
that sends nothing, and User event (22) UserEventScene. With no decks, Official Event and Championship stop at "There
are no valid Decks available." and OK returns to the map.

| Request                                                                                                          | Reply (error)                          | Fields                                                                                                                                                | Evidence                                                                 | Status                            |
| ---------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------- |
| `0x177A` event list (bare, `0x4D0BB0`)                                                                           | `0x177B` (`0x177C`, dialog `0x4F7940`) | top `result_number`; items `{event_type, status, event_id}`. TournamentScene looks for `event_type` 10 with `status` 12; none means its no-event path | `0x4D0BB0`; the Stadium opens                                            | shape (empty)                     |
| `0x1789` event search `{search_flag, pname, status, pack_type, base_point, front_num 30, back_num}` (`0x4D9E37`) | `0x177B` (`0x177C`)                    | as above; the list case `0x4D66A0` reads `result_number` (tracer) and `premium` from another dataset                                                  | captured request; live created/participating user events described below | confirmed (see User events below) |
| `0x339A` own user event (bare, `0x4D9C2B`)                                                                       | `0x33FE` (`0x33FF`)                    | `remain`, `event_id`: the scene keeps `event_id` only while `remain` ≠ 0 (its own event: relation 0 in the list, Create off)                          | `0x4D9BB0`; live                                                         | confirmed                         |

**User events** (Satellite Plaza; Create needs `premium`). The list `0x1789` → `0x177B {result_number}` has a row per
open or running event: `event_id`, `event_name`, `comment`, `status` (12 open, 13 running, 14 finished), `join_flag`
(non-zero crosses the row out and the details offer only Close: we send 1 only for a Participation Limit above OFF,
whose value is 1), `password1`, `prize_type`, `entry_max`, `entry_num`, `match_type`, `regulation_id`, the levels,
`eval_level`, `lang`, the duel settings and the host's `player_id`/`pname`. The client works out `user_relation`
itself (`0x4D6970`: 0 own, 1 open, 2 restricted, 3 running, 4 finished; the lobby 5), which with `numMenuButton`
(`user_event_detail.swf`: bit 0 Close, 1 Participate or Start, 2 Observe or Cancel) picks the buttons.

| Request                                                                                                                                                                                                                                                            | Reply (error: next id)                                                                                                                         | Fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Evidence                                                                                          | Status                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| Create `0x339B` (bare)                                                                                                                                                                                                                                             | `0x3400 {create_status}` (`0x3401`)                                                                                                            | 0 opens the form; bits 4 premium needed, 8 already in another event, 2 wait, 1 full                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | live                                                                                              | confirmed                                     |
| the form `0x3390 {event_name, comment, entry_max, prize_type, level_min, level_max, eval_level, player_limit, password1, lang, match_type, duel_surrender, duel_watch, duel_time_turn, duel_time_total, duel_lp_start, regulation_id, join_max}` (top level, live) | `0x33F4 {event_id}`                                                                                                                            | then `0x1784 {event_id}` → `0x1785 {join_id, event_id}` (the global dispatcher keeps `event_id`, which the lobby asks with: 0 until we echoed it; we echo only a user event's, numbered from 100: echoing Auto Duel's 2 made the clients queue again by themselves after each match, live) and the entry `0x3397 {event_id}` → `0x33FA`                                                                                                                                                                                                                                                                                                                                                                                                                     | live                                                                                              | confirmed                                     |
| Participate: `0x1784`, `0x3397`; leave `0x178E {event_id}`                                                                                                                                                                                                         | `0x33FA`, `0x178F`                                                                                                                             | the others get `0x3399 {entry_name, entry_type: 1 joined, else left}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | live (join), test                                                                                 | confirmed                                     |
| lobby `0x177F {event_id}`                                                                                                                                                                                                                                          | `0x1780` item[0]                                                                                                                               | `official` 0, `player_id` the host (compared with the client's own, `0x4D0B56`: lobby `+0x2E0` 1 host, 2 entrant, 3 observer), and the row's fields, which the Event Menu (the menu bar's duel button) shows                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | live                                                                                              | confirmed                                     |
| `0x32CA {event_id, start_index}`                                                                                                                                                                                                                                   | `0x32FD {total_num, items {player_id, player_name, remain, avatar1}}`                                                                          | the seats of the bracket: `0x4D13F0` keeps the list (`+0x3E8`), and `0x4CEBE0` looks each seat's `player_id` up in it for `player_name`, `country`, `remain` and `avatar1` (read by `0x531FF0`: first item whose field matches). We send name, face and `remain` 0; `country` is the player's flag (see "Country"; left out for a player who is not online)                                                                                                                                                                                                                                                                                                                                                                                                 | `0x4D13F0`, `0x4CEBE0`; live                                                                      | confirmed                                     |
| `0x32CE`                                                                                                                                                                                                                                                           | `0x3305 {phase_block1–3, remain}`                                                                                                              | the blocks the round plays in, lit on the bracket map (`0x4CF4A0`: -1 for none); we send the blocks of the round's duels                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | `0x4CF4A0`; live                                                                                  | confirmed (shape)                             |
| `0x32CC`                                                                                                                                                                                                                                                           | `0x3301 {timer}`                                                                                                                               | seconds to the next round (`0x4CC5B0` keeps `now + timer`; `0x4CEAD0` shows it as the lobby's TIME `hh:mm:ss`); 0 none                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | live                                                                                              | confirmed                                     |
| `0x32CD`                                                                                                                                                                                                                                                           | `0x3303 {alive_num, entry_num, watch_num}`                                                                                                     | the "Duelist alive/entries" counts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | live                                                                                              | confirmed                                     |
| `0x32C9`                                                                                                                                                                                                                                                           | `0x32FB {event_id, status, phase, round, total_round, winner, winner_name, second, second_name, tournament_id, championship_id, year, season}` | `status` is the lobby's state (`+0x2F8`, texts in `0x4CE670`): 0/1 "Event coming up", 2 Draw Announcement, 3 Entry Draw, 4 "Match-ups coming up", 5 Match-ups Announcement, 6 "Prepare to Duel", 7 "Duel in Progress", 8 "Event Completed" (a user event adds 800 to the texts' ids); above 1 the host's Start/Cancel go. We send 0 open, 4 between rounds, 6 once a round is paired, 7 when a player sat at one of its tables, 8 at the end. `total_round` is **the round's name**, not a count: the lobby shows the text `0x3BCD + n` beside the status ("1st Round" ... "6th Round", 7 Quarter-Final, 8 Semi-Finals, 9 Final); we send the round's name by rounds to go (see `Event::round_label`) and the same in `round`. `winner`/`second` at the end | `0x4CE670`, `0x4CC5B0`; live                                                                      | confirmed                                     |
| `0x32D5`                                                                                                                                                                                                                                                           | `0x3309 {map_data}`                                                                                                                            | a flag byte per bracket block, 0x49 of them (`0x4D0F40` copies them unchecked: without it the lobby crashed, NULL at `0x4D10A9`). A non-zero byte means the block exists (`0x4CF4A0`: zero greys it, and the lobby shows block 0 first when its flag is set). Blocks 0–0x3F are 8 seats each, 0x40–0x47 hold the champions of eight of those, 0x48 the champions of those                                                                                                                                                                                                                                                                                                                                                                                   | `0x4D0F40`, `0x4CF4A0`; live                                                                      | confirmed                                     |
| `0x32CB {event_id, block_index}`                                                                                                                                                                                                                                   | `0x32FF` with an item `{block_number, start_level, player_id1–8, win_count1–8}`                                                                | `0x4D10F0` asks for the block at its index (once the map says it exists) and keeps the item whose `block_number` matches (`0x531FF0`); `0x4CEBE0` draws it: a tree of 8 seats and 14 nodes (seats 0–7, the winners of round one 8–11, of round two 12–13). `win_count` is the rounds the seat has won in this block: 1 puts the player on nodes 8–11, 2 on 12–13, 3 makes them the block's champion (they go on to the next level's block). `player_id` 0 is an empty seat. A bye counts as a win. No item for an index without a block clears the bracket                                                                                                                                                                                                  | `0x4CEBE0`, `0x4D10F0`; live: names, faces and the advance lines drawn, the champion on the final | confirmed                                     |
| push `0x32D4 {flag}`                                                                                                                                                                                                                                               | (the lobby asks again)                                                                                                                         | `0x4CC5B0`: 2 reloads the entries and phase blocks (`0x4D13F0`), 3 the bracket (map `0x32D5`, then the block `0x32CB`), 4 the timer (`0x32CC`), 5 the counts (`0x32CD`), 6 the state (`0x32C9`). We send 2, 3 and 5 to the others when someone enters or leaves, and 6, 5, 2, 3 when a round is paired                                                                                                                                                                                                                                                                                                                                                                                                                                                      | `0x4CC5B0`; live                                                                                  | confirmed                                     |
| push `0x32D3 {round, total_round, status}`                                                                                                                                                                                                                         | (the round banner)                                                                                                                             | `status` 1 shows "Starting ..." (text `0x3C05 + total_round`), 0 "... has now finished" (`0x3C0E + total_round`, with a counts refresh); `total_round` is the round's name as above (9 "Starting the Final")                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | `0x4CC5B0`; live (Starting Semi-Finals dialog)                                                    | confirmed                                     |
| push `0x3307 {winner, winner_name, second, second_name}`                                                                                                                                                                                                           | (the end)                                                                                                                                      | a banner at the end of the event (winner 0: another text); the lobby keeps the names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | `0x4CC5B0`                                                                                        | shape (the banner's dialog was not seen live) |
| push `0x3301 {timer}`, `0x3302`                                                                                                                                                                                                                                    | (the wait)                                                                                                                                     | `0x3301` starts the TIME countdown, `0x3302` clears it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | live (TIME counting down 14, 5)                                                                   | confirmed                                     |
| Start `0x3395 {event_id}` (the host)                                                                                                                                                                                                                               | `0x33F8`                                                                                                                                       | everyone gets `0x3396` ("The event has started!"), then each pair the match push `0x1978 {vsPlayerData, duelData {duel_id, match_type, regulation_id, lp_start, surrender, time_turn, time_total}}`, which the lobby takes like the matching agent (`0x4CC60C`): Select Deck, janken, duel                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | live                                                                                              | confirmed                                     |
| Cancel `0x3393 {event_id}` (the host)                                                                                                                                                                                                                              | `0x33F6`                                                                                                                                       | the others get `0x3394` ("Event cancelled.", the lobby closes)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | test                                                                                              | shape                                         |

The server runs the event as a knockout: the first round pairs the entrants in order (an odd one goes through), each
duel's winner (the table's report) stays in, and when a round's duels are over the lobby waits (20 s: "Match-ups
coming up" and the TIME countdown, while the winners get through the result screens: a match push that arrives on a
result screen is lost) and the next round is paired, until one player is left and the event is finished (it leaves the
list; the host may create another). A player who left forfeits. The lobby's bracket, round banners and status follow
each step (the pushes above). **Kept across restarts:** the events document in `server.db` (once `events.json`) holds the events that are
open (with their entrants, form and the id counter), and a restart brings them back; a running event cannot come back
(its duels and the players' lobbies died with the server), so it ends at the restart without a winner. Live: three
players entered "Friday Cup", the open event survived a server restart (list 2/4 and the entries kept, entering again
was accepted), Start drew Yugi, Kaiba, Joey (Joey on a bye), "Starting Semi-Finals", Yugi beat Kaiba (Kaiba's client
closed mid-duel), the lobby showed the advance lines, "Match-ups coming up" with TIME 14, 5, then "Final", and Yugi won
and was crowned ("Event Completed").

**Ready/Standby:** the lobby swf (`official_event_lobby.swf`, sprite 172 frame 3) has the Ready and Not Ready buttons
(texts `0x3EFC`/`0x3EFD`, helps 16134/16135 "Duel event will commence once all participants are Ready"), but they
are the official tournament's "bosyuu" entry menu (`decideLobbyMenu`, `0x4D2340` and the code after it open dialogs
only): no request builder for a Ready or Standby message exists in the client (no id between the event requests
`0x3390`–`0x339B` and the lobby's `0x32C9`–`0x32D5` is built anywhere for it, searched in the decompiled client). The
host's Start is the only gate. **Impossible without a request to answer**, so none is sent.

### Decks (deck editor `DeckEditTest`, handler `0x43FC60`)

Hosting requires readable game card data and starting decks before accepting players (**confirmed**, startup
contracts for missing/corrupt archives). The former empty-world fallback could send a valid `9000` shape with
zero owned cards; a scratch-account A/B reproduced that behavior with missing host data and restored 218 distinct
owned cards when the host used the complete installation. This is a reproduced failure path relevant to
[issue #4](https://github.com/usersina/yo3-revival/issues/4), not a confirmed diagnosis of that reporter's installation.
The reply shapes below are unchanged; only explicit in-process protocol fixtures may opt into an empty world.

The client keeps decks as **local files**: `<My Documents>\Yu-Gi-Oh! ONLINE 3\players\<pname>\decks\<deck_name>.y3d`
(saved by `0x52EE60`, loaded by `0x52EB70`). Opening Edit Deck asks for the deck list and saves every listed deck the
player doesn't have yet (by name; `0x43E550` checks). The NPC duel (`0x4BD880`) then needs one local deck that
validates (`0x51AB00` → `0x51AA80`: 5 no deck, 2 sizes, 3 cards not owned, 4 regulation, 0 valid) against the
collection built from the trunk (`0x4F6340`) and the login regulation (`0x2201 regulation_id`, kept at connection
`+0xC8`). Sizes (`0x51AC50`): main 40–60, extra and side under 16. Regulation (`0x51A9A0`, `0x51A930`): a card may
appear as often as the index of the list naming it (`card0` 0 … `card3` 3), else `addition_card`; the list at the
default's index is skipped; cards 5280, 5434 and 6276 are never allowed.

The required mod prepares the first local deck automatically (**confirmed**, live fresh-player NPC duel).
After a successful `0x1F4C` selection, the first `0x1FA4` returns `9000`, `0x251C`, `0x252A`, `0x2D6C`, then
`0x22C8` with the existing tag-6 deck shape below. Candidates are the starter then official character decks that
pass the server's ownership/regulation check, at most ten; no eligible candidates means no extra reply. Subsequent
trunk refreshes send no automatic deck list until another player selection. The stock global dispatcher ignores
`0x22C8` (`0x4F4A4F` routes it to `0x4F6057`). Our mod observes it after native dispatch, once trunk slot 18 and
rental slot 8 exist, and runs the native checks (`0x51AB00`, `0x51AA80`) against the selected player's collection
and login regulation. Any existing local `.y3d` file, even an invalid/unfinished deck, or a nonempty saved deck
selection prevents setup.
Otherwise the first valid, absent candidate is written through `0x52C320` (the editor's serializer), published
without replacing files, and reloaded with `0x52F240`. Only an empty `deckInfo.cursor_deck_name` is set, persisted
through `0x52E200`. The mod records `revival-starter.initialized` in the player's local profile directory after
creating or discovering existing decks or a saved selection. Its presence prevents regeneration after manual edits
or deletion, across restarts. Existing files and nonempty selections are preserved. See the
[mod ABI evidence](client-internals.md#automatic-local-deck-initialization) and `deck_contract`,
`regulation_contract`, and the shared C policy test `starter_policy`.

| Request                                                                                                                                                                                                                                                               | Reply (error)                      | Fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Status                                                                          |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `0x1F4E` deck list (bare, `0x440C43`; also re-sent after save and delete)                                                                                                                                                                                             | `0x22C8` (`0x22C9`)                | items **tag 6**: `deck_id` (int), `deck_name`, `discription` (string), `card_protector` (int), `main`, `extra`, `side` (binary u16 BE CARD_Prop rows, read by `0x414410`); at most 10 are used. We list the starter deck, then the player's uploaded decks; the official character decks and bought structure decks are on the Official tab instead (see "The Official tab")                                                                                                                                                                                   | tracer: all found; "Starter Deck.y3d" written; the NPC duel opens; live: the Deck tab lists the starter and the uploads                                                                                                                                                                                                                                                                                                                                                                  | confirmed                                                                       |
| `0x1F50` delete `{delete_type, deck_name}` (`0x4449A9`, after deleting the local file `0x52F0C0`)                                                                                                                                                                     | `0x22CC` (`0x22CD` shows an error) | none                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | "deck delete succes"; live: the deck leaves the Load Deck list                                                                                                                                                                                                                                                                                                                                                                                                                           | confirmed; the server also forgets its uploaded copy of that name (below)       |
| `0x1F4F` update `{duel_id, event_id, update_type}` + the deck (tag 6) (every list is read up to 100 rows, the name up to 64 bytes; an update that names a duel passes the deck check, and one other than `update_type` 1 that fails it is ignored, answered `0x22CA`); Edit Deck's save through the connection DLL: `{update_type 1}` + the deck, no `duel_id` | `0x22CA` (`0x22CB`); none to an editor save | none                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | The stock editor's save only writes the local file (`0x4442E0` → `0x4159B0` → `0x52EE60`) and `cursor_deck_name`, though the editor has a "deck add or update succes" case for `0x22CA` (it asks for the list again); the connection DLL sends the record `0x415610` makes there (see "Decks follow the player"), unanswered. Otherwise the request comes from a duel's Select Deck (DuelManager `0x50ECF0`, see below); live: the duel scene loads ; Card Swap between a match's duels (`0x50FDF0` state 3) sends `update_type` 2 with the swapped deck (MainGamen `+0x838`), and the server keeps it as the player's entry for the match | confirmed (live: update_type 2 after Card Swap); live: an editor save (New, Save → overwrite) arrives with no `duel_id` |

**Decks follow the player.** The stock editor's Save and New only write the local file, and the editor refuses to save
a deck of the wrong size itself ("Your Deck must hold between 40 and 60 Cards. Side Decks must be 15 cards or less in
size."). A duel's Select Deck sends the whole deck it picked (`0x1F4F` `update_type` 1: `deck_id`, `deck_name`, `main`,
`extra`, `side`, `card_protector`), and the connection DLL (`tools/mod/deck-upload.h`) sends Edit Deck's saves the same
way: the editor (`DeckEditTest`, handler `0x43FC60`; save `0x4442E0`, reached from Save, its overwrite confirmation and
New) calls `0x4159B0`, which makes the deck record (`YO3Deck`, type 6, with `0x415610`) and writes it (`0x52EE60`). The
DLL redirects that one call to `0x415610` (`0x415A4D`; the site is checked before patching) and sends the record as
Select Deck does (`0x50EEAD`–`0x50EF8C`): `0x1F4F {update_type 1}` and the deck item (no `deck_id`), no `duel_id` or
`event_id`, through the front connection the editor itself uses (UserData `0x40A500` +`0x60`, component `-10507`,
`0x441340`; its `+0x74` sends). The request's item list does not own the record (`+0x50` cleared, as `0x50EF63` does),
since `0x4159B0` still writes and deletes it. Select Deck always names a duel, so an upload without `duel_id` is an editor
save: kept unchecked (a save is not a duel; Select Deck checks it later) and **unanswered**, because the editor reloads its
deck list on `0x22CA` (`0x4F33D0` sends `0x1F4E`, and the list's cursor resets) and opens a dialog (`0x444BE0`) on
`0x22CB`, and no timer waits for `0x1F4F` (see "Requests with a timeout"). The server keeps both kinds per player, by
name (a newer upload replaces it, ten at most, the oldest goes), and lists them after the starter and bought decks in
`0x22C8`; a client that lacks a deck of the name saves it (`0x43E550`), so a deck saved or used on one machine appears on
another. A machine that already has a deck of that name keeps its own copy (the client saves only missing names). Not
kept: a Card Swap's deck (`update_type` 2), the starter deck's name (the server's own copy wins), an empty deck. Live
(Select Deck): "Dragon Test" (a copy of the starter made with New) was picked for a duel, its local file deleted
(another machine), and Edit Deck → Load Deck listed it again, with its 40 cards, and wrote the file back. Live (editor,
2026-11-13, two headless clients of one account with separate prefixes and Documents): New → "Editor Upload" on the first
sent `0x1F4F {update_type 1}` with 40 cards and wrote the local file, with no dialog; the second machine's Load Deck
listed it and loaded its 40 cards. There, adding a card and Save → overwrite → OK sent the 41-card deck; on the first
machine, after a server restart and with its local file deleted, Load Deck brought back the 41-card version. Tests:
`deck_contract` `a_deck_saved_in_the_editor_is_listed_on_another_machine`,
`an_editor_save_is_kept_unchecked_but_not_an_empty_deck_or_the_starters_name`.

**Decks checked on the server.** At Select Deck (`update_type` 1) the server checks the deck as the client does
(`0x51AB00`): main 40-60 cards, extra and side under 16 (`0x51AC50`), every card owned (the starter deck, bought and
traded cards, rentals; counted per Konami id, so a bit more lenient than the client's per row), and the duel's
regulation (`0x51A9A0`): the rule's `regulation_id` (an NPC duel's `0x17AC` item, a hosted duel's or event's rule),
else the login one. A refused deck gets `0x22CB`: the client shows "You have been disconnected from the Duel Server."
(text `0x2919`, the only text that path has; `0x50ECF0` state 2 compares only `0x22CA` and `0x22CB`, stores `error_code` at `+0x90` and `0x510610` never uses it, and the texts "Invalid Deck" `0x2940`, "This Deck can not be used." `0x2942` and `0x295C` have no code that shows them: settled, no other reply or code is expected) and, at OK, leaves the duel scene for the map; nothing is kept for the
duel. A stock client can't send a deck the check refuses (it checks first, even the host before Create Duel: "There
are no valid Decks available."), so this is for modified clients; live with the testing aid `--duel-regulation 4` (the
starter deck breaks it): both Auto Duel players got `0x22CB`, the dialog, and the map. The opponent of a refused
player gets a `0x2009` with `duel_end` 3 (no counts): DuelScene shows text `0x2918` ("Time is up! This Duel will be
deemed a non-Result.") and, at OK, leaves for the map; the match ends, won by the opponent for an event's round. (A
duel-connection command 2 {1, 14} is ignored before the janken; a janken command first doesn't make it work either.)
Live with the asymmetric check (only Yugi's deck refused): Kaiba, at the pre-janken screen, got that dialog and went
back to the map. Card Swaps (`update_type` 2) that fail the deck check are ignored, with `0x22CA`.

**Match membership (server enforcement).** Before a deck update can enter or cancel a duel, its selected player
must belong to that match, or the NPC duel must have been created by this session. An outsider's Select Deck gets
the existing `0x22CB` refusal (`0x50ECF0`); other update types are ignored with `0x22CA` (`0x50FDF0`). Neither changes
the matched players' entries or cancels their match. Likewise `0x17D4` from an outsider gets only `0x17D5`, without
the `0x22C4` handoff (`0x50F030`, `0x50F690`). These are server authorization rules, not new wire shapes. Tests:
`regulation_contract` `an_outsider_cannot_upload_a_deck_for_another_players_match`,
`an_outsiders_invalid_deck_cannot_cancel_another_players_match`,
`an_outsider_is_not_sent_another_matches_duel_server_handoff`.

Every player gets one deck, "Starter Deck": the first `deck_type` 1 preset (`data/recipe`, ids 1–6, 501, 502; they
look like the starter decks) that the login regulation allows. Preset 1 holds a card regulation 1 forbids (4966), so
it is preset 2 (**guess**: the original let a new player choose). The server lists it every time, so a deleted
starter deck comes back the next time Edit Deck opens. After it the list holds only the decks the player's clients
uploaded (see "Decks follow the player"), so the ten the client takes are the player's own.

**The Official tab.** Load Deck's Official tab (`0x43D8D0`, "Show official Recipe.") reads nothing from the server:
for each entry of the item catalogue (`yo3pbininfo.bin`, data manager `+0x1C`, 8 bytes each) whose bit is set in the
player's owned-items bitmap (UserData `0x40F320` `+0x68`, the own record's `pbin`), of kind 5 or 8 and with a recipe
id (`a`, at `+4`), it loads the client's own recipe (`0x52DC10`, `data/recipe/`) and lists it under its name text. Every
recipe names its item (`pbin_no`): kind 5 the starter and structure decks (preset 2 → 773, GADGET BOX 502 → 2564), kind
8 every NPC's deck (Miracle Magic 20 → 615, Power of Bonds 261 → 1621). The server sets in the player's own record
(`0x2260`, `0x2CF3`) the items of the starter preset's recipe and the nine official character decks (every player's),
and of each structure deck the player bought: buying one already sets its bit in the client's own copy (PresentWindow
case 1), and the server keeps it for the next login. A deck loaded from the tab and saved (Save → New) becomes the
player's own, and is uploaded like any editor save; its cards are owned. Other readers of kinds 5 and 8 were not found
(the shop's protector list reads kind 11). Live (2026-11-13): the Official tab listed the nine character decks and
CONCEALERS DECK (the starter's recipe), the Deck tab only the starter and the player's uploads; Miracle Magic loaded
with its 42 + 3 cards and saved as the player's deck. A bought structure deck on the tab was checked only by contract
test (`shop_contract` `a_bought_structure_deck_is_on_the_official_tab`; `deck_contract`
`the_official_decks_are_on_the_official_tab`).

### NPC duels and duel start

The NPC roster comes from the main screen's startup poll `0x2710`; the global dispatcher keeps `0x2711` in slot 4.
SelectNpcScene (Kaiba Corporation) reads it three items at a time (`0x4BEB40`), one per level: the first item
gives `npc_type` (the tab, 0–2: Yu-Gi-Oh!, GX, 5D's, texts 15504–15506) and `npc_id`, each of the three gives
`need_ex`. Level j is mastered once the experience with its own id (`npcbin` entry 3; absent = 0) reaches its
`need_ex`, counting from the first; the list offers the number mastered (at most level 2) and shows it as stars, gold
once all three are (then the duel is "unlimited", regulation 3). A `need_ex` of 0 is mastered from the start (ours
were 0, 1, 2 until 2026-11-07, so every NPC opened on level 2); we send 100, 200, 300 (**guess**). The tabs follow the
series: NPCs 0–8 on tab 0, 9–16 on tab 1, 17–24 on tab 2 (**guess** from the names; the free NPCs, first ids 120,
123, 126, 147, 150, 153, 171, 174, 177 at `0x79039C`, are three per series). An NPC is enabled when the login's
`premium` isn't 0 or it is a free one (`0x4BF520`). NPC ids start at 120 (`0x78`), three per NPC: names are text
`0x5208 + 2 * ((id - 120) / 3)` (`0x4BFE34`), deck titles text `0x5FB4 + 2 * id` (`0x4BFE05`), and each NPC deck
preset (`deck_type` 3) carries that title id as `name_id`, which is how the server finds the ids: 25 NPCs (a 26th,
ids 195–197, has decks but no name and would be a blank row, so it is left out). Hovering an NPC computes
`sum(min(ex, need_ex)) * 1000 / sum(need_ex)` over its levels (`0x4BFDC4`, the Friendship Level bar): all-zero
`need_ex` divides by zero. Seto Kaiba, Yami Marik, Alexis Rhodes and Jack Atlas (first ids 123, 141, 153, 174) refuse
to duel ("Who the hell are you?", speech kind 36) unless the record's `pbin` has bits `0x250`–`0x253` (`pbin[74]`
bits 0–3, `0x4BDAB0` state 2); the server sets them in every record.

**Experience** (`npcbin`, a raw field of the player record, read by `0x51C680`): per NPC level id, four u16
little-endian at `8 * (id - 1) + 2k`: k 0 wins, 1 losses, 2 draws (the info panel's "Wins %d / Losses %d" sums the
three ids', `0x4BFBA0`), 3 experience. No request reports an NPC duel's result; the server keeps it against the
`cpu_id` (the duel records, keyed by player and opponent) and counts experience 100 a win, 50 a draw, 10 a loss
(**guess**). The client reads it unchecked up to id 197, so the blob is 2048 bytes; it comes with the player list
(`0x2260`) and the refresh (`0x2CF3`), which the client asks for after a duel. Live: after Yugi beat Seto Kaiba
(level 1, `--duel-lp 1000`), the list showed him with two stars, the Friendship Level bar a sixth full and "Wins 1 /
Losses 0".

Picking a level (`decideNpc` `0x4C0C80`), its speech and "Start Duel?" lead to the duel start step `0x4BE390`. It
finds the roster item whose `npc_id` is the first id + level and hands it to `DuelMatchingAgent` (`0x4A0B50`) in mode
3; that item becomes the agent's record and, later, the opponent's `PlayerData` (MainGamen `+0x408`). The agent
checks the deck (`0x4A0E90`; the item may carry `regulation_id`, else the player's), then:

| Step       | Request                                                                                                                                                                                                         | Reply (error)                          | Fields read                                                                                                                                                                    | Evidence                                                          | Status                                                   |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- | -------------------------------------------------------- |
| `0x4A1090` | `0x1784 {event_id}` (1 for an NPC duel)                                                                                                                                                                         | `0x1785` (`0x1786`)                    | `join_id` (agent `+0x58`); the whole reply is copied to MainGamen `+0x868`; the global dispatcher (`0x4F4AE7`) stores `event_id`, `join_id`, `server_id`, `event_score` (ints) | live: client then sends `0x17AC`, trace "event join offer notice" | confirmed; `join_id` = a server-wide counter (**guess**) |
| `0x4A1790` | `0x17AC {event_id, join_id}` + item (tag 4) `{cpu_id, duel_server_id, match_type, regulation_id, surrender, time_total, time_turn}`; live: `cpu_id` 124 (the NPC id), `match_type` 4, `surrender` 1, the rest 0 | `0x17AD` (`0x17AE`, dialog `0x4F7940`) | `duel_id` (agent `+0x5C`); the reply is copied to MainGamen `+0x88C` with the agent's `regulation_id`; `country` is read from the player's own data                            | live: the scene opens DuelManager's Select Deck                   | confirmed; `duel_id` = a server-wide counter (**guess**) |

Mode 1 then asks for a password (`duel_password_flag` from the agent's record), mode 2 goes to `0x4A1380`, mode 0
(the DUEL panel) waits for a match in `0x4A1D10`. Mode 4 (watching a duel: PlayerInfoWindow's Watch Duel button for
a player whose `duel_status` is duelling, shown only when the entry's `duel_watch` is not 0, `0x4B6510`; the server
sends `duel_watch` 1 and `duel_match_type` for players in a matched duel, the host's own for a hosted one) starts at `0x4A3340`: `0x1838 {duel_id}` → `0x1839` (`0x183A` = error, back
to the map through `0x4A35F0`), which sets MainGamen `+0x8B8` = 5 and `+0x8BC` = 1 (watching) and is read twice:

| Reader     | Fields                                                                                                                                                                                                                                                                                                                                                                                    | Status |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| `0x4A3340` | `host` (string), `port` (int): DuelServerConnection `+0x78`, `+0x94`                                                                                                                                                                                                                                                                                                                      | code   |
| `0x4A3650` | items found by `name` (`0x531EB0`): `avatar_1` and `avatar_2`, the two players' records (loaded into MainGamen's two PlayerData, as `vsPlayerData` is); top-level `card_protector_1`, `card_protector_2` (int); item `duelData` {`event_type`, `official`, `duel_id`}, copied in as the duel's rule (`0x41BB10`); `official` 0 and `event_type` not 5 pick the field `0x14 + duel_id % 8` | code   |

DuelManager then opens the duel scene in its watching state `0x50F320`: it connects DuelServerConnection and sends
**`0x1838 {duel_id}` on the duel connection** (not `0x200C`), skips `0x17D4` (`0x50F030` goes straight to `0x50F690`
when `+0x8BC` is set), and `0x50F690`, finding the duel connection open, skips the `0x22C4` wait and `0x200A`. From
there the scene reads engine commands like a player's. The server answers a spectator's `0x1838` from the duel's
table: seat 0's opening, then every engine message of the duel so far, replayed through the spectator's own translator,
then the clocks; after that it gets every event, never a prompt. The opening starts with seat 0's janken, because the
spectator's scene opens on the pre-duel screen like a player's: 1500, 1501 {result, opponent's hand, own hand} of the
decisive round, 1502 {0} (no choice to make), 1503 {turn order}; then setup `0x15`, both decks in the card table,
duel start and LP, with **both** players' hand, deck and face-down cards hidden (row 0). Without the janken the
spectator stayed on the pre-duel screen (live). A spectator present when a match's next duel opens gets that duel's
opening. At the match's end it gets seat 0's duel end (command 2) and then **1505 {seat 0's match result}**, which its
DuelResult takes as the match result (see below); without it the spectator went back to the pre-duel screen (live,
even with a `0x2009` pushed to its front connection, which a watching DuelResult never reads). The front refuses
(`0x183A`) a duel that isn't being played between two seated players, or a hosted duel whose `watch` isn't 1 (Allow
Watch Duel ON). Live: Joey watched Yugi and Kaiba's Auto Duel from turn 1 (both hands face down, the clocks running),
and when Kaiba quit, Joey's client went back to the map. Live (Match Mode, `--duel-lp 1000`): after Yugi won the first
duel, Joey's client went to the pre-duel screen and waited there while Yugi used Card Swap and both played the janken;
the second duel's opening took it to the field with the match's win marks (Yugi one), and at the match's end (Yugi 2–0)
it went back to the map.

DuelManager (scene `0xFFFFD8FD`, constructor `0x50E470`) picks its first state by MainGamen `+0x8B8`: 5 → `0x50F320`
(connect DuelServerConnection, then `0x1838` on it), 6 → `0x50F030`, else → `0x50ECF0` (the NPC duel):

| Step                   | Request                                                                                                                                                                                                | Reply (error)                                                                      | Fields read                                                                                                                                                                                                                                                 | Evidence                                                                            | Status                                                                                            |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Select Deck `0x50ECF0` | `0x1F4F {duel_id, event_id 0, update_type 1}` + the chosen deck (tag 6)                                                                                                                                | `0x22CA` (`0x22CB`, text `0x2919`)                                                 | none (`0x50EFC1` compares the id; the deck editor and the shop take `0x22CA` too)                                                                                                                                                                           | live: the duel scene loads                                                          | confirmed                                                                                         |
| `0x50F030` state 2     | `0x17D4 {event_id, join_id, duel_id}`, once the avatar object (`AvatarViewer` `0x4DC320`; an NPC opponent is set up by its vfunc `0x68`) reports state 2 or −1; skipped when MainGamen `+0x8BC` is set | `0x17D5` (`0x17D6`, text `0x2919`)                                                 | none                                                                                                                                                                                                                                                        | live: sent right after the duel scene loaded, before any play                       | confirmed (meaning: "ready", **open**)                                                            |
| `0x50F690` state 1     | (none: waits for front flag `0x8000`)                                                                                                                                                                  | push `0x22C4`, which the global dispatcher keeps in slot 20 and flags (`0x4F4F34`) | items found by their `name` field (`0x531EB0`): `duelData` {`host` string, `port` int}, and its sub-item `rule`, which replaces MainGamen `+0x88C` (`match_type` read from it). Host and port go to DuelServerConnection (`+0x78`, `+0x94`), which connects | tracer: `host`, `port`, `name`, `match_type` found; live: the duel connection opens | confirmed; `rule` = the item of the duel's `0x17AC` (**guess**: only `match_type` is read so far) |
| `0x50F690` state 2     | **duel connection**: `0x200A {player_id, autoSet}` (`autoSet` from the `duelOption` `auto_place`), unless MainGamen `+0x8BC` is set; then heartbeats `0x2724` every 30 s (`0x4F1730`)                  | no reply: the server starts the janken (see "Duel connection")                     |                                                                                                                                                                                                                                                             | live: `{autoSet 1, player_id 1002}`                                                 | confirmed                                                                                         |

### Duels between players (automatic matching)

The menu bar's duel button (`go_duel` in `main_menubar.swf`, enabled once the main screen unlocks) calls `GoDuel`
(`0x46FEB0`): with the matching agent idle it opens DuelMenu (`0x49D070`, component `0xFFFFD908`,
`duel_kind_menu.swf`); while the agent waits for a match it cancels the wait (`0x4A0BD0`). DuelMenu's callbacks
(`0x49D8F0`): `requestDataPlayers`, `requestDataAuto`, `decideAuto`, `requestDataDuel`, `decideDataDuel`, `cancelMenu`,
`openRegSelect`. Auto Duel offers Rule Format (Single, Match, Both) and Opponent's Level (Any, About the Same, Strong,
Very Strong), kept in the client's own option store as `autoMatching {match_type, level}` (like `duelOption`;
`match_type` 1, 3 or 6, see Match Mode under the result screen);
Create Duel (`duelMaking`: password, watch, lang_mask, detail, lp_start, time_turn, time_total, regulation, party…)
is below. Confirm starts the agent in mode 2, whose deck check (`0x4A0E90`) needs a valid local deck
(`0x51AB00` → 5 otherwise, "There are no valid Decks available."). The required mod prepares the first local deck
after login, as described in [Decks](#decks-deck-editor-deckedittest-handler-0x43fc60).

| Step                 | Request                                                              | Reply (error)                  | Fields read                                                                                                                                                                                                                    | Evidence                                                                                                                                                                                              | Status                                  |
| -------------------- | -------------------------------------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| DuelMenu `0x49D310`  | `0x17FC` (bare)                                                      | `0x17FD` (`0x17FE`)            | `duel_playing`, `duel_ready`, `duel_auto` (menu `+0x58`, `+0x5C`, `+0x60`), shown as "Duel in Progress", "Create Duel" and "Auto Duel" counts                                                                                  | live: the Duel Menu opens with the counts                                                                                                                                                             | confirmed; counts = players (**guess**) |
| `0x4A1090`           | `0x1784 {event_id 2}`                                                | `0x1785`                       | `join_id`                                                                                                                                                                                                                      | live                                                                                                                                                                                                  | confirmed                               |
| `0x4A1380`           | `0x1798 {event_id, join_id, match_type, level}` (the menu's choices) | `0x1799` (`0x179A`)            | none                                                                                                                                                                                                                           | live: "Waiting for opponent." with Cancel                                                                                                                                                             | confirmed                               |
| `0x4A15D0` (Cancel)  | `0x17A2 {event_id, join_id}`                                         | `0x17A3` (`0x17A4`)            | none                                                                                                                                                                                                                           | code                                                                                                                                                                                                  | shape                                   |
| `0x4A1D10` (waiting) | push `0x1978`                                                        |                                | goes to `0x4A2D30` at once                                                                                                                                                                                                     | live                                                                                                                                                                                                  | confirmed                               |
| `0x4A2D30`           |                                                                      | push `0x1978` (`0x17CC` fails) | item `vsPlayerData` (by `name`): loaded into the opponent's PlayerData (MainGamen `+0x408`) like the player list's records; item `duelData` (by `name`): copied into MainGamen `+0x88C` as the duel's rule, its `duel_id` kept | live: "LV1 Kaiba / Default / Starting Duel / Single Mode", then the duel starts as an NPC duel does (Select Deck with a 30 s timer and an Opponent chat tab, `0x17D4`, `0x22C4`, the duel connection) | confirmed                               |

The wait step also takes `0x17B6` (another player's offer: `playerInfo {name, player_id, pname, dlv, title}`; a
player on the black list is declined at once, else a dialog asks) and answers `0x17CA {event_id, join_id, duel_id,
answer}` (2 accept, 1 decline); accepting goes to `0x4A2D30`. That is how hosted duels (mode 1, `0x4A28F0`:
`0x17B6`–`0x17B8`, `0x17C0`, `0x17CA`) meet; see below.

**Hosted duels (Create Duel).** The Duel Menu's Create Duel (`decideDataDuel` `0x49EE90`) starts the agent in mode 0
with the `duelMaking` choices. Challengers find the host in player search and send a request from its Player
Information window (PlayerInfoWindow, `0x4B6060`/`0x4B6510`/`0x4B8730`/`0x4B8F30`, `decide` `0x4B9D00` → mode 1; its
Watch button → mode 4):

| Who        | Request                                                               | Reply / push                                                                                                                                                                                                                              | Fields                                                                                                                                                                                                                                                                                                                                                                                                                          | Evidence                                                                | Status    |
| ---------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | --------- |
| host       | `0x17AC {event_id 1, join_id}` + item (tag 4)                         | `0x17AD {duel_id}`, then the wait `0x4A1D10` ("Waiting for opponent.", Cancel)                                                                                                                                                            | live item: `{match_type 1 (Single), surrender 1, level 1 (Any), party 1 (limits off), password "", watch 1, lang_mask 127, lp_start 8000, time_turn 180, time_total 600, regulation_id 1, status 1, duel_server_id 0}`; an NPC duel's item has `cpu_id` and no `status`                                                                                                                                                         | live                                                                    | confirmed |
| host       | `0x17F2 {event_id, duel_id}` (Cancel, `0x4A2500`)                     | `0x17F3` (`0x17F4`)                                                                                                                                                                                                                       | none                                                                                                                                                                                                                                                                                                                                                                                                                            | live                                                                    | confirmed |
| challenger | `0x2CF0 {search_flag 0x40000, player_id}`                             | `0x2CF7`                                                                                                                                                                                                                                  | the host's entry: `duel_status` 1 (Duel button; 0x4B6060: 0 none, 1 and 2-with-`duel_id` challengeable, others duelling: Watch), `duel_id`, and `duel_match_type`, `duel_regulation_id`, `duel_watch`, `duel_party`, `duel_password_flag`, `duel_level`, `duel_lang_mask` (the client checks the level and language limits), `duel_surrender`, `duel_lp_start`, `duel_time_turn`, `duel_time_total` (the Duel Conditions panel) | live: "Duel Conditions" with the host's choices and Send Duel Request   | confirmed |
| challenger | `0x17B6 {event_id, join_id, duel_id, password}` (`0x4A28F0`)          | `0x17B7` ("Requesting Duel"), or `0x17B8 {error_code}`: 3000 "Another player is requesting a duel.", 3001 "Already Duelling", 4006 "Password is Incorrect.", 201 "Duel Request was refused.", others "A transmission error has occurred." | the server answers 201 to a challenger outside the host's Opponent's Limits (below)                                                                                                                                                                                                                                                                                                                                             | live                                                                    | confirmed |
| host       | push `0x17B6 {duel_id}` + item `playerInfo` (the challenger's record) |                                                                                                                                                                                                                                           | the wait step reads `player_id` (a player on the black list is declined at once), `pname`, `dlv`, `title`                                                                                                                                                                                                                                                                                                                       | live: "LV1 Kaiba / Default / You have a Duel Request! Will you accept?" | confirmed |
| host       | `0x17CA {event_id, join_id, duel_id, answer}` (2 Yes, 1 No)           | `0x17CB` (`0x17CC` fails, `0x4A2D30`)                                                                                                                                                                                                     |                                                                                                                                                                                                                                                                                                                                                                                                                                 | live                                                                    | confirmed |
| challenger | push `0x17CA {answer}`                                                |                                                                                                                                                                                                                                           | 2: on to `0x4A2D30`; else refused                                                                                                                                                                                                                                                                                                                                                                                               | live                                                                    | confirmed |
| challenger | `0x17C0 {event_id, join_id, duel_id}` (its Cancel)                    | `0x17C1` (no `0x17C1`/`0x17C2` is compared anywhere and the global dispatcher ignores both: the id is free); the host gets push `0x17C0`, closes the offer and waits again                                                                |                                                                                                                                                                                                                                                                                                                                                                                                                                 | code                                                                    | shape     |

**Opponent's Limits** (Create Duel: OFF, Friend, Team) is the host's `party`: 1 off, 2 friends, 3 team. Challengers see
it as `duel_party` ("Opponent's Limits Friend" in Duel Conditions), and PlayerInfoWindow (`0x4B6510`, button 0) offers
Send Duel Request for 2 only when the host is in the viewer's friend list (`+0x49C`, from slot 6) and for 3 only to
a team member (`+0x49E`). The server checks the same on `0x17B6` (201 otherwise); Team permits only members of the
host's team (see the Teams section). Live: Yugi hosted with Friend; Joey's Player Information showed no Send Duel Request,
Kaiba's (his friend) did, and Kaiba's request reached Yugi.

On Yes both get the match push `0x1978` (the host's `duel_id`; `duelData` carries `match_type`, `regulation_id` and
`lp_start`, which becomes the duel's starting LP) a second later, and the duel runs as an automatic match's. The
host's `regulation_id` also stays in `0x22C4`'s rule and the server's deck check (`0x51A9A0`), rather than falling
back to the login regulation after acceptance. Tests: `matching_contract`
`a_challenge_reaches_the_host_and_acceptance_starts_the_duel`, `regulation_contract`
`a_hosted_banlist_is_enforced_with_an_unlimited_login`, `a_hosted_unlimited_duel_overrides_the_logins_banlist`.
Live: Kaiba found Yugi's
hosted duel in player search, sent the request, Yugi accepted, and both reached the janken.

The server pairs the first two players waiting for the same `match_type`, gives the duel a `duel_id`, and pushes
`0x1978 {vsPlayerData: the opponent's record, duelData {duel_id, match_type}}` to the one who waited at once and to the
other a second after its `0x1799` (**guess**: the agent takes one message per step). Select Deck's `0x1F4F` then
keeps each player's deck with the opponent for the duel server, and `0x17D4` announces it as for an NPC duel.

So an NPC duel does go through the duel server: the server announces it with `0x22C4`, the client connects and asks
`0x200A`. The duel connection uses the same framing as the front but opens with no version string. Other readers of
`duelData` (agent `0x4A2D30` for the DUEL panel: `duel_id`, `vsPlayerData`, `country`; mode 4 `0x4A3650`: `duel_id`,
`event_type`, `official`) show the PvP shape of the same push.

## Duel connection

What the server does when a duel can't go on (all **server choices**, none of it seen from the original): a player
leaving or a client disconnecting at any point before the match is over (the janken and turn order included) sends the
other seat command 2 {1, 14} (won, disconnected) and its `0x2009`; only a duel that had begun is counted. Left at the
janken or turn order, that `0x2009` has `duel_end` 4 (`front::duel_left`): the janken screen ignores command 2 and
`duel_end` 1 (live: the player stayed there for good). An engine
that dies, hangs for 30 s or keeps refusing the answer to one prompt ends the duel for both seats with command 2 {3, 0}
(a draw, no reason text), counted once as a draw. A duel connection that arrives after its match ended is sent command 2
{1, 14} and ignored, and its front connection the cancelled duel's `0x2009` (`duel_end` 3, which the pre-janken screen
answers). A table whose second player never connects is cancelled after `table::NO_SHOW` (90 s; the client may have
closed after Select Deck): the one waiting gets the same `0x2009`. A match neither player joins (both match pushes lost,
say) ends after `front::NO_JOIN` (3 minutes) with no winner, so an event's round goes on. An NPC duel's Select Deck is
refused (`0x22CB`) when its rule's `cpu_id` names no NPC the server has, with a deck: the duel connection would have had
nothing to open (live before the check, the client sat after the janken). A server without the client's NPC data (no
game files, as in CI) knows no NPCs and skips the check, keeping its placeholder opponent. `0x200A {player_id}` is claimed only from the address of the front connection that picked the deck.
The turn order `0xD` counts once per janken.

Every message is key/value (tag 4) with a `duelRequestName`. The client sends request `0x200C` (header built by
`0x49F990`), except its first message `0x200A`. The server's messages go through the pump `0x5233A0`, called every
tick by DuelServerConnection: it pops one message, ignores any without a `request` field, switches on
`duelRequestName` (`0x5201A0`) and returns the request id; a negative one counts as a lost connection (`0x4F1860`).
We send request `0x200D` (**guess**: no reader).

| Server `duelRequestName` | Fields                                                                                       | What the client does                                                                                                                                                                                                                                                                     | Status                 |
| ------------------------ | -------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `0x10`                   | `BusyEffectID`                                                                               | `0x520AB0`                                                                                                                                                                                                                                                                               | shape                  |
| `0x11` engine command    | `id`, `param1`…`param4`                                                                      | queued as `{id, request, param1..param4}` (`0x523541`) on the engine (`0x82D2C0` `+0x54`); scenes pop one when their queued states (animations) are done (`0x522A50`, `0x4D3220`)                                                                                                        | confirmed (live)       |
| `0x12`                   | `comIDsize`, `helpIDsize`, `otherFlagSize`, `COMIDDATA` (raw, u32 BE)                        | command ids per card, help ids, flags                                                                                                                                                                                                                                                    | shape                  |
| `0x14` timers            | `timerKind`, `start`, `count1`, `total1`, `count2`, `total2`, `other` and their `_max`       | sets the engine's five clock slots (`0x520A00`: value, now, stopped, max) — 0 `count1`, 1 `count2`, 2 `total1`, 3 `total2`, 4 `other` — then `timerKind` 0 or 2 runs (`start` ≠ 0) or stops slots 0 and 2, 1 or 3 slots 1 and 3, 4 runs slot 4 (pump case `0x52396A`). See "Duel clocks" | confirmed (live below) |
| `0x15` duel setup        | `FirstPlayer`, `PlayerNum`, `PlayerType001/002`, `MainDeckNum001/002`, `ExtraDeckNum001/002` | stored on the engine (`+0x1AC`…`+0x1E4`)                                                                                                                                                                                                                                                 | shape                  |
| `0x7A12` (Revival)       | `slot`, `text` (UTF-8)                                                                       | dropped by the stock pump (cases `0x10`–`0x15` only, `0x52341C`); the connection DLL keeps the text as duel prompt id `0x3F00 + slot`. See "Server texts" | ours (DLL contract)    |

| Client `duelRequestName` | Fields                                                                                                                                                           | Sender                                    |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| 1                        | `size` …                                                                                                                                                         | `0x520C20`                                |
| 2                        | `CommandID`, `index`, `player`, `position` (a card command)                                                                                                      | `0x520FD0`                                |
| 3                        | `phase`                                                                                                                                                          | `0x521180`                                |
| 4                        | `DLGResult`, `debug` (a dialog answer; the card list's number of picks)                                                                                          | `0x521A70`, from each dialog's close      |
| 5                        |                                                                                                                                                                  | `0x5212B0`                                |
| 6                        | `SetIndex` (the card list's first pick)                                                                                                                          | `0x521BC0`, from the card list `0x4C74F0` |
| 7                        | `index` (a card list position), `data` (its pick order, also kept in the high byte of the `0x3F1` entry)                                                         | `0x521CC0`, once per pick                 |
| 8                        | `player`                                                                                                                                                         | `0x521980`                                |
| 9 / 0xA / 0xB            | debug: `DebugID`; `wCardID`, `fFace`, `fTurn`; `id`, `type` (with `index`, `player`, `position`)                                                                 | `0x5213C0`, `0x521560`, `0x5217C0`        |
| 0xC                      | `selectHand` (janken: 0 rock, 1 scissors, 2 paper)                                                                                                               | `0x4F1330`                                |
| 0xD                      | `choice` (1 first, 2 second)                                                                                                                                     | `0x4F1430`                                |
| 0xE                      | none: Card Swap is done (state 4, once the front answered its `0x1F4F` with `0x22CA` or `0x22CB`); then the next janken (`0x50F690`), so the server answers 1500 | `0x50FDF0` (DuelManager)                  |
| 0xF                      | `player`, `autoSet` (duel options; live on entering the duel field: `{player 0, autoSet 1}`)                                                                     | `0x515790`, `0x4A0390`                    |
| `0x7A13` (Revival)       | `seq`: the batch ending with settle marker `0x5F1 {seq}` has played (issue #11; see "Duel clocks")                                                                 | the connection DLL (`duel-observer.h`)    |

**Janken** (who goes first; JankenScene `0x518400`, `duel_janken.swf`). Engine commands, all confirmed live:

| Command | Params                                                                                                                     | Scene                                                                                                                                                                                                                                                                                                                                                                             |
| ------- | -------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1500    | none                                                                                                                       | `beginJanken`, then the hands (`0x518D80`); sent after `0x200A`                                                                                                                                                                                                                                                                                                                   |
| 1501    | param1 result (0 draw, 1 win, 2 lose), param2 the opponent's hand, param3 this player's hand                               | `resolveJanken(myHand, yourHand, result)` (`0x518F50`); after a draw the scene asks for a new hand by itself (`selectJanken` `0x518ED0`), so the server sends nothing more                                                                                                                                                                                                        |
| 1502    | param1 ≠ 0: this player chooses                                                                                            | `selectFirst` (FIRST TO GO / SECOND TO GO), else `beginFirst` (wait) (`0x5191A0`)                                                                                                                                                                                                                                                                                                 |
| 1503    | param1 1 = this player goes first                                                                                          | `resolveFirst(isFirst)` (`0x519230`); once the movie is done the step leaves the scene (`0x519370`) and the duel field opens                                                                                                                                                                                                                                                      |
| 1505    | param1 the match result (MainGamen `+0x820`; the other's at `+0x84C`, `0x504D00`); param2 100 or param3 3 take other paths | **not part of the janken**: the match's end before another duel (`0x5185A3`): a DuelResult, or with param3 3 text `0x2918`, then the scene leaves. DuelScene drops it unless param3 is 3 (`0x515183`). We sent it after 1503 until traced: `0x5185A3` never ran, DuelScene took it. A spectator's DuelResult takes it as the match result (`0x512070`); the server sends it there |

The server plays the CPU: a random hand; when the CPU wins it goes first (**guess**).

**Against another player** (a matched duel: both entries carry the `duel_id`): both duel connections sit at one table
(`table.rs`). 1500 goes to both once the second `0x200A` arrives; with both hands in, each client gets its own 1501
(its result, the other's hand, its own) and the winner 1502 {1}, the loser 1502 {0}; a draw lets both scenes ask
again. The winner's `0xD {choice}` gives each client its own 1503. One engine runs the duel, and each client has its
own translator in which it is player 0 (`PlayerNum` 0, its deck first, `FirstPlayer` from its side): the engine's
events go to both, each prompt to the client it asks, and either client's commands reach the other through its
connection's outbox. The opponent's hidden cards (deck, hand, extra deck, face-down) go with row 0 in 1000 and `0x1A`
and face-down monsters get no field record (**confirmed** live: card backs with no stats; only the original server's own
placeholder value, like Online 2's, is unknown). NPC duels do the same for the
NPC's cards (`duel::npc_translator`, since 2026-11-13): the client draws any card it has a row for, face-down or not,
so with rows the attack-target window and the card info panel showed an NPC's face-down monster in full (live,
v0.0.10, Pegasus' Toon Masked Sorcerer). Live after the change: his face-down monster showed as a card back in both,
with no stats, and with its art once flipped in battle; his own summon cut-in still named its card. A client that leaves loses:
the other gets command 2 {1, 14 "has disconnected"}. Both front connections get `0x2009` with the record against
each other. Live (Yugi against Kaiba, LP 2000): Kaiba saw Yugi's Giant Orc on the opponent's side and Yugi's hand
face down; Kaiba's Set card showed face down on Yugi's field; the direct attack ended the duel with WINNER on Yugi's
client and LOSER on Kaiba's, and Next took both back to the map.

**Opening** (sent right after 1503; live: LP 8000 on both sides, Turn 1, the first player's DP lit in their colour):

| Message                               | Fields                                                                                                                                                                                                                                                                      | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Status                                                                                                                                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| setup (`0x15`)                        | `FirstPlayer` (engine player 0/1), `PlayerNum` (this client's player: 0; the scenes compare players with it, engine `+0x1B0`), `PlayerType001/002` (0 / 1), `MainDeckNum001/002`, `ExtraDeckNum001/002` (the decks' sizes: the one picked at Select Deck, the NPC's preset) | pump case `0x15`; the counts are the engine's per-zone counts (`+0x1C8`, `+0x1CC`, …) that DeckReset shows                                                                                                                                                                                                                                                                                                                                                                                                             | confirmed (live: the deck piles below); `PlayerType001/002` are stored at engine `+0x1C4`/`+0x1DC` (`0x523635`, `0x523676`) and no reader of either offset exists, so their values are immaterial (settled) |
| command 1                             | none: duel start                                                                                                                                                                                                                                                            | DuelScene `0x514290`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | shape                                                                                                                                                                                                       |
| command `0xF`                         | param1 player, param2 LP                                                                                                                                                                                                                                                    | DuelCockpit `0x502EB0` (`setLp`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | confirmed (live)                                                                                                                                                                                            |
| command 5                             | param1 player, param2 phase (0 draw)                                                                                                                                                                                                                                        | DuelScene, DuelCockpit                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | confirmed (live: DP lit)                                                                                                                                                                                    |
| command 6                             | param1 the player whose turn starts                                                                                                                                                                                                                                         | DuelScene; the pop adds 1 to the turn count (engine `+0x1BC`) and flips the active player                                                                                                                                                                                                                                                                                                                                                                                                                              | confirmed (live: Turn 2), so not sent for the first turn                                                                                                                                                    |
| command 1000 (one per card, before 1) | param1 uid (low 24 bits; the top byte is a flag, meaning unknown), param2 CARD_Prop row, param3 the row's Konami id (as in all 53,586 entries of Konami's 227 original logs; hidden cards 0), param4 table place `player \| zone<<8 \| index<<16`                                                                                                    | the pop's `0x521DF0` sets or adds the card in the engine's card table (engine `+0x90`); the scenes look cards up there by place (`0x523DF0`) or uid (`0x523D20` → param3, `0x523D40` → param2); a card object shows the row's image (`0x4FB090`, `data/card/<cid>_<n>.png`)                                                                                                                                                                                                                                            | confirmed (live)                                                                                                                                                                                            |
| deck piles = `0x16` DeckReset ×4      | {`param1` the player (compared with `PlayerNum`), `param2` the pile zone `0xC`–`0xF`}, for each player's deck (`0xD`) and extra deck (`0xC`), after the LP and before the first draw                                                                                        | DuelCardManager `0x4FF3A5`: the pile (`+0x4C + (side × 0x11 + zone) × 4`) takes the engine's count for that zone (`0x5240F0`: the setup's `MainDeckNum`/`ExtraDeckNum` at first) and is shown with its count on the field label (`0x4FE4D0` → `0x50BED0`), or hidden at 0. After that each pile counts itself: a card landing on the extra deck, deck or graveyard adds 1 (`0x4FBF30`), one leaving subtracts 1 (`0x5000E0`). Without DeckReset the deck pile starts at 0 and never shows (it was so until 2026-11-07) | confirmed (live: Yugi's deck pile 34 and Kaiba's 35 after the opening and Yugi's draw, then Kaiba's 34 after his; Yugi's one-card extra deck showed a pile of 1 on both clients)                            |
| draw = 1000 + `0x1A`                  | 1000 puts the card at hand index i; `0x1A` {param1 `uid \| kind<<16` (the move's kind, below), param2 from, param3 to} as move places `player \| zone<<1 \| index<<6`                                                                                                                                 | DuelCardManager `0x4FF6A2` creates the card object (`0x5003A0`) and moves it; deck → hand counts the draw and plays the draw sound; the pop moves the per-zone counts                                                                                                                                                                                                                                                                                                                                                  | confirmed (live: five cards in hand, five face down in the CPU's); the index bits are a **guess** (Online 2's packing)                                                                                      |

The server shuffles both main decks, numbers the cards 1.. (the client's deck, then the CPU's), sends them all to the
card table, starts the duel, sets LP, deals five cards each (the first player first) and then sends the Draw Phase. The
NPC's hidden cards go with row 0, as the opponent's do in a player duel (`duel::npc_translator`, since v0.0.11; above);
the client shows them face down.

The duel connection only names the player (`0x200A {player_id}`), so the front server keeps, by player, the deck sent
with `0x1F4F` and the NPC of the duel's `0x17AC` rule (`cpu_id`).

**Engine commands seen so far**, by the component that handles them (a scene or a component reacts to event `0x2001`,
which DuelScene `0x514140` sends for every popped command):

| Component (handler)               | Commands                                                                                                                                                                                                                                                                                                                |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the pop `0x522A50` (engine state) | 2, 4, 6 (turn), `0x1A` (card move: param2 from, param3 to, as `player \| zone<<1`; zones 0–4 monsters, 5–9 Spell/Trap, 0xB hand, 0xC extra, 0xD deck, 0xE graveyard, 0xF banished), `0x36`, `0x37`, 1000–1019 (data loads: per-card records `0x3E9`–`0x3EB`, `0x3F5`; arrays `0x3EF`–`0x3F3`; card map `0x3F9`/`0x3FA`) |
| DuelScene `0x514290`              | 1, 2 (duel end: param1 winner, param2 reason), 4, 5, 6, 7, `0x15`, `0x1E`, `0x22`–`0x25`, `0x36`–`0x3A`, `0x3C`, `0x3D`, `0x3F`, 1505                                                                                                                                                                                   |
| DuelCardManager `0x4FF000`        | `0x12`–`0x14`, `0x16`–`0x1D`, `0x1F`, `0x20`, `0x25`–`0x29` (cards are looked up in the engine's card map: `0x523DF0(player, zone, index)`)                                                                                                                                                                             |
| DuelCockpit `0x502EB0`            | 5, `0xF` (LP), `0x10` (LP change)                                                                                                                                                                                                                                                                                       |
| DuelFieldManager `0x50A690`       | 4 (highlight the cards with commands), `0x26`                                                                                                                                                                                                                                                                           |
| DuelSummon `0x5165A0`             | `0x2A`–`0x32`                                                                                                                                                                                                                                                                                                           |
| DuelChain `0x501560`              | `0x33`, `0x34`                                                                                                                                                                                                                                                                                                          |
| DuelCutin `0x505A70`              | 6, `0x43`, `0x44`, `0x46`–`0x48`, `0x4C`, `0x4D`                                                                                                                                                                                                                                                                        |
| DuelBattle `0x4F8A90`             | 4, `0xA`–`0xE`, `0x10`                                                                                                                                                                                                                                                                                                  |

**Command names.** The ids follow Konami's view-type vocabulary as Master Duel's `DuelViewType` lists it (YgoMaster
`Enums/Duel.cs`) up to `CardUpdate` (`0x27`), and one below it after that (Master Duel added `ManaSet`): 1 DuelStart, 2
DuelEnd, 4 WaitInput, 5 PhaseChange, 6 TurnChange, `0xA`–`0xE` BattleInit / BattleSelect / BattleAttack / BattleRun /
BattleEnd, `0xF` LifeSet, `0x10` LifeDamage, `0x12`–`0x14` HandShuffle / HandShow / HandOpen, `0x15` DeckShuffle, `0x16`
DeckReset, `0x17` DeckFlipTop, `0x18` GraveTop, `0x19` CardLockon, `0x1A` CardMove, `0x1B` CardSwap, `0x1C`
CardFlipTurn, `0x1D` CardCheat, `0x1E` CardSet, `0x1F` CardVanish, `0x20` CardBreak, `0x25` CardEquip, `0x26`
CardIncTurn, `0x27` CardUpdate (counters in Online 3, see Stat changes), `0x28` MonstDeathTurn, `0x29` MonstShuffle,
`0x2A`–`0x32` Tribute / Material / Tuning Set, Reset, Run, `0x33` ChainSet, `0x34` ChainRun, `0x35` RunSurrender, `0x36`
RunDialog, `0x37` RunList, `0x38` RunSummon, `0x39` RunSpSummon, `0x3A` RunFusion, `0x3B` RunDetail, `0x3C` RunCoin,
`0x3D` RunDice, `0x43`… Cutin*. Every component above handles exactly its family, and the known ones match (5, 6,
`0xA`–`0xE`, `0xF`, `0x10`, `0x1A`, `0x36`, `0x37`): the names are **shape**, each command's parameters still need their
handler read.

`0x1C` CardFlipTurn {place as moves give it, with face up bit 14 and defense bit 15} (DuelCardManager `0x4FF000`): the
field card at that place takes the new face and position (`+0x82`), is redrawn, and sound `0x3B` plays. The server
sends it for MSG_POS_CHANGE (a Set card activated, an attacked monster flipped, Giant Orc's own change) and
MSG_FLIPSUMMONING, without the defense bit for a Spell/Trap; the opponent's card turning face-up gets its row (1000)
and field record first. Live: Giant Orc turned sideways at the end of its Battle Phase (`0x1C 0xC000`); a Set Trap
Hole turned face-up as it was activated.

**Duel clocks** (DuelCockpit's update `0x502660`). Each side shows a turn clock (Flash `setTime`, with `setTimeGauge`
against its max) and a total clock (`setTotalTime`, mm:ss): slot (`PlayerNum` ≠ side) for the turn clock and that + 2
for the total, so with `PlayerNum` 0 slots 0 and 2 are the client's, 1 and 3 the opponent's. A running slot counts
down by wall-clock seconds (`_time32`) to 0; nothing in the client acts when it gets there, so the server decides.
Running out of time ends a duel with reason 11, text 2010 "@@s has ran out of time.". The server runs a chess
clock in duels between players: each player has a turn clock, full again at the start of each of their turns, and a
total clock for the duel; both run while the engine waits on that player, and the first to empty loses (command 2
{result, 11}). Each client gets `0x14` with its own clocks as 1 whenever the waited-on player changes. A hosted duel
takes its `time_turn` and `time_total` (0: that clock off); Auto Duel takes Create Duel's defaults, 180 s and 600 s
(**guess**); NPC duels have none. The client handles `0x14` as it pops it (one server message per tick, `0x4F1730`;
only engine commands wait in the animation queue, `0x523541`), DuelCockpit counts whatever the scene is doing, and no
duel request says when the client has played a batch (requests 2-7, 12-15 answer prompts; 1022's acknowledgement
carries only the player). So a player's clocks start after an allowance for the batch their client was just sent: a
duration per animated command (a move 0.4 s, a phase banner 1.2 s, a turn's banner 1.5 s, a summon's effect 1 s, an
activation 0.8 s, a chain link 0.5 s, a shatter 0.6 s, an LP change 0.7 s, a battle 1.5 s, a reveal's popup 1 s), at
most 15 s (**guess**; Konami's logs hold no timers message). Until then both clients get the clocks stopped (`start`
0), and the clock pass sends them running once the allowance has passed. The server still calls a time-out 5 s after 0. Live (Auto Duel, Match Mode): Yugi's clock ran (TIME 169, 09:49)
while Kaiba's stood at 180 and 10:00, each mirrored on the other client with "Now Thinking"; Kaiba, left idle, lost
duel 1 by time out (his total down to 07:00), the match went on, and in duel 2 Yugi's screen read "YOU LOSE — Yugi
has ran out of time." with its turn clock at 3 (hence the grace).

**The client says when it has played a batch** (issue #11, through the connection DLL). The server ends the waited-on
player's batch with engine command `0x5F1 {seq}`, after its animations and before the prompt that ends it (the input
wait 4, a dialog `0x36`/`0x37` and their data, which hold the queue until the player answers; `clock::mark_settle`).
The scene pops a command only once the commands before it have played (`0x522A50`); the DLL's duel observer takes
`0x5F1` there (the stock dispatcher has no case for it), waits until cards, battles and chains have settled (the
snapshot's own test), and sends `duelRequestName 0x7A13 {seq}` as the client's duel requests are sent (a `YO3Request`
`0x200C` through the duel connection `0x82D2C4`; not in a replay, engine `+0` 1). The player's clocks count from that
answer, earlier or later than the allowance, which stays the fallback: an answer more than 30 s after its batch, or
none, leaves the allowance's start (**guess**: the window), and both clients get `0x14` again when the answer moves the
start. Live (2026-11-14, Auto Duel Alpha against Bravo on the private software-rendered clients): a summon's batch had
an allowance of 1.45 s; the client answered 4.4 s after it, and both clients' clocks were re-counted from there (the
first player's 143 again); the opening took 16 s to play against an allowance of 8.7 s, hence the 30 s window.

**Battles** (DuelBattle `0x4F8A90`; its caller `0x4F840E` passes `param1` in ECX, `param3` in EDX and `param2` on the
stack; DuelScene's busy check `0x515300` holds the queue while its state `+0x74` runs):

| Command            | Params                                                                                                                                                                             | What the client does                                                                                                                                                                                     | Status                                                                               |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `0xA` BattleInit   | none                                                                                                                                                                               | battle mode `+0x78` = 1                                                                                                                                                                                  | shape                                                                                |
| `0xC` BattleAttack | `param1` attacker's player \| its zone << 8, `param2` the target's player, `param3` the target's zone, or from 5 a direct attack (we send 16; `0xFFFFFFFF` plays nothing)          | at a target: the attack, with the attacker's duelist cut-in (the NPC's line, or the player's `ace_monster`), and the hit on the target                                                                   | confirmed (live: "I shall obliterate you!", the hit on Giant Orc)                    |
| `0xD` BattleRun    | **per player**: `param1` the damage figure on the client's card, `param2` on the opponent's; `param3` flags, low byte the client's card, high byte the opponent's: bit 1 destroyed | states 5–6 (`0x4F8430`) play a damage or destroy effect at each side's card (`0x4F8920`, size by damage / 500); a destroyed card's effect ends hiding it (`0x4FB220` mode 0), before the server moves it | confirmed (live: values by attacker/target hid the wrong card when the CPU attacked) |
| `0xE` BattleEnd    | none                                                                                                                                                                               | battle mode off; takes down the swords the battle input wait drew (`0x4F8E40`, as `0xB` and `0xC` do)                                                                                                    | confirmed (code)                                                                     |

The server sends Init and Attack for MSG_ATTACK, Run for MSG_BATTLE (figures by ATK against ATK or DEF, destroy flags
clear; a direct attack, whose target the engine reports as an empty place (ocgcore-wasm's shape, which `yo3-engine` keeps), gets no figures since there's no
card to show them at), End at DAMAGE_STEP_END, ATTACK_DISABLED, the next attack or a new phase, and always when a
Battle Phase ends: in battle mode the input wait (command 4 mode 3, `0x4F8A90` case 4) draws swords over the player's
possible attackers (effect `0x3C`, kept at `+0x64`) that only `0xB`, `0xC` and `0xE` take down, so a Battle Phase with
no attack left them up (live, the VM round). After `0xC` at a target the client plays the attacker's cut-in, then the
strike effect `0xCB` toward the target (states 3, 4, 1, 2 of `0x4F8430`). Any target zone from 5 is a direct attack
there (state 3: the direct cut-in, or the finishing one when the defender's LP don't cover the ATK, then the strike),
while `0xFFFFFFFF` plays nothing at all (case `0xC`), which a direct attack showed (live, the VM round): the server
sends the defending player and zone 16 (the field's centre, as ChainSet's; any value from 5 reads the same). LP still change
through `0x10`.

**Destruction by effects** (DuelCardManager `0x4FF000`, its switch `0x50007C` by command id; both read `param1` from the
command record's params like the other components):

| Command           | Params                                                                                                       | What the client does                                                                                                                                                                                                                                                                                                                                                                                 | Status                                                                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `0x1F` CardVanish | `param1` the player, `param2` byte 0 the zone (at most `0xB`: the field or the hand), byte 1 the index in it | the card (`0x4F8240`) is marked off the field (`+0xD7`, cleared when it lands again, `0x4FBF30`) and runs effect mode 3 (`0x4FB7F0` → state `0x4FC520`): its zone's label goes (field Flash `setCurrent`, `setVisible` 0), effect `0x33` plays at its place (in the hand `0x4E5540`), and the card is hidden (`0x4FB220`). The queue waits for the card manager (busy check `0x5153CA` → `0x5004E0`) | confirmed (live: Masked Dragon, and later Sangan, shattered into shards when Trap Hole destroyed them, then went to the graveyard) |
| `0x20` CardBreak  | `param1` the player, `param2` the field zone (at most 10), `param3` the card's uid                           | the card (by uid, `0x500470`) is marked off the field and its zone's label goes; no effect                                                                                                                                                                                                                                                                                                           | confirmed (code; live: nothing visible); not sent                                                                                  |

The card's next move (`0x1A`) flies it on as usual (`0x5000E0` always runs effect mode 0). The server sends CardVanish
before the move of a card the engine actually destroyed (MSG_MOVE's `reason` has DESTROY, including battle).
MSG_BATTLE's `destroyed` flag is provisional: limited protection such as Fortress Warrior's and Shield Wing's is
applied afterwards. Passing that flag to BattleRun hid a surviving card while leaving its slot occupied and usable
as an attack target or Synchro Material. BattleRun now leaves both destroy bits clear; CardVanish runs only when
an actual move confirms destruction. The duel check models BattleRun's hide bits too. Real-core regressions cover
both client perspectives, face-up/Set Fortress Warrior, and Shield Wing's exhausted protection.
Status: **confirmed**, native hide `0x4F8920` → `0x4FB220`, move handling `0x4FF000`, `tests/yusei_contract.rs`.
The engine's messages keep ocgcore-wasm 0.1.6's shape (`crates/yo3-engine/src/decode.rs`), which drops MSG_MOVE's `reason`, so the engine reads it back with
a query (flag `REASON`) at the card's new place after each processing step.

**Revealed cards** (MSG_CONFIRM_CARDS, 31 `{player: who it is shown to, cards}`; EDOPro's ConfirmCards: Witch of the Black
Forest's "show it to your opponent", a hand reveal). `0x13` HandShow {player, hand index} (DuelCardManager `0x4FF15C`, a
card with the face-up bit `0x4000` clear: skipped when set; it loads the face from the card table entry's row
(`0x523D40` → `0x4FB090`) and runs effect mode `0xB`): the hand card flips to its face for a moment, back down again by
itself, and the card info panel takes it. Live (Auto Duel, Witch of the Black Forest's search, recorded on the
opponent's display): the new card flipped to its face in the opponent's hand row for about a second and the info panel
named it; the owner's info panel took the card too. The opponent's hand cards have row 0 in the table against a player, so the
entry (1000, with the row) goes first. The client sees a card shown to it and its own cards shown to the opponent,
not the opponent's cards shown to the opponent. A hand card shows at once; a card from the Deck (a search) shows when
it reaches the hand: the server keeps its Konami id until the move to the hand, then sends the show at its new index.
**Hand shuffles** (MSG_SHUFFLE_HAND, 33 `{player, cards}`: EDOPro shuffles a hand after a card in it was shown, as
after a search's confirm). `cards` is the hand's new order as bare passcodes; the engine adds `cids` beside them. The
server reorders its mirror of the hand to match, re-sends each hand card's table entry (1000) at its new index, then
`0x12` HandShuffle {player} (DuelCardManager `0x4FF000` case `0x12`): the client rebuilds that player's hand list index
by index from the table (`0x523DF0(player, 0xB, i)`, then the object by uid `0x500470`), renumbers each card and redraws
it. An index with no table entry stops it with an error state (`0x1200`), so every index below the hand's count gets one.
A shuffle that leaves the order unchanged (identical cards) sends nothing. **Confirmed** (live, `--duel-deck
248,2627,807`: after Sangan's search the server sent the seven entries and `0x12 {0}`, the hand showed the new order, and
the card clicked at index 5 was the one the engine summoned). Missed, it put the client's hand out of step with the
engine's: see the hand-index rule under the moves above, and the crash it caused in
[client-internals](client-internals.md#crash-history-and-the-rule-each-one-taught).

**Card relations** (not sent yet). The client keeps a list of relations between field places (engine store `+0x90`,
list at `+0xB8`/`+0xCC`) and draws them while the pointer is on a face-up field card (mouse move `0x50B700` →
`0x50AC20` → `0x50B1C0`; command `0x26` redraws them too): every relation from the hovered card's place draws a
marker on the related slot. Commands: 1017 (`0x3F9`) {param1 the source place `player | zone << 8` (client-relative,
zone below `0xB`), param2 the target place (same form), param3 the marker type 1–8, param4 1 add (`0x5224E0`) / 0
remove the entry matching all three (`0x522530`)}. (1018, `0x3FA`, is another table, a map at `+0xD8`: {param1 →
param2} set (`0x5226B0`), or param1's entry deleted when param2 is `0xFFFF` (`0x522750`); what reads it is **open**.)
Type n draws field effect `0x33 + n`, except 6 →
`0x35` and 7 → `0x37` (bit `0x100` of the type adds the type-8 marker); the effects use `cicon/duel_eff_cicon_01`–`08`.
**Confirmed** live (a probe build sent eight relations from the client's monster): type 1 a cyan orb, 2 and 6 a yellow
∞, 3 a green compass star, 4 and 7 a red ✕ circle, 5 orange turning arrows, 8 a pink target reticle, each on its
slot (the opponent's row mirrored: their zone 0 on the client's right). What each type meant to the original server
is **open** (no names in the client).

The server sends the engine's links: an equip (MSG_EQUIP 93 {card, target}; a new one replaces that equip's earlier
link) with type 2, a lasting target (MSG_CARD_TARGET 96, ended by MSG_CANCEL_TARGET 97) with type 8 (the pairing is a
**guess**). Each link goes both ways (either card as the source), follows the cards by uid, so a card that changes place
has its relations removed at the old place and added at the new one, and goes when either card leaves the field. After
each batch of engine events the server diffs what the client holds against what it should: removals, additions, then
`0x26`. **Confirmed** live (`--duel-deck 807,581`, Axe of Despair on Vorse Raider): hovering the monster drew the yellow
∞ on the Axe, and hovering the Axe drew it on the monster (2026-11-13; the hover pick `0x50B510` gives a Spell/Trap slot
as `zone << 8` too, and `0x50B1C0` looks the hovered place up in the relations with `0x522170`).
The duel check holds every relation to two occupied slots (240 self-played duels: 84 equips, 121 lasting targets).

**Card overrides** (not sent yet). 1019 (`0x3FB`) stages {param2 type, param3 value, param4} and 1020 (`0x3FC`)
{param1 a place} stores the staged entry for that place, or clears it when nothing is staged (`0x522850` /
`0x522910`). Command `0x26` then copies each Spell/Trap place's entry onto its card (DuelCardManager `0x4FF000`): type
1 → `+0xB0`, 2 → `+0xB4`, 3 → `+0xAC`, which the card's labels show instead of its own data (`0x4FBAA0`: two label
values, compared with the card's own bits, and `+0xAC` a card id whose `data/card/%4d_1.png` replaces the picture).
**Shape**: overrides of a card's shown data (a changed Attribute or Type, a card shown as another), not turn
counters.

**Moves out of a hand go by index.** `0x5000E0` takes a card out of its hand list by the move's from index
(`0x5005D0(player, index)`), not by uid. When the server's mirror finds the card the engine named at another index than
the engine's sequence, the move names the mirror's index (where the client has it). A card that lands in a hand goes to
the end of the list (`0x52A400`, a push_back), whatever index the move names, and the hand is renumbered by position
(`0x500640`); a field zone (0–10) the move leaves loses whatever card its slot held, and the one it lands on holds the
card (the slot array at `+0x4C`). The duel check (below) follows exactly these rules.

**Field cards that trade places.** `0x1B` CardSwap {_, place A, place B} as moves give them, both field places
(DuelCardManager `0x4FF000` case `0x1B`): the cards in the two slots (`0x4F8240`) trade places, each taking the word of
the place it lands on as its place and position (`+0x82`; `0x5000E0` with param_6 1, which leaves hands and piles
alone), and the slot array is swapped. So each word carries the face and position of the card arriving there. The
server uses it for MSG_SWAP (55 `{card1, card2}`: Creature Swap, two monsters change sides, each keeping its position)
and MSG_SHUFFLE_SET_CARD (36: Magical Hats shuffles Set cards among their zones), then sends each card's table entry at
its new place and, for a face-up monster, its zone's record. A card whose new zone is empty moves there with `0x1A`
instead. **Confirmed** for MSG_SWAP (live, `--duel-deck 2627,1228`: Creature Swap between a Gene-Warped Warwolf and
Kaiba's Ancient Lamp; each stood on the other side with its own label, the duel went on, and the server's duel check
found no difference). MSG_SHUFFLE_SET_CARD: the core writes the shuffled cards' old places, then as many Xyz material
places (zero when none), and the engine's messages (ocgcore-wasm's shape) read them as `{from, to}` pairs, which pairs old places with each other; no
list gives a card's new place. The engine adds `places` (each shuffled zone and the card it holds after the step, read
back from the duel) and the server matches its cards to them by id (identical cards are interchangeable). **Shape**: the
self-played duels (below) agree with the engine after every Magical Hats; not seen on the client.

**A Graveyard and Deck swap** (MSG_SWAP_GRAVE_DECK, 35 `{player, deck_size, returned_to_extra}`: Exchange of the Spirit)
says only the Deck's new size. The engine adds the player's `grave` and `extra` as they are after the step; the server
moves each old Graveyard card out with `0x1A` (to the Extra Deck while the engine's holds more of its id than the
mirror's, else to the Deck), then each new Graveyard card in from the Deck in the engine's order. **Shape** (self-played
duels only).

**Xyz materials.** The engine (in ocgcore-wasm's shape) gives a material's place as its Xyz Monster's location with an `overlay_sequence` (one
attached in the Extra Deck reads as the Extra Deck). The client has no place under a monster, so a field card that
becomes a material gets CardVanish (`0x1F`: off the field and hidden; its slot keeps the object until another card
lands there) and the server keeps it among the materials; one that leaves for a pile moves there with `0x1A` from the
field slot it left when that is still empty (the client clears the move's slot and so lets go of the hidden object),
else from any empty field slot. **Shape**: the shatter plays for a material, and the client's card list has none of the
Xyz Monsters tried (Gem-Knight Pearl, Number 39: Utopia), so this was not seen live. A material from a hand or pile is
not followed (logged).

**Counters and stats across control changes.** Counters follow the card object (`0x4FB6C0`, map `+0x130`);
a face-bit change clears them (`0x4FBE50` → `0x4FB770`). The translator no longer removes counters for a control
change: **confirmed** with Breaker and Snatch Steal on both running clients. Field-to-field moves and
swaps also carry cached stat records with the card, avoiding a reset to printed stats when an equal-stat swap
produces no changed engine field report: **shape**, covered by an engine-discovered contract regression.

**The duel check** (`crates/yo3-server/src/duelcheck.rs`, the server's `--duel-check`): a model of the client's card
manager built only from the rules above (the hand lists, field slots, pile counts, card table and each card's place
word), fed every command the server sends and compared with where the engine has every card (the engine's read-only
`state` op) after each engine step. It catches a translator that drifts from the client or the engine at the first step
where they differ. The tests self-play a few hundred seeded duels through it (`tests/duel_check.rs`); the protocol
monkey's duels run the server with it. What it found is in the [roadmap](roadmap.md#-duel-check-2026-11-09-the-clients-view-checked-against-the-engine).

The independent `state` query also supplies LP, current/base ATK/DEF, Level and counters. Turn/phase expectations
come from engine messages 40/41, independently of outgoing commands. Unknown engine events or native commands fail
the check. `duelcheck/coverage.rs` explicitly distinguishes state, interaction, presentation and known unsupported
mechanics; `[YO3 GAP]` records the last group. Agreement on state does **not** validate every legal menu or animation.
Mutation fixtures remove/change LP, stats, counters, turn/phase and visibility updates and require a difference.

**Native diagnostic checkpoints, schema 1 — confirmed with our required mod.** With `--duel-check`, each nonempty
checked batch ends in `0x200D`, `duelRequestName=17`, `id=0x5F0`, parameters `{1, sequence, inventory_hash, stats_hash}`.
This is our extension; stock DuelScene's dispatcher has no such command. The mod intercepts the verified native
dispatch call at `0x514212` and polls after the normal scene update callback `0x515A30`. It waits for card, battle
and chain activity to settle, then reads memory without calling mutating game helpers. A missing, superseded,
unreadable or timed-out snapshot is incomplete validation, never success.

Both hashes are FNV-1a over little-endian u32 words. Inventory starts with schema 1, then for each logical player:
cached LP; 11 slots (UID, and for occupied slots normalized place bits `0xC03F` and visible flag); hand count and
ordered UIDs; Extra/Deck/Graveyard/banished counts. Stats starts with schema 1, then each occupied slot in the same
order: face-up monsters' Level/ATK/DEF/base ATK/base DEF, followed by every card's nonzero counter count and sorted
`{native kind, amount}` pairs. Native offsets and busy gates are in [client internals](client-internals.md#native-duel-observer).
This checks cached client state and visibility, not rendered pixels, art identity, every effect or menu legality.

Server expectations (`[YO3 EXPECT]`) and actual snapshots (`[YO3 CHECK]`) use existing session logs. The
`tools/monkey/duel-report.py` report requires every expected acknowledgement, including identical checkpoints on two
seats, rejects mismatches/crashes, and rejects a fully validated result when `[YO3 GAP]` occurs. Without a compatible
observer it reports missing acknowledgements. No credential or account identifier is sent in the checkpoint.

`0x14` HandOpen {player, open flag, uid or 0 for the whole hand} (`0x4FF1E7`: effect mode `0xD` closes, `0xC` opens, the
card's face bit set to the flag) is the way to hold a hand open; not used. Cards revealed elsewhere (a Set card, the
top of the Deck) are not shown: **open**. CardLockon `0x19` and RunDetail `0x3B` were not needed (`0x3B` has no case in
DuelScene `0x514290`).

**The command queue and effects** (DuelScene busy check `0x515300`). A command waits for its component before the next
one starts: card commands `0x12`–`0x1C`, `0x1F`, `0x26`, `0x28` for the card manager (`0x5004E0`), `0x33`/`0x34` for the
chain, `0x37` for the list dialog. The summon effect (`0x38`/`0x39`, DuelScene `+0xD4`, an effect handle) is waited for
only by `0x22`–`0x25`, `0x38`, `0x39` (`0x5153D9`, `0x5153F9`, `0x515445`: when it has finished `+0xD4` is cleared, and
`0x22`/`0x25` first call `0x4FB220` mode 0 on the card manager's `+0x8C` card), so a `0x1C` after a summon can run while
the summon's sparkles still play. Live, with Stumbling (a continuous Spell that changes every summoned monster to
Defense Position, so the summon is followed by ChainSet, ChainRun and `0x1C` in one burst), the summoner's monster and the
other player's both turned in three tries (the summoner's own Stumbling and the opponent's, with and without a
question in between); the failure seen on the Windows VM was not reproduced there. It was on the VM round (Yugi
headless against the VM's Stumbling: both of Yugi's summoned monsters stayed upright on his own screen, and later turns
of them didn't show either; `0x4FB7F0` keeps a pending effect mode per card at `+0xDC`). So within one batch a summon
move onto a monster zone, followed by its summon effect and then a `0x1C` of that zone, gets the turn's position on
the move itself (`settle_positions`, applied to every batch): the card lands in its final position and nothing turns
while the effect plays. Later in the same round the VM player's own monsters turned by their own Stumbling stayed
upright on the VM as well, a question splitting the batch or not, while the other client turned them: a player's own
summoned monster doesn't follow `0x1C` on its owner's screen. So a monster's turn is now also a CardMove onto its own
slot, from the old position to the new one with its row, before the `0x1C` (a moved card is drawn in its position).
**shape**: the live check is pending.

**Move kinds.** A card move's `param1` high half is the third argument of the client's move (`0x5000E0`), as Konami's
server sent it in 17,207 moves of 227 logs: 0 a plain move (draws, discards, cards sent or banished), 2 a Normal
Summon, 3 a Special Summon, 4 a Spell/Trap activated from the hand, 5 a Set, 6 destroyed by an effect, 8 by battle, 9
tributed (1 and 7 are rare, meaning **open**). The client plays no move sound for 6, 8 and 9 (the destruction has its
own) nor for 3 from anywhere but the hand. The server derives the kind from MSG_MOVE's reason and the places; we sent
the row there before the logs were found, which played the move sound under every destruction and silenced cards
whose row is 3, 6, 8 or 9.

**Summons** (DuelScene): `0x38` RunSummon {the card's place as moves give it, with its face and position bits
(`0x514E5A`: the effect plays at that zone; the player alone played every summon's effect at the leftmost zone, live on
the VM round; Konami's logs send `0x4000 | place`, a Special Summon's `0x4000`, `0xC000` or `0x8000`), the summoned
card's uid, then 0 (Konami's sent 1 in 95 of 1,160 Normal Summons and 1 in 19 of 1,497 Special Summons, not with
tributes; meaning **open**)} plays the summon effect at the card (`0x21`,
or `0x23` for the player's `ace_monster`); `0x39` RunSpSummon {player, uid} the Special Summon effect (`0x19`). The
server sends them for MSG_SUMMONING / MSG_SPSUMMONING, which come once the card is in its zone. Live: sparkles around
a summoned Giant Orc. Confirmed.

**Player actions** (confirmed live with a test build that gave every hand card all command bits, then sent phase 2 and
command 4):

- Phases (command 5 param2): 0 Draw, 2 Main 1 (live: M1 lit); by the panel's order 1 Standby, 3 Battle, 4 Main 2, 5 End
  (**shape**).
- `0x12` command map: `comIDsize` pairs of u32 BE (card place `player | zone<<8 | index<<16`, command bits), then
  `helpIDsize` u32 pairs (none so far) and `otherFlagSize` byte pairs {key, value} (the phase buttons). DuelFieldManager
  (command 4) highlights the cards whose bits, less `0x2000`, are set.
- Command 4 {mode, player}: wait for that player's input (the field highlights the player's playable cards; clicking one
  opens its menu, a left click runs its first entry). Modes 2 and 3 turn the phase panel on (`0x504890`); each button
  (0 DP .. 5 EP) is then enabled by an other-flags pair {button, 1} in the command map (`0x501C00`, map `0x520450`).
  A pressed button sends `duelRequestName 3 {phase: the button}` (`0x521180`); live: EP → `{phase 5}`.
- The menu (`0x4F9320`, `duel_cardcmd.swf`) lists a card's bits in the order 0, 2, 3, 4, 5, 6, 7, 9, 8, 12, 1, 10, 11,
  13: bit 0 Attack, 1 See Details, 2 Special Summon, 3 Activate Effect, 4 Summon, 5 Flip Summon, 6 and 7 Set, 8 Change
  to Attack Position, 9 Change to Defence Position, 10 Surrender, 11 Confirm, 12 Draw, 13 Shuffle.
- Choosing an entry sends `duelRequestName 2 {CommandID: the bit, player, position: the zone, index}`; live: Summon on
  hand card 1 → `{CommandID 4, player 0, position 11, index 1}`.

Carrying out a command needs the rules: which cards may do what, and what follows. The server runs EDOPro's core for
that (`crates/yo3-engine`, the `yo3-engine` program; see roadmap.md, M5); `translate.rs` maps its draws, turns and phases to the commands above (ocgcore phases
1, 2, 4, 8–`0x80`, `0x100`, `0x200` → 0–5), its idle-command prompt to a command map (summons bit 4, special
summons 2, position changes 5, 8 or 9 by the card's position as last shown (Flip Summon face down, to Attack
Position in face-up defense, to Defence Position in attack: offering both showed "to Attack Position" on an attack
position monster, live), monster sets 6, spell sets 7, activates 3; the Set bits are confirmed: the entry index is the bit number, `duel_cardcmd.swf` draws item 6 as a horizontal face-down card (a monster Set in defence) and item 7 as a vertical one (a Spell/Trap Set), both labelled text 19 "Set", and a click is accepted only if `1 << CommandID` is in the card's mask, `0x520F00`)
plus command 4, and the client's card command back to the prompt's entry. Live: after the CPU's turn the client's Main
Phase 1 highlights only the legal cards, and a monster's menu reads Summon / Set.

Card moves (`MSG_MOVE`) become a card-table entry plus `0x1A`, with move places `player | zone<<1 | index<<6`, face up
bit 14 and defence bit 15 on field slots (`0x5000E0`), and face up into the Graveyard and, when face up, the banished
pile. A card landing on a pile is removed and only adds to the pile's count (landing state `0x4FBF30`, case
`0x4FC0AD`); the pile then redraws (`0x4FE4D0`, also on a card leaving, `0x5000E0`): it shows its top card's face
(top = index count − 1 for the extra deck and Graveyard, 0 for the deck) when that card's table entry has the flag
that 1000's uid top byte sets (`0x521DF0` keeps `top byte != 0`, read by `0x523DA0`), else a back. The pointer over a
pile previews that card on the same condition (`0x50B9C0`). So each Graveyard showed a back on both clients (live,
even moved face up). The server sets the flag (`uid | 1 << 24`) on Graveyard and banished cards, and when a card
leaves a pile from under its top, sends the renumbered entries before the move, so the redraw finds the new top.
GraveTop `0x18` {player, zone `0xE`, uid} (`0x4FF501`) instead makes a separate face-up card object at the top of a
Graveyard; tried first, it went stale (live: a back once a card left from under the top), so it is not sent. Live
(the flag): MST on a Set Spellbinding Circle, Black Rose Dragon's wipe and Monster Reborn taking Junk Synchron from
under Spellbinding Circle left each Graveyard showing its top card with its count on both clients, and hovering one
previewed it; field slots are zones 0–4 (monsters), 5–9 (Spell/Trap) and 10
(Field Spell) at index 0, then hand `0xB`, extra `0xC`, deck `0xD`, graveyard `0xE`, banished `0xF`. Cards behind a
removed list card get their table entry again. Both extra decks are in the card table too (zone `0xC`, uids after
both main decks, the opponent's with row 0 against a player or an NPC); a card leaving the deck or the extra deck is found by its
id, so an extra deck summon moves the right card from zone `0xC` and the extra deck pile counts down. Commands on
cards in the extra deck, Graveyard or banished pile (an idle prompt's Special Summon from the extra deck, an effect
activated from the Graveyard) go in the command map at that pile's zone, index 0: the pointer over a pile gives the
place (pile zone, index 0) (`0x50B510`), and the menu and the highlight look bits up by that exact key (`0x5204E0`),
so a command at the index of a card other than the pile's first was unreachable. The client sends the command at index
0; the server finds the listed cards of that pile with that command (by `cid`: the engine's extra deck order isn't the
card table's): one answers at once, several open the card list (one pick or none, kind 1) with the original's "Select
the monster(s) to Special Summon from your Extra Deck." (26; 25 elsewhere, "Select effect to activate." 276 for
effects), whose pick answers the engine and whose right-click goes back to the prompt (**shape**: the original's
flow is unknown, the text is its own). EDOPro asks for Synchro and Fusion
materials one at a time (MSG_SELECT_UNSELECT_CARD {can_finish, can_cancel, select_cards, unselect_cards}: the Tuner
first); each round is the card list of `select_cards` with one pick (kind `0x27`, or 1 when the engine lets the player
finish or cancel, where no pick answers index null), "Select monsters to use as Fusion Material." (92) for
HINTMSG_FMATERIAL, and for HINTMSG_SMATERIAL (512), which the client's table has no text for, the server's own
"Select the monster(s) to use as Synchro Material." (see "Server texts"; before the DLL could show it, 58 "Select the
card to send to the Graveyard."). Live 2026-11-14: Gaia Knight from Gilasaurus and Junk Synchron. Live
(`--duel-deck 3339,1449 --duel-extra 3745`): with Junk Synchron and Giant Orc on the field, clicking the extra deck
listed Junk Synchron, then Giant Orc; both went to the Graveyard, the position dialog asked for Ancient Fairy Dragon,
and it stood in attack position on both clients with the extra deck pile down to 1. **Places.** Where a summoned, Set or activated card goes (MSG_SELECT_PLACE, 18) is the server's choice, as it was
Konami's: every one of the 129 placements in Konami's logs fills the free zones centre out (2, 3, 1, 4, 0; Spell/Trap
zones the same), so the server takes the first free zone of the prompt's `field_mask` in that order: own monster zones,
own Spell/Trap zones, the opponent's (Give and Take and Black Garden place there), then the Field zone; as many as its
`count`. A zone an effect takes (MSG_SELECT_DISFIELD, 24: Ground Collapse, Seismic Shockwave, Ojama King and Knight,
Mystical Cards of Light, Numbing Grub, Senet Switch's move) is the player's. The client has no zone dialog, but every
zone, empty or not, is a click target (pick areas `0x509CD0`, hover place `0x50B510`) whose click sends the card command
of the command map entry keyed as a card on that zone would be (monster zones zone 0, Spell/Trap zones zone 5, the
zone's index in bits 16–23): duelRequestName 2 `{CommandID, player, position: the zone, index 0}` (`0x4F9510`). So the
server first shows an OK dialog (kind 0) with the client's own prompt, 155 "Select Monster Card Zones to render
unusable.", 324 "Select a Spell & Trap Card Zone to make unusable." or, after HINTMSG_TOZONE (571, Card.SelectAdjacent),
472 "Move this card to an adjacent unoccupied Monster Card Zone.", then the free zones as a command map (Summon for a
monster zone, Set for a Spell/Trap zone; a right-click menu would name them) and the input wait, a click per zone until
`count`. A taken zone or a repeat changes nothing. Live (2026-11-13): Ground Collapse asked with 155, the clicks on the
client's monster zone 1 and the CPU's zone 3 came back as `{4, 0, 1, 0}` and `{4, 1, 3, 0}`, and both zones showed
unusable. Positions: see Choices.

**Unusable zones.** MSG_FIELD_DISABLED (56) `{field_mask}`: per player (× 16) monster zones bits 0–4 and Spell/Trap
zones 8–12, set when disabled. The client's 1006 `{mask}` (pump `0x522A50` → engine `+0x124`) has its own side in the
low half and the opponent's in the high half, monster zones bits 0–4 and Spell/Trap zones 5–9, **set when usable**;
DuelFieldManager reads it on the next `0x26` (`0x50AA90`) and DuelInvalidZone (`FieldManager+0x34`, RTTI vtable
`0x7B30D8`) draws a zone that became unusable (effect `0x2D`, a red cross) or usable again (`0x2E`). The server sends 1006
then `0x26`; the opening says every zone is usable (`0x3FF03FF`), as Konami's openings did (227 times in its logs,
where the other 12 values clear a Trap Monster's Spell/Trap zone).

**Tokens.** EDOPro's tokens have passcodes with no Konami id (YGOProDeck lists none for Scapegoat's Sheep Tokens), so
the engine adds the card's `name` from `cards.cdb` where it finds no id. The client has its own token cards: CARD_Prop
ids 3900–3999 (75 rows, e.g. four "Sheep Token", 3902–3905, one per colour). A token comes from nowhere (MSG_MOVE
from location 0): the server takes the client's token card of its Konami id when it has one (Doomsday Token 3940,
Grinder Golem's 3936, Jam Breeding Machine's Slime Token 3907), else of its name (in turn when there are several), gives it a
uid of its own (from `0x4000`), and sends its card table entry, its field record and a move from its slot onto
itself (`0x1A` finds no object by uid and makes one, `0x5003A0`). A token leaving the field (to location 0, or to a
pile) gets CardVanish and leaves the mirror; one that changes sides (Snatch Steal on a Sheep Token) moves like any card. A card list finds a token at its place (it has no id to find it by).
Live (Scapegoat): four Sheep Tokens in four colours, in defense, on both clients; Witch of the Black Forest's attack
listed them as their own cards and the one destroyed shattered and left no Graveyard card.

**Pile views.** The field highlights a pile of either player (zones `0xC`–`0xF`, index 0) whose command bits there
include Activate Effect or Confirm (`0x808`, DuelFieldManager `0x50A690`), and the card menu's bit 11 reads "Confirm".
The duel prompts keep the original's pile titles: 582/583 "Your/Opponent's Extra Deck", 584/585 Deck, 586/587
Graveyard, 588/589 "Removed Cards", next to 590 "Confirm". So the server puts Confirm on both players' Graveyards and
banished piles and the player's own extra deck, when not empty, in every idle and battle command map. Confirm
(`2 {CommandID 11, player, position: the pile zone, index 0}`) opens the card list of the pile, bottom card first, with
max 0 (view only: `dlg_cardlist.swf` adds no click handler when `selMax` is 0, and with no picks required a right-click
calls `decide`), kind 1, the opponent's cards marked, under the pile's title. Closing it sends `4 {DLGResult 0}`; the
engine is not answered and the pending prompt (command map and command 4) goes again. Kind 4 lists read a
per-card flag (the card table entry's byte from 1000's uid top byte, `0x521DF0`) at the opponent's banished place
of the same index and draw the card back where it is clear (`0x4C7A13`): the original's view of face-down banished
cards, used for the mixed lists below. Live: "Your Extra Deck" listed the three Synchros with ATK/DEF, Kaiba's "Opponent's Graveyard"
listed Yugi's Spellbinding Circle, and a right-click closed each and gave the field back.

A monster's field label and card info read a 9-value record per place (`0x523F00`, key `player | zone<<8`; the info
panel `0x453740`): shown Konami id, a second id, race, attribute, level, ATK, DEF, base ATK, base DEF (ATK below base in
red, above in blue, `0xFFFF` = "?"). Data commands build it: `0x3E9 {_, values 0, 2, 3}`, `0x3EA {_, values 4, 5, 6}`,
`0x3EB {key, values 7, 8, 1}`, which commits (the pop; stores happen before the copy). Level, attribute and race are
CARD_Prop's second word bits 12–15, 8–11 and 19–23. The server sends it when a monster enters a monster zone, with the
CARD_Prop stats. Live: Giant Orc's label reads 2200 and its info "ATK 2200/DEF 0" with its icons and level 4. A
per-place 3-value map (`0x3F5`, `0x523F50`…) holds a typed counter for the same panel.

**Stat changes.** DuelCardManager receives every command (event `0x2001`, entry `0x4FEE70`, whatever the id), and
after each one its common tail (`0x4FFF14`) re-reads both players' monster-zone records (`0x5222D0`, key `zone<<8 |
player`): a face-up monster (`+0x44` bit 0, face-up bit `0x4000`) whose record differs from its copy (`+0xA8`) takes
the new one and has its label redrawn (`+0xD5` → `0x4FBAA0`: label fields 6 ATK, 7 DEF, 4 and 5 from values 7 and 8).
So a changed record shows at the next command, and needs no command of its own. The rules engine reports no stat
changes, so the engine queries both players' monster zones (position, level, ATK, DEF, base ATK, base DEF) after each
processing step and, when they changed, adds `{type: "field", monsters: [{controller, sequence, position, level,
attack, defense, base_attack, base_defense}]}` before the step's prompt. The server re-sends the record of a face-up
monster whose values differ from those it last sent (level from CARD_Prop when the engine has none, as for Xyz
monsters); race and attribute stay CARD_Prop's. Live: Axe of Despair equipped to Giant Orc sent its record again
(`0x3EA {_, 4, 3200, 0}`, `0x3EB {0, 2200, 0, cid}`), and at the next prompt its label and card info read ATK 3200,
in blue.

**Counters.** `0x27` (named CardUpdate above) is a card's counters: `param1` byte 0 the player, byte 1 the zone (the
card at index 0 there, `0x4F8240`, as CardVanish finds it), `param2` a change added to the count of counter kind
`param3` (`0x4FB6C0`/`0x4FB730`: a map at `+0x130`; a count of 0 erases the kind, a negative one is ignored), then
effect mode 5 (`0x4FC7E0`: on the field, the zone's label `setProp(8, kind, icon, count)` → `setCounter`, the icon
from the table at `0x789020` by kind) and sound `0x58` (up) or `0x57` (down) (DuelCardManager `0x4FFD8E`). The card
info lists each kind as UI text 200 + kind with its count (`0x4551DB`): the texts 200–224 and 235–243 are counter names
exactly where the icon table has entries (0 "Spell Counters", 2 Clock, 3 Hyper-Venom, 4 Crystal, 5 Chronicle, 6
Bushido, 7 D, 8 Shine, 9 Gate, 10 Worm, 11 Morph, 12 Flower, 13 Plant, 14 Speed, 15 Psychic, 16 Earthbound Immortal,
17 Junk, 18 Genex, 19 Dragonic, 20 Ocean, 21 Black Feather, 22 Death, 23 Mechanical, 24 Magical Stone, 35 Maiden, 36
Magic Reflector, 37 A-, 38 "Counter", 39 Venom, 40 Fog, 41 Protection, 42 Wedge, 43 Guard; 1 reads "counter"). The
client empties a card's counters only when it lands or turns face down on the field (`0x4FBF30`, `0x4FBE50` →
`0x4FB770`); a label loses its icons when its card moves (`removeAllCounter`). `0x25` (key -1) and `0x28` (key -2,
`param3` low byte) set two special entries (meaning **open**, not sent).

The server translates EDOPro's MSG_ADD_COUNTER / MSG_REMOVE_COUNTER {counter_type, controller, location, sequence,
count} on a field card to `0x27`, the kind by name from EDOPro's counter ids (strings.conf; the id keeps
COUNTER_WITHOUT_PERMIT `0x1000`, e.g. Wedge `0x1002`): Spellstone as Magical Stone and Des as Death (**guess**), ids
the client has no name for as 38 "Counter". It keeps the client's counts per card: a removal never takes more than
the client holds, and since EDOPro drops a card's counters without a message when it leaves the field, the card's
counts go to 0 just before its move. Live (`--duel-deck 1513`, Breaker the Magical Warrior): its summon placed a Spell
Counter, a green icon with 1 on both clients' labels, ATK 1900, and the card info read "Spell Counters 1"; when the two
Breakers destroyed each other, each client got `-1` for both cards before the moves, and no icon stayed on the field.

Battle: the engine's battle prompt becomes a command map with bit 0 (Attack) on each attacker and bit 3 on chainable
cards, the M2 and EP buttons as allowed, and command 4 mode 3. A monster in slot s sits at command-map place (zone 0,
index s), and the client sends Attack as `{CommandID 0, position 0, index s}`. LP changes: command `0x10 {player,
signed change, kind << 8 | shown}` (DuelCockpit `0x502EB0`) for ocgcore's DAMAGE, PAY_LPCOST (negative) and RECOVER,
twice as Konami's server sent it: with `shown` 1 it shows the change's number and plays the kind's sound (4 a gain:
recover sound, 2 a cost: payment sound, else damage), with 0 it applies the change (clamped to 0–99999); the damage
line `0x4D` goes between. Kinds from the original logs: 0 effect damage, 1 battle damage (after BattleRun), 2 a cost, 4
a gain. Before the logs we sent the second only, so changes applied silently. `0xF` for LPUPDATE. Live: a direct attack took the CPU from 8000 to 5700 LP. Attack animations use
DuelBattle commands `0xA`–`0xE`, mapped in the battle-command table above.

**Presentation from Konami's logs.** Commands the original server sent that only show something, now sent the same
way (all from the 227 original duel logs, their handlers read in the client):

| Command | Params | When | Handler |
| ------- | ------ | ---- | ------- |
| 7 FieldChange | the Field Spell's Konami id, or 0 | a card face up in the Field zone; 0 when it leaves | DuelScene → `0x50CD60`: the duel field's background (Dragon Ravine, Geartown, Necrovalley... in the logs) |
| `0x1E` CardSet | player, zone, 1 for a monster | after a Set's move | DuelScene: the Set effect `0x22` |
| `0x22` CardActivate | the card's place as moves give it (face up), Konami id, uid | before an activation's ChainSet | DuelScene: the activation effect by card type (`0x1A`–`0x20`; `0x20` for the player's "trump card"), waited for |
| `0x43` | player, 1, 0 | the turn player's Draw Phase | DuelCutin `0x505A70`: the draw line (param3 non-zero: none) |
| `0x44` | player, `0x8000` \| uid, kind | before a summon's effect | the summon line (the "ace monster" line for the configured card; kind 8 a Synchro's). Kinds in the logs: 0 Normal, 1–2 Tribute, 5 Special, 6 Fusion, 8 Synchro; we send 0 and 5 |
| `0x47` | player, `0x8000` \| uid, step | before `0x22` | steps 1 (a Set card) or 2 (from the hand) only note how the card came, 3 and up speak (the "trump card" line, else by card type); a Spell/Trap gets 2 then 3, a monster's effect 4 |
| `0x48` | player, Konami id, face up, to a Spell/Trap zone | before a card's move from the hand to the field (Normal Summon, activation, Set) | a Set's line (`param3` 0), none otherwise; the id is never read, so a Set the client may not see goes with 0 |
| `0x4C` | player | the turn's end, before the turn change | the turn-end line |
| `0x4D` | player, LP change, kind | between an LP change's two `0x10` | the damage line by the LP left (to 0, 1000+, 2000+); none for a cost or a gain |

The speech lines are each player's Avatar Settings lines (speech00–22), which the client already holds; its duel
option "speech" (`.y3p` `duelOption.speech`) turns them off. The command picks a slot (DuelCutin `0x505A70`): 1 the draw
(20, 21 variants), 2 a summon (22 a Synchro's, 23 the "ace monster"), 6–8 a Spell, Trap or other activation, 9 a
monster's effect, 17 the "trump card", 10–11 a Set, 12 the turn's end, 13–15 damage by the LP left, 16 to 0; `0x46`
`{player, 1, Konami id}` plays slot 19 and silences the next `0x47` (`+0x68`): Konami's logs send it before a Set
Spell/Trap turns face up to activate (227 times) and after the ChainSet of a hand card's effect (about 75), and so does
the server. `0x505EA0` first offers an animated cut-in ("my_effect", "your_effect", "npc_effect"
options): by a frequency table per situation (NPCs `0x788BEC`: the first time always, then every 2nd to 8th; the
draw every 8th) it plays the character's own animation with its built-in line; otherwise the slot's text shows in a
speech bubble with the speaker's name. A slot reading `/preset N` is replaced by a built-in line. **An NPC's slots are
all set to `/preset 0`** by the duel's player setup (the branch around `0x428295`'s caller, slots listed at
`0x792660`), so its bubbles show its built-in lines (Pegasus' draw: text 63501 "My turn now!", 63502 "I think I'll
play..."), whatever the roster item carries: speech fields sent in an NPC's `0x2711` item were ignored (live,
2026-11-13). Our NPCs speak in the chat window instead (`crates/yo3-server/src/banter.rs`, lines in `npc_lines.txt`):
a `0x2455` push to the player's front connection (`{from_id: npc_id, pname, chat_type: 0, chat_str}`), sent as the
client plays the moment it belongs to (see the chat table above). Not sent, with reasons
(Konami's logs and the client's handlers, 2026-11-13): CardBreak `0x20` `{owner, monster zone 0–4, uid}`, 578 times, only
after a battle (its move kind 8; an effect's destruction got CardVanish), only drops the zone's label (`0x4FF000` case
`0x20`): the shatter came from BattleRun's destroy bit, which the server leaves clear because the engine sets it
before a protection (Fortress Warrior, Shield Wing) is known, so the server keeps CardVanish's shatter. BgmUpdate 9
(after every summon cut-in and every finished LP change, 4,400 times) has no handler in any duel component or the pump:
nothing to send. `0x45`, `0x49`–`0x4B` and `0x4E` have no handler either; 1013–1016, 1018, 1021, 1022 (the client's own
acknowledgement request), 1024 and 1025 are bookkeeping, and 1019/1020 a shown-data override for a few cards. CardCheat
`0x1D` is sent with the reveals below. Konami's opening also differs (1013 for every
place, 1006, pile counts 1023 with only the top card in the table); ours, a table entry for every card, works.

**Orders.** MSG_SORT_CHAIN (21, chain links triggered together) and MSG_SORT_CARD (25, cards put back on the Deck: Big
Eye, Gishki Chain, Fruits of Kozaky's Studies) are the player's. The card list orders picks by itself
(`dlg_cardlist.swf`: `onCardClick` pushes the clicked position and draws a badge 1..N, a second click takes it back,
and it decides at `selMax` picks), so the server opens it as kind 6 (all of max) with every card and "Select the cards
you wish to return to the Deck in order of your choice." (245) or, for chain links, "Select effect to activate." (276),
plus "Select %d card(s).". The picks arrive in click order (`7 {index}` each, `0x4C74F0`); card `picks[k]` gets rank
k, the core's answer being one byte per card, its rank, with no count before them (playerop.cpp SortCard,
processor.cpp: rank 0 the Deck's top or chain link 1). Anything but each card once keeps the engine's order. Live: Big
Eye's five cards, the first click came up as the next draw.

**Reveals** (Konami's own sequences, from its logs). A face-down field card shown to the client (MSG_CONFIRM_CARDS 31,
`player` the one shown, with a field place): its table entry with the row, CardCheat `0x1D {place word | 0x4000,
Konami id, uid}` (the card object reloads its face, `0x4FF000`), CardFlipTurn `0x1C` face up, CardLockon `0x19 {player,
zone, 2}` (the card pops up), then the entry without its row, CardCheat and the flip back. The Deck's top cards
(MSG_CONFIRM_DECKTOP 30, both players; or MSG_CONFIRM_CARDS of the Deck's top cards, to one: Ancient Telescope), one at
a time: the card's entry at the Deck's index 0 (the pile's top, `0x4FE4D0`), DeckReset `0x16` once, CardCheat `{0x4000
| 0xD << 1 | player, id, uid}`, DeckFlipTop `0x17 {player, 0xD}` (the pile's top turns over from index 0's row; 611
Deck and 48 Extra Deck uses in Konami's logs), CardLockon, DeckFlipTop again (back), and index 0's last entry again
(while that card is still in the Deck: the client's Deck entries are never renumbered, so another card's could pull it
off its new place) and DeckReset. A face-up Deck (Convulsion of Nature: MSG_REVERSE_DECK 37 turns both, MSG_DECK_TOP 38
`{player, 0, code, position}` names each top card after every change; the cards keep their face-down position, so a
reversed Deck's top is public whatever the position): the top card's entry at index 0 with the public flag (uid top
byte) and its row, then DeckReset, as a Graveyard shows its top card; turned back, the flag goes. Live (2026-11-13):
Ancient Telescope turned two of Yami Bakura's cards over on his pile in turn; Convulsion of Nature showed both Decks'
tops. Extra Deck reveals (CONFIRM_EXTRATOP 42) are not sent: no card of the client's makes them.

**Changed Types and Attributes.** The monster info panel reads race and attribute from the place's record (`0x453740`
→ `0x523F00`: `0x3E9`'s values 2 and 3), not from CARD_Prop. The engine's field report after each step carries both
(query flags `0x40` attribute, `0x80` race, 64-bit in this core), and a monster whose Type or Attribute differs from
CARD_Prop's gets its record again with the client's numbers (the place in its type or attribute list, plus one: Dark
Magician DARK 2 / Spellcaster 18). Live: Scroll of Bewitchment's LIGHT showed on Giant Orc's field label and in the
panel.

**Server texts** (issue #10, through the connection DLL). A duel dialog takes only ids of the client's duel prompt
table: a message (`0x519870`: an id, or entries of text ids, card names and numbers), a Select dialog's labels
(`0x4C5880`, which keeps 14 bits of a label's id, `0x4C592E`) and every `%s`/`%d` text. Each looks its id up with
`0x40BB20`, that table's `vector::at` (EDI the vector at data manager +8 `+0x84`, EBX the id; the text is the table's
string base `+0x9C` plus the entry; past the end it throws, `0x537CF0`). So the server sends its own texts on the duel
connection, `0x7A12 {slot, text}` ahead of the dialog (the stock pump drops the id), and names them as ids `0x3F00 +
slot`, 256 slots used in turn; the DLL (`tools/mod/dialog-text.h`) keeps each text and answers lookups of those ids
past the end of that one table with an entry pointing at it, so the client's own markup applies (`0x519F80`: `@n`
colours; the server escapes `<`, `>`, `&`). Messages are built in a 1024-unit buffer, so a text is cut at 400.
Live (2026-11-14): Pyramid Energy's two options showed as the Select dialog's labels and the first answered +200 ATK;
the Synchro material list read "Select the monster(s) to use as Synchro Material." in the client's colours.

**Option texts.** A card's options the client has no label for (`card_options.rs` pairs 44 cards' strings with the
prompt table) open as the Select dialog with the engine's own text for each option (the card's strings, `str1`–`str16`
in `cards.cdb`) as the server's texts, an option with a client label keeping it; a label is one line, so a text longer
than 56 characters is cut at a word with "…" (live: 57 of Pyramid Energy's 61 characters showed). The dialog has four
labels: more options open as the card list, the card once per option, and the server puts their texts in the player's
chat window as the list opens, numbered as listed: `0x2455 {from_id 0, pname "Effect", chat_type 0, chat_str "1: …"}`
(**guess**: the name and type, chosen to show in the duel's chat window). A selection hint or a Yes/No question that
is a card's own string (code << 20 | n) is shown too: the engine adds `hint_text` / `description_text`, a card list
takes it as its message, and a Yes/No dialog adds it on a line under its client text ('Use the effect of "%s"?').
EDOPro's hints with no client text (`HINT_WORDS` in `translate.rs`: Synchro, Xyz and Link Material, positions, …) are
the server's own words in the table's style (**guess**: the original's wording is unknown).

**Numbers.** The number dialog (`0x4C6430`: max `param3 >> 24`, step and min 4 bits each, format 302 thousands / 303
hundreds / else ones; `NumberSprite` to five figures) says "Declare monster level." (280) only after the Level hint
(HINTMSG_LVRANK 567, Reasoning): EDOPro's Duel.AnnounceLevel is an alias of AnnounceNumberRange, and the client's
other cards declare counts and die rolls with it (Big Eye, Ancient Telescope, Card Trooper, Sixth Sense).

**What the dialogs can't do** (`dlg_selstand.swf`, `dlg_seltype.swf`, `0x4C5E50`, `0x4C5B50`): the position dialog has two
buttons, face-up attack and face-up or face-down defense, so a prompt allowing face-down attack, or three or four
positions, can't offer them all (none came up in 1,200 self-played duels of 600 cards); the type dialog takes one type
(`onItemClick` stops at the first), and no card of the client's declares several; numbers beyond 255 of a unit, or with
a minimum or step above 15, don't fit (no card of the client's asks for one).

**Sums.** MSG_SELECT_SUM (cards whose Levels make a total: a Ritual Summon's tributes, Machina Fortress's discards)
is the client's card list of `selects`, kind 1, at most `max` picks (0: no bound), under the engine's hint or else
text 207 "Tribute the necessary monsters to match the required number of Level.". The list has no OK button: the
player confirms the picks with a right-click (`dlg_cardlist.swf`'s `onMouseUp`, `_cancelable` always set by
`0x4C73A0`; kind `0x27` would close only at `max` picks). The server checks they make the sum (exactly, or with
`select_max` at least it with no card to spare, a card counting as either of its two levels) and shows the list again,
with its text, when they don't. A sum nothing can make gets the automatic answer, so no player is held by an
impossible list. Before, the server picked the smallest fitting set for the player: 140 times in 2,270 self-played
duels of Konami's own decks. Live (`--duel-deck 4222,2600,2602,2153`): Machina Fortress listed the Machines in hand
under "Select the card(s) in your hand you want to discard."; Sniper and Cyber Dragon with a right-click summoned it;
Cyber Dragon alone brought the list back, and Cyber Dragon and Sniper summoned the second.

**Backing out.** A choice the engine lets the player cancel (`can_cancel`: a card choice, or a summon procedure that
picks its cost in a cancellable step, as The Tricky's discard does) opens as kind 1, and a right-click with nothing
picked answers it cancelled; the duel check's round trip requires that answer to be reachable. Live: The Tricky's
discard list, right-clicked empty, went back to the Main Phase with the hand whole. A procedure that picks its cost
inside its own operation (Machina Fortress's `SelectWithSumGreater`, among others) has no cancel in the engine: the
summon has begun, and the list stays until the player picks.

The client's other two windows for these are banners, not pickers: Discard (dialog kind 13, `DuelDiscardDialog`,
`dlg_discard.swf`, `0x4C67F0`) shows the message entries and `setCardNum`, the cards still to discard, read from the
engine's `+0x131C` and redrawn as it changes (`0x4C68E0`), closing at 0; Ritual (kind 14, `DuelRitualDialog`,
`dlg_ritual.swf`, `0x4C6A50`), a draggable one, shows `setStarNum`, its target (`param3`) less the engine's `+0x1320`,
closing a second after reaching it (`0x4C6D00`). Command 1014 (`0x3F6 {+0x1320, +0x131C}`) sets both counts, and the
original server sent it 15 times a duel. So the original let the player click the cards on the field and in the hand
while a banner counted down; the client sends a list's picks only when the list closes, so these banners can't follow
picks in the card list, which needs no banner.

**Choices.** DuelScene opens dialogs with two commands (both popped like any other):

- `0x36 {kind, param2, param3}` (`0x514C0C`) builds a dialog by kind through the factory `0x4C8CD0` (kind table at
  `0x4C9178`): 0 OK, 1 Info, 2 Yes/No, 3 Effect, 4 Select, 5–6 Type, 7 Attribute, 8 Position ("Stand"), 9 Coin, 10
  Dice, 11 Number, 13 Discard, 14 Ritual (12 and above 14 build nothing). Each answers `duelRequestName 4 {DLGResult}`.
  Online 2's notes give the same kinds. Yes/No (`DuelYesNoDialog`, `0x4C4FB0`, `dlg_yesno.swf`): param2 is its
  message (1 = the message entries below, else a duel prompt text by id), buttons 0 "No" (UI text `0x2775`) and 1
  "Yes" (`0x2774`); `DLGResult` is the pressed button (`0x4C50F0`).
- Effect (`DuelEffectDialog`, `0x4C5120`): the same Yes/No with param2 as its message, as `dlg_yesno_card.swf` showing
  the list's `0x3F2` rows (at most five) when `0x3EF`'s third value is `0xFF`, else `dlg_yesno.swf` (`0x4C51A0`).
  param3 is a level: 4 or more always shows it; below, the player's duel option `invoke_conf` and two flags of the
  game object (`+0x770`, meaning open) decide (`0x4C5560`); unshown, it answers `DLGResult 0` at once.
- Select (`DuelSelectDialog`, `0x4C5750`, `dlg_select.swf`): a list of the `0x3F0` values up to `0x3EF`'s count
  (`0x4C5880`): the low 14 bits a duel prompt text, listed when bit 15 is set; four labels at most. param2 is the title.
  `DLGResult` is the chosen value's index (`0x4C5B20`).
- Position (`DuelSelectStandDialog`, `0x4C5E50`, `dlg_selstand.swf`): param3 is a card uid (low 16 bits), whose
  CARD_Prop row from the card table (`0x5220B0`) shows on both buttons, and a flag (bit 16): button 0 is "head_atk",
  button 1 "head_def" with the flag, else "tail_def" (the card's back). param2 is the message. `DLGResult` is the button.
- `0x37 {player, kind, message}` (`0x514CCB`) opens the card list (`DuelCardListDialog`, `0x4C72D0`; kinds `0x4C`–`0x4F`
  the extended one, `0x5131E0`), `dlg_cardlist.swf`. It shows the list the data commands left in the engine state
  (pop `0x522A50`): `0x3EF {count, max picks, _}` (`+0x2A8`, `+0x9B8`; max 0 = view only) resets the lists, `0x3F2`
  gives CARD_Prop rows four at a time (`+0x9BC`, read by getCardInfo `0x4C78A0`), `0x3F3` a mark per card four at a
  time (`+0xE6C`: bit 15 the opponent's card, low bits icons whose meaning is open); `0x3F0` gives u32 values four at a
  time (`+0x2B0`, low 31 bits kept). The kind sets how many picks
  OK needs (`0x4C72D0`): 1–5, `0x48`, `0x4B` none; 6–`0x26` all of max; `0x27`–`0x47`, `0x4A` one. A non-zero message
  builds the dialog's text from `0x3EC {entries}` and `0x3ED {entry ×4}` (`0x519870`); an entry is kind << 28 | value:
  1 a text of the duel prompt table, 2 a line break, 5 a card name by Konami id (`0x5374B0`) put into the text's
  `%s`; 3 and 4 (formatted texts), 6, 7 (names through `0x5375D0`, `0x5376B0`) and 8 (a number) are not used yet. Closing it sends `6 {SetIndex}`, a
  `7 {index, data}` per pick and `4 {DLGResult: picks}` (`0x4C74F0`).
  Entry kind 8 is a number put into the previous text's `%d` (`0x519D60`; shape, live: "Select 2 card(s)." read
  right). A list that needs two or more picks (the engine's min, even when the list opens as kind 1 because it can be
  cancelled) ends its message with a line break and duel prompt text 592, "Select %d card(s).", with the list's max
  as the number. Live (Magical Hats, `--duel-deck` of Hats and Captains): the Deck's Spell/Traps list is a plain
  MSG_SELECT_CARD min 2 max 2 (17 cards, kind 6): one pick only marks (1), the second decides at once (the Flash has
  no OK button to show, `onCardClick`), and the hint alone ("Select the monster(s) to Special Summon.", which is
  EDOPro's hint and right for the card) never said a second pick was needed, so the stall in the screenshot session
  was a player waiting for an OK that this list never has. Live after the change: Gaia The Fierce Knight over two
  Marauding Captains read "Select the monster(s) as Tribute. / Select 2 card(s)." and two picks summoned it. The
  server, like the original, waits for the player on an open list (between players the duel clocks end it, see
  Clocks).

The server uses the card list for the client's MSG_SELECT_CARD and MSG_SELECT_TRIBUTE (targets, tributes, attack
targets, discards…): the rows from its mirror, the opponent's cards marked, kind 1 for min 0, `0x27` for min 1, else 6
with max lowered to min. Settled by the client: only `0x4C72D0` reads the kind, as the number of picks OK needs (`acceptCount`): kinds 1–5, `0x48`, `0x4B` 0; `0x27`–`0x47`, `0x4A` 1; 6–`0x26` all of max (`+0x58`); the Flash decides as soon as the picks reach `selMax` and `acceptCount` can only be 0, 1 or max, so a min of 2 or more with a larger max cannot be asked and max is lowered to min. Titles come from the `0x37` message, not the kind; no per-kind sound code exists. Marks: the whole mark goes to the Flash (`0x4C78A0`); `0x8000` selects the owner frame (`mcOwner`) and every other set bit k draws the `CardAttr` icon `e<k>` (k 0–14, 16 px apart: decorative status icons no client code ties to a meaning), so no icons is right. The list draws each card from `data/mini/` by its row's Konami id ("%4d_%d",
getCardInfo `0x4C78A0`), so an opponent's hidden card (row 0) drew nothing and could not be clicked (live: Mystical
Space Typhoon could not target a Set card). Only kind `0x4A` (all cards, marked the
opponent's, one pick, no way out) and kind 4 (see Pile views) draw `0000.png`, the card back. Kind 4 draws it only for
the cards whose table flag is clear, so a list holding a hidden card (a Set card has row 0 for the client) with min ≤ 1
opens as kind 4 with the list's player (command `0x37` param1, `+0x48`) set to the CPU (1; with 0 the check is skipped
and every card shows its face, `0x4C7A38`): the flag is read at the banished place of the same index on the side
`client != player` (`0x523DA0 {side, 0xF, index}`), which is the opponent's. The server sends a table entry (1000) per
list card there first, with a made-up uid (`0xF00000 + index`), the row and the flag (`1 << 24`) only for the cards
shown, and sends the opponent's real banished entries again before anything else once the list has closed. Kind 4
needs no pick (like kind 1), so a click on a card decides at once when picks reach max, a right-click closes with none,
and a forced choice closed with nothing picked is asked again. Face-up cards keep their faces and ATK/DEF, Set cards
are backs, all marked the opponent's. Live (Auto Duel, Giant Orc's attack on a Set Witch and a face-up Giant Orc): one
back, one face; a right-click withdrew the attack; a click on the back attacked the Set card. (Kaiba's MST listing a
Set Spellbinding Circle was the first use of `0x4A`; it is no longer sent.) EDOPro's hint
before the choice (MSG_HINT type 3, HINTMSG_*) picks the message when a text says the same thing: 500 → 18 Tribute,
501 → 347 discard, 502 → 281 "Select card to destroy.", 503 → 70 remove from play, 504 → 58 Graveyard, 505 → 348
return to the hand, 506 → 88 add to hand, 507 → 78 return to the Deck, 508 → 39 Normal Summon, 509 → 25 Special
Summon, 510 → 46 Set, 514 (face-up, which EDOPro asks for targets such as Spellbinding Circle's) → 94 target, 518 →
101 "Select a monster to equip.", 520 → 119 take control, 526 → 341 show, 528 → 357 change position, 549 → 222 attack
target, 551 → 94 target; otherwise none. Live: MST read "Select card to destroy.", Gravity Axe - Grarl "Select a
monster to equip.". The picks answer the engine;
no pick cancels when it allows: a choice the player may back out of (`can_cancel`, or a chain list that isn't forced)
opens as kind 1 whatever its min. The list's own script (`dlg_cardlist.swf`) then cancels on a right-click: with
`_cancelable` (always set, `0x4C73A0`) and no picks required (kind 1–5 → `+0x5C` 0, `0x4C72D0`) it calls `decide`
with what is picked, here nothing, so `4 {DLGResult 0}`; a card click still decides as soon as the picks reach the
list's max, and its OK button never showed. Live (Windows VM): right-click withdrew Dream Clown's attack, which could
then attack again. Kinds `0x4C`–`0x4F` open DuelCardListDialogEx (`dlg_cardlistex.swf`, `0x514290` case `0x37`), a
list with Search and Revert. Live: Giant Orc's attack opened the list with "Select an attack target monster." and
the CPU's monster marked as the opponent's; picking it sent `6`, `7 {index 0, data 1}`, `4 {DLGResult 1}` and the
attack dealt 1200.

**Attack targets are a dialog, as in the original.** The client has no way to pick a target on the field: DuelBattle
(`0x4F8A90`) handles command 4 only in battle mode 3, to highlight the client's own attackers (bit 0), and `0xA`–`0xE`
are the battle's animations; DuelFieldManager's command 4 (`0x50A690`) highlights cards of either side that have
command bits, and a click runs the hovered card's menu entry (`0x50B700` → `0x4F9510`), with no target step; modes 4
and 5 are "Checking the Field. Right-click to finish." (UI text `0x2214`; the right click, with other-flag 6, sends
`duelRequestName 5`). The dialog text table (`DLG_Text_E.bin`, the duel prompts) holds the original server's battle
dialogs instead: 222 "Select an attack target monster.", 225 "Attack your opponent directly?", 223 "Replay happens.
Continue to attack?", 265 "Switch the attack target to another monster?", 154 "An attack target was selected.". So the
server keeps the card list for targets, and words EDOPro's battle questions (processor.cpp: `SelectYesNo` 31 when the
attacker could attack directly or a monster, 30 at a replay) with 225 and 223. Live (`--duel-deck 429`, Jinzo #7
against Jinzo #7): Kaiba's attack asked "Attack your opponent directly?"; No opened "Select an attack target monster."
with Yugi's Jinzo, and the two destroyed each other. With no monster to attack EDOPro asks nothing, so the server
asks 225 itself before the attack of a card that `can_direct` on an empty field: Yes answers the engine, No sends the
battle commands again (**guess**: the original asked before every direct attack).

The client's MSG_SELECT_EFFECTYN opens the Yes/No dialog with 'Activate the effect of "%s"?' (250) and the card's
name, MSG_SELECT_YESNO with 'Use the effect of "%s"?' (226) naming the card whose script string the question is (the
engine adds `description_cid` for a description `code << 20 | n`, EDOPro's `aux.Stringid`), else "Use the effect now?" (270) (**guess**: the
original questions are the card scripts' own strings). `DLGResult` 1 answers yes. Live (`--duel-deck 1149,1449`):
summoning Marauding Captain asked 'Activate the effect of "Marauding Captain"?'; Yes sent `DLGResult 1`, the card list
followed with "Select the monster(s) to Special Summon." and the hand's monsters, and the picked Giant Orc was Special
Summoned.

**Declarations.** The dialog command `0x36 {kind, message, param3}` has a window for three of EDOPro's declarations
(factory `0x4C8CD0`, kinds by the table `0x4C9178`), and the card list one for the fourth:

| Declaration                            | Client window                                                                                                                                                                                                                                                                                                    | `param3`                                                                                                                                                                                                                                                                                                                           | DLGResult                                        | Status           |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ---------------- |
| MSG_ANNOUNCE_RACE {count 1, available} | kind 5, DuelSelectTypeDialog (`dlg_seltype.swf`), "Select @3Type(s)@0." (241)                                                                                                                                                                                                                                    | the allowed types as bits 1–22 in the client's order (UI texts 1031–1052: Dragon, Zombie, Fiend, Pyro, Sea Serpent, Rock, Machine, Fish, Dinosaur, Insect, Beast, Beast-Warrior, Plant, Aqua, Warrior, Winged Beast, Fairy, Spellcaster, Thunder, Reptile, Psychic, Divine-Beast), one pick (`dialog.typeFlag`, `dialog.selMax` 1) | the type's number                                | confirmed (live) |
| MSG_ANNOUNCE_ATTRIB {count, available} | kind 7, DuelSelectAttrDialog (`dlg_selattr.swf`), "Select @3Attribute(s)@0." (242)                                                                                                                                                                                                                               | allowed (bits 1–7: LIGHT, DARK, WATER, FIRE, EARTH, WIND, DIVINE, texts 1061–1067) << 16 \| picks                                                                                                                                                                                                                                  | the picks as bits                                | confirmed (live) |
| MSG_ANNOUNCE_NUMBER {options}          | kind 11, DuelSelectNumberDialog (`0x4C6430`, `dlg_selnum.swf`), "Declare @3monster@0 level." (280) for levels, else none                                                                                                                                                                                         | max << 24 \| step << 20 \| min << 16 \| format: 302 "%d000LP" counts in thousands, 303 "%d00LP" in hundreds, other formats (283 "%d") in ones (`dialog.numMin`, `numMax`, `step`)                                                                                                                                                  | the number in those units (`0x4C66D0`)           | confirmed (live) |
| MSG_ANNOUNCE_CARD {opcodes}            | the card list `0x37 {player, 0x4C, 0}`: DuelCardListDialogEx (`0x5131E0`, `dlg_cardlistex.swf`) lists the client's own card database with a name search ("Input text.", Filter); kinds `0x4C`–`0x4F` pick its filter (CardDuelFilter `0x4976A0`: any card, no extra deck monsters, main deck monsters, monsters) | —                                                                                                                                                                                                                                                                                                                                  | the card's Konami id (`0x4C8290`, id & `0x3FFF`) | confirmed (live) |

The answers carry EDOPro's terms: the type's and attributes' RACE_/ATTRIBUTE_ bits, the number's index among the
options (the smallest at least as large as the one picked: the engine may list them largest first, Card Trooper's
3, 2, 1), and a card by Konami id, which the engine turns into a passcode; races go to the core as one 64-bit value. Several types at once and numbers out of the dialog's range keep the automatic answer. The CPU names a card
the effect's filter allows: the engine runs ANNOUNCE_CARD's `opcodes` (EDOPro's `card_data::is_declarable` stack
program: type, race, attribute, archetype and code tests) over the card database and adds `fits`, the duel's own cards
first; without `fits`, the duel's cards in turn (the core asks again when a name doesn't fit). The client has no other card name window
(DuelIidInputDialog is a debug key on the field). Live (`--duel-deck 1933,229,3609,1449`): Abyssal Designator asked
for an Attribute (DARK, `DLGResult` 4) and a Type (Fiend, 3), and Kaiba had to send a Giant Orc; Prohibition opened
the card search, "Giant Orc" filtered it to one card and `DLGResult` 5586 named it; Star Blast offered 500–1500 LP in
steps of 500 (`param3` 15 << 24 \| 5 << 20 \| 5 << 16 \| 303) and 1000 LP was paid.

Positions: the client's MSG_SELECT_POSITION {code, positions} opens the position dialog with 'Select position for
"%s".' (327), any uid holding the card's id, and the flag when face-up defense is allowed; button 0 answers face-up
attack (else face-down attack), button 1 face-up defense (else face-down). Without an attack position the first
allowed one is taken. Live (`--duel-deck 1149,1151`): Marauding Captain's Special Summon asked for Warrior Dai
Grepher's position with the card upright and sideways; Defence sent `DLGResult 1` and the card moved face-up in
defense (`0x1A` to `0xC004`).

Options: MSG_SELECT_OPTION {options} opens the Select dialog when the client has a label for every option. EDOPro's
system strings with one: SELECT_HEADS/TAILS 60/61 → "Heads"/"Tails" (400/401) under "Coin Toss Selection:" (56), and
DECLTYPE_MONSTER/SPELL/TRAP 70–72 → "Monster Cards", "Spell Cards", "Trap Cards" (440–442) under "Select card type."
(260). Options that are a card's own strings (EDOPro's `aux.Stringid`, `code << 20 | n`, not `<< 4`) get the prompt
table's per-card labels (400–495, 369/370 for a side of the field), which the original server listed per card and
`card_options.rs` pairs by hand with EDOPro's strings where both say the same thing: 110 options of 44 cards
(**guess**: the original pairing is unknown), under "Select effect to activate." (276). A card with an option the table
lacks takes the first. Live (`--duel-deck 1404,1449`): Card Shuffle's effect from the field paid 300 LP and listed
"Your Deck" and "Opponent's Deck"; the second sent `DLGResult 1` and the engine shuffled Kaiba's deck. Live (Time Wizard):
"Coin Toss Selection:" listed Heads and Tails; Tails sent `DLGResult 1`, the coin came up heads, and Time Wizard was
destroyed with 250 LP lost.

Chains: the client's MSG_SELECT_CHAIN {spe_count, forced, hint_timing, selects} with cards to chain becomes the Effect
dialog with those cards, at a level that lets the player's **Activation Message** duel option (`duelOption
invoke_conf`, DUEL OPTIONS) decide whether it shows (`0x4C5560`; the mouse-button flags are the game object's `+0x770`
input state, per the option's help texts):

| Activation Message (`invoke_conf`) | Shows the question                                                                                       |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Manual (0)                         | always, unless the right mouse button is held                                                            |
| Semi Auto Cancel (1, the default)  | at level 2 and up ("low priority" below), or while the left button is held; never while the right one is |
| Auto Cancel (2)                    | only while the left button is held                                                                       |
| (any)                              | always at level 4 and up                                                                                 |

Unshown, the dialog answers `DLGResult 0` (No) by itself (constructor `0x4C5120`). The server sends level 3 when
something happened (EDOPro's `hint_timing` of either player has a timing other than a phase or step, or a chain is in
progress), 1 at a phase or step, and 0 when no card fits the timing (`spe_count` 0, which EDOPro's own client skips)
(client behaviour confirmed: only three bands are observable, below 2 (0 and 1 act the same), 2–3 (the same), 4 and up; which windows the original judged "low priority" is Konami-server-only; `spe_count` is not a client field). The question names the chain's last link (MSG_CHAINING, until CHAIN_END): 'The effect of "%s" is activated. Chain another card or effect?' (7), '"%s"
is activated. ...' (8) for a card in the Spell/Trap zone (**guess**: no client code chooses between 7, 8 and 269, they are the dialog body's text), else "Activate the effect of a card?" (269). No
answers no chain; Yes chains the only card, or opens the card list (one pick) with "Select effect to activate." (276),
where no pick is no chain. A forced chain of several goes straight to the list; of one needs no choice. Live (two Set
Trap Holes, the CPU's summon): the question showed both cards; Yes opened the list, the second pick activated it, the
next chance read '"Trap Hole" is activated. Chain another card or effect?', and No resolved it, destroying the CPU's
monster. Places: see "Places" above.

**Chain links** (DuelChain `0x501560`, states `0x501250`; the record pump `0x523541` queues `{id, request, param1…4}` and
the component reads from `param1`). DuelChain keeps a list of links `{opponent's, zone, link number, effect}`; the
busy check (`0x515481`) holds the queue while it animates (`+0x58` ≠ 0):

| Command         | Params                                                                                                                                                                                                                                                                                                                                                                     | What the client does                                                                                                                                                                                                                                                                                                                                                                                    | Status                       |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `0x33` ChainSet | `param1` the player (compared with `PlayerNum`, engine `+0x1B0`), `param2` the zone the number shows over (0–16: the place table `0x81AE38` read by `0x4E7990`, numbered as moves give zones: 0–4 monsters, 5–9 Spell/Trap, `0xA` Field Spell, `0xB` hand, `0xC` extra deck, `0xD` deck, `0xE` graveyard, `0xF` banished, 16 the field's centre), `param3` the link number | link 1 clears the list and adds itself silently; a later link is added and played (state 1: effect `0x31` at its zone, and at link 1's too when it is link 2; state 2: then the held number `0x30`). The new link is read at index `param3 − 1` (bounds-checked: a skipped link would stop the client)                                                                                                  | confirmed (code; live below) |
| `0x34` ChainRun | `param3` the resolving link's number (read from the same place as ChainSet's); `param1`, `param2` unread, sent as the link's ChainSet (Konami's logs)                                                                                                                                                                                                                                                           | with fewer than 2 links: clears the list. Else link `param3` resolves: the last link first shows a banner (states 3–4, 400), then each plays effect `0x32` at its zone and leaves the list's end (states 5–6), so they must come last link first. Live: the link in `param1` made state 5 look up link −1, and the bounds check (`0x501620`, called at `0x5014B2`) stopped the client with `0xC0000417` | confirmed (code; live below) |

The server sends ChainSet at each MSG_CHAINING {controller, location, sequence, chain_size} (an Xyz material's
effect at its monster's zone; a place the client lacks at the centre, **guess**) and ChainRun at each
MSG_CHAIN_SOLVING {chain_size}, which the engine sends link by link from the last. Live (two Set Trap Holes
chained to the CPU's summon, twice): ChainSet `{0, 5, 1}` and `{0, 6, 2}` showed the numbered links 1 and 2 over
both cards, ChainRun `{0, 0, 2}` and `{0, 0, 1}` played the chain banner and faded each number, Trap Hole destroyed
the monster, and the duel went on.

The duel's end: ocgcore's WIN {winning team, reason} becomes command 2 {result, reason}. DuelScene keeps the result at
MainGamen `+0x81C` and its 1 ↔ 2 swap as the opponent's, and adds a DuelEndEffect (`0x5083A0`) that plays
`data/gfx/duel_win.swf` / `duel_lose.swf` / `duel_draw.swf` for result 1 / 2 / 3 (name table built at `0x754550`) with
text 1999 + reason (`0x508650`): 1 LP 0, 2 deck out, 3 Exodia, 4 Destiny Board, 5 surrender (we send 12: 106 of
Konami's 227 logs end on 12, none on 5), 6 Last Turn, 7 Final
Countdown, 8 "won with" a card, 9 Vennominaga, 10 Exodius, 11 time out, 12 surrender, 13 disqualified, 14
disconnected. Its update (`0x5084D0`) waits until the movie reports it has finished (about 3 s), then marks the
effect done (`+0x28`). EDOPro's core keeps prompting after a win, so the engine ends the duel at the WIN message; its
WIN player 2 is a draw (result 3, no reason text).

After the movie DuelScene (`0x515A30`, flag `+0xD8` set by command 2) opens DuelResult (`0x515D30`, component slot
`0x13`), a Flash screen run by nine steps (`0x510B70`). Its first step (`0x512070`) waits for one of:

- duel command 1500 or 1504 at the head of the queue (peeked, not taken): the match goes on. DuelManager (`0x50FC00`)
  then takes 1504 to Card Swap (`0x50FDF0`, below) or leaves 1500 for the next duel's janken (`0x50F690`); a
  watching client goes to `0x50F690` either way;
- an empty queue and front flag `0x20000` (or MainGamen `+0x8BC`): the match is over. It reads the result below, and
  DuelManager leaves the duel (`0x510220`, scene `0xFFFFD901`) because the match result (MainGamen `+0x820`) is set;
- watching (`+0x8BC`): 1505 at the head is taken, its param1 is the match result (`+0x820`, the other's by
  `0x504D00`: 1 ↔ 2, 3 stays). A watching client skips the front's result (step 1 reads it only when `+0x8BC` is 0),
  so on an empty queue it goes on with `+0x820` unset: the match goes on, back to the pre-duel screen.

**Match Mode.** The rule's `match_type` comes from DuelMenu's Rule Format buttons through the table at `0x790C50`:
1 Single Mode, 3 Match Mode, 6 Both (Auto Duel only); NPC duels send 4. A match is several duels on one duel
connection: after each duel the server sends either 1500 (the next duel's janken; DuelManager's state 0 then skips
the `0x22C4` wait because the duel connection is still open) or, once a player has won two, the front's `0x2009` with
the match result (`duel_result` is the match's, not the last duel's). Auto Duel pairs the same Rule Format, and Both
with either (two Both players get a Single duel, **guess**); a hosted duel's `match_type` goes into the match push.
Live (Create Duel in Match Mode, `--duel-lp 1000`): Kaiba won duel 1, both clients went from the result screen
straight to duel 2's janken, Kaiba won again, and the WINNER / LOSER panels closed the match. Not served: the loser's
choice of who goes first (each duel has its own janken, **guess**).

**Card Swap** (side decking between a match's duels). 1504 instead of 1500 takes the client to `0x50FDF0`: state 1
opens DeckReplace (`0x448EA0`, `data/gfx/cardchange.swf`) on the player's own deck (MainGamen `+0x838`); cards move
between the main or extra deck and the side deck by dragging (`CardDrag`, then `cardmove.swf`), and the exit button
(`ExitReplace`) ends it. State 3 sends the front `0x1F4F {update_type 2, event_id, duel_id}` with the deck as swapped,
state 4 waits for `0x22CA` or `0x22CB` and, with the duel connection open, sends a bare `0xE` on it, then opens the next
janken (`0x50F690`), which needs 1500. The server sends 1504 to a player whose deck has a side deck and 1500 to the
other (**guess**: who the original sent 1504 to is unknown), keeps the swapped deck as the player's entry for the match,
and answers `0xE` with 1500; the next duel uses the swapped deck if it holds the same cards (else the deck as it was).
Card Swap's countdown (`0x4496C0` → `timeSet`) reads the engine's clock slot 4 (`0x448E50`, the timers message's
`other`); at 0 the swap ends by itself. The janken scene shows the same slot (`0x518446`) and at 0 clears a flag of its
movie (`+0x41`), so the server leaves slot 4 at 0: no time limit, and the screen shows 00:00. Live (`--duel-lp 1000`,
Yugi with a 2-card side deck): Yugi lost duel 1 on time, got Card Swap while Kaiba went to the janken, swapped Danipon
in for Crass Clown, the client sent `0x1F4F` update_type 2 → `0x22CA` → `0xE` → 1500, and duel 2 was dealt from the
swapped deck.

Other commands at the head are taken and dropped. The match is over when the **front** server pushes `0x2009`
(dispatcher `0x4F4C9E`), sent as soon as the engine ends the duel:

| Field                                                            | Where                  | Read by                                                                                                                                                                                                                                                            | Status                                                                                                                                                         |
| ---------------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| item `duelData` (by `name`), **tag 7**                           | `0x2009`               | the dispatcher keeps it in front slot `0x15` and sets flag `0x20000`; tag 7 makes a `YO3DuelData` (factory `0x531B90`), which finds its children `gameServer`, `regulation`, `rule`, `players` (`+0x78`) and `result` (`+0x7C`) by name (`0x5315C0`)               | confirmed (tracer, live)                                                                                                                                       |
| `score_<player_id>` int                                          | `0x2009` top level     | the dispatcher, copied as `event_score`                                                                                                                                                                                                                            | shape; left out                                                                                                                                                |
| `duel_end` int                                                   | `result`               | DuelScene (`0x515BD3`): 2 or 3 show text `0x2918` and leave; JankenScene (`0x518631`): 2 and 3 the same, 4 text `0x291B` "%s has disconnected." (the opponent's name from its own data) and leave, 5 opens DuelResult (`0x5196D0`); 1 and anything else do nothing | confirmed; we send 1 (nothing happens) for a normal end, 3 for a duel cancelled before it began, and 4 to a player whose opponent left at the janken or turn order (live, 2026-11-13: "Bob has disconnected.", OK, the WINNER panel with the records unchanged, Next, the map; with 1 the player had stayed on the janken for good) |
| `duel_result` int                                                | `result`               | DuelResult step 1 (`0x5122B0`): the match result (1 won, 2 lost, 3 draw) at MainGamen `+0x820`                                                                                                                                                                     | confirmed                                                                                                                                                      |
| `dlogURL` string                                                 | `result`               | step 1, only when MainGamen `+0x804` is 0: opens a component (`0xFFFFD90B`) on that URL                                                                                                                                                                            | sent for duels between players (see "Duel log files")                                                                                                          |
| `player_id` int                                                  | each item of `players` | `0x510830` finds this player's record                                                                                                                                                                                                                              | confirmed                                                                                                                                                      |
| item `resultData` (by `name`)                                    | the player's record    | step 1 sets DuelResult `+0x60`                                                                                                                                                                                                                                     | confirmed                                                                                                                                                      |
| `win`, `lose`, `draw` and `win_add`, `lose_add`, `draw_add` ints | `resultData`           | getResult (`0x511200`, `0x511934`–`0x511D2D`): the player's panel "Wins 1 (+001)", then lose and win swapped for the opponent's panel                                                                                                                              | confirmed (live); totals and deltas are our reading                                                                                                            |

Without `0x2009` the result screen waits forever. Live (`--duel-lp 1000`): "YOU LOSE" with "Kaiba's Life Points
have been reduced to 0", then LOSER / WINNER panels (Kaiba / Seto Kaiba) with Wins, Losses, Draws and a Next button;
Next leaves to Kaiba Corporation, where the NPC says his after-duel line. The client then refreshes its player
(`0x2CEE`). 1504 instead opened Card Swap. Command 1506 is taken inside the queue pop (`0x522A50`) and makes the
client send a bare `0x1842` on the duel connection (meaning **open**).

The numbering differs from Online 2's engine (whose turn change is `0x04` and LP set `0x26`), so YO2 research is a guide
only; the card mapping and opening hand are implemented as described above.

### Duel log files (`0x512070`, `0x498700`, `0x498B10`, `duellog.rs`)

After a duel the client records it itself from the `0x2009` it gets (front slot 21, flag `0x20000`). DuelResult's tick
(`0x512070`, state 1) finds the player's item in `players`, reads `duel_result` and the text `dlogURL` from the
`result` item, and, **only when MainGamen `+0x804` is 0** (a duel against a player: the flag is set for NPC duels, whose
result screen takes the other branch of `0x511200`) and the text isn't empty, starts the duel log task
(`0x498700 (url)`): `0x498B10` makes a connection object (`0x4EF4B0`, id `-9973`: an HTTP download of the URL) writing
to `duellogs\<temp>\<file name>`, and when it is done (state 10) copies the file into `myduels\` (`0x4980A0`) and
deletes the oldest beyond 100 (`0x498510`). So the log is a file the **server** serves, and the stock client fetches
none after a duel against an NPC (the only other caller of `0x498700` is the Server Duel Logs tab's download; see
"NPC duels" below for the DLL's change). `dlogURL` is
`http://<host>:8080/duellogs/NAME.dlog` (the host is the address the client reached), served by our patch HTTP server from
the `duel_logs` table of `server.db` (the newest 1000 are kept; older versions wrote `data/duellogs/`, which the
server now reports as safe to delete); the name is the URL's, ours `END_TIME-DUEL_ID-PLAYER_ID.dlog`, one file per player. The log is
that of the match's last duel. Live: both clients fetched theirs, Latest Duel Logs listed it ("Lv1 Yugi V.S. Lv1
Kaiba"), Play replayed the duel on the field from the janken to the duel's end, then asked "Save this duel log?"
(No) and returned to the main screen.

**NPC duels** (issue #12). The connection DLL removes DuelResult's `+0x804` jump (`0x5122E6`, `tools/mod/npc-duel-log.h`),
so the `dlogURL` step runs after NPC duels too (with no URL it does nothing, `0x512364`). The server keeps an NPC duel
the engine played as it keeps one between players: from the side of whoever went first, the janken as that side saw
it, every card with its row (the NPC's hidden cards are hidden in the duel, not in its replay). The NPC's block has its
npc_id as `player_id`, its name, level 0 and no avatar, record or speeches (**guess**: what the original put there is
unknown; the replay shows a default avatar). Live (2026-11-14, Seto Kaiba): the client fetched the log, Latest Duel
Logs listed "Lv1 Alpha V.S. Lv0 Seto Kaiba", and Play replayed it from the janken to the last attack, then asked "Save
this duel log?".

The file (readers `0x538AB0` header, `0x5384F0` all, `0x538BD0` comment rewrite; little endian): u16 `0x09E7`, u16 1,
then u32s at 4 (360, the header's size), 8 `duel_id`, 12 `player_1`, 16 `player_2`, 20 `match_type`, 24 `dlv_1`, 28 `dlv_2`,
32 `end_time`; 36 `pname_1` (32 bytes), 68 `pname_2` (32), 100 `dlog_comment` (256), 356 a u32 the rewrite keeps (the body's
length), and from 360 the body, run-length coded as (value, count) byte pairs (`0x538360`). The body
(`0x5384F0`, then `0x524330`, which builds from it the match push `0x1978` of a watched duel, a `duelData` and the
setup): 20 u32s (0–3 the file offsets of the duel data, the two blocks and the commands as if the body weren't coded:
376, 440, 9096, 17752; 4 `duel_id`, 5 the end time (`0x17C`), 6 `match_type`, 7 always 5 or 4, 8 `event_type`, 9
`official`), two player blocks of 8656 bytes, then a u32 count and that many commands of six u32s `{id, 0, param1..param4}`
(`0x522A50`: the engine commands; entries whose second u32's low byte is negative are skipped). A block: 11 u32s
`{player_id, dlv, MainDeckNum, ExtraDeckNum, title, ?, PlayerType, card_protector, n_win, n_lose, n_draw}`, `pname` (32
bytes), the avatar (128), `country` (4), 32 speech lines of 256 bytes and a 256-byte deck name. The setup is not in
the file: `0x524330` makes it from the blocks (FirstPlayer and PlayerNum are 0). So a replay is the duel from the side of
the player who went first, every card with its row as in Konami's replays (both hands show): our commands are the
janken as that player saw it (1500-1503, which take the pre-duel
screen to the field; without them the replay waits on it), the card table, the duel's start and LP, the engine's events
and 1505 (the match's end for that player; without it the replay ends on the pre-duel screen). The first player's
block comes first, so `player_1` is whoever went first.

**Konami's own logs.** A player's 2010–2012 client folder (227 readable logs downloaded from the original server,
`tools/re/dlog.py`) settles the layout: the u32 at 4 is 360 in all, the u32 at 356 the body's length, the body's first
four u32s the offsets above, its u32 7 is 5 (4 in 6 logs, meaning unknown), the blocks' sixth u32 and deck name 0, the
comment empty, `player_1` the first player in all, and no card table entry carries row 0. `event_type` ran 1–11
(`official` 1 with 3, 4, 10 and 11); we send 0 for both, not knowing which of ours each was. The original named files
`DUEL_ID_HHMMSS.dlog` (ten digits, Japan time), one file per duel; ours keep the end time first, since our duel ids
restart with the server. A command's second u32 was 0, except `0x80` on 2,627 card-relation commands (1017, skipped
on replay) and 1 on 87 dialogs.

## Accounts

The client never serializes a user id or password: its login credentials stay on the front connection object
(`+0x118`/`+0x134`, on `0x219F`), `0x1F47` is bare and `0x1F48` has only `lang`. Delete Player's password prompt
sends no password either (`0x1F4B {player_id}`, live packet). The authenticated TLS transport therefore chooses the
account before the original login protocol runs. `Directory::authenticate` checks the account key directly and
never falls back to the source address. Separate launcher sessions have distinct front/duel identities.

Select Player still chooses one of up to four account players (`0x1F4C`), and creates/deletes them (`0x1F4A`/`0x1F4B`,
slot `pindex` 0–3). `CharMake.cpp` (`0x2CF5`/`0x2CF6`, the map's GoCharMake) remains the appearance editor. Account
and player records keep their existing document schema and ids, stored in SQLite's `accounts` document.
`profile EXISTING_PLAYER FILE --advertise HOST` exports
a private launcher profile, adding a key to an old address-only account without moving its cards or other data.

The old patch-URL/IP association and HTTP self-service join page have been retired. `/join` and INI downloads now
return 404; fetching `yo3.idx` cannot select or create an account. The internal asset service serves the client's
index, `/duellogs/NAME.dlog` and `/app/`; the launcher carries these requests over TLS. The DLL routes the client's
WinINet calls to its local authenticated asset proxy, including duel-log URLs (`0x498700`). No account key is put
into a remote URL. See [authentication.md](authentication.md) for setup and credential lifetimes.

Persistence uses `data/server.db`, schema version 1. The `state_documents` table stores each domain's existing
serialized document: accounts, records, friends, mail, teams, events and rentals. References elsewhere in this
catalog to `accounts.json`, `friends.json`, etc. name their original formats, now imported once into SQLite.
The importer commits all documents together, preserves source JSON and validates accounts before recording success.
`storage.version` prevents an accidentally deleted database from causing a stale reimport. This changes storage,
not Konami request/reply fields; tests live in `store.rs` and `tests/accounts.rs`.

The desktop's **own** local protocol is `POST /manager` with a private admin bearer token and JSON `op`:
`status`, `accounts`, `create_account {name}`, `add_player {who,name}`, `export_account {who}`, or `shutdown`.
It binds loopback and the TLS asset-role whitelist denies access. Account listings omit credentials; export is the
explicit credential-bearing operation. Unknown operations/fields and incorrect tokens are refused. Mutations persist
once and restore memory if saving fails. **Status: implemented/tested**, evidence `tests/manager_contract.rs` and
the GUI's live account/create/play check; these are our API definitions, not inferred game-client reply shapes.
Status also reports the actual `listen` socket and `release`, so a reopened manager and local game use the active
listener rather than guessing from saved settings. Only the authenticated game/discovery port is public; internal
management/assets HTTP stays loopback.

## Fields the client reads that we leave out

The field tracer shows these lookups failing, and the client copes: an absent int reads as 0, an absent raw value as
NULL, which these callers check. Send them when we know what they mean.

| Reply             | Field                                                                     | Where it's read                    | Notes                                                                                                 |
| ----------------- | ------------------------------------------------------------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `0x2201`          | `bgm` int (top level)                                                     | global dispatcher `0x4F506A`       | probably the lobby music                                                                              |
| `0x21FC`          | `icon`, `lefttime` (int), `maintenance` (string), in the `{lang}` item    | `0x4F477C`, `0x44E6EF`, `0x44E72D` | a maintenance countdown; absent = none                                                                |
| player record     | `npc_id` int                                                              | `0x51BFE2`                         | 0 = a human player                                                                                    |
| `0x27D9`          | `cpuUse`, `notRelease` (raw) in its record                                | `0x4F2080`, `0x4F20BA`             | meaning open (CPU duels? unreleased cards?)                                                           |
| (around `0x2CF7`) | `premium` uint, from **another dataset**, not the reply                   | `0x44F3E7`                         | probably the account's Duelist Premium flag                                                           |
| `0x2CF7` items    | `event_score`, `tprank`, `world_evid`, `official_evid`, `user_evid` (int) | `0x475A20`                         | omitted values read as 0; duel id/status/match type are sent when applicable (see automatic matching) |

Settled from the client's code (no value to send):

- `0x2201` `bgm` is stored at `UserData+0xCC` (`0x4F506A`); no read of that field was found (heuristic sweep): omit.
- `0x21FC` `icon`, `lefttime`, `maintenance` (`0x4F4700`, `0x44E130`): `lefttime` > 0 sets flag 2; `icon` 1 or 2 with `endtime`
  sets bell flag 4 or 8 until then; InfoIcon shows the maintenance notice only when `maintenance` is non-empty, not
  "-1", and `lefttime` > 0. Absent = no badge, no notice, which is right.
- player record `npc_id`: `0x51BF90` picks the loader: 0 the human one (`0x51C260`), non-zero the NPC one (`0x51C4F0`).
  Omitted = 0 = right for players.
- `0x27D9` `cpuUse` / `notRelease`: raw lists of u16 card keys in the class built by `0x4F2010`; `notRelease` keys map to
  0, `cpuUse` keys to 2, ids `0x430` and `0x530` to 3, and the predicate `0x410EA0` counts a card only when it is absent
  from the map or 3. The main screen (`0x4705F0`) uses it for the collection-completion percentages. Sending neither
  makes every card count, the neutral choice; the key type and byte order were not verified, so we leave them out.
- `PlayerType001/002` (pump case `0x15`): stored, never read. Card table `param3`: only the hover help of command 100
  (`0x523D20`, a text id; 0 shows nothing); the flag byte (top byte of the uid) is stored (`0x5000E0`), no consumer found.

## Requests with a timeout

Only eight poll state machines have a timer: on expiry they show text `0x2910` "There is no response from the server."
with the request id in decimal. Each sends one request and waits for its reply or error id: `0x2072` (`0x2398`/`0x2399`,
`0x472DA0`), `0x2583` (`0x2588`/`0x2589`, `0x472F70`), `0x27D8` (`0x27D9`/`0x27DA`, `0x4731A0`), `0x271A`
(`0x271B`/`0x271C`) and `0x2710` (`0x2711`/`0x2712`) (both `0x4733C0`), `0x2CEE` (`0x2CF3`/`0x2CF4`, `0x4736C0`), `0x2AFB`
(`0x2B13`/`0x2B14`, `0x473830`), `0x2AFD` (`0x2B17`/`0x2B18`, `0x473970`). The check is `0x4038C0` (a start tick plus a
duration). Every other request has no timer: scenes read specific reply ids or ignore the reply, so a missing reply to
a trade or rental ready or cancel hangs nothing visible.

## Families seen in the client, not mapped yet

- `0x2CEC`–`0x2CF8`: mapped in the login, search and avatar sections; any other ids in this range remain open.
- `0x2774`–`0x277A`: presents (`PresentWindow.cpp`). `0x2774` = count, a push the global dispatcher keeps in slot 23
  (it also sets the "presents pending" flag `0x80`); `0x2777/0x2779` carry full present records (`present_id`,
  `type`, `level`, `form`, `item_img`, `end_time`, `item`…); `0x2778/0x277A` mean none (see `0x2775`).
  The same numbers also show up as UI text ids (`FUN_0051E6C0(table, id)`): don't confuse the two.
- 48 more `REQUEST_` / `RESPONSE_COMMUNICATION_*` names for trade and rental (search the exe's strings).
- `DuelServerConnection` is mapped under "Duel connection"; remaining command meanings are labelled open there.
- Official duel logs were served over HTTP (`http://download.yugioh-online.net/duellogs/last/`), alongside team and
  Online 2 downloads under `download.yugioh-online.net`.
