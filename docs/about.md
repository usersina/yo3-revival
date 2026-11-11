# About YO3 Revival

YO3 Revival is a private server and connection app for the retired **Yu-Gi-Oh! ONLINE 3 (Duel Accelerator)**.
Play on your own computer or join a friend's server. There is no project-operated public server.

The server is written in Rust and reverse-engineered from the original client. Duels use EDOPro's core through
ocgcore-wasm. The [playing guide](playing.md#features-and-limitations) describes what works;
the [roadmap](roadmap.md) records the remaining work. This is a pre-alpha project, so expect bugs.

## Game ownership and credits

This is a non-commercial fan preservation project, **not affiliated with or endorsed by Konami**.
Yu-Gi-Oh!, Yu-Gi-Oh! ONLINE 3, the game client, cards and artwork belong to their respective owners.
Bring your own supported installation: Revival does not distribute the original game.

The repository contains this project's code, documentation and screenshots. It does not include game archives,
Konami data files, decompiled code or packet logs. Screenshots under `screenshots/` show the original game and our
own Desktop app; they do not provide the underlying game assets. The server reads game data from your installation.

Release bundles also carry other people's open-source work, including Node.js, ocgcore-wasm and ProjectIgnis'
card scripts and database. Their notices are included in `THIRD-PARTY.md`, `licenses/` and `engine/THIRD-PARTY.md`
inside each bundle.

## License

[MIT](../LICENSE) covers this project's own code and documentation. It does not cover Konami's game or its content,
or replace the licenses of bundled third-party components.

## Help and support

Read [troubleshooting](launcher.md#troubleshooting), then [open an issue](https://github.com/usersina/yo3-revival/issues/new/choose)
with the steps and a diagnostic report if needed. [Contributors start here](development.md).

To support the work: [Buy me a coffee](https://buymeacoffee.com/usersina), or BTC
`bc1qawq699ewyq0vgyux6nmyyx29r4vhj573l77elj`.
