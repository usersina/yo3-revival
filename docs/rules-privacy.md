# Rules and privacy

What we ask of players on a YO3 Revival server, and what the server keeps about you. This page describes the server
as this project ships it. A host can change some settings, and the host's own announcements come first.

YO3 Revival is a non-commercial fan project. It is **not affiliated with or endorsed by Konami**. Yu-Gi-Oh! ONLINE 3
and its content belong to their owners.

**Contents**

- [Rules](#rules)
- [House rules](#house-rules)
- [What the server keeps](#what-the-server-keeps)
- [Who can see what](#who-can-see-what)
- [Deleting your account](#deleting-your-account)

## Rules

1. Be decent in chat, D-Mail, team names and player names: no harassment, hate, threats or sexual content, and no
   spam.
2. Play fair. Don't use modified clients, bots or bugs to win, to get BP or cards, or to spoil other players' duels.
   If you find a bug that gives you something, report it instead.
3. One person, one account. Don't share your account, sell it, or sign in as someone else.
4. Don't attack or overload the server, and don't try to get into other players' accounts.
5. Don't post other people's personal details.

Moderators can mute you (chat and D-Mail), restrict you, ban you, or sign you out everywhere. Each of these can be
for a set time or until it is lifted. Every moderator action is written to the server's audit log with the
moderator's name and the reason.

## House rules

Duels follow the official rules of the card game, with these changes for every duel on YO3 Revival servers:

- **"Ignoring its Summoning conditions" means all of them.** A card that Special Summons a monster "ignoring its
  Summoning conditions" (Illusion Gate, Arduous Decision, the anime Underworld Circle...) also ignores "must first be
  properly Summoned": Illusion Gate can take a Divine Serpent Geh that was only ever sent to the Graveyard, or a Synchro
  Monster that was never Synchro Summoned. The official ruling keeps that limit; the anime never showed it. Monster
  Reborn, Call of the Haunted and every other revival that ignores nothing keep every limit, and a monster that "cannot
  be Special Summoned by other ways" is still out of their reach (rules engine: `crates/yo3-engine/src/house_rules.lua`).

## What the server keeps

Times are what the server enforces today. **Open** means nothing in the server deletes it yet: it stays until the
host removes it.

| What | Where | How long |
| --- | --- | --- |
| Your account: User ID, password hash (argon2id; nobody can read your password), a digest of your recovery code, role, standing and mute with their reasons | The server's database | Until the account is deleted ([deleting your account](#deleting-your-account)) |
| Your players: names, level, cards, decks, BP, items and titles, wins and losses, friends, ignore list, team | The server's database | As long as the account exists. Deleting a player in the game takes it off your account. |
| Country | The server's database | Set from your game's language, or by a moderator. It is not looked up from your IP address. |
| Signed-in devices ("Stay signed in") | A digest of each device's token, labelled only "Windows PC" or "Linux PC" | 90 days after the device was last used, at most 10 devices. A password change, a recovery or "sign out everywhere" ends them all. |
| Game sessions | Server memory | 12 hours at most. A restart forgets them. |
| **Linked accounts** (**Since v0.1.5 (2026-11-20)**) | The server's database, with your account: a digest of your launcher's random install id, and the addresses you signed in or played from, each with when it was last seen | The install id's digest 90 days after it was last seen, an address 14 days, at most 16 of each. Used to tell that two accounts play from one PC or network (a duel between them pays nothing, [hosting](hosting.md#linked-accounts-and-fair-play)); staff see which accounts are linked, not the addresses. Deleted with the account. |
| **IP addresses** | The server's session logs: each connection and account request, with your account number | 14 days, or less once the logs pass 100 MB. Not in the database. The address limits in memory forget an address within minutes of its last request. |
| **IP addresses on a VPS** | The service's output in the machine's system journal (systemd) | **Open:** the machine's journal settings decide, not this server |
| **Chat** (rooms, whispers, team) | Not kept: delivered to whoever is meant to get it | A hosted server (`packet_log = false`, Host for friends) writes no chat anywhere. With packet logging on (development, Scripts edition), lines are in the packet files and session logs. |
| D-Mail | The server's database, until the recipient's mail window fetches it | Then only on the recipient's PC. At most 100 wait from players. |
| Duel logs (replays) | The server's database | The newest 1,000 on the server |
| **Duel recordings**: every duel's rules-engine traffic (decks, moves, the players' numbers and names), for replaying a reported problem | The server's `recordings/` folder | 14 days (the host's `recording-days`; 0 makes none). One you mark with the `bug` chat command is pinned and kept until the host deletes it. |
| **Player reports and `bug` notes**: who reported whom, the reason you typed, and what the staff did with it | The server's database | Until the staff delete them, or the reporter's or reported player's account is deleted |
| Admin audit log: who changed what, including moderation | The server's database | 365 days, then removed (the host's `audit-keep-days` setting; 0 keeps it for ever) |
| BP grant retry receipts: request ID, host or staff account number, input fingerprint, target account number, resulting BP and time | The server's database | No expiry, independent of audit pruning, so a retry cannot pay twice. No player names, passwords or recovery codes. |
| Recent server log lines (admin panel) | Server memory | The last 2,000 lines, until the server restarts |
| **Problem reports** (**Since v0.1.10 (2026-11-21)**): your launcher's log of a game whose connection broke or that closed with an error, or sent with Report a problem; the server's own log lines about your account from the 15 minutes before | The server's `reports/` folder, under your account number | 30 days, at most 2,000 reports or 256 MB in all, the oldest first; 20 a day per account. Passwords, keys and tokens are removed before sending and again on the server. Turn it off in the launcher's Setup or Settings → Advanced; the host can refuse them all (`problem-reports`). |
| **Report a problem** (**Since v0.1.22 (2026-11-24)**): what you write, how to reach you if you add it, the error shown, your launcher's recent logs (personal details removed), this PC's system, processor, memory and graphics card, and in Singleplayer the duels you marked with `bug` | A file on your own PC (your Downloads folder, [what's inside](report-format.md)). Nothing is sent: you decide whether and where to send it | Until you delete it |
| Backups of the database | The server's `backups/` folder | The newest 24, taken hourly and at each start and stop |
| Off-site backups (if the host sets them up) | The host's storage | restic: 14 daily and 8 weekly copies (about 2 months). rclone: 30 days by default. |
| Your PC: the launcher's settings, logs and device token | Your own PC. The token is in the system's credential store. | Launcher logs: 14 days or 100 MB. They leave your PC only as problem reports, to the server you play on (above). |

## Who can see what

- **Other players** see your player name, level, avatar, title, country, team, record, online status and what you
  say in chat. A whisper or a D-Mail goes only to its recipient (or your team). Other players never see your User
  ID, your password or your IP address.
- **Players in a duel with you** see the duel as the game shows it. Anyone signed in who knows a duel log's name can
  download that replay.
- **Moderators** see, in the admin panel: your User ID, your players, BP and cards, your standing and mutes, how many
  devices are signed in, the audit log, and the recent server log lines (which hold IP addresses). They can't see your
  password or your recovery code.
- **Moderators and the host** can read problem reports (the dashboard's Problem reports page, `problem-reports`).
- **The host** runs the machine and can read everything the server keeps, including the logs and backups.

## Deleting your account

Ask the host or a moderator through the server's support channel, from the account's User ID (or show that you own
it). An admin deletes it with `delete-account` or the admin panel's **Delete account**.

What goes, at once:

- your account: User ID, password hash, recovery code digest, role, standing, mute, every signed-in device and what it was seen on (the linked-accounts record);
- your players, with their cards, decks, BP, items, titles and wins and losses;
- your friends and ignore lists, and your place in other players' lists;
- invitations from or to you (friend, trade, rental, team), your rentals, and a team you lead (it is disbanded) or
  your place in one;
- D-Mail you sent that hasn't been read yet, and D-Mail waiting for you;
- user events you host, and your entry in those not started yet;
- your own copy of each duel log;
- player reports you made or that were made about you;
- the admin audit log's rows about your account.

What stays:

- one audit row: which admin deleted which User ID, and when;
- other players' own records of their duels against you, and their copy of a duel log;
- duel recordings with your player in them, until they age out (14 days) or, for a pinned one, until the host deletes it;
- the database **backups**, until they are replaced: on the server the newest 24 (taken hourly, so about a day), and
  off-site copies as long as the host keeps them (see [what the server keeps](#what-the-server-keeps));
- the server's session logs (IP addresses with your account number), until they age out after 14 days.

Your game is disconnected. You can delete one of your players yourself in the game.
