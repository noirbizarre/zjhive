<p align="center">
  <img src="docs/images/icon.svg" alt="zjhive" width="160">
</p>

<h1 align="center">zjhive</h1>

<p align="center"><strong>Zellij plugin for Hive ACP sessions</strong></p>

<p align="center">
  <a href="https://github.com/noirbizarre/zjhive/actions/workflows/ci.yaml">
    <img src="https://github.com/noirbizarre/zjhive/actions/workflows/ci.yaml/badge.svg" alt="CI">
  </a>
  <a href="https://codecov.io/gh/noirbizarre/zjhive">
    <img src="https://codecov.io/gh/noirbizarre/zjhive/graph/badge.svg" alt="Codecov">
  </a>
  <img src="https://img.shields.io/github/v/release/noirbizarre/zjhive" alt="Release">
  <img src="https://img.shields.io/github/license/noirbizarre/zjhive" alt="License">
</p>

---

ZJHive lets you observe and control existing [Hive](https://github.com/noirbizarre/hive)
ACP sessions inside Zellij. It provides a live session catalogue, explicit takeover
and relinquish, replay/gap detection, prompts with safe same-key retries,
cancellation, permission choices, forms and URL consent/completion.

Hive owns sessions and agent processes. ZJHive owns both the WASI UI and its small
native `zjhive-bridge` companion. The companion adapts finite Zellij WebAccess
requests to a persistent ACPv2 WebSocket connection; it never manages sessions.

## Installation

Prerequisites: **Zellij 0.45.1**, a running compatible Hive daemon, and the matching
plugin/companion pair. The initially packaged native platform is Linux x86-64.
Other native platforms have template CI coverage but no packaged release assets.

For a published release, download the matching native archive and checksum file,
verify with `sha256sum --check` (also download the standalone WASM referenced by
that checksum file), and extract the archive. Install its `zjhive-bridge` on PATH
and its `zjhive.wasm` in your Zellij plugin directory. The archive also contains an
example layout whose absolute plugin path must be edited before loading.

From source (Rust 1.98+ and Python 3.11+):

```sh
rustup target add wasm32-wasip1
cargo build --locked -p zjhive --target wasm32-wasip1 --release
cargo build --locked -p zjhive-bridge --release
install -Dm755 target/release/zjhive-bridge ~/.local/bin/zjhive-bridge
install -Dm644 target/wasm32-wasip1/release/zjhive.wasm ~/.config/zellij/plugins/zjhive.wasm
```

Inside a Zellij session:

```sh
zellij plugin --configuration 'url=http://127.0.0.1:8765,bridge=http://127.0.0.1:8766' -- "file:$HOME/.config/zellij/plugins/zjhive.wasm"
```

Grant **WebAccess** and **RunCommands**. Ensure `zjhive-bridge` is on the Zellij
server's PATH (restart Zellij after changing PATH). The plugin starts the companion
when its health endpoint is unavailable. Alternatively run it explicitly:

```sh
zjhive-bridge --listen 127.0.0.1:8766 --url http://127.0.0.1:8765
```

The `url` and `bridge` settings are preserved from the former in-tree plugin.
The executable and WASM artifact have changed: install both together, replacing
`hive zellij-bridge` and `hive_zellij.wasm`. There is no crates.io install path.
Release workflows attach matching assets and checksums; no release has been
published or tested as part of the extraction.

## Controls

Use arrows and Enter to observe a session. Ctrl-T takes control, Ctrl-H relinquishes,
Enter submits a prompt, Ctrl-C cancels, Ctrl-R retries the same uncertain prompt
(or explicitly accepts live-only history after a replay gap), and Esc detaches.
Forms support Tab, arrows, typed values and `|`-separated multiselect values;
permission requests use the displayed option numbers. Enter accepts and Esc declines
an interaction. PgUp/PgDn inspect long interaction details.

Zellij may reserve Ctrl-T/Ctrl-H; switch to its locked mode to forward these keys.
Detaching, reloading the plugin or stopping the companion does not close Hive sessions.

## Development

```sh
cargo test --locked --workspace
mise run ci
mise run docs:check
mise run package
mise run probe:host
```

`mise run package` produces the plugin, a host-native archive and checksums in
`dist/`. See [architecture and compatibility](docs/architecture.md),
[transport evidence](docs/transport-validation.md), and
[the smoke-test procedure](docs/testing.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
