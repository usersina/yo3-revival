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
| `0x2710` NPC roster | `0x2711` (`0x2712`) | items `{npc_type, npc_id, need_ex}`, three per NPC (one per level), kept in slot 4; see "NPC duels and duel start" | `0x473646`, `0x4BEB40`; live: 26 NPCs listed by name | confirmed; `npc_type` 0 for all and `need_ex` = the level are **guesses** |
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
| `0x1F50` delete `{delete_type, deck_name}` (`0x4449A9`, after deleting the local file `0x52F0C0`) | `0x22CC` (`0x22CD` shows an error) | none | "deck delete succes"; live: the deck leaves the Load Deck list | confirmed |
| `0x1F4F` update `{duel_id, event_id, update_type}` + the deck (tag 6) | `0x22CA` (`0x22CB`) | none | Saving in the editor only writes the local file (`0x4442E0` → `0x4159B0` → `0x52EE60`) and `cursor_deck_name`, though the editor has a "deck add or update succes" case for `0x22CA`. The request comes from a duel's Select Deck (DuelManager `0x50ECF0`, see below); live: the duel scene loads | confirmed; nothing is stored yet |

Every player gets one deck, "Starter Deck": the first `deck_type` 1 preset (`data/recipe`, ids 1–6, 501, 502; they
look like the starter decks) that the login regulation allows. Preset 1 holds a card regulation 1 forbids (4966), so
it is preset 2 (**guess**: the original let a new player choose). The server lists it every time, so a deleted
starter deck comes back the next time Edit Deck opens.

### NPC duels and duel start

The NPC roster comes from the main screen's startup poll `0x2710`; the global dispatcher keeps `0x2711` in slot 4.
SelectNpcScene (Kaiba Corporation) reads it three items at a time (`0x4BEB40`), one per level: the first item
gives `npc_type` (the tab, 0–2) and `npc_id`, each of the three gives `need_ex`. A level is open while the player's
experience with that NPC (`npcbin` entry 3 of its id; absent = 0) reaches its `need_ex`, counting from the first.
NPC ids start at 120 (`0x78`), three per NPC: names are text `0x5208 + 2 * ((id - 120) / 3)` (`0x4BFE34`), deck
titles text `0x5FB4 + 2 * id` (`0x4BFE05`), and each NPC deck preset (`deck_type` 3) carries that title id as
`name_id`, which is how the server finds the 78 ids (26 NPCs; the 26th has no English name). Hovering an NPC
computes `sum(min(ex, need_ex)) * 1000 / sum(need_ex)` over its levels (`0x4BFDC4`): all-zero `need_ex` divides
by zero.

