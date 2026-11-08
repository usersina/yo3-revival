# Roadmap

Goal: play Yu-Gi-Oh! ONLINE 3 with friends on our own server. Each milestone ends in something checked against the
real client (`tools/client/vclient.sh`), not just against tests. Tick boxes in the same commit as the work.

## ▶ Overnight plan (2026-11-07): everything left, end to end

Goal: finish what the client can use, then hand over for the real friend test. Each slice goes the usual way
(evidence, failing test, code, live check on the headless clients, docs, its own commit fast-forwarded into `main`),
most valuable first. Research for the later slices runs ahead in parallel (read-only).

1. [x] Duels: an extra deck summon seen live (Synchro, materials picked in the card list); [x] the Number, Type and Attribute declarations
       (dialog kinds 11, 5, 7) and card name declarations (the card list's searchable kind `0x4C`); [x] options that
       are a card's own strings (the prompt table's 400–495 labels)
2. [x] Duels: a spectator through a match's later duels (live, no change needed); settled without code: chain
       chances outside timely ones (EDOPro's "always chain" asks with no card able to respond, which this client's
       Activation Message option has no setting for) and a server-side deck check (done later, see M5; the client checks sizes,
       ownership and the banlist itself; only a modified client gets past it, out of scope for friends)
3. [x] NPC experience (`npcbin`): wins against an NPC unlock its higher levels; every NPC enabled (`premium`), on
       its series tab, the four gated ones willing to duel
4. [x] D-Mail between players, kept for players offline
5. [x] Main Menu screens: Card Album, Rental Card List, My Profile, Duel Log, Leaderboard, BOOSTERPASS, each with
       what the server knows (collection, records, duels played)
6. [x] Teams (create, join, leave, members, team chat, Create Duel's Team limit) and user events (create, join, a
       knockout of event duels)
7. [x] Player search criteria: language, manner level, status, event, chat room, country (the account's setting)
8. [x] Shop: protectors and avatar items for BP; card trade and card rental between players
9. [x] A Windows server build (cross-compiled) in the release bundle, with the friends' guide updated
10. [x] Final: full test run (250 tests, clippy clean), every flow seen live during the night on the final code paths
        (and a last Auto Duel match and single duel, which caught the 0x1785 re-queue), the release bundles rebuilt
        (Linux and Windows); [ ] then the real friend test (M6 exit)

Not possible with this client, so left out: logins with a user id and password (the client never sends
credentials; the delete dialog's password prompt is local too). (Corrected later: the client does have Create Player and
Delete on Select Player, `0x1F4A`/`0x1F4B`, now answered.)

## ▶ Windows VM test (2026-11-08): what a real Windows client turned up

The user played the real client natively on a Windows 11 VM (its own address, its own account) against a headless
client, through the Linux release bundle's `run.sh`. Findings, in the order they came:

- [x] `yo3.ini`: only the gate line was changed, so the patch check went to Konami and the game never connected. The
      friends' guide now says all five lines change, how to edit a file in Program Files, and a browser check.
- [x] The host's firewall (firewalld's `libvirt` zone) blocked the VM; friends over the VPN (trusted zone) aren't
      affected. Noted in CLAUDE.md.
- [x] No banlist by default (the user's choice); `--regulation` picks one.
- [x] A new player needs Edit Deck once before any duel (the client alone writes deck files): the login news says so.
- [x] Attacks and activations from any field slot but the first were ignored and the duel stalled: the client sends
      a field card's command as (zone slot, index 0).
- [x] Set Spells/Traps lay sideways like set monsters (ocgcore's face-down defense position).
- [x] No way back from an attack's or a card's target, or a chain list: cancelable lists now need no pick (kind 1),
      and the list's own script then takes a right-click as "decide with nothing picked", which cancels (its OK
      button never shows; a click on a card still decides at once when one is allowed). Live: right-click withdrew an
      attack and the monster could still attack.
- [x] The opponent's Set Spell/Trap showed blank white once activated: it now moves onto its own slot with its row.
      Live: Spellbinding Circle and Magic Drain showed their art on the other client.
- [x] The Graveyard pile showed a card back on both clients, even with cards moved in face up: a pile draws its top
      card's face only when that card's table entry has the flag of command 1000's uid top byte. Graveyard and
      banished cards now carry it. Live: both Graveyards showed their top card on both clients, also after a card left
      from under the top, and hovering one previews it.
- [x] An attack position monster offered "to Attack Position" too: a position change now offers only the entry that
      fits (to Defence in attack, to Attack in face-up defense, Flip Summon face down). Live: Giant Orc in attack
      position offered only "Change to Defence Position".
- [x] An attack on an empty field asked nothing (EDOPro asks only when a monster could be attacked instead): a direct
      attack now asks the original's "Attack your opponent directly?" first (**guess**: the original asked every
      time). Live: No went back to the battle commands (the question came again on the next click), Yes dealt 2700.
- [x] The Graveyard couldn't be opened. Now Confirm (the menu's bit 11, which highlights a pile) sits on both
      players' Graveyards and banished piles and the player's own extra deck, and opens a view-only card list under
      the original's own titles ("Opponent's Graveyard", "Your Extra Deck", ...); a right-click closes it and gives
      the field back. Live on both clients.
- [x] Card target lists had no message: EDOPro's HINTMSG_* hints for destroy, equip, face-up (Spellbinding Circle's
      target), discard, return, Set, control, show and position changes now read as the client's own prompts. Live:
      "Select card to destroy." (MST), "Select a monster to equip." (Gravity Axe).
- [x] Found while checking: an opponent's Set card in a target list drew nothing and couldn't be clicked (MST couldn't
      hit a Set card; an attack on a Set monster would have been stuck the same way). Such lists now open as the
      client's card-back kind (`0x4A`). Live: MST listed and destroyed a Set Spellbinding Circle. **Limitation**: that
      kind shows every card of the list as a back and can't be backed out of, so in a list of face-up and Set cards
      the player can't tell them apart.
