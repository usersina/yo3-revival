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

**Banlists.** The client ships 20 regulations (`data/regulation/*.y3r`, named in the text table): 1 "Beta
Forbidden/Limited", 2 "Pre-Season", 3 "unlimited" (no lists: every card 3 times), 4–15 the OCG lists of 2004-03 to
2009-09 (which also limit the card pool to the cards out by then: `addition_card` 0 and the pool as `card3`), 16–20
"Season 12" to "Season 16". Only the client enforces them: its deck check (`0x51AB00`) before Auto Duel and NPC duels
uses the login's `regulation_id` (`0x219A`, `0x2201`), and a hosted duel uses the host's choice. The server sends the
one its `--regulation` setting names (an id, or a name such as "unlimited"; default 1, the Beta list, a **guess**);
the starter deck is still the first preset regulation 1 allows. Live: the Starter Deck breaks regulation 4 (six cards
newer than March 2004); with `--regulation 4` Kaiba Corporation answered "There are no valid Decks available.", with
`--regulation unlimited` the same deck reached the janken. Create Duel → Customize → Regulations shows the login
regulation as "Latest Regulation" (with "unlimited": no Forbidden cards, "Cards not listed are Unlimited.") and Select
lists the others by name with their Forbidden / Limited / Semi-Limited tabs (an OCG list: Limited, Semi-Limited and
"Any", "Cards not listed are Forbidden."). The same screen offers Wait Time Per Turn (Standard, Any, 2, 3, 5 minutes)
and Total Wait Time (Standard, Any, 5, 10, 20 minutes): Standard is the 180 s / 600 s the clocks use.
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
them has a global case. Every list case checks its item count (`0x410D90`) before reading. The shop's BP
(BOOSTERPASS Points, `user_bp`) is the account's `dpoint` from the login replies.

| Request | Reply (error) | Fields | Evidence | Status |
|---|---|---|---|---|
| `0x1B66` text (bare, sent from the `9000` case, `0x48BEDD`) | `0x1C20` (`0x1C21`) | `text`: the shopkeeper's line. The case also reads `evalnum`, `evaltotal`, `evallevel`, but from the player record; they pick a stock line (`$33003`–`$33007`) when `text` is empty | "shop text success" L286 | confirmed |
| `0x1B67` items (bare, `0x48ACC5`) | `0x1C22` (`0x1C23`) | items `{item_type, item_id, item_cost, item_max, sale_id, sale, item_trade, modified, rel_time}` (`0x485E50`; dates are text); `item_type` < 3 go on the shelves, others are kept only when `item_param` isn't owned (a bit set at the player data's `+0x68`). The kept list (`+0xD4`, 0xD8 bytes each) fills the Purchase screen (`0x48E0A0`): kinds 0 and 3 call the Flash `AddPack(id, name, comment, image, date, cost, max, trade)` (`0x48C790`) with the name, description, image and date from the client's own pack file `item_id`, 1 and 4 `AddStruct`, 6 `AddProtector`, 10 and 11 `AddEventPack` | tracer: all nine found; live: three packs on the Booster Packs shelf with their own art, then "Pack 1", "A rearranged pack comprising of season 1 Cards.", 100 BP | confirmed |
| `0x1B6F` (bare, `0x48AE5F`) | `0x1C26` (`0x1C27`) | items `{item_id, item_cost, item_trade, item_type, modified, rel_time, sale_id}` | "shop exchange item success" L360 | confirmed (empty) |
| `0x1B6B` (bare, `0x48B3B8`) | `0x1C32` (`0x1C33`) | items `{item_id}` | "shop protector item success" L409 | confirmed (empty) |
| `0x1B6D` (bare, `0x48B4C3`) | `0x1C2E` (`0x1C2F`) | items; toggles `_root.spac_btn_disable` (Special Packs) | "shop event item success" case; the client moves on to `0x1B72` | shape (empty) |
| `0x1B72` (bare, `0x48B64D`) | `0x1C36` | `perk_num` u32, compared with 500 to show `_root.expack_del` | tracer: found at `0x48B6CD` | shape (value **guess** 0) |
| `0x2C40` rental terms (bare, `0x48B777`) | `0x2D75` (`0x2D76`) | `rental_cardnum`, `rental_point`, `rental_term` u32 | read at `0x48C20E`; the case then sends `0x1B69` | shape (values **guess** 0) |
| `0x1B69` (bare, `0x48C391`) | `0x1C2A` (`0x1C2B`) | items `{item_type, item_param}`: avatar items; clears `isLoading` | "shop avatar item success"; the Shop Menu opens | confirmed (empty) |
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
and when Kaiba quit, Joey's client went back to the map.

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
| challenger | `0x17B6 {event_id, join_id, duel_id, password}` (`0x4A28F0`) | `0x17B7` ("Requesting Duel"), or `0x17B8 {error_code}`: 3000 "Another player is requesting a duel.", 3001 "Already Duelling", 4006 "Password is Incorrect.", 201 "Duel Request was refused.", others "A transmission error has occurred." | | live | confirmed |
| host | push `0x17B6 {duel_id}` + item `playerInfo` (the challenger's record) | | the wait step reads `player_id` (a player on the black list is declined at once), `pname`, `dlv`, `title` | live: "LV1 Kaiba / Default / You have a Duel Request! Will you accept?" | confirmed |
| host | `0x17CA {event_id, join_id, duel_id, answer}` (2 Yes, 1 No) | `0x17CB` (`0x17CC` fails, `0x4A2D30`) | | live | confirmed |
| challenger | push `0x17CA {answer}` | | 2: on to `0x4A2D30`; else refused | live | confirmed |
| challenger | `0x17C0 {event_id, join_id, duel_id}` (its Cancel) | `0x17C1` (**guess**); the host gets push `0x17C0`, closes the offer and waits again | | code | shape |

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
summons 2, position changes 8 and 9, monster sets 6, spell sets 7, activates 3; which Set bit is which is a **guess**)
plus command 4, and the client's card command back to the prompt's entry. Live: after the CPU's turn the client's Main
Phase 1 highlights only the legal cards, and a monster's menu reads Summon / Set.

