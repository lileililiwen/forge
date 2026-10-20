## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The readiness judgement of a release plan | `forge` | Yes. `src/release/engine/prepare.rs`. |
| The apply step's use of that judgement | `forge` | Yes. `src/release/engine/apply.rs`. |
| The release check kinds and their statuses | `forge` | No. Read only. |
| The release stages, identity and retry rules | `forge` | No. Read only. |
| The `forge.yaml` contract and its parser | `forge` | No. Read only. |
| The gate and DriftWatch adapters | `forge` | No. Read only. |
| The web release contract fixture | `forge` | Yes. `tests/forge_web_project_release_contract.rs`. |
| Every sibling module | its owner | No. Not edited. |

## Language and Runtime

- **Forge Core** is Rust, in the root `forge` crate.
- The change adds one pure function and one small value type to the release
  engine. It touches no I/O, spawns no process and reads no file. The checks
  themselves are already captured by the caller; this change only judges them.
- The files it touches are well under the repository's 1000-line cap:
  `src/release/engine/prepare.rs` is 475 lines, `src/release/engine/apply.rs` is
  465 lines and `src/release/engine/engine_tests.rs` is 410 lines.

## Ownership and Shared Code

Two call sites judge a release:

| Call site | File | Effect of the judgement |
|---|---|---|
| Prepare | `src/release/engine/prepare.rs::prepare_release` | Sets `PlanReport.ready` and the plan note. |
| Apply | `src/release/engine/apply.rs::apply_release` | Refuses the release before any stage runs. |

Both call sites currently repeat the same expression:

```rust
checks.iter().filter(|c| c.applicable).all(|c| c.status == CHECK_PASS)
```

The change replaces both with one call to one function. The function lives in
`src/release/engine/prepare.rs` next to `collect_captured_checks`, which the
apply step already imports, so the two call sites cannot drift apart.

The four known check kinds are currently a local array inside
`collect_captured_checks`:

```rust
let known = ["doctor", "test", "driftwatch", "gate"];
```

The change hoists it to a module constant so the judgement and the capture
agree on what a check kind is.

## User Experience and Interface

No verb is added and no verb changes shape. The plan gains one honest verdict
where it used to report a vacuous pass, and the apply step refuses.

`forge release prepare` still exits zero for a not-ready plan, exactly as it
does today for a failing check: the plan is a read-only review artifact, and
`ready: no` is its verdict. The registry records the operation as `blocked`,
as it already does for a failing check.

`forge release apply` refuses with `error[release-check-failed]`, the code it
already uses for a failing check. The reason names the missing evidence.

## Behavioral Model

| Actor | Block | Boundary |
|---|---|---|
| A project that declares a check that passes | The plan is ready. | The count of passing checks is reported. A project that declares `doctor` and `test` behaves exactly as before. |
| A project that declares a check that fails, is unavailable or is stale | The plan is not ready and the reason names the check and its status. | Unchanged behaviour. |
| A project that declares no check | The plan is not ready and the reason names the missing check and the four kinds `release.checks` accepts. | The plan reports no passing check count. |
| The apply step for a project that declares no check | The release is refused with `release-check-failed` before the commit stage. | No tag, no push, no publication, and no release state that claims a delivered stage. |
| The apply step for a project whose check fails | The release is refused, as before. | Unchanged behaviour. |

## Contract and Compatibility

The release contract, the `forge.yaml` contract and the manifest schema version
are untouched. A manifest with an empty `release.checks` list still parses and
still resolves a `ReleaseConfig`; the release it describes is refused.

The readiness rule, stated once:

> A release is ready when at least one check is applicable and every applicable
> check passes.

`all()` on an empty set is not a rule a release can pass on. The judgement
therefore counts the applicable checks first, and reports the absence of
evidence as its own reason rather than as a failing check.

The reason strings:

| Case | Reason |
|---|---|
| No applicable check | `no applicable release check; declare at least one of doctor, test, driftwatch, gate in release.checks` |
| An applicable check that is not passing | `failing check(s) <kind>:<status>, ...` |

The plan note keeps its existing shape:

```
release plan not ready: <reason>; release execution blocks before tag or publication
```

so a project with a failing check reads exactly as it did before.

The apply refusal keeps its existing shape too:

```
<reason> at revision `<short revision>`
```