- [x] Found while checking: a pile's commands were looked up at index 0 only, so an extra deck summon or Graveyard
      effect of any card but the pile's first was unreachable. They now sit at index 0, and with several cards able the
      card list asks which (the original's "Select the monster(s) to Special Summon from your Extra Deck.").
- [x] Found while checking: tokens never showed (Scapegoat's Sheep Tokens were invisible, so they couldn't be seen,
      attacked knowingly or used). EDOPro's tokens have no Konami id; the sidecar now names them and the server shows
      the client's own token card of that name. Live: four coloured Sheep Tokens on both clients; one destroyed by an
      attack shattered and left the field.
- [x] Testing aid: `--duel-time 0,0` turns the clocks of duels between players off (live checks run slower than
      180 s a turn).
- [x] Checked live in three more duels between the headless clients (to the end at 4000 LP): Pot of Greed, Change of
      Heart on a Set monster (it moved to the taker's side; its menu offered only Flip Summon), Man-Eater Bug's flip
      effect, Book of Moon, an attack on a Set monster (the card-back list), Dark Hole, Black Rose Dragon's wipe with a
      chained Spellbinding Circle, Monster Reborn from either Graveyard, Mirror Force, Witch of the Black Forest's
      deck search, a three-link chain, and WINNER / LOSER with the records on both clients.
- [x] Cards an effect reveals (EDOPro's MSG_CONFIRM_CARDS, e.g. Witch of the Black Forest's "show it to your opponent")
      weren't shown to the opponent: they are now, through HandShow (`0x13`): a hand card shows at once, a searched
      card when it reaches the hand. Live: the new card flipped to its face in the opponent's hand row and the info
      panel named it (recorded frames). Revealed Set cards and Deck tops still aren't shown (open).
- [x] A target list of face-up and Set cards opened as the card-back kind `0x4A` (every card a back, no way out): it now
      opens as kind 4 with per-card table flags, so face-up cards keep their faces, Set cards are backs and a right-click
      backs out. Live: Giant Orc's attack list showed a back and a face; right-click withdrew it; a click on the back
      attacked the Set card.
- [ ] Open: a monster whose position an effect changes right after its summon (Stumbling) stayed upright on its
      owner's screen when no prompt came in between (the other client turned it; with a chain question in between both
      did). Not reproduced headless in three tries (own and opponent's Stumbling, with and without a question): the
      monster turned on both screens each time, after the summon effect. Found: the summon effect is waited for only
      by `0x22`–`0x25`, `0x38`, `0x39`, so a `0x1C` runs while it plays; a safe barrier needs a command whose
      execution is a no-op, and none is known (`0x25` would call `0x4FB220` on a card whose identity is unknown).
      Next: reproduce on the VM with `YO3_TRACE_HITS` on `0x4FB7F0` (the card's effect modes) or a cold file cache.
- [ ] Card preview on hover needs a held button on the VM (the client shows it on hover under Wine): likely the VM's
      PS/2 mouse or window focus, not the server.

## ✅ M0: Research and tooling

- [x] Feasibility study ([docs/research.md](docs/research.md))
- [x] Wire crypto and message format; the client's data files decode with the same codec
- [x] Headless client on Xvfb with a virtual mouse; the client's own debug trace captured
- [x] Annotated disassembly, RTTI map, reply-dispatch router, Ghidra headless export
- [x] GhidraMCP (headless) registered for the project

## ✅ M1: Stable world

- [x] Gate redirect, version check, auth, login, player list, player select
- [x] World entry (`0x1FA4` → `9000`)
- [x] Every main-screen poll answered; news list; player search; regulations
- [x] Rust server (`crates/`), test-first, at parity on the real client
- **Exit:** a cold start reaches a stable main screen with no crash and no timeout dialog (met 2026-11-06)

## ✅ M2: An instrumented client

Stop finding missing fields one crash at a time.
- [x] Field-read tracer: a 32-bit DLL (mingw-w64) hooking the one lookup behind every field getter
      (`std::map::find` at `0x41A980`) and list-item reads (`0x410D60`). It logs the reply being parsed, field,
      getter type, found/missing, value type (from RTTI), the asking function and top/sub level.
- [x] Loaded by `vclient.sh start --trace` through an injector, so the game folder is never modified
- [x] `tools/tracer/summarize.py`: per reply, the fields read and the ones missing, with the asking function
- [x] First catch: `0x2775` (its reply later corrected to `0x2778`), `0x2CF7.result_number`, and the read-but-absent
      fields in `docs/protocol.md`
- [x] Code hits: `YO3_TRACE_HITS=0x…,0x…` logs when chosen client addresses run (int3 and a vectored handler); it
      found the startup step that kept the map locked
- Standing practice (moved to CLAUDE.md): name functions in GhidraMCP as we understand them
- **Exit:** one session prints every field each reply was asked for, with the missing ones flagged (met 2026-11-06)

## ▶ M3: A social world

- [x] Answer `0x2775` (next present: `0x2778` = none, which closes PresentWindow and unlocks the map and menu)
- [x] Chat rooms (the chat panel's top-left button) and chat messages: room list, enter, leave, room chat
      with echo, whispers
- [x] Sessions registry: who is online, in which room (`Hub`); sessions leave on disconnect
- [x] Accounts by client address (the client never sends credentials), up to four players each
- [x] Two clients at once: `YO3_INSTANCE=b` with its own display and reflinked Wine prefix
- [x] Player search returns the players online, filtered by name, level or id; both clients list each other
      (needed the friend and black list pushes, `0x251C`/`0x252A`)
- [x] Map locations: the map unlocks, and each of its six facilities opens and returns to the map (rooms are the
      chat rooms above; the DUEL panel belongs to duels, M5)
  - [x] Shop: opens to the Shop Menu with empty shelves and returns to the map (rental records pushed at entry)
  - [x] NPC duel: opens (Kaiba Corporation), says there are no valid decks, returns to the map (the trunk now has
        `cards`/`noTrade` arrays, all zero)
  - [x] Character settings: Avatar Settings opens with a default avatar and returns to the map (`avatar1` needs
        41 entries); saving changes is not handled yet
  - [x] Official Event (Stadium) and Championship (tournament map): open, report no valid decks, return
  - [x] User event (Satellite Plaza): events created, listed, joined, started and run as a knockout
- [x] Friends and the ignore list: friend requests through the bell, accepting or declining, withdrawing, removing,
      the ignore list, both lists at login and on request, friends' online state pushed as they come and go, kept in
      `friends.json`; player search's friend, ignore and team tabs. Live: Yugi and Kaiba became friends, stayed so
      after a restart, and Yugi removed Kaiba
- [x] Main Menu's Friend List and Ignore List views: their search reply comes before their list, so the rows draw.
      Live: Kaiba listed with the friend icon, online and after logging out
- [x] D-Mail: Write Mail to a friend (`0x2580` → `0x2584`), the recipient's bell (`0x2581`), the fetch (`0x2583` →
      `0x2588`, `0x2582` → `0x2586`); mail waits in `data/mail.json` for players offline. Live: Yugi to Kaiba
- [x] Teams: form, invite, answer, leave, remove, authority, disband; the team in records, search and Player
      Information; team chat, team mail and Create Duel's Team limit; `data/teams.json`. Live: Millennium formed,
      Kaiba invited and joined, team chat, Set as subleader
- [x] Player search criteria: language (from the login), status (duelling, hosting by rule), chat room, manner level
      (from the manner ratings), events (nobody). Live: the waiting tab listed only the host
- [x] Manner ratings: after a duel between players the server pushes `0x2CEC {eval_id}`, DuelResult opens "Please
      evaluate NAME." (`dlg_manner.swf`), the answer `0x2CED {eval_id, eval_data}` (-10..10) is kept in the accounts
      and the average sets `evallevel` (search entries, Player Information, the player records). The client has no
      ratings for trades. Live: Yugi rated Kaiba +10, Kaiba rated Yugi -10; each saw the other's face icon change
- [x] Country: no request carries the client's country, so it is a setting per account (`yo3-server set-country`, kept in
      `accounts.json`; default from the login's language), sent in the login item, player records, search entries (and
      the search's country criterion) and the bracket. Live: Player Information showed the UK flag, then Japan's
- [x] Main menu screens: Rental Card List (`0x2C34`/`0x2C35` chain), My Profile (wins and losses against players),
      Duel Log (`0x1793`, `0x19C8`), BOOSTERPASS (codes refused); Card Album, Leaderboard (a Konami
      web page) and Duel Options need nothing. Live: each opened; Team is with teams
- [x] Local duel logs: the client keeps no log by itself: after a duel against a player it downloads the `dlogURL`
      of the `0x2009` result (`0x512070`; never against an NPC), a `.dlog` the server builds (`duellog.rs`: header,
      both players, the spectator's command stream). Latest Duel Logs lists it and Play replays the duel. Live: Yugi
      and Kaiba's duel, listed, replayed to "Save this duel log?"; a match keeps its last duel only
- [x] Saving character settings: Avatar Settings' Confirm sends `0x2CEF` {avatar1, speech00–22, comment00–01,
      title} → `0x2CF5`; kept with the account and sent back in the player record. Live: purple hair after a restart
- [x] User events: Create, the list and details, Participate and leave, the lobby and its Event Menu (Start, Cancel),
      the event's duels paired round by round through the match push. Live: "Friday Cup", Yugi beat Kaiba in it
- [x] User events, the lobby: the bracket's names, faces and advances (`0x32FF` blocks, `0x32FD` entries), the round
      banner and status, a wait between rounds with the TIME countdown, the winner; open events kept across restarts
      (`events.json`; a running event ends at a restart). Live with three players, a bye and a final. Ready/Standby:
      the client builds no such request (docs/protocol.md), so none exists to answer
- **Exit:** two headless clients log in as different players, find each other and chat (met 2026-11-06:
  Yugi and Kaiba chat in the Lobby; the room list shows the other player)

## M4: Accounts, collection and economy

- [x] Real accounts: not applicable. The login path has no id or password field: `0x1F47` is bare, `0x1F48` carries
      only `lang`, the opener only the version string, and Delete Player's "Enter your Password" prompt sends nothing
      (`0x1F4B {player_id}`, live packet log; no call of the connection's `+0x118`/`+0x134` copies is serialized).
      Several accounts exist by client address; persistence stays JSON (`data/accounts.json`) with no concrete need
      for SQLite
- [x] Player creation and deletion: Select Player's Create Player and Delete (`0x1F4A {pindex, pname}` → `0x2262`/`0x2263`,
      `0x1F4B {player_id}` → `0x2264`/`0x2265`; `CharMake.cpp`'s `0x2CF5/6` is only the appearance editor), within four
      slots kept with the account. Live: Joey made in slot 3 (name, then Avatar Settings, then the main screen), deleted
      with its password prompt (local, nothing sent), the slot empty again
- [x] Trunk contents: the card collection (the starter deck's cards and the player's bought cards, `0x1FA4` → `9000`)
- [ ] Deck list, deck editor saves (`HolderSelect.cpp` "deck list")
  - [x] Every player owns a starter deck's cards and gets it in the deck list (`0x1F4E` → `0x22C8`, tag-6
        items); Edit Deck saves it locally and the NPC duel accepts it
  - [x] Deleting a deck (`0x1F50` → `0x22CC`); saving never reaches the server (local files only)
  - [x] Decks on the server, so they follow a player to another machine: the editor never uploads a save, but a duel's
        Select Deck sends the whole deck (`0x1F4F` update_type 1), which the server keeps per player and lists in
        `0x22C8`. Live: a deck used in a duel came back after its local file was deleted. A deck never used in a duel
        can't reach the server (the client has no other upload)
- [x] Shop: every booster pack of the client's pack files on the shelves (`0x1C22`), bought with BOOSTERPASS Points
      (`0x1B68` → `0x1C24`), opened as presents (`0x2775` → `0x2777` with the cards); BP and bought cards kept per
      account and player in `data/accounts.json`. Live: three Collection Pack vol.1 opened card by card, and after a
      server restart the shop showed 4700 BP and the pack 21.4% collected
- [x] BP from duels: each counted result pays into the account (win 100, draw 50, loss 30), which the login replies
      and every purchase re-read. Live: a lost NPC duel took 5000 to 5030, shown at the next login; a pack then left 4930
- [x] Structure decks: SD001–SD005 (the art-carrying `deck_type` 1 recipes) on the STRUCTURE DECK tab, bought as
      type-1 presents with the deck id; the deck's cards join the collection. Live: GADGET BOX (deck 502) for 500 BP,
      shown as its 40-card grid, then 41 cards (with its extra deck) in the account
- [x] Bought structure decks in the deck list (`0x22C8`), under their names from the client's text table
      (`data/text/txttbl_E.bin`, now read by `yo3-data`), kept per player. Live: GADGET BOX was saved locally as
      "GADGET BOX.y3d", listed in Load Deck and loaded with its 40 + 1 cards
- [x] Protectors and avatar items for BP: their shelves from the item catalogue, presents type 7 and 6, owned ids kept
      with the player in the record's pbin. Live: P-01K and Domino: Red bought
- [x] Card trade between players: invitation, the trade scene's offers, OKs and confirmation; ownership checked and
      the cards moved in the accounts; a new trunk after. Live: Tainted Wisdom for Windstorm of Etaqua
- [x] Card rental between players: invitation, the rental scene (conditions, cards, Confirm), the records `0x2D6C` the
      client counts its trunk from, the Rental Card List, terms of 1/3/7/14 days that end by themselves, kept in
      `data/rentals.json`. Live: Yugi lent two cards to Kaiba, both trunks changed, a restart kept it and the term's end
      gave the cards back with both online. Not seen live: a Deck rental (three lists); the client has no early return
- [x] Presents between players: not possible, the client has no way to send one. PresentWindow only asks for the next
      present (`0x2775`); no screen builds a send request (docs/protocol.md, "No presents between players"). Cards
      move between players by trade and rental, both done
- **Exit:** create a player, open packs, build a deck, restart the server, everything is still there

## ✅ M5: Duels

- [x] The main screen's DUEL panel: the menu bar's duel button opens the Duel Menu (`0x17FC` counts); Auto Duel asks
      for a match (`0x1798`), and the server pairs two players waiting for the same rule format (push `0x1978` with
      the opponent and the duel). Live: two clients matched and saw each other
- [x] Create Duel: a hosted duel shows on its host in player search (Duel Conditions, Send Duel Request); the
      offer reaches the host, who accepts or declines; passwords, a busy host and a withdrawn offer are handled; the
      duel starts with the host's LP. Live: Kaiba challenged Yugi's hosted duel and both reached the janken
- [x] Match Mode: best of three on one duel connection (1500 starts the next duel's janken, `0x2009` carries the
      match result); Auto Duel's Both matches either rule format. Live: Kaiba won a hosted match 2–0
- [x] Duel clocks between players: the timers message (`0x14`) drives each side's turn and total clocks; the server
      runs them while the engine waits on a player and ends the duel when one runs out (reason 11). Live: in an Auto
      Duel Kaiba's clock ran down from 180 while Yugi's stood still, and an idle player lost: "Yugi has ran out of
      time.", then the match went on to its next duel
- [x] Watching duels: Player Information's Watch Duel on a duelling player (mode 4): the front sends the duel
      server and both players (`0x1838` → `0x1839`), the duel server catches the spectator up from seat 0's side
      (janken, setup, the duel so far, both hands hidden) and then sends every event, and at the match's end 1505
      with the result. Live: Joey watched Yugi and Kaiba's Auto Duel and went back to the map when Kaiba quit, and
      watched a whole Match Mode match (the pre-duel screen between duels, the second duel, the map at the end)
- [x] Card Swap between a match's duels: a player with a side deck gets 1504 (DeckReplace), the swapped deck comes
      back through the front (`0x1F4F` update_type 2) and the duel connection's `0xE` starts the next janken. Live:
      Yugi swapped Danipon in for Crass Clown and duel 2 was dealt from the swapped deck
- [x] Create Duel's Opponent's Limits: Friend lets only the host's friends challenge (the client hides the button,
      the server refuses with 201); Team takes nobody until teams exist. Live: Joey had no Send Duel Request, Kaiba
      did and his request reached Yugi
- [x] NPC roster and duel start: 26 NPCs (ids 120–197 from the NPC deck presets) listed by name with their
      levels; picking one runs `0x1784` → `0x17AC` → Select Deck (`0x1F4F`) → `0x17D4`, and the duel screen opens
      with both players. The client builds NPC duels itself (DuelManager `0x50ECF0`); no duel server is involved
- [x] Duel server handoff: after `0x17D4` the server pushes `0x22C4` (`duelData {host, port}` + `rule`), the
      client connects to our duel listener (port 1214, logged only) and sends `0x200A {player_id, autoSet}`
- [x] Duel connection mapped (both directions' `duelRequestName`s, the engine command pump) and the janken played
      against the server as the CPU (commands 1500–1503): the duel field opens, Turn 1
- [x] Duel opening: setup `0x15` with both decks' sizes, duel start, LP 8000 (`0xF`), the first player's Draw Phase
      (5); the engine commands are mapped to the components that handle them (docs/protocol.md)
- [x] Cards: both shuffled decks go into the engine's card table (command 1000) and each player draws five (1000 +
      the card move `0x1A`); the opening hand shows, the CPU's face down
- [x] Player actions mapped: phases, the command map `0x12`, the input wait 4, the menu's 14 command bits and the
      client's card command (`duelRequestName` 2), confirmed live with a test build
- [x] Rules engine sidecar: EDOPro's core (ocgcore-wasm, rule MR2) in `engine/duelcore.ts` under Node, one process
      per duel, JSON lines, cards as Konami ids (4,702 of the client's 4,771 map to passcodes through YGOProDeck's
      `konami_id`; the rest look like tokens); `engine/fetch.sh` fetches the core, ProjectIgnis' scripts and database
- [x] Translator, first part: the engine drives the duel session; its draws, turns and phases become engine commands,
      the client's idle prompt a command map of the legal commands, the client's card commands its responses; the CPU
      ends its turns at once (a placeholder bot) and other prompts are answered with defaults
- [x] Card moves: the engine's MOVE becomes a card-table entry plus `0x1A`; place prompts are answered for the client
      (first free zone). Live: a Normal Summon from the hand
- [x] Phase buttons: Main Phase prompts enable BP and EP as the engine allows (command 4 mode 2 + other flags); a
      pressed button ends the phase. Live: summon, EP, the CPU's turn, our Turn 3 with BP and EP lit
- [x] Battle: the battle prompt offers the attackers (Attack) and M2/EP; damage, costs and recovery change the LP
      shown. Live: a direct attack, 8000 → 5700
- [x] A placeholder CPU: it Normal Summons (or Sets) the first monster it may, attacks with everything, never chains;
      other prompts get simple answers (first cards, yes, first option), also for the client's prompts without a window.
      Two such AIs play a full engine duel to a win (test). Live: the CPU summoned, attacked and destroyed our monster
      (LP 8000 → 7350; it went to the graveyard pile)
- [x] Field stats: a monster entering a monster zone gets its record (data commands `0x3E9`–`0x3EB`) with CARD_Prop's
      ATK/DEF, level, attribute and race; the field label and card info show them
- [x] The duel's end on screen: command 2 {result, reason} plays the win/lose/draw movie with its reason; the front
      push `0x2009` (duel data, tag 7) ends the match, so the result screen shows the record against this opponent and
      Next returns to the map. Live: "YOU LOSE", LOSER/WINNER panels, back at Kaiba Corporation
- [x] Players kept across server restarts (accounts and records in `data/`, M6)
- [x] The client's own card choices: targets, tributes, attack targets and other card selections open the card list
      (command `0x37` and its data commands) with a message from the client's duel prompts. Live: an attack target
- [x] Yes/no and effect questions: the Yes/No dialog (command `0x36` kind 2) with the card's name. Live: Marauding
      Captain's optional effect, then its Special Summon choice. Testing aid `--duel-deck ROWS`
- [x] Positions, options and chains: the position dialog (kind 8), the option list (kind 4) for options the client has
      labels for (coin calls, card types), and the chain question (kind 3, with the cards) then the card list. Live:
      Marauding Captain's Special Summon in Defence, Time Wizard's coin call, two Trap Holes chained to the CPU's summon
- [x] Declarations: types, attributes and numbers in their dialogs, card names in the card list's searchable kind.
      Live: Abyssal Designator (DARK, Fiend), Prohibition (Giant Orc), Star Blast (1000 LP)
- [x] Options that are a card's own strings: EDOPro's `code << 20 | n` paired with the prompt table's per-card labels
      (110 options of 44 cards). Live: Card Shuffle's "Your Deck" / "Opponent's Deck"
- [x] Places and chains outside timely chances, settled: the client has no place window (the server takes the first
      free zone), and EDOPro's "always chain" (asking when no card can respond) has no setting in the client
- [x] Position changes and flips on the field: command `0x1C` (CardFlipTurn) for MSG_POS_CHANGE and flip summons; the
      command ids named after Konami's view types (Master Duel's `DuelViewType`). Live: Giant Orc to defense after
      attacking, a Set Trap Hole turning face-up
- [x] Battle and summon animations: BattleInit/Attack/Run/End (`0xA`–`0xE`, figures and destruction per player)
      and RunSummon/RunSpSummon (`0x38`/`0x39`). Live: the NPC's attack cut-in, the hit, the destroyed card going,
      summon sparkles
- [x] Chain link animations: ChainSet `0x33` {player, zone, link} at each MSG_CHAINING and ChainRun `0x34` {_, _,
      link} at each MSG_CHAIN_SOLVING. Live: two Trap Holes chained to the CPU's summon showed links 1 and 2, the
      chain banner (`duel_chain.swf`), and each number fading as its link resolved
- [x] Destruction by effects: CardVanish `0x1F` {player, zone | index << 8} before the card's move, for MSG_MOVE's
      reason DESTROY without BATTLE (the sidecar reads the reason back, since ocgcore-wasm drops it). CardBreak `0x20`
      only hides a card's label. Live: Trap Hole shattered Sangan, then the duel went on
- [x] Stat changes from effects: the sidecar reports the monster zones after each engine step when they changed, and
      a face-up monster's record (`0x3E9`–`0x3EB`) goes again, which the client re-reads after every command. Live: Axe
      of Despair took Giant Orc's label and card info to ATK 3200 (in blue)
- [x] Attack targets: the original used dialogs too (its dialog texts have the target list, "Attack your opponent
      directly?" and the replay question; the client has no field target step), so the card list stays, and EDOPro's
      direct-attack and replay questions read as the client's own (225, 223). Live: Jinzo #7's attack asked "Attack
      your opponent directly?"
- [x] Counters: EDOPro's counters on a field card become `0x27` in the client's own counter kinds (the icon on
      the card's label, "Spell Counters 1" in the card info); a card leaving the field drops its counts first. Live:
      Breaker the Magical Warrior's Spell Counter
- [x] Deck piles on the field: DeckReset (`0x16`) for each deck and extra deck after the opening's LP; extra decks
      join the card table so their summons move from the pile. Live: deck piles of 34 and 35 counting down with the
      draws, and a one-card extra deck pile on both clients
- [x] Extra deck summons: commands on extra deck, Graveyard and banished cards at their pile; Synchro and Fusion
      materials one at a time in the card list. Live: Junk Synchron and Giant Orc into Ancient Fairy Dragon. Testing
      aid `--duel-extra ROWS`
- [x] NPC experience (`npcbin`) so higher levels unlock: wins, losses, draws and experience per NPC level id from
      the duel records; `need_ex` 100/200/300, the series tabs, `premium` and the gated NPCs' pbin bits. `0x17D4` only
      signals ready. Live: beating Seto Kaiba opened his level 2 and showed "Wins 1 / Losses 0"
- [x] Online 3's duel stream is Konami's engine family but renumbered (turn change 6, LP set `0xF`; docs/protocol.md)
- [x] Regulations enforced: Select Deck's deck is checked for sizes, cards owned (rentals counted) and the duel's
      regulation (`deckcheck.rs`), refused with `0x22CB`. Live (testing aid `--duel-regulation 4`): both Auto Duel
      players saw "You have been disconnected from the Duel Server." and the map; a legal deck reached the janken. A
      stock client checks first, so only a modified one gets refused. The refused player's opponent gets a non-result `0x2009` (`duel_end` 3) and leaves for the map, live
- [x] PvP on the duel connection: both matched clients share one table (`table.rs`): the janken between their hands,
      the winner's turn order, one engine with a translator per client (each sees itself as player 0), each prompt to
      the client it asks, the opponent's hidden cards without rows, a player leaving loses. Results reach both front
      connections. Live: Yugi against Kaiba, Giant Orc's direct attack, WINNER / LOSER screens, both back on the map
- [x] Players' records written back (`data/records.json`, M6)
- **Exit:** two clients play a full duel with real decks to a finish (met 2026-11-07: two headless clients matched
  through Auto Duel and played their starter decks to a win)

## ▶ M6: Play with friends

- [x] Run on a LAN or VPN: `--bind 0.0.0.0`; the gate and the front send each client to the address it reached
      (`--advertise` only for port forwarding); firewall notes (TCP 5730, 1213, 1214, 8080). Live: a client through
      the LAN address and one through loopback, two accounts, a duel to a win between them
- [x] Accounts kept in `data/accounts.json` and duel records in `data/records.json`, backed up at each start; new
      players' names made unique (Duelist, Duelist2…)
- [x] Admin commands: `accounts`, `add-player ADDRESS NAME` (name a friend's player before they connect), `rename`
- [x] Linux release bundle (`tools/release.sh`: server, engine sources, `run.sh`) and the setup guide for the host
      and the players (`docs/friends.md`: `yo3.ini`, Windows or Wine, LAN / VPN / port forwarding)
- [x] A native Windows server build: `tools/release.sh` cross-compiles `yo3-server.exe` (x86_64-pc-windows-gnu,
      mingw-w64) into a zip with `run.cmd` and `engine\fetch.cmd`. Live: under Wine, a client logged in through it
- [x] Chain questions follow the player's Activation Message option (Manual / Semi Auto Cancel / Auto Cancel): the
      server sends a level per chance (3 after an event, 1 at a phase, 0 when nothing fits). Live: no question at the
      CPU's phases, the question at its summon
- **Exit:** friends connect from their own machines and duel (the network path is verified on one machine: a client
  through the LAN address against one through loopback, 2026-11-07; real friends' machines not tried yet)

## Open questions and guesses (revisit)

| Item | Current choice | Why it's uncertain |
|---|---|---|
| `pbin` size | 512 zero bytes | Guessed from the 4,096 entries of `yo3pbininfo.bin` |
| Starter deck and collection | the first preset of type 1 that regulation 1 allows (preset 2); one copy of each of its cards | The original let new players choose; a real collection comes with the economy |
| Trunk (`9000`) contents | one record: `cards` (the starter deck's), `noTrade` all zero | Other fields later screens may read are unknown |
| `regulations` list | all 20 ids | Which banlists the UI should offer |
| `0x21FC` `lang` | `"1"` | Read as a string; semantics unknown |
| `0x271A/0x2AFB/0x2AFD/0x27D8` | minimal replies | Meanings unknown; they only stop timeouts |
| `avatar1` length and slots | 41 zero entries (a default male avatar) | Slot meanings unmapped |
| `bgm`, `icon`/`lefttime`/`maintenance`, `npc_id`, `cpuUse`/`notRelease` | left out (read as 0 / NULL) | The tracer shows the client reads them; see "Fields the client reads that we leave out" in docs/protocol.md |
| Chat room `region`, `status`, `type`; `0x2458.lang` | 0 / 1 | Shown fine; meanings unknown |
| Whisper `chat_type` values | any type with `to_name` is a whisper | Only room chat (type 1) was seen live |
| Player search paging | skip `base_point`, return at most `front_num` | Only page 1 (`0, 100`) was seen |
| Player search by name | case-insensitive substring | Konami's matching rule is unknown |
| Friend and black list pushes | sent once, right after `9000` | The original timing is unknown; the client only needs them before it lists a player |
| Shop lists besides packs, `perk_num`, rental terms | all empty or 0 | Only booster packs are stocked; `perk_num`'s meaning is unknown |
| Pack prices and contents | 100 BP a pack, at most 10 per purchase; 5 cards: 4 of the pack list's first ascending run, 1 of a later run (each run half as likely as the one before), read as rarity tiers | The original's prices, pack size and odds are unknown; the runs match the OCG's rarity split (44/18/9/9) |
| Starting BP | 5000 per account (`START_BP`) | The original's amounts are unknown |
| Structure decks | SD001–SD005 only (the starters 1–3 have no art); 500 BP each, at most 3 per purchase, sale id 20000 + deck id; the owned-items bit (`pbin_no`) the client sets isn't kept by the server | The original's prices are unknown; nothing read so far depends on the bit for decks |
| BP from duels | win 100, draw 50, loss 30 per counted result (NPC duels and duels between players; a match counts once) | The original's duels paid no BP (no reply after a duel sets it); the client shows the new total at its next login or purchase |
| Pack presents | one per pack, `present_id` counting from 1 per session, `level`/`form` 0, `item_img` and `end_time` empty; the cards join the collection at purchase | Only the fields the window reads; presents aren't kept across reconnects (the cards are) |
| Event lists (`0x177B`), own user event (`0x33FE`) | official events: none; user events as created; paging by `base_point`/`front_num` like player search | No official events are held |
| User events | a knockout paired in entry order; lobby status 0/4/6/7/8 as the round goes; a 20 s wait between rounds; the bracket's blocks above 8 seats built the same way (not seen live); a restart ends a running event | How the original drew and ran its tournaments is unknown (the statuses and texts are the client's) |
| Rental records push (`0x2D6C`) | sent after `9000`, when a rental starts and when one ends (only the rentals not over) | The original timing is unknown; the shop only needs it before it opens |
| Card rental | the ready request `0x2C3E` is answered `0x2D68`; error codes 504 (cards) and 503 (no scene, wrong seat) as the trade's; terms 1, 3, 7, 14 days (the dialog's); a No keeps the scene; `trade_result` 0 for it; a rental ends exactly at its term, checked each second; a lender can't put up cards they have lent | The original's reply id for ready, its error codes and its rules (rental list limits, early return) are unknown |
| Country | the account's setting, else GB/JP/DE/FR/IT/ES/KR/CN from the login's language | The original took the player's country from their registration; ours has none |
| NPC roster `npc_type`, `need_ex` | tabs by series (NPCs 0–8, 9–16, 17–24); `need_ex` 100/200/300 | No client file gives the tabs or the experience each level needs; `need_ex` can't all be 0 (`0x4BFDC4`) |
| Teams | `level` 1 and scores 0 for every team; handing over the leadership makes the old leader a member; the founding time in Japan time | No team scores or levels exist without the original's events |
| Search status bits | `0x1000` duelling, `0x800` hosting, `0x200` hosting Single, `0x400` hosting Match | Read from the Detailed Search texts |
| Manner level | the average of a player's ratings (-10..10) mapped onto 0–4, (average + 10) / 5 rounded; 2 with no ratings; `evaltotal` the sum, 0 when negative | The original's thresholds and `evaltotal`'s sign are unknown |
| Trade end | a refused trade sends `0x2BCA {trade_result 0}`; the trunk is pushed after a trade | The original's reply to a No, and how it refreshed the trunk, are unknown |
| Item prices | protectors 200 BP, avatar items 100 BP, one of each | The original prices are unknown |
| Avatar items for sale | the catalogue's parts with sub 0 (178 male, 148 female) | Read as "to acquire" next to the default (2) and starting (1) parts |
| Wins and losses | My Profile and Player Information count duels against players only, not NPCs | The original's counting is unknown |
| NPC experience | 100 a win, 50 a draw, 10 a loss, per NPC level id | No request reports it; the original's amounts are unknown |
| `premium` | 1 for every account (all NPCs, Form Team, Create Event) | The original sold it (Duelist Premium); a friends' server gives it to all |
| `join_id`, `duel_id` | server-wide counters from 1 | The client only echoes them back |
| `0x17D4` | answered `0x17D5`, nothing kept | Sent once the duel object is ready; may also carry results later |
| `0x22C4` push | right after `0x17D5`; `rule` = the fields of the duel's `0x17AC` item | The original timing and rule contents are unknown; only `match_type` is read so far |
| Server duel messages | request `0x200D` | The pump reads only `duelRequestName` (and needs the id non-negative) |
| CPU janken | random hand; the CPU goes first when it wins | The original CPU's choices are unknown |
| Duel setup `PlayerType001/002` | 0 (client) / 1 (CPU) | Meaning unknown |
| Card table entries (1000) | param3 0, flag byte 0; uids 1.. in table order | Meanings of param3 and the flag unknown |
| Hidden cards | against another player, the opponent's deck, hand, extra deck and face-down cards go with row 0 and face-down monsters get no field record; against the CPU, real rows | Online 2 used placeholders, whose value is unknown; a hidden card that flips without moving keeps row 0 until it moves (position changes aren't translated yet) |
| Automatic matching | the first two players waiting for the same `match_type`, Both (6) with either and two Both players in a Single duel; the level choice is ignored; the second player's `0x1978` goes a second after its `0x1799` | Konami's matching rules are unknown; the delay assumes the agent reads one message per step |
| Match Mode | best of three; every duel opens with a janken; a drawn duel counts for neither; the records count the match once | Online 3's rules for who goes first in later duels, draws and the side deck are unknown |
| Hosted LP and `--duel-lp` | the testing aid wins over a hosted `lp_start` when it isn't 8000 | A testing convenience, not the original's |
| Duel Menu counts | `duel_playing` = 2 × matched duels, `duel_ready` = hosted duels, `duel_auto` = players waiting | Whether they count players or duels is unknown |
| Hosted duels | `duel_status` 1 while hosting, 3 while in a duel between players; Team-only duels take nobody (no teams); `0x17C0` answered `0x17C1` | The original values and the limits' rules are unknown |
| PvP setup | `PlayerType001/002` 0/1 as against the CPU | Their meaning is unknown |
| A player leaving | the other wins (command 2 {1, 14 "disconnected"}) and both records count it | The original's handling (time-outs, `0x2724` heartbeats) is unknown |
| Menu bits for Set | 6 monster set, 7 Spell/Trap set | Both entries read "Set" |
| Card list kind | 1 for min 0, `0x27` for min 1, else 6 with max = min | The kinds' other meanings (title, sounds) are unknown; a choice between 2 and more picks is narrowed to exactly min |
| Card list marks | `0x8000` for the CPU's cards, no icons | The low bits draw icons whose meaning is unknown |
| Choice messages | EDOPro's hint mapped to the client's text for 7 hints; none otherwise | The original server's message per choice is unknown |
| Yes/no questions | 'Activate the effect of "X"?' for optional effects, 'Use the effect of "X"?' for other questions | The cards' own question strings (EDOPro's) aren't in the client |
| Position dialog | button 0 face-up attack (face-down if only that), button 1 face-up defense when allowed, else face-down; the flag shows the face-up defense picture | The dialog has two buttons; ocgcore allows any mix of four positions |
| ChainRun `param1`/`param2`, ChainSet's place for a card off the field map | 0; the field's centre (16) | DuelChain reads neither; the original's values are unknown |
| Destruction by effects | CardVanish (`0x1F`, the shatter) before the move; CardBreak (`0x20`) never sent | Which of the two the original sent, and when, is unknown; CardVanish is the one with an effect |
| MSG_MOVE's reason | read back by a query at the card's new place after each processing step | A card that moves again within the same step reports its later reason |
| Stat changes | level, ATK, DEF, base ATK, base DEF from the engine; race and attribute stay CARD_Prop's | The client's race and attribute numbering isn't ocgcore's; type-changing effects are rare in the card pool |
| Duel clocks | turn clock full again at each of the player's turns, both clocks running while the engine waits on that player (also during client animations), a time-out 5 s after 0; Auto Duel 180 s / 600 s, hosted duels their `time_turn` / `time_total` (0: off), NPC duels none | The original's clock rules and Auto Duel's limits are unknown; the clocks follow Create Duel's own defaults |
| Counter kinds | EDOPro ids mapped by name to the client's kinds (texts 200 + kind); Spellstone as "Magical Stone", Des as "Death", unnamed ones as 38 "Counter"; Mechanical (23) unused | The original's kind per card is unknown; EDOPro's and Konami's names differ for a few |
| Card Swap | offered (1504) only to a player whose deck has a side deck; no time limit (the clock slot 4 the screen counts down also drives the janken's); a swapped deck with other cards is refused | Who the original offered it to, and its time limit, are unknown |
| Friends | a request needs the other player online with no request waiting for them; no limit on friends (110 "The Player List is full." never sent); declining tells the requester; the team tab lists nobody | The original's limits and offline requests are unknown |
| Watching | from seat 0's side (the first player to enter); the decisive janken round only; NPC duels can't be watched | The original's spectator view and opening are unknown |
| Login banlist | regulation 3 ("unlimited", no banlist; the user's choice) unless `--regulation` names another | Which list the original used for Auto Duel at each date is unknown; the client calls the login one "Latest Regulation" |
| Declarations | several types at once and numbers outside the number dialog (min or step above 15 units, max above 255) answered automatically; the CPU names the duel's cards in turn | The client's dialogs pick one type and a range of numbers; the original server's handling of the rest is unknown |
| Material choice | Synchro and Fusion materials one at a time in the card list (EDOPro's order: the Tuner first); Fusion materials under "Select monsters to use as Fusion Material." (92), Synchro materials without a message | The client has no Synchro material text; how the original asked is unknown |
| Option labels | EDOPro's system strings 60/61 and 70–72 mapped to the client's labels; a card's own strings paired by hand with the prompt table's per-card labels (`card_options.rs`), else the first option | The original server's label per card option is unknown |
| Chain chances | level 3 after an event (summon, attack, damage… or during a chain), 1 at a phase or step, 0 when no card fits (`spe_count` 0); the player's Activation Message option decides; messages 7/8 (Spell/Trap zone)/269 | The original server's levels and which windows it judged "low priority" are unknown |
| Rules | EDOPro's `MODE_MR2` | Online 3 (2010) ran Master Rule 2; EDOPro's preset may differ in details |
| CPU play | first summonable monster, attack with all, never chain | The original NPCs' AI is unknown |
| Duel end reason | EDOPro's reason mapped to the client's texts (LP 0, deck out, surrender, Exodia...; others "won with a card") | The original server's values are unknown (100 takes another path) |
| Duel's end push `0x2009` | sent when the engine ends the duel; `duel_end` 1; no `score_<id>`, no `dlogURL` | The original timing and `duel_end` values are unknown (2 and 3 show an abnormal-end dialog) |
| Result screen counts | `win`/`lose`/`draw` = totals against this opponent including this duel, `*_add` = this duel's change; kept in memory | The client only shows them ("Wins 1 (+001)") |
| Duel port | 1214 | The original's port is unknown; the client takes whatever `duelData` says |
| Accounts | one per client address | The client sends no credentials; friends behind one address share an account |
| Uploaded decks | the decks a duel's Select Deck sent, ten per player by name, listed after the starter and bought decks | The client has no other upload; the original server's deck storage is unknown |
| Deck check | sizes, ownership per Konami id, the duel's regulation, refused with `0x22CB` | The client's own refusal reply for a deck is unknown; this one only shows text `0x2919` |
| Player names | the starting names (`--players`, default Duelist) made unique with a number; players make more at Select Player (at most 16 characters, unique ignoring case: **guess**), the host can still `add-player` / `rename` | The original's name rules are unknown; a team's leader can't be deleted and an account keeps one player (ours) |

## Leads (outside the code)

- Old Online 3 duel logs (`players\<name>\duellogs\`) would be official duel recordings.
- The DuelEvolution Discord reportedly has Online 3 traffic captures (deferred).
- archive.org's `yo3setup_20110728.exe` is three weeks newer than our client; check its protocol version.
