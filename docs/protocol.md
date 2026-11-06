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

## Wire format

- Frame: `u32 BE (len(body) ^ 0x77777777)` + body.
- Body: Blowfish-ECB, key `"12345678"`, each 32-bit word little-endian (`BlowfishLE`), zero-padded to 8 bytes.
- Message: `u8 tag | u16 nfields | (u8 namelen, name, u8 vtype, value)… | u16 nitems | items…`, big-endian.
  - Value types:

    | vtype | Kind | Encoding |
    |---|---|---|
    | 5 / 6 | u32 | 4 bytes |
    | 7 / 9 | text | `u32 len` + bytes; the client stops reading at the first NUL |
    | 8 | binary | `u32 len` + bytes; NULs are kept |

  - Tags: replies and requests use 4, list records use 3. The message ID is the `request` field.
- Every connection opens with a bare version string: `00 12 "2010.03.23 skelton"`.
- The client's `.y3d/.y3a/.y3r` files use the same frame and message layout.
- Client-side failures:
  - An unanswered poll shows "There is no response from the server. (*decimal id*)". For example, `(8,306)` is
    `0x2072`.
  - A missing field that the client reads without a NULL check is a crash: Wine's "Unhandled page fault" at the
    reading address.
- Timestamps sent to the client are **JST** (UTC+9). The client converts them to local time.

## Gate (TCP 5730)

| Request | Reply | Fields | Evidence | Status |
|---|---|---|---|---|
| opener | `0x21FE` | `host` text, `port` u32 (front server) | `LoginWindow.cpp L303/L314` | confirmed |

## Front (TCP 1213)

### Login (dispatcher `0x469C00`, `LoginWindow.cpp`)

| Request | Reply | Fields | Evidence | Status |
|---|---|---|---|---|
| opener | `0x219F` | none | "front server : version valid." L370 | confirmed |
| `0x1F47` auth | `0x219A` | `regulation_id`, `user_id`, `uname`, `dpoint` | "user auth success" L395 | confirmed |
| `0x1F48` login `{lang}` | `0x2201` | top: `regulation_id`; item[0]: `user_id`, `uname`, `dpoint` | L449, `uid : [1001]` L457 | confirmed |
| `0x1F49` player list | `0x2260` | items = player records (below) | "MyPlayerList" L509, loader `0x51C260` | confirmed |
| `0x1F4C` select `{player_id}` | `0x2266` | none | "my player use success" L577 | confirmed |
| `0x1FA4` trunk `{player_id}` | `9000` (`0x2328`) | exactly one record; item[0] is copied wholesale into `UserData+0x8C8` | "trunk data success" L666, `0x46AFA8` | shape (contents: **guess**, empty) |
| (server push after `9000`) | `0x251C` friend list, `0x252A` black list | items `{friend_id, friend_name, login_flag}` / `{black_id, black_name, login_flag}` (`0x475A20` modes 2 and 3); we send none | the global dispatcher stores them in dataset slots 6 and 7; the client never asks, and listing any player crashes without them (`0x45E734`) | shape (empty); push timing **guess** |

**Player record** (read by `0x46C1E4` and `0x51C260`):
- `pname`, `pindex`, `player_id`, `dlv`, `title`, `n_win`, `n_lose`, `n_draw`, `team_level`, `team_name`,
  `team_authority`.
- `avatar1`: **binary**, 34 bytes (17 BE u16). Sent as text, it crashes at `0x46C750`.
- `pbin`: **binary**, required. The main screen reads byte `0x4B` at `0x4727DE`; without it the client crashes.
  We send 512 bytes, a **guess** based on the 4,096 entries of `yo3pbininfo.bin`.
- `speech00`–`speech22` and `comment00`–`comment01`: text.

### Main screen (MainGamen polls `0x472EDC…`, global dispatcher `0x4F4A00`)

