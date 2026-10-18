# Design: Governance and Gate status projection separation

## Implementation boundary

Repository/project: Forge Rust workspace. Change `tests/governance_cross_surface.rs` and add the canonical delta in `openspec/changes/governance-status-surface-regression/specs/governance-status-surface-regression/spec.md`. Do not change `src/governance/engine.rs`, `src/checker/mod.rs`, or `src/portal/sections.rs`: source inspection shows their status ownership is already correct.

The relevant existing public entry points are `forge governance status <project>`, `forge check <project>`, and `forge portal view settings <project>`. Their JSON outputs are the test oracle.

## Language and runtime

Rust 2021 workspace, Forge CLI integration tests, stable toolchain declared in `Cargo.toml`/`rust-toolchain.toml`. Focused command:

```sh
RUSTC_WRAPPER= CARGO_INCREMENTAL=0 cargo test --test governance_cross_surface -- --test-threads=1
```

Related provider contract command:

```sh
RUSTC_WRAPPER= CARGO_INCREMENTAL=0 cargo test --test governance_preset_contract -- --test-threads=1
```

The full suite is `scripts/check-tests.sh`; the required runtime Gate is `./target/debug/forge gate --dry-run`, followed by bounded `./target/debug/forge gate --timeout-secs 1200` after building the current binary.

## Ownership and shared code

Forge owns normalized observations and the checker/portal projections. Workspace Governance owns audit findings and emits adapter observations. Gate evidence belongs to Forge Gate. No new shared package, schema, adapter, or dependency is introduced.

## User experience and interface

No user-visible behavior changes. The CLI governance status response exposes the normalized governance provider and status. Checker exposes its warning/error alert. Portal `settings` exposes the Gate aggregate and freshness only. Existing accessible interfaces and navigation are unchanged; responsive, keyboard, focus, screen-reader, and browser behavior are not affected by this test-only correction.

## Behavioral model

| Input condition | Governance status | Checker projection | Portal settings projection |
|---|---|---|---|
| Configured adapter path is missing/unstartable | `unavailable` | Warning attributed to selected governance provider | Gate evidence is evaluated independently; absent evidence is `unknown` |
| Adapter returns `fail` | `fail` | Error attributed to selected governance provider | Gate evidence is evaluated independently; absent evidence is `unknown` |
| No Gate evidence exists | No effect on governance observation | No effect on governance alert | `unknown`, `freshness=absent`, with no governance provider attribute |

The test shall run governance status after the adapter is removed and after a fixture returns `fail`; it shall assert the checker mapping for each. For both states, portal settings remains an absent-evidence Gate row and does not claim to carry governance state.

## Contract and compatibility

No serialized contract changes. Governance status JSON shape, checker alert schema, portal entry schema, Gate evidence schema, and adapter protocol version remain unchanged. The test no longer relies on a stale portal interpretation introduced before the settings row became Gate-specific.

## Failure and boundary policy

- Missing adapter or spawn failure: normalized governance status is unavailable; checker reports a warning.
- Provider-reported fail: governance status remains fail; checker reports an error.
- Missing Gate evidence: portal reports unknown, not unavailable or healthy.
- A malformed provider configuration remains a typed command error and is outside this regression's fixtures.
- Test fixture setup errors fail with existing contextual assertions; no fallback status is accepted.

## Verification oracle

1. Run the focused regression and related provider contract tests; inspect CLI JSON status, checker severity/symbol, and portal status/freshness/attributes.
2. Run `scripts/check-tests.sh` to cover every workspace test target.
3. Run Forge Gate dry-run and full Gate with a bounded timeout.
4. Run name preflight, `openspec validate --all --strict --no-interactive`, and `git diff --check` before archive.

Expected observable assertions: missing adapter => CLI `unavailable`, checker warning, portal `unknown` + absent Gate freshness; adapter `fail` => CLI `fail`, checker error, portal still `unknown` + no governance attribute.

## Decision ledger

- **Resolved:** test failure is caused by stale assertions, not a production status mapper. Evidence: the portal function reads only `crate::gate::load_latest_evidence`; its absent branch returns `PortalStatus::Unknown` and `freshness=absent`.
- **Resolved:** preserve Gate and governance ownership and correct the integration oracle. The previously suggested source mapping fix is superseded by this traced evidence.
- **Deferred:** a dedicated portal governance observation surface would be a separate product feature and is not needed for this fix.
- **No blocker:** current source and canonical behavior provide exact expected outputs.
