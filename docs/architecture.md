# Architecture and compatibility

## Responsibilities

The plugin discovers sessions through Hive's operational `GET /v1/sessions`.
It uses completion-based Zellij WebAccess for finite calls to the local companion.
Each registered plugin gets a separate ACPv2 connection to Hive `/acp`.
The companion forwards all control, prompts, cancellation and human decisions
through ACP; it adds no state-changing HTTP route to the Hive daemon.

Hive alone owns sessions, workers, fencing, replay and turn execution. Local
registration handles, bounded update/interaction queues and uncertain prompt keys
are transient client state. They do not create a second session manager.

The plugin requires Zellij 0.45.1 and matching `zellij-tile =0.45.1`. The build
target is `wasm32-wasip1`, confirmed by that release's upstream build pipeline and
live host loading. Preview 2 components and browser WebSocket APIs are not the
supported plugin ABI. Workers use the same host API; CLI pipes supply IPC, not a
network client. A native network pump would still need native distribution.
The tested finite-HTTP adapter preserves the existing flow without new IPC semantics.

## Versioned boundaries

The extracted client uses published ACP Rust SDK and HTTP transport **3.4.0**,
including unstable ACPv2. Compatibility was tested against Hive revision
`0100e387d91f886b8af7eab14ca72f866401decc` before the route migration. The extracted
client now targets the `/v1` operational API introduced by Hive `9cf7f30` and was
revalidated against Hive `93ccd98` with the extraction applied. The old `/hive/v1`
routes are not used, so older daemons exposing only those routes are unsupported.
The browser UI moved to `/ui`; ACP remains at `/acp`. No broader version-range
guarantee is made.
Hive extensions used include `_meta.hive` role/generation/token/key metadata,
`_hive_seq` inclusive replay, sequence numbers, `_hive_control`, and catalog fields.
See Hive's [ACP contract](https://noirbizarre.github.io/hive/acp-compatibility/).
Draft ACP or Hive-extension changes require compatibility testing and may require
a coordinated update; using the same SDK version alone does not prove compatibility.

The companion `/health` returns `ready`, `protocol: 1`, and package `version`.
The plugin refuses incompatible protocol versions before registration. Install
plugin and companion from the same release; an old Hive-owned bridge is rejected.
Local `/bridge` names/configuration are retained for migration, not promised as a
public third-party API. Hive remains the only authority for mutation acceptance.

Client validation and conformance cases originate from Hive `0100e38`, under MIT.
They are owned/tested here with the published SDK, without a source-tree dependency.
Hive keeps its own authoritative validation. Library diagnostics redact submitted
values because forms can contain credentials.

## Configuration and trust

Defaults are daemon `http://127.0.0.1:8765` and companion
`http://127.0.0.1:8766`. The companion accepts `--listen`, `--url`, and `HIVE_URL`.
Only loopback listeners are permitted. HTTP base URLs are converted structurally
to WebSocket URLs (HTTPS to WSS); this does not promise remote Hive support.
Hive currently supports one trusted local OS user, with no separate client
authentication or TLS configuration here. Pilot tokens fence stale controllers;
they are not authorization against malicious local users. Do not expose either
listener through a public proxy. No credentials are added to the plugin settings.

## Failure and lifecycle behavior

- First attach observes; takeover is explicit. Relinquish remains an observer.
- Replay is inclusive and bounded (256 updates); gaps require explicit live-only
  recovery, visibly marked as missing earlier history. Restart rollback is stale.
- Upstream disconnect clears ownership and interactions, retries after two seconds,
  and reconnects as observer using the last sequence. It never silently takes control.
- Plugin polling is approximately two seconds for a selected session and four for
  the catalog, with failure backoff up to sixteen seconds. Stale views disable control.
- Prompt acknowledgements mean accepted, not complete. An uncertain acknowledgement
  retains exact key/text for a same-key retry and blocks a distinct new prompt.
  Slow acknowledgements do not block polling, cancellation or interaction replies.
- Permission IDs and interaction identities prevent stale answers. Forms are validated;
  unsupported schemas or unsafe URLs cannot authorize anything. Session-scoped
  questions follow the pilot; request-scoped elicitation can follow its original caller.
- URL consent is distinct from completion; early completion and deadlines are correlated.
- Companion handles expire after thirty seconds without calls, checked every five
  seconds. Detach or expiry drops the ACP client, never closes a Hive session.
- A manually started companion stays available until stopped with Ctrl-C or a process
  supervisor. Zellij's command execution callback is emitted on process exit, not
  incrementally; updates therefore use finite HTTP polling, not helper stdout.

## Existing limitations

The UI opens existing sessions; create them in another Hive client. It does not
offer durable transcripts, interrupted-work acknowledgement or every rich ACP event.
Use Hive's TUI/browser for those workflows. Replay and pending keys are not durable
across a complete companion restart; after losing in-memory retry identity, inspect
the daemon session before submitting again. Human prompts may need to be reissued
after transport loss. The adapter is a local client, not a resumable job scheduler.
