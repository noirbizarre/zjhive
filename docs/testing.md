# Testing

## Independent checks

No Hive checkout, installed Hive binary or daemon is needed:

```sh
cargo test --locked --workspace
mise run ci
mise run docs:check
mise run dashboard:check
mise run plugin:build
```

Native dashboard tests cover response validation, selection identity, daemon revision
reset, cached stale data, independent endpoint freshness, overdue and delayed requests,
local details, safe rendering, and viewport navigation. Python contract checks enforce
the WebAccess/GET-only host integration and single-WASM/checksum packaging boundary.
The dedicated plugin workflow runs those checks and builds/packages the WASM.

## Real Zellij smoke

```sh
mise run plugin:smoke
```

Python 3.11+, Unix PTYs and Zellij 0.45.1 are required. The smoke loads the actual
release WASM in a disposable locked-mode Zellij session and starts a finite HTTP
fixture on an ephemeral loopback port. It grants WebAccess, inspects local details,
reorders records, simulates failure/recovery, shows an empty catalogue, and exercises
the documented CLI loading syntax. Recorded requests must be GETs to the two
operational endpoints. Cleanup kills only its own host session and fixture server.

The optional `mise run probe:host` reproduces the historical socket/SSE host
capability findings in [transport validation](transport-validation.md).
Native tests or compilation alone do not prove actual WASI loading.

For optional real-Hive verification, load the plugin with `url` pointing at an
existing compatible daemon. Compare catalogue details and daemon version with its
operational HTTP responses. Select a session and confirm the observer count does not
increase. Inspect messages/control in Hive's own clients, rather than creating or
mutating a session merely to test the dashboard.

## Packaging

```sh
mise run package
```

The outputs are `dist/zjhive-<version>.wasm` and `dist/zjhive-<version>.sha256`.
In `dist/`, run `sha256sum --check zjhive-<version>.sha256`, substituting the current
workspace version. No platform-specific native archive is built or published.
gh-ship continues to own release preparation, versioning and tags; local packaging
does not test release credentials or execute uploads.

See [validation results](validation-results.md) for executed checks. Historical
companion-era results are not evidence for the new dashboard implementation.
