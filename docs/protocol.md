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
| `0x1F48` login `{lang}` | `0x2201` | top: `regulation_id`; item[0]: `user_id`, `uname`, `dpoint` | L449, `uid : [1001]` L457 | confirmed |
| `0x1F49` player list | `0x2260` | items = player records (below) | "MyPlayerList" L509, loader `0x51C260` | confirmed |
| `0x1F4C` select `{player_id}` | `0x2266` | none | "my player use success" L577 | confirmed |
| `0x1FA4` trunk `{player_id}` | `9000` (`0x2328`) | exactly one record; item[0] is copied wholesale into `UserData+0x8C8`, and the global dispatcher keeps the reply in slot 18. The record holds `cards` and `noTrade` (binary): owned and untradeable count per card, one byte at index row − 1 (CARD_Prop rows, row 0 a dummy, so 4,833 bytes). `0x410DB0` reads them bounds-checked; the collection builder `0x4F6340` writes into both unchecked (crash at `0x4F6A82`, opening the NPC duel) | "trunk data success" L666, `0x46AFA8`, `0x410DB0` | confirmed; `cards` = one copy per card of the starter deck (**guess**: a real collection comes with the economy); other contents **guess** |
| (server push after `9000`) | `0x251C` friend list, `0x252A` black list | items `{friend_id, friend_name, login_flag}` / `{black_id, black_name, login_flag}` (`0x475A20` modes 2 and 3); we send none | the global dispatcher stores them in dataset slots 6 and 7; the client never asks, and listing any player crashes without them (`0x45E734`) | shape (empty); push timing **guess** |
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
| `0x27D8` | `0x27D9` (`0x27DA`) | one record (copied into a 0xC4-byte object) | item[0] read unchecked at `0x4F5BE9` | shape |
| `0x2CEE` player refresh `{player_id, type}` | `0x2CF3` (`0x2CF4`) | item = the selected player's record; its `player_id` must match | `0x47378A`, `0x4F5FC2` | confirmed |
| `0x2583` | `0x2588` (`0x2589`) | `count` u32 | `0x4730B0` | shape (unread mail or presents?) |
| `0x271A` | `0x271B` (`0x271C`) | none | `0x47352F` | shape |
| `0x2710` NPC roster | `0x2711` (`0x2712`) | items = NPCs `{npc_type (tab 0–2), npc_id}`, each with up to three sub-items `{need_ex}` (`0x4BEB40`); kept in slot 4 for the NPC duel's opponent list. We send none | `0x473646`; an experiment with one NPC listed it (two stars, no name) but picking it sent nothing: the NPC's details come from somewhere not found yet | shape (empty) |
| `0x2AFB` | `0x2B13` (`0x2B14`) | none | `0x4738FA` | shape |
| `0x2AFD` | `0x2B17` (`0x2B18`) | none; the global case reads `member_id`, `authority` if present | `0x473A3A`, `0x4F5E0E` | shape (team?) |
| `0x2775` next present (bare) | `0x2778` none (`0x2777`/`0x2779` would carry a present) | none | builder `0x4F3A00`. The `0x2201` reply sets flag `0x80` (so does a `0x2774` push), so startup step `0x473AE0` opens PresentWindow, which sends `0x2775` and holds the main screen locked until it closes. Its switch (`0x476E74`): `0x2777`/`0x2779` "PRESENT content get success", `0x2778`/`0x277A` "content get error", which retries while `+0x58` > 0 and then closes (`0x4783C9`); its `0x2774` case reads `present_count` and drops it | confirmed (live: the facilities unlock after `0x2778`) |

### Info and chat windows

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x1F41` info list | `0x21FC` | item[0] `{lang}` whose items are `{starttime "%Y-%m-%d %H:%M:%S" JST, title, body}` | "information recv success!" `InfoWindow.cpp L1547`, `0x458A64` | confirmed (`lang` value **guess** "1") |
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
  (`0x2000`/`0x4000`/`0x8000`). The preset tabs above the list send fixed sets of these bits. Which status bit is
  which is open.
- Live: the dialog's name search sends `{search_flag 2, pname "kai", search_type 1, base_point 0, front_num 100,
  back_num 0}`.
- `0x4B6AF0` looks one player up: `{search_flag 0x40000, player_id, search_type 0}`, and reads item[0]'s
  `player_id`, `pname`, `duel_id`, `duel_status` (and `friend_id`/`black_id`/`member_id`) through `0x4B6060`.
- Reply items, read by `0x475A20` in player mode (ECX = 0): `player_id` (int; 0 is skipped), `pname`, `dlv`,
  `duel_id`, `event_score`, `duel_status`, `tprank`, `duel_match_type`, `world_evid`, `official_evid`, `user_evid`,
  `userevent_remain`. The handler skips the searcher's own `player_id`, so a server may list everyone.
- We filter on name (case-insensitive substring, a **guess**), level range and player id, and ignore the other
  criteria. Paging is a **guess**: skip `base_point`, return at most `front_num`.

### Chat (ChatWindow dispatcher `0x435950`)

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x2135` room list | `0x2458` (`0x2459`) | top `lang`; items = rooms `{id, name, capacity, num, region, status, type}` (read by `0x434D20`); `id` ≥ 1 | "chat room list success!" `ChatWindow.cpp L339`; the room chooser lists them with `[num/capacity]` | confirmed (`region`/`status`/`type` **guess** 0) |
| `0x2136` enter `{room_id}` | `0x245A {room_id}` (`0x245B {error_code}`) | the client ignores `room_id` < 1 | "In>>Lobby" printed, tab renamed | confirmed |
| `0x2137` leave | `0x245C` | none | "chat room exit" | shape |
| `0x2134` send | `0x2456` (`0x2457`) | fields sit in **one item (tag 10)**: `chat_str`, `chat_type`, and `to_id` or `to_name`. Room chat = `chat_type` 1 with `to_id` = the room id; whisper = `to_name` | captured packets; builders `0x4F3E00` (to_id) and `0x4F4030` (to_name) | confirmed (room); whisper shape |
| (server push) | `0x2455` | items `{from_id, pname, chat_type, chat_str}`; the handler also checks `friend_id`/`member_id` | two clients see each other's lines; the client shows nothing on `0x2456`, so the sender needs the echo | confirmed |

