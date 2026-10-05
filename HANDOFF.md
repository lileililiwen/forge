# Forge handoff

## Current state

`scaffold-prewires-shared-layer` is implemented, verified and archived. Its
requirements were promoted into
[openspec/specs/scaffold-prewires-shared-layer/spec.md](openspec/specs/scaffold-prewires-shared-layer/spec.md).

`contract-parity-gate-real-digests` is active and implemented but **not
archived**. It makes `scripts/contract-parity.sh` actually compare the vendored
contract bytes against the resolved `platform-contracts` source: the previous
version performed zero comparisons and printed an unconditional
`contract-parity: OK`. It also re-syncs three drifted mirror files. The change is
not archived and no commit has been made for it.

current_spec: governance-adapter-request-write-race

## Three changes are in flight and none is archived

`.ai-rules/workflow.md` allows one active change at a time. **Three** active
change directories exist right now, because the owner explicitly authorized
this work to proceed alongside the two already parked. Nothing below was
archived, deleted or committed on another change's behalf.

| Change | State | Concern |
|---|---|---|
| `contract-parity-gate-real-digests` | active, implemented, **not archived**, no commit | vendored contract bytes |
| `manifest-wire-contract-shape` | active, implemented, verified, **not archived** | the emitted manifest's wire shape |
| `governance-adapter-request-write-race` | active, implemented, verified, **not archived** | a governance adapter that never reads its request |

The three are independent concerns and none depends on another. The
`current_spec:` pointer above names the change most recently worked on;
`governance-adapter-request-write-race` was selected next, not instead of the
other two.

`manifest-wire-contract-shape` is implemented, verified and **not archived**.
It is a *second* active change alongside the parked
`contract-parity-gate-real-digests`; that change is untouched and still
unarchived.

## What governance-adapter-request-write-race does

`tests/governance_contract.rs` was flaky: ~50 % failure rate, 1–2 tests failing
per run, varying every time. It was reported as a shared-state problem and it
was **not** one. Two measurements settle it:

```
$ for i in $(seq 1 20); do cargo test --test governance_contract -- --test-threads=1; done
7 runs FAILED (1-3 tests each), 13 runs ok
```

Serialising the target does not fix it, which is what a per-test race does and
what shared state does not. The four tests that ever failed are exactly the
four whose adapter script never reads standard input:

```sh
#!/bin/sh
printf '%s' '{"provider":"external", ...}'
```

Such an adapter exits immediately and closes the read end of its stdin pipe,
so Forge's own request write can lose the race and come back
`Broken pipe (os error 32)`. `run_external_provider` maps that to
`ProviderStatus::Unavailable` and **throws the adapter's answer away** — the
answer was already complete on stdout. Measured detail, captured through the
real `save_provider_selection` + `check_project` path:

```
status=Unavailable detail=Some("cannot write adapter request: Broken pipe (os error 32)")
```

The fix is in `src/governance.rs`, not in the test. `BrokenPipe` is the one
write error that says something about the *adapter* rather than about Forge's
plumbing, so `write_adapter_request` treats it as a completed write and Forge
reads the exit status, stdout and stderr the adapter actually produced. Every
other write error keeps its typed unavailable refusal, and now kills and reaps
the child before returning. No test was ignored, serialised, slept or retried.

The shared-state hypothesis was tested and cleared rather than assumed: no
`set_var`/`remove_var` or `set_current_dir` on this path, no `static` /
`OnceLock`, per-test `TempDir`, pid-keyed temp file names that cannot collide
across distinct directories, no socket, and read-only fixtures.

### Verification (2026-10-05)

- `cargo test --test governance_contract`, **245 consecutive runs** at default
  parallelism across five blocks: **244 passed, 1 failed**, and the final
  contiguous block of 20 was **20/20 passed** (see the open item below). A
  further 20 runs at `--test-threads=8`: 20/20 passed. A further 20 at
  `--test-threads=1`: 20/20 passed.
- Before the fix, for comparison: 13 failures across 8 of 20 default runs, and
  7 failures across 4 of 20 serialised runs.