The main screen re-arms each poll on a timer (`obj+0xE8 = request id`). Every reply also passes through the global
front dispatcher; `python3 tools/re/dispatch.py 0x4f4a4f <id>` shows the case it lands in.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x2072` regulations | `0x2398` (`0x2399`) | `regulations`: binary u16 BE regulation ids | read unchecked at `0x4F545B` | confirmed field; list contents **guess** (all 20) |
| `0x27D8` | `0x27D9` (`0x27DA`) | one record (copied into a 0xC4-byte object) | item[0] read unchecked at `0x4F5BE9` | shape |
| `0x2CEE` player refresh `{player_id, type}` | `0x2CF3` (`0x2CF4`) | item = the selected player's record; its `player_id` must match | `0x47378A`, `0x4F5FC2` | confirmed |
| `0x2583` | `0x2588` (`0x2589`) | `count` u32 | `0x4730B0` | shape (unread mail or presents?) |
| `0x271A` | `0x271B` (`0x271C`) | none | `0x47352F` | shape |
| `0x2710` | `0x2711` (`0x2712`) | none | `0x473646` | shape |
| `0x2AFB` | `0x2B13` (`0x2B14`) | none | `0x4738FA` | shape |
| `0x2AFD` | `0x2B17` (`0x2B18`) | none; the global case reads `member_id`, `authority` if present | `0x473A3A`, `0x4F5E0E` | shape (team?) |
| `0x2775` present count (bare) | `0x2774` | `present_count` u32 | builder `0x4F3A00`; PresentWindow `0x476DE0` reads it (field trace) | confirmed |

### Info and chat windows

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x1F41` info list | `0x21FC` | item[0] `{lang}` whose items are `{starttime "%Y-%m-%d %H:%M:%S" JST, title, body}` | "information recv success!" `InfoWindow.cpp L1547`, `0x458A64` | confirmed (`lang` value **guess** "1") |
| `0x2CF0` player search `{back_num, base_point, front_num, search_flag, search_type}` | `0x2CF7` (`0x2CF8`) | `result_number` u32 (top, read at `0x4591CA`); items = players (shape unknown; we send none) | `RESPONSE_PLAYER_SEARCH_SUCCESS` `ChatWindow.cpp L651`, field trace | confirmed (empty) |
| `0x2724` heartbeat | none | none | builders `0x4F17DF` (every 30 s) and `0x4F45A5` (after 120 s); no reply is checked | confirmed |

### Chat (ChatWindow dispatcher `0x435950`)

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x2135` room list | `0x2458` (`0x2459`) | top `lang`; items = rooms `{id, name, capacity, num, region, status, type}` (read by `0x434D20`); `id` ≥ 1 | "chat room list success!" `ChatWindow.cpp L339`; the room chooser lists them with `[num/capacity]` | confirmed (`region`/`status`/`type` **guess** 0) |
| `0x2136` enter `{room_id}` | `0x245A {room_id}` (`0x245B {error_code}`) | the client ignores `room_id` < 1 | "In>>Lobby" printed, tab renamed | confirmed |
| `0x2137` leave | `0x245C` | none | "chat room exit" | shape |
| `0x2134` send | `0x2456` (`0x2457`) | fields sit in **one item (tag 10)**: `chat_str`, `chat_type`, and `to_id` or `to_name`. Room chat = `chat_type` 1 with `to_id` = the room id; whisper = `to_name` | captured packets; builders `0x4F3E00` (to_id) and `0x4F4030` (to_name) | confirmed (room); whisper shape |
| (server push) | `0x2455` | items `{from_id, pname, chat_type, chat_str}`; the handler also checks `friend_id`/`member_id` | two clients see each other's lines; the client shows nothing on `0x2456`, so the sender needs the echo | confirmed |

## Accounts

The client never sends a user id or password. `0x1F47` is a bare request; the credentials typed at login are copied
onto the front connection object (`+0x118`/`+0x134`, on `0x219F`) and never serialized. The server therefore tells
accounts apart by the client's address and lets the player pick one of up to four players at Select Player
(`0x1F4C {player_id}`).

## Fields the client reads that we leave out

The field tracer shows these lookups failing, and the client copes: an absent int reads as 0, an absent raw value as
NULL, which these callers check. Send them when we know what they mean.

| Reply | Field | Where it's read | Notes |
|---|---|---|---|
| `0x2201` | `bgm` int (top level) | global dispatcher `0x4F506A` | probably the lobby music |
| `0x21FC` | `icon`, `lefttime` (int), `maintenance` (string), in the `{lang}` item | `0x4F477C`, `0x44E6EF`, `0x44E72D` | a maintenance countdown; absent = none |
| player record | `npc_id` int | `0x51BFE2` | 0 = a human player |
| `0x27D9` | `cpuUse`, `notRelease` (raw) in its record | `0x4F2080`, `0x4F20BA` | meaning open (CPU duels? unreleased cards?) |
| (around `0x2CF7`) | `premium` uint, from **another dataset**, not the reply | `0x44F3E7` | probably the account's Duelist Premium flag |

## Families seen in the client, not mapped yet

- `0x2CEC`–`0x2CF8`: player search, character creation (`CharMake.cpp`: `0x2CF5/6`) and player data.
- `0x2774`–`0x277A`: presents (`PresentWindow.cpp`). `0x2774` = count; `0x2777/0x2779` carry full present records
  (`present_id`, `type`, `level`, `form`, `item_img`, `end_time`, `item`…); `0x2778/0x277A` re-request `0x2775`.
  The same numbers also show up as UI text ids (`FUN_0051E6C0(table, id)`): don't confuse the two.
- 48 more `REQUEST_` / `RESPONSE_COMMUNICATION_*` names for trade and rental (search the exe's strings).
- `DuelServerConnection`: the third server tier, not reached yet.
- Official duel logs were served over HTTP (`http://download.yugioh-online.net/duellogs/last/`), alongside team and
  Online 2 downloads under `download.yugioh-online.net`.
