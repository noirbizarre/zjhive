# Transport decision and real-host evidence

## Reproduction

```sh
cargo build --locked -p transport-probe --target wasm32-wasip1 --release
python tools/transport_probe.py
```

This launches a uniquely named Zellij session in a sized PTY with temporary config,
grants the probe's WebAccess request, and runs a loopback HTTP fixture on port 18769.
It kills only its own session. The port must be free. It prints JSON observations
and fails if finite HTTP or the final SSE callback was not demonstrated.
It does not use a browser or standalone WASM runtime.

## Observed on 2026-10-24

Host: `/usr/bin/zellij` 0.45.1, Linux x86-64. SDK: `zellij-tile =0.45.1`.
Compiler: Rust 1.99.0; target: `wasm32-wasip1`.

1. Direct `TcpStream::connect` to the listening fixture returned
   `Unsupported: operation not supported on this platform` inside the real plugin.
   Failure occurred before a WebSocket handshake or ACP initialization; neither
   is claimed to have succeeded directly. Preview 1 does not expose socket creation
   through the supported Zellij imports. No arbitrary browser/JS WebSocket host exists.
2. Finite GET completed at 0.37 seconds, status 200, body `finite-ok`.
3. SSE fixture flushed messages at 0.37, 2.37 and 4.37 seconds and closed at 6.37.
   There was no intermediate SSE callback. A single result at 6.37 contained all
   three messages. Six timer ticks rendered while the HTTP request was pending.
   An indefinitely open ACP SSE stream therefore cannot deliver its messages
   incrementally through this supported API.
4. The SDK's workers and CLI pipe/message surfaces supply background processing
   and IPC, not additional socket creation. No native-free persistent transport
   was identified. CLI-pipe networking was not live-prototyped; it would still
   need an external network pump. The companion-free dashboard therefore uses only
   finite operational HTTP requests and leaves ACP interaction to Hive's own clients.

Supporting exact-version source:

- [Plugin build target](https://github.com/zellij-org/zellij/blob/v0.45.1/xtask/src/build.rs)
- [Host WebRequest implementation](https://github.com/zellij-org/zellij/blob/v0.45.1/zellij-server/src/background_jobs.rs)
- [WASI loader/imports](https://github.com/zellij-org/zellij/blob/v0.45.1/zellij-server/src/plugins/plugin_loader.rs)

The host awaits `res.bytes()` before emitting `WebRequestResult`. The experiment
confirms the consequence in the supported runtime, rather than relying on that
source alone. These findings concern the supported API/version, not every possible
future runtime or networking extension.

An initial `cdylib` artifact compiled but failed host loading with “could not find
exported function.” Building a WASI executable supplies `_start`; both diagnostic
and actual plugins subsequently loaded. Native tests/WASM compilation alone did
not catch this ABI issue.

Historically, the former companion architecture passed a real-host integration smoke:
catalog selection, observer attachment, takeover, accepted prompt, stale state on
companion loss, observer-only reconnect after companion restart, detach and session
survival. Permission/form/URL semantics and old-pilot fencing additionally passed
the external-Hive contract test; those were automated protocol tests, not claims
of manually exercised forms in the Zellij host. See [testing](testing.md).