The `release-check-failed` error renders the closing clause "release execution
blocks before tag or publication" itself, so the reason carries the evidence
and the revision only. The refusal text repeated that clause before this
change; it now appears once.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The project declares no check | The plan is not ready and the apply step refuses. |
| The project declares a check that cannot run | The check is `unavailable`, the plan is not ready and the reason names the check. Unchanged behaviour. |
| The project declares a check that is stale | The check is `stale`, the plan is not ready. Unchanged behaviour. |
| The project declares a check that passes | The plan is ready. Unchanged behaviour. |
| The manifest has no `release` block at all | The release is refused earlier, by `release_config_from_manifest`. Unchanged behaviour. |
| The changelog is missing | The release is refused earlier, by the changelog load. Unchanged behaviour. |
| The apply step runs without `--confirm` | The confirm refusal still fires. The readiness judgement runs first, exactly as the failing-check judgement does today, so a project with a passing check still sees the confirm refusal. |

## Verification Oracle

```sh
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
forge gate --dry-run
forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
git diff --check
```

The oracle is the behaviour, not the build:

- A project that declares no release check produces a plan that is not ready,
  whose reason names the missing check, and that reports no passing check count.
- The apply step for that project refuses with `release-check-failed`, and the
  project has no tag afterwards.
- A project that declares a passing check still produces a ready plan, and its
  apply step still walks the stages.
- A project whose check fails is still refused, with the failing check named.

## Implementation Plan

**The judgement.** `src/release/engine/prepare.rs` gains:

- `KNOWN_CHECK_KINDS` — the four check kinds, hoisted out of
  `collect_captured_checks`.
- `CheckReadiness` — a small value type with a `ready` flag and a `reason`.
- `check_readiness(checks)` — the single rule.

`prepare_release` uses it to set `PlanReport.ready` and the plan note.
`apply_release` uses it to decide whether to refuse.

**The procedure description.** `src/procedure/catalog.rs` describes the
`prepare-release` procedure. Its sentence about refusing becomes true for the
new case as well.

**The tests.** `src/release/engine/engine_tests.rs` gains a fixture whose
manifest declares no release check, and the tests that pin the rule: the plan
is not ready, the reason names the missing check, the apply step refuses, and
no tag exists afterwards. A unit test of the judgement covers the empty set,
the all-disabled set, a passing set and a failing set.

**The web release contract.** `tests/forge_web_project_release_contract.rs`
built its fixture with a `release:` block that declared no check, and its
comment said the plan was `ready` for exactly that reason. The fixture is
given the `doctor` and `test` checks and a minimal Cargo workspace so both
pass. The test's eight assertions are unchanged; only the fixture's evidence
changed. This is the one place where the old behaviour was load-bearing, and
it is a test fixture, not a product path.

## Decision Ledger

- **Resolved:** the rule lives in the engine, not in the manifest parser. A
  manifest with an empty `release.checks` list is a valid manifest; the
  release it describes has no evidence. Refusing the manifest would move the
  failure away from the release and into every other reader of the manifest.
- **Resolved:** the rule is one function used by two call sites. The two
  call sites already repeat the same expression today, which is how the two
  drifted into agreeing on a vacuous pass. One function is the fix.
- **Resolved:** the empty set is reported as its own reason, not as a failing
  check. "failing check(s) " with nothing after it would be a worse message
  than the one it replaces.
- **Resolved:** the reason names the four check kinds. The owner can act
  without reading the source.
- **Resolved:** the plan keeps its exit-code behaviour. `forge release
  prepare` is a read-only review step; it already reports `ready: no` for a
  failing check, and it now reports `ready: no` for a missing check too.
- **Resolved:** `forge publish` is not touched. The publish lane has no
  release-check coupling today; folding the lanes together and making the gate
  mandatory is the `single-delivery-lane` package.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order. The
  authorized surfaces are the files listed in the proposal.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A release plan is ready only with applicable evidence | The readiness judgement replaces the repeated expression | `forge release prepare` reports `ready: no` and the missing check | Ready needs at least one applicable check, and all of them passing | `src/release/engine/prepare.rs`, `src/release/engine/apply.rs` | No declared check is refused; a passing check is ready; a failing check is refused | 2.1–2.4 | Engine tests, the repository suite |
| The apply step cannot apply a release without evidence | The apply call site uses the same judgement | `forge release apply` refuses with `release-check-failed` | One function, two call sites | `src/release/engine/apply.rs` | The apply step refuses and no tag exists; a failing check is still refused | 2.5 | Engine tests, a real run |
| The refusal tells the owner what to declare | The reason names the four check kinds | The owner reads the kinds in the refusal | The known kinds are one constant | `src/release/engine/prepare.rs` | The reason names `doctor`, `test`, `driftwatch` and `gate` | 2.4 | Engine tests |