- The deterministic guard was checked against the **pre-fix** code: with the
  `BrokenPipe` arm removed,
  `a_request_write_to_an_adapter_that_already_exited_is_not_a_failure` fails
  1/1 with `cannot write adapter request: Broken pipe (os error 32)`. It is not
  a vacuous test. The end-to-end guard
  `an_adapter_that_never_reads_its_request_still_answers` is a *weaker*
  instrument — it detected the pre-fix defect in 0/10 whole-target runs,
  because `run_adapter` writes immediately after `spawn()` and the child must
  win a sub-millisecond race. It is kept as the only place in the suite that
  names the scenario as a requirement, not as the deterministic pin. A guard
  that raised the odds with CPU pressure or repetition was rejected: that is the
  nondeterminism this change exists to remove.
- `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`,
  `cargo fmt --check`, `cargo clippy --workspace --all-targets` (zero findings
  in either file this change touches), `git diff --check`,
  `node scripts/check-openspec-change-names.mjs`,
  `openspec validate --all --strict --no-interactive` — see the totals in the
  change's `tasks.md` §4.
- No shared Gate Runtime is configured; no Gate pass is claimed.

### Outstanding

- **One unexplained failure in 245 runs.** A single run of the 245 (default
  parallelism) reported `20 passed; 1 failed` and the capture did not record
  which test or why. It was not reproduced in the 20 runs at
  `--test-threads=8`, the 20 at `--test-threads=1`, or 220 further default
  runs, and it is **not** claimed as fixed. It is most likely a different,
  environmental flake — see the studio port note below — but that is a
  hypothesis, not a measurement.
- **Three adjacent defects found while mapping `run_adapter`, not fixed here**
  and recorded in the change's `design.md` §5: stdout is not drained while the
  child runs, so an adapter writing more than one 64 KiB pipe buffer deadlocks
  until its deadline against a 256 KiB `MAX_ADAPTER_OUTPUT_BYTES`; `git_revision`
  has no deadline at all; the wait loop polls at 10 ms. None of them is the
  cause of this flake — no adapter in this repository produces more than ~10 KiB
  — and the first needs concurrent drain + deadline handling, a materially
  larger change.
- **`tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`
  is a separate, pre-existing flake**, seen in a whole-suite run and outside
  this change's scope. It binds the fixed range 45800–45863, which lies inside
  this machine's Linux ephemeral range (`/proc/sys/net/ipv4/ip_local_port_range`
  = `32768 60999`), so any concurrent outbound connection from any process can
  take one of the 64 ports and the test's own bind then returns 63. Mechanism
  proved, not guessed: with another process holding the range, the same
  assertion fails (`left: 0, right: 64` when all 64 are held; `left: 63,
  right: 64` when one is). It passes in isolation. The fix is not a free port
  or a free port *range*; it belongs to its own change.

## What manifest-wire-contract-shape does

Forge is the **producer** of the manifest a separate static Hugo site consumes.
Three fields it emitted were rejected by the consumed schema, so the first real
export would have failed the consumer's build. The schema is
`platform-contracts/schemas/public-portfolio-manifest.schema.json`
(`platform.public-portfolio-manifest/1.0.0`, digest
`07a3c47769da8923998273cda602ddffb195f983e3dcf344fb1d86540c6bc986`).

| Field | Was | Now |
|---|---|---|
| `schema_family` | `"public-portfolio-manifest"` | `"platform.public-portfolio-manifest"` |
| `schema_version` | JSON number `1` | string `"1.0.0"` |
| `manifest_revision` | JSON number (`u32`) | string `"rev_<revision>"` |

The internal revision is **unchanged everywhere it is stored**: the
`manifest_revision INTEGER` column, the `i64` approval, audit, publication
report and adapter envelope, and `build_manifest(records, u32)`. Only the
serialized field is a string, produced by one function,
`wire_manifest_revision`, so no call site can invent a second spelling.

The encoding is a pure, injective function of the integer, so an unchanged
catalog still hashes identically. One honest consequence: an approval made
before this change is bound to the old hash, so `publish` now refuses it until
the operator previews and approves again. No stored approval or audit entry was
rewritten — that would forge an approval nobody gave.