Card moves (`MSG_MOVE`) become a card-table entry plus `0x1A`, with move places `player | zone<<1 | index<<6`, face up
bit 14 and defence bit 15 on field slots (`0x5000E0`); field slots are zones 0–4 (monsters), 5–9 (Spell/Trap) and 10
(Field Spell) at index 0, then hand `0xB`, extra `0xC`, deck `0xD`, graveyard `0xE`, banished `0xF`. Cards behind a
removed list card get their table entry again. The client has no window for ocgcore's place prompt, so the server
takes the first free zone (positions: see Choices). Live: Summon from the hand put the monster in the
first monster zone, the hand closed up, and the next prompt offered only the Spell card.

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

`0x27` (named CardUpdate above) is a card's counters: `param1` byte 0 the player, byte 1 the zone, `param2` a change
added to the count of counter kind `param3` (`0x4FB6C0`/`0x4FB730`, a map at `+0x114`), then effect mode 5 and sound
`0x58` (up) or `0x57` (down) (DuelCardManager `0x4FFD8E`). Not sent yet.

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
with max lowered to min (**guess**). EDOPro's hint before the choice (MSG_HINT type 3, HINTMSG_*) picks the message
when a text says the same thing: 500 → 18 Tribute, 503 → 70 remove from play, 504 → 58 Graveyard, 506 → 88 add to
hand, 509 → 25 Special Summon, 549 → 222 attack target, 551 → 94 target; otherwise none. The picks answer the engine;
no pick cancels when it allows. Live: Giant Orc's attack opened the list with "Select an attack target monster." and
the CPU's monster marked as the opponent's; picking it sent `6`, `7 {index 0, data 1}`, `4 {DLGResult 1}` and the
attack dealt 1200.

The client's MSG_SELECT_EFFECTYN opens the Yes/No dialog with 'Activate the effect of "%s"?' (250) and the card's
name, MSG_SELECT_YESNO with 'Use the effect of "%s"?' (226) naming the card whose script string the question is (the
sidecar adds `description_cid` for a description `code << 4 | n`), else "Use the effect now?" (270) (**guess**: the
original questions are the card scripts' own strings). `DLGResult` 1 answers yes. Live (`--duel-deck 1149,1449`):
summoning Marauding Captain asked 'Activate the effect of "Marauding Captain"?'; Yes sent `DLGResult 1`, the card list
followed with "Select the monster(s) to Special Summon." and the hand's monsters, and the picked Giant Orc was Special
Summoned.

Positions: the client's MSG_SELECT_POSITION {code, positions} opens the position dialog with 'Select position for
"%s".' (327), any uid holding the card's id, and the flag when face-up defense is allowed; button 0 answers face-up
attack (else face-down attack), button 1 face-up defense (else face-down). Without an attack position the first
allowed one is taken. Live (`--duel-deck 1149,1151`): Marauding Captain's Special Summon asked for Warrior Dai
Grepher's position with the card upright and sideways; Defence sent `DLGResult 1` and the card moved face-up in
defense (`0x1A` to `0xC004`).

Options: MSG_SELECT_OPTION {options} opens the Select dialog when the client has a label for every option. EDOPro's
system strings with one: SELECT_HEADS/TAILS 60/61 → "Heads"/"Tails" (400/401) under "Coin Toss Selection:" (56), and
DECLTYPE_MONSTER/SPELL/TRAP 70–72 → "Monster Cards", "Spell Cards", "Trap Cards" (440–442) under "Select card type."
(260). Options that are a card's own strings (`code << 4 | n`; 51 of the client's cards) have no text on the client
(the prompt table's 400–490 are the original server's per-card labels), so the first is taken. Live (Time Wizard):
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
