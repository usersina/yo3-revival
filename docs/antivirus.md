# Antivirus warnings

Windows or your antivirus may warn about YO3 Revival, block it, or move one of its files to quarantine. This page says
why, how to check that your download is really ours, and how to get a file back.

- [Antivirus warnings](#antivirus-warnings)
  - [Why YO3 Revival gets flagged](#why-yo3-revival-gets-flagged)
  - [Which files and folders](#which-files-and-folders)
  - [Check your download](#check-your-download)
  - [Restore a quarantined file](#restore-a-quarantined-file)
  - [Allow a file you checked](#allow-a-file-you-checked)
  - [Report a false positive](#report-a-false-positive)

## Why YO3 Revival gets flagged

To connect the old game to a new server, YO3 Revival starts the game with a small helper that loads our connection DLL
into it. Cheats and malware do the same thing, so antivirus tools are suspicious of any program that does it. Our files
are also new and not signed with a code-signing certificate yet, so Windows doesn't know them: SmartScreen may say
"Windows protected your PC", and an antivirus may flag a file it has never seen before.

A warning is not proof that something is wrong, and not proof that it's fine either. Check the download first.

### What you'll see on Windows 11

Checked with v0.1.3 on Windows 11 with Microsoft Defender (signatures 1.463.368.0), downloaded through Edge from the
release page (2026-11-19):

1. **Downloading** the zip: no warning in Edge, and Defender didn't flag it.
2. **Extract All**: about 15,000 small files (mostly the rules engine's card scripts), which Defender checks one by one,
   so this took about 18 minutes on our test PC. Let it finish: a half-extracted folder won't start.
3. **The first start** of `yo3-revival.exe`: SmartScreen says "Windows protected your PC" with only **Don't run**.
   Choose **More info**; it shows "App: yo3-revival.exe, Publisher: Unknown publisher" and adds **Run anyway**. This
   is because the files aren't signed yet ([why](#why-yo3-revival-gets-flagged)); check the download first.
4. **PLAY**: no further warning. The helper, your own server and the rules engine start without SmartScreen,
   Defender or firewall prompts, and the game opens.

Other antivirus software may behave differently, and SmartScreen can change as it learns about a release.

## Which files and folders

| Where                                                       | YO3 Revival's files                                                                                     |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| The folder you extracted (`yo3-revival-desktop\` or `yo3-revival-scripts\`) | `yo3-auth-launch.exe` (the helper that starts the game) and `yo3-auth.dll` (the connection DLL): both are required |
| Same folder                                                 | `yo3-revival.exe` (the app), `yo3-launcher.exe`, `yo3-server.exe`, `yo3-engine.exe` and the `engine\` folder |
| Same folder, Desktop only                                   | `yo3-bootstrap.dll` and `yo3-launcher-windows.exe`, for opening the game directly                        |
| After an in-app update                                      | the same files in `%LOCALAPPDATA%\YO3 Revival\updates\installs\<version>-<platform>\`, a new folder for each version |
| The game's folder, beside `yo3.exe`                         | `winmm.dll`, put there only when [opening the game directly](launcher.md#open-the-game-directly) is on (off for a new player) |

The helper and the DLL are the files most often flagged. PLAY loads the DLL from YO3 Revival's folder; it is not
copied into the game's folder. The game's own folder doesn't need an exception.

## Check your download

Download YO3 Revival only from the [official releases](https://github.com/usersina/yo3-revival/releases/latest). Each
release lists a SHA-256 checksum for every archive, in its notes and in a `SHA256SUMS` file beside the downloads. A
file with the same checksum is exactly the file we published.

On Windows, open PowerShell in the folder you downloaded to (in File Explorer, Shift + right-click the folder, then
**Open PowerShell window here**), and run:

```powershell
Get-FileHash .\yo3-revival-desktop-vX.Y.Z-windows-x86_64.zip
```

Use the name of the file you downloaded. The `Hash` it prints must be the same as the archive's line in the release
notes or `SHA256SUMS` (capital letters don't matter). If it differs, delete the download and don't run anything from
it.

On Linux, in the download folder, after saving `SHA256SUMS` there too:

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

Each archive you have should say `OK`.

## Restore a quarantined file

If PLAY says **YO3 Revival needs repair**, a file may be missing because your antivirus removed it. In Windows Security:

1. Open **Windows Security**, then **Virus & threat protection**, then **Protection history**.
2. Find the item that names a YO3 Revival file from [the table](#which-files-and-folders). Note the file and the
   detection name.
3. If your archive's checksum matched, choose **Actions**, then **Restore** (or **Allow on device**).
4. In YO3 Revival, choose **Retry**, then **PLAY**.

Simpler still: once you have [checked the archive](#check-your-download), extract it again over YO3 Revival's folder.
Your settings and Singleplayer data are kept elsewhere and stay as they are. Don't mix files from different releases.

Other antivirus programs have the same steps under a name such as Quarantine, Virus Chest or History.

## Allow a file you checked

Only allow files from an archive whose checksum you [checked](#check-your-download). Keep your antivirus on.

1. **Windows Security**, **Virus & threat protection**, then under **Virus & threat protection settings**, **Manage
   settings**.
2. Under **Exclusions**, **Add or remove exclusions**, then **Add an exclusion**.
3. Choose **File** and pick the flagged file, for example `yo3-auth.dll`. If several files are flagged, you can choose
   **Folder** and pick YO3 Revival's extracted folder instead.

Never exclude your whole Downloads, Games or Program Files folder. After an in-app update YO3 Revival runs from a new
folder (see [the table](#which-files-and-folders)), so an exception for the old folder no longer covers it.
[Microsoft's guide to Windows Security](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/virus-and-threat-protection-in-the-windows-security-app)
has pictures of each step.

If SmartScreen shows "Windows protected your PC" when you open a checked download, choose **More info**, then **Run
anyway**.

## Report a false positive

Reporting a wrong detection helps every player: once the antivirus company looks at the file, it usually stops
flagging it for everyone.

- **Microsoft Defender:** submit the file at
  [Microsoft's file submission page](https://www.microsoft.com/en-us/wdsi/filesubmission) as a home customer, and say
  you believe it is incorrectly detected. Name the detection shown in Protection history.
- **Another antivirus:** look for "false positive" on its website; most have a form.
- **Tell us too:** [open an issue](https://github.com/usersina/yo3-revival/issues/new/choose), or use **Report a
  problem** in the app, with the antivirus's name, the detection name, the file, YO3 Revival's version and the file's
  checksum (`Get-FileHash` on that file).

YO3 Revival is open source: anyone can read the code, or [build it themselves](development.md#build-from-source).
