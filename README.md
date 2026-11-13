# YO3 Revival

A private server and launcher for the retired **Yu-Gi-Oh! ONLINE 3 (Duel Accelerator)**. Play again against the
computer or with friends, using the original game client on your own server.

## In the game

Duel against Kaiba or challenge friends. Build and edit decks, open booster packs, trade cards and chat in the
original game world.

![Stardust Dragon on the field in the original game client, running on Revival](screenshots/duel.png)

| Duel against Kaiba                                             | Build your deck                                                                            |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| ![Meet Seto Kaiba at Kaiba Corporation](screenshots/kaiba.png) | ![Elemental Hero Shining Flare Wingman selected in the deck editor](screenshots/decks.png) |

[Features and limitations](docs/playing.md#features-and-limitations) · [Development roadmap](docs/roadmap.md)

## Start playing

You need the original game **v1.153**, installed separately. Revival does not include it.
Use **64-bit Windows 10/11**, or **Linux with Wine**. **Windows 7 is not supported.**
[Game installation help](docs/launcher.md#getting-the-original-game).

1. [Download the latest release](https://github.com/usersina/yo3-revival/releases/latest). Choose **Desktop** for
   your system and extract **the whole archive**.
2. Open **`yo3-revival.exe`** on Windows or **`yo3-revival`** on Linux inside `yo3-revival-desktop/`.
3. Stay in **Normal** mode. **Browse…** to your game folder—the one containing `yo3.exe`—and wait for the automatic check.
4. Click **Play**. In the game, choose **ONLINE**, then select your player. Your starting deck is ready automatically.
5. Visit **Kaiba Corporation** on the map for your first duel. [Playing guide](docs/playing.md).

**Antivirus flags Revival?** For a verified false positive, allow the affected file or exclude only the extracted
`yo3-revival-desktop/` folder (after an in-app update, the version's folder under
`%LOCALAPPDATA%\YO3 Revival\updates\installs\`). [What to check and which files to allow](docs/guide.md#windows-smartscreen-and-antivirus-warnings).

![Normal mode: game folder, automatic checks, Play and logs](screenshots/desktop.png)

[Join a friend](docs/launcher.md#join-another-server) ·
[Troubleshooting](docs/launcher.md#troubleshooting) ·
[All documentation](docs/README.md)

## Host or manage a server

- **Desktop:** Switch to **Advanced → Hosting** to start or stop your server, manage accounts and export private
  invitations for friends. [Hosting guide](docs/launcher.md#host-friends).
- **Scripts edition:** Follow the [terminal hosting setup](docs/scripts.md#host-friends) and
  [server and account commands](docs/guide.md).

Pre-alpha: **Help & support → Report a problem…** includes diagnostics. [Reporting guide](docs/launcher.md#logs-and-support) · [GitHub issues](https://github.com/usersina/yo3-revival/issues/new/choose).
[MIT](LICENSE) · [Project, credits and support](docs/about.md). Unaffiliated with Konami.

## For developers

The server is written in Rust, reverse-engineered from the game client, and runs duels on
[EDOPro](https://github.com/edo9300/edopro)'s rules engine. Everything technical (building from source, the protocol,
the client's internals, the roadmap) is in [docs/](docs/README.md). To contribute, read
[AGENTS.md](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md) first.

---

<sub>If you'd like to support the work: [Buy me a coffee](https://buymeacoffee.com/usersina) · BTC
`bc1qawq699ewyq0vgyux6nmyyx29r4vhj573l77elj`</sub>
