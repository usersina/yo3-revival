# Changelog

Beta: expect bugs, and please [report them](https://github.com/usersina/yo3-revival/issues/new/choose). The beta starts
at v0.1.0; v0.0.1 to v0.0.26 were the alpha (published as "pre-alpha").

Each section is a release. To release, rename the "Unreleased" section at the top to the next version
(`## vX.Y.Z (date)`), set the same workspace version in `Cargo.toml`, and push to `main`: the release workflow builds
the bundles, publishes the version and container images, then updates the configured VPS automatically. After a healthy
deployment, only the newest two beta releases and their images remain on GitHub; the alpha releases (v0.0.x), Git tags
and this history stay. A push
whose newest version is already released skips the server release but can deliver compatible dashboard changes.

Older entries describe their release at the time. For current setup, use the [README](README.md) and
[graphical setup guide](docs/launcher.md). Releases through v0.0.3 predate the graphical launcher and TLS login.

## Unreleased

- **Desktop says less, and explains on request.** Each setting is one line: its name and its control. What it does
  waits behind the small **?** beside its name: rest the pointer on it. The same goes for Setup's last step, Host for
  friends and Multiplayer's dialogs. The data and logs folders open from a folder button beside Backups and Export logs….

## v0.1.27 (2026-11-27)

- **Dartz joins Kaiba Corporation.** A new opponent on the Yu-Gi-Oh! tab, in three levels: his Seal of Orichalcos
  decks, printed cards normally and the Orichalcos cards he used in the anime while the server plays the anime cards.
  He talks before and during the duel in his own words, and stands, poses and plays his duel scenes as a clothes-store
  mannequin until he has his own art. More characters can follow the same way (#95).
- **The Field Spells the game never had show their own field.** The Seal of Orichalcos, Advanced Dark, Toon Kingdom and
  the other added Field Spells come with their duel field picture, EDOPro's, downloaded with the card pictures (Settings →
  Game → Card pictures). Until then they leave the plain board, where the field used to turn white.
- **A Set Field Spell can be activated.** A Field Spell you Set had no menu to activate it later; it has now, and
  turned face up it shows its field (Sogen's grassland) like one played from the hand.

- **Set cards can be picked when a card needs several.** Brionac, Dragon of the Ice Barrier and other cards that
  pick two or more cards on the field showed the opponent's Set cards as blank slots you couldn't click; they show as
  card backs and can be picked (#90).
- **Black Brutdrago from Tuned Magician** (made a Tuner by Unleash Your Power!) is covered by a test; the crash it
  caused before v0.1.26 is gone (#87).

- **The Extra Deck always asks which monster.** With only one Synchro you could summon, a click on the Extra Deck went
  straight to the materials without naming it; the list now shows it first, and closing it takes you back (#92).
- **The server's own duel prompts speak your game's language.** "Select the monster(s) to use as Synchro Material." and
  the other lines the game has no text for are now in Japanese, German, French, Italian, Spanish and Korean too (#92).
  A card's own options (Enemy Controller's, Elemental HERO Stratos') are still in English.

- **Nobleman of Crossout no longer takes minutes.** When it banishes a Flip monster each Deck is shown to the other
  player; instead of turning every card over one at a time, the chat window lists your opponent's Deck in one line (#94).

- **Each duel says its banlist.** When you pick your deck, the chat window tells you which banlist the duel uses, so an
  opponent's three Morphing Jar in a room on Unlimited no longer looks like cheating.

- **Hosts can add their own banlists, in EDOPro's format.** Drop an EDOPro `lflist.conf` into the server's
  `banlists` folder and restart it: each list in it can be chosen like the game's own (`/p Server banlist NAME`,
  Create Duel, events), and the game's Edit Deck shows its limits (#77, a first step: players' own lists come next).

- **Pick an effect's target on the field.** Mystical Space Typhoon, Dust Tornado and other cards that target one card
  on the field let you click it there, as attacks do, instead of a list of look-alike Set cards; right-click opens the
  list if you prefer it (#91).

- **Dartz hails his trump cards.** He calls out the Seal of Orichalcos and Divine Serpent Geh in the chat when he
  plays them.

- **Your banlist, chosen in Edit Deck.** Edit Deck → Change Regulations is no longer a preview: the banlist you confirm
  there is yours for duels against the computer, at once, without logging in again, and Create Duel starts on it.
  `/p Server banlist NAME` does the same. Auto Duel always uses the host's banlist, so everyone meets everyone, and
  an event under its own banlist checks your deck against it without asking you to change yours.
- **Your own banlists.** Import an EDOPro banlist (`lflist.conf`) in Desktop (Settings → Singleplayer → Your data →
  Banlists, or Decks… under your User ID on a friend's server), up to 3 on a server. Everyone there finds it in Edit Deck
  → Change Regulations from the game's next start; export it back for EDOPro or delete it any time.
- **"There are no valid Decks available." now says why.** The chat window lists what breaks each of your decks under
  that duel's banlist.

## v0.1.26 (2026-11-26)

- **EDOPro decks.** Desktop imports an EDOPro deck file (`.ydk`) into your decks, and exports yours for EDOPro: Settings →
  Singleplayer → Your data → EDOPro decks on your own server, and **Decks…** under your User ID on a friend's server.
  The deck is in Load Deck the next time the game lists your decks. Cards the server doesn't have are left out and
  listed; cards you don't hold are named. Hosts: `decks`, `import-deck` (a `.ydk` file or a `ydke://` link) and
  `export-deck`.
- **The Anime Collection pack.** Blister's sells a new pack while the server plays the anime cards: the anime-only
  cards (never printed), five a pack with a rarer card among them, for 300 BP. It is the only way to get them: they
  aren't in the shop's Rental or Trade Card. With anime cards off the pack isn't on the shelf. `where` names it. Hosts:
  `anime-pack-cost` sets its price.
- **Rarities for the anime-only cards**: 12 Ultra Rare, 13 Super Rare and 25 Rare, picked by their place in the anime; the
  rest are Common. The reveal shows each in its frame.
- **Why a deck can't duel.** The game only says "Invalid Deck" or "There are no valid Decks available."; now the
  server says why in the chat, card by card: "Pot of Greed is Forbidden in Season 16.", "Mirror Force: 2 copies, but
  Season 16 allows 1.", "The Main Deck has 38 cards: it needs 40 to 60.", "You hold 1 Blue-Eyes White Dragon but the
  deck has 3.". It says so when you save such a deck in Edit Deck, when a duel refuses one, and for the new `deck` chat
  command (`deck` checks all your decks, `deck NAME` one).

- **358 more cards from the anime.** Cards the Duel Monsters, GX and 5D's anime showed that the game never had: the
  Advanced Crystal Beasts, Valkyries, Timelords, T.G. monsters, Fortune Fairies, Life Stream Dragon, the Legendary
  Dragon Spells and many more. A host gives them with `give-cards`; with anime cards on, 238 of them play their anime
  versions.
- **Anime-only cards, first batch: Yami Yugi's, Yami Bakura's and Rafael's.** 42 cards the anime showed and that were
  never printed (Magical Pigeon, Necro Wall, Guardian Formation, Summoning Clock and more), played with anime cards on.
- **Anime-only cards, second batch: Seto Kaiba's, Joey Wheeler's and Alister's.** 35 more (Sky Union and Air Fortress
  Ziggurat, Tank Corps and KC-1 Crayton, Toy Robot Box, Clone Dragon, Roll of Fate, Compensation Mediation and more),
  with their Robot, Tank and Player Tokens.
- **Anime-only cards, third batch: Yugi Muto's, Weevil Underwood's and Leon von Schroeder's.** 28 more (Mirage Ruler,
  Turn Jump, Spell Textbook, Weevil's Parasite cards and Infestation, Leon's Little Red Riding Hood, Thorn Princess,
  Seven Kid Goats and more).
- **Tokens the game never had.** Blackwing - Steam the Cloak, Ecole de Zone, Gadget Box, Demiurge Ema, Black Wing
  Revenge and Spirit Illusion join the new cards (and, with anime cards on, Natural Selection, Cell Division and
  Reckless Parasite), with their Tokens (Steam, Mask, Gadget Box, Homunculus, Black Crest, Doppelganger, Selection,
  Cell Division and Parasite Tokens, and the anime Gadget Box's Gadget Token), shown on the field like the game's own;
  a Token that copies a monster shows that monster's Level, Type and Attribute.
- **Anime card fixes.** With anime cards on, Bounce Spell no longer stops when its target is gone, Limit Tribute also
  stops Summons that need two Tributes, Call of the Haunted brings a monster destroyed in battle back with 10% more than its own ATK, growing with each revival (an Equip Spell's boost isn't counted, and applies on top), Take One Chance leaves a card that can't be activated in the Graveyard, Seven Kid Goats destroys the Beast it picks, Iron Cage sends the monster to its owner's Graveyard, Dark Element summons
  Dark Guardian, Advanced Crystal Beast Emerald Tortoise can turn a monster that attacked to Defense Position, Spirit
  Shield negates the attack even with its last Fiend, and Catapult Turtle's effect no longer fails when its target is gone.
- **Synchro Summons with many possible materials** are worked out quickly: with the anime T.G. Cyber Magician (any
  monster in the hand can be a material), finding the summons took the rules engine half a minute or more.
- **Revivals reach further by default, as in the anime.** Monster Reborn and every other revival from the Graveyard or
  banishment brings back a monster that was never properly Summoned (a discarded Ritual Monster, a Synchro Monster sent
  from the Extra Deck). A host who wants the official ruling sets `revival-rule` to `official` (Desktop: Settings →
  Singleplayer → Revival → Official rules); a server that already chose a rule keeps it.
- **With anime cards off, anime-only cards are Forbidden in every banlist** (Unlimited too): Edit Deck shows them with
  the Forbidden mark, and the game refuses a deck with one before you queue, instead of at Select Deck.
- **Unlock all cards happens once.** Desktop shows how many cards you have out of all of them (1,214 / 5,030); after the
  unlock the button reads All cards unlocked, and comes back as Unlock N new only when an update adds cards. Players
  made later start with the unlocked cards, and cards you traded away stay gone.
- **Watching a duel needs a game started with the server's card versions**: after a host switches anime cards on or
  off, a game opened before is told to restart instead of watching duels with the wrong cards.
- **An Equip Spell left with no monster to equip** does nothing instead of failing with a script error.
- **A Spell or Trap moved to the other player's field** (Bounce Spell) shows without the counters it lost, and **a card
  put back on the field from the Graveyard** (Compensation Mediation) no longer shows a Synchro Summon if it was once a
  Synchro Material.
- **A monster made a Tuner by an effect** (Noble Knight's Spearholder) tunes as one in the Synchro Summon's animation.

## v0.1.25 (2026-11-25)

- **The card panel shows ∞ too.** A card with an infinite ATK reads "ATK ∞" in the card panel beside the field instead of
  a long number.
- **Hosted servers update again.** The server's container image failed to build for v0.1.23 and v0.1.24, so servers
  run from it stayed on v0.1.22; this release brings them everything since.

## v0.1.24 (2026-11-25)

- **∞ LP.** Mystik Wok on an infinite ATK gives **∞ LP**, shown as ∞ in the LP disc. Damage, costs and gains leave it
  as it is and show nothing, no number and no reaction, as a hit by a monster with 0 ATK.
- **∞ is one value**, for LP and ATK alike. Half or double of ∞ is ∞ (Solemn Judgment, Riryoku, Megamorph), +1000 or
  -700 leave an ∞ ATK as it is, and ∞ against ∞ is a tie. A card that takes all your LP takes ∞: Ra's "pay all but 1"
  leaves you 1 and gives Ra ∞, De-Fusion gives it back; Yu-Jo Friendship gives both players ∞; "your LP become X" makes
  it X (Life Equalizer, Wiraqocha Rasca). An infinite ATK still takes all LP, ∞ too.
- **∞ damage shows as ∞.** The damage pop-up shows **-∞** (red) for infinite damage and **∞** (cyan) for an infinite
  gain, instead of 99999.
- **Comeback wins count right.** A win after your LP fell to 1000 or less counts as a comeback even when an effect set
  your LP that low (Wiraqocha Rasca, Life Equalizer) or after ∞ (Ra's "pay all but 1").
- **Long number choices work.** A card that asks for an amount from a long list (Wall of Revealing Light, Bark of Dark
  Ruler with a lot of LP) offered only part of it, sometimes nothing; it now offers 255 choices, up to the largest.

## v0.1.23 (2026-11-25)

- **The Gods play their manga versions** with anime cards on: Obelisk, Slifer and Ra, and the Wicked Gods Dreadroot,
  Avatar and Eraser, marked [Manga]. Each has its Divine Hierarchy, so a God is no longer destroyed in battle by a
  stronger ordinary monster, and Obelisk's Soul Energy Max (Tribute two in the Battle Phase) wins the duel. Skill
  Drain negates a God's effects for one turn only, as its text says. A God turned face-down by Book of Moon is still a
  God (Spell and Trap cards can't destroy it) and turns face-up again at the end of that turn.
- **New card: The Winged Dragon of Ra - Sphere Mode.** Tribute it for The Winged Dragon of Ra from your hand or Deck
  with 4000 ATK and DEF. With anime cards on, that is the manga Ra, and it stays on the field with its 4000: it is Ra
  out of its sphere form, so it doesn't go back at the End Phase as other Special Summoned Gods do.
- **∞ on the field.** An infinite ATK shows as ∞ instead of 99999: white when it is the card's own printed ATK (the
  anime Divine Serpent Geh), blue when an effect gave it (Obelisk's Soul Energy Max, Avatar matching it).
- **Infinite is infinite.** Soul Energy Max, the anime Divine Serpent Geh's attack, Ring of Destruction on Geh and
  Avatar risen to infinity take every LP a player has, even tens of millions from Mystik Wok (it used to turn into a
  huge gain). Half of ∞ is still ∞, so the Wicked Dreadroot leaves an infinite ATK alone. The anime Geh's ∞ is its
  printed ATK: Skill Drain negates its effects but leaves the ∞, and other cards' ATK changes (The Seal of Orichalcos,
  Axe of Despair, Shrink, Shield & Sword) don't touch it.
- LP above 99,999 shows as 99,999 and stays right as it changes, and a huge LP change no longer freezes the duel.
- **New card pictures** for the manga Gods (Obelisk, Slifer, Ra, Dreadroot, Avatar, Eraser) and the anime Divine
  Serpent Geh, drawn for YO3 Revival.
- **Your own card pictures, as in EDOPro:** put `<passcode>.jpg` or `.png` in the `pics` folder beside `anime-art` and
  it becomes that card's picture from the next PLAY, for anime and manga versions and the cards the game never had.
- **New cards the game never had:** Ra's Disciple, The Seal of Orichalcos, Divine Serpent Geh, Illusion Gate and
  Underworld Circle. A host gives them with `give-cards` for now. With anime cards on, Divine Serpent Geh and Underworld
  Circle play their anime versions.
- **Anime-only cards**, played when the host turns on anime cards: Orichalcos Gigas, Kyutora, Dexia, Aristeros,
  Deuteros and Tritos, and Arduous Decision. Their text starts with [Anime]. With anime cards off, a deck holding one
  can't duel, and the chat tells you which card to take out.
- Added cards are found by Edit Deck's Search Key filters (Special Summon, Destroy monster...), and their new-card
  reveal shows their rarity's frame.
- Hosts can choose a house rule on reviving monsters that "must first be properly Summoned" (Fusion, Synchro, Ritual
  monsters...), the `revival-rule` setting: the official ruling (the default), cards that say "ignoring its Summoning
  conditions" ignore that too (Illusion Gate takes a Divine Serpent Geh that was only ever sent to the Graveyard), or
  every revival from the Graveyard does, as in the anime (Monster Reborn revives a discarded Ritual monster). Playing
  alone, pick it in Settings, Singleplayer, **Revival**.
- **Smaller downloads:** each edition carries only the card scripts the game can use (about 5,700 files instead of
  19,300), and the developer documents stay on GitHub. Desktop 0.0.9 or older can't update to this version by itself:
  download it again.

## v0.1.22 (2026-11-24)

Multiplayer: update to v0.1.22 to join the hosted server after deployment.

- Updates no longer download by themselves. "Update X is available" in the footer (or **See the update** in Settings,
  About) opens the update's window with what's new and a **Download** button; it then installs when you restart. Turn
  on **Download updates automatically** in Settings, About if you prefer the old way.
- **Restart to install** hides this window right away, so the new version's window no longer opens beside it.
- **Report a problem** now saves one file in your Downloads folder with what you wrote, an optional way to reach you,
  your recent logs (personal details removed), this PC's details and, in Singleplayer, the duels you marked with `bug`.
  Nothing is sent anywhere: send that file to the developer on Discord, privately, or on a GitHub issue.
- In Singleplayer, the `bug` chat command also tells you where the duel's recording was saved on your PC. On someone
  else's server it says the host keeps it.
- Logs are now in one readable format with exact times, so problems are easier to track down.

## v0.1.21 (2026-11-23)

Multiplayer: update to v0.1.21 to join the hosted server after deployment.

- A draw in the rock-paper-scissors against the computer now says so in the chat window: the game asks for another
  hand with no message, which looked like a freeze (#83).

## v0.1.20 (2026-11-23)

Multiplayer: update to v0.1.20 to join the hosted server after deployment.

- Blister greets you in your game's language when you open the shop: the first line was always in English (#84).
- A Starter Deck you delete in Edit Deck stays deleted instead of coming back the next time you open the editor (#85).
  Its recipe stays under Load Deck → Official.
- A booster pack no longer holds the same card twice (#86).

## v0.1.19 (2026-11-22)

Multiplayer: update to v0.1.19 to join the hosted server after deployment.

- The computer Sets its Flip monsters, such as Morphing Jar and Man-Eater Bug, face down instead of summoning them in
  Attack Position, so their effects work when they are flipped (#80).
- Choose the flag others see next to your name: whisper `flag BR` to the server (`/p Server flag BR`) with your
  country's two-letter code, or `flag auto` to follow your game's language again (#81). Others see it at once.
- Fixed: your own game could show you the German flag after you entered the map, whatever your language.
- The shop's avatar items are every part the game has, 104 Duel Disks among them: the player guide now says so (#82).

## v0.1.18 (2026-11-22)

Multiplayer: update to v0.1.18 to join the hosted server after deployment. The hosted server never got v0.1.17 (its
server image failed to build), so this release also brings v0.1.17's changes to it: Shooting Quasar Dragon,
Doppelwarrior's question after a chain.

- Fixed: your opponent's Fusion and Synchro Summons, a computer opponent's included (Jaden's Fusions), showed only the
  light: the summoned monster now appears in the animation (#79). The same fix shows the card when your opponent
  Tributes a Set monster, uses Fusion materials from their hand or activates an effect from their hand.
- Fixed: a monster your opponent Special Summoned face down and then showed you for a moment arrived face up without
  its picture before it turned face down.

## v0.1.17 (2026-11-22)

Multiplayer: update to v0.1.17 to join the hosted server after deployment.

- New card: **Shooting Quasar Dragon**, the first card the game never had. It plays by its real effects (two Synchro
  Monsters and a Synchro Tuner summon it; it attacks once per non-Tuner, negates an activation once per turn and brings
  Shooting Star Dragon when it leaves). Your host gives it for now. Its picture comes with the other card pictures; until
  then it shows a stand-in picture. With anime cards on, it plays as the anime's Shooting Quasar Dragon (when it leaves the field it destroys
  every monster and brings Shooting Star Dragon).
- Fixed: an effect you could use right after a chain resolved was skipped without asking with the default Activation
  Message option, such as Doppelwarrior coming from your hand when Junk Synchron brings a monster back from the Graveyard.
  The game now asks.
- Singleplayer's banlist is now **Unlimited** by default, so any deck works from the start, the character decks
  included. Choose Season 16 (or any other list) in Settings → Singleplayer → Banlist to play by the original rules.
- The Multiplayer tab's server list now updates every 3 seconds while you look at it: online status, players and ping.
- Card pictures moved to Settings → Game → **Card pictures**: the anime cards' and the new cards' pictures, with
  **Download automatically**. Setup's last step has the same tick, ticked, so new players get them in the background
  after setup. While they download, the footer shows the progress; click it to open the group.
- Setup's game check now says "Wine took too long to start" in plain words, and waits longer for a slow first start.
- Edit Deck now lists Blue Rose Dragon: the game's last card was missing from every trunk.
- Hosts and developers: `--duel-field 0:mzone:ID,1:hand:ID` starts every duel between players with those cards in place,
  to test a card without building the board first.

## v0.1.16 (2026-11-22)

Multiplayer: update to v0.1.16 to join the hosted server after deployment.

- New chat command `bug what happened` (`/p Server bug …`): if a card effect went wrong, send it right after the duel
  (or during a duel against another player, on the Opponent tab).
  Your duel goes to the host's staff with your note, and they can play it again exactly as it happened to find the
  cause.
- Chat that never arrived now does: lines on the Opponent tab (before and during a duel against another player), on
  the trade screen's Trade Partner tab and on a chat room's TRADE tab used to answer "That command is not available."
  and reach no one.
- The chat joins the Lobby by itself after you log in, so what you type on the map reaches the room straight away (you
  can still pick another room, or leave it).
- Hosts: every duel is now recorded for two weeks (about 10 KB each; the `recording-days` setting, 0 turns it off), and
  the duels players send with `bug` are kept until you delete them. They show on the Reports page.

## v0.1.15 (2026-11-21)

Multiplayer: update to v0.1.15 to join the hosted server after deployment.

- The Tribute list now shows the opponent's face-up monsters you may Tribute (Soul Exchange) with their faces, not as
  card backs.

## v0.1.14 (2026-11-21)

Multiplayer: update to v0.1.14 to join the hosted server after deployment.

- `where CARD` now names the Special Pack (an Official Event win's prize) for the cards only it gives, such as Malefic
  Paradox Dragon and Malefic World, instead of "no normal route".

## v0.1.13 (2026-11-21)

Multiplayer: update to v0.1.13 to join the hosted server after deployment.

- A Synchro Summon that uses Formula Synchron as a material now treats it as the Tuner it is. Before, the server
  ordered it among the other materials, which could break the game's Synchro animation.

## v0.1.12 (2026-11-21)

Multiplayer: update to v0.1.12 to join the hosted server after deployment.

- **See a pack's cards before you buy it.** The game's website links (the title screen's Official Website, Main Menu →
  Leaderboard) now have a **Packs** page: every booster pack on sale with every card it can give, by rarity, and the odds
  of each rarity. In the chat, `/p Server pack NAME` lists a pack's cards, rarest first; `pack` alone lists the packs.
- The website pages show the server's name and a bar to every page: Home, Leaderboard, Teams, Packs, BP and Events. BP
  says how points come in on the server, Events where tournaments and official events are on the map.
- When a page can't load, you now see why (you're not signed in, the game is still signing in, the server didn't answer
  or took too long) instead of the browser's "connection reset".
- `where CARD` names packs as the shop does ("Pack 1", not "Booster Pack 1").
- Anime cards: each anime card's text now starts with **[Anime]**, so you can see which cards play as their anime versions.
- Anime cards: The Winged Dragon of Ra now shows its anime text. It played the anime effect, but its text was too long for
  the game, which showed the normal card's text instead.

## v0.1.11 (2026-11-21)

Multiplayer: update to v0.1.11 to join the hosted server after deployment.

- Brings v0.1.10 to the hosted server: its update didn't go through (the staff dashboard didn't start, and the server
  went back to v0.1.9 by itself). Nothing else changes for players.

## v0.1.10 (2026-11-21)

Multiplayer: update to v0.1.10 to join the hosted server after deployment.

- The game now gets each reply from the server in one piece. This aims at the shop disconnecting some players
  (#65): the game waits for a whole reply before reading it, and on some Windows PCs it could give up on the shop's
  larger lists instead.
- If the game drops its connection anyway, its log now says why and where. Please send **Settings → Advanced → Export
  logs…** with a report.
- A sign-in or a game connection no longer waits while the server is busy saving.
- If your connection drops while making an account, making it again with the same User ID and password now works and
  shows you a new recovery code, instead of "That User ID is taken".
- A sign-in that can't reach the server says so within about 12 seconds, not 40.
- One lost answer no longer shows a server as Offline until you click RETRY.
- An old website page that stops loading now ends after 30 seconds.
- When a game's connection breaks or it closes with an error, its logs now go to the server you played on by
  themselves, so the host can see what went wrong without asking you for files. Setup asks about it (on unless you
  untick it); Settings → Advanced → **Send logs when something breaks** turns it off. Passwords and keys are never in
  them.
- **Report a problem** sends your logs the same way and puts their report ID in the GitHub form: there is nothing to
  attach any more. In Singleplayer it still saves a file for you to attach.

## v0.1.9 (2026-11-21)

Multiplayer: update to v0.1.9 to join the hosted server after deployment.

- **Destiny Draw** (off by default): in a duel against the computer, once per duel, at the start of your Draw Phase with
  1000 LP or less, the game asks whether to use Destiny Draw; if you do, you pick any card left in your Deck and draw it.
  Say no and you are asked again at a later draw, until you use it. Turn it on in Settings → Singleplayer → Cards, or, as
  a host, with `yo3 set destiny-draw on` (it applies from the next duel). Duels between players and events never have it.
- **The Stadium pays in Special Packs.** Every win in an official event (a duel, or a match) earns a Special Pack of five
  cards no other pack gives (the promo cards such as the Malefic monsters, Dragunity Arma Leyvaten, Blackwing - Zephyrus and
  the Masked Heroes, and the cards you could only rent until now), up to 10 in one event, as a present with a D-Mail from the Host. Your host's BP prizes still arrive when an
  event ends.
- **Event titles.** TRIALIST, ULTRA and ULTIMATE TRIALIST (3, 10, 30 Duelist Trial wins), GUNSLINGER to ULTIMATE GUNSLINGER
  (a best streak of 3, 7, 10, 15), EVOLUTION to ULTIMATE EVOLUTION (30, 100, 300, 500 stars), THE ANNIVERSARIAN, and the
  three CHAMPION titles for first place in a finished event: the numbers are the game's own title texts.
- **Deck recipes.** Beat an NPC for the first time and you get the recipe of the deck it played (Load Deck → Official). The
  22 theme decks, the two other starter decks and the 57 decks of NPCs you can't duel (the shop's shelf can't show them)
  are sold by a chat command: whisper
  `recipes` to the server to see them and `recipes NAME` to buy one with its cards.
- **Other artworks.** Booster packs and Special Packs now sometimes (about one pull in twenty) give another artwork of a
  card, such as Blue-Eyes White Dragon's other pictures.
- **More trades.** Blister's Trade Card list has one more pool, the game's own card list, with cards no booster pack gives.
- The map's **Championship** (Tournament tile) works: your host schedules one with `create-championship` (or the staff
  dashboard's new Championship page) with an entry window and a start time. Opening the tile while entries are open enters
  you (the host may ask for a number of duels played). At the start the lobby shows "Selecting players for entry…", draws
  up to 16 players, and plays a single-day knockout Final (1st Round, Quarter-Final, Semi-Finals, Final) with byes where
  seats are empty; fewer than two entrants cancels it with a D-Mail. The titles Y.O.C. QUALIFIER, Q-FINALIST, S-FINALIST,
  FINALIST and WINNER follow the rounds, the host's BP prizes arrive by D-Mail, and Duel Log lists past Championships
  (its viewer of one shows an empty tree: no replays are kept).
- In a user event or Championship, a player who leaves the janken or the turn order choice now loses the round to the one
  who stayed (both used to be out).

## v0.1.8 (2026-11-20)

Multiplayer: update to v0.1.8 to join the hosted server after deployment.

- Settings → Singleplayer → Your data has **Unlock all cards**: three of every card, in every artwork, for every player of
  your own server. Hosts have the same as `unlock-cards`.
- Anime pictures: the download keeps going when you leave Settings and ends in a toast, a picture that times out or
  arrives broken is tried three times before it's given up, and the progress shows only under the switch, never in a
  dialog. The section under Anime cards is tidier.
- An effect with more than four choices now shows them as buttons, three at a time with "More choices" to see the next
  ones, instead of a card list with the choices written in the chat window.
- More of the hover markers in a duel: pointing at a monster or a card now also shows which Field Spell powers it up,
  which continuous Spell or Trap holds it back (Gravity Bind, Messenger of Peace) and what negates it, both ways.

## v0.1.7 (2026-11-20)

Multiplayer: update to v0.1.7 to join the hosted server after deployment.

- Choices the game's own windows could not show are now asked of you, in steps, instead of being picked for you. A
  monster that can be placed in three or four positions asks Attack or Defense first, then face-up or face-down. An effect
  that makes you declare several Types asks one Type at a time, "Select Type 1 of 2." and so on, without the ones you
  already named. A number the number window cannot hold is narrowed in a few steps of ranges ("1000-75000") until you
  pick the number itself. No card in the game today asks for these, so you may never see them.
- The Stadium's Ultimate Match can be played against the computer: if no other player joins you within about half a minute
  of pressing Ready, a computer opponent plays you instead. A win counts as a star like a win against a player. Hosts can
  change the wait or turn it off with the `ultimate-cpu-wait` setting.

## v0.1.6 (2026-11-20)

Multiplayer: update to v0.1.6 to join the hosted server after deployment.

- Cards from a pack, structure deck or exchange are in Edit Deck right away. Before, the cards you bought only showed there
  after you opened the shop again or signed in again.
- Desktop is calmer. Messages now float as a toast at the bottom of the window instead of pushing the screen around: good
  news goes by itself after a few seconds, a problem stays until you dismiss it, and Settings' jobs show their progress
  there too.
- **Open the game without the launcher** is one tick, asked once in setup (off unless you tick it) and kept in Settings →
  Game. Ticking it sets it up right away; if your game folder is protected, Windows asks for permission then. Say no and it
  simply stays off: PLAY no longer asks you to **Allow** every time.
- Anime pictures now sit right under the **Anime cards** tick in Settings → Singleplayer, shown while it is on.
- A new footer: a red-heart **Support us** on the left, the version in the middle, and **Help** and **Settings** on the right
  (Settings moved there from the title bar). Help opens a small window: read the guide, ask the community, or report a
  problem; every problem's toast also has its own Report link.
- Settings is easier to read: each section is short groups of rows, with the setting's name and one line on the left and
  its control on the right, and switches for everything that is on or off.
- Fixed: **Create account** always has the **Invite code** box now, unless the server says anyone can join (#70). Before,
  it only appeared once the server had answered, so it could be missing.
- Fixed: choosing another game folder in setup no longer asks Windows for permission to tidy a folder another launcher set
  up; the anime picture count refreshes when you come back to it; after Sign out you can type your User ID straight away;
  Cancel during a slow start no longer shows "Start your server first."; REPAIR checks the files again first, so a file you put back yourself turns it back into PLAY.

## v0.1.5 (2026-11-20)

Multiplayer: update to v0.1.5 to join the hosted server after deployment.

- Anime card pictures without anyone hosting them (#60). Settings → Game has a new **Anime pictures** line ("12 of 418"): **Get
  pictures** downloads them (about 10 MB) from ProjectIgnis' picture server when you press it, one card at a time, with a
  count and **Cancel**, and pressing it again only fetches what is missing. **Add pictures from a file…** takes a zip of
  pictures and **Save pictures to a file…** makes one to share with a friend. It all works without any server, so Singleplayer
  gets them too. Pictures show from your next PLAY; a card without one keeps the game's own picture with the anime card's
  stats and text, and pictures you put there yourself are never replaced.
- New **Anime cards** switch in Settings → Singleplayer (off by default): play some cards as they were in the anime. Turning
  it on while pictures are missing opens a window offering to get them (about 10 MB), with a progress bar; the switch stays on
  whichever you choose. It takes effect from the next game start, and an open game offers RESTART THE GAME.
- Desktop now shows a progress bar where it waits: the picture download, an update downloading in What's new, a game starting
  or closing, the setup wizard's folder check, and Settings' longer jobs (a moving band when it can't tell how long).
- Desktop's Settings now says "YO3 Revival's own file in the game folder needs repair. The next PLAY puts it right." when the
  game folder holds our loader without its receipt and "Open the game without the launcher" is on (it only said the next PLAY
  sets it up). The "Starting a game as…" line under Play as another account no longer stays on screen after that game has started.
- Desktop fixes: Cancel or CLOSE GAME during a start no longer leaves a red "The game could not start" notice; a failed
  Play as another account now says so on the Multiplayer tab, with Repair; Escape closes the topmost dialog or list first;
  the sign-in form puts the cursor in its first empty field; a long server name no longer hides its state in the picker.
- YO3 Revival no longer calls its own `winmm.dll` another program's (#65). If the game folder holds our loader (any version)
  but its small receipt file is missing, Desktop now says it needs repair and PLAY fixes it, instead of warning about "an
  unowned WINMM mod" and offering to move it aside. Another program's `winmm.dll` is still named as before. Sorry to the
  players we told otherwise.
- If the game's connection is cut on your PC, the log now says whether the game's side was reading or writing, how much was
  on its way, and whether Cheat Engine, a debugger or a similar tool was running, so a report can show who closed it. Nothing is
  blocked for those tools. If the shop disconnects you, see the [new troubleshooting row](docs/launcher.md#troubleshooting).
- Several games from one launcher (#64): signed in on a server, **Play as another account…** starts another game signed in
  to a different account (the same account twice is refused). Running games are listed with their account and server and
  can be closed one by one; closing the launcher asks, then closes them all. Each extra game has its own ports and
  connection, its own saved settings, decks and duel logs, and on Linux its own Wine prefix, removed when it closes.
  An update waits until every game is closed. Singleplayer stays at one game.
- Hosts: accounts seen on the same PC or network are linked (#64). A duel between them is played but pays no BP and counts no
  record, rating or title, they can't trade or rent to each other, and the dashboard shows an account's linked accounts.
  `linked-accounts off` turns that off, and `games-per-device` (default 1, which changes nothing until you raise it) caps
  the games one PC runs. See [hosting](docs/hosting.md#linked-accounts-and-fair-play).
- Hosts: `clean-game on` makes the launcher refuse to start the game on your server while another program's files or network
  software are inside it, with no Continue anyway (#69). A tidiness rule for friends' games, not anti-cheat. See
  [hosting](docs/hosting.md#clean-game).
- Hosts can turn on anime card versions (#60): `anime-cards on` (a live setting in the admin panel and the command line, off
  by default, at the next duel) makes NPC and player duels play 415 cards by their anime effects (Card of Sanctity, Mirror
  Force, Junk Warrior and others), and the game shows them as the anime cards: type, Level, stats, text and Main or Extra Deck
  (the card profile: every kind of difference is released, from type flags, Spell/Trap kinds, monster types, Main and Extra Deck,
  Level, attribute, race, ATK and DEF to a Monster, Spell or Trap that changes class). Cards and decks stay yours, a changed class moves a card to the other list by itself, and a change of the setting needs a
  restart of the game. Anime artwork is never shipped: put 200x290 PNGs in the launcher's `anime-art` folder. The
  [list](docs/anime-cards.md) names the cards and says why 3 others keep the game's version (Theinen the Great Sphinx's trigger from the Deck, and the two "?" ATK cards Clear Vice Dragon and Blackwing - Aurora, play as the anime has them). When the host changes the
  setting, Desktop offers RESTART THE GAME instead of leaving you with the old cards, and the launcher places the cards of
  your own saved decks in the right lists before the game starts (the first version of a changed deck is kept beside it).
  A game that is still open with the old cards cannot start any duel (Auto Duel, NPC duels, hosting or challenging a duel,
  events): the server refuses it with a chat line telling you to press Play again, so nobody duels with card data that
  differs from what the server plays. With the setting off every game duels as before.
- Two anime cards ProjectIgnis has no real script for are now our own reconstructions, played with the anime setting on and
  marked as reconstructions in the card's text: **Dark Strike Fighter** (Level 7, 2600/1800; once per turn for each copy,
  in either Main Phase, Tribute another monster you control to inflict its Level x 200 damage, and it cannot attack the turn
  you do) and **Fortune Lady Fire** (the official card, but it destroys a monster and burns its ATK whenever it is Special
  Summoned face-up in Attack Position, by anything). Temple of the Kings, Time Wizard and Thousand Dragon keep the game's version on purpose (the list gives the reasons). Skill
  Drain now treats the three Egyptian Gods as the anime has it: the Gods already on the field are negated until that turn
  ends and then come back (Ra's ATK is 0 for the rest of that turn), and a God summoned while Skill Drain is face up is not
  affected. Each activation counts on its own: a second Skill Drain negates the Gods on the field again for that turn. See [the list](docs/anime-cards.md#reconstructions).
- The Stadium on the map now has events (#18). Your host schedules Gunslinger (win streak), Duelist Trial and Anniversary
  Trial (wins), and Evolution and Ultimate Match (stars, played as matches). Open one, press Ready and you are paired with
  another player in the same event, and matched again after every duel until you press Not Ready. Your score for the event
  shows in the lobby and on the result screen, the prize (BP) arrives by D-Mail when the event ends, and Tournament Points
  on My Profile add up from the events you played. Everyone counts as premium here.
- Main Menu's Leaderboard and the team ranking now show everyone's real wins and losses against other players, best first
  (#16). The Leaderboard shows your own country's players when the game opens it with one.
- Hosts: `create-event`, `events`, `edit-event` and `cancel-event` (and a new Events page in the staff dashboard) schedule
  the official events with BP prizes for taking part and for reaching a score, optionally under one banlist. See the
  [hosting guide](docs/guide.md#official-events-the-stadium).

## v0.1.4 (2026-11-19)

Multiplayer: update to v0.1.4 to join the hosted server after deployment.

- Duels can be given up (#57): click your own Deck pile, choose Surrender and answer Yes (from the fifth turn on, as the original game's text says). The other player wins, in NPC duels, Auto Duel and challenge duels; a host who turns Surrender off in Create Duel leaves it out.
- The Extra Deck lights up only when you can summon from it (#16). To look through it at other times, click it and choose See Details.
- Cards that target (a Spell aimed at a monster) now show the lock-on cursor on each target with the aiming marker until the effect resolves, and a shuffled Deck plays the shuffle animation, as the original game did (#52).
- Hosts: the staff dashboard now asks every admin and moderator for an authenticator code (TOTP) after the password.
  A staff account that has not set one up reaches only a setup screen with a QR code and ten one-time recovery codes.
  Sensitive actions (roles, deletions, setup codes and keys, settings, backups, stopping the server) ask for a fresh code,
  and "Sign out everywhere" and new recovery codes are under Settings. A lost phone is fixed on the server with
  `yo3-server staff-mfa-reset USER`. Back up `data/staff-mfa.key` with your other secrets. While staff enroll, `--staff-mfa optional`
  (`YO3_STAFF_MFA=optional` for Compose) keeps password-only sessions for accounts without an authenticator; remove it afterwards.
  Public backup downloads stay off unless you start the server with `--dashboard-public-backups`. See [hosting](docs/hosting.md#staff-sign-in-with-an-authenticator) (#43).
- PLAY checks for other programs running inside the game (#68). Another program's file in the game folder now opens a dialog with **Move it aside** (Settings → Advanced puts it back) or **Continue anyway**, with advice for a broken network setup or security software. A program that changes the game's own connection stops PLAY, since the game can't work with it.
- Opening `yo3.exe` directly now uses the higher resolution your last PLAY used (#16).
- Clicking the launcher's footer version opens Settings → About. The update notice still opens What's new.
- Chat commands: the server now answers a few commands in your own chat window, nobody else's. Send them as a private message to **Server** (`/p Server banlist`; the game keeps `/p Server ` in the box) or start a chat line with `!`. [Playing guide](docs/playing.md#chat-commands).
  - `banlist` shows the banlist your Auto Duel and computer duels use, and `banlist unlimited` (or `season 16`, any name or number from the list) chooses your own, so the Yugi, Jaden and Yusei decks no longer need editing for the host's list (#63). It applies the next time you log in, and Auto Duel only matches players on the same banlist. `banlist default` goes back to the host's.
  - `draw5` (or `draw 7`, or `draw 5 part of a deck name`) draws random cards from your newest deck, to see what an opening hand could look like (#58).
  - `where red-eyes` tells you where a card comes from: which booster packs, structure deck, Blister's Trade Card pool, the shop's Rental or starting deck holds it, or "no normal route" (#55).
  - `report PLAYER reason` tells the host's staff about a player, without leaving the game; a few reports an hour (#44).
- Hosts can set what the shop asks: a booster pack's and a structure deck's BP, the number of cards a Trade Card swap takes, and the Rental's cards, BP and days, as live settings (#16). The staff dashboard has a Reports page for the `report` command, and shows each account's banlist, which staff can change.
- No player can be named "Server", the name the chat commands go to.
- Searching cards in Edit Deck and at Blister's ignores upper and lower case: "red" finds Red-Eyes (#54). A search still looks inside the cards listed; Reset (the circular arrow) lists them all again.

## v0.1.3 (2026-11-18)

Multiplayer: update to v0.1.3 to join the hosted server after deployment.

- GitHub update checks, downloads, the official server list and bug reporting use bundled Mozilla certificates,
  so an outdated Windows certificate store no longer prevents secure connections. If update checks already fail,
  download this version manually once. Windows 7 remains unsupported.

- Play warns when an unrecognized `winmm.dll` is in the game folder, without changing or removing it.
  Connection errors now identify whether the game side or server side failed. The reported shop disconnect
  remains under investigation.

- The playing guide explains which monsters contribute to Junk Warrior's ATK gain, backed by new regression tests.

- Developer tools are grouped by purpose, with a shared local/CI check command, scratch-server setup and Windows VM helpers.

- Release CI uses measured browser/build concurrency on the home runner, reducing component times without
  dropping checks. Local image builds keep their conservative compilation default.
- Adding Rust integration tests no longer makes release images rebuild unchanged third-party dependencies.
  All existing release checks remain required, with an additional cache-invalidation regression.

## v0.1.2 (2026-11-18)

- The staff dashboard’s card picker now shows original game thumbnails and a larger preview with readable original
  effect text. Images stay on the host, load on demand and use a private browser cache; missing artwork never prevents
  giving a card.
- The home runner keeps Cargo build output locally instead of transferring a large Rust cache archive to GitHub.
  The cache is bounded, survives checkout cleanup and leaves installed tools intact; release checks are unchanged.

## v0.1.1 (2026-11-18)

- The Windows game-runtime check now falls back to the game’s own DLL search when Windows rejects newer
  loader flags. This fixes a false “missing DirectX” result on older systems; Windows 7 remains unsupported.
- Dashboard dropdowns now use shadcn controls with keyboard navigation and consistent light/dark styling.
  Settings groups can collapse; searching reveals matching settings. Invalid fields show a specific inline error,
  keep focus on the first invalid input, and clear errors when corrected or reset.
- The staff dashboard has a saved light/dark toggle and a System appearance option. Success, warnings, errors,
  information, fields and dialogs now share a consistent palette in both modes.
- Server settings are grouped by purpose, with readable names and units, group/search/custom-value filters,
  and visible default/custom states. Additional server settings remain available under Audit & other.
- Accounts now have their own dashboard pages at `/accounts/<id>`, with player cards, rewards, access controls,
  profile and moderation sections. Closing an action keeps the account open; balance changes refresh in place.
  Direct links and reloads are supported by nginx, so this routing change ships with a full release.
- Dashboard sign-in now uses the project logo and a compact centered form. Failed sign-ins keep their error
  visible, including after session expiry. Tablet navigation, audit/invite/code/backup rows, log insets and
  backup confirmations now follow the same responsive layout as the account pages.
- Dashboard account/player actions now fit phones and tablets. Long dialogs keep their actions visible, searches
  can be cleared, and card gifts list artworks with icons, an official card-database link and checked copy counts.
- Compatible dashboard changes can deploy automatically from `main` without restarting the game server or requiring
  player updates. The generated API must match the deployed release, and runtime/browser checks precede delivery.
- The dashboard shows its own build alongside the server version and offers an explicit reload when new files are
  available. Open forms stay intact, and missing page files get a recovery message.
- Release and dashboard delivery share a FIFO queue on the `self-hosted, home` runner. Cleanup protects the running
  and previous successful dashboard alongside retained releases, after a disposable package-deletion probe.

## v0.1.0 (2026-11-17)

Hosts: releases now update the configured VPS automatically after all builds pass, with a pre-upgrade backup and
rollback if startup fails. Multiplayer servers require launchers at least as new as their own release; older players
are told to update before playing. Container builds require the latest published release tag at the exact source
commit. After successful deployment, GitHub keeps that release and the previous version, removing older beta downloads
and container versions; the alpha releases (v0.0.x) are always kept. The updater check always starts from the latest published Desktop bundle.

The server container now uses a minimal scratch runtime with its required libraries and health/admin tools, still
running as an unprivileged user. Builds cache compiled Rust dependencies across releases, avoid recompiling for card
override edits and remove duplicate frontend API checks. Temporary files cannot execute or gain privileges; image
checks cover the packaged engine, runtime permissions and actual cards before deployment.

The first beta: a new launcher, accounts with a User ID and password, and a server that can be hosted for real.
Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server, rules
engine and required connection components. Original game files are not included. **Windows 7 is not supported.**

**The beta starts fresh.** Everyone who plays on the same server updates together: a v0.0.x launcher can't join a beta
server, and the beta doesn't read v0.0.x invitation files or saved connections. Your v0.0.x Singleplayer collection is
left untouched in the app's `server` folder, but the beta's Singleplayer starts a new one. Hosts: back up your whole
data folder, with the server stopped, before updating. On Desktop, click the version, then **Review & install…** and
**Download & restart**, as before; from v0.1.0 on, updates download by themselves. See the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.1.0/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.1.0/docs/scripts.md).

- **A new launcher that looks like the game's era.** A first-run setup finds and checks your game, then the main
  screen opens straight away from then on. **Singleplayer** is one PLAY: your own server starts with the game and
  stops after it. Settings, Help, **Report a problem** and plain error messages, with the technical details behind
  **Details**.
- **Multiplayer with a User ID and password.** The project's **YO3 Revival** server is listed already. Add another
  server by address or with an invite link from its host, create
  an account with an invite code, sign in, and stay signed in if you like. Save the recovery code you get at sign-up:
  **Forgot password** uses it, or a reset code from the server's moderators.
- **Opening `yo3.exe` on its own still signs you in,** to whichever you last played. After **Delete Login Data**, the
  game asks for your User ID and password itself, as the original did.
- **Each server keeps its own local game data**, so deck saves from one server don't mix with another's.
- **Updates you can see:** a dot on the version and What's new, downloaded in the background only when the release
  carries the project's signature, and installed when you choose to restart.
- **Host for friends** from the launcher, or on a Linux server with the Server bundle. Hosts get an admin panel, live
  settings without a restart (banlist, life points, clocks, news, registration), moderation (kick, restrict, ban,
  mute, the game's own maintenance countdown), backups with restore, and an audit log of every change. A separate web
  dashboard and a Docker setup with HTTPS are included for servers on a VPS ([deployment guide](https://github.com/usersina/yo3-revival/blob/v0.1.0/docs/deployment.md)).
- **Attacks are aimed on the field.** Hover a monster to aim the arc at it, click to attack, right-click to take the
  attack back, as the original did. Before, a list of targets opened.
- **The card info panel** now shows a hand monster's level, and a Graveyard monster's attribute, Type and level.
- **Fixed:** a duel ended as a loss for the player who was waiting for their opponent's move, when their game had
  gone quiet for three minutes; that player's game also stayed stuck on the field.

## v0.0.26 (2026-11-17)

A duel fix: effects you can use during a phase are asked about again. Choose **Desktop** for the graphical app or
**Scripts** for terminal commands. Both editions include the server, rules engine and required connection components.
Original game files are not included. **Windows 7 is not supported.**

Players and hosts should update: the fix is in the server, so it reaches your duels once whoever runs the server you
play on (your own for Singleplayer, the host's for friends) has updated. Friends on v0.0.13 or later can still join.
Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.26/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.26/docs/scripts.md).

- **Effects you can use during a phase are asked about again** (#53). When Stardust Dragon negated an effect in your
  opponent's turn, its "Special Summon this card during the End Phase" never asked you, and it stayed in the Graveyard.
  When such an effect came up together with another card you could activate, the game's default **Activation
  Message** setting (Semi Auto Cancel) answered No for you. Questions in the Draw, Standby, Battle and End Phases now
  always show on the default setting. You may see the question a little more often, for example when a Set Spell or
  Trap could be activated at the End Phase.

## v0.0.25 (2026-11-16)

A startup fix for Windows 10 version 1809 (LTSC 2019). Choose **Desktop** for the graphical app or **Scripts** for
terminal commands. Both editions include the server, rules engine and required connection components. Original game
files are not included. **Windows 7 is not supported.**

Players on Windows should update; hosts need nothing new. Friends on v0.0.13 or later can still join. Before updating,
close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.25/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.25/docs/scripts.md).

- **The game starts on Windows 10 version 1809 (LTSC 2019) again.** It closed at once with `0xc0000139`, or with
  "The procedure entry point waveInGetDevCapsW could not be located", when opened directly. On that Windows, its own
  sound library loads the `winmm.dll` YO3 Revival puts beside `yo3.exe`, and that file had only the few functions the
  game itself uses. It now passes every other Windows multimedia function on to Windows. The next **Play** updates
  the file in your game folder. If you replaced it with a copy of Windows' own `winmm.dll` as a workaround, delete
  that copy first, so YO3 Revival can put its own back.

## v0.0.24 (2026-11-16)

A duel fix. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the
server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

Hosts should update: the fix is in the server, and players need nothing new. Friends on v0.0.13 or later can still
join. Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.24/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.24/docs/scripts.md).

- **Coin tosses and dice rolls are shown.** A card that tossed a coin or rolled a die (Time Wizard, Sasuke Samurai #4,
  Graceful Dice and others) showed nothing, only its outcome, so it could look as if the result was always the same.
  The duel now plays the coin and dice animations with the real result: each coin's face, highlighted when it lands
  on the side you called, and the die's number. The results were always random; now you can see them.

## v0.0.23 (2026-11-16)

A duel fix. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the
server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

Hosts should update: the fix is in the server, and players need nothing new. Friends on v0.0.13 or later can still
join. Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.23/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.23/docs/scripts.md).

- **Choosing a monster's position no longer freezes the duel.** When you Special Summoned a monster while your opponent
  (an NPC, or another player) still had another copy of it hidden in their hand or Deck, the position window could
  show no card on either button, with nothing to click. Reported with Doom Shaman; it now always shows the card.

## v0.0.22 (2026-11-16)

A shop fix. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the
server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

Hosts should update: the fix is in the server, and players need nothing new. Friends on v0.0.13 or later can still
join. Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.22/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.22/docs/scripts.md).

- **Every pack in the shop can be bought again.** Cards from Blister's **Trade Card** list also showed on the
  **Purchase Pack** shelves, as "packs" at 10 BP, and buying one always said "An error occurred." They are gone from
  the shelves; the real packs, at 100 BP, were never affected, and Trade Card works as before
  ([#46](https://github.com/usersina/yo3-revival/issues/46)).

## v0.0.21 (2026-11-15)

Token fixes. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the
server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

Hosts should update: the fixes are in the server, and players need nothing new. Friends on v0.0.13 or later can still
join. Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.21/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.21/docs/scripts.md).

- **Choosing a token's position no longer freezes the duel.** Gorz the Emissary of Darkness's token (and those of
  Fiend's Sanctuary, Phantom Skyblaster and others) showed an empty position window that couldn't be clicked; it now
  shows the token.
- **Sweet Corn's Grain Corn Token now appears on the field.**

## v0.0.20 (2026-11-15)

Fusion Summons fixed. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions
include the server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

Hosts should update: the fixes are in the server, and players need nothing new. Friends on v0.0.13 or later can still
join. Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.20/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.20/docs/scripts.md).

- **Fusion Summons from the hand no longer crash the game.** Polymerization with its materials in the hand crashed
  the game as they swirled into the vortex, whether they were the listed materials or King of the Swamp in one's place
  ([#40](https://github.com/usersina/yo3-revival/issues/40)).
- **Two copies of a card are two materials.** A Fusion Summon with two copies of the same card from the hand
  (Ultimate Ancient Gear Golem's two Ancient Gear Golems) now shows both copies going into the vortex, not one of them
  twice.

## v0.0.19 (2026-11-15)

Safer saves and hosting. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions
include the server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

Hosts should update: most changes are in the server. Players need it too for the game-folder fix in Desktop. Friends on
v0.0.13 or later can still join. Before updating, close the game and stop hosting. Keep your server data and local
game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.19/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.19/docs/scripts.md).

- **Failed account and player saves are now refused.** If the server can't save those changes (a full disk, a
  locked file), a BOOSTERPASS code, a duel's BP, a title, a trade, a rating, a deck upload or an avatar change is refused
  instead of seeming to work and being lost at the next start.
- **One server per data folder.** Starting a second server on the same data (Desktop and Scripts together, or the same
  server twice) is now refused instead of both running and overwriting each other's saves.
- **Changing the game folder no longer gets stuck.** It used to ask you to turn off direct launch first, an option
  that is gone. Now the old folder is cleaned up for you, and a Windows registry entry that was already missing no
  longer stops the cleanup halfway.
- **Tighter checks against modified clients.** A duel closed to watchers can no longer be watched by going around the
  menu, one player can't take both seats of a duel, and an avatar must have the game's own size.

## v0.0.18 (2026-11-15)

Titles to earn, working website links, Auto Card Placing, and opening the game directly with nothing to set up.
Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server, rules
engine and required connection components. Original game files are not included. **Windows 7 is not supported.**

Hosts and players should both update: titles and Auto Card Placing come from the server, the website links need
both, and opening the game directly comes with the app. Friends on v0.0.13 or later can still join. Before updating,
close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.18/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.18/docs/scripts.md).

- **Titles you can earn.** Lose to an NPC, master all its levels, win 10 times against it, play duels, summon high-level,
  Fusion and Synchro Monsters, win with Exodia or after falling to 1000 LP, collect cards, wear an Afro… about 120
  titles now come from playing, each announced in chat. Wear them in Avatar Settings. Hosts can hand out the special
  ones with `yo3-server grant-title`.
- **No more dead links.** Leaderboard, team ranking, the BOOSTERPASS page and the title screen's Official Website used
  to open Konami's long-gone website. They now open a short page from your host's server instead. Both the host and the
  player need this version for it.
- **Auto Card Placing works.** Turn it OFF in **Main Menu → DUEL OPTIONS** and you choose the zone for every monster
  you summon or set and every Spell or Trap you play; ON (the default), the game places them for you as before.
- **Open the game directly, with nothing to switch on.** Press Play once in YO3 Revival and from then on `yo3.exe` or
  its shortcut connects to the server and account you last played on. The "Allow opening yo3.exe directly" checkbox is
  gone. **Delete Login Data** on the title screen signs that PC out until you press Play again.
- **Shop errors explained for hosts.** When a purchase fails, the game only says "An error occurred."; the host's
  server log now says why, which player and which pack.

## v0.0.17 (2026-11-15)

Summons look like the original game's again. Choose **Desktop** for the graphical app or **Scripts** for terminal
commands. Both editions include the server, rules engine and required connection components. Original game files are
not included. **Windows 7 is not supported.**

Only the host needs to update: these changes are all in the server, and friends on v0.0.13 or later can still join.
Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.17/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.17/docs/scripts.md).

- **Summons look like the original again.** Tribute Summons show the tribute leaving, Fusion Summons play the full-screen
  Fusion vortex with the materials swirling away, and Synchro Summons play the original Synchro animation: the Tuner's
  rings, the materials passing through, the monster bursting out of the light. Synchro Summons also use the avatar's
  Synchro line. Contact Fusions (Gladiator Beasts) play the Fusion animation too, and Flip Summons play the avatar's
  summon cut-in.

## v0.0.16 (2026-11-14)

Avatar Settings works again, and Pegasus has met his match. Choose **Desktop** for the graphical app or **Scripts** for
terminal commands. Both editions include the server, rules engine and required connection components. Original game
files are not included. **Windows 7 is not supported.**

Only the host needs to update: these changes are all in the server, and friends on v0.0.13 or later can still join.
Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.16/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.16/docs/scripts.md).

- **Fixed: Avatar Settings had nothing to pick.** Hair styles, eyes, clothes, the duel disk and titles were empty
  lists. Every player now starts with the game's free parts and the BEGINNER DUELIST title, and items bought in the
  shop show up next to them.
- **A new look shows straight away.** Other players see your changed avatar without you logging in again.
- **Pegasus can't read a Millennium Eye holder.** Wear Bakura's Accessories (the Millennium Eye, sold in the shop) as
  Facial Accessory 2, and Pegasus stops seeing your hand and Set cards, and lets you know he's not happy about it.

## v0.0.15 (2026-11-14)

Cards that play by the text on them, and a way to give rented cards back early. Choose **Desktop** for the graphical
app or **Scripts** for terminal commands. Both editions include the server, rules engine and required connection
components. Original game files are not included. **Windows 7 is not supported.**

Friends on v0.0.13 or v0.0.14 can still join a v0.0.15 host: the connection files work together. The card changes
come from the host's rules engine, so they apply once the host updates; the Return button appears for each player who
updates. Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.15/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.15/docs/scripts.md).

- **Cards play by the text you read.** 38 more cards whose rules changed after 2012 now do what the game's own card
  text says: Sangan, Witch of the Black Forest, Sinister Serpent, Rescue Cat, Dark Strike Fighter, Catapult Turtle,
  Goddess of Whim, Brionac and Mind Master have no once-per-turn limit; each Ancient Fairy Dragon, Dewloren, Second
  Coin Toss and Red-Eyes Darkness Metal Dragon can use its effect once per turn on its own; Dark Magician of Chaos adds
  its Spell when it is summoned; Disk Commander draws every time it comes back from the Graveyard; Exchange of the
  Spirit needs 15 cards in your Graveyard only; Makyura lets you activate any number of Traps from your hand, however
  it reached the Graveyard; Ring of Destruction takes any face-up monster in either turn and hits both players for its
  ATK; Chaos Emperor Dragon deals 300 for every card it sends; Temple of the Kings lets every Trap be activated the
  turn it is Set; Mausoleum of the Emperor's LP payment is part of your Normal Summon; Imperial Custom's Continuous
  Traps can't be destroyed at all. Crush Card Virus also checks the cards your opponent draws until their 3rd turn;
  Brain Control takes any face-up monster, Synchro and Fusion Monsters too; Necrovalley lets a card in the Graveyard
  use its own effect; Darkness Approaches keeps the monster's battle position; you pay for Imperial Order only in your
  own Standby Phase, and may choose not to; Future Fusion sends its materials as soon as it is activated; Ultimate
  Tyranno must attack first, and attack every monster; Twin-Headed Behemoth comes back by itself; Night Assailant can
  return another Night Assailant; and Y-Dragon Head, Z-Metal Tank, W-Wing Catapult, Heavy Mech Support Platform,
  Armored Cybern and the Machina Unions follow the old Union rules on your card text (back in Attack Position, and
  the first three only protect from battle).
- **Give borrowed cards back early.** In **Main Menu → Rental Card List**, the **Borrowed** tab now has a Return button
  (the round arrow under the list): pick a rental and press it, and the cards go back to their owner straight away, as
  if its term had ended. It works for rentals from friends and from the shop, and with servers from v0.0.13 on, so
  hosts don't need to update for it.

## v0.0.14 (2026-11-14)

A real banlist by default, and a host setting to choose it. Choose **Desktop** for the graphical app or **Scripts** for
terminal commands. Both editions include the server, rules engine and required connection components. Original game
files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.14/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.14/docs/scripts.md).

- **A real banlist by default.** Auto Duel and duels against the computer now use **Season 16**, the newest
  Forbidden/Limited list the game has, as on the original server. Hosts can pick another one (or "unlimited") in
  Desktop under **Hosting → Overview → Banlist** (Normal mode shows it with a **Change** link), or with
  `--regulation` in Scripts. The Yugi, Jaden and Yusei character decks hold cards Season 16 forbids or limits, so edit
  them before using them there; your starter deck is fine.
- **Fixed:** Desktop's dropdown lists (Resolution, Connect to…) were see-through, their choices mixed with the page.

## v0.0.13 (2026-11-14)

Duel windows that speak the card's words, a bigger and sharper game, NPC duel logs, fairer clocks and the shop's
Trade Card and Rental. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions
include the server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

**Everyone needs this version:** the connection files changed, so a host and their friends must all update together.
Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.13/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.13/docs/scripts.md).

- **You choose the zones.** Ground Collapse, Seismic Shockwave and the other cards that make zones unusable now ask you
  with the game's own message, and you click the zones; Senet Switch asks where to move. Unusable zones show a red
  cross. Cards you summon, Set or activate fill the middle zones first, as on the original server.
- **You choose the order.** Big Eye and the other cards that put cards back on your Deck, and effects that trigger
  together, let you click them in order; the first click goes on top.
- **Revealed cards are shown.** Ancient Telescope, Reversal Quiz and other looks at a Deck turn its top cards over on
  the pile, a Set card shown to you flips up for a moment, and with Convulsion of Nature both Decks show their top card.
- **Changed Types and Attributes show.** After Scroll of Bewitchment, DNA Surgery or Zombie World, the card's label and
  info panel show the new Attribute or Type.
- **The duel windows speak for themselves.** A card's effect choices now show as buttons with the card's own words
  (Pyramid Energy: ATK or DEF), Synchro Summons ask for "Synchro Material", and a card's own question or hint shows in
  its window. Big Eye no longer asks you to "Declare monster level". Choices with more than four effects still list
  them in the chat window.
- **Duel logs for NPC duels.** A duel against an NPC is now saved like one against a player: find it in Duel Log →
  Latest Duel Logs and press Play to watch it again.
- **Fairer duel clocks.** Your clock no longer runs while the game is still playing animations: your game now tells
  the server when it has finished them, so a slower computer no longer loses thinking time.
- Your avatar speaks its line when you flip a Set card or reveal a hand card to activate it, as in the original.
- **A bigger, sharper game (optional).** Desktop: Advanced → Game setup → Display → Resolution; Scripts: add
  `--resolution 1440x1080` (or `auto`) to `play`. The game keeps its 4:3 shape, with black bars on a wider screen;
  card pictures stay as detailed as their files. Opening `yo3.exe` directly still uses the original size.
- **Trade Card at the shop.** Give Blister 10 cards you don't need and get any card you pick from the first 48
  booster packs.
- **Rent from the shop.** Borrow 5 cards of your choice for 100 BP for 7 days, to try a deck before you build it.
- **Fixed:** a token equipped to Allure Queen stayed on the field after it left.
- **Fixed:** summoning a Psychic with Brain Research Lab's extra Normal Summon could stop the duel with an error.
- **Fixed:** Card Trooper could send a different number of cards than the one you picked.
- **Goyo Guardian plays by its card text:** any Tuner will do for its Synchro Summon, not only an EARTH one.
- **Fixed:** with two Double Spells in your opponent's Graveyard, a duel could freeze or stop with an error.

## v0.0.12 (2026-11-13)

Decks that follow you, an Official deck tab and fixes for getting stuck or thrown out. Choose **Desktop** for the
graphical app or **Scripts** for terminal commands. Both editions include the server, rules engine and required
connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.12/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.12/docs/scripts.md).

- **Your decks follow you as soon as you save them.** Saving a deck in Edit Deck (Save or New) now sends it to the
  server, so it shows up in Load Deck when you play from another computer, without using it in a duel first. A
  computer that already has a deck of that name keeps its own copy.
- **Official decks have their own tab.** The Yugi, Jaden and Yusei character decks, your starter deck's original
  recipe and the structure decks you bought are now under Edit Deck → Load Deck → **Official**, which used to be
  empty. Your **Deck** tab keeps your own decks, so they all fit. To duel with an official deck, load it and save it
  with New. Copies you already have stay where they are.
- **Fixed: stuck at rock-paper-scissors.** If your opponent left a duel while you were choosing rock, paper or
  scissors, you stayed on that screen for good and had to close the game. You now see "(name) has disconnected." and
  return to the map, with nothing counted.
- **Fixed: the game closing when the host goes away.** If the host's server stopped or crashed, or the network
  dropped, the game itself crashed. You now see "You have been disconnected" and go back to the title screen. A host
  whose computer went to sleep or lost its network is noticed within about a minute and a half too.
- **Fixed: waiting for an opponent who never shows up.** If your opponent's game closed after choosing a deck, you
  waited for good. After 90 seconds the duel is cancelled and you're sent back. In events, a match nobody joins now
  ends on its own so the tournament goes on.
- **Each server keeps its own decks.** Everyone starts as "Duelist", so joining a second server showed the first
  server's decks (which it might refuse) and skipped its starter deck. Each server now has its own local game data.
  The first server you play on keeps your current decks; any other server starts with your display settings and a
  fresh starter deck.
- **Fixed:** a duel against a computer opponent the server doesn't have now stops at deck selection instead of
  leaving you stuck after rock-paper-scissors.

## v0.0.11 (2026-11-13)

A fairness fix for duels against the computer. Choose **Desktop** for the graphical app or **Scripts** for terminal
commands. Both editions include the server, rules engine and required connection components. Original game files are
not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.11/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.11/docs/scripts.md).

- **No more peeking at computer opponents' face-down cards.** In NPC duels the attack-target window and the card
  panel showed a face-down monster's name, art and ATK/DEF. They now show a card back until the card is revealed, as
  in duels between players.

## v0.0.10 (2026-11-13)

Computer opponents that think, talk and have a character of their own, and much smaller downloads. Choose
**Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server, rules engine
and required connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.10/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.10/docs/scripts.md).

- **No more Node.js inside:** the rules engine is now a small program of its own, built from the same EDOPro core,
  instead of the core running inside a bundled copy of Node.js. Downloads are much smaller and hosts still install
  nothing. Duels play exactly as before: every test duel was played through both engines and they agreed.
  Updating from v0.0.9 or older works as usual from the app.
- **Computer opponents actually play now.** Kaiba and the other NPCs used to summon their first monster and attack
  with everything. They now think about each move: they try their options ahead within their turn, use their Spells
  and Traps (removal when you summon, Ritual Summons, Set Traps waiting for your attack), choose good targets and
  keep blockers up, and they think through your next turn before committing, in about a second at most. They
  play fair: they never peek at your hand, your Set cards or anyone's Deck (with a few famous exceptions below).
- **Each opponent has a character:** Kaiba and Jack Atlas attack relentlessly, Syrus and Luna play it safe, and
  Maximillion Pegasus... reads your mind. His Millennium Eye sees your face-down monsters and Set cards.
- **The heroes believe in the Heart of the Cards:** Yami Yugi, Yugi Muto, Jaden Yuki and Yusei Fudo always draw the
  card they need. Good luck.
- **The opponents talk.** Every NPC has its own lines in the duel's chat window: a greeting, taunts, its big
  monsters by name, its direct attacks, its reaction to a heavy hit, and a word when the duel ends. The heroes call
  on the Heart of the Cards as they draw, and Pegasus tells you which card is in your hand and which one you just
  Set. He is not bluffing.

## v0.0.9 (2026-11-12)

Every card checked against the rules engine, duels that look and sound like the original game, and choices back in
your hands. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the
server, rules engine and required connection components. Original game files are not included. **Windows 7 is not
supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.9/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.9/docs/scripts.md).

- **Duels show what the original game showed,** worked out from duel logs recorded on Konami's own server in
  2010–2012: Life Point changes show their number and sound, a Field Spell changes the duel field's background,
  Sets and activations play their effects, and your avatar speaks your Avatar Settings lines when drawing,
  summoning, activating, Setting, ending a turn and taking damage (the game's "speech" duel option turns it off).
  Moves play the right sound, and a surrender reads as in the original.
- **Every card checked:** each of the game's cards now has to load in the rules engine with the game's own Level,
  ATK and DEF, and each is played through a full test duel, as are the decks people used on Konami's server. This
  found and fixed:
  - Barrel Dragon and Red-Eyes Darkness Metal Dragon didn't work in v0.0.8.
  - A deck's second-print Dark Magician or Polymerization silently vanished from the duel.
  - Placing a card on the opponent's field (Give and Take, Black Garden), disabling two zones (Ground Collapse) and
    removing counters from several cards (Cloudian - Altus) could stall the duel.
  - Machina Fortress couldn't be Special Summoned, and Doomsday Token, Grinder Golem's tokens and Slime Tokens never
    appeared.
- **Your choices are yours:** a card with two effects you can activate at once asks which one; an effect's options
  the game has no label for show as cards to pick from instead of the first option being chosen for you; and when
  cards must add up to a Level (Machina Fortress's discards, a Ritual Summon's tributes) you pick them, then confirm
  with a right-click.
- **Replays show both players' cards,** as the original's did, and duel log files match the original's layout.
- **Decks the duel can't play are refused:** a Fusion or Synchro Monster in the main deck (the game's deck editor
  doesn't allow it either), and the 12 game exclusives the rules engine doesn't have (Ulevo, Xexex and others; no
  pack or preset deck holds them).
- Removed the starting-deck onboarding notice from in-game login news.
- Desktop folders now include `yo3-revival.ico` and `yo3-revival.png` for custom shortcuts. The Windows executable
  continues to contain the launcher icon itself.

## v0.0.8 (2026-11-12)

A new look for the Desktop app, BP that shows up right away, and BOOSTERPASS codes you can manage from the app.
Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server,
rules engine and required connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.8/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.8/docs/scripts.md).

- **A new look for the app:** calm dark panels with small corner labels and one blue accent, in the spirit of Master
  Duel's menus, over a backdrop of faint circuit traces like the icon's. Logs keep the original game's green-on-black
  chat style, with errors in red and warnings in amber.
- The app has a new Duel Disk icon with a Monster Reborn badge, shown in the app itself, its window and the Windows
  executable.
- **BP shows up right away:** BP from duels and from the host appears in the game on its next screen, without logging
  in again. Before, the game showed new BP only after a purchase or the next login. A gift from the host also arrives
  as a D-Mail "BP received" (the mail icon lights up, even during a duel).
- **Give BP and BOOSTERPASS codes in the app:** Advanced → Hosting → Players & invitations has Give BP for the selected
  account, and a BOOSTERPASS codes section to make codes (random ones too), see who claimed each and when, and remove
  them. The `codes` command lists claim times too.
- **Duel logs in the database:** the replays the game downloads after a duel between players are now kept in
  `server.db` (the newest 1000) instead of loose files. An old `duellogs/` folder is reported as safe to delete.
- **Clearer logs:** errors are red and warnings amber in the app, and in a server terminal. Old `accounts.json` copies
  are reported as a warning that they are no longer read and safe to delete (accounts live in `server.db`).
- **No firewall prompt when playing alone:** your local server only accepts this computer until you save a sharing
  address for friends. Before, Windows asked to allow `yo3-server.exe`, and choosing Cancel quietly blocked friends
  later. If that happened to you, see Connection help in the Desktop guide.
- **Coming from the Scripts edition:** Hosting → Advanced shows which data folder the app uses; choose your Scripts
  `data/` folder there to keep its accounts and BP. Starting while another server uses the same port now says so.
- Fixed: a shop purchase that could not be saved took the BP without giving the cards. Two games on one account
  can no longer buy the same protector or avatar item twice, and BP given during a session can be spent at once.
- Fixed: a command-line (Scripts) server's certificate expired after a year and friends could no longer connect.
  The server now renews it at start, keeping the same identity, so existing invitations and profiles keep working.

## v0.0.7 (2026-11-12)

A smoother Desktop experience, clearer startup errors and bug reports with diagnostics attached automatically.
Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server,
rules engine and required connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.7/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.7/docs/scripts.md).

### Built-in bug reports

- Added in-app GitHub sign-in and **Publish bug report**. Reports include version/system details, captured errors and
  linked diagnostic logs automatically; the review keeps the public-sharing notice and action buttons visible.
  The smaller **Exclude chat, decks, paths & addresses** option filters the report while preserving original logs.
- Failed report publishing preserves progress across reopening, reuses uploaded logs and checks uncertain requests
  before retrying. GitHub credentials use the system credential store, with a session-only fallback when unavailable.

### Startup and recovery

- Added **Help & support → Reset local game data…** with **Back up & reset**. The default preserves every player file, saved
  deck and selected deck; an explicit full reset moves the entire client folder into a dated backup. Server data
  and Revival connections are preserved, with running-game and folder-overlap checks before changes.
- Fixed intermittent Windows authentication failures caused by TLS response cleanup. Complete, validated replies
  remain successful when the peer resets during closure; incomplete replies and authentication failures still fail.
- Failed Desktop server startup now stops its own unfinished process and permits another Start. Shutdown waits
  before permitting a restart, including when Desktop reconnects to an already-running server.
- Connection timeouts identify the stalled stage in the logs, distinguishing TCP, TLS and discovery response waits.

### Simpler Desktop controls

- Desktop now uses one **Play** button with checking/starting progress. Choosing a game folder checks it and saves
  successful results automatically; Play rechecks required files, and imported invitations verify the server and account.
- Refined Desktop guidance with inline field errors and visible filenames, failure stages and native codes when
  available. Original launch failures stay visible during background checks. Status colors and contextual help remain
  consistent; formatted update notes scroll separately while restart warnings and action buttons stay visible.
- Direct launch is now a checkbox with contextual repair and clear prerequisites. Normal/Advanced controls fit the
  sidebar, and server/account selection visibly locks while an operation is running.
- Missing required files now appear together as exact paths, with a short antivirus/recovery hint. Local Play and
  hosting recheck the bundled server, Node.js and engine dependencies, including when hosting is already running.
- Error notices show the useful diagnosis directly, with a copy icon replacing the details expander. Normal mode
  has a smaller log box that groups repeats and hides Wine diagnostics; exports preserve the original output.
- Removed empty space in short log panels, release notes and forms, with consistent mode labels and dialog spacing.

## v0.0.6 (2026-11-11)

- Desktop logs open at the newest line and follow new output only while you are at the bottom. Scrolling up keeps
  older lines in view; scrolling back down resumes following. The scrollbar now sits at the right edge in both modes.
- Rust package versions now match the application release: `0.0.6`.

## v0.0.5 (2026-11-11)

A simpler launcher, native Windows startup fixes and more reliable duels. Choose **Desktop** for the graphical app
or **Scripts** for terminal start/play commands. Both editions include the server, rules engine and required
connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.5/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.5/docs/scripts.md).

### Simpler play and hosting

- Normal mode is one screen: choose the game folder, **Validate game**, **Start**, and visible logs.
  Switch to **Advanced** for server connections, hosting and full controls without losing settings or drafts.
- The launcher opens in a smaller, centered window so Version, GitHub and Help remain visible on laptop-sized screens.
- Advanced hosting lists the server's actual CLI flags, including independent `--duel-check` controls, with
  validation and per-option reset. New run flags appear automatically.
- Startup has animated feedback, live logs, cancellation and timeouts. Errors stay visible with details, copy and
  log export; diagnostic failures have their own recovery controls. Missing game files and missing Revival files
  get different repair guidance, and early helper/server failures are captured. A folder-picker fallback allows pasting a path.
- Releases provide **Desktop** and **Scripts** editions for Windows and Linux. Archives remain versioned, while
  their inner folders are `yo3-revival-desktop/` and `yo3-revival-scripts/`. Desktop still downloads and installs
  complete verified updates after confirmation; terminal users get start-server/play scripts and a dedicated guide.
- Script defaults can be overridden for bind/data/log paths. Ctrl+C during early game startup now cleans up.
- The README is a short first-play guide; detailed help is organized in `docs/`, with current screenshots in
  root `screenshots/`. Release bundles keep the same documentation layout and check their links.

### Windows startup and game validation

- Fixed Windows game startup requesting administrator access and then failing to inject the connection DLL.
  Desktop and Scripts now run the game with your normal user permissions.
- Fixed the connection DLL rejecting supported Windows games when ASLR changes their load address. Login,
  starter-deck setup and duel diagnostics now follow the actual image base without disabling ASLR.
- Fixed native Windows path handling: a supported game no longer fails runtime validation merely because its
  resolved path uses Windows' extended-path prefix. Game, connection DLL and direct-start paths use the compatible form.
- Game validation checks the supported v1.153 executable's hook signatures and probes its legacy DirectX/VC++ 2008
  dependencies before startup. Missing runtimes get specific repair instructions.
- DLL startup failures include their native stage and available Windows error/exit codes in launcher details and exported
  logs, with credential redaction. Logs also identify the executable fingerprint and connection mod version.
- Hosting now refuses missing, corrupt or unusable game card/deck data instead of accepting players with an empty
  collection. Fix the host's game-folder selection and retry; existing accounts, decks and identity files are kept.
- Added clear antivirus recovery guidance, including which Revival and game-folder files may need restoring after
  a verified false positive. The app does not change antivirus or firewall settings.

### Login and duels

- **ONLINE** now opens player selection directly. The required connection mod skips the unused original login
  form, including when opening the game through the installed direct-start loader.
- Fixed a local gate disconnect race exposed by automatic login, which could crash the client during connection.
- Fixed monsters such as Fortress Warrior and Shield Wing disappearing after surviving a protected battle.
  They remain visible and usable; destruction is shown only when the rules engine actually removes the card.
- Detailed diagnostics now also enables the duel checker on the next hosted-server start. The checker tracks
  battle animations that hide cards, closing a gap that let occupied but invisible slots go undetected.
- Added real-engine regressions for battle protection and Stardust's legal/illegal Debris Dragon combinations,
  plus simulations using all three original Yusei decks from the installed game.
- Fixed counters being removed during control changes and dynamic monster stats resetting during some swaps.

### Logs and bug reports

- Managed game launches automatically retain game output, connection-mod messages and decoded server packets;
  engine stderr is retained too. No capture checkbox needs enabling before a problem happens.
- Current-session and retained-history ZIP exports include full diagnostic detail by default. Optional
  **Remove personal data** filters only the exported ZIP, leaving local logs available for a later full report.
  Credentials remain scrubbed in every mode; nothing is uploaded automatically.
- Duel diagnostics now compare LP, monster stats, counters, turns and phases, and independently inspect the running
  game's state through the connection DLL. Reports reject missing snapshots and explicitly list coverage gaps.
  Exact engine recordings include deck order, choices and dependency fingerprints for developer replay.
- Detailed duel diagnostics allow cold dependency fingerprinting up to two minutes without extending the
  30-second deadline for game actions. Recording errors no longer claim that the engine and model disagree.

### Development and release checks

- All seven Rust crates inherit shared dependency versions, package metadata, one crate version and common lints
  from the workspace manifest. Resolved dependencies, enabled features and `Cargo.lock` are unchanged.
- Four test fixtures now exercise the same assertions on native Windows as on Linux.
- Publishing requires the final Desktop archive to pass the packaged-updater startup check after tests and builds.
- Fixed packaging with MinGW toolchains whose runtime code exceeds the DLL test fixture's old fixed reservation.
  Regression checks preserve its hook addresses, callback relocation and native Windows section layout.

## v0.0.4 (2026-11-10)

A graphical launcher for playing and hosting from one window. Download the bundle for your system, extract the
whole folder, and open **yo3-revival.exe** (Windows) or **yo3-revival** (Linux). Game files are not included;
Linux players also need Wine. This is still a pre-alpha.

### Play, join and host

- **Play** starts your local server automatically, or connects to a saved friend's server. Import a private
  invitation, or enter a server address and confirm its identity before supplying an account key.
- **My server** manages hosting independently of your game. Create accounts, add players and export private
  invitations without stopping hosting. Advanced interface, port and sharing options stay out of the normal flow.
- **Settings** checks your game folder and required connection components before login. Optional **direct launch**
  lets the original game executable or shortcut use your saved connection, with fail-fast component/server checks
  and repair guidance. The original executable and INI stay unchanged.
- **Activity** shows server and game logs. Export the current session or all retained sessions as a diagnostic ZIP,
  with a size/date preview and credential redaction. Nothing is uploaded automatically.
- The sidebar shows the version, Help and GitHub links. Updates are checked at startup; future updates download,
  verify and install a complete bundle **after confirmation**, with recovery to the previous version if startup fails.
- Fresh players receive a legal, selected starting deck automatically. Setup runs only once; existing decks,
  default selections, unfinished decks, and later manual edits or deletions are preserved by automatic setup.
- The launcher uses native Iced controls, icons, question-mark help and four clear pages. Wine options appear only
  on Linux. Starting a server does not request administrator access or change firewall rules.

### Upgrade from v0.0.3 or earlier

**Stop the old server and back up its entire data folder before upgrading.** Extract this bundle into a separate
folder, set the game folder in Settings, then select your existing data folder under **My server → Advanced**
**before starting hosting**. Accounts, cards, BP and records import into SQLite automatically; original JSON files
remain as migration copies. Keep players' local `Documents/Yu-Gi-Oh! ONLINE 3` folders too.

The revival launcher and connection DLL are now required. Accounts sharing one router remain separate through
TLS authentication. Allow the chosen TCP port (**7443** by default); the old raw game ports and browser `/join`
page are retired. Choose each existing account in **Players & invitations**, export its private invitation, and
send it only to its owner. Existing account credentials and player data carry over.

The GUI keeps its preferences and connections in a separate SQLite database in private application storage.
Self-managed server certificates are maintained when starting through the GUI, preserving the private identity key.
Existing launchers from v0.0.3 and earlier have no updater: download this release manually. The
[illustrated setup and upgrade guide](https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md)
walks through the complete process, backups and rollback limits.

### Other fixes

- Changing a trade after confirmation requires both players to approve the new offer. Existing key-based accounts
  can trade, rental offers check ownership, and collections refresh after another session changes them.
- Refusing deletion of an account's last player preserves that player's friends, mail and rentals.
- An unrelated player cannot enter or cancel another pair's duel. Create Duel keeps the host's chosen banlist.
- Exporting another player's invitation does not switch the account used on Play. Start uses saved settings.
- Game exit, hosting after closing/reopening the manager, Unicode game/DLL paths and cold headless startup behave
  correctly. Missing or incompatible connection components stop startup before account login.

## v0.0.3 (2026-11-09)

Fixes a locked main screen and a crash. Download the bundle for your system below, unpack it, copy the `data` folder
from your old one into it (your accounts, cards and decks), and run it as before.

- Fixed: for a player with a friend (or someone ignored), the main screen stayed locked after login: only the chat
  answered clicks. The game was waiting for an old Yu-Gi-Oh! Online 2 step the server didn't answer.
- Fixed: clicking the eagle icon at the top left crashed the game. It now shows your Duelist premium.
- The "Duels are unavailable" message now says what to do with a release bundle (unpack it again) instead of asking
  for Node.js.

## v0.0.2 (2026-11-09)

BOOSTERPASS codes and BP from the host. Download the bundle for your system below, unpack it, copy the `data` folder
from your old one into it (your accounts, cards and decks), and run it as before.

- BOOSTERPASS codes: the host makes codes (`add-code`) that players type in the game's BOOSTERPASS window (Main Menu →
  BOOSTERPASS) for BP, each account once; the game shows the points and "Acquired Duelist premium!".
- `give-bp` gives BP to one player's account. Both work while the server runs, from the server's own PC.
- The admin commands that need the server stopped now say so instead of having their change overwritten.

## v0.0.1 (2026-11-09)

The first pre-alpha. A private server for Yu-Gi-Oh! ONLINE 3 (ver. 1.153): log in with the original game client and
duel the computer or your friends. Download the bundle for your system below, unpack it and run `run.cmd` (Windows) or
`./run.sh` (Linux); nothing else to install. Then follow the [README](https://github.com/usersina/yo3-revival#readme).

### What works

- Duels against the computer opponents at Kaiba Corporation and between players (Auto Duel, Create Duel, Single and
  Match Mode with side decks, spectating), on EDOPro's rules engine, in the game's own dialogs and animations. Duel
  logs and replays.
- Edit Deck, the starter deck and nine official decks for Yugi, Jaden and Yusei, banlists and a server-side deck check.
- The shop (booster packs, structure decks, protectors, avatar items) paid in BOOSTERPASS Points earned in duels, card
  trades and rentals.
- Chat rooms and whispers, player search, friends and ignore lists, D-Mail, teams, user events (knockout tournaments).
- A join page (`http://HOST:8080/join`) where friends make their own account in a browser.

### Fixed since the first test builds

- The rules engine ships inside the bundle with its own Node.js: no Node.js, Git or curl to install. A duel whose
  engine can't run ends as a draw instead of stopping at the first Draw Phase.
- The game client no longer crashes after a search (Sangan, and any card that adds from the Deck): the server now
  follows the engine's hand shuffles.
- Creature Swap, Magical Hats, Exchange of the Spirit, stolen tokens and Xyz materials show where the engine has them.
- Hovering a monster shows markers on its equip cards and on the cards that target it.
