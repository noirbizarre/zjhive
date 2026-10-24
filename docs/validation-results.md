# Extraction validation results

Recorded 2026-10-24 on Linux x86-64, Rust 1.99.0, Zellij 0.45.1.
These are local executed checks, not claims that remote CI or release publishing ran.

## Main-branch rebase and HTTP route backport

The extraction was subsequently rebased onto Hive `93ccd98`, including the
breaking HTTP route migration `9cf7f30` (#39) and TUI empty-state improvements (#38).
Modify/delete conflicts retained removal of Hive's plugin and bridge; the CLI
documentation conflict retained `/ui` while removing the helper instructions.
ZJHive's catalog calls, external contract checks and host harness now use `/v1`.

Revalidation after that backport:

- Hive: `cargo build --locked --workspace` and `mise run ci` passed; **217 tests
  passed**, six optional tests skipped. Upstream empty-state cases are included.
- ZJHive: `mise run ci` passed (20 tests, one optional contract test skipped),
  `mise run docs:check` passed, and both WASM plugin and native companion rebuilt.
- The command below passed its external-Hive contract and real Zellij 0.45.1 smoke
  test using the newly built, rebased daemon:

  ```sh
  HIVE_TEST_BIN=/home/noirbizarre/Workspaces/tools/hive/refactor-extract-zjhive/target/debug/hive ZJHIVE_LIVE_SMOKE=1 cargo test --locked -p zjhive-bridge bridge_attaches -- --ignored --nocapture
  ```

- `mise run package` rebuilt the updated matching release artifacts.
- Both working trees passed `git diff --check`; no unresolved index conflicts remain.

The results below retain the original extraction baseline; the section above is
the current post-rebase result.

## ZJHive

| Command | Result |
| --- | --- |
| `cargo metadata --locked --format-version 1` | Passed; only local packages are `zjhive`, `zjhive-bridge`, `transport-probe`; no Hive source/path dependencies |
| `cargo build --locked --workspace` | Passed, native build |
| `cargo test --locked --workspace` | Passed: 20 tests, one optional external-Hive test ignored |
| `cargo +1.98.1 check --locked --all-targets` | Passed on installed MSRV-family toolchain |
| `cargo build --locked -p zjhive --target wasm32-wasip1 --release` | Passed; executable module loaded in real host |
| `cargo build --locked -p transport-probe --target wasm32-wasip1 --release` | Passed; diagnostic loaded in real host |
| `mise run ci` | Passed: rustfmt, Clippy with denied warnings, actionlint, Markdown lint, spelling, nextest (20 passed, one skipped) |
| `mise run docs:check` | Passed for integration documentation |
| `prek run --all-files --show-diff-on-failure` | Passed for tracked scaffold files; new untracked Rust files separately covered by Cargo checks |
| `prek run --files … --show-diff-on-failure` (all new crate, workflow, fixture, docs, layout and tool files) | Passed, including Rust format/Clippy hooks |
| `gh ship validate` | Passed release configuration/workflow contract |
| `mise run package` | Passed: matching WASM, Linux x86-64 companion archive and checksums |
| `tar -tzf zjhive-0.1.0-x86_64-unknown-linux-gnu.tar.gz` (in `dist/`) | Passed; companion, plugin, KDL, README, LICENSE present |
| `sha256sum --check zjhive-0.1.0-x86_64-unknown-linux-gnu.sha256` (in `dist/`) | Both checksums passed |
| `python tools/transport_probe.py` | Passed in actual Zellij; socket Unsupported; SSE buffered until EOF; timers responsive |

The independently supplied executable contract/live-host command was run both
before and after Hive removal; the final run used an isolated installed Hive:

```sh
HIVE_TEST_BIN=/tmp/opencode/hive37-install/bin/hive ZJHIVE_LIVE_SMOKE=1 cargo test --locked -p zjhive-bridge bridge_attaches -- --ignored --nocapture
```

Passed: one contract test plus its actual Zellij smoke subprocess. The contract
test covers observe/takeover/fencing, prompt, permission/form/URL, stale answers,
completion and detach. The host subprocess covers catalog, observe, takeover,
prompt, stale state, companion restart, observer recovery and detach/session survival.
The documented `zellij plugin --configuration … -- file:…` load command also passed
inside that real host, independently of loading the plugin through its startup layout.
The independent fixture separately forces an uncertain prompt acknowledgement,
rejects a distinct prompt, verifies exact same-key/content retry, and requires
permission before acknowledgement while polling/replies remain responsive.

## Hive after removal

| Command | Result |
| --- | --- |
| `cargo metadata --locked --format-version 1 --no-deps` | Six member crates; no Zellij crate |
| `cargo build --locked --workspace` | Passed |
| `mise run ci` | Passed: 212 tests, six optional real-agent tests skipped; formatting, Clippy, spelling, workflows, Markdown and docs build passed |
| `mise run lint:md` and `mise run docs:build` after final doc cleanup | Passed |
| `cargo install --debug --locked --path crates/hive-cli --root /tmp/opencode/hive37-install` | Passed; installed without plugin/helper |
| `/tmp/opencode/hive37-install/bin/hive --version` | `hive 0.1.0` |
| `target/debug/hive --help` and `target/debug/hive completions bash` | No Zellij-specific command |
| External ZJHive contract/live-host command above | Passed using the installed post-removal Hive daemon |
| `git diff --check` | Passed |

## Failures resolved during implementation

- Initial `cdylib` probe compiled but could not load: the host lacked `_start`.
  Diagnostic and actual plugins now build as WASI executables and load successfully.
- Probe launch/layout syntax and release-notes overlay initially prevented host
  execution; the repeatable harness now uses a valid isolated layout/config.
- External daemon test initially collided with the user's instance lock; it now
  isolates both XDG runtime and state directories.
- Initial optional smoke expected echoed user text from a fixture that emits only
  state events. It now checks accepted prompt and real state transitions.
- New protocol negotiation required updating the expired-handle regression case;
  it now also verifies rejection of an incompatible companion.
- Long release builds exceeded early command timeouts; resumed packaging completed.

## Not run / limitations

- GitHub-hosted OS matrix and release uploads were not executed. Packaging is
  locally verified for Linux x86-64 only. No versions, tags or releases were created.
- MSRV 1.98.1 was checked; exact 1.98.0 was not installed/tested.
- No direct WebSocket handshake was possible after the supported WASI socket
  creation path failed. No native-free CLI-pipe network implementation was built.
- Permission/form/URL UI, Ctrl-H, full daemon restart, plugin reload during active
  generation and every listed manual scenario were not all exercised in the real
  Zellij host; retained native/daemon tests cover their respective contracts.
- Native validators and plugin state tests are not a substitute for real-host UI
  coverage. Existing UI limitations are listed in [architecture](architecture.md).
