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
  - The client builds each item as an object chosen by its tag (factory `0x531B90`, a jump table on tags 0–10):
    3, 5 and 7 make a plain dataset, 4 a request object, **6 a `YO3Deck`** (`0x414240`), 8 and 9 other types,
    10 the chat item. Code that treats an item as a deck needs tag 6; a tag-3 deck aborts the client (`0x413F63`).
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
| `0x1F48` login `{lang}` | `0x2201` | top: `regulation_id`; item[0]: `user_id`, `uname`, `dpoint`, `premium` (copied into UserData by `0x51B570`; non-zero opens every NPC, `0x4BF520`, and Form Team and Create Event) | L449, `uid : [1001]` L457; live: all 25 NPCs enabled with `premium` 1 | confirmed; `premium` 1 for everyone is our choice |
| `0x1F49` player list | `0x2260` | items = player records (below) | "MyPlayerList" L509, loader `0x51C260` | confirmed |
| `0x1F4C` select `{player_id}` | `0x2266` | none | "my player use success" L577 | confirmed |
| `0x1FA4` trunk `{player_id}` | `9000` (`0x2328`) | exactly one record; item[0] is copied wholesale into `UserData+0x8C8`, and the global dispatcher keeps the reply in slot 18. The record holds `cards` and `noTrade` (binary): owned and untradeable count per card, one byte at index row − 1 (CARD_Prop rows, row 0 a dummy, so 4,833 bytes). `0x410DB0` reads them bounds-checked; the collection builder `0x4F6340` writes into both unchecked (crash at `0x4F6A82`, opening the NPC duel) | "trunk data success" L666, `0x46AFA8`, `0x410DB0` | confirmed; `cards` = one copy per card of the starter deck (**guess**: a real collection comes with the economy); other contents **guess** |
| (server push after `9000`) | `0x251C` friend list, `0x252A` black list | items `{friend_id, friend_name, login_flag}` / `{black_id, black_name, login_flag}` (`0x475A20` modes 2 and 3); see "Friends and the ignore list" | the global dispatcher stores them in dataset slots 6 and 7; the client never asks, and listing any player crashes without them (`0x45E734`) | shape; push timing **guess** |
| (server push after `9000`) | `0x2D6C` rental records | items carry `from_type` (the shop checks for 1); we send none | the global dispatcher stores it in slot 8 (`0x4F5FBA`); opening the shop reads slot 8 unchecked (`0x48F0F6`). `RentalRecord`/`CardRental` ask for it with `0x2C34` (bare), but not before the shop opens | shape (empty); push timing **guess** |

**Player record** (read by `0x46C1E4` and `0x51C260`):
- `pname`, `pindex`, `player_id`, `dlv`, `title`, `n_win`, `n_lose`, `n_draw`, `team_level`, `team_name`,
  `team_authority`.
- `avatar1`: **binary**, BE u16 entries copied into PlayerData+`0x28`. Sent as text, it crashes at `0x46C750`. Character
  settings read entries 17–40 through a checked index (`0x429210`; with only 17 entries the CRT aborts from
  `0x42923F`), so we send 41 zero entries (82 bytes): a default male avatar. The real length is unknown (**guess**).
- `pbin`: **binary**, required. The main screen reads byte `0x4B` at `0x4727DE`; without it the client crashes.
  We send 512 bytes, a **guess** based on the 4,096 entries of `yo3pbininfo.bin`.
- `speech00`–`speech22` and `comment00`–`comment01`: text.

### Main screen (MainGamen polls `0x472EDC…`, global dispatcher `0x4F4A00`)

