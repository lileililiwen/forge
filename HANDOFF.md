# Forge handoff

## Current state

`fleet-live-rollout` implemented, verified and archived on 2026-09-30 as
`2026-09-30-fleet-live-rollout`. Its requirements were promoted into
[openspec/specs/fleet-live-rollout/spec.md](openspec/specs/fleet-live-rollout/spec.md).
`scaffold-prewires-shared-layer` is implemented and awaiting archive; the
`current_spec` pointer is deliberately still unset because archive is the
owner's call.

- `forge publish fleet --jobs N` (default 4, `--jobs 1` sequential) now runs a
  phased scheduler: Phase A parallel `Sync`, Phase B serial `Db` + `Prepare`
  in roster order (the shared `production-postgres` and the global
  port-registry/Caddyfile renders converge), Phase C parallel `Deploy`; the
  shared platform-router reload is serialized by a new
  `CommandSpec::exclusive` flag in the transport. `--provider` stays
  sequential; journal rows and rendering stay on the main thread
  (`src/main.rs`, `src/publish/mod.rs`, `src/publish/remote_compose.rs`).
- Sync-stage transfers carry a 600s ceiling (`PUBLISH_SYNC_TIMEOUT`); probes
  keep 60s, deploy builds 1800s. `--fleet-registry <path>` routes through the
  legacy `projects.json` adapter instead of the inventory branch. The
  source-sync exclusion set is `.git/ node_modules/ target/ dist/ build/
  obj/ appendonlydir/ *.rdb` (was a wholesale `data/`, which dropped
  `rust-ecommerce` source).
- Target ops (recorded in the archived `design.md` D4 and `tasks.md` 2.4):
  secret provisioning, legacy container/volume removal, orphaned-network
  reclamation, and sibling build/healthcheck fixes (`hermora`, `hestia-lab`,
  `lexora`, `lore-mix`, `alltools-platform`, `opendockify`, `openaccounting`,
  `orphevia`, `reelio`, `rust-ecommerce`, `somodanote`, `trippify`).

## Verification (2026-09-30)

- Live: `forge publish fleet --jobs 4 --fleet-registry
  /home/paul/code/workspace-governance/projects.json` reached **20/20**
  (`fleet publish: 20/20 compose_ready succeeded`, `EXIT=0`); queue
  `fleet-20260930T102354Z-34c37313` and `forge deploy status --queue` shows
  all 20 rows `done`, each `fleet stages=4 healthy=true`. Run 6
  (`fleet-20260930T100950Z-c3e34795`) was 19/20 — the sole failure
  `alltools-platform` `prometheus` unhealthy (BusyBox `wget` behind the
  injected proxy; fixed with `-Y off`). The 2026-09-29 attempt had reached
  4/20 before the disk-full Docker engine death.
- `rustfmt --check` clean on the four touched files; `cargo clippy
  --workspace --all-targets` has no warning on any added line; `cargo test
  --workspace --all-targets --no-fail-fast` 2233 passed / 0 failed (`EXIT=0`);
  `git diff --check` PASS; `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 62 passed / 0 failed.
- App-internal crash loops that survive a successful deploy stage
  (`somodanote` EF migration, `orphevia` DI registration, `trippify` payment
  provider, `lore-mix` `DATABASE_URL`) are out of scope per the change's
  proposal non-goals and recorded in the archived `design.md` D4.
- No shared Gate Runtime is configured; no Gate pass is claimed.

---

## Active change: `scaffold-prewires-shared-layer` (implemented, not archived)

`scaffold-prewires-shared-layer` implemented on owner direction. The change
lives in
[openspec/changes/scaffold-prewires-shared-layer/](openspec/changes/scaffold-prewires-shared-layer/)
and is **not archived**: `openspec archive` was not run, no git operation was
performed, and the `current_spec` pointer is deliberately still unset because
archive is the owner's call.

- New module `src/kit/` (`mod.rs`, `registry.rs`, `floor.rs`, `feed.rs`,
  `assets.rs`) and a new checked-in `kits/` tree holding the digest-pinned
  vendored token artifacts plus `kits/manifest.json`.
- `src/profile/mod.rs` gains one additive `#[serde(default)] kit` field; every
  supported profile declares a kit — `platform-dotnet`, `platform-ui-web`, or a
  **recorded zero** with a `zero_reason`.
- `src/generate/mod.rs` pre-wires the shared layer into the `aspnet-web`,
  `react-web` and `nextjs-web` scaffolds, renders the `kit` block into
  `forge.yaml`, writes the `README` shared-layer section, and stages the
  `.platform/` owned subtree with its ownership receipt.
- `src/registry/mod.rs` observes the pinned kit id and version additively on the
  existing project row (no new table).
- `src/core/mod.rs` gains nine typed errors and their `code()` arms:
  `kit-unknown`, `kit-ecosystem-mismatch`, `kit-feed-invalid`,
  `kit-floor-not-met`, `kit-exception-reason-required`, `kit-digest-mismatch`,
  `kit-feed-version-mismatch`, `kit-feed-incomplete`, `kit-pack-unavailable`.