### Shop (`shopmain.cpp`, handler `0x48AA60`)

The map's top-left facility. Opening it re-requests the trunk (`0x1FA4`), and each reply below asks for the next, in
this order; the last one clears `isLoading`. Success and error replies are pairs (`0x1C20`/`0x1C21`, …); none of
them has a global case. The lists are empty until the economy exists (M4); every list case checks its item count
(`0x410D90`) before reading.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x1B66` text (bare, sent from the `9000` case, `0x48BEDD`) | `0x1C20` (`0x1C21`) | `text`: the shopkeeper's line. The case also reads `evalnum`, `evaltotal`, `evallevel`, but from the player record; they pick a stock line (`$33003`–`$33007`) when `text` is empty | "shop text success" L286 | confirmed |
| `0x1B67` items (bare, `0x48ACC5`) | `0x1C22` (`0x1C23`) | items `{item_type, item_param}`; `item_type` < 3 are packs | "shop item success" L331 | confirmed (empty) |
| `0x1B6F` (bare, `0x48AE5F`) | `0x1C26` (`0x1C27`) | items `{item_id, item_cost, item_trade, item_type, modified, rel_time, sale_id}` | "shop exchange item success" L360 | confirmed (empty) |
| `0x1B6B` (bare, `0x48B3B8`) | `0x1C32` (`0x1C33`) | items `{item_id}` | "shop protector item success" L409 | confirmed (empty) |
| `0x1B6D` (bare, `0x48B4C3`) | `0x1C2E` (`0x1C2F`) | items; toggles `_root.spac_btn_disable` (Special Packs) | "shop event item success" case; the client moves on to `0x1B72` | shape (empty) |
| `0x1B72` (bare, `0x48B64D`) | `0x1C36` | `perk_num` u32, compared with 500 to show `_root.expack_del` | tracer: found at `0x48B6CD` | shape (value **guess** 0) |
| `0x2C40` rental terms (bare, `0x48B777`) | `0x2D75` (`0x2D76`) | `rental_cardnum`, `rental_point`, `rental_term` u32 | read at `0x48C20E`; the case then sends `0x1B69` | shape (values **guess** 0) |
| `0x1B69` (bare, `0x48C391`) | `0x1C2A` (`0x1C2B`) | items `{item_type, item_param}`: avatar items; clears `isLoading` | "shop avatar item success"; the Shop Menu opens | confirmed (empty) |
| `0x1B68` buy `{sale_id, item_num, end_time, ex_pack}` | `0x1C24` (`0x1C25`) | `dpoint` | "shop buy succes" | open |
| `0x1B6A` exchange `{sale_id, item_id, kaban}` | `0x1C28` (`0x1C29`) | | "shop exchange succes" | open |

### Events (TournamentScene, UserEventScene)

The map's Official Event tile (`MenuSelect` 21) opens TournamentScene, Championship (20) a 64-player tournament map
that sends nothing, and User event (22) UserEventScene. With no decks, Official Event and Championship stop at "There
are no valid Decks available." and OK returns to the map.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x177A` event list (bare, `0x4D0BB0`) | `0x177B` (`0x177C`, dialog `0x4F7940`) | top `result_number`; items `{event_type, status, event_id}`. TournamentScene looks for `event_type` 10 with `status` 12; none means its no-event path | `0x4D0BB0`; the Stadium opens | shape (empty) |
| `0x1789` event search `{search_flag, pname, status, pack_type, base_point, front_num 30, back_num}` (`0x4D9E37`) | `0x177B` (`0x177C`) | as above; the list case `0x4D66A0` reads `result_number` (tracer) and `premium` from another dataset | captured packet; User Event lists nothing, page 1/1 | shape (empty) |
| `0x339A` own user event (bare, `0x4D9C2B`) | `0x33FE` (`0x33FF`) | `remain`, `event_id`: the scene keeps `event_id` only while `remain` ≠ 0 | `0x4D9BB0` | shape (both 0) |

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
| `0x1F50` delete `{delete_type, deck_name}` (`0x4449A9`) | `0x22CC` (`0x22CD`) | | "deck delete succes" | open |
| deck add or update | `0x22CA` (`0x22CB`) | | "deck add or update succes" | open |

Every player gets one deck, "Starter Deck": the first `deck_type` 1 preset (`data/recipe`, ids 1–6, 501, 502; they
look like the starter decks) that the login regulation allows. Preset 1 holds a card regulation 1 forbids (4966), so
it is preset 2 (**guess**: the original let a new player choose).

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
| player record | `npcbin` raw: four u16 per NPC id (the player's experience with each NPC, compared with `need_ex`) | `0x51C680` (NULL-safe) | absent = no experience |
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
- `DuelServerConnection`: the third server tier, not reached yet.
- Official duel logs were served over HTTP (`http://download.yugioh-online.net/duellogs/last/`), alongside team and
  Online 2 downloads under `download.yugioh-online.net`.
