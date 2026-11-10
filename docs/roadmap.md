# Roadmap

Goal: play Yu-Gi-Oh! ONLINE 3 with friends on our own server. Each milestone ends in something checked against the
real client (`tools/client/vclient.sh`), not just against tests. Tick boxes in the same commit as the work.

**Contents**

- [Status](#status)
- [▶ Overnight plan (2026-11-07): everything left, end to end](#-overnight-plan-2026-11-07-everything-left-end-to-end)
- [▶ Windows VM test (2026-11-08): what a real Windows client turned up](#-windows-vm-test-2026-11-08-what-a-real-windows-client-turned-up)
- [▶ Robustness review (2026-11-08): what three code reviews found, and the fixes](#-robustness-review-2026-11-08-what-three-code-reviews-found-and-the-fixes)
- [▶ Monkey testing (2026-11-08): two monkeys, what they covered and found](#-monkey-testing-2026-11-08-two-monkeys-what-they-covered-and-found)
- [✅ Screenshot session (2026-11-08): two things seen playing NPC duels](#-screenshot-session-2026-11-08-two-things-seen-playing-npc-duels)
- [✅ M0: Research and tooling](#-m0-research-and-tooling)
- [✅ M1: Stable world](#-m1-stable-world)
- [✅ M2: An instrumented client](#-m2-an-instrumented-client)
- [▶ M3: A social world](#-m3-a-social-world)
- [M4: Accounts, collection and economy](#m4-accounts-collection-and-economy)
- [✅ M5: Duels](#-m5-duels)
- [▶ M6: Play with friends](#-m6-play-with-friends)
- [Open questions and guesses (revisit)](#open-questions-and-guesses-revisit)
  - [Settled by the client's own code (no longer guesses)](#settled-by-the-clients-own-code-no-longer-guesses)
- [Leads (outside the code)](#leads-outside-the-code)

## Status

Review fixes (2026-11-10), each with a failing regression test before the implementation:

- [x] Trade confirmations belong to the current offers: an offer change clears both approvals, and final answers
      before both Ready marks are refused. Tests: `trade_contract` `changing_an_offer_withdraws_every_confirmation`,
      `a_final_answer_before_both_ready_marks_is_refused`. Live TCP: the stale answer was refused, both collections
      stayed unchanged, then two fresh approvals executed the changed offer once.
- [x] A refused player deletion leaves friends, ignores, waiting mail, rentals and team membership intact. Eligibility
      is checked against the current accounts before cleanup. Tests: `player_contract`
      `a_refused_last_player_deletion_preserves_their_social_data`, `a_team_leader_cannot_be_deleted_even_with_another_player`.
      Live TCP: the last-player deletion was refused and fresh friend, ignore, mail and rental replies were unchanged.
- [x] Key-only accounts use the same saved collections as address accounts: trades complete, rental offers are
      checked for ownership, and trunk requests refresh cards changed by another session. Three red-to-green tests
      in `trade_contract` and `rental_contract`. Live TCP: two `/join` accounts traded, an older session refreshed its
      trunk, and a rental refused an unowned card before accepting an owned one.

As of 2026-11-08 the server covers every feature the client uses: M0 to M2 (research, a stable world, an instrumented
client) and M5 (duels) are done, M3 (social) and M6 (play with friends) are done apart from checking on real friends'
machines, and M4 (accounts, collection and economy) has one open item, the deck list's own save. Two duel problems found in the screenshot session are settled (one fixed, one hardened and watched). What is left is
tracked as `[ ]` boxes in the sections below and in "Open questions and guesses".

| Where | What it is |
|---|---|
| The dated sections (overnight plan, Windows VM test, robustness review, monkey testing, screenshot session, duel check) | What each work session did and found |
| M0 to M6 | The milestones, each ending in something checked against the real client |
| Open questions and guesses | Every value we picked without client evidence, to revisit |
| Leads | Ideas outside the code |

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
- [x] When the rules engine can't run (Node.js missing or older than 22.18, engine not fetched), the server logs an ERROR
  at start, the login news and the join page say duels are unavailable, and the launchers (`run.sh`, `run.cmd`,
  `engine/fetch.sh`) stop before starting a server whose duels would all end as draws.
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
- [x] A monster whose position an effect changes right after its summon (Stumbling) stayed upright on its owner's
      screen. Reproduced live on the VM round (2026-11-08, Yugi headless against the VM): Goblin Elite Attack Force and
      Giant Orc, summoned under the VM player's Stumbling, stood upright on Yugi's screen (and stayed so through later
      changes: Enemy Controller to attack, Goblin's own change to defense) while the VM showed them right. Their batch
      was the summon move (attack), RunSummon `0x38`, ChainSet, ChainRun and `0x1C` to defense, in one burst; `0x1C`
      waits only for the card manager, so it ran while the summoner's summon effect played (`0x4FB7F0` keeps a pending
      effect mode per card, `+0xDC`). Now, within a batch, the summon move carries the position its card turns to
      (`settle_positions`), so no turn overlaps the effect. Test: `a_monster_turned_right_after_its_summon_lands_in_its_new_position`.
      Then the same round showed more: the VM player's own monsters (Lily, Reflect Bounder, Giant Orc) turned by their
      own Stumbling stayed upright on the VM too, even with a question splitting the batch, while Yugi saw them turn.
      So a player's own summoned monster ignores in-place turns on its owner's screen, whatever the timing; a card that
      arrives by a move is drawn in its position. A monster's turn now also moves it onto its own slot in the new
      position (with its row) before the 0x1C. Test: `a_monster_that_changes_position_moves_onto_its_own_slot`.
      Live check pending.
- [x] The summon effect played at the leftmost monster zone, not the summoned monster's: RunSummon / RunSpSummon's
      first value is the card's place (`player | zone << 1`, `0x514E5A`), not only the player.
- [x] The swords over the possible attackers stayed after a Battle Phase with no attack: BattleEnd now goes whenever a
      Battle Phase ends (`0x4F8E40` takes them down).
- [x] A direct attack showed no sword (the user, VM round): BattleAttack's target zone `0xFFFFFFFF` plays nothing, while
      any zone from 5 plays the direct cut-in and the strike `0xCB` (`0x4F8430` state 3). Direct attacks now go at the
      defending player, zone 16. Live check pending; an attack on a monster plays the cut-in and strike already.
- [ ] Card preview on hover needs a held button on the VM (the client shows it on hover under Wine): likely the VM's
      PS/2 mouse or window focus, not the server.

## ▶ Robustness review (2026-11-08): what three code reviews found, and the fixes

Three reviews of the server looked for what one client (or one bug) could do to everyone. Each item was checked against
the code first, then fixed test first (`crates/yo3-server/tests/robustness_contract.rs` and the unit tests beside the
code); the evidence is the test named in each line.

P0, crashes and stalls:
- [x] 1 Nesting depth: `Message::decode` stops at 16 levels (`Error::TooDeep`); one 1 MB frame of 200,000 nested
      messages overflowed the stack and aborted the process before login. Tests: `codec.rs`
      `deeply_nested_messages_are_refused_not_a_stack_overflow`, `robustness_contract` `wire::a_crafted_…`. Live: the
      crafted frame sent to the running server, which went on answering a login.
- [x] 2 Poisoned locks: every hub, table and directory lock goes through `lock.rs` (`Locked::locked`, recovering from a
      panic). The clock pass runs in `spawn_blocking` (a panic in a pass is logged, not the end of the clock) and a table
      that panicked, under its lock or in its tick, ends its duel as a draw and goes. Tests: `duel.rs`
      `a_poisoned_hub_…`, `a_table_that_panicked_…`.
- [x] 3 Engine failure: when the sidecar dies, hangs or can't start, both seats (and the NPC duel's player) get a draw
      end (command 2 {3, 0}), the match is reported once as a draw and the engine is dropped. Tests:
      `an_engine_that_dies_mid_duel_…`, `…cannot_open_the_duel…`, `…in_an_npc_duel…` (a proxy sidecar that dies on
      cue).
- [x] 4 Blocking: engine calls time out after 30 s (`engine.rs`, a reader thread; the hung sidecar is killed, then 3
      applies); connection handlers run under `block_in_place`, the clock on a blocking thread, and a table whose lock
      is busy is skipped (`try_lock`). Tests: `engine.rs` `a_sidecar_that_never_answers_…`, `duel.rs`
      `a_table_that_is_busy_is_skipped_…`.
- [x] 5 Prompts with no answer: `MSG_SELECT_SUM` is answered (the smallest set of cards that makes the sum, exact or
      "at least with none to spare", counting a card's two levels; the client's card list for it is not made: it takes the
      automatic answer), and a client's card name declaration with no dialog is answered; the other prompt types all
      had answers. A prompt that comes back identical 8 times, or more than 10,000 answers between two client choices,
      ends the duel as in 3. Tests: `translate_contract` `a_sum_prompt_is_answered_…`, `robustness_contract`
      `a_prompt_the_core_keeps_refusing_…`, `the_same_goes_for_an_npc_duel`.
- [x] 6 A player leaving before the engine started (janken, turn order, deck select, between a match's duels after a
      draw) now sends the other seat command 2 {1 won, 14 disconnected} and its result screen (`0x2009`) with the records
      unchanged; only a duel that began is counted. Tests: `a_player_leaving_before_the_duel_started_…`,
      `…in_the_turn_order_choice_…`.
- [x] 7 A duel connection arriving after its match ended gets no table: it is told (command 2) and closed
      (`Hub::ended`). Test: `a_second_player_arriving_after_the_first_left_…`.
- [x] 8 `Hub::leave` ends a match of the leaving player that has no table (the other player is sent out as at a
      refused deck, `0x2009` duel_end 3, and an event's round goes on with them as the winner). Test:
      `a_matched_player_whose_client_dies_before_the_duel_frees_the_match`. Not done: removing an offline entrant from an
      event's list on leave (the round start already forfeits a player who is gone).
- [x] 9 The turn order (0xD) is taken once per janken, at the table and against the CPU (a second 0xD, or a janken
      after it, opened another node process each time). Test: `the_turn_order_is_taken_once`.

P1, data integrity and stuck state:
- [x] 10 A side file that doesn't parse (friends, mail, teams, rentals, records, events) is renamed
      `NAME.corrupt-<time>` and logged as an error, and the server starts empty; the start-up backup skips files that don't
      parse (so repeated bad starts never rotate the last good copy out). `accounts.json` still stops the start with the
      parse error, as before. Tests: `store.rs`, `a_corrupt_side_file_is_kept_aside_…`.
- [x] 11 `next_player` and `next_user` are saved with the accounts; deleting a player also clears them from the friend,
      ignore and request lists, waiting mail, rentals (as lender or borrower), team invites and the team. Tests:
      `accounts.rs` `player_ids_are_never_reused_…`, `player_contract` `a_deleted_player_leaves_the_friends_…`.
- [x] 12 Team invitations (made and received) end when the player disconnects. Test: `team_contract`
      `a_team_invitation_ends_with_a_player_who_disconnects`.
- [x] 13 Trade and rental: a player in either scene can't invite or be invited, an acceptance refuses (a "no" to the
      inviter, `0x2BDC`/`0x2D55`) when either side is in a scene now, and ending a scene removes every one of the
      player's. Tests in `trade_contract` and `rental_contract`.
- [x] 14 A trade's four updates are one `Directory::update_players` (one save, all or none). Test: `accounts.rs`
      `several_players_change_together_or_not_at_all`.
- [x] 15 A purchase debits the stored BP under the directory lock (`Directory::charge`). Test: `accounts.rs`
      `a_charge_takes_from_the_stored_bp_…` (the race itself can't be made in a single thread).
- [x] 16 The engine is dropped when a duel is reported, abandoned, left or timed out (also covered by 3 and 6).

P1, network hygiene:
- [x] 17 Accept loops back off (50 ms doubling to 2 s) after an error; a connection silent for 180 s is closed (the
      client heartbeats every 30 s; `Config::idle_timeout`) and so is one that stalls inside a frame; a frame's buffer
      grows 16 KiB at a time as its bytes arrive (at most the 1 MiB frame cap per connection). The HTTP endpoint gives
      a request line 10 s. Test: `wire::a_connection_that_goes_silent_…`. Not done: a limit on the number of connections.
- [x] 18 The packet log writes at most 2048 bytes of a packet in hex (plus `raw_len`), 256 characters of a text, 64
      fields and 32 items per message. Test: `log.rs` `a_huge_packet_is_logged_in_part`.
- [x] 19 `0x1F4F`: any update that names a duel passes the deck check; update_type 1 is refused as before, any other is
      ignored (nothing kept, answer `0x22CA`) so a Card Swap can't bring a deck the check refuses (it only takes a deck of
      the same cards in the match anyway). Each deck list is cut at 100 rows and a deck name at 64 bytes. Tests:
      `regulation_contract` `the_deck_check_applies_to_every_update_that_names_a_duel`, `a_decks_row_counts_are_capped`.
- [x] 20 A failed `0x1F4C` takes the old registration offline, and until a player is selected only the login and
      Select Player requests (`0x1F47`-`0x1F4C`) and the heartbeat are answered. Test:
      `a_failed_reselect_drops_the_old_registration_…`.
- [x] 21 A deck entry remembers the address of the front connection that brought it and a duel connection from another
      address that names the player is closed (`DuelSession::with_peer`). Test: `a_duel_entry_is_claimed_only_from_…`.
      Friends behind one NAT share an address, but each entry still belongs to its own player id.
- [x] 22 Caps: an avatar blob 256 bytes (else `0x2CF6`), speech lines and comments 200 bytes, a mail's title 128 and
      body 2048, a form's fields (user event, hosted duel, NPC duel rule) 64 fields of 256 bytes, one unfinished event per
      host (`0x33F5` for a second) and 100 on the server, one hosted duel per player, 32 rule sets per session, 256 card
      list picks. Tests: `what_a_client_can_make_the_server_keep_is_capped`, `mail_contract`,
      `translate.rs` `the_picks_of_a_card_list_are_capped`.
- [x] 23 `price * count` is `checked_mul` (refused with `0x1C25`); the language and manner level shifts in player search
      are `checked_shr`. Tests: `shop_contract`, `search_contract`.

Skipped as the review allowed: mail deleted before the client fetched it; accounts made for any address (how friends
join). Live (release build, two headless clients, scratch data directory): the crafted nested frame left the server
answering; an Auto Duel between the two clients went from the match through Select Deck and the janken to the result
screens and the evaluation dialog (Chiron's direct attack at 1000 LP); a trade between two more accounts (raw protocol
from `127.0.0.2` and `127.0.0.3`, the real clients being busy in the duel) invited, accepted, refused a second scene
for the inviter (`0x2BD8` 106), both OK, both yes, `0x2BCA` and the trunks. Not done live: the trade scene on the real
client (unchanged by this work), a sidecar that really hangs (covered by the engine test).

## ▶ Monkey testing (2026-11-08): two monkeys, what they covered and found

Tools: `crates/yo3-monkey` (protocol monkey, run through `tools/monkey/protocol-monkey.sh`) and
`tools/monkey/ui-monkey.py` (the two real headless clients); how to run them is in CLAUDE.md (Commands).

Protocol monkey (a supervised server on a scratch data dir; plans from a seed and the packet logs' real requests,
replayed in order and shuffled, fields mutated, random and garbage frames, drips, reconnects, plans as the clients of
four accounts, floods, HTTP junk; a canary logging in and listing players; rss/fds/threads/data-dir sampling; panic and
ERROR scan; Auto Duel pairs and NPC duels played with random commands, picks, dialog results, phase buttons, leaving at
random moments; SIGKILL and restart on the same data):
- fuzz seeds 1-1500: 49,464 messages; http-only 400 plans; 1500 client addresses (`addrs`); a flood of 400 connections
  every 12 s in every soak.
- soaks (24 or 20 connections plus duel pairs): 300 s x2 (about 0.9 M messages each), 600 s multi-account (79,423 plans,
  3.1 M messages, 99 duel pairs, 58 duel results), 2400 s (131,160 plans, 5.1 M messages, 237 duel pairs, 81 results),
  180 s with 7 SIGKILLs (1.2 M messages). Duel-only: 40 pairs (71 duel connections, 54 began, 33 results, 4,207 random
  actions). Peak server rss 205 MB, 455 fds, which fall back to 14 fds; canary p99 under 175 ms (a 24-connection load).
- Result: no panic, no ERROR, no death, no stall, no fd leak, no corrupt data file after crashes, clean restarts.

UI monkey: 40 min (seed 3): 4,845 actions (clicks, random clicks, right clicks, drags, keys, typing), 6 Auto Duel rounds,
12 duel connections, 8 duel results, clients killed mid-duel and logged in again; no crash, no dialog, no server
trouble, no stall. A second pass of 20 min (seed 4): 2,249 actions, 3 Auto Duel rounds, 6 duel results, no failure. The
Documents folder was restored and verified identical after each run.

Findings:
- [x] 1 The packet log grew without limit (2.6 GB in five minutes of a client that only sends). It now rotates at
      128 MB and keeps four files per run. Test: `log.rs` `the_packet_log_rotates_and_keeps_only_its_newest_files`.
- Monkey-side, not server bugs: Delete Player and Create Player (`0x1F4A/B`) changed the account the canary logged in
      as (a deleted Kaiba: no reply to `0x1F4C`, as designed), so the monkey leaves them out; a canary refused while the
      monkey itself kills the server is expected.
- Open, by design: one account per client address means new addresses keep making accounts (about 2 KB of file and 40 KB
  of memory each; 1500 addresses were fine). Only a spoofing client could do this over TCP. Rotation is per run: a
  server restarted often keeps four files each time.
- Open, client: with the server frozen (SIGSTOP) the client goes to a black screen and shows no "no response" dialog
  within 60 s; the UI monkey's OCR check for that dialog was not exercised live because the server answered every
  request. OCR itself reads the client's text (tested on the Duel Menu).
- Real-client requests the server answers with nothing: duel `0x200A` (the first seat waits for the second, by design)
  and `0x200C` (duel messages that only produce commands). Nothing else was UNHANDLED.

## ✅ Screenshot session (2026-11-08): two things seen playing NPC duels

Found while playing Kaiba Corporation duels on the headless client for the README's screenshots (Elemental Heroes
and Miracle Magic official decks). Both are traced below.

- [x] Magical Hats (a Trap, chained to an attack) "stalled" the duel: after "Yes", the target pick works, then a
      list of 17 Spell/Traps shows; picking one only marked it and no OK button showed. Traced live (2026-11-08):
      the engine asks MSG_SELECT_CARD min 2 max 2, the list is right (kind 6, max 2) and the second pick decides at
      once; the hint never says two picks are needed. Fixed by naming the count on every list that needs two or more
      picks ("Select 2 card(s)."). See "Card lists" in the protocol notes.
- [x] A card's art went wrong once (Mystical Space Typhoon with Call of the Haunted, then a Normal Summon of
      Elemental Hero Sparkman showing the Typhoon's art). Not reproduced: the move streams of this chain replay
      consistently in a contract test and the packet logs of the session show no move whose row differs from its
      table entry. One way the mirror can go wrong was closed anyway: a card leaving a hand, Graveyard or banished
      pile was taken at the engine's sequence without checking its id; it is now taken by its id when another card
      sits there (a warning logs "mirror out of step" if it ever happens). If the art goes wrong again, look for
      that line in the server log.

## ▶ Duel check (2026-11-09): the client's view checked against the engine

After the client crash at `0x516185` (an engine hand shuffle the server didn't follow put the client's hand out of step,
and a card left it by the wrong index; see the crash history in
[client-internals](client-internals.md#crash-history-and-the-rule-each-one-taught)), a program now looks for that class
of bug instead of a list of cases:

- **A model of the client** (`crates/yo3-server/src/duelcheck.rs`): the hand lists, field slots, pile counts, card
  table and each card's place word, as DuelCardManager keeps them after the commands the server sends, built only from
  the client's code (each rule cites its address; see "Moves out of a hand go by index" and the paragraphs after it in
  [protocol](protocol.md)). It notes the client going wrong as it happens (a hand card taken out by an index holding
  another card, a HandShuffle with an empty index) and compares every hand, slot and pile with the engine's real field
  (the sidecar's read-only `state` op) after each engine step.
- **Self-play** (`duelcheck::selfplay`, `tests/duel_check.rs`): two seeded random players (any summon, Set, activation,
  attack, chain, pick or answer the engine offers) duel through the engine and a translator, decks drawn from a pool of
  searches, discards, hand and deck churn, control changes, swaps, flips, hats, tokens, Synchro and Xyz Summons; one
  duel in three as against a player (the opponent's hidden cards without rows), every fourth starting at 2000 LP. 240
  duels run in `cargo nextest run --workspace` (`YO3_CHECK_DUELS=n` per test for more); 400 more seeds were played
  while fixing. `tests/duelcheck_model.rs` checks the model alone.
- **In the server** (`--duel-check`, a testing aid): every NPC duel and table checks each step and logs `ERROR duel
  check` with the difference (once per duel). The protocol monkey's `duel` and `soak` runs start the server with it.
  Proved by a server built with a translator broken on purpose (the opponent's draws dropped): ERRORs in every table and
  NPC duel, reported by the monkey as failures.

What it found (each fixed with a contract test in `translate_contract.rs`, then none in 400 seeds):
- [x] Magical Hats (MSG_SHUFFLE_SET_CARD 36) ignored: the client showed the cards under the wrong hats. ocgcore-wasm also
      misreads the message; the sidecar now reads the shuffled zones back. CardSwap `0x1B` trades the cards.
- [x] Creature Swap (MSG_SWAP 55) ignored: each monster stayed on its old side. CardSwap `0x1B`; **confirmed live**.
- [x] A token whose control changed (Snatch Steal on a Sheep Token) vanished instead of moving.
- [x] Xyz materials read as moving into the Extra Deck (ocgcore-wasm gives the Xyz Monster's location), and a detached
      one as leaving its monster's zone (which would clear the monster's slot on the client). Now off the field with
      CardVanish and moved on from an empty slot (shape; see "Xyz materials" in protocol).
- [x] Exchange of the Spirit (MSG_SWAP_GRAVE_DECK 35) ignored: both piles kept their old counts.
- [x] (Before the checker, the crash itself: MSG_SHUFFLE_HAND 33 ignored, and moves out of a hand named the engine's
      index; fixed in ac0afd8.)

The engine message types the translator leaves alone, and why (the audit):
- No card changes place: hints, waits, relations (CREATE/RELEASE_RELATION; targets and equips are followed, as hover
  markers), coin and dice, chain negations and disables, field zones disabled, MSG_SET (the move that follows does the
  work), RPS results, AI names.
- Only the Deck's or Extra Deck's order changes, which the client never sees (the Deck is a count; cards leaving the
  Deck or Extra Deck are found by id): SHUFFLE_DECK 32, REFRESH_DECK 34, REVERSE_DECK 37 and DECK_TOP 38 (Convulsion of
  Nature: the face-up Deck's top card isn't shown, **open**), SHUFFLE_EXTRA 39, CONFIRM_DECKTOP 30 and
  CONFIRM_EXTRATOP 42 (reveals not shown, **open**).
- [ ] Places change, not followed (never seen in 640 self-played duels with this pool): REMOVE_CARDS 190 (cards taken
      out of the duel), RELOAD_FIELD 162 (the whole field anew), TAG_SWAP 161 (tag duels, not played here), an Xyz
      material attached from a hand or pile (logged).
- [x] Equips and lasting targets (EQUIP 93, CARD_TARGET 96, CANCEL_TARGET 97) shown as hover markers between the
      cards (1017, see "Card relations" in protocol); the duel check holds them to occupied slots.
- [ ] Hovering a Spell/Trap shows no markers (hovering the monster does): its hovered place isn't the key sent.
- [ ] Not seen live: Magical Hats, Exchange of the Spirit and Xyz materials on the client (Creature Swap was). The
      client lacks the Xyz Monsters tried, so Xyz may not come up at all.

## ✅ M0: Research and tooling

- [x] Feasibility study ([research.md](research.md))
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
- [x] Accounts by client address (the client never sends credentials), up to four players each; since then also by key
      (see M4 "Real accounts")
- [x] Self-service join page (`http://HOST:8080/join`, `--no-join` to turn it off): a friend picks a player name and gets
      a key account with a downloadable `yo3.ini`; a repeat visit shows the same key, an address-only account can be
      kept with a key; at most 4 accounts per address and 20 an hour (a guess). Tests: `tests/join.rs`.
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

- [x] Real accounts, through the patch URL. The login path has no id or password field: `0x1F47` is bare, `0x1F48`
      carries only `lang`, the opener only the version string, and Delete Player's "Enter your Password" prompt sends
      nothing (`0x1F4B {player_id}`, live packet log; no call of the connection's `+0x118`/`+0x134` copies is
      serialized). But the client fetches `PatchUpdateUrl` + `yo3.idx` before every connection, so an account's
      secret key rides in it (`/yo3/<key>/yo3.idx`): the patch server notes the address that presented it and that
      address's next login gets the key's account, on any address (`Directory::present_key`/`login`; tests in
      `accounts.rs` and `flow.rs`). No key: the address account, as before. Host: `add-account`, `key ADDRESS|PLAYER`.
      Live: Yugi's headless client with a key in `yo3.ini` listed the key account's players, the same client without
      it the address account's. Persistence stays JSON (`data/accounts.json`) with no concrete need for SQLite
- [x] Player creation and deletion: Select Player's Create Player and Delete (`0x1F4A {pindex, pname}` → `0x2262`/`0x2263`,
      `0x1F4B {player_id}` → `0x2264`/`0x2265`; `CharMake.cpp`'s `0x2CF5/6` is only the appearance editor), within four
      slots kept with the account. Live: Joey made in slot 3 (name, then Avatar Settings, then the main screen), deleted
      with its password prompt (local, nothing sent), the slot empty again
- [x] Trunk contents: the card collection (the starter deck's cards and the player's bought cards, `0x1FA4` → `9000`)
- [ ] Deck list, deck editor saves (`HolderSelect.cpp` "deck list")
  - [x] Every player owns a starter deck's cards and gets it in the deck list (`0x1F4E` → `0x22C8`, tag-6
        items); Edit Deck saves it locally and the NPC duel accepts it
  - [x] Official character decks: every player also gets Yami Yugi's, Jaden's and Yusei's three decks each (the
        client's own NPC decks 120-122, 147-149, 171-173) and owns their cards. The deck list holds 10 and the client
        keeps every listed deck it lacks (by name), so the official decks take turns in the room the starter deck and
        the player's own decks leave, each list going on where the last stopped: any PC gets them all, with nothing for
        the host to run. Live: Load Deck listed the nine, Yugi's "Miracle Magic" loaded with its 42 + 3 cards. (The Load Deck window's Official tab reads recipes from the client's
        own `data/recipe/` files, not from the server.)
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
      and the players (`docs/guide.md`: `yo3.ini`, Windows or Wine, LAN / VPN / port forwarding)
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
| Starter deck and collection | the first preset of type 1 that regulation 1 allows (preset 2); one copy of each of its cards | The original let new players choose; a real collection comes with the economy |
| Trunk (`9000`) contents | one record: `cards` (the starter deck's), `noTrade` all zero | Other fields later screens may read are unknown |
| `regulations` list | all 20 ids | Which banlists the UI should offer |
| `0x271A/0x2AFB/0x2AFD/0x27D8` | minimal replies | Meanings unknown; they only stop timeouts |
| `avatar1` slots | 41 zero entries (a default male avatar); the length is the client's own (settled) | Which slot holds which part category is not mapped (`0x42CA60` copies slots 0, 1, 2, 4–10 into separate fields, `0x429210` slots 17–40; 11–16 unread there) |
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
| Card rental | the ready request `0x2C3E` is answered `0x2D68` (the client reads no reply: settled); error codes 504/503 are never read by the rental handler (settled; every error shows "Rental Cancelled."); terms 1, 3, 7, 14 days (the dialog's); a No keeps the scene with `trade_result` 0 (settled, `0x41CAB1`); a rental ends exactly at its term, checked each second; a lender can't put up cards they have lent | The original's rental rules (list limits, early return: the client has none) are unknown |
| Country | the account's setting, else GB/JP/DE/FR/IT/ES/KR/CN from the login's language | The client has no language-to-country table (it derives `lang` from the Windows language, `0x40DE50`); `country` is server data |
| NPC roster `need_ex` | 100/200/300 (the tabs follow the series: settled by the NPC name texts) | No client file or script holds the thresholds; it can't all be 0 (`0x4BFDC4`) |
| Teams | `level` 1 and scores 0 for every team; handing over the leadership makes the old leader a member; the founding time in Japan time | No team scores or levels exist without the original's events |
| Search status bits | `0x1000` duelling, `0x800` hosting, `0x200` hosting Single, `0x400` hosting Match | Read from the Detailed Search texts |
| Manner level | the average of a player's ratings (-10..10) mapped onto 0–4, (average + 10) / 5 rounded; 2 with no ratings; `evaltotal` the sum, 0 when negative | The original's thresholds and `evaltotal`'s sign are unknown |
| Trade end | a refused trade sends `0x2BCA {trade_result 0, error_code 503}` ("Trade Cancelled", the scene closes: settled by `0x424F71` and seen live on both clients, with a later trade completing); the trunk is pushed after a trade | How the original refreshed the trunk is unknown |
| Item prices | protectors 200 BP, avatar items 100 BP, one of each | The original prices are unknown |
| Avatar items for sale | the catalogue's parts with sub 0 (178 male, 148 female) | Read as "to acquire" next to the default (2) and starting (1) parts |
| Wins and losses | My Profile and Player Information count duels against players only, not NPCs | The original's counting is unknown |
| NPC experience | 100 a win, 50 a draw, 10 a loss, per NPC level id | No request reports it; the original's amounts are unknown |
| `premium` | 1 for every account (all NPCs, Form Team, Create Event) | The original sold it (Duelist Premium); a friends' server gives it to all |
| `0x17D4` | answered `0x17D5`, nothing kept | Sent once the duel object is ready; may also carry results later |
| `0x22C4` push | right after `0x17D5`; `rule` = the fields of the duel's `0x17AC` item | The original timing and rule contents are unknown; only `match_type` is read so far |
| Server duel messages | request `0x200D` | The pump reads only `duelRequestName` (and needs the id non-negative) |
| CPU janken | random hand; the CPU goes first when it wins | The original CPU's choices are unknown |
| Card table entries (1000) | param3 0 (it is only a hover-help text id for command 100, `0x523D20`), flag byte 0 (stored, no consumer found); uids 1.. in table order | The flag byte's use is unverified |
| Hidden cards | against another player, the opponent's deck, hand, extra deck and face-down cards go with row 0 and face-down monsters get no field record; against the CPU, real rows | Online 2 used placeholders, whose value is unknown; a hidden card that flips without moving keeps row 0 until it moves (position changes aren't translated yet) |
| Automatic matching | the first two players waiting for the same `match_type`, Both (6) with either and two Both players in a Single duel; the level choice is ignored; the second player's `0x1978` goes a second after its `0x1799` | Konami's matching rules are unknown; the delay assumes the agent reads one message per step |
| Match Mode | best of three; every duel opens with a janken; a drawn duel counts for neither; the records count the match once | Online 3's rules for who goes first in later duels, draws and the side deck are unknown |
| Hosted LP and `--duel-lp` | the testing aid wins over a hosted `lp_start` when it isn't 8000 | A testing convenience, not the original's |
| Duel Menu counts | `duel_playing` = 2 × matched duels, `duel_ready` = hosted duels, `duel_auto` = players waiting | Whether they count players or duels is unknown |
| Hosted duels | `duel_status` 1 while hosting, 3 while in a duel between players; Team-only duels take nobody (no teams); `0x17C0` answered `0x17C1` | The original values and the limits' rules are unknown |
| A player leaving | the other wins (command 2 {1, 14 "disconnected"}) and both records count it | The original's handling (time-outs, `0x2724` heartbeats) is unknown |
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
| Chain chances | level 3 after an event, 1 at a phase or step, 0 when no card fits; messages 7/8 (Spell/Trap zone)/269 | Client side settled (`0x4C5560`: bands below 2, 2–3, 4+; Activation Message decides below 4); which windows the original judged low priority, and its message choice, are Konami-only |
| Rules | EDOPro's `MODE_MR2` | Online 3 (2010) ran Master Rule 2; EDOPro's preset may differ in details |
| CPU play | first summonable monster, attack with all, never chain | The original NPCs' AI is unknown |
| Duel end reason | EDOPro's reason mapped to the client's texts (LP 0, deck out, surrender, Exodia...; others "won with a card") | The original server's values are unknown (100 takes another path) |
| Duel's end push `0x2009` | sent when the engine ends the duel; `duel_end` 1 (normal: no effect) or 3 (cancelled before it began); no `score_<id>`; `dlogURL` for duels between players | The original timing is unknown. Values settled by `0x515BD3`/`0x518631`: 2, 3 dialog `0x2918`; 4 "has disconnected" (we don't send it); 5 opens the result |
| Result screen counts | `win`/`lose`/`draw` = totals against this opponent including this duel, `*_add` = this duel's change; kept in memory | The client only shows them ("Wins 1 (+001)") |
| Duel port | 1214 | The original's port is unknown; the client takes whatever `duelData` says |
| Accounts | by the key in the patch URL (`/yo3/<key>/yo3.idx`, fetched before every login: the address that fetched it last, within 10 minutes, logs in to the key's account); no key: one per client address (`move-account` for a changed address) | The client sends no credentials; the key rides in the one request it makes with a configurable path. Two friends behind one address starting within the same moment can swap; keyless friends behind one address share an account |
| Uploaded decks | the decks a duel's Select Deck sent, ten per player by name, listed after the starter and bought decks | The client has no other upload; the original server's deck storage is unknown |
| Deck check | sizes, ownership per Konami id, the duel's regulation, refused with `0x22CB` (client text `0x2919`: settled, `0x50ECF0` compares only `0x22CA`/`0x22CB` and never uses `error_code`) | The rules of the check are ours |
| Player names | the starting names (`--players`, default Duelist) made unique with a number; players make more at Select Player (at most 16 characters, unique ignoring case: **guess**), the host can still `add-player` / `rename` | The original's name rules are unknown; a team's leader can't be deleted and an account keeps one player (ours) |
| CardSwap `0x1B` | each of the two words carries the face and position of the card arriving there (MSG_SWAP, Magical Hats) | Read in the code (`0x4FF000` case `0x1B`); live only for Creature Swap, whose two monsters were both face-up attack |
| Magical Hats' new places | read back from the duel after the step (the sidecar's `places`), matched by id | The core's message lists only old places (the core's writing order as remembered, and ocgcore-wasm's pairing); the self-played duels agree with the engine, the client never saw it |
| Xyz materials | CardVanish as one becomes a material, a move from an empty field slot as it leaves | How Online 3 showed materials is unknown; the client lacks the Xyz Monsters tried |

### Settled by the client's own code (no longer guesses)

Evidence for each is in docs/protocol.md: `pbin` 512 bytes (`0x47E490`); `avatar1` length 41; `0x21FC lang`, chat room
`region`/`status`/`type`, `0x2458.lang`, `PlayerType001/002`, `join_id`/`duel_id` (opaque echoes), ChainRun
`param1`/`param2`; `bgm`, `icon`/`lefttime`/`maintenance`, `npc_id` (omitted is right); menu bits 6/7 for Set (the icons
and `0x520F00`); card list kinds and marks (`0x4C72D0`, `CardAttr`); the NPC tabs; the `0x17C1` and `0x2D68` ids (no
reader). `cpuUse`/`notRelease` (`0x27D9`) are real but only drive the main screen's completion percentages: left out
(every card counts) because the key type isn't verified.

## Leads (outside the code)

- Old Online 3 duel logs (`players\<name>\duellogs\`) would be official duel recordings.
- The DuelEvolution Discord reportedly has Online 3 traffic captures (deferred).
- archive.org's `yo3setup_20110728.exe` is three weeks newer than our client; check its protocol version.