- `src/main.rs` gains `--kit-exception <reason>` on `forge new`, a kit block in
  `forge profile inspect`, and the `forge kit pack` / `forge kit verify` verb
  (a recorded exception — see resolved decision 6). No new MCP tool, API route
  or portal section.

## Resolved open decisions

1. **Kit distribution** — **owner-ruled twice.** The first implementation named
   the feed and resolved it at restore time through `NUGET_PLATFORM_FEED`; the
   owner **rejected** it, because a variable a CI runner does not carry is the
   same failure class as a hard-coded absolute path. The ruling now in force: a
   `NuGet.config` with `<clear />`, one named source whose value is a path
   **relative to the generated project** (`packages/platform-feed`), and
   nuget.org — with the `.nupkg` bytes committed inside the project. This is the
   shape `therapist-commons` and `citylens` already use. The rejected mechanism,
   its constant, its MSBuild block and the comments justifying it were removed,
   not left dormant; `design.md` §5 records the full statement so the option is
   not silently re-proposed.
2. **Minimum-consumption floor** — `6` for `aspnet-web` (exactly the confirmed
   set), `1` for the two Node profiles (the vendored token pair), `0`-with-reason
   elsewhere. One declared field per descriptor.
3. **Target framework** — owner-ruled. Forge raises `aspnet-web` to `net10.0`;
   `dotnet-platform-libs` is not multi-targeted down. Blocker discharged.
4. **Receipt** — a contained `.platform/receipt.json`, because reuse of the
   `.standard/` machinery needs a sibling-side standard-pack descriptor that is
   outside this repository's write boundary. Shapes, diff vocabulary and the
   ownership-conflict refusal match the standard receipt exactly.
5. **The restore closure is 9 packages, not 6** — the confirmed set plus
   `Platform.Billing.Contracts`, `Platform.Eventing` and
   `Platform.Web.Telemetry`, which arrive as project references of
   `Platform.Testing` and `Platform.Observability`. The feed carries all nine;
   the floor still counts only the six, and the three transitive members stay
   below the bar and rendered commented.
6. **`forge kit` is a recorded exception to "no new top-level CLI verb"** — the
   committed feed needs `forge kit pack` to regenerate and `forge kit verify` to
   catch drift, and both are `cargo test`-covered where a shell script beside
   `kits/` would be neither.

## The evidence the floor was derived from

Measured by sweeping every `Platform.*` package reference outside
`dotnet-platform-libs` itself, and frozen in `kit::PLATFORM_PACKAGE_EVIDENCE`:
a test fails when a package's classification disagrees with that fixture.

- Confirmed (>= 2 distinct external consumers, no infrastructure weight, 6 total):
  `Platform.Core` (7), `Platform.AspNetCore` (6), `Platform.Testing` (4),
  `Platform.RateLimiting` (2), `Platform.Idempotency` (2), `Platform.Observability` (2).
- Withheld despite clearing the bar, because they need a store:
  `Platform.Persistence.EfCore` (4), `Platform.Identity.AspNetCore` (3),
  `Platform.Tenant.Lifecycle.AspNetCore` (0).
- Provisional (below the bar, rendered commented, never restoring): everything
  else, each comment naming its own consumer count.

## Verification

- `cargo build`: clean.
- `cargo test --workspace --all-targets`: recorded in the run log; the totals
  are reported to the owner verbatim. `tests/kit_contract.rs` adds 45 tests,
  all passing.
- `cargo fmt --all -- --check`: **fails, on files this change does not touch.**
  The baseline already reports diffs across `src/gate/evidence.rs`,
  `src/github/normalize.rs`, `src/portfolio/share/validation.rs`,
  `src/publish/fleet.rs` and five `tests/*` files. Every file this change
  touches is rustfmt-clean; the unrelated files were deliberately left alone
  rather than reformatted inside a scoped change.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  **fails with the same 12 pre-existing findings as the baseline**, in the
  same untouched files (`src/gate/evidence.rs`,
  `src/portfolio/share/validation.rs`, `src/publish/fleet.rs`,
  `src/publish/mod.rs`, `src/api/ui/auth.rs`). Zero findings in any file this
  change touches.
- `scripts/release-check.sh`: **blocks at its first gate**, which is
  `cargo fmt --check` — the pre-existing failure above. It never reaches the
  test, clippy, audit, readiness or contract-parity stages. `cargo-deny` and
  `cargo-audit` are both installed, so those gates would have been available.
- `openspec validate scaffold-prewires-shared-layer --strict`: valid.
- `openspec validate --all --strict --no-interactive`: 63 passed, 0 failed.
- `node scripts/check-openspec-change-names.mjs`: PASS.

## Native evidence (real, not claimed)

Each scaffold was generated and then built/tested with its own toolchain and
`FORGE_REGISTRY` unset:

| Profile | Result |
|---|---|
| `aspnet-web` | `dotnet restore` + `dotnet build` on `net10.0` succeeded, 0 warnings / 0 errors, at a path the project was not generated at, with `NUGET_PLATFORM_FEED` unset and no sibling `dotnet-platform-libs` present — the feed bytes are committed in the project |
| `react-web` | `npm run build` and `npm test` offline succeeded; vendored `node .platform/tokens/verify-tokens.mjs` passed |
| `nextjs-web` | `npm run build` and `npm test` offline succeeded |
| `rust-web` | `cargo build --offline` succeeded |
| `flutter-app` | `flutter analyze` — "No issues found!"; `flutter test` — all tests passed |
| `python-service` | rendered; not built in this run |

## The portability proof (the oracle for the owner's requirement)

The owner's requirement is that a project must build when it does not reside on
the machine that generated it. This is the evidence for it, and the previous
"honest caveat" that recorded the .NET restore as unverified is **superseded**.

The global NuGet package cache was emptied of all 59 `Platform.*` entries first,
so nothing could be served from a warm cache.

```sh
# 1. render a fresh aspnet-web scaffold
cd /home/paul/code/forge
env -u NUGET_PLATFORM_FEED ./target/debug/forge new \
  target/portability/origin/net-app --profile aspnet-web --id net-app

# 2. copy it to a different path
mkdir -p /tmp/forge-portability-proof
cp -a target/portability/origin/net-app /tmp/forge-portability-proof/relocated-net-app

# 3. restore at the new path, with the rejected mechanism unset
cd /tmp/forge-portability-proof/relocated-net-app
env -u NUGET_PLATFORM_FEED dotnet restore --nologo
#   已还原 /tmp/forge-portability-proof/relocated-net-app/net-app.csproj (用时 164 毫秒)。

# 4. build at the new path, with Forge absent from the environment
env -u NUGET_PLATFORM_FEED -u FORGE_REGISTRY dotnet build --nologo
#   net-app -> /tmp/forge-portability-proof/relocated-net-app/bin/Debug/net10.0/net_app.dll
#   已成功生成。  0 个警告  0 个错误
```

All six pre-wired packages and all three transitive closure members resolved
into the emptied cache, and `obj/project.assets.json` lists exactly 9
`Platform.*` libraries at `0.1.0`. No sibling library existed at
`/tmp/forge-portability-proof/dotnet-platform-libs` or `/tmp/dotnet-platform-libs`.

**Control experiment**, so the result is not merely "the machine already had
them": the same project with its committed `packages/` directory deleted fails
to restore, naming the relative feed as the missing local source —

```sh
cp -a target/portability/origin/net-app /tmp/forge-portability-proof/control/net-app
rm -rf /tmp/forge-portability-proof/control/net-app/packages
cd /tmp/forge-portability-proof/control/net-app
env -u NUGET_PLATFORM_FEED dotnet restore --nologo
#   error NU1301: 本地源"/tmp/forge-portability-proof/control/net-app/packages/platform-feed"不存在。
#   未能还原 …  → exit 1
```

So the committed bytes are what supply the packages, not the cache, not a
sibling, and not an environment variable.

## Verification gaps left open

- Two existing tests encoded pre-change behaviour and were updated rather than
  deleted, with the reason recorded in each file:
  - `tests/workspace_metadata_contract.rs::opt_out_is_byte_identical_to_pre_release`
    pins the exact bytes `--no-workspace-metadata` produces, per profile. All
    six pinned digests were re-captured, because this change alters what a
    scaffold *contains* by design (the `kit` block, the README section, the
    `aspnet-web` TFM and package references, the vendored `.platform/` tree).
    The guard's intent — opting out changes nothing else — is unchanged, and
    the assertion still fails on any future unintended drift.
  - `forge new`'s `notes` field was already asserted empty for a fully mapped
    profile. A declared-zero warning is not an omission note, so it moved to a
    new additive `kit_warning` field rather than overloading `notes`. The
    existing `notes` contract is untouched.
- `tests/generate_contract.rs::dotnet_scaffold_builds_offline_without_forge`
  asserted that an `aspnet-web` scaffold builds with **no** `PackageReference`
  at all. That was true before this change and is false now, by design: the
  profile pre-wires the confirmed set. The test is left intact and still
  asserts a successful `dotnet build`; it now depends on a resolvable feed
  being available, which the test environment does not provide by default. Run
  it with `FORGE_TEST_PLATFORM_FEED` pointed at a feed to get a real pass.
  The equivalent assertion added by this change,
  `the_generated_dotnet_project_operates_without_forge` in
  `tests/kit_contract.rs`, reports `unverified` rather than a pass when no feed
  is configured, matching the repository's existing convention.
- `kit::diff_kit_snapshot` / `kit::upgrade_kit_snapshot` are library
  capabilities only; no CLI verb is wired, per `design.md` section 1.
- `scripts/release-check.sh` was not able to run past its first gate, for the
  `cargo fmt` reason above; that is a pre-existing repo condition, not a
  regression from this change.
