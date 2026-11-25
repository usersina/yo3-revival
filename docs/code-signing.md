# Code signing policy

**Status: a proposal, deferred.** SignPath Foundation declined the application, and Authenticode signing waits until it
can be funded (#48). Published releases are unsigned (their update files carry the project's own Ed25519 signatures);
no certificate exists and CI signing is not enabled. This page describes the policy for future signed releases.

If approved, the signing service will be credited as follows:

Free code signing provided by [SignPath.io](https://signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

## Scope

We request signing for the Windows programs and libraries built from this repository: `yo3-revival.exe`,
`yo3-launcher.exe`, `yo3-server.exe`, `yo3-engine.exe`, `yo3-auth-launch.exe`, `yo3-auth.dll` and `yo3-bootstrap.dll`.
The Linux Desktop archive also contains Windows helpers for Wine, including the renamed
`yo3-launcher-windows.exe`. The bootstrap DLL is installed beside the game as `winmm.dll`.

The connection DLL redirects the player's own game to their selected Revival server and supplies compatibility
hooks. The injector loads it with the player's permission. The proprietary game executable is not changed or
distributed. We do not request signatures for Konami's game, game archives or unrelated third-party binaries.

`yo3-engine` is our Rust program linking the open-source EDOPro core and Lua. Its signing scope needs SignPath's
explicit review under its upstream-code rules; it is not assumed to be approved. Bundled components keep their
own licenses and notices ([credits](about.md#game-ownership-and-credits)).

## Responsibility and release process

- Author, reviewer and sole signing approver: [usersina](https://github.com/usersina).
- Changes from other contributors must be reviewed before merging. Each signing request requires the approver's
  manual approval; approval is not delegated to the CI token.
- GitHub access uses two-factor authentication. SignPath access must also use MFA before signing is enabled.
- Signing will be restricted to artifacts built from the public repository by GitHub-hosted release jobs.
  SignPath must verify their origin. Its service keeps the certificate's private key; it is not stored in GitHub.
- Signed binaries must identify the product as **YO3 Revival**, with consistent product versions. Signing must
  precede bundle inventories, archive checksums and publishing, including for the Windows helpers in Linux bundles.
  Signed files must not be changed afterwards.
- The release notes must link to this policy and state which files are signed. Until the approved signing process
  is configured and verified, releases must continue to be described as unsigned.

A signature identifies a publisher and detects changes to a signed file. It does not guarantee that Windows or
antivirus software will accept a new build without warnings. See [Microsoft's SmartScreen explanation](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation)
and our [download and antivirus help](guide.md#windows-smartscreen-and-antivirus-warnings).

## Privacy

The launcher checks GitHub for new releases at startup. Versions configured with an official server list also fetch
that list from GitHub. These requests expose ordinary network metadata, such as your IP address, to GitHub; update
checks do not include your Revival account credentials. An update downloads when you choose it (or by itself with
**Download updates automatically** on), is used only when its signature checks out, and installs when you restart the app ([application updates](launcher.md#application-updates)).

Account and gameplay requests go to the Revival server you select. Its operator controls that server's account
data and logs, including gameplay and chat. Hosting on your own computer keeps that server data on your computer.

Diagnostic logs are retained locally. **Report a problem** saves one file on your PC (your Downloads folder) with your
words and your recent logs, passwords, keys and tokens removed; it sends nothing, and you decide where to send it
([what's inside](report-format.md)). When a game breaks, its helper sends that game's log to the server you played on
(off in Settings → Advanced). Issues are public. Logs can include player names, chat, decks, paths and network addresses.
The reports exclude credentials, databases, private profiles and original game files.
See [reporting and log retention](launcher.md#logs-and-support) and [GitHub's privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).

SignPath processes the maintainer account and submitted build artifacts for signing, not player diagnostics.
Its [privacy policy](https://signpath.io/privacy-policy) applies to that service.

## Removal and reporting concerns

Close the game before removing Revival. Turn off **Open the game without the launcher** in Settings → Game
([direct start](launcher.md#open-the-game-directly)) to remove its owned loader, then close the app and delete its
extracted folder. Keep or back up saved accounts and decks before removing application or server data
([data and backups](launcher.md#your-data-and-upgrades)).

For a suspected tampered download or signing misuse, contact [the maintainer](https://github.com/usersina).
Include the release URL, filename and SHA-256 hash; do not post private logs, account keys or credentials.
