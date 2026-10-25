# AGENTS.md

Notes for anyone — human or otherwise — changing this repository.

## What this project is

A read-only Zellij catalogue/status dashboard for Hive, distributed as one WASI
plugin. Hive alone owns sessions and agents. The plugin uses finite operational
HTTP requests through Zellij WebAccess; it has no native runtime helper.

## Non-negotiable invariants

Each of these should be enforced by a hook or a test. An invariant nothing
checks is a comment, and it will be violated.

1. **WebAccess only, finite GETs, no native commands** — enforced by
   `tools/test_dashboard.py` and `dashboard.rs` endpoint tests.
2. **Session details never attach or control** — enforced by the local-details
   test in `crates/zjhive/src/dashboard.rs` and the real-host smoke.
3. **Failed refresh preserves cached data as stale** — enforced by the independent
   freshness tests in `crates/zjhive/src/dashboard.rs`.
4. **One WASM artifact and matching checksum** — enforced by `tools/test_dashboard.py`.

## Layout

A Cargo workspace: `Cargo.toml` at the root is a virtual manifest — no code of
its own — and each member crate lives under its own directory with its own
`Cargo.toml` and `src/`.

## Style

**Every non-obvious line carries a comment saying why.** Not what — the code
says what. Ideally naming the failure it prevents. A comment that restates the
code is worse than none.

**Errors are typed and actionable.** `thiserror` for the library, `miette` at
the binary edge. A diagnostic must carry the two things the user does not
already know: what specifically failed, and what to do about it. Diagnostic
codes are `zjhive::<module>::<kind>`, and a code is a public identifier
users grep for — renaming one is a breaking change.

**Test names are sentences.** `an_unchanged_input_produces_no_output`, not
`test_run_2`. The name should say what would be broken if it failed.

## Commits

Conventional Commits, enforced by commitlint on `commit-msg`. The type becomes a
changelog heading, so choose it as if someone will read it in release notes —
because they will.

## Releases

Driven by gh-ship. Never bump a version or push a tag by hand: `cliff.toml`
derives the version from the commit history, `prepare-release` applies it, and
`.github/ship.yml` is the contract between them. See CONTRIBUTING.md.

## Before you push

```sh
mise run ci
```

Formatting, Clippy, spelling, workflow and Markdown linting, tests. Same as CI.

## This repository is generated from a template

The toolchain, hooks, CI and release workflows come from
[rust.tpl](https://github.com/noirbizarre/rust.tpl) and are updated with
`git tpl update`. Files carrying template-owned content end with a
`# --- project-specific ---` marker: add below it, never above.

Changing template-owned content here fixes it in one repository. Changing it in
the template fixes it in all of them — prefer that.
