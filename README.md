<p align="center">
  <img src="docs/images/icon.svg" alt="zjhive" width="160">
</p>

<h1 align="center">zjhive</h1>

<p align="center"><strong>Read-only Zellij catalogue and status dashboard for Hive</strong></p>

Browse [Hive](https://github.com/noirbizarre/hive) sessions, inspect their metadata,
and monitor daemon status inside Zellij. ZJHive is a single WASM plugin: no native
companion, command execution, or additional runtime tools are needed.

Hive owns sessions and agents. The dashboard polls its finite operational HTTP
endpoints through Zellij WebAccess. Details show catalogue metadata, not conversation
history; use Hive's TUI or WebUI for messages, prompts and control.

## Installation

Requirements: **Zellij 0.45.1** and a running Hive daemon exposing
`/v1/sessions` and `/v1/status` (see [compatibility](docs/architecture.md)).

Download `zjhive-<version>.wasm` and `zjhive-<version>.sha256` from the release
assets. In their download directory, verify the checksum before installing:

```sh
sha256sum --check zjhive-<version>.sha256
install -Dm644 zjhive-<version>.wasm ~/.config/zellij/plugins/zjhive.wasm
```

Replace `<version>` with the downloaded release version. The same artifact is
used on every supported Zellij host platform; there is no crates.io install path.

From source (Rust 1.98+):

```sh
rustup target add wasm32-wasip1
cargo build --locked -p zjhive --target wasm32-wasip1 --release
install -Dm644 target/wasm32-wasip1/release/zjhive.wasm ~/.config/zellij/plugins/zjhive.wasm
```

Inside a Zellij session:

```sh
zellij plugin --configuration 'url=http://127.0.0.1:8765' -- "file:$HOME/.config/zellij/plugins/zjhive.wasm"
```

Grant **WebAccess**. The `url` setting defaults to `http://127.0.0.1:8765`.
An example startup layout is in [layouts/zjhive.kdl](layouts/zjhive.kdl); edit its
plugin path before loading. An obsolete `bridge` setting is ignored with a migration
message. The old companion is no longer started or required.

## Controls and status

| Key | Action |
|---|---|
| Up / Down | Select a session, or scroll details |
| Page Up / Page Down | Move by a page |
| Home / End | First / last session or detail page |
| Enter | Inspect selected session metadata locally |
| Esc | Return to the catalogue |
| `r` | Refresh catalogue and daemon status |

If Zellij intercepts a key, switch to its locked mode to forward it to the plugin.

Details include lifecycle/recovery, activity, cwd, ownership, observers, native
session metadata, reported token/cost usage, and a WebUI URL. Selecting a session
does not attach an observer, take control, or change anything in Hive.

Catalogue refresh is approximately every four seconds; daemon status every twenty.
Failures retain cached data marked **STALE**, with independent retry backoff per
endpoint. Unknown usage is displayed as unknown. Overdue requests have bounded
retries; if the host leaves requests stuck, reload the plugin to recover.

## Development

```sh
mise run ci
mise run docs:check
mise run dashboard:check
mise run plugin:smoke
mise run package
```

`mise run package` writes only the versioned WASM and its checksum to `dist/`.
The real-host smoke needs Python 3.11+, Unix PTYs and Zellij; no Hive daemon is
required. The optional `mise run probe:host` checks host transport capabilities.
See [architecture](docs/architecture.md), [transport evidence](docs/transport-validation.md),
[testing](docs/testing.md), and [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
