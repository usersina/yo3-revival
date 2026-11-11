# Documentation

New here? The [main README](../README.md) gets you into your first game.

## Play and host

| I want to…                                       | Read                                                                                                                                     |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Set up the Desktop app, join a friend or host    | [Desktop guide](launcher.md)                                                                                                             |
| Learn the game controls and start duelling       | [Playing guide](playing.md)                                                                                                              |
| Use terminal scripts instead of the app          | [Scripts edition](scripts.md)                                                                                                            |
| Find a server command or change network settings | [Command-line reference](guide.md)                                                                                                       |
| Fix a setup problem or report a bug              | [Troubleshooting](launcher.md#troubleshooting)                                                                                           |
| Update or back up an existing setup              | [Desktop updates](launcher.md#application-updates), [backups](launcher.md#your-data-and-upgrades), [Scripts updates](scripts.md#updates) |
| Understand the project, credits or license       | [About YO3 Revival](about.md)                                                                                                            |

## Build and contribute

- [Development guide](development.md): source builds, checks, duel diagnostics and release packaging.
- [Working guide](https://github.com/usersina/yo3-revival/blob/main/AGENTS.md): repository rules, tools and live-test safety.
- [Roadmap](roadmap.md): implementation status, outstanding work and recorded investigations.
- [Authentication](authentication.md): account keys, TLS, the required connection DLL and session binding.
- [Protocol](protocol.md): wire format and request/reply shapes, with client evidence.
- [Client internals](client-internals.md): executable addresses, native hooks, data formats and crash history.
- [Research](research.md): the original feasibility study, retained as history.
- [Screenshots](../screenshots/): the real Desktop app and game, used throughout these guides.

The client binary is the protocol spec. Technical claims carry client evidence and a status:
**confirmed**, **shape**, **guess** or **open**. Historical notes describe the project at the time;
use the Desktop or Scripts guide for current setup.

## Check documentation

From a source checkout:

```bash
tools/docs-check.py                         # repository links and anchors
tools/docs-check.py --toc docs/launcher.md  # generate a table of contents
tools/docs-check.py --root /path/to/bundle  # check an extracted release
```

Release packaging checks its copied documentation too, so screenshots and links must work both here and offline
inside the extracted bundle.