## Verification (2026-10-05)

- `cargo build`: clean. One `unused import: ShareSurface` warning in
  `src/portfolio/share/validation.rs`, **verified pre-existing** (present with
  this change stashed).
- `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  **2294 passed / 0 failed / 3 ignored**. The skip is the pre-existing hang
  described under "Pre-existing conditions" below. The three ignored tests are
  the pre-existing `contract::tests::parity_walk`, the pre-existing
  `packing_leaves_the_sibling_checkout_byte_identical`, and this change's own
  acceptance test, run explicitly.
- `cargo test --test manifest_wire_contract -- --ignored --nocapture`:
  **1 passed** — `jsonschema` accepted a Forge-produced manifest.
- **Acceptance, consumer's own oracle**: a scratch registry, a real
  `target/debug/forge portfolio share set → preview → approve → publish` cycle,
  and then
  `python3 lileililiwen.github.io/scripts/validate_manifest.py --strict`
  against `/home/paul/code/platform-contracts/schemas/public-portfolio-manifest.schema.json`:
  `manifest OK … (schema 1.0.0, 1 project(s), revision rev_1)`, **exit 0**.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 65 passed / 0 failed.
- `cargo fmt --check`: clean (it was already clean on `main` before this
  change; the "five dirty files" note further down is stale).
- `cargo clippy --workspace --all-targets`: exit 0, and **zero findings in any
  file this change touches** (`src/portfolio/share/mod.rs`,
  `src/portfolio/share/manifest.rs`, `tests/portfolio_share_cli_contract.rs`,
  `tests/manifest_wire_contract.rs`). The remaining warnings are all in files
  this change does not own.
- `git diff --check`: PASS.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## Three further contract mismatches found, and not fixed

Found while mapping the document, reproduced, and deliberately left as
non-goals of this change rather than silently widening its scope. All three are
reachable from the command line today, and each produces a published manifest
the consumer rejects.

1. **`visibility`** — the schema's enum is `["public"]`; Forge's `Visibility::ALL`
   also admits `unlisted`. `share set --visibility unlisted` publishes a
   document rejected with `projects/0/visibility: 'unlisted' is not one of
   ['public']`.
2. **`status_evidence`** — the schema is `additionalProperties: false` and
   permits only `observed_at`, `source`, `note`; Forge's `EVIDENCE_KEYS` admits
   `observed_at`, `source_system`, `source_revision`, `state`, `source`. A
   record with `--evidence '{"observed_at":"…","source_system":"…"}'` is
   rejected with `Additional properties are not allowed ('source_system' was
   unexpected)`.
3. **`id` length** — the schema caps `id` at 64 characters.
   `validate_project_id` already pins the schema's *pattern* exactly but has no
   length bound, so a 70-character id registers, publishes and is rejected with
   `projects/0/id: … is too long`.

Fixing these means deciding whether Forge narrows its vocabularies or the
contract widens them. That is a product decision, not a mechanical one, and it
belongs to its own change.

## A mirror gap worth naming

`contracts/schemas/public-portfolio-manifest.schema.json` is **absent from
Forge's own mirror**, which is why nothing inside this repository could have
caught the shape defect at all. Re-syncing the mirror belongs to
`contract-parity-gate-real-digests`, which is still parked. Until it lands, the
acceptance test reads the schema from the sibling checkout and is `#[ignore]`d
rather than silently skipping, so a green suite never stands in for a check that
did not run.

The work landed on `main`. It was originally committed on a
`feat/scaffold-prewires-shared-layer` branch; `main` was fast-forwarded onto it
and the branch deleted, per owner direction that Forge work goes straight to
`main`. See "Why a branch appeared" below.

## What this change does

- A profile may declare a **versioned shared-layer kit** as a compiled-in
  descriptor resolved offline. `forge new` renders that reference into the
  generated project's own native manifest — the `kit` block in `forge.yaml` —
  with no feed access, no sibling checkout and no network (`src/kit/`,
  `src/profile/mod.rs`, `src/generate/mod.rs`).