MainGamen's constructor (`0x46D8A0`) queues startup steps that run one after another, each waiting for its reply
(`obj+0xE8 = request id`): `0x472780` (Online 2 lists), `0x472DA0` (`0x2072`), `0x4731A0` (`0x27D8`), `0x4736C0`
(`0x2CEE`), `0x472F70` (`0x2583`), `0x4733C0` (`0x271A`, `0x2710`), `0x473830` (`0x2AFB`), `0x473970` (`0x2AFD`),
`0x473BE0` (a window if dataset slot 13 has items), `0x473AE0` (presents, below), `0x4725B0` (unlock), `0x473AB0`.
Until the unlock step runs, the map's facilities, the menu and the duel button stay disabled (`0x472230(0x3F)`), so
clicks do nothing. Every reply also passes through the global front dispatcher; `python3 tools/re/dispatch.py
0x4f4a4f <id>` shows the case it lands in.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x2072` regulations | `0x2398` (`0x2399`) | `regulations`: binary u16 BE regulation ids | read unchecked at `0x4F545B` | confirmed field; list contents **guess** (all 20) |

**Banlists.** The client ships 20 regulations (`data/regulation/*.y3r`, named in the text table): 1 "Beta
Forbidden/Limited", 2 "Pre-Season", 3 "unlimited" (no lists: every card 3 times), 4–15 the OCG lists of 2004-03 to
2009-09 (which also limit the card pool to the cards out by then: `addition_card` 0 and the pool as `card3`), 16–20
"Season 12" to "Season 16". Only the client enforces them: its deck check (`0x51AB00`) before Auto Duel and NPC duels
uses the login's `regulation_id` (`0x219A`, `0x2201`), and a hosted duel uses the host's choice. The server sends the
one its `--regulation` setting names (an id, or a name such as "Season 16"; default 3, "unlimited", the user's
choice since 2026-11-08); the starter deck is still the first preset regulation 1 allows, so it passes any list. Live: the Starter Deck breaks regulation 4 (six cards
newer than March 2004); with `--regulation 4` Kaiba Corporation answered "There are no valid Decks available.", with
`--regulation unlimited` the same deck reached the janken. Create Duel → Customize → Regulations shows the login
regulation as "Latest Regulation" (with "unlimited": no Forbidden cards, "Cards not listed are Unlimited.") and Select
lists the others by name with their Forbidden / Limited / Semi-Limited tabs (an OCG list: Limited, Semi-Limited and
"Any", "Cards not listed are Forbidden."). The same screen offers Wait Time Per Turn (Standard, Any, 2, 3, 5 minutes)
and Total Wait Time (Standard, Any, 5, 10, 20 minutes): Standard is the 180 s / 600 s the clocks use.
| `0x27D8` | `0x27D9` (`0x27DA`) | one record (copied into a 0xC4-byte object) | item[0] read unchecked at `0x4F5BE9` | shape |
| `0x2CEE` player refresh `{player_id, type}` | `0x2CF3` (`0x2CF4`) | item = the selected player's record; its `player_id` must match | `0x47378A`, `0x4F5FC2` | confirmed |
| `0x2583` | `0x2588` (`0x2589`) | `count` u32: D-Mail waiting (see D-Mail); a count sets flag `0x1000`, the bell's mail icon | `0x4730B0` | confirmed |
| `0x271A` | `0x271B` (`0x271C`) | none | `0x47352F` | shape |
| `0x2710` NPC roster | `0x2711` (`0x2712`) | items `{npc_type, npc_id, need_ex}`, three per NPC (one per level), kept in slot 4; see "NPC duels and duel start" | `0x473646`, `0x4BEB40`; live: 25 NPCs on three tabs | confirmed; the tabs by series and `need_ex` 100/200/300 are **guesses** |
| `0x2AFB` | `0x2B13` (`0x2B14`) | none | `0x4738FA` | shape |
| `0x2AFD` | `0x2B17` (`0x2B18`) | none; the global case reads `member_id`, `authority` if present | `0x473A3A`, `0x4F5E0E` | shape (team?) |
| `0x2775` next present (bare) | `0x2778` none (`0x2777`/`0x2779` would carry a present) | none | builder `0x4F3A00`. The `0x2201` reply sets flag `0x80` (so does a `0x2774` push), so startup step `0x473AE0` opens PresentWindow, which sends `0x2775` and holds the main screen locked until it closes. Its switch (`0x476E74`): `0x2777`/`0x2779` "PRESENT content get success", `0x2778`/`0x277A` "content get error", which retries while `+0x58` > 0 and then closes (`0x4783C9`); its `0x2774` case reads `present_count` and drops it | confirmed (live: the facilities unlock after `0x2778`) |

### Info and chat windows

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x1F41` info list | `0x21FC` | item[0] `{lang}` whose items are `{starttime "%Y-%m-%d %H:%M:%S" JST, title, body}` | "information recv success!" `InfoWindow.cpp L1547`, `0x458A64` | confirmed (`lang` value **guess** "1") |
| `0x2CEF` player update (Avatar Settings' Confirm, built by `0x51C960`): top-level `avatar1` (binary, 82 bytes live), `speech00`–`speech22`, `comment00`–`comment01` (text), `title` (int) | `0x2CF5` (`0x2CF6`) | none ("avatar update success", CharMake `0x42AC70`); the client then reads its player again | live: Confirm sent it with the new hair colour in `avatar1`; after a restart the player record brought it back | confirmed; we answer `0x2CF6` when `avatar1` isn't binary |
| `0x2CF0` player search (below) | `0x2CF7` (`0x2CF8`) | `result_number` u32: the total, the client shows pages of 100 (`/100 + 1`); items = players `{player_id, pname, dlv}` | InfoWindow `0x458410` (`0x2CF7` case), item reader `0x475A20`; `RESPONSE_PLAYER_SEARCH_SUCCESS` `ChatWindow.cpp L651` | confirmed (fields); paging and name matching **guess** |
| `0x2724` heartbeat | none | none | builders `0x4F17DF` (every 30 s) and `0x4F45A5` (after 120 s); no reply is checked | confirmed |

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
  event). The country matches everyone: the client never tells the server its country, so entries carry none (Player
  Information then shows the first flag). Paging is a **guess**: skip `base_point`, return at most `front_num`.
  Live: with Kaiba hosting, the waiting tab listed only him with the hosting icon.

### Chat (ChatWindow dispatcher `0x435950`)

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x2135` room list | `0x2458` (`0x2459`) | top `lang`; items = rooms `{id, name, capacity, num, region, status, type}` (read by `0x434D20`); `id` ≥ 1 | "chat room list success!" `ChatWindow.cpp L339`; the room chooser lists them with `[num/capacity]` | confirmed (`region`/`status`/`type` **guess** 0) |
| `0x2136` enter `{room_id}` | `0x245A {room_id}` (`0x245B {error_code}`) | the client ignores `room_id` < 1 | "In>>Lobby" printed, tab renamed | confirmed |
| `0x2137` leave | `0x245C` | none | "chat room exit" | shape |
| `0x2134` send | `0x2456` (`0x2457`) | fields sit in **one item (tag 10)**: `chat_str`, `chat_type`, and `to_id` or `to_name`. Room chat = `chat_type` 1 with `to_id` = the room id; whisper = `to_name` | captured packets; builders `0x4F3E00` (to_id) and `0x4F4030` (to_name) | confirmed (room); whisper shape |
| (server push) | `0x2455` | items `{from_id, pname, chat_type, chat_str}`; the handler also checks `friend_id`/`member_id` | two clients see each other's lines; the client shows nothing on `0x2456`, so the sender needs the echo | confirmed |

### Friends and the ignore list (community agent `0x4A4EE0`)

PlayerInfoWindow (`0x4B6060`) looks the player up in the friend list (dataset slot 6, items by `friend_id`) and the
ignore list (slot 7, by `black_id`), matching the player's id as text (`0x531EB0`), and offers "Send Friend Request" /
"Add to Ignore List" (or their opposites). Its buttons start the community agent (`0x4B9D00` → `0x4A4EE0`), whose
mode picks a state: 0 send a friend request (`0x4A50D0`, then `0x4A5390` waits for the answer), 1 answer one
(`0x4A5B70`), 2 remove a friend (`0x4A5880`), 3 / 4 add to / remove from the ignore list (`0x4A6290`), 5 and up teams.
The global dispatcher keeps both lists: `0x251C` and `0x252A` replace them (the left panel's lists redraw, InfoWindow
`0x458410`), and the replies below add or remove one entry by themselves. Errors carry `error_code` (`0x43D4F0`).

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x24B9 {to_id}` friend request (top level) | `0x251E` (`0x251F {error_code}`: 106 "This player cannot receive another request.", 110 "The Player List is full.", 112 " The opponent has left the lobby.", else "A transmission error has occurred.") | none | `0x4A50D0`, switch `0x4A522E`; live packet `{request 0x24B9, to_id 1002}` | confirmed (request); replies code |
| (push to the other player) | `0x24BA {from_id, from_name}` | kept in slot 24, sets flag `0x100`: the bell's notification, whose answer runs mode 1 (`0x450460` case 5) with this message: a dialog naming `from_name` | dispatcher `0x4F54D7`, `0x4A5B70` | code |
| `0x24BB {to_id: from_id, answer}` (2 accept, 1 decline) | `0x2522 {answer, friend_id, friend_name}` (`0x2523 {error_code}`) | `answer` 2 adds `{friend_id, friend_name, login_flag 1}` to the list (dispatcher `0x4F55AA`) and the agent says so; else the flag `0x100` clears | `0x4A5F5D`, `0x4A5B70` state 1 | code |
| (push to the requester) | `0x24BC {answer, friend_id, friend_name}` | the same list update; the waiting agent (`0x4A5390`) shows accepted or declined | `0x4F55AA`, `0x4A5390` | code |
| `0x24BD` cancel the request (bare; the waiting dialog's button) | `0x2524` (`0x2525 {error_code}`) | the other player gets `0x24BE` (their answer dialog says the request was withdrawn; flag `0x100` clears); `0x24C0` clears it too, as an error | `0x4A578E`, `0x4A5B70` | code |
| `0x24BF {to_id}` remove a friend | `0x2528 {to_id}` (`0x2529`) | the dispatcher drops the entry whose `friend_id` matches `to_id` | `0x4A5A60`, `0x4F57CE` | code |
| `0x24C2 {to_id}` / `0x24C3 {to_id}` add to / remove from the ignore list | `0x252C {to_id, to_name}` / `0x252E {to_id}` (`0x252D {error_code}`) | `0x252C` adds `{black_id, black_name, login_flag}`, `0x252E` drops it | `0x4A6290`, `0x4F58DD`, `0x4F5A54` | code |
| (login push) | `0x251C` friends `{friend_id, friend_name, login_flag}`, `0x252A` ignored `{black_id, black_name, login_flag}` | `login_flag` 1 = online (the dispatcher's own entries use 1); both lists are read unchecked when listing a player, so they are always sent | `0x475A20` modes 2 and 3, `0x45E734` | shape |
| `0x24B8` / `0x24C1` (bare) | `0x251C` / `0x252A` | the left panel's friend and ignore list views ask for their list (`0x4F2FB0`, `0x4F3090`, sent by `0x459690` / `0x459760`) | tracer: `friend_id`, `friend_name`, `login_flag` found (`0x475A20`) | confirmed |

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
`friends.json` next to the accounts. A request needs the other player online and with no request waiting (106 else,
112 when offline); answering, cancelling and removing reach the other player as the pushes above, and whenever a player
logs in or out their online friends get their friend list again (`0x251C`) with the new `login_flag`. Live: Yugi's
Send Friend Request showed "Sending Friend Request to Kaiba...", Kaiba's bar showed a friends icon whose dialog read
"You have received a Friend Request from Yugi. Add to Friend List?"; Yes gave both "… has been added to your Friend
List."; after a server restart Kaiba still had the friend icon in Yugi's search, "Display online friends." listed him
but not Joey, and Player Information offered "Remove from Friend List", whose Yes sent `0x24BF` and emptied the tab.

### D-Mail (MailWindow `0x4AC6B0`, `dmail_main.swf`)

The client keeps its mail in its own files (`players\<pname>\mail\<box>\<mail_id>.y3m`) and reads and deletes it
there; the server only holds a mail until its recipient's window fetches it. All fields are top level.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x2583` (bare; the window opening and the main screen's startup step `0x4730B0`) | `0x2588 {count}` (`0x2589`) | the mail waiting | live | confirmed |
| `0x2582 {count}` (when the count isn't 0) | `0x2586` items (`0x2587 {error_code}`) | `mail_id`, `mail_type` (0 a player, 3 a team), `from_id`, `from_name`, `to_id`, `to_name`, `title`, `body`, `send_date` "YYYY-MM-DD HH:MM:SS", `team_id`, `team_name` (`0x4AD3C0`); processing it clears the bell's mail flag | live | confirmed |
| `0x2580 {mail_type, title, body, to_id}` (Send D-mail, `0x4AE8E0`; subject 16 characters, body 255) | `0x2584 {mail_id}` (`0x2585 {error_code}`: 0 and 201 silent, 300 "This Player is currently offline.", 301 "There are no Players with this name.", 700 a transmission error) | the recipient is picked from the friend list (slot 6), offline friends too; team mail (type 3) has no `to_id` and carries `team_id`, `team_name`: every other member gets it | live; team mail by test | confirmed |
| (push) `0x2581` (bare) | — | a mail came: flag `0x1000`, the mail icon on the bell, which opens the window | live | confirmed |

The server keeps waiting mail in `data/mail.json` with the last mail id, so ids go on across restarts (the client
files mail by id), and drops a mail once fetched. Live: Yugi wrote to Kaiba from Write Mail (the friends button listed
Kaiba); Send D-mail got `0x2584 {mail_id 1}`, Kaiba's bar showed the mail icon, and his D-Mail window listed "Yugi /
Rematch? / 2026-11-07" with the body.

### Teams (the community agent's modes 5–12, Team window `0x4BBEC0`, `main_teaminfo.swf`)

The global dispatcher keeps the team info `0x2B13` whole in slot 17 (non-null = in a team: the chat's Team tab,
`0x436CA0`; Send to Team in D-Mail) and the members `0x2B17` in slot 9 (the player's own row gives their authority,
front `+0xE4`). After every team push (`0x2B07`–`0x2B0B`) and after forming or joining, the agent asks for both again
(`0x4A9100`). Authority: 0 member, 1 subleader, 2 leader (`0x4B6510`; Player Information shows Invite to Team to a
subleader or leader for a player without a team, Remove from Team for a lower authority, the authority buttons to the
leader). All request fields are top level; some ids come as text (live: `0x2B00`'s `to_id` "1001", `team_id` "1").

| Request | Reply (error: the next id, `{error_code}`) | Pushes | Evidence | Status |
|---|---|---|---|---|
| `0x2AFB` (bare; the Team window, the main screen's startup) | `0x2B13 {team_id, team_name, level, leader_name, created, season_score, score}`, or `0x2B14 {error_code 404}`, which clears slot 17 (`0x4F5CCA`): no team. A bare `0x2B13` (ours until 2026-11-08) made an empty team: the chat's Team tab showed for everyone | — | live | confirmed; `level` 1 (above 1 opens team titles and protectors), scores 0 (**guess**) |
| `0x2AFD` (bare) | `0x2B17` items `{member_id, member_name, login_flag, authority}`, the player's own row among them (without it the Team window stays blank), or `0x2B18` without a team (changes nothing) | — | live | confirmed |
| Form Team `0x2AF9 {team_name}` (`0x4A6490`; needs `premium`) | `0x2B0F {team_name}` "You have formed the team" (`0x2B10`: 409 the name is taken, 808 bad characters) | — | live: "You have formed the team "Millennium"." | confirmed |
| Invite `0x2AFE {to_id}` (`0x4A6D20`) | `0x2B19`, "Inviting %s to the team" waits (`0x2B1A`: 106 "This player cannot receive another request." for a player in a team or invited; else "A transmission error has occurred.") | `0x2AFF {from_id, from_name, team_id, team_name}` to the player: their bell, "%s has invited you to the team "%s". Accept?" | live | confirmed |
| Cancel the invitation `0x2B02` (bare) | `0x2B1F` | `0x2B03` to the player (the invitation was cancelled) | code | shape |
| Answer `0x2B00 {to_id: the inviter, team_id, answer: 2 yes, 1 no}` (`0x4A76F0`) | `0x2B1D` | `0x2B01 {answer}` to the inviter ("%s has joined the team "%s"." for 2, else declined), `0x2B08 {member_name}` to the members | live: Yugi saw "Kaiba has joined the team "Millennium"." | confirmed |
| Disband `0x2AFA {player_id}` (the leader, `0x4A6880`) | `0x2B11` | `0x2B07 {team_name}` to the others (clears their slots 17 and 9) | test | shape |
| Remove from Team `0x2B05 {to_id}` (`0x4A7F50`) | `0x2B25` | `0x2B0A {admin_name, member_name}` to everyone, the one removed too (its text has a third `%s` the client fills with nothing) | test | shape |
| Leave `0x2B04` (bare, not the leader, `0x4A8230`) | `0x2B23` | `0x2B09 {member_name}` to the others | test | shape |
| Authority `0x2B06 {to_id, authority}` (the leader, `0x4A84E0`): 1 subleader, 0 back to member, 2 the leadership | `0x2B27` | `0x2B0B {authority, to_name}` to everyone (dropped by a client without slot 17) | live: "Set Kaiba as subleader?", Kaiba's bell | confirmed; the old leader becomes a member on a hand-over (**guess**) |

Elsewhere: the player's record carries `team_level` (1 in a team, below 1 none), `team_name`, `team_authority` (the
icon frame, `0x46C1D0`); search entries carry `team_id` (0 none), `team_name`, `team_authority` (Player Information,
`0x4B6AF0`/`0x4B7350`); the search's team tab (`search_flag 0x20`) lists the player's team; chat type 3 goes to the
team (`to_id` = slot 17's `team_id`, `0x439D20`); D-Mail type 3 goes to every other member with the team's
`team_id`/`team_name`; Create Duel's Opponent's Limits → Team takes requests only from the host's team. Teams are kept
in `data/teams.json`. Live: Yugi formed Millennium in the Team window (Lv1, founded, leader Yugi), invited Kaiba from
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

| Request | Reply (error: next id, `{error_code}`) | Pushes | Evidence | Status |
|---|---|---|---|---|
| `0x2BC0 {to_id}` | `0x2BD7 {pname}` "Sending Trade Request to %s..." (`0x2BD8`: 106 "This player cannot receive another request.") | `0x2BC1 {from_id, from_name}` to the player: "You have received a Trade Request from %s. Accept?" | live | confirmed |
| cancel `0x2BC4` | `0x2BDD` | `0x2BC5` to the player | research | shape |
| answer `0x2BC2 {to_id (text, live), answer: 2 yes, 1 no}` | `0x2BDB` (OFFER_REPLY_SUCCESS, in the scene) | `0x2BC3 {answer}` to the inviter (OFFER_NOTICE): 2 opens its scene | live | confirmed |
| ready `0x2BD5` (each scene on opening) | `0x2BED` | once both: `0x2BD6` (NOTICE_START) to both | live | confirmed |
| offer `0x2BC6 {main_list}` (the cards dragged from the trunk, big-endian u16 rows) | `0x2BE1` "card list send ok" (`0x2BE2`: 502 offer player illegal, 503 trade error, 504 card number illegal, 505 status, 700 server error) | `0x2BCF {main_list}` to the partner (NOTICE_OTHER, read unchecked) | live: Tainted Wisdom `main_list 0011` | confirmed |
| `0x2BC7` (the partner's scene has the offer) | `0x2BE3` | `0x2BCE` (NOTICE_OWN) and `0x2BD0` (NOTICE_CHANGE_ARRIVE) to the offerer | live | confirmed |
| Ready to Trade. `0x2BC8 {status: 1 on, 0 off}` | `0x2BE5` | `0x2BD1 {status}` to the partner; both on: `0x2BD2` (ALLREADY) to both, which shows "Execute Trade?" | live | confirmed |
| "Execute Trade?" `0x2BC9 {answer: 2 yes, 1 no}` (`0x427640`) | `0x2BE7` | once both answered: `0x2BCA {error_code, own_status, other_status, trade_result}` to both (LAST_CONFIRM_NOTICE; `trade_result` 1 "Trade Completed"), then a new trunk `9000` (no request asks for one there) | live: Yes sent 2; "Trade Completed", the cards changed sides | confirmed; `trade_result` 0 (a No) left the scene waiting (live, with our first guess of the Yes value), what the original sent then is open |
| leave `0x2BCB` | `0x2BE9` | `0x2BCD` to the partner: "The trade partner has cancelled." (also when a player goes offline) | research, test | shape |

The server checks each offer against what the player owns (their starter deck's cards, the ones bought or got, less
the ones given away) and moves both offers when both said yes, in the accounts (`data/accounts.json`: `cards` and
`given` per player). A session refreshes its player's collection from there on each trunk request, so the partner's
next shop visit shows it too. Live: Yugi's Tainted Wisdom for Kaiba's Windstorm of Etaqua.

### Manner ratings (`dlg_manner.swf`, DuelResult `0x512A40`, dialog `0x4C2A10`/`0x4C3F20`)

| Message | Fields | Evidence | Status |
|---|---|---|---|
| push `0x2CEC` (after the duel's `0x2009`, to each player of a duel between players, at the match's end) | `eval_id` | the global dispatcher keeps it in slot 22 and sets front flag `0x40` (`0x4F6024`); DuelResult state 0 (`0x512A40`) sees the flag once the duel's result is known (MainGamen `+0x820`), clears it and opens the dialog "Please evaluate NAME." with the **opponent's name** (the client's own). The `0x2CEC` arms of CardRental (`0x41D164`) and CardTrade (`0x425114`) only set a flag: a trade never opens the dialog, so the client has no trade ratings | live: both players got the dialog | confirmed |
| `0x2CED {eval_id, eval_data}` | `eval_data` is the chosen button's value from the table at `0x790004`: -10, -5, 0, 5, 10 (sent as a u32, so -10 arrives as 4294967286); top row (five stars) +10, bottom row (one star) -10 | `0x4C3F20`; live | confirmed |
| reply `0x2CF1` (error `0x2CF2 {error_code}`) | none | DuelResult state 2 (`0x512A40`) waits for an id above `0x2CF0` and below `0x2CF3`; the global dispatcher ignores both | live: the dialog closed, no "no response" | confirmed (`error_code` 0 **guess**) |

The server keeps the evaluations it asked for (`eval_id` -> rater, player rated; ours count up from 1): an answer counts once, from
the rater, with one of the five values. Each player's ratings (count and sum) are in `data/accounts.json`
(`eval_num`, `eval_total`). `evallevel` (0-4) is the average mapped onto the buttons' scale, (average + 10) / 5 rounded
(a **guess**: the original's thresholds are unknown), 2 without ratings. It is in the search entries (Player
Information's face icon: live, a blue smile for Kaiba after a +10, a dead red face for Yugi after a -10 and the star
icons between) and, with `evalnum` and `evaltotal` (the sum, 0 when negative: **guess**, only the level is read), in the
player records.

### Main Menu screens (MainGamen's menu `0x46FA60`)

| Screen | Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|---|
| Rental Card List (RentalRecord `0x481A30`, `rentalrecord.swf`) | `0x2C34` (bare), then `0x2C35 {rental_kind 0, base_id}` and `0x2C35 {rental_kind 1}` | `0x2D6C` (`0x2D6D`), `0x2D6E` (`0x2D6F`) twice | the screen is built only after all three (`0x482250`); `0x2D6C` also replaces slot 8. Borrowed items `{from_name, from_type, rent_end, deck_type, confirm_to}`, lent `{to_name, rent_end, deck_type, confirm_from}`, decks `main`/`extra`/`side`; empty lists are safe. Without the replies the screen stays black | live | confirmed (empty) |
| My Profile (`profile.swf`) | `0x1F49` (bare) | `0x2260` | the item whose `pindex` is the player's; ProfileGet (`0x4705D0`) reads `pname`, `title`, `dlv`, `tp`, `n_win`, `n_lose`, `dscore`, `n_duelall`, `turns`, `n_a_win/lose/draw`, `n_nogame`, `isurrender`, `usurrender`, `lpleft`, `n_m_win/lose`, `n_s_win/lose`, `sidedeck`, `sacrifice1..3`, `fusion`, `synchro`, `gravesummon`, `spsummon`, `itrap`, `utrap`, `imagic`, `umagic`, `iefmon`, `uefmon`, `reversalwin`, `reversallose` (absent = 0). We send `n_win`, `n_lose`, `n_draw`, `n_duelall` from the duel records against players (also in the search entries, which Player Information's "Number of Wins" reads) | live: Wins 4, Losses 2 | confirmed; NPC duels not counted (**guess**) |
| Duel Log (DuelLogWindow `0x499750`, `duel_log.swf`) | `0x1793` (bare) on opening; Server Duel Logs `0x19C8 {pname, offset}` | `0x177B` (`0x177C`), `0x19C9` (`0x1790`) | `0x177B` is the championship list, items `{year, season, event_id}`; the server log tab queues behind it. `0x19C9` items `{end_time (epoch), match_type, dlv_1, dlv_2, pname_1, pname_2, dlog_comment, duel_id, filename, dlogURL}` (`0x49ADF0`) would be replays to download; we keep none. **Latest Duel Logs** lists the `*.dlog` files of `players\<name>\duellogs\myduels\` (`0x49A950`: each loaded by `0x524CA0`, whose header fields are the `0x19C9` item's; the row prints `end_time` through `_localtime32`, so the client's own time zone, `dlv_1`/`dlv_2` as `Lv%d`, `pname_1 V.S. pname_2`); Duel Log Collection is `favorites\`. Those files are **downloaded by the client after each duel**, see "Duel log files" | live: both lists empty without logs; with them Latest Duel Logs lists the duel | confirmed |
| BOOSTERPASS (BpInputWindow, `boosterpass.swf`) | `0x20D0 {dpcode}` (a 16-character code) | `0x23F0 {dpauth_result, dpPoints, dpGetPoints, premium, premium_expire}`, no error reply | 0 adds `dpGetPoints` and shows the points; anything else "Incorrect BOOSTERPASS CODE.". The global dispatcher keeps `premium` from it (`0x4F5349`), so it goes with 1 | live: "Incorrect BOOSTERPASS CODE." | confirmed; every code refused (Konami's are gone) |
| Leaderboard | none | — | ShellExecute on Konami's dead ranking page with the login item's `country` | code | nothing to serve |
| DUEL OPTIONS | none | — | local options file | code | nothing to serve |

### Shop (`shopmain.cpp`, handler `0x48AA60`)

The map's top-left facility. Opening it re-requests the trunk (`0x1FA4`), and each reply below asks for the next, in
this order; the last one clears `isLoading`. Success and error replies are pairs (`0x1C20`/`0x1C21`, …); none of
them has a global case. Every list case checks its item count (`0x410D90`) before reading. The shop's BP
(BOOSTERPASS Points, `user_bp`) is the account's `dpoint` from the login replies.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x1B66` text (bare, sent from the `9000` case, `0x48BEDD`) | `0x1C20` (`0x1C21`) | `text`: the shopkeeper's line. The case also reads `evalnum`, `evaltotal`, `evallevel`, but from the player record; they pick a stock line (`$33003`–`$33007`) when `text` is empty | "shop text success" L286 | confirmed |
| `0x1B67` items (bare, `0x48ACC5`) | `0x1C22` (`0x1C23`) | items `{item_type, item_id, item_cost, item_max, sale_id, sale, item_trade, modified, rel_time}` (`0x485E50`; dates are text); `item_type` < 3 go on the shelves, others are kept only when `item_param` isn't owned (a bit set at the player data's `+0x68`). The kept list (`+0xD4`, 0xD8 bytes each) fills the Purchase screen (`0x48E0A0`): kinds 0 and 3 call the Flash `AddPack(id, name, comment, image, date, cost, max, trade)` (`0x48C790`) with the name, description, image and date from the client's own pack file `item_id`, 1 and 4 `AddStruct`, 6 `AddProtector`, 10 and 11 `AddEventPack` | tracer: all nine found; live: three packs on the Booster Packs shelf with their own art, then "Pack 1", "A rearranged pack comprising of season 1 Cards.", 100 BP | confirmed |
| `0x1B6F` (bare, `0x48AE5F`) | `0x1C26` (`0x1C27`) | items `{item_id, item_cost, item_trade, item_type, modified, rel_time, sale_id}` | "shop exchange item success" L360 | confirmed (empty) |
| `0x1B6B` (bare, `0x48B3B8`) | `0x1C32` (`0x1C33`) | Purchase Protectors: items like the packs' (`0x485E50`) with `item_type` 6 (AddProtector `0x48D520`) and `item_id` the protector's pbin id (2305–2512, the catalogue's kind 11; the client leaves out the ones the player's pbin owns, images `data/protector/p%04d.png`) | "shop protector item success" L409; live: ten protectors a page with their art | confirmed; 200 BP each (**guess**) |
| `0x1B6D` (bare, `0x48B4C3`) | `0x1C2E` (`0x1C2F`) | items; toggles `_root.spac_btn_disable` (Special Packs) | "shop event item success" case; the client moves on to `0x1B72` | shape (empty) |
| `0x1B72` (bare, `0x48B64D`) | `0x1C36` | `perk_num` u32, compared with 500 to show `_root.expack_del` | tracer: found at `0x48B6CD` | shape (value **guess** 0) |
| `0x2C40` rental terms (bare, `0x48B777`) | `0x2D75` (`0x2D76`) | `rental_cardnum`, `rental_point`, `rental_term` u32 | read at `0x48C20E`; the case then sends `0x1B69` | shape (values **guess** 0) |
| `0x1B69` (bare, `0x48C391`) | `0x1C2A` (`0x1C2B`) | Purchase Avatar Items: items with `item_type` above 2 and `item_param` the item's pbin id, which the client leaves out once owned, then the packs' fields; `0x496A50` sorts them into four tabs by id (tables `0x78D2E8`, `0x7A83B0`) and by gender, skipping unknown ids. We sell the catalogue's parts to acquire (kinds 1 and 2, sub 0: 178 male, 148 female); clears `isLoading` | "shop avatar item success"; live: the Male and Female lists by tab, a preview on the avatar | confirmed; 100 BP each (**guess**) |
| `0x1B68` buy `{sale_id, item_num, end_time}` (live: `end_time` the text `""`) | `0x1C24` (`0x1C25`) | `dpoint`: the BP left, shown as `user_bp`; then the client asks for presents (`0x2775`, builder `0x4F3A00`, through `0x47C4A0`) | "shop buy succes"; live: the purchase dialog's "BP After Purchase", then the pack opening | confirmed |
| `0x1B6A` exchange `{sale_id, item_id, kaban}` | `0x1C28` (`0x1C29`) | | "shop exchange succes" | open |

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
login replies and each purchase: the client sees the new total at its next login or in the next `0x1C24`.

Each pack file's card list is ascending runs: 44, 18, 9, 9 for an 80-card pack, the OCG split of Normal, Rare, Super
and Ultra Rares. The server reads the runs as rarity tiers (**guess**) and puts four cards of the first tier and one of
a higher one in a pack (**guess**: the original's pack size and odds are unknown); a pack of one run gives five of it.

### Events (TournamentScene, UserEventScene)

The map's Official Event tile (`MenuSelect` 21) opens TournamentScene, Championship (20) a 64-player tournament map
that sends nothing, and User event (22) UserEventScene. With no decks, Official Event and Championship stop at "There
are no valid Decks available." and OK returns to the map.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x177A` event list (bare, `0x4D0BB0`) | `0x177B` (`0x177C`, dialog `0x4F7940`) | top `result_number`; items `{event_type, status, event_id}`. TournamentScene looks for `event_type` 10 with `status` 12; none means its no-event path | `0x4D0BB0`; the Stadium opens | shape (empty) |
| `0x1789` event search `{search_flag, pname, status, pack_type, base_point, front_num 30, back_num}` (`0x4D9E37`) | `0x177B` (`0x177C`) | as above; the list case `0x4D66A0` reads `result_number` (tracer) and `premium` from another dataset | captured packet; User Event lists nothing, page 1/1 | shape (empty) |
| `0x339A` own user event (bare, `0x4D9C2B`) | `0x33FE` (`0x33FF`) | `remain`, `event_id`: the scene keeps `event_id` only while `remain` ≠ 0 (its own event: relation 0 in the list, Create off) | `0x4D9BB0`; live | confirmed |

**User events** (Satellite Plaza; Create needs `premium`). The list `0x1789` → `0x177B {result_number}` has a row per
open or running event: `event_id`, `event_name`, `comment`, `status` (12 open, 13 running, 14 finished), `join_flag`
(non-zero crosses the row out and the details offer only Close: we send 1 only for a Participation Limit above OFF,
whose value is 1), `password1`, `prize_type`, `entry_max`, `entry_num`, `match_type`, `regulation_id`, the levels,
`eval_level`, `lang`, the duel settings and the host's `player_id`/`pname`. The client works out `user_relation`
itself (`0x4D6970`: 0 own, 1 open, 2 restricted, 3 running, 4 finished; the lobby 5), which with `numMenuButton`
(`user_event_detail.swf`: bit 0 Close, 1 Participate or Start, 2 Observe or Cancel) picks the buttons.

| Request | Reply (error: next id) | Fields | Evidence | Status |
|---|---|---|---|---|
| Create `0x339B` (bare) | `0x3400 {create_status}` (`0x3401`) | 0 opens the form; bits 4 premium needed, 8 already in another event, 2 wait, 1 full | live | confirmed |
| the form `0x3390 {event_name, comment, entry_max, prize_type, level_min, level_max, eval_level, player_limit, password1, lang, match_type, duel_surrender, duel_watch, duel_time_turn, duel_time_total, duel_lp_start, regulation_id, join_max}` (top level, live) | `0x33F4 {event_id}` | then `0x1784 {event_id}` → `0x1785 {join_id, event_id}` (the global dispatcher keeps `event_id`, which the lobby asks with: 0 until we echoed it; we echo only a user event's, numbered from 100: echoing Auto Duel's 2 made the clients queue again by themselves after each match, live) and the entry `0x3397 {event_id}` → `0x33FA` | live | confirmed |
| Participate: `0x1784`, `0x3397`; leave `0x178E {event_id}` | `0x33FA`, `0x178F` | the others get `0x3399 {entry_name, entry_type: 1 joined, else left}` | live (join), test | confirmed |
| lobby `0x177F {event_id}` | `0x1780` item[0] | `official` 0, `player_id` the host (compared with the client's own, `0x4D0B56`: lobby `+0x2E0` 1 host, 2 entrant, 3 observer), and the row's fields, which the Event Menu (the menu bar's duel button) shows | live | confirmed |
| `0x32CA {event_id, start_index}`, `0x32CC`, `0x32CD`, `0x32CE`, `0x32C9`, `0x32D5`, `0x32CB {block_index}` | `0x32FD {total_num, items {player_id}}`, `0x3301 {timer}`, `0x3303 {alive_num, entry_num, watch_num}`, `0x3305 {phase_block1–3, remain}`, `0x32FB {event_id, status, phase, round, total_round, winner, winner_name, second, second_name, tournament_id, championship_id, year, season}`, `0x3309 {map_data}`, `0x32FF` | `0x32FB`'s `status` is the lobby's state (`+0x2F8`; above 1 the event has begun and the host's Start/Cancel go, `0x4CE670`): we send 0 open, 2 running, 3 finished (**guess** beyond 0–1). `map_data` is a flag byte per bracket node, 0x49 of them (`0x4D0F40` copies them unchecked: without it the lobby crashed, NULL at `0x4D10A9`). `0x32FF`'s block data (the names in the bracket) is open | live | confirmed; the bracket's names **open** |
| Start `0x3395 {event_id}` (the host) | `0x33F8` | everyone gets `0x3396` ("The event has started!"), then each pair the match push `0x1978 {vsPlayerData, duelData {duel_id, match_type, regulation_id, lp_start, surrender, time_turn, time_total}}`, which the lobby takes like the matching agent (`0x4CC60C`): Select Deck, janken, duel | live | confirmed |
| Cancel `0x3393 {event_id}` (the host) | `0x33F6` | the others get `0x3394` ("Event cancelled.", the lobby closes) | test | shape |

The server runs the event as a knockout: the first round pairs the entrants in order (an odd one goes through), each
duel's winner (the table's report) stays in, and when a round's duels are over the next one is paired, until one
player is left and the event is finished (it leaves the list; the host may create another). A player who left
forfeits. Events live in memory only. Live: Yugi created "Friday Cup", Kaiba found it and pressed Participate, the
host's Event Menu offered Start, Start opened Select Deck for both, Yugi won the duel, and Next took both back to the
lobby.

### Decks (deck editor `DeckEditTest`, handler `0x43FC60`)

The client keeps decks as **local files**: `<My Documents>\Yu-Gi-Oh! ONLINE 3\players\<pname>\decks\<deck_name>.y3d`
(saved by `0x52EE60`, loaded by `0x52EB70`). Opening Edit Deck asks for the deck list and saves every listed deck the
player doesn't have yet (by name; `0x43E550` checks). The NPC duel (`0x4BD880`) then needs one local deck that
validates (`0x51AB00` → `0x51AA80`: 5 no deck, 2 sizes, 3 cards not owned, 4 regulation, 0 valid) against the
collection built from the trunk (`0x4F6340`) and the login regulation (`0x2201 regulation_id`, kept at connection
`+0xC8`). Sizes (`0x51AC50`): main 40–60, extra and side under 16. Regulation (`0x51A9A0`, `0x51A930`): a card may
appear as often as the index of the list naming it (`card0` 0 … `card3` 3), else `addition_card`; the list at the
default's index is skipped; cards 5280, 5434 and 6276 are never allowed.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x1F4E` deck list (bare, `0x440C43`; also re-sent after save and delete) | `0x22C8` (`0x22C9`) | items **tag 6**: `deck_id` (int), `deck_name`, `discription` (string), `card_protector` (int), `main`, `extra`, `side` (binary u16 BE CARD_Prop rows, read by `0x414410`); at most 10 are used | tracer: all found; "Starter Deck.y3d" written; the NPC duel opens | confirmed |
| `0x1F50` delete `{delete_type, deck_name}` (`0x4449A9`, after deleting the local file `0x52F0C0`) | `0x22CC` (`0x22CD` shows an error) | none | "deck delete succes"; live: the deck leaves the Load Deck list | confirmed |
| `0x1F4F` update `{duel_id, event_id, update_type}` + the deck (tag 6) | `0x22CA` (`0x22CB`) | none | Saving in the editor only writes the local file (`0x4442E0` → `0x4159B0` → `0x52EE60`) and `cursor_deck_name`, though the editor has a "deck add or update succes" case for `0x22CA`. The request comes from a duel's Select Deck (DuelManager `0x50ECF0`, see below); live: the duel scene loads ; Card Swap between a match's duels (`0x50FDF0` state 3) sends `update_type` 2 with the swapped deck (MainGamen `+0x838`), and the server keeps it as the player's entry for the match | confirmed (live: update_type 2 after Card Swap); a deck-editor save is not stored |

Every player gets one deck, "Starter Deck": the first `deck_type` 1 preset (`data/recipe`, ids 1–6, 501, 502; they
look like the starter decks) that the login regulation allows. Preset 1 holds a card regulation 1 forbids (4966), so
it is preset 2 (**guess**: the original let a new player choose). The server lists it every time, so a deleted
starter deck comes back the next time Edit Deck opens. Structure decks a player bought are listed after it, under
their own names from the client's text table (`name_id`, e.g. "GADGET BOX"), so they reach the editor and Select
Deck ready to play; a deck bought twice is listed once.

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

| Step | Request | Reply (error) | Fields read | Evidence | Status |
|---|---|---|---|---|---|
| `0x4A1090` | `0x1784 {event_id}` (1 for an NPC duel) | `0x1785` (`0x1786`) | `join_id` (agent `+0x58`); the whole reply is copied to MainGamen `+0x868`; the global dispatcher (`0x4F4AE7`) stores `event_id`, `join_id`, `server_id`, `event_score` (ints) | live: client then sends `0x17AC`, trace "event join offer notice" | confirmed; `join_id` = a server-wide counter (**guess**) |
| `0x4A1790` | `0x17AC {event_id, join_id}` + item (tag 4) `{cpu_id, duel_server_id, match_type, regulation_id, surrender, time_total, time_turn}`; live: `cpu_id` 124 (the NPC id), `match_type` 4, `surrender` 1, the rest 0 | `0x17AD` (`0x17AE`, dialog `0x4F7940`) | `duel_id` (agent `+0x5C`); the reply is copied to MainGamen `+0x88C` with the agent's `regulation_id`; `country` is read from the player's own data | live: the scene opens DuelManager's Select Deck | confirmed; `duel_id` = a server-wide counter (**guess**) |

Mode 1 then asks for a password (`duel_password_flag` from the agent's record), mode 2 goes to `0x4A1380`, mode 0
(the DUEL panel) waits for a match in `0x4A1D10`. Mode 4 (watching a duel: PlayerInfoWindow's Watch Duel button for
a player whose `duel_status` is duelling, shown only when the entry's `duel_watch` is not 0, `0x4B6510`; the server
sends `duel_watch` 1 and `duel_match_type` for players in a matched duel, the host's own for a hosted one) starts at `0x4A3340`: `0x1838 {duel_id}` → `0x1839` (`0x183A` = error, back
to the map through `0x4A35F0`), which sets MainGamen `+0x8B8` = 5 and `+0x8BC` = 1 (watching) and is read twice:

| Reader | Fields | Status |
|---|---|---|
| `0x4A3340` | `host` (string), `port` (int): DuelServerConnection `+0x78`, `+0x94` | code |
| `0x4A3650` | items found by `name` (`0x531EB0`): `avatar_1` and `avatar_2`, the two players' records (loaded into MainGamen's two PlayerData, as `vsPlayerData` is); top-level `card_protector_1`, `card_protector_2` (int); item `duelData` {`event_type`, `official`, `duel_id`}, copied in as the duel's rule (`0x41BB10`); `official` 0 and `event_type` not 5 pick the field `0x14 + duel_id % 8` | code |

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

| Step | Request | Reply (error) | Fields read | Evidence | Status |
|---|---|---|---|---|---|
| Select Deck `0x50ECF0` | `0x1F4F {duel_id, event_id 0, update_type 1}` + the chosen deck (tag 6) | `0x22CA` (`0x22CB`, text `0x2919`) | none (`0x50EFC1` compares the id; the deck editor and the shop take `0x22CA` too) | live: the duel scene loads | confirmed |
| `0x50F030` state 2 | `0x17D4 {event_id, join_id, duel_id}`, once the avatar object (`AvatarViewer` `0x4DC320`; an NPC opponent is set up by its vfunc `0x68`) reports state 2 or −1; skipped when MainGamen `+0x8BC` is set | `0x17D5` (`0x17D6`, text `0x2919`) | none | live: sent right after the duel scene loaded, before any play | confirmed (meaning: "ready", **open**) |
| `0x50F690` state 1 | (none: waits for front flag `0x8000`) | push `0x22C4`, which the global dispatcher keeps in slot 20 and flags (`0x4F4F34`) | items found by their `name` field (`0x531EB0`): `duelData` {`host` string, `port` int}, and its sub-item `rule`, which replaces MainGamen `+0x88C` (`match_type` read from it). Host and port go to DuelServerConnection (`+0x78`, `+0x94`), which connects | tracer: `host`, `port`, `name`, `match_type` found; live: the duel connection opens | confirmed; `rule` = the item of the duel's `0x17AC` (**guess**: only `match_type` is read so far) |
| `0x50F690` state 2 | **duel connection**: `0x200A {player_id, autoSet}` (`autoSet` from the `duelOption` `auto_place`), unless MainGamen `+0x8BC` is set; then heartbeats `0x2724` every 30 s (`0x4F1730`) | no reply: the server starts the janken (see "Duel connection") | | live: `{autoSet 1, player_id 1002}` | confirmed |

### Duels between players (automatic matching)

The menu bar's duel button (`go_duel` in `main_menubar.swf`, enabled once the main screen unlocks) calls `GoDuel`
(`0x46FEB0`): with the matching agent idle it opens DuelMenu (`0x49D070`, component `0xFFFFD908`,
`duel_kind_menu.swf`); while the agent waits for a match it cancels the wait (`0x4A0BD0`). DuelMenu's callbacks
(`0x49D8F0`): `requestDataPlayers`, `requestDataAuto`, `decideAuto`, `requestDataDuel`, `decideDataDuel`, `cancelMenu`,
`openRegSelect`. Auto Duel offers Rule Format (Single, Match, Both) and Opponent's Level (Any, About the Same, Strong,
Very Strong), kept in the client's own option store as `autoMatching {match_type, level}` (like `duelOption`;
`match_type` 1, 3 or 6, see Match Mode under the result screen);
Create Duel (`duelMaking`: password, watch, lang_mask, detail, lp_start, time_turn, time_total, regulation, party…)
is below. Confirm starts the agent in mode 2, whose deck check (`0x4A0E90`) needs a local deck: a new
player has none until Edit Deck has opened once ("There are no valid Decks available.", `0x51AB00` → 5).

| Step | Request | Reply (error) | Fields read | Evidence | Status |
|---|---|---|---|---|---|
| DuelMenu `0x49D310` | `0x17FC` (bare) | `0x17FD` (`0x17FE`) | `duel_playing`, `duel_ready`, `duel_auto` (menu `+0x58`, `+0x5C`, `+0x60`), shown as "Duel in Progress", "Create Duel" and "Auto Duel" counts | live: the Duel Menu opens with the counts | confirmed; counts = players (**guess**) |
| `0x4A1090` | `0x1784 {event_id 2}` | `0x1785` | `join_id` | live | confirmed |
| `0x4A1380` | `0x1798 {event_id, join_id, match_type, level}` (the menu's choices) | `0x1799` (`0x179A`) | none | live: "Waiting for opponent." with Cancel | confirmed |
| `0x4A15D0` (Cancel) | `0x17A2 {event_id, join_id}` | `0x17A3` (`0x17A4`) | none | code | shape |
| `0x4A1D10` (waiting) | push `0x1978` | | goes to `0x4A2D30` at once | live | confirmed |
| `0x4A2D30` | | push `0x1978` (`0x17CC` fails) | item `vsPlayerData` (by `name`): loaded into the opponent's PlayerData (MainGamen `+0x408`) like the player list's records; item `duelData` (by `name`): copied into MainGamen `+0x88C` as the duel's rule, its `duel_id` kept | live: "LV1 Kaiba / Default / Starting Duel / Single Mode", then the duel starts as an NPC duel does (Select Deck with a 30 s timer and an Opponent chat tab, `0x17D4`, `0x22C4`, the duel connection) | confirmed |

The wait step also takes `0x17B6` (another player's offer: `playerInfo {name, player_id, pname, dlv, title}`; a
player on the black list is declined at once, else a dialog asks) and answers `0x17CA {event_id, join_id, duel_id,
answer}` (2 accept, 1 decline); accepting goes to `0x4A2D30`. That is how hosted duels (mode 1, `0x4A28F0`:
`0x17B6`–`0x17B8`, `0x17C0`, `0x17CA`) meet; see below.

**Hosted duels (Create Duel).** The Duel Menu's Create Duel (`decideDataDuel` `0x49EE90`) starts the agent in mode 0
with the `duelMaking` choices. Challengers find the host in player search and send a request from its Player
Information window (PlayerInfoWindow, `0x4B6060`/`0x4B6510`/`0x4B8730`/`0x4B8F30`, `decide` `0x4B9D00` → mode 1; its
Watch button → mode 4):

| Who | Request | Reply / push | Fields | Evidence | Status |
|---|---|---|---|---|---|
| host | `0x17AC {event_id 1, join_id}` + item (tag 4) | `0x17AD {duel_id}`, then the wait `0x4A1D10` ("Waiting for opponent.", Cancel) | live item: `{match_type 1 (Single), surrender 1, level 1 (Any), party 1 (limits off), password "", watch 1, lang_mask 127, lp_start 8000, time_turn 180, time_total 600, regulation_id 1, status 1, duel_server_id 0}`; an NPC duel's item has `cpu_id` and no `status` | live | confirmed |
| host | `0x17F2 {event_id, duel_id}` (Cancel, `0x4A2500`) | `0x17F3` (`0x17F4`) | none | live | confirmed |
| challenger | `0x2CF0 {search_flag 0x40000, player_id}` | `0x2CF7` | the host's entry: `duel_status` 1 (Duel button; 0x4B6060: 0 none, 1 and 2-with-`duel_id` challengeable, others duelling: Watch), `duel_id`, and `duel_match_type`, `duel_regulation_id`, `duel_watch`, `duel_party`, `duel_password_flag`, `duel_level`, `duel_lang_mask` (the client checks the level and language limits), `duel_surrender`, `duel_lp_start`, `duel_time_turn`, `duel_time_total` (the Duel Conditions panel) | live: "Duel Conditions" with the host's choices and Send Duel Request | confirmed |
| challenger | `0x17B6 {event_id, join_id, duel_id, password}` (`0x4A28F0`) | `0x17B7` ("Requesting Duel"), or `0x17B8 {error_code}`: 3000 "Another player is requesting a duel.", 3001 "Already Duelling", 4006 "Password is Incorrect.", 201 "Duel Request was refused.", others "A transmission error has occurred." | the server answers 201 to a challenger outside the host's Opponent's Limits (below) | live | confirmed |
| host | push `0x17B6 {duel_id}` + item `playerInfo` (the challenger's record) | | the wait step reads `player_id` (a player on the black list is declined at once), `pname`, `dlv`, `title` | live: "LV1 Kaiba / Default / You have a Duel Request! Will you accept?" | confirmed |
| host | `0x17CA {event_id, join_id, duel_id, answer}` (2 Yes, 1 No) | `0x17CB` (`0x17CC` fails, `0x4A2D30`) | | live | confirmed |
| challenger | push `0x17CA {answer}` | | 2: on to `0x4A2D30`; else refused | live | confirmed |
| challenger | `0x17C0 {event_id, join_id, duel_id}` (its Cancel) | `0x17C1` (**guess**); the host gets push `0x17C0`, closes the offer and waits again | | code | shape |

**Opponent's Limits** (Create Duel: OFF, Friend, Team) is the host's `party`: 1 off, 2 friends, 3 team. Challengers see
it as `duel_party` ("Opponent's Limits Friend" in Duel Conditions), and PlayerInfoWindow (`0x4B6510`, button 0) offers
Send Duel Request for 2 only when the host is in the viewer's friend list (`+0x49C`, from slot 6) and for 3 only to
a team member (`+0x49E`). The server checks the same on `0x17B6` (201 otherwise); there are no teams yet, so a
team-only duel takes nobody. Live: Yugi hosted with Friend; Joey's Player Information showed no Send Duel Request,
Kaiba's (his friend) did, and Kaiba's request reached Yugi.

On Yes both get the match push `0x1978` (the host's `duel_id`; `duelData` carries `match_type` and `lp_start`, which
becomes the duel's starting LP) a second later, and the duel runs as an automatic match's. Live: Kaiba found Yugi's
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

Every message is key/value (tag 4) with a `duelRequestName`. The client sends request `0x200C` (header built by
`0x49F990`), except its first message `0x200A`. The server's messages go through the pump `0x5233A0`, called every
tick by DuelServerConnection: it pops one message, ignores any without a `request` field, switches on
`duelRequestName` (`0x5201A0`) and returns the request id; a negative one counts as a lost connection (`0x4F1860`).
We send request `0x200D` (**guess**: no reader).

| Server `duelRequestName` | Fields | What the client does | Status |
|---|---|---|---|
| `0x10` | `BusyEffectID` | `0x520AB0` | shape |
| `0x11` engine command | `id`, `param1`…`param4` | queued as `{id, request, param1..param4}` (`0x523541`) on the engine (`0x82D2C0` `+0x54`); scenes pop one when their queued states (animations) are done (`0x522A50`, `0x4D3220`) | confirmed (live) |
| `0x12` | `comIDsize`, `helpIDsize`, `otherFlagSize`, `COMIDDATA` (raw, u32 BE) | command ids per card, help ids, flags | shape |
| `0x14` timers | `timerKind`, `start`, `count1`, `total1`, `count2`, `total2`, `other` and their `_max` | sets the engine's five clock slots (`0x520A00`: value, now, stopped, max) — 0 `count1`, 1 `count2`, 2 `total1`, 3 `total2`, 4 `other` — then `timerKind` 0 or 2 runs (`start` ≠ 0) or stops slots 0 and 2, 1 or 3 slots 1 and 3, 4 runs slot 4 (pump case `0x52396A`). See "Duel clocks" | confirmed (live below) |
| `0x15` duel setup | `FirstPlayer`, `PlayerNum`, `PlayerType001/002`, `MainDeckNum001/002`, `ExtraDeckNum001/002` | stored on the engine (`+0x1AC`…`+0x1E4`) | shape |

| Client `duelRequestName` | Fields | Sender |
|---|---|---|
| 1 | `size` … | `0x520C20` |
| 2 | `CommandID`, `index`, `player`, `position` (a card command) | `0x520FD0` |
| 3 | `phase` | `0x521180` |
| 4 | `DLGResult`, `debug` (a dialog answer; the card list's number of picks) | `0x521A70`, from each dialog's close |
| 5 | | `0x5212B0` |
| 6 | `SetIndex` (the card list's first pick) | `0x521BC0`, from the card list `0x4C74F0` |
| 7 | `index` (a card list position), `data` (its pick order, also kept in the high byte of the `0x3F1` entry) | `0x521CC0`, once per pick |
| 8 | `player` | `0x521980` |
| 9 / 0xA / 0xB | debug: `DebugID`; `wCardID`, `fFace`, `fTurn`; `id`, `type` (with `index`, `player`, `position`) | `0x5213C0`, `0x521560`, `0x5217C0` |
| 0xC | `selectHand` (janken: 0 rock, 1 scissors, 2 paper) | `0x4F1330` |
| 0xD | `choice` (1 first, 2 second) | `0x4F1430` |
| 0xE | none: Card Swap is done (state 4, once the front answered its `0x1F4F` with `0x22CA` or `0x22CB`); then the next janken (`0x50F690`), so the server answers 1500 | `0x50FDF0` (DuelManager) |
| 0xF | `player`, `autoSet` (duel options; live on entering the duel field: `{player 0, autoSet 1}`) | `0x515790`, `0x4A0390` |

**Janken** (who goes first; JankenScene `0x518400`, `duel_janken.swf`). Engine commands, all confirmed live:

| Command | Params | Scene |
|---|---|---|
| 1500 | none | `beginJanken`, then the hands (`0x518D80`); sent after `0x200A` |
| 1501 | param1 result (0 draw, 1 win, 2 lose), param2 the opponent's hand, param3 this player's hand | `resolveJanken(myHand, yourHand, result)` (`0x518F50`); after a draw the scene asks for a new hand by itself (`selectJanken` `0x518ED0`), so the server sends nothing more |
| 1502 | param1 ≠ 0: this player chooses | `selectFirst` (FIRST TO GO / SECOND TO GO), else `beginFirst` (wait) (`0x5191A0`) |
| 1503 | param1 1 = this player goes first | `resolveFirst(isFirst)` (`0x519230`); once the movie is done the step leaves the scene (`0x519370`) and the duel field opens |
| 1505 | param1 the match result (MainGamen `+0x820`; the other's at `+0x84C`, `0x504D00`); param2 100 or param3 3 take other paths | **not part of the janken**: the match's end before another duel (`0x5185A3`): a DuelResult, or with param3 3 text `0x2918`, then the scene leaves. DuelScene drops it unless param3 is 3 (`0x515183`). We sent it after 1503 until traced: `0x5185A3` never ran, DuelScene took it. A spectator's DuelResult takes it as the match result (`0x512070`); the server sends it there |

The server plays the CPU: a random hand; when the CPU wins it goes first (**guess**).

**Against another player** (a matched duel: both entries carry the `duel_id`): both duel connections sit at one table
(`table.rs`). 1500 goes to both once the second `0x200A` arrives; with both hands in, each client gets its own 1501
(its result, the other's hand, its own) and the winner 1502 {1}, the loser 1502 {0}; a draw lets both scenes ask
again. The winner's `0xD {choice}` gives each client its own 1503. One engine runs the duel, and each client has its
own translator in which it is player 0 (`PlayerNum` 0, its deck first, `FirstPlayer` from its side): the engine's
events go to both, each prompt to the client it asks, and either client's commands reach the other through its
connection's outbox. The opponent's hidden cards (deck, hand, extra deck, face-down) go with row 0 in 1000 and `0x1A`
(**guess**, like Online 2's placeholders) and face-down monsters get no field record. A client that leaves loses:
the other gets command 2 {1, 14 "has disconnected"}. Both front connections get `0x2009` with the record against
each other. Live (Yugi against Kaiba, LP 2000): Kaiba saw Yugi's Giant Orc on the opponent's side and Yugi's hand
face down; Kaiba's Set card showed face down on Yugi's field; the direct attack ended the duel with WINNER on Yugi's
client and LOSER on Kaiba's, and Next took both back to the map.

**Opening** (sent right after 1503; live: LP 8000 on both sides, Turn 1, the first player's DP lit in their colour):

| Message | Fields | Evidence | Status |
|---|---|---|---|
| setup (`0x15`) | `FirstPlayer` (engine player 0/1), `PlayerNum` (this client's player: 0; the scenes compare players with it, engine `+0x1B0`), `PlayerType001/002` (0 / 1), `MainDeckNum001/002`, `ExtraDeckNum001/002` (the decks' sizes: the one picked at Select Deck, the NPC's preset) | pump case `0x15`; the counts are the engine's per-zone counts (`+0x1C8`, `+0x1CC`, …) that DeckReset shows | confirmed (live: the deck piles below); `PlayerType` values **guess** |
| command 1 | none: duel start | DuelScene `0x514290` | shape |
| command `0xF` | param1 player, param2 LP | DuelCockpit `0x502EB0` (`setLp`) | confirmed (live) |
| command 5 | param1 player, param2 phase (0 draw) | DuelScene, DuelCockpit | confirmed (live: DP lit) |
| command 6 | param1 the player whose turn starts | DuelScene; the pop adds 1 to the turn count (engine `+0x1BC`) and flips the active player | confirmed (live: Turn 2), so not sent for the first turn |
| command 1000 (one per card, before 1) | param1 uid (low 24 bits; the top byte is a flag, meaning unknown), param2 CARD_Prop row, param3 0 (meaning unknown), param4 table place `player \| zone<<8 \| index<<16` | the pop's `0x521DF0` sets or adds the card in the engine's card table (engine `+0x90`); the scenes look cards up there by place (`0x523DF0`) or uid (`0x523D20` → param3, `0x523D40` → param2); a card object shows the row's image (`0x4FB090`, `data/card/<cid>_<n>.png`) | confirmed (live) |
| deck piles = `0x16` DeckReset ×4 | {`param1` the player (compared with `PlayerNum`), `param2` the pile zone `0xC`–`0xF`}, for each player's deck (`0xD`) and extra deck (`0xC`), after the LP and before the first draw | DuelCardManager `0x4FF3A5`: the pile (`+0x4C + (side × 0x11 + zone) × 4`) takes the engine's count for that zone (`0x5240F0`: the setup's `MainDeckNum`/`ExtraDeckNum` at first) and is shown with its count on the field label (`0x4FE4D0` → `0x50BED0`), or hidden at 0. After that each pile counts itself: a card landing on the extra deck, deck or graveyard adds 1 (`0x4FBF30`), one leaving subtracts 1 (`0x5000E0`). Without DeckReset the deck pile starts at 0 and never shows (it was so until 2026-11-07) | confirmed (live: Yugi's deck pile 34 and Kaiba's 35 after the opening and Yugi's draw, then Kaiba's 34 after his; Yugi's one-card extra deck showed a pile of 1 on both clients) |
| draw = 1000 + `0x1A` | 1000 puts the card at hand index i; `0x1A` {param1 `uid \| row<<16`, param2 from, param3 to} as move places `player \| zone<<1 \| index<<6` | DuelCardManager `0x4FF6A2` creates the card object (`0x5003A0`) and moves it; deck → hand counts the draw and plays the draw sound; the pop moves the per-zone counts | confirmed (live: five cards in hand, five face down in the CPU's); the index bits are a **guess** (Online 2's packing) |

The server shuffles both main decks, numbers the cards 1.. (the client's deck, then the CPU's), sends them all to the
card table, starts the duel, sets LP, deals five cards each (the first player first) and then sends the Draw Phase. The
CPU's cards go with their real rows; the client shows them face down. Online 2's server sent placeholders for hidden
cards, which a PvP duel will need.

The duel connection only names the player (`0x200A {player_id}`), so the front server keeps, by player, the deck sent
with `0x1F4F` and the NPC of the duel's `0x17AC` rule (`cpu_id`).

**Engine commands seen so far**, by the component that handles them (a scene or a component reacts to event `0x2001`,
which DuelScene `0x514140` sends for every popped command):

| Component (handler) | Commands |
|---|---|
| the pop `0x522A50` (engine state) | 2, 4, 6 (turn), `0x1A` (card move: param2 from, param3 to, as `player \| zone<<1`; zones 0–4 monsters, 5–9 Spell/Trap, 0xB hand, 0xC extra, 0xD deck, 0xE graveyard, 0xF banished), `0x36`, `0x37`, 1000–1019 (data loads: per-card records `0x3E9`–`0x3EB`, `0x3F5`; arrays `0x3EF`–`0x3F3`; card map `0x3F9`/`0x3FA`) |
| DuelScene `0x514290` | 1, 2 (duel end: param1 winner, param2 reason), 4, 5, 6, 7, `0x15`, `0x1E`, `0x22`–`0x25`, `0x36`–`0x3A`, `0x3C`, `0x3D`, `0x3F`, 1505 |
| DuelCardManager `0x4FF000` | `0x12`–`0x14`, `0x16`–`0x1D`, `0x1F`, `0x20`, `0x25`–`0x29` (cards are looked up in the engine's card map: `0x523DF0(player, zone, index)`) |
| DuelCockpit `0x502EB0` | 5, `0xF` (LP), `0x10` (LP change) |
| DuelFieldManager `0x50A690` | 4 (highlight the cards with commands), `0x26` |
| DuelSummon `0x5165A0` | `0x2A`–`0x32` |
| DuelChain `0x501560` | `0x33`, `0x34` |
| DuelCutin `0x505A70` | 6, `0x43`, `0x44`, `0x46`–`0x48`, `0x4C`, `0x4D` |
| DuelBattle `0x4F8A90` | 4, `0xA`–`0xE`, `0x10` |

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
(**guess**); NPC duels have none. A client starts counting when it takes `0x14` off its queue, after the animations
before it, so the server calls a time-out 5 s after 0. Live (Auto Duel, Match Mode): Yugi's clock ran (TIME 169, 09:49)
while Kaiba's stood at 180 and 10:00, each mirrored on the other client with "Now Thinking"; Kaiba, left idle, lost
duel 1 by time out (his total down to 07:00), the match went on, and in duel 2 Yugi's screen read "YOU LOSE — Yugi
has ran out of time." with its turn clock at 3 (hence the grace).

**Battles** (DuelBattle `0x4F8A90`; its caller `0x4F840E` passes `param1` in ECX, `param3` in EDX and `param2` on the
stack; DuelScene's busy check `0x515300` holds the queue while its state `+0x74` runs):

| Command | Params | What the client does | Status |
|---|---|---|---|
| `0xA` BattleInit | none | battle mode `+0x78` = 1 | shape |
| `0xC` BattleAttack | `param1` attacker's player \| its zone << 8, `param2` the target's player, `param3` the target's zone or `0xFFFFFFFF` (direct) | at a target: the attack, with the attacker's duelist cut-in (the NPC's line, or the player's `ace_monster`), and the hit on the target | confirmed (live: "I shall obliterate you!", the hit on Giant Orc) |
| `0xD` BattleRun | **per player**: `param1` the damage figure on the client's card, `param2` on the opponent's; `param3` flags, low byte the client's card, high byte the opponent's: bit 1 destroyed | states 5–6 (`0x4F8430`) play a damage or destroy effect at each side's card (`0x4F8920`, size by damage / 500); a destroyed card's effect ends hiding it (`0x4FB220` mode 0), before the server moves it | confirmed (live: values by attacker/target hid the wrong card when the CPU attacked) |
| `0xE` BattleEnd | none | battle mode off | shape |

The server sends Init and Attack for MSG_ATTACK, Run for MSG_BATTLE (damage by ATK against ATK or DEF, destroyed from
the message; a direct attack, whose target ocgcore-wasm reports as an empty place, gets no figures since there's no
card to show them at), End at DAMAGE_STEP_END, ATTACK_DISABLED, the next attack or a new phase. LP still change
through `0x10`.

**Destruction by effects** (DuelCardManager `0x4FF000`, its switch `0x50007C` by command id; both read `param1` from the
command record's params like the other components):

| Command | Params | What the client does | Status |
|---|---|---|---|
| `0x1F` CardVanish | `param1` the player, `param2` byte 0 the zone (at most `0xB`: the field or the hand), byte 1 the index in it | the card (`0x4F8240`) is marked off the field (`+0xD7`, cleared when it lands again, `0x4FBF30`) and runs effect mode 3 (`0x4FB7F0` → state `0x4FC520`): its zone's label goes (field Flash `setCurrent`, `setVisible` 0), effect `0x33` plays at its place (in the hand `0x4E5540`), and the card is hidden (`0x4FB220`). The queue waits for the card manager (busy check `0x5153CA` → `0x5004E0`) | confirmed (live: Masked Dragon, and later Sangan, shattered into shards when Trap Hole destroyed them, then went to the graveyard) |
| `0x20` CardBreak | `param1` the player, `param2` the field zone (at most 10), `param3` the card's uid | the card (by uid, `0x500470`) is marked off the field and its zone's label goes; no effect | confirmed (code; live: nothing visible); not sent |

The card's next move (`0x1A`) flies it on as usual (`0x5000E0` always runs effect mode 0). The server sends CardVanish
before the move of a card the engine destroyed other than by battle (MSG_MOVE's `reason` has DESTROY and not BATTLE;
battles have BattleRun's own destroy effect). ocgcore-wasm drops MSG_MOVE's `reason`, so the sidecar reads it back with
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
question in between); the failure seen on the Windows VM was not reproduced, see ROADMAP.

**Summons** (DuelScene): `0x38` RunSummon {player, the summoned card's uid} plays the summon effect at the card (`0x21`,
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
that (`engine/`, see ROADMAP M5); `translate.rs` maps its draws, turns and phases to the commands above (ocgcore phases
1, 2, 4, 8–`0x80`, `0x100`, `0x200` → 0–5), its idle-command prompt to a command map (summons bit 4, special
summons 2, position changes 5, 8 or 9 by the card's position as last shown (Flip Summon face down, to Attack
Position in face-up defense, to Defence Position in attack: offering both showed "to Attack Position" on an attack
position monster, live), monster sets 6, spell sets 7, activates 3; which Set bit is which is a **guess**)
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
both main decks, the opponent's with row 0 against a player); a card leaving the deck or the extra deck is found by its
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
HINTMSG_FMATERIAL and no message for Synchro materials (the client has no such text; **guess**). Live
(`--duel-deck 3339,1449 --duel-extra 3745`): with Junk Synchron and Giant Orc on the field, clicking the extra deck
listed Junk Synchron, then Giant Orc; both went to the Graveyard, the position dialog asked for Ancient Fairy Dragon,
and it stood in attack position on both clients with the extra deck pile down to 1. The client has no window for ocgcore's place prompt, so the server
takes the first free zone (positions: see Choices). Live: Summon from the hand put the monster in the
first monster zone, the hand closed up, and the next prompt offered only the Spell card.

**Tokens.** EDOPro's tokens have passcodes with no Konami id (YGOProDeck lists none for Scapegoat's Sheep Tokens), so
the sidecar adds the card's `name` from `cards.cdb` where it finds no id. The client has its own token cards: CARD_Prop
ids 3900–3999 (75 rows, e.g. four "Sheep Token", 3902–3905, one per colour). A token comes from nowhere (MSG_MOVE
from location 0): the server takes the client's token card of that name (in turn when there are several), gives it a
uid of its own (from `0x4000`), and sends its card table entry, its field record and a move from its slot onto
itself (`0x1A` finds no object by uid and makes one, `0x5003A0`). A token leaving the field (to location 0, or to a
pile) gets CardVanish and leaves the mirror. A card list finds a token at its place (it has no id to find it by).
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
changes, so the sidecar queries both players' monster zones (position, level, ATK, DEF, base ATK, base DEF) after each
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
signed change}` (DuelCockpit, clamped to 0–99999; param3 non-zero adds a pop-up) for ocgcore's DAMAGE, PAY_LPCOST
(negative) and RECOVER, and `0xF` for LPUPDATE. Live: a direct attack took the CPU from 8000 to 5700 LP. The attack has
no animation yet (DuelBattle's commands `0xA`–`0xE` are unmapped).

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

The server uses the card list for the client's MSG_SELECT_CARD and MSG_SELECT_TRIBUTE (targets, tributes, attack
targets, discards…): the rows from its mirror, the opponent's cards marked, kind 1 for min 0, `0x27` for min 1, else 6
with max lowered to min (**guess**). The list draws each card from `data/mini/` by its row's Konami id ("%4d_%d",
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
sidecar adds `description_cid` for a description `code << 20 | n`, EDOPro's `aux.Stringid`), else "Use the effect now?" (270) (**guess**: the
original questions are the card scripts' own strings). `DLGResult` 1 answers yes. Live (`--duel-deck 1149,1449`):
summoning Marauding Captain asked 'Activate the effect of "Marauding Captain"?'; Yes sent `DLGResult 1`, the card list
followed with "Select the monster(s) to Special Summon." and the hand's monsters, and the picked Giant Orc was Special
Summoned.

**Declarations.** The dialog command `0x36 {kind, message, param3}` has a window for three of EDOPro's declarations
(factory `0x4C8CD0`, kinds by the table `0x4C9178`), and the card list one for the fourth:

| Declaration | Client window | `param3` | DLGResult | Status |
|---|---|---|---|---|
| MSG_ANNOUNCE_RACE {count 1, available} | kind 5, DuelSelectTypeDialog (`dlg_seltype.swf`), "Select @3Type(s)@0." (241) | the allowed types as bits 1–22 in the client's order (UI texts 1031–1052: Dragon, Zombie, Fiend, Pyro, Sea Serpent, Rock, Machine, Fish, Dinosaur, Insect, Beast, Beast-Warrior, Plant, Aqua, Warrior, Winged Beast, Fairy, Spellcaster, Thunder, Reptile, Psychic, Divine-Beast), one pick (`dialog.typeFlag`, `dialog.selMax` 1) | the type's number | confirmed (live) |
| MSG_ANNOUNCE_ATTRIB {count, available} | kind 7, DuelSelectAttrDialog (`dlg_selattr.swf`), "Select @3Attribute(s)@0." (242) | allowed (bits 1–7: LIGHT, DARK, WATER, FIRE, EARTH, WIND, DIVINE, texts 1061–1067) << 16 \| picks | the picks as bits | confirmed (live) |
| MSG_ANNOUNCE_NUMBER {options} | kind 11, DuelSelectNumberDialog (`0x4C6430`, `dlg_selnum.swf`), "Declare @3monster@0 level." (280) for levels, else none | max << 24 \| step << 20 \| min << 16 \| format: 302 "%d000LP" counts in thousands, 303 "%d00LP" in hundreds, other formats (283 "%d") in ones (`dialog.numMin`, `numMax`, `step`) | the number in those units (`0x4C66D0`) | confirmed (live) |
| MSG_ANNOUNCE_CARD {opcodes} | the card list `0x37 {player, 0x4C, 0}`: DuelCardListDialogEx (`0x5131E0`, `dlg_cardlistex.swf`) lists the client's own card database with a name search ("Input text.", Filter); kinds `0x4C`–`0x4F` pick its filter (CardDuelFilter `0x4976A0`: any card, no extra deck monsters, main deck monsters, monsters) | — | the card's Konami id (`0x4C8290`, id & `0x3FFF`) | confirmed (live) |

The answers carry EDOPro's terms: the type's and attributes' RACE_/ATTRIBUTE_ bits, the number's index among the
options (the first at least as large as the one picked), and a card by Konami id, which the sidecar turns into a
passcode; races go to the core as BigInts (64-bit in ocgcore-wasm; a plain number threw "Cannot mix BigInt and other
types"). Several types at once and numbers out of the dialog's range keep the automatic answer; the CPU names the
duel's cards in turn (the core asks again when a name doesn't fit). The client has no other card name window
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

| Activation Message (`invoke_conf`) | Shows the question |
|---|---|
| Manual (0) | always, unless the right mouse button is held |
| Semi Auto Cancel (1, the default) | at level 2 and up ("low priority" below), or while the left button is held; never while the right one is |
| Auto Cancel (2) | only while the left button is held |
| (any) | always at level 4 and up |

Unshown, the dialog answers `DLGResult 0` (No) by itself (constructor `0x4C5120`). The server sends level 3 when
something happened (EDOPro's `hint_timing` of either player has a timing other than a phase or step, or a chain is in
progress), 1 at a phase or step, and 0 when no card fits the timing (`spe_count` 0, which EDOPro's own client skips)
(**guess**: the original server's levels are unknown). The question names the chain's last link (MSG_CHAINING, until CHAIN_END): 'The effect of "%s" is activated. Chain another card or effect?' (7), '"%s"
is activated. ...' (8) for a card in the Spell/Trap zone (**guess**), else "Activate the effect of a card?" (269). No
answers no chain; Yes chains the only card, or opens the card list (one pick) with "Select effect to activate." (276),
where no pick is no chain. A forced chain of several goes straight to the list; of one needs no choice. Live (two Set
Trap Holes, the CPU's summon): the question showed both cards; Yes opened the list, the second pick activated it, the
next chance read '"Trap Hole" is activated. Chain another card or effect?', and No resolved it, destroying the CPU's
monster. Places are still answered for the player.

**Chain links** (DuelChain `0x501560`, states `0x501250`; the record pump `0x523541` queues `{id, request, param1…4}` and
the component reads from `param1`). DuelChain keeps a list of links `{opponent's, zone, link number, effect}`; the
busy check (`0x515481`) holds the queue while it animates (`+0x58` ≠ 0):

| Command | Params | What the client does | Status |
|---|---|---|---|
| `0x33` ChainSet | `param1` the player (compared with `PlayerNum`, engine `+0x1B0`), `param2` the zone the number shows over (0–16: the place table `0x81AE38` read by `0x4E7990`, numbered as moves give zones: 0–4 monsters, 5–9 Spell/Trap, `0xA` Field Spell, `0xB` hand, `0xC` extra deck, `0xD` deck, `0xE` graveyard, `0xF` banished, 16 the field's centre), `param3` the link number | link 1 clears the list and adds itself silently; a later link is added and played (state 1: effect `0x31` at its zone, and at link 1's too when it is link 2; state 2: then the held number `0x30`). The new link is read at index `param3 − 1` (bounds-checked: a skipped link would stop the client) | confirmed (code; live below) |
| `0x34` ChainRun | `param3` the resolving link's number (read from the same place as ChainSet's); `param1`, `param2` unread, sent 0 | with fewer than 2 links: clears the list. Else link `param3` resolves: the last link first shows a banner (states 3–4, 400), then each plays effect `0x32` at its zone and leaves the list's end (states 5–6), so they must come last link first. Live: the link in `param1` made state 5 look up link −1, and the bounds check (`0x501620`, called at `0x5014B2`) stopped the client with `0xC0000417` | confirmed (code; live below) |

The server sends ChainSet at each MSG_CHAINING {controller, location, sequence, chain_size} (an Xyz material's
effect at its monster's zone; a place the client lacks at the centre, **guess**) and ChainRun at each
MSG_CHAIN_SOLVING {chain_size}, which the engine sends link by link from the last. Live (two Set Trap Holes
chained to the CPU's summon, twice): ChainSet `{0, 5, 1}` and `{0, 6, 2}` showed the numbered links 1 and 2 over
both cards, ChainRun `{0, 0, 2}` and `{0, 0, 1}` played the chain banner and faded each number, Trap Hole destroyed
the monster, and the duel went on.

The duel's end: ocgcore's WIN {winning team, reason} becomes command 2 {result, reason}. DuelScene keeps the result at
MainGamen `+0x81C` and its 1 ↔ 2 swap as the opponent's, and adds a DuelEndEffect (`0x5083A0`) that plays
`data/gfx/duel_win.swf` / `duel_lose.swf` / `duel_draw.swf` for result 1 / 2 / 3 (name table built at `0x754550`) with
text 1999 + reason (`0x508650`): 1 LP 0, 2 deck out, 3 Exodia, 4 Destiny Board, 5 surrender, 6 Last Turn, 7 Final
Countdown, 8 "won with" a card, 9 Vennominaga, 10 Exodius, 11 time out, 12 surrender, 13 disqualified, 14
disconnected. Its update (`0x5084D0`) waits until the movie reports it has finished (about 3 s), then marks the
effect done (`+0x28`). EDOPro's core keeps prompting after a win, so the sidecar ends the duel at the WIN message; its
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

| Field | Where | Read by | Status |
|---|---|---|---|
| item `duelData` (by `name`), **tag 7** | `0x2009` | the dispatcher keeps it in front slot `0x15` and sets flag `0x20000`; tag 7 makes a `YO3DuelData` (factory `0x531B90`), which finds its children `gameServer`, `regulation`, `rule`, `players` (`+0x78`) and `result` (`+0x7C`) by name (`0x5315C0`) | confirmed (tracer, live) |
| `score_<player_id>` int | `0x2009` top level | the dispatcher, copied as `event_score` | shape; left out |
| `duel_end` int | `result` | DuelScene (`0x515BD3`): 2 or 3 show text `0x2918` and leave | shape; we send 1 (**guess**) |
| `duel_result` int | `result` | DuelResult step 1 (`0x5122B0`): the match result (1 won, 2 lost, 3 draw) at MainGamen `+0x820` | confirmed |
| `dlogURL` string | `result` | step 1, only when MainGamen `+0x804` is 0: opens a component (`0xFFFFD90B`) on that URL | sent for duels between players (see "Duel log files") |
| `player_id` int | each item of `players` | `0x510830` finds this player's record | confirmed |
| item `resultData` (by `name`) | the player's record | step 1 sets DuelResult `+0x60` | confirmed |
| `win`, `lose`, `draw` and `win_add`, `lose_add`, `draw_add` ints | `resultData` | getResult (`0x511200`, `0x511934`–`0x511D2D`): the player's panel "Wins 1 (+001)", then lose and win swapped for the opponent's panel | confirmed (live); totals and deltas are our reading |

Without `0x2009` the result screen waits forever. Live (`--duel-lp 1000`): "YOU LOSE" with "Kaiba's Life Points
have been reduced to 0", then LOSER / WINNER panels (Kaiba / Seto Kaiba) with Wins, Losses, Draws and a Next button;
Next leaves to Kaiba Corporation, where the NPC says his after-duel line. The client then refreshes its player
(`0x2CEE`). 1504 instead opened Card Swap. Command 1506 is taken inside the queue pop (`0x522A50`) and makes the
client send a bare `0x1842` on the duel connection (meaning **open**).

The numbering differs from Online 2's engine (whose turn change is `0x04` and LP set `0x26`), so YO2 research is a guide
only. Next: the card map, so decks, hands and draws show.

### Duel log files (`0x512070`, `0x498700`, `0x498B10`, `duellog.rs`)

After a duel the client records it itself from the `0x2009` it gets (front slot 21, flag `0x20000`). DuelResult's tick
(`0x512070`, state 1) finds the player's item in `players`, reads `duel_result` and the text `dlogURL` from the
`result` item, and, **only when MainGamen `+0x804` is 0** (a duel against a player: the flag is set for NPC duels, whose
result screen takes the other branch of `0x511200`) and the text isn't empty, starts the duel log task
(`0x498700 (url)`): `0x498B10` makes a connection object (`0x4EF4B0`, id `-9973`: an HTTP download of the URL) writing
to `duellogs\<temp>\<file name>`, and when it is done (state 10) copies the file into `myduels\` (`0x4980A0`) and
deletes the oldest beyond 100 (`0x498510`). So the log is a file the **server** serves, and **a duel against an NPC
leaves none** (the only other caller of `0x498700` is the Server Duel Logs tab's download). `dlogURL` is
`http://<host>:8080/duellogs/NAME.dlog` (the host is the address the client reached), served by our patch HTTP server from
`data/duellogs/`; the file name is the URL's, ours `END_TIME-DUEL_ID-PLAYER_ID.dlog`, one file per player. The log is
that of the match's last duel. Live: both clients fetched theirs, Latest Duel Logs listed it ("Lv1 Yugi V.S. Lv1
Kaiba"), Play replayed the duel on the field from the janken to the duel's end, then asked "Save this duel log?"
(No) and returned to the main screen.

The file (readers `0x538AB0` header, `0x5384F0` all, `0x538BD0` comment rewrite; little endian): u16 `0x09E7`, u16 1,
then u32s at 4 (unknown, we send 0), 8 `duel_id`, 12 `player_1`, 16 `player_2`, 20 `match_type`, 24 `dlv_1`, 28 `dlv_2`,
32 `end_time`; 36 `pname_1` (32 bytes), 68 `pname_2` (32), 100 `dlog_comment` (256), 356 a u32 the rewrite keeps (we send
the body's length), and from 360 the body, run-length coded as (value, count) byte pairs (`0x538360`). The body
(`0x5384F0`, then `0x524330`, which builds from it the match push `0x1978` of a watched duel, a `duelData` and the
setup): 20 u32s (`duel_id` third, the end time fourth (`0x17C`), `match_type` fifth, `event_type` seventh, `official`
eighth), two player blocks of 8656 bytes, then a u32 count and that many commands of six u32s `{id, 0, param1..param4}`
(`0x522A50`: the engine commands; entries whose second u32's low byte is negative are skipped). A block: 11 u32s
`{player_id, dlv, MainDeckNum, ExtraDeckNum, title, ?, PlayerType, card_protector, n_win, n_lose, n_draw}`, `pname` (32
bytes), the avatar (128), `country` (4), 32 speech lines of 256 bytes and a 256-byte deck name. The setup is not in
the file: `0x524330` makes it from the blocks (FirstPlayer and PlayerNum are 0). So a replay is what a spectator of
the player who went first sees: our commands are the janken as that player saw it (1500-1503, which take the pre-duel
screen to the field; without them the replay waits on it), the card table, the duel's start and LP, the engine's events
and 1505 (the match's end for that player; without it the replay ends on the pre-duel screen). The first player's
block comes first, so `player_1` is whoever went first. Unknown: the u32 at 4, the blocks' sixth u32, the first u32s of
the body other than the four named; the hands show as white boxes in a replay (a spectator sees none).

## Accounts

The client never sends a user id or password. `0x1F47` is a bare request; the credentials typed at login are copied
onto the front connection object (`+0x118`/`+0x134`, on `0x219F`) and never serialized. The server therefore tells
accounts apart by the client's address and lets the player pick one of up to four players at Select Player
(`0x1F4C {player_id}`). Players can't be made from the client: `CharMake.cpp` (`0x2CF5`/`0x2CF6`, reached from the
map's `GoCharMake`) edits an existing player's appearance, and the original made players on Konami's website. So
the host names them (see `docs/friends.md`).

The gate's redirect (`0x21FE {host, port}`) and the duel server's `0x22C4 duelData {host, port}` carry the address
each client reached the server at (the connection's own address), unless the server is told one: one server then
answers on loopback, a LAN and a VPN at once. Live: a client whose `yo3.ini` named the LAN address got the LAN
address for the front and duel servers, another on `127.0.0.1` got loopback.

## Fields the client reads that we leave out

The field tracer shows these lookups failing, and the client copes: an absent int reads as 0, an absent raw value as
NULL, which these callers check. Send them when we know what they mean.

| Reply | Field | Where it's read | Notes |
|---|---|---|---|
| `0x2201` | `bgm` int (top level) | global dispatcher `0x4F506A` | probably the lobby music |
| `0x21FC` | `icon`, `lefttime` (int), `maintenance` (string), in the `{lang}` item | `0x4F477C`, `0x44E6EF`, `0x44E72D` | a maintenance countdown; absent = none |
| player record | `npc_id` int | `0x51BFE2` | 0 = a human player |
| player record | `evalnum`, `evaltotal`, `evallevel` uint | the shop's `0x1C20` case (`0x48ABB6`…), through UserData+`0x2C` | the player's manner rating; only used when the shop text is empty |
| `0x27D9` | `cpuUse`, `notRelease` (raw) in its record | `0x4F2080`, `0x4F20BA` | meaning open (CPU duels? unreleased cards?) |
| (around `0x2CF7`) | `premium` uint, from **another dataset**, not the reply | `0x44F3E7` | probably the account's Duelist Premium flag |
| `0x2CF7` items | `duel_id`, `event_score`, `duel_status`, `tprank`, `duel_match_type`, `world_evid`, `official_evid`, `user_evid` (int) | `0x475A20` | 0 = not duelling, no rank, no event; they pick the list icon |

## Families seen in the client, not mapped yet

- `0x2CEC`–`0x2CF8`: player search, character creation (`CharMake.cpp`: `0x2CF5/6`) and player data.
- `0x2774`–`0x277A`: presents (`PresentWindow.cpp`). `0x2774` = count, a push the global dispatcher keeps in slot 23
  (it also sets the "presents pending" flag `0x80`); `0x2777/0x2779` carry full present records (`present_id`,
  `type`, `level`, `form`, `item_img`, `end_time`, `item`…); `0x2778/0x277A` mean none (see `0x2775`).
  The same numbers also show up as UI text ids (`FUN_0051E6C0(table, id)`): don't confuse the two.
- 48 more `REQUEST_` / `RESPONSE_COMMUNICATION_*` names for trade and rental (search the exe's strings).
- `DuelServerConnection`: the third server tier; see "Duel connection".
- Official duel logs were served over HTTP (`http://download.yugioh-online.net/duellogs/last/`), alongside team and
  Online 2 downloads under `download.yugioh-online.net`.
