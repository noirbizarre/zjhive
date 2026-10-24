# Testing

See [recorded extraction results](validation-results.md) for executed checks and
the distinction between protocol fixtures, live host evidence and unrun scenarios.

## Independent checks

No Hive checkout, installed Hive binary or daemon is needed for these:

```sh
cargo metadata --locked --format-version 1
cargo build --locked --workspace
cargo test --locked --workspace
cargo build --locked -p zjhive --target wasm32-wasip1 --release
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
mise run ci
mise run docs:check
gh ship validate
```

Native tests cover plugin epochs/stale results, replay gaps, control and retry UI,
interaction validation and a standalone published-SDK ACP fixture that delivers a
permission request before prompt acknowledgement. That fixture verifies polling
and replies remain possible while the acknowledgement is pending. It is not a
second session manager and does not assert daemon behavior.

One contract test is deliberately ignored by the ordinary suite: it launches an
independently supplied Hive executable with temporary configuration/state/runtime
directories and a deterministic ACP agent. It uses public interfaces only.

```sh
HIVE_TEST_BIN=/absolute/path/to/hive cargo test -p zjhive-bridge bridge_attaches -- --ignored --nocapture
```

It verifies catalog identity, observe/takeover, stale-pilot fencing, prompts,
permission/form/URL answers, stale interaction rejection, completion, conditional
same-key retry, and detach preserving the session. The daemon is terminated on
test cleanup; the user's running daemon is never used.

## Real Zellij smoke

Build the actual plugin, diagnostic and debug companion. Run the capability probe
as described in [transport validation](transport-validation.md). Then run:

```sh
cargo build --locked -p zjhive-bridge
cargo build --locked -p zjhive --target wasm32-wasip1 --release
HIVE_TEST_BIN=/absolute/path/to/hive ZJHIVE_LIVE_SMOKE=1 cargo test -p zjhive-bridge bridge_attaches -- --ignored --nocapture
```

`tools/live_smoke.py` opens the plugin in an isolated Zellij session with locked
key mode, grants permissions, selects the fixture session, takes control, submits
a prompt, restarts the companion, checks observer recovery and detaches. It checks
rendered host output and the real daemon catalog, and cleans up its own processes.
Python 3.11+, Unix PTYs, Zellij 0.45.1 and a free loopback port are required.

For additional human UI coverage, use `python tools/live_smoke.py --url URL`
against a disposable daemon with an existing session. Exercise a permission, each
supported form field, URL consent followed by completion, Ctrl-C cancellation,
Ctrl-H relinquish, plugin reload during work, a gap larger than retained replay,
daemon restart, and ambiguous acknowledgement retry. Never infer success from
unexecuted checklist items; automated protocol coverage is distinct from host UI
coverage. The scripted host smoke currently covers the narrower sequence above.

## Packaging

```sh
mise run package
tar -tzf dist/zjhive-0.1.0-x86_64-unknown-linux-gnu.tar.gz
```

Use the current workspace version in artifact names. CI builds the WASM plus a
Linux x86-64 companion and packages matching artifacts. gh-ship still owns release
preparation, versioning and tags; publishing attaches the built assets to its release.
Local packaging does not test the repository's release credentials or execute uploads.