- A profile declares a **minimum consumption floor**. Unmet is a typed refusal
  before anything is written; there is no warn-and-continue path. An operator
  can override with `--kit-exception <reason>`, which is recorded visibly in
  `forge.yaml` and the README. A profile with no registered kit records a
  **declared zero** with a `zero_reason`, which is a distinct state from a floor
  failure (`src/kit/floor.rs`).
- The `aspnet-web`, `react-web` and `nextjs-web` scaffolds **pre-wire** the
  shared layer. .NET restores from a **committed, project-relative feed** — a
  `NuGet.config` with `<clear />` and one named source at
  `packages/platform-feed` — so a fresh clone restores with no sibling checkout,
  no environment variable and no secret. The Node profiles get digest-pinned
  vendored tokens under an owned `.platform/` subtree with an ownership receipt
  (`src/kit/feed.rs`, `src/kit/assets.rs`, `kits/`).
- `forge kit` is the one new top-level verb: `pack` regenerates the feed,
  `verify` catches drift, `upgrade` is the single sanctioned way a pinned
  project moves between kit versions.
- The pinned kit id and version are observed additively on the existing project
  row. No new SQLite table (`src/registry/mod.rs`).

## The floor, and the evidence it came from

`6` for `aspnet-web`, `1` for the two Node profiles, `0`-with-reason elsewhere.
Derived from per-package distinct external consumer counts, frozen in
`kit::PLATFORM_PACKAGE_EVIDENCE`; a test fails when a classification disagrees
with the fixture.

- Confirmed (6): `Platform.Core` (7), `Platform.AspNetCore` (6),
  `Platform.Testing` (4), `Platform.RateLimiting` (2), `Platform.Idempotency`
  (2), `Platform.Observability` (2).
- Withheld despite clearing the bar, because they need a store:
  `Platform.Persistence.EfCore` (4), `Platform.Identity.AspNetCore` (3),
  `Platform.Tenant.Lifecycle.AspNetCore` (0).
- Provisional (below the bar, rendered commented, never restoring): everything
  else, each comment naming its own consumer count.

The **restore closure is 9 packages, not 6**. `Platform.Billing.Contracts`,
`Platform.Eventing` and `Platform.Web.Telemetry` arrive as project references of
the confirmed set, so the feed carries all nine. The floor still counts only the
six; the three transitive members are never counted and never directly
referenced.

## Verification (2026-10-03)