Picking a level (`decideNpc` `0x4C0C80`), its speech and "Start Duel?" lead to the duel start step `0x4BE390`. It
finds the roster item whose `npc_id` is the first id + level and hands it to `DuelMatchingAgent` (`0x4A0B50`) in mode
3; that item becomes the agent's record and, later, the opponent's `PlayerData` (MainGamen `+0x408`). The agent
checks the deck (`0x4A0E90`; the item may carry `regulation_id`, else the player's), then:

| Step | Request | Reply (error) | Fields read | Evidence | Status |
|---|---|---|---|---|---|
| `0x4A1090` | `0x1784 {event_id}` (1 for an NPC duel) | `0x1785` (`0x1786`) | `join_id` (agent `+0x58`); the whole reply is copied to MainGamen `+0x868`; the global dispatcher (`0x4F4AE7`) stores `event_id`, `join_id`, `server_id`, `event_score` (ints) | live: client then sends `0x17AC`, trace "event join offer notice" | confirmed; `join_id` = a server-wide counter (**guess**) |
| `0x4A1790` | `0x17AC {event_id, join_id}` + item (tag 4) `{cpu_id, duel_server_id, match_type, regulation_id, surrender, time_total, time_turn}`; live: `cpu_id` 124 (the NPC id), `match_type` 4, `surrender` 1, the rest 0 | `0x17AD` (`0x17AE`, dialog `0x4F7940`) | `duel_id` (agent `+0x5C`); the reply is copied to MainGamen `+0x88C` with the agent's `regulation_id`; `country` is read from the player's own data | live: the scene opens DuelManager's Select Deck | confirmed; `duel_id` = a server-wide counter (**guess**) |

Mode 1 then asks for a password (`duel_password_flag` from the agent's record), mode 2 goes to `0x4A1380`, mode 0
(the DUEL panel) waits for a match in `0x4A1D10`. Mode 4 starts at `0x4A3340`: `0x1838 {duel_id}` → `0x1839
{host (string), port (int)}` (`0x183A` = error), written into DuelServerConnection (`+0x78`, `+0x94`), and
MainGamen `+0x8B8` = 5, `+0x8BC` = 1. Nothing we have reached uses mode 4 yet.

DuelManager (scene `0xFFFFD8FD`, constructor `0x50E470`) picks its first state by MainGamen `+0x8B8`: 5 → `0x50F320`
(connect DuelServerConnection, then `0x1838` on it), 6 → `0x50F030`, else → `0x50ECF0` (the NPC duel):

| Step | Request | Reply (error) | Fields read | Evidence | Status |
|---|---|---|---|---|---|
| Select Deck `0x50ECF0` | `0x1F4F {duel_id, event_id 0, update_type 1}` + the chosen deck (tag 6) | `0x22CA` (`0x22CB`, text `0x2919`) | none (`0x50EFC1` compares the id; the deck editor and the shop take `0x22CA` too) | live: the duel scene loads | confirmed |
| `0x50F030` state 2 | `0x17D4 {event_id, join_id, duel_id}`, once the avatar object (`AvatarViewer` `0x4DC320`; an NPC opponent is set up by its vfunc `0x68`) reports state 2 or −1; skipped when MainGamen `+0x8BC` is set | `0x17D5` (`0x17D6`, text `0x2919`) | none | live: sent right after the duel scene loaded, before any play | confirmed (meaning: "ready", **open**) |
| `0x50F690` state 1 | (none: waits for front flag `0x8000`) | push `0x22C4`, which the global dispatcher keeps in slot 20 and flags (`0x4F4F34`) | items found by their `name` field (`0x531EB0`): `duelData` {`host` string, `port` int}, and its sub-item `rule`, which replaces MainGamen `+0x88C` (`match_type` read from it). Host and port go to DuelServerConnection (`+0x78`, `+0x94`), which connects | tracer: `host`, `port`, `name`, `match_type` found; live: the duel connection opens | confirmed; `rule` = the item of the duel's `0x17AC` (**guess**: only `match_type` is read so far) |
| `0x50F690` state 2 | **duel connection**: `0x200A {player_id, autoSet}` (`autoSet` from the `duelOption` `auto_place`), unless MainGamen `+0x8BC` is set; then heartbeats `0x2724` every 30 s (`0x4F1730`) | no reply: the server starts the janken (see "Duel connection") | | live: `{autoSet 1, player_id 1002}` | confirmed |

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
| `0x14` timers | `timerKind`, `start`, `count1`, `total1`, `count2`, `total2`, `other` and their `_max` | `0x520A00` | shape |
| `0x15` duel setup | `FirstPlayer`, `PlayerNum`, `PlayerType001/002`, `MainDeckNum001/002`, `ExtraDeckNum001/002` | stored on the engine (`+0x1AC`…`+0x1E4`) | shape |

| Client `duelRequestName` | Fields | Sender |
|---|---|---|
| 1 | `size` … | `0x520C20` |
| 2 | `CommandID`, `index`, `player`, `position` (a card command) | `0x520FD0` |
| 3 | `phase` | `0x521180` |
| 4 | `DLGResult`, `debug` (a dialog answer) | `0x521A70` |
| 5 | | `0x5212B0` |
| 6 | `SetIndex` | `0x521BC0` |
| 7 | `data`, `index` | `0x521CC0` |
| 8 | `player` | `0x521980` |
| 9 / 0xA / 0xB | debug: `DebugID`; `wCardID`, `fFace`, `fTurn`; `id`, `type` (with `index`, `player`, `position`) | `0x5213C0`, `0x521560`, `0x5217C0` |
| 0xC | `selectHand` (janken: 0 rock, 1 scissors, 2 paper) | `0x4F1330` |
| 0xD | `choice` (1 first, 2 second) | `0x4F1430` |
| 0xE | | `0x50FDF0` (DuelManager) |
| 0xF | `player`, `autoSet` (duel options; live on entering the duel field: `{player 0, autoSet 1}`) | `0x515790`, `0x4A0390` |

**Janken** (who goes first; JankenScene `0x518400`, `duel_janken.swf`). Engine commands, all confirmed live:

| Command | Params | Scene |
|---|---|---|
| 1500 | none | `beginJanken`, then the hands (`0x518D80`); sent after `0x200A` |
| 1501 | param1 result (0 draw, 1 win, 2 lose), param2 the opponent's hand, param3 this player's hand | `resolveJanken(myHand, yourHand, result)` (`0x518F50`); after a draw the scene asks for a new hand by itself (`selectJanken` `0x518ED0`), so the server sends nothing more |
| 1502 | param1 ≠ 0: this player chooses | `selectFirst` (FIRST TO GO / SECOND TO GO), else `beginFirst` (wait) (`0x5191A0`) |
| 1503 | param1 1 = this player goes first | `resolveFirst(isFirst)` (`0x519230`); once the movie is done the step leaves the scene (`0x519370`) and the duel field opens |
| 1505 | param1 the match result (MainGamen `+0x820`; the other's at `+0x84C`, `0x504D00`); param2 100 or param3 3 take other paths | **not part of the janken**: the match's end before another duel (`0x5185A3`): a DuelResult, or with param3 3 text `0x2918`, then the scene leaves. DuelScene drops it unless param3 is 3 (`0x515183`). We sent it after 1503 until traced: `0x5185A3` never ran, DuelScene took it |

The server plays the CPU: a random hand; when the CPU wins it goes first (**guess**).

**Opening** (sent right after 1503; live: LP 8000 on both sides, Turn 1, the first player's DP lit in their colour):

| Message | Fields | Evidence | Status |
|---|---|---|---|
| setup (`0x15`) | `FirstPlayer` (engine player 0/1), `PlayerNum` (this client's player: 0; the scenes compare players with it, engine `+0x1B0`), `PlayerType001/002` (0 / 1), `MainDeckNum001/002`, `ExtraDeckNum001/002` (the decks' sizes: the one picked at Select Deck, the NPC's preset) | pump case `0x15` | confirmed (live, no visible effect of the counts yet); `PlayerType` values **guess** |
| command 1 | none: duel start | DuelScene `0x514290` | shape |
| command `0xF` | param1 player, param2 LP | DuelCockpit `0x502EB0` (`setLp`) | confirmed (live) |
| command 5 | param1 player, param2 phase (0 draw) | DuelScene, DuelCockpit | confirmed (live: DP lit) |
| command 6 | param1 the player whose turn starts | DuelScene; the pop adds 1 to the turn count (engine `+0x1BC`) and flips the active player | confirmed (live: Turn 2), so not sent for the first turn |
| command 1000 (one per card, before 1) | param1 uid (low 24 bits; the top byte is a flag, meaning unknown), param2 CARD_Prop row, param3 0 (meaning unknown), param4 table place `player \| zone<<8 \| index<<16` | the pop's `0x521DF0` sets or adds the card in the engine's card table (engine `+0x90`); the scenes look cards up there by place (`0x523DF0`) or uid (`0x523D20` → param3, `0x523D40` → param2); a card object shows the row's image (`0x4FB090`, `data/card/<cid>_<n>.png`) | confirmed (live) |
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
summons 2, position changes 8 and 9, monster sets 6, spell sets 7, activates 3; which Set bit is which is a **guess**)
plus command 4, and the client's card command back to the prompt's entry. Live: after the CPU's turn the client's Main
Phase 1 highlights only the legal cards, and a monster's menu reads Summon / Set.

Card moves (`MSG_MOVE`) become a card-table entry plus `0x1A`, with move places `player | zone<<1 | index<<6`, face up
bit 14 and defence bit 15 on field slots (`0x5000E0`); field slots are zones 0–4 (monsters), 5–9 (Spell/Trap) and 10
(Field Spell) at index 0, then hand `0xB`, extra `0xC`, deck `0xD`, graveyard `0xE`, banished `0xF`. Cards behind a
removed list card get their table entry again. The client has no window for ocgcore's place and position prompts, so
the server takes the first free zone and face-up attack when allowed. Live: Summon from the hand put the monster in the
first monster zone, the hand closed up, and the next prompt offered only the Spell card.

A monster's field label and card info read a 9-value record per place (`0x523F00`, key `player | zone<<8`; the info
panel `0x453740`): shown Konami id, a second id, race, attribute, level, ATK, DEF, base ATK, base DEF (ATK below base in
red, above in blue, `0xFFFF` = "?"). Data commands build it: `0x3E9 {_, values 0, 2, 3}`, `0x3EA {_, values 4, 5, 6}`,
`0x3EB {key, values 7, 8, 1}`, which commits (the pop; stores happen before the copy). Level, attribute and race are
CARD_Prop's second word bits 12–15, 8–11 and 19–23. The server sends it when a monster enters a monster zone, with the
CARD_Prop stats. Live: Giant Orc's label reads 2200 and its info "ATK 2200/DEF 0" with its icons and level 4. A
per-place 3-value map (`0x3F5`, `0x523F50`…) holds a typed counter for the same panel.

Battle: the engine's battle prompt becomes a command map with bit 0 (Attack) on each attacker and bit 3 on chainable
cards, the M2 and EP buttons as allowed, and command 4 mode 3. A monster in slot s sits at command-map place (zone 0,
index s), and the client sends Attack as `{CommandID 0, position 0, index s}`. LP changes: command `0x10 {player,
signed change}` (DuelCockpit, clamped to 0–99999; param3 non-zero adds a pop-up) for ocgcore's DAMAGE, PAY_LPCOST
(negative) and RECOVER, and `0xF` for LPUPDATE. Live: a direct attack took the CPU from 8000 to 5700 LP. The attack has
no animation yet (DuelBattle's commands `0xA`–`0xE` are unmapped).

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
  then takes 1504 to Card Swap (`0x50FDF0`, side deck, then `0x1F4F` again) or 1500 to the next duel (`0x50F690`);
- an empty queue and front flag `0x20000` (or MainGamen `+0x8BC`): the match is over. It reads the result below, and
  DuelManager leaves the duel (`0x510220`, scene `0xFFFFD901`) because the match result (MainGamen `+0x820`) is set.

Other commands at the head are taken and dropped. The match is over when the **front** server pushes `0x2009`
(dispatcher `0x4F4C9E`), sent as soon as the engine ends the duel:

| Field | Where | Read by | Status |
|---|---|---|---|
| item `duelData` (by `name`), **tag 7** | `0x2009` | the dispatcher keeps it in front slot `0x15` and sets flag `0x20000`; tag 7 makes a `YO3DuelData` (factory `0x531B90`), which finds its children `gameServer`, `regulation`, `rule`, `players` (`+0x78`) and `result` (`+0x7C`) by name (`0x5315C0`) | confirmed (tracer, live) |
| `score_<player_id>` int | `0x2009` top level | the dispatcher, copied as `event_score` | shape; left out |
| `duel_end` int | `result` | DuelScene (`0x515BD3`): 2 or 3 show text `0x2918` and leave | shape; we send 1 (**guess**) |
| `duel_result` int | `result` | DuelResult step 1 (`0x5122B0`): the match result (1 won, 2 lost, 3 draw) at MainGamen `+0x820` | confirmed |
| `dlogURL` string | `result` | step 1, only when MainGamen `+0x804` is 0: opens a component (`0xFFFFD90B`) on that URL | left out |
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
- `DuelServerConnection`: the third server tier; see "Duel connection".
- Official duel logs were served over HTTP (`http://download.yugioh-online.net/duellogs/last/`), alongside team and
  Online 2 downloads under `download.yugioh-online.net`.
