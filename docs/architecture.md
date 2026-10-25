# Architecture and compatibility

## Responsibilities and boundaries

ZJHive is a read-only catalogue/status dashboard distributed as one WASM plugin.
Hive alone owns sessions, agents and conversation/control workflows. The plugin
requests only Zellij WebAccess and issues finite GET requests:

- `/v1/sessions`: authoritative catalogue snapshots, including offline/recovery records.
- `/v1/status`: daemon version, default agent and live-session counts by profile.

Selecting a session opens local metadata details, without attaching an ACP observer.
The dashboard neither connects to `/acp` nor consumes SSE. There are no native
commands, network helpers or loopback listeners. It displays a `/ui?session=...`
URL for handoff; it does not launch a browser.

`main.rs` contains host integration; `model.rs` projects the operational JSON API;
`dashboard.rs` owns pure state transitions and rendering. The optional
`transport-probe` crate is a developer diagnostic, excluded from release assets.

## Compatibility

The supported host is Zellij 0.45.1 with `zellij-tile =0.45.1`, targeting
`wasm32-wasip1`. This is a WASI executable, not a Preview 2 component or a browser
module. See [the real-host transport evidence](transport-validation.md).

Hive must expose the `/v1` operational API introduced by revision `9cf7f30`.
Dashboard models were checked against local Hive revision `0f0662b`.
This is a schema reference, not a claim of a live-daemon integration test.
The former `/hive/v1` routes are unsupported.

Catalogue responses require `revision`, `sessions`, and each session's nonempty
unique `sessionId`, nonempty `agent`, and `cwd`. Additional fields are tolerated;
newer lifecycle, recovery and usage metadata may be absent. Unknown status values
remain inspectable. Usage tokens are nested under `usage.tokens`; absent values
are unknown, not zero. Estimated cost and actual session cost remain separate.

Catalogue revision is not a durable daemon incarnation or ACP replay cursor.
Lower revisions after restart are accepted. Status and catalogue are independent
snapshots: momentary count differences are normal. `activeByAgent` counts live
sessions, including idle sessions, rather than running work.

## Configuration

`url` defaults to `http://127.0.0.1:8765`. HTTP(S) base URLs may have a proxy path
prefix; credentials, query strings, fragments and other schemes are rejected.
No authentication configuration is supplied. The legacy `bridge` setting is ignored
with a migration message; the old companion is not required or launched.

## Freshness and failure behavior

- Catalogue polling runs approximately every four seconds, status every twenty.
- Each endpoint independently tracks success, errors, in-flight requests and capped
  exponential retry backoff (up to sixty seconds). `r` requests an immediate refresh
  when request capacity is available.
- Malformed responses and HTTP failures preserve cached data, explicitly marked stale.
  Successful status refresh cannot make an old catalogue fresh, or vice versa.
- Selection follows session identity through reorder. If it disappears from a valid
  snapshot, details close with an explanatory message.
- Requests become overdue after fifteen timer ticks. Since the host API cannot
  cancel them, at most two outstanding requests per endpoint are allowed. Expired
  callbacks release capacity but never replace newer accepted data. If both stay
  stuck, reload the plugin; retries never accumulate without bound.
- Daemon text is filtered for terminal controls and constrained to pane width.
  Navigation remains visible when a catalogue exceeds the viewport.

## Scope

The dashboard does not expose conversation history, prompts, takeover, cancellation,
permissions, forms or URL-consent workflows. Full interactive ACP support without
a native helper needs finite ACP delivery in Hive or streaming/socket APIs in Zellij.
Provider discovery, filtering and richer accounting views are deferred.