- `cargo test --workspace --all-targets --no-fail-fast -- --skip generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  **2291 passed / 0 failed**. The one skipped test is discussed under
  "Pre-existing conditions".
- `cargo test --test kit_contract`: **60 passed / 0 failed**, stable across
  three consecutive parallel runs.
- `cargo clippy --workspace --all-targets`: **zero findings in any file this
  change touches** (`src/kit/*`, `src/generate/mod.rs`, `src/main.rs`,
  `tests/kit_contract.rs`). Three findings my own code introduced during review
  were fixed rather than recorded.
- `rustfmt --check`: clean on every file this change touches.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 63 passed / 0 failed.
- `git diff --check`: PASS.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## The sibling is now provably read-only

This was the most serious defect found on review, and it is worth stating
plainly because the code *claimed* the opposite.

The spec says no file in the sibling checkout is written, moved or removed, and
`feed.rs` repeated the claim in its own doc comment. It was false.
`dotnet pack -o <dir>` redirects only the final `.nupkg`; it also wrote restore
assets into `<project>/obj/` and build output into `<project>/bin/`, inside the
checkout. Those paths are gitignored, so `git status` stayed clean and the
mutation was invisible to every check that looks at version control.

Proved by running it: the original invocation rewrote **6 file entries** under
`dotnet-platform-libs/src/*/obj/`.

The fix packs from a **scratch copy** of the sibling — `src/` without any
build-output directory, plus every regular file at the root — so the requirement
is true by construction. Redirecting MSBuild's output roots was implemented
first and **rejected on evidence**: it does not work, because the default
`**/*.cs` glob still reads the sibling's own `obj/`, and on a sibling carrying a
stale `net8.0` output the build compiles two copies of the same generated
assembly attributes and fails.

After the fix, a full pack of all nine packages leaves the sibling
**byte-identical across all 10,051 files** under `src/`.

## Defects found and fixed on review

The package's 25 tasks were all ticked before this review. Four of them were
ticked against something that did not hold up:

1. **The explicit kit upgrade had no operator entry point.** `diff_kit_snapshot`
   and `upgrade_kit_snapshot` existed and were tested, but `forge kit`
   implemented only `pack` and `verify`. The spec scenario is "*when an
   operator runs* the explicit kit upgrade"; its only caller was `#[cfg(test)]`.
   A capability nobody can invoke is not an explicit path. Now `forge kit
   upgrade <path> --to <kit@version>`, review-only by default, applied on
   `--confirm`.
2. **The upgrade did not move the declared pin.** It rewrote the owned files
   and the receipt but not `kit.version` in the project's `forge.yaml` — and
   `forge kit verify <path>` deliberately reads what the project *says* it pins.
   A successful upgrade therefore produced a project that failed its own drift
   gate. Now recorded as a targeted line edit, never a YAML round-trip, reusing
   the generator's own `yaml_scalar` so the escaping cannot drift.
3. **The source-mode refusal was dead code.** `refuse_source_reference` existed
   and `FeedRejection::SourceReference` was never constructed outside a test;
   `validate_feed_value` could not produce it. "Forge SHALL refuse a source-mode
   project reference" was unimplemented while looking implemented. Now
   `find_source_reference` scans every rendered manifest before anything is
   staged. A reference that stays *inside* the generated project is deliberately
   allowed — a solution with its own test project is portable.
4. **Two tests asserted nothing.** The pattern-catalog colour check searched
   for the literal string `#[0-9a-fA-F]{6}`, which no hex colour contains. The
   completeness check asserted `matches!(class, Confirmed | Provisional)` against
   a two-variant enum, so "every package is classified" could never fail. Both
   now parse and compare for real.

A fifth, smaller one: the feed's byte-level digest check was gated on
`feed_dir.starts_with(kits_dir())` with no reporting, so a generated project's
own committed feed was version-checked and the byte check was **silently
skipped** — a green result that never ran the check. `FeedVerificationReport`
now carries `digests_verified`, the digest record is a parameter
(`verify_committed_feed_with_digests`), and the CLI prints a note when the check
did not run.

## Pre-existing conditions (not regressions)

- `cargo fmt --all -- --check` used to fail on five files this change does not
  own. **This is now stale and was re-checked on 2026-10-05: the tree is
  rustfmt-clean on `main` at `b8b0fb5`, so that earlier failure has been fixed
  by intervening work rather than by this change.** The claim below is kept as
  written history: at the time, the parent-commit versions of
  `src/gate/evidence.rs`, `src/github/normalize.rs`,
  `src/portfolio/share/validation.rs` and `src/publish/fleet.rs` were already
  rustfmt-dirty, so that failure was not drift from that change.
- `scripts/release-check.sh` blocks at its first gate, `cargo fmt --check`, for
  the reason above. It never reaches its test, clippy, audit or readiness
  stages. `cargo-deny` and `cargo-audit` are both installed.
- `generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`
  **hangs indefinitely** in this environment, at 0% CPU, blocked on the cargo
  package-cache lock taken by the outer `cargo test`. **Verified pre-existing**:
  it hangs identically with this change stashed, in isolation, and was killed by
  a 240s timeout both times. It is the only test excluded from the run above.

## Verification gaps left open

- `packing_leaves_the_sibling_checkout_byte_identical` is `#[ignore]`d, and the
  reason is measured rather than guessed. A real pack is ~15 seconds of heavy
  parallel CPU; in a whole-suite run that load pushed
  `governance_contract::valid_external_response_is_normalized_and_redacted` past
  its 5-second external-adapter timeout (`left: Unavailable, right: Pass`) and
  turned an unrelated green suite red. The control run — same tree, that test
  skipped — is 2291 passed / 0 failed, and the pack test passes on its own, so
  the assertion is sound and the interference is real. Run it explicitly:

  ```sh
  cargo test --test kit_contract packing_leaves_the_sibling_checkout_byte_identical -- --ignored
  ```

  Last explicit run: **passed**.
- The floor-refusal scenario ("WHEN a floor refusal occurs … the destination
  directory, the staging area and the project registry are byte-identical") is
  proven for a real pre-staging refusal through the CLI, but **no shipped
  profile has an unmet floor**, so that exact trigger is unreachable from the
  command line. The typed refusal itself is covered. Closing it properly needs a
  way to induce an unmet floor end to end, which is a product decision.
- `tests/workspace_metadata_contract.rs::opt_out_is_byte_identical_to_pre_release`
  pins the exact bytes `--no-workspace-metadata` produces, per profile. All six
  digests were re-captured because this change alters what a scaffold contains
  by design. The guard's intent is unchanged and it still fails on future drift.
- `forge new`'s `notes` field was already asserted empty for a fully mapped
  profile. A declared-zero warning is not an omission note, so it moved to a new
  additive `kit_warning` field rather than overloading `notes`.
- `tests/generate_contract.rs::dotnet_scaffold_builds_offline_without_forge`
  asserted that an `aspnet-web` scaffold builds with **no** `PackageReference` at
  all. That was true before and is false now, by design. The test is left intact
  and still asserts a successful `dotnet build`; it now needs a resolvable feed,
  which the default test environment does not provide. The equivalent assertion
  added by this change, `the_generated_dotnet_project_operates_without_forge`,
  reports `unverified` rather than a pass when no feed is configured.

## Why a branch appeared

The work was committed on `feat/scaffold-prewires-shared-layer` rather than
straight onto `main`, contrary to the owner's standing direction.

**AGENTS.md has never contained any branch instruction.** Its full history is
three commits (`5a2d272`, `d069d59`, `290832f`), and neither AGENTS.md nor
`.ai-rules/` nor README.md mentions "branch" or "main" anywhere — a
repository-wide search of those files returns nothing. So the rule the owner
believes was written down is not in the repository, and the previous agent was
not working from a written instruction to override.

The most likely driver is the agent harness rather than the repository: the
default commit guidance in this environment is "if on the default branch, branch
first", which fires even when a project says nothing. The reflog shows the
branch created at `2420e2c` and the single commit landing on it, with `main`
left at `2420e2c`.

Corrective action taken: `main` was fast-forwarded to the commit, the branch
deleted, and all subsequent work committed on `main`. Nothing was lost — the
branch tip and `main` are the same commit.

**To make the owner's rule durable, it needs to be written down.** It is not
currently in any file this repository reads. The one-line addition belongs in
`AGENTS.md` under "Required workflow", something like "Commit to `main`
directly; do not create topic branches unless the owner asks for one."

## Native evidence (real, not claimed)

| Profile | Result |
|---|---|
| `aspnet-web` | `dotnet restore` + `dotnet build` on `net10.0` succeeded, 0 warnings / 0 errors, at a path the project was not generated at, with `NUGET_PLATFORM_FEED` unset and no sibling present — the feed bytes are committed in the project |
| `react-web` | `npm run build` and `npm test` offline succeeded; vendored `node .platform/tokens/verify-tokens.mjs` passed |
| `nextjs-web` | `npm run build` and `npm test` offline succeeded |
| `rust-web` | `cargo build --offline` succeeded |
| `flutter-app` | `flutter analyze` — "No issues found!"; `flutter test` — all tests passed |
| `python-service` | rendered; not built |

The portability proof, which is the oracle for the owner's requirement: the
global NuGet package cache was emptied of all 59 `Platform.*` entries first, a
fresh `aspnet-web` scaffold was rendered, copied to an unrelated path, and
restored and built there with the feed variable removed and no sibling library
present. All nine `Platform.*` libraries resolved at `0.1.0` into the emptied
cache, and `obj/project.assets.json` lists exactly 9. The control experiment —
the same project with its committed `packages/` deleted — fails with
`error NU1301`, naming the relative feed as the missing local source. So the
committed bytes are what supply the packages: not the cache, not a sibling, not
an environment variable.
