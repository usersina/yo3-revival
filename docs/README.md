# Documentation

Everything about the project that is too detailed for the [main README](../README.md). The documents are for different
readers; this page says which is which and in what order to read them.

**Contents**

- [Documentation](#documentation)
  - [The documents](#the-documents)
  - [Where to start](#where-to-start)
  - [Conventions](#conventions)
  - [Checking the docs](#checking-the-docs)

## The documents

| Document                                   | For                        | What is in it                                                                                                                                                       |
| ------------------------------------------ | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [guide.md](guide.md)                       | Hosts and players          | TLS server setup, private launcher profiles, accounts, playing and troubleshooting. A copy ships in each release bundle as its `README.md`                          |
| [authentication.md](authentication.md)     | Hosts and maintainers      | The launcher/DLL, TLS tickets, session binding, account migration and security checks                                                                               |
| [roadmap.md](roadmap.md)                   | Maintainers                | Status, the milestones M0 to M6 with their checkboxes, the dated work sessions (reviews, monkey testing), every guess we made, open questions                       |
| [protocol.md](protocol.md)                 | Maintainers                | The protocol catalog: wire format, the gate, front and duel servers, every request/reply pair with its client evidence and a status (confirmed, shape, guess, open) |
| [client-internals.md](client-internals.md) | Reverse engineers          | Addresses and functions in `yo3.exe`, the field tracer, data file formats, behaviours worth knowing, crash history                                                  |
| [research.md](research.md)                 | Curious readers            | The feasibility study from before any server code existed, kept as history                                                                                          |
| [../AGENTS.md](../AGENTS.md)               | Contributors and AI agents | The working guide: ground rules, repo map, commands, the loop for answering a new client request, conventions, environment gotchas                                  |
| [images/](images/)                         | The READMEs                | Screenshots of the real client running on this server                                                                                                               |

## Where to start

- **I want to play or host:** start with the [project README](../README.md), then [guide.md](guide.md) for commands.
- **I want to contribute:** [../AGENTS.md](../AGENTS.md), then [roadmap.md](roadmap.md) to pick something open, then
  [protocol.md](protocol.md) for the area you will touch.
- **I want to know how a screen works:** [protocol.md](protocol.md) for the messages, then
  [client-internals.md](client-internals.md) for the client side of it.
- **I want the history:** [research.md](research.md), then [roadmap.md](roadmap.md) from the top.

## Conventions

- The client binary is the spec. Protocol claims cite a client address, a `File.cpp Lnnn` trace line or a logged
  packet, and carry a status: **confirmed**, **shape**, **guess** or **open**.
- Four places stay in step: `protocol.md`, `crates/yo3-server/src/front.rs`, the contract tests under
  `crates/yo3-server/tests/` and `roadmap.md`.
- The docs cite addresses and field names but never paste decompiled code, Konami data or packet logs.
- Screenshots in `images/` are 1024x768 frames of the client, with test player names only.
- Long technical documents start with a table of contents; the short project README and changelog do not need one.
  `tools/docs-check.py --toc FILE` prints one for a file.

## Checking the docs

```bash
tools/docs-check.py                    # every relative link and #anchor in root Markdown files and docs/**/*.md
tools/docs-check.py --toc docs/FILE.md # a table of contents to paste under a document's title
```
