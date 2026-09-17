current_spec: adapter-deployment

# Forge handoff

## Current state

`release-publishing` implemented, verified and archived on 2026-09-17
as `2026-09-17-release-publishing`; canonical specs promoted to
[openspec/specs/release-publishing/spec.md](openspec/specs/release-publishing/spec.md).
New in this cycle: `src/release` (versioned `Semver`/`ReleaseConfig`/
`ReleaseIdentity`/`ReleaseRequest`/`ReleaseReport`/`ReleaseState`/
`StageOutcome` contract v0.1.0; `Semver::parse` rejects empty,
non-triple, non-numeric, leading-zero and invalid-prerelease
versions and the manifest's `release.versioning` accepts only
`semver` so an unsupported versioning scheme can never reach
the apply path; `ReleaseConfig::from_manifest_meta` refuses
duplicate, empty or unknown `release.checks` kinds, package
paths and Dockerfile paths that lexically resolve outside the
project, and an empty changelog; the release id is derived
from `<project>-<semver>-<12-hex-sha>` so a different revision
or version cannot silently reuse the previous release record;
`prepare_release` captures the working-tree revision, the
changelog content hash and a bounded excerpt, the configured
doctor/test/DriftWatch checks (each bound to the captured
revision so a stale plan cannot be applied after the tree has
moved on) and the enabled `docs.translations.<locale>` locales
(R1 boundary: a project without any configured locale omits the
docs stage entirely) into a `PlanReport` whose `ready` verdict
is true only when every applicable check passed at the captured
revision; `apply_release` re-runs the captured checks and
refuses without `--confirm` (typed `release-invalid`), marks the
plan not ready with `release-check-failed` when any applicable
check fails, is unavailable or is stale, and walks every
selected stage — `commit`, `tag`, `push`, `mirror`, `package`,
`container`, `docs`, `notes` — with stable statuses
`delivered`/`skipped`/`disabled`/`failed`/`conflict`; the tag
stage is annotated at the captured revision and refuses to
replace an existing tag that points at a different commit
(R2 boundary: `release-identity-conflict` evidence named
verbatim, recovery points to `git tag -d` and to releasing at
a different semver); a previously-delivered stage on the same
revision is reported as `skipped` so retries do not redo work
the registry has already observed; the package, container and
notes adapters are external binaries (defaults
`forge-package-publisher` / `forge-container-publisher` /
`forge-notes-renderer`, overridable via `FORGE_PACKAGE_BIN` /
`FORGE_CONTAINER_BIN` / `FORGE_NOTES_BIN`) invoked with
argument arrays and a bounded per-run timeout so an
unresponsive provider cannot hang the registry; a missing
binary, non-zero exit, timeout or empty receipt surfaces as
`failed` for that stage while the prior `ReleaseState` and
prior stage outcomes stay intact; `ReleaseState` is persisted
under `.forge/release/<project-id>/<release-id>/state.json`
via atomic write (`.tmp` + rename) so a successful run
overwrites the prior state and a failed run leaves it
untouched; the mirror stage reuses the `distribution` contract
so a project with a `distribution` block performs a real
`forge mirror` and a project without one reports the mirror
stage as `disabled`; the docs stage reuses the
`documentation-translation` contract so a project with
enabled translation locales performs a real `forge docs
translate` per locale and a project without configured locales
reports the docs stage as `disabled`; credential-shaped
evidence is redacted by `redact_release_evidence` which
delegates to `policy::redact_credentials`; CLI `forge release
prepare|apply|list|inspect` (human/JSON, the per-stage
`evidence` is rendered on stdout before the typed exit-code
error so a partial run is observable); release operations
journaled in the registry's `operations` table under the
`release` kind with a `done`/`blocked`/`partial` verdict; and
the spec contract from `repository-distribution` and
`documentation-translation` still holds after a release run on
the same project (the mirror and docs stages reuse the same
Core contracts and the registry journal remains independent
of the release surface).

`documentation-translation` implemented, verified and archived on 2026-09-17
as `2026-09-17-documentation-translation`; canonical specs promoted to
[openspec/specs/documentation-translation/spec.md](openspec/specs/documentation-translation/spec.md).
New in this cycle: `src/docs` (versioned `DocsConfig`/
`LocaleConfigEntry`/`TranslateRequest`/`TranslateReport`/
`TranslateOutcome`/`LocaleFreshness`/`FreshnessStatus`/`ReviewStatus`
contract v0.1.0; manifest `docs.source` defaults to `README.md` and
`docs.translations.<locale>.enabled` defaults to `false` so
translation is never enabled by default; locales are validated
against a strict language-tag grammar that refuses separators,
`..`, and embedded paths so a locale can never smuggle a path
into the state layout; `run_translate` plans and applies one
explicit locale or `--all` enabled locales, reuses unchanged
segments by content hash, requests only changed segments from
the provider, merges translations in source order with code
blocks reinserted verbatim, revalidates the source hash after
the provider returns (a source edit during generation is
reported as `failed` without overwriting the prior derivative),
preserves link destinations and explicit non-translatable
terms (a violation marks the derivative `needs-review` instead
of claiming translation quality from provider success), and
writes the state to `.forge/docs/<locale>/state.json`); the
provider is an external binary (default `forge-docs-translator`,
overridable via `FORGE_DOCS_TRANSLATOR_BIN`) invoked with an
argument array — never a shell — and a per-run timeout, with
`spawn` + bounded wait so an unresponsive tool cannot hang the
registry; a missing binary, non-zero exit, timeout, contract
mismatch or unparseable output surfaces as `failed` while the
prior derivative and state stay intact; CLI `forge docs
translate [LOCALE] [--all] [--project PATH]` (human/JSON, the
typed `error[docs-invalid]` / `error[translation-failed]`
errors render on stderr and the per-locale outcome JSON
prints to stdout on partial failure so a partial run is
observable); a derivative path that resolves to the source
file or outside the project is refused before any write, a
disabled locale is refused on explicit request and skipped on
`--all` without invoking the provider, and a credential-shaped
substring in evidence is redacted by `redact_docs_evidence`
which delegates to `policy::redact_credentials`; doctor
exposes `docs-<locale>` and `docs-freshness` findings with
`pass`/`warn`/`fail` derived from `assess_freshness` (read-
only: stale when the recorded source hash no longer matches,
`never-translated` when the derivative is absent, `needs-review`
when the recorded review state names violations, and `misconfigured`
when the source or derivative path is broken), so the existing
doctor contract still holds after a successful run; and the spec
contract from `repository-distribution` still holds after a
docs-translate run on the same project (the registry journal
remains independent of the docs surface).
as `2026-09-17-repository-distribution`; canonical specs promoted to
[openspec/specs/repository-distribution/spec.md](openspec/specs/repository-distribution/spec.md).
New in this cycle: `src/distribution` (versioned
`DistributionConfig`/`MirrorConfigEntry`/`MirrorProvider`
`github`+`gitee` supported, `gitlab`+`codeberg` planned/
`MirrorSupportStatus`/contract v0.1.0 over JSON-RPC 2.0 stdio;
`DistributionConfig::from_manifest_meta` validates the
manifest's `distribution` block, refuses duplicate enabled
mirrors and unknown providers, and defaults the first
declared mirror to the remote name `mirror-<provider>`;
`plan_mirror` and `apply_mirror` drive `forge mirror` end
to end, with per-remote `MirrorRemoteOutcome` carrying role
`primary` or `mirror` and stable statuses `delivered`/
`skipped`/`disabled`/`diverged`/`unavailable`/`failed`;
`MirrorState` persisted under
`.forge/distribution/<project-id>/state.json` so a
`--retry-failed` re-push only fires when the local HEAD SHA
differs from the previously delivered SHA (already-delivered
refs at the same SHA are reported as `skipped`); diverging
mirror history surfaces as `diverged` with a recovery
guidance and refuses `--force`; a disabled mirror is reported
as `disabled` without contacting the remote (R1 boundary);
a partial primary+mirror run records each remote independently
so the primary's `delivered` is never misreported when a
mirror fails; credential-shaped evidence is redacted by
`redact_distribution_evidence` which delegates to
`policy::redact_credentials` (the same redaction the policy
adapter consumes); Core errors `distribution-invalid`/
`mirror-disabled`/`mirror-diverged`/`mirror-credentials` with
stable codes; CLI `forge mirror [TARGET] --ref REF --confirm
[--dry-run] [--retry-failed]` (human/JSON, the per-remote
evidence is printed to stdout before the typed exit-code
error so partial runs are observable); MCP `mirror_project`
tool classified as `external_write` and dispatched through
the same Core contracts the CLI uses; mirror operations
journaled in the registry's `operations` table under the
`mirror` kind with a `done`/`partial` verdict; and the spec
contract from `agent-runtime-workflows` still holds after a
mirror run on the same project (the push `confirm`-required
guard and the registry journal remain independent of the
distribution surface).
as `2026-09-17-mature-mcp-surface`; canonical specs promoted to
[openspec/specs/mature-mcp-surface/spec.md](openspec/specs/mature-mcp-surface/spec.md).
New in this cycle: `src/mcp` (versioned `McpToolDescriptor`/
`McpToolKind` (`ReadOnly` / `Mutating` / `ExternalWrite`) /
`McpRequest` / `McpResponse` / `McpRpcError` contract v0.1.0 over
JSON-RPC 2.0 stdio; tool registry of sixteen mature operations
(`list_projects`/`inspect_project`/`list_profiles`/
`inspect_profile`/`list_features`/`run_doctor` read-only,
`create_project`/`import_project`/`add_feature`/`remove_feature`/
`upgrade_feature`/`generate_spec`/`run_agent`/`run_tests`/
`commit` mutating, `push` external-write) with each tool's
declared JSON Schema and stable contract version; one
`tools/list` RPC and sixteen tool RPCs, each dispatched through
the same Core contracts the CLI uses, with the JSON-RPC
response carrying the full `data` envelope and the diagnostic
stream carrying a redaction-safe summary keyed on the tool
name so a credential embedded in a failed Core call does not
leak through stderr; `create_project` rejects shell
metacharacters in the project id before any file is created
or registry row is written (treated as literal data, never
executed as shell code); `push` is refused with the
`push-confirm-required` data code when `confirm` is missing
or `false`, so an implicit remote write is never accepted;
`deploy` / `publish` / `release` / `mirror` / `docs` are
intentionally absent from the registry because their Core
operations are not implemented yet (R1 + R2 boundary
scenarios); Core errors `mcp-invalid` / `mcp-unauthorized`
with stable codes; CLI `forge mcp serve` (human, the
JSON-RPC response stream is the output; `tools/list` and the
sixteen tool RPCs are the input/output contract); MCP
operations journaled in the registry's `operations` table
under the `mcp` kind; and the spec contract from
`agent-runtime-workflows` still holds after the new
transport serves an inspect request and the CLI surface
returns the equivalent domain record).

`agent-runtime-workflows` implemented, verified and archived on 2026-09-17
as `2026-09-17-agent-runtime-workflows`; canonical specs promoted to
[openspec/specs/agent-runtime-workflows/spec.md](openspec/specs/agent-runtime-workflows/spec.md).
New in this cycle: `src/agent` (versioned `AgentSession`/`AgentProvider`/
`SessionState`/`SessionTransition`/`TransitionRecord` contract v0.1.0
covering start/pause/takeover/resume/restart/new-session; bundled
OpenCode/Codex adapters return explicit `unsupported` for
pause/takeover because the existing PTY manager is not wired into
this build, with the recovery note naming the integration point;
session storage under `.forge/agents/<id>/` carrying
`session.json` and `transitions.log`; `run_spec` refuses with
`error[spec-invalid]` when the bound spec is missing, leaving
the session file preserved so the boundary scenario is
observable) and `src/gitops` (versioned `TestOutcome`/
`CommitOutcome`/`PushOutcome` contract v0.1.0; profile-aware
`test` runs the descriptor's `cargo test`/`npm test`/etc. and
surfaces a non-zero exit as `error[test-failed]` with the
captured stdout/stderr; `commit --path X --message M` stages
only the listed paths via `git add --` and refuses when the
working tree has tracked edits outside the requested paths,
preserving untracked files; `push --remote X --ref-name Y
--confirm` requires explicit confirmation and refuses
implicit remote writes with `error[push-confirm-required]`),
Core errors `agent-unavailable`/`agent-unsupported`/
`test-failed`/`git-dirty`/`push-confirm-required` with stable
codes, CLI `forge agent start|pause|takeover|resume|restart|
new-session|status|list|run-spec` and `forge test|commit|push`
(human/JSON), operations journaled in the registry's
`operations` table with the `agent` kind, and the spec
contract from `specification-remediation` still holds after a
session that ends without verification preserves its
`session.json` with the original spec id.

`specification-remediation` implemented, verified and archived on 2026-09-17
as `2026-09-17-specification-remediation`; canonical specs promoted to
[openspec/specs/specification-remediation/spec.md](openspec/specs/specification-remediation/spec.md).
New in this cycle: `src/spec` (versioned `SpecRequest`/`SpecDraft`/
New in this cycle: `src/spec` (versioned `SpecRequest`/`SpecDraft`/
`SpecProvenance`/`SpecGenerateOutcome`/`SpecStatus` contract v0.1.0 with
provenance covering project id, path, profile, source revision (manifest
mtime), finding ids/categories, DriftWatch policy ids and dependencies;
storage layout `.forge/specs/<project>-<short-hash>/` carrying
`proposal.md` / `design.md` / `tasks.md` / `manifest.json`; bounded
proposal size enforced at `MAX_FINDINGS_PER_SPEC = 32`; `generate_spec`
is idempotent on the same project + sorted finding set so an unchanged
finding re-run reports the existing spec without rewriting files
(boundary scenario); `SpecRoute` distinguishes `Deterministic`,
`Semantic` and `Manual` queues and `apply_routing` requires the
deterministic action's evidence (or generated spec, or recorded manual
state) before reporting completion; `route_finding` keeps the doctor
finding's `Manual` remediation class as a judgment call (no project
changes, no AI claim) and routes any finding whose id matches a
known lifecycle action to `Deterministic`; Core errors
`spec-invalid`/`spec-write-failed` with stable codes, CLI
`forge spec generate|list|inspect|route|apply` (human/JSON, structured
`error[spec-invalid]` for empty/oversized/ambiguous requests and the
existing `feature-ownership-conflict` handoff from `project-upgrade-orchestration`
now resolves through `forge spec apply semantic-<feature>` writing the
bounded proposal without changing files), the `SpecRequest` is
validated before any write so a refusal leaves the project untouched,
and existing doctor/upgrade/feature/import/generate/quality-policy
contracts still hold after the new commands run against a generated
project.

`quality-policy-integration` implemented, verified and archived on 2026-09-17
as `2026-09-17-quality-policy-integration`; canonical specs promoted to
[openspec/specs/quality-policy-integration/spec.md](openspec/specs/quality-policy-integration/spec.md).
New in this cycle: `src/policy` (versioned `DriftWatchConfig`/default
binary `driftwatch` overridable via `FORGE_DRIFTWATCH_BIN`, bounded
`Command::new` + argument-array invocation with per-run `wait_timeout`
so an unresponsive tool cannot hang the registry; `PolicyReport`/`PolicyFinding`/`PolicySeverity`
contract v0.1.0 with custom deserialization that accepts `info`/`ok` as
aliases for `pass`; project-scoped execution with `current_dir(dir)` and
`--project <dir>`; `PolicyOutcome::Reported` / `Unavailable` so a
missing binary, non-zero exit, timeout or unparseable JSON surfaces as
an `unavailable` finding instead of `pass`; `redact_credentials` and
`redact_report_in_place` covering AWS / GitHub / GitLab / Slack / JWT /
private-key / `key=value` shapes and run on every consumed report as
defense in depth; `observation_is_stale` keyed on `forge.yaml` and
configured driftwatch-file mtimes), Core `PolicyUnavailable`
(`policy-unavailable`), CLI `forge doctor` invokes the adapter and
threads the outcome through `run_doctor(..., Some(&outcome))` with
per-rule `driftwatch-<id>` findings, `driftwatch-policy` rollup and
not-applicable policies preserved with their reason and
`applicable: false`, and existing doctor contract still holds (no
registry/observation/dependency-drift regression on the unchanged
fixtures).

`extended-profile-catalog` implemented, verified and archived on 2026-09-17
as `2026-09-17-extended-profile-catalog`; canonical specs promoted to
[openspec/specs/extended-profile-catalog/spec.md](openspec/specs/extended-profile-catalog/spec.md).
New in this cycle: `ProfileSupportStatus` (`Supported` / `Planned`) on
`ProfileDescriptor`, the sixth supported profile `react-web`
(`adapter-react`, typescript, `npm@20` 0.1.0) with a tested native-buildable
template (`forge new --profile react-web` renders
`forge.yaml`/`README.md`/`index.html`/`src/main.js`/`src/app.test.mjs`/
`package.json`/`scripts/build.mjs`/`vite.config.js`/`Dockerfile`/
`.gitignore`; build `npm run build`, test `npm test`), and seven reserved
specialist candidates (`aspnet-saas`, `flutter-client`, `nextjs-content`,
`python-ai`, `python-data`, `rust-cli`, `rust-worker`) discoverable through
`inspect_profile` with `support_status: planned` but refused by
`resolve_profile` / `preflight_profile` / `generate` with a new
`ForgeError::UnsupportedProfile` (code `unsupported-profile`) before any
file change; the original five MVP ids and their semantics stay intact,
react-web refuses server-side capabilities with the existing backend
boundary hint, `flutter-client` description names the backend boundary
('rust-web'/'python-service') it needs, and CLI `forge profile inspect`
shows the support status plus description in human and JSON output.

`project-upgrade-orchestration` implemented, verified and archived on 2026-09-17
as `2026-09-17-project-upgrade-orchestration`; canonical specs promoted to
[openspec/specs/project-upgrade-orchestration/spec.md](openspec/specs/project-upgrade-orchestration/spec.md).
New in this cycle: `src/upgrade` (pinned
[`UpgradePlan`](src/upgrade/mod.rs) with old/new versions, kind-ordered
steps, asset list, validators, migration strategy and recovery
implications; `apply_upgrade` precondition sweep journals a `blocked`
`upgrade` row and emits a structured `SemanticConflict` naming the
owned file and the suggested `forge spec generate` follow-up;
already-satisfied upgrades are a no-op with no file, manifest or
registry write; missing requested features install at the tested
version; `run_fleet` snapshots the explicit registry selection,
journals each project with `done`/`failed`/`blocked`/`skipped` states,
isolates per-project failures and reports a healthy verdict only when
nothing failed or blocked; retry re-plans from the current manifest so
completed steps are not blindly repeated; `postgres` steps are marked
irreversible with declared strategy
`manifest-repin+manual-schema-review`), Core `record_operation`
append-only journal, CLI `forge upgrade [TARGET] [--feature FEATURE]
[--all] [--dry-run]` (human/JSON, structured `error[unknown-feature]`
and `error[feature-ownership-conflict]`, fleet owns its exit code
without the generic error path), and existing feature-lifecycle
contracts still hold after fleet upgrades (admin depends on auth, both
reach 0.1.0 in dependency order with all manifest sections preserved).

`feature-lifecycle` implemented, verified and archived on 2026-09-16
as `2026-09-16-feature-lifecycle`; canonical specs promoted to
[openspec/specs/feature-lifecycle/spec.md](openspec/specs/feature-lifecycle/spec.md).
New in this cycle: `src/feature` (18-descriptor versioned catalog at
tested `0.1.0` with compatibility derived from the MVP profile
descriptors, dependencies, conflicts, install/upgrade strategies,
validation policies, docs and tests; dependency-ordered deterministic
plans with exact versions; add/remove/upgrade through manifest-only
edits preserving all other sections plus deterministic `.forge/features`
ownership receipts; reverse-dependency and user-edit preflight blocks
with full preservation; atomic writes with restore-on-validation-failure;
registry refresh so source/manifest/registry agree), Core errors
`unknown-feature`/`incompatible-feature`/`feature-ownership-conflict`,
CLI `forge feature list|inspect|resolve|add|remove|upgrade`
(human/JSON, stable `error[code]` diagnostics), and `forge new`
dependency-closure selection (`--feature admin` records `auth`+`admin`)
while preserving the existing `incompatible-profile` contract.

`doctor-maturity-assessment` implemented, verified and archived on 2026-09-16
as `2026-09-16-doctor-maturity-assessment`; canonical specs promoted to
[openspec/specs/doctor-maturity-assessment/spec.md](openspec/specs/doctor-maturity-assessment/spec.md).
New in this cycle: `src/doctor` (read-only finding inventory with stable
rule IDs, PASS/WARN/FAIL/UNAVAILABLE statuses, evidence, applicability and
automatic/AI/manual remediation classes; versioned L0-L4 maturity policy
descriptors with target-gated applicability so L0 prototypes are respected;
registry-observation staleness via manifest-mtime comparison; unknown profile
reported as unavailable findings, never a hard refusal), Core
`--target L0..L4` parsing (`parse_target_level`), CLI `forge doctor [<path>]
[--target]` (human/JSON, exit 0 with `healthy:false` on FAIL/UNAVAILABLE/
unmet/stale; hard errors only for path/manifest IO via existing codes), and
`healthy` defined as no FAIL/UNAVAILABLE, no unmet applicable controls and
no stale observation.

`core-manifest-registry` implemented, verified and archived on 2026-09-16
as `2026-09-16-core-manifest-registry`; canonical specs promoted to
[openspec/specs/core-manifest-registry/spec.md](openspec/specs/core-manifest-registry/spec.md).
Rust workspace `forge` 0.1.0: `src/core` (schema-v1 `forge.yaml`
validation, legacy `platform.yaml` only via explicit `--manifest`),
`src/registry` (SQLite via bundled rusqlite, unique id/canonical-path,
nullable observations, pending→done/failed operation journal with
open-time reconciliation), `src/main.rs` (thin CLI: `list`, `inspect`,
`register`, human/JSON output, stable `error[code]` diagnostics, exits
0/1/2). Foundation decisions recorded in
[ADR 0001](docs/adr/0001-foundation-toolchain.md); build commands recorded
in [README.md](README.md).

`profile-registry` implemented, verified and archived on 2026-09-16 as
`2026-09-16-profile-registry`; canonical specs promoted to
[openspec/specs/profile-registry/spec.md](openspec/specs/profile-registry/spec.md).
New in this cycle: `src/profile` (five versioned MVP descriptors with
capabilities, packages, layout, conventions, build/test commands,
deployment defaults and quality policies; `list`/`inspect`/`resolve`/`preflight`;
descriptor validation naming the missing field), Core errors
`unknown-profile`/`invalid-profile`/`incompatible-profile`/`toolchain-missing`,
CLI `forge profile list|inspect|resolve|preflight` (human/JSON, stable
`error[code]` diagnostics), and `register` gating on profile resolution
plus feature compatibility before any row mutation.

`project-import` implemented, verified and archived on 2026-09-16 as
`2026-09-16-project-import`; canonical specs promoted to
[openspec/specs/project-import/spec.md](openspec/specs/project-import/spec.md).
New in this cycle: `src/import` (read-only detection of language,
framework, package manager, database, Docker, CI, auth, features,
DriftWatch, Git remote and deployment with unknown-vs-missing evidence
and suggested profile/maturity; `ambiguous-import` on profile or
monorepo-root disagreement until `--profile` selects; minimal validated
manifest written only on `--accept` after a registry identity
pre-check, with rollback on registration failure and no legacy
doubling), Core errors `ambiguous-import`/`import-conflict`, CLI
`forge import [<path>] [--profile] [--accept] [--id]` (human/JSON,
stable `error[code]` diagnostics), and `Registry::check_identity_available`
for mutation-free collision checks.

The machine-readable line above is the single current OpenSpec pointer. It
selects the next eligible future implementation package; it does not claim
work has started.

## Next change

Implement [adapter-deployment](openspec/changes/adapter-deployment/proposal.md)
only when implementation is requested. Its prerequisite
(`release-publishing`) is implemented and verified. Then
follow the roadmap prerequisites. Later changes remain planning-only
with zero implementation tasks completed.

## Verification evidence (release-publishing, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 374 passed, 0 failed (206 lib incl. 14 new release
  unit tests for semver parsing, version rejection, config defaults
  and check/package/dockerfile validation, identity derivation
  stability, changelog presence/outside-project refusal, manifest
  maturity preservation, prepare success, prepare failure when
  doctor reports blocking findings, prepare boundary when no
  translation locale is configured, apply refusing without
  `--confirm`, apply recording tag conflict when the same semver
  is requested at a different commit, apply reporting `skipped`
  when the tag already points at the working-tree revision, apply
  persisting a release state for retry, and list reporting zero
  entries for a fresh project; 10 new release CLI contract tests
  for help listing the `prepare|apply|list|inspect` subcommands,
  prepare capturing a `ready` plan with the changelog path and
  the three check kinds, prepare refusing an unknown semver,
  prepare refusing a manifest without a `release` section, apply
  refusing without `--confirm`, apply recording per-stage
  `delivered` outcomes for commit/tag/package/notes through
  `FORGE_PACKAGE_BIN` and `FORGE_NOTES_BIN` shell fixture
  adapters, apply recording package `failed` on a second pass
  while the prior tag stays `skipped` (per-stage independence),
  apply reporting tag `conflict` when HEAD moves past a tagged
  commit, list reporting the persisted release with the
  expected project and version, and inspect returning the
  release state with the per-stage outcomes; 4 new release
  cross-surface tests for the doctor verdict staying unchanged
  across a successful release prepare, the registry
  `release` journal row carrying the per-stage summary, the
  release surface reading the new manifest verbatim after a
  `feature add`, and a prior tag reporting `conflict` after a
  new commit; plus the unchanged 5 CLI contract, 4
  cross-surface regression, 8 doctor contract, 10 feature
  contract, 12 generate contract, 10 import contract, 7
  profile contract, 7 quality_policy_contract, 12 upgrade
  contract, 12 spec contract, 8 agent contract, 8 gitops
  contract, 13 mcp contract, 9 mcp cross-surface, 4
  agent-runtime-workflows cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4 quality policy
  cross-surface, 7 quality policy contract, 12 documentation
  contract, 4 documentation cross-surface; the slow
  `cargo build+test` evidence path is exercised through the
  `rust_scaffold_builds_and_tests_with_native_toolchain`
  test that finishes in ~195s when the host toolchain is
  on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id rel-smoke` then
  appending a `release` block to the manifest and creating
  `CHANGELOG.md`; `forge release prepare rel-smoke --version 1.0.0`
  returns `ready: yes` with the captured changelog, the captured
  source revision, the three check kinds (`doctor` and `test`
  passing, `driftwatch` disabled), and a registry `release`
  journal row recorded as `done`; `forge release apply rel-smoke
  --version 1.0.0 --confirm --stage tag --stage package
  --stage notes` with `FORGE_PACKAGE_BIN` and `FORGE_NOTES_BIN`
  pointing at fixture shell scripts (`fake-pkg.sh` echoes
  `npm:rel-fixture@0.1.0 receipt-ok`, `fake-notes.sh` echoes
  `notes:rendered:…`) records the tag as `delivered`, the
  package stage as `delivered` and the notes stage as
  `delivered`, writes `.forge/release/rel-smoke/rel-smoke-1.0.0-12e19a9efefc/state.json`
  with the per-stage outcomes, and the registry records a
  second `release` journal row; `forge release list rel-smoke`
  renders the persisted release ids with their project id,
  version, stage count and last-run timestamp; `forge release
  inspect <id> rel-smoke` returns the same per-stage outcomes
  the apply produced; `forge release apply rel-smoke --version
  1.0.0 --confirm --stage tag` after a new commit is made reports
  the tag as `conflict` with both `existing:` and `requested:`
  evidence lines and the recovery notes naming `git tag -d` and
  a different semver; `forge release prepare rel-smoke-fail
  --version 1.0.0` on a project whose doctor reports blocking
  findings returns `ready: no` with the failing checks named
  in the summary; `forge release apply rel-smoke-fail
  --version 1.0.0 --confirm --stage tag` on the same project
  exits 1 with `error[release-check-failed]` and writes no
  release state.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate release-publishing --strict
  --no-interactive`: valid pre-archive; `openspec archive
  release-publishing --yes`: archived as
  `2026-09-17-release-publishing` with the canonical
  `spec/release-publishing` promoted; `openspec validate
  --all --strict --no-interactive`: 25 passed, 0 failed
  (post-archive, includes the promoted
  `spec/release-publishing`).
- `git diff --check`: PASS; staged set reviewed (4 files
  modified: `src/core/manifest.rs` for the new
  `release.*` typed fields, `src/core/mod.rs` for the
  `release-invalid` / `release-check-failed` /
  `release-identity-conflict` typed errors, `src/lib.rs` to
  register the new module, `src/main.rs` for the
  `forge release` subcommand and the `cmd_release_*`
  helpers; 2 files added: `src/release/mod.rs` with 14 unit
  tests, `src/release/engine.rs` with the prepare/apply/list
  logic and 8 unit tests; 2 integration files added:
  `tests/release_contract.rs` with 10 contract tests,
  `tests/release_cross_surface.rs` with 4 cross-surface
  regression tests; plus the promoted spec and the change
  archive — 9 files; archive under
  `openspec/changes/archive/2026-09-17-release-publishing/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real provider integration is not exercised: a real
  `forge-package-publisher` / `forge-container-publisher` /
  `forge-notes-renderer` binary is not present in the local
  sandbox, so the contract is validated through
  `FORGE_PACKAGE_BIN` / `FORGE_NOTES_BIN` fixture shell
  scripts that stand in for real provider round trips. The
  credential redaction rule set is the same as
  `policy::redact_credentials`, which is itself verified
  through the existing quality policy contract tests. A real
  package/container/notes provider round trip is a
  downstream integration step and is not claimed here.

## Verification evidence (documentation-translation, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 337 passed, 0 failed (183 lib incl. 16 new docs
  contract tests for locale validation, config defaults and
  opt-in, explicit source and paths, empty terms and bad
  locales refusal, default derivative path matching the brief
  model, segmentation and verbatim code preservation, output
  validation flagging dropped links and non-translatable
  terms, derivative-equal-to-source refusal, derivative
  outside project refusal, unknown and disabled locales
  refusal, missing binary without touching prior state, state
  round-trip preserving hashes and review, request validation
  rejecting empty and ambiguous, freshness tracking current /
  stale / never / misconfigured, and report health requiring
  all locales ok; 12 new docs CLI contract tests for help
  listing, success with hash + review + verbatim code +
  preserved link destination and state, unchanged source
  reporting `current` without a second provider request,
  incremental retranslation of only changed segments
  (provider sees one segment, three reused, code block and
  link destination preserved), derivative-equal-to-source
  refusal with `error[docs-invalid]`, derivative-outside-
  project refusal with `error[docs-invalid]`, disabled locale
  refusal + `--all` skip without provider contact, unknown
  locale refusal with `error[docs-invalid]`, provider failure
  keeping the prior derivative and state intact while the
  leaked credential-shaped secret is redacted in stdout and
  stderr, missing translator binary failing without writing,
  unparseable translator output failing cleanly, and altered
  links / non-translatable terms marking the derivative
  `needs-review` with both violation reasons named; plus the
  2 new doctor `docs-zh-CN` and `docs-freshness` contract
  tests for never-translated warn with the recovery note and
  misconfigured fail; the unchanged 5 CLI contract, 3
  cross-surface regression, 8 doctor contract, 10 feature
  contract, 12 generate contract, 10 import contract, 7
  profile contract, 7 quality_policy_contract, 12 upgrade
  contract, 12 spec contract, 8 agent contract, 8 gitops
  contract, 13 mcp contract, 9 mcp cross-surface, 4
  agent-runtime-workflows cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4 quality policy
  cross-surface, 7 quality policy contract, and 12
  distribution cross-surface; the slow `cargo build+test`
  evidence path is exercised through the unchanged fixture
  tests that finish in ~200s when the host toolchain is on
  PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id smoke-app`
  then appending a `docs` block to the manifest and creating
  `README.md`, `FORGE_DOCS_TRANSLATOR_BIN=…/fake-translator.sh
  forge docs translate zh-CN --project <proj>` on the
  registered rust-web project runs the fake `sed` translator
  end to end and writes `docs/README.zh-CN.md` carrying the
  translated text, the verbatim code block and the preserved
  link destination, plus
  `.forge/docs/zh-CN/state.json` carrying the source hash,
  review `ok`, and segments keyed by content hash; `forge
  doctor` on the same project reports `[PASS] docs-zh-CN`
  and `[PASS] docs-freshness` (the existing doctor contract
  still holds); seeding a `state.json` with a stale hash and
  a derivative file flips both findings to `[WARN]` with the
  recovery note `re-run `forge docs translate zh-CN``; `forge
  docs translate zh-CN --project <proj>` on a manifest with
  `path: ../evil.md` exits 1 with
  `error[docs-invalid]: docs invalid: derivative path
  `../evil.md` resolves outside the project; keep derivatives
  inside the project directory` and writes nothing; `forge
  docs translate fr --project <proj>` on a manifest with
  `fr.enabled: false` exits 1 with
  `error[docs-invalid]: docs invalid: locale `fr` is disabled;
  set `docs.translations.fr.enabled: true` to translate it`
  and writes nothing; `forge docs translate zh-CN --project
  <proj>` with a deliberately failing provider that emits
  `token ghp_abcdefghijklmnopqrstuvwxyz0123456789` on stderr
  keeps the prior `docs/README.zh-CN.md` and
  `.forge/docs/zh-CN/state.json` byte-identical and reports
  `error[translation-failed]` on stderr plus a typed `failed`
  per-locale outcome on stdout with the credential replaced
  by `[REDACTED]` everywhere it surfaces.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate documentation-translation --type change
  --strict --no-interactive`: valid pre-archive; `openspec
  validate documentation-translation --type spec --strict
  --no-interactive`: valid post-archive; `openspec validate
  --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive, includes the promoted
  `spec/documentation-translation`).
- `git diff --check`: PASS; staged set reviewed (6 files
  modified, 2 files added: `src/core/manifest.rs` for the
  new `docs.source` / `docs.non_translatable` manifest
  fields, `src/core/mod.rs` for the `docs-invalid` /
  `translation-failed` typed errors, `src/lib.rs` to register
  the new module, `src/doctor/mod.rs` for the
  `docs-<locale>` / `docs-freshness` findings plus 2
  contract tests, `src/main.rs` for the `forge docs
  translate` subcommand, `tests/agent_runtime_workflows_
  cross_surface.rs` for a formatting-only adjustment, plus
  `src/docs/mod.rs` with 16 unit tests, `tests/docs_
  contract.rs` with 12 contract tests, and the promoted
  spec — 9 files; archive under
  `openspec/changes/archive/2026-09-17-documentation-translation/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real provider integration is not exercised: a real
  `forge-docs-translator` binary is not present in the local
  sandbox, so the contract is validated through
  `FORGE_DOCS_TRANSLATOR_BIN` fixture scripts that stand in
  for the real provider. The credential redaction rule set is
  the same as `policy::redact_credentials`, which is itself
  verified through the existing quality policy contract
  tests. A real translation provider round trip is a
  downstream integration step and is not claimed here.

## Verification evidence (repository-distribution, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 300 passed, 0 failed (165 lib incl. 18 new
  distribution contract tests for provider validation,
  supported-vs-planned classification, manifest config
  parsing with primary+mirror separation, duplicate-mirror
  refusal, empty distribution refusal, request validation
  (empty refs, dash-prefixed refs, whitespace, implicit
  remote write refusal), plan/apply independence per
  remote, disabled-mirror boundary outcome, planned-provider
  unavailable outcome, credential redaction round-trip,
  state file round-trip, divergent mirror recovery,
  partial-failure independence and retry-skip-on-match);
  12 new distribution CLI/MCP contract tests for help
  listing, full primary+mirror delivery against bare-repo
  remotes, partial failure when the mirror remote is
  unconfigured, `--confirm` refusal without the flag,
  disabled mirror dry-run outcome, retry that records
  `skipped` for already-delivered refs, credential-shaped
  evidence round-trip, missing-distribution-section refusal,
  MCP `tools/list` advertising `mirror_project` as
  `external_write` with `confirm` required, MCP
  `mirror_project` typed refusal without `confirm`, MCP
  envelope matching the CLI JSON shape, and MCP dry-run not
  contacting any remote; 4 new distribution cross-surface
  tests for doctor verdict preservation after a successful
  mirror, registry `mirror` journal row carrying the
  per-remote summary, `commit`+`mirror` sequencing with a
  second commit re-pushing on retry (the state file tracks
  the SHA, not just the ref name), and agent session
  independence from the distribution surface; plus the
  unchanged 5 CLI contract, 4 cross-surface regression, 8
  doctor contract, 10 feature contract, 12 generate
  contract, 10 import contract, 7 profile contract, 7
  quality policy contract, 12 upgrade contract, 12 spec
  contract, 8 agent contract, 8 gitops contract, 13 mcp
  contract (incl. the new `mirror_project` registration in
  the tool list and the consistent `external_write` kind),
  9 mcp cross-surface; rust_scaffold and react_web scaffold
  tests are exercised in the long `cargo build+test` run
  that finishes in ~220s when the host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge mirror <proj> --ref main --confirm
  --dry-run` on a registered rust-web project with a
  `distribution: { primary: github, mirrors: [gitee] }`
  section lists the per-remote `would-push` plan; `forge
  mirror <proj> --ref main --confirm` against a project
  whose `origin` is a local bare repository and whose
  `mirror-gitee` remote is intentionally not configured
  reports `primary github delivered commit=<sha>` and
  `mirror gitee failed: ... fatal: 'mirror-gitee' does not
  appear to be a git repository ...` and exits 1 with
  `error[distribution-invalid]` so the per-remote evidence
  on stdout names the failing remote while the typed exit
  code is preserved; a `forge mirror <proj> --ref main
  --confirm --retry-failed` run after the partial failure
  records `primary github skipped commit=<sha>` (the SHA
  matches the state file) and re-attempts the failed
  mirror; divergent history on the mirror (a push to the
  mirror from a side clone) is detected as
  `mirror gitee diverged` with the recovery note
  `investigate the mirror's diverging history before
  re-pushing / remove the diverging commits from
  mirror-gitee or align with origin`; `forge mirror` on a
  project without a `distribution` section exits 1 with
  `error[distribution-invalid]: distribution invalid: project
  ... has no `distribution` section; declare a primary or at
  least one mirror`; `forge mcp serve` advertises
  `mirror_project` with `kind: external_write` and a schema
  that requires `path` and `confirm`; a JSON-RPC request to
  `mirror_project` with `confirm: false` is refused with
  `TOOL_REFUSED` (`-32012`) and the data code references
  the confirm boundary; a successful `mirror_project` MCP
  request returns the same `{contract, mirror: {...}}`
  envelope the CLI JSON output carries, and the
  `tools/list` snapshot does not include `deploy`,
  `publish`, `release`, `mirror` (as a top-level tool) or
  `docs`; a credential-shaped substring in evidence is
  redacted through `redact_distribution_evidence` which
  delegates to the same `policy::redact_credentials` helper
  the DriftWatch adapter uses; and a second commit on the
  same branch after the first mirror run re-records the
  local SHA in the state file so a subsequent
  `mirror --retry-failed` re-pushes the new commit instead
  of reporting `skipped` (the state file tracks the SHA,
  not just the ref name, so a stale approval cannot
  silently skip a new commit).
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate repository-distribution --strict
  --no-interactive`: valid pre-archive; `openspec validate
  --all --strict --no-interactive`: 25 passed, 0 failed
  (post-archive, includes the promoted
  `spec/repository-distribution`).
- `git diff --check`: PASS; staged set reviewed (4 files
  modified: `src/core/mod.rs` for the new typed errors,
  `src/lib.rs` to register the new module, `src/main.rs`
  for the `forge mirror` subcommand and the `cmd_mirror`
  helper, `src/mcp/mod.rs` for the `mirror_project` tool
  and dispatcher; 2 files added: `src/distribution/mod.rs`
  with 18 unit tests, `tests/distribution_contract.rs` with
  12 CLI/MCP contract tests, and
  `tests/distribution_cross_surface.rs` with 4 cross-surface
  regression tests, plus the promoted spec — 8 files;
  archive under
  `openspec/changes/archive/2026-09-17-repository-distribution/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real provider integration is not exercised: the local
  bare repository stands in for a GitHub primary and a
  Gitee mirror, so the contract is verified through
  `git push` against in-process bare repos. Real Gitee or
  GitHub HTTPS endpoints are not contacted from the
  sandbox, so a real mirror round trip (a push that hits a
  provider API) is a downstream integration step and is
  not claimed here. The credential redaction rule set is
  the same as `policy::redact_credentials`, which is itself
  verified through the existing quality policy contract
  tests.

## Verification evidence (mature-mcp-surface, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 273 passed, 0 failed (147 lib incl. 22 new mcp
  contract tests for tool registry / kind classification /
  unknown-tool / schema validation / shell-metacharacter id
  refusal / push confirm-required / project id kebab-case
  rejection / JSON-RPC envelope parse errors / diagnostic
  redaction / id round-trip on the wire / CLI/MCP domain
  equivalence on inspect / journal entry from a mutating tool;
  13 new mcp_contract tests for the full stdio loop through
  the built binary covering the help listing, the mature
  registry snapshot, the kind boundary (read-only/mutating/
  external_write), the unknown-tool structured error, the
  inspect/CLI domain equivalence, the create-project end-to-end
  with manifest and CLI list observability, the
  shell-metacharacter literal-data refusal, the
  push-confirm-required refusal with the data code, the
  parse-error envelope shape, the diagnostic redaction, the
  list/CLI domain equivalence, and the multi-request round
  trip; 5 new mcp_cross_surface tests for the doctor/CLI
  finding-count equivalence, the create + CLI feature-add
  invariant preservation, the generate-spec idempotency
  through the wire, the agent session recording through MCP
  observable from the CLI surface, and the commit
  paths-only contract against a tracked edit outside scope;
  plus the unchanged 5 CLI contract incl. the new `mcp`
  subcommand in the help output, 4 cross-surface regression,
  8 doctor contract, 10 feature contract, 12 generate
  contract, 10 import contract, 9 profile contract, 7
  quality_policy_contract, 12 upgrade contract, 12 spec
  contract, 8 agent contract, 8 gitops contract; rust_scaffold
  and react-web scaffold skipped in the regular run; the slow
  `cargo build+test` evidence path is exercised through the
  existing scaffold tests that finish in ~210s when the host
  toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge mcp serve` on the in-repo binary
  answers `tools/list` with the sixteen mature tools and the
  right `kind` for each (read-only/mutating/external_write),
  serves `create_project` for `rust-web` and reports the
  generated manifest + files + journal entry, serves
  `list_projects` afterwards and reports the registered
  record, and serves an `inspect_project` request that the
  CLI surface re-renders to the equivalent domain record;
  a request to a `deploy` tool is refused with the
  `TOOL_MISSING` code and a `tools/list` snapshot does not
  advertise `deploy` / `publish` / `release` / `mirror` /
  `docs`; a request with `confirm: false` for `push` is
  refused with the `push-confirm-required` data code; a
  request to `create_project` with `id: "evil; rm -rf /"`
  is refused as literal data and the destination directory
  is left empty; a request to `create_project` with an
  id that contains a credential-shaped substring is refused
  on the JSON-RPC response (the model can see it) but the
  diagnostic stream does not echo the secret.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate mature-mcp-surface --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive, includes the promoted
  `spec/mature-mcp-surface`).
- `git diff --check`: PASS; staged set reviewed (6 files
  modified, 3 files added: `src/mcp/mod.rs`,
  `tests/mcp_contract.rs`, `tests/mcp_cross_surface.rs`, plus
  the promoted spec — 9 files; archive under
  `openspec/changes/archive/2026-09-17-mature-mcp-surface/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real model-issued MCP traffic stays untested: the contract
  is verified end to end through synthetic JSON-RPC requests
  that match the v0.4 shape; the tool list advertises only
  operations whose Core contract was already verified in
  earlier cycles (read-only surfaces from `core-manifest-
  registry` / `profile-registry` / `doctor-maturity-assessment`,
  mutating surfaces from `deterministic-project-generation` /
  `feature-lifecycle` / `specification-remediation` /
  `agent-runtime-workflows` / `quality-policy-integration` /
  `project-upgrade-orchestration`, and the external-write
  `push` from `agent-runtime-workflows`). A real model
  consumer wiring `forge mcp serve` into an MCP-capable
  agent remains a downstream integration step and is not
  claimed here.

## Verification evidence (agent-runtime-workflows, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 233 passed, 0 failed (125 lib incl. 12 new agent
  contract tests for session-id validation, present-or-missing
  provider handling, pause/takeover unsupported state, restart
  preservation, run-spec refusal on missing bound spec,
  write/read roundtrip, and provider binary probe; 7 new gitops
  contract tests for the profile-aware test command, the
  requested-paths-only commit, the tracked-edit refusal, the
  untracked-file preservation, the empty-message refusal, the
  not-a-git-repository refusal, the push-confirm-required guard,
  the no-confirm and with-confirm push attempts, and the
  unrelated-tracked-changes helper for porcelain rename
  handling; 4 new cross-surface tests for the agent × spec
  interaction through pause, the commit × feature-add
  interaction, the test × upgrade dry-run interaction, and the
  push journal × confirm-flag interaction; 10 agent contract
  through the built binary, 8 gitops contract, 4
  agent-runtime-workflows cross-surface, 5 CLI contract incl.
  the new `agent`/`test`/`commit`/`push` subcommands in the
  help output, 3 cross-surface regression, 8 doctor contract,
  10 feature contract, 12 generate contract, 10 import
  contract, 9 profile contract, 7 quality_policy_contract, 12
  upgrade contract, 12 spec contract; rust_scaffold and
  react-web scaffold skipped in the regular run; the slow
  `cargo build+test` evidence path is exercised through the
  existing scaffold tests that finish in ~210s when the host
  toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id smoke-app`
  renders a `rust-web` project; `forge commit . --path
  README.md --message "bump readme"` produces a commit whose
  `files_changed` is `["README.md"]`; `forge push . --remote
  origin --ref-name main` exits 1 with
  `error[push-confirm-required]`; `forge push . --remote origin
  --ref-name main --confirm` reaches the underlying `git push`
  and reports the typed `git-dirty` failure with the captured
  stderr; `forge agent start . --session sess-1 --provider
  opencode` records an `active` state with provider
  `opencode`, spec binding, and writes
  `.forge/agents/sess-1/session.json` plus
  `transitions.log`; `forge agent pause . --session sess-1`
  returns `state: unsupported` with the explicit
  `pause primitive` evidence and a recovery note naming the
  existing PTY-based manager as the integration point;
  `forge agent takeover` returns the same `unsupported`
  state with `takeover primitive` evidence; `forge agent
  status . --session sess-1` renders the recorded session
  plus the full transition timeline; `forge agent list .`
  reports the session inventory with provider, state and
  transition count.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate agent-runtime-workflows --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/agent-runtime-workflows`).
- `git diff --check`: PASS; staged set reviewed (4 files modified,
  6 files added: `src/agent/mod.rs`, `src/gitops/mod.rs`,
  `tests/agent_contract.rs`, `tests/gitops_contract.rs`,
  `tests/agent_runtime_workflows_cross_surface.rs`, the
  promoted spec — 10 files; archive under
  `openspec/changes/archive/2026-09-17-agent-runtime-workflows/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real OpenCode/Codex PTY-manager integration stays untested:
  the existing PTY-based agent manager is not wired into this
  build, so the bundled adapters report `unsupported` for
  pause/takeover with explicit evidence rather than
  simulating success; the contract is verified through the
  test fixtures that confirm the unsupported state and the
  recovery note. Real PTY manager integration remains future
  work, matching the design decision that contract fixtures
  supplement but do not replace a real integration run.
- Real `git push` to a remote is also untested: the sandbox
  has no remote configured, so the with-confirm push attempt
  surfaces the typed `git-dirty` error with the captured
  `git push` stderr (`fatal: 'origin' does not appear to be a
  git repository`); the contract is verified end to end for
  the confirm-required guard and the underlying `git push`
  invocation.

## Verification evidence (specification-remediation, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 199 passed, 0 failed (113 lib incl. 7 new spec
  contract tests for spec-id stability, finding-set hash,
  validate-request refusal, build-draft provenance, idempotent
  generation, list, manual/deterministic/semantic routing and
  apply_routing outcomes, 9 spec contract incl. traceable proposal
  with provenance, idempotent re-run with mtime preservation,
  empty/oversized refusal, route classification, deterministic
  apply with no files, manual apply with no AI claim, semantic apply
  producing a bounded proposal, list/inspect roundtrip, and
  doctor-after-spec regression, 4 cross-surface incl. 1 new
  upgrade × spec semantic-conflict handoff, 5 CLI contract, 8 doctor
  contract, 10 feature contract, 12 generate contract, 10 import
  contract, 9 profile contract, 7 quality_policy_contract, 12 upgrade
  contract; rust_scaffold and react-web scaffold skipped in the
  regular run; the slow `cargo build+test` evidence path is exercised
  through the existing scaffold tests that finish in ~210s when the
  host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge spec generate` on a rust-web project writes
  `proposal.md` / `design.md` / `tasks.md` / `manifest.json` under
  `.forge/specs/<project>-<hash>/`; `forge spec list` reports the
  entry; `forge spec inspect <id>` shows the traceable provenance
  (project, profile, path, source revision, contract, generated_at,
  findings, dependencies, acceptance scenarios); a second
  `forge spec generate` with the same finding set reports
  `spec existing: ...` and writes nothing (boundary scenario);
  `forge spec generate` with no findings exits 1 with
  `error[spec-invalid]: spec invalid: spec generate requires at
  least one finding id`; `forge spec route dependency-drift` returns
  `route: deterministic` with `action: forge upgrade`; `forge spec
  route manifest-valid` returns `route: manual` with no action and
  no suggested spec; `forge spec apply driftwatch-DEPLOY-002` returns
  `route: semantic`, `status: spec-generated`, the bounded proposal
  is written, and the doctor verdict is unchanged after the spec
  operations; `forge spec apply manifest-valid` records the manual
  status with `note: manual boundary: no project changes and no AI
  fix claimed`; `forge upgrade` on a project with a drifted receipt
  still exits 1 with `error[feature-ownership-conflict]` and the
  stderr names `forge spec generate`; the cross-surface
  `upgrade_semantic_conflict_handoff_resolves_through_spec_apply`
  test confirms the receipt and manifest are preserved while the
  bounded proposal is written under `.forge/specs/`.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate specification-remediation --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/specification-remediation`).
- `git diff --check`: PASS; staged set reviewed (8 files modified,
  3 files added: `src/spec/mod.rs`, `tests/spec_contract.rs`, the
  promoted spec — 11 files; archive under
  `openspec/changes/archive/2026-09-17-specification-remediation/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real AI/agent implementation stays untested: the bounded spec is
  the handoff and the agent runtime is the next change; this cycle
  proves the spec storage, provenance, idempotency and routing
  contracts end to end, not the agent that consumes the spec. The
  spec's `tasks.md` enumerates the agent-side follow-up and the
  finding ids, dependencies and source revision are preserved for
  the agent adapter.

## Verification evidence (quality-policy-integration, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 180 passed, 0 failed (104 unit incl. 18 new policy
  contract tests for redaction shapes, JSON parsing, missing binary,
  invalid output, non-zero exit, stale observation, plus 5 new doctor
  policy integration tests for unavailable rollup, rule-id/severity
  preservation, not-applicable applicability, redaction-on-consume and
  stale-source demotion, 5 CLI contract, 3 cross-surface regression,
  8 doctor contract incl. the new `driftwatch-policy` finding, 10
  feature contract, 12 generate contract, 10 import contract, 9
  profile contract, 7 quality_policy_contract incl. missing-binary,
  parseable-report normalization with not-applicable preservation,
  non-zero exit, invalid output, credential redaction, per-project
  isolation and human-output redaction, 12 upgrade contract;
  rust_scaffold skipped in the regular run; the slow
  `cargo build+test` evidence path is exercised through the
  `rust_scaffold_builds_and_tests_with_native_toolchain` test that
  finishes in ~210s when the host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web` then `forge doctor` with
  no `FORGE_DRIFTWATCH_BIN` reports a `driftwatch-policy` finding
  with status `unavailable` and evidence naming the missing binary
  (`driftwatch invocation failed: binary not found on PATH`) so the
  project is not labeled healthy; `FORGE_DRIFTWATCH_BIN=…fake.sh forge
  doctor` against the same project reports `driftwatch-AUTH-001`
  (warn), `driftwatch-DEPLOY-002` (fail) and `driftwatch-FLUTTER-AUTH-001`
  (applicable:false, reason preserved) plus a `driftwatch-policy`
  rollup at `fail`; the same fake script reporting a credential-laden
  payload surfaces every secret as `[REDACTED]` in both JSON and
  human output; two projects running `forge doctor` in sequence each
  reference only their own evidence strings; aging `forge.yaml` after
  a successful run flips `driftwatch-AUTH-001` from pass to warn with
  a stale observation line.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate quality-policy-integration --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/quality-policy-integration`).
- `git diff --check`: PASS; staged set reviewed (5 implementation +
  test files modified, 3 files added: `src/policy/mod.rs`,
  `tests/quality_policy_contract.rs`, the promoted spec — 8 files;
  archive under `openspec/changes/archive/2026-09-17-quality-policy-integration/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real DriftWatch adapter execution stays untested: a real
  `driftwatch` binary is not present in the local sandbox, so the
  contract is validated through `FORGE_DRIFTWATCH_BIN` fixture
  scripts that stand in for the real tool. The brief's "real
  DriftWatch integration evidence" claim is deferred until a real
  binary is available, matching the design decision that contract
  fixtures supplement but do not replace a real integration run.

## Verification evidence (extended-profile-catalog, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 148 passed, 0 failed (80 unit incl. 11 new profile
  support_status/react-web/planned tests after dropping the stale
  `react-web` unknown-profile probe, 5 CLI contract, 3 cross-surface
  regression, 8 doctor contract incl. 1 new planned-profile
  doctor finding, 10 feature contract, 12 generate contract incl.
  react-web render + planned-profile generation refusal, 9 profile
  contract incl. 3 new react-web and planned-profile contract
  cases, 10 import contract, 11 upgrade contract; rust_scaffold
  skipped in the regular run; the slow `cargo build+test` evidence
  path is exercised through the `rust_scaffold_builds_and_tests_
  with_native_toolchain` test that finishes in ~218s when the host
  toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge profile list` shows all six supported ids
  (aspnet-web, flutter-app, nextjs-web, python-service, react-web,
  rust-web) at 0.1.0 and omits the planned candidates; `forge
  profile inspect react-web` (human + JSON) reports
  `support_status: supported` with `adapter-react`, toolchain
  `npm@20`, build `npm run build`, test `npm test`, and the
  client-only capability set; `forge profile inspect flutter-client`
  reports `support_status: planned` with a description that names
  the backend boundary; `forge profile resolve react-web --feature
  i18n` resolves `react-web@0.1.0 via adapter-react`; `forge
  profile resolve react-web --feature postgres` exits 1 with
  `error[incompatible-profile]` and the backend hint; `forge
  profile resolve aspnet-saas` exits 1 with
  `error[unsupported-profile]`; `forge new --profile react-web`
  creates a registered project with the expected 10 files;
  `forge new --profile rust-cli` exits 1 with
  `error[unsupported-profile]` and writes nothing.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate extended-profile-catalog --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/extended-profile-catalog`).
- `git diff --check`: PASS; staged set reviewed (8 implementation +
  test files, 5 archive files, 1 promoted spec — 14 files; 1033
  insertions, 36 deletions).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Per-service native react-web validation was not exercised in this
  cycle (no node toolchain integration in the local sandbox);
  rendering alone is verified and the
  `react_web_scaffold_builds_and_tests_with_native_toolchain` test
  remains available as the `npm run build` / `npm test` evidence
  path when the host toolchain is on PATH. Planned candidates stay
  discoverable through `inspect_profile` and `planned_profiles` but
  refuse generation before any file change; their promotion to
  `Supported` remains future work, not an implementation claim here.

## Verification evidence (project-upgrade-orchestration, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 138 passed, 0 failed (71 unit incl. 11 new upgrade
  plan/apply/fleet tests after the drifted-receipt test fix, 5 CLI
  contract, 3 cross-surface regression incl. 1 new
  upgrade × feature-lifecycle interaction, 8 doctor contract,
  10 feature contract, 12 generate contract, 10 import contract,
  7 profile contract, 12 new upgrade contract incl. dry-run plan with
  old/new versions/assets/recovery, apply advancing versions and
  changing files, semantic-conflict handoff on drifted receipt with
  preserved files, already-satisfied no-op, unknown feature failure
  before edits, missing requested feature install, postgres schema
  irreversible with declared strategy, fleet completion with
  per-project journals, fleet isolation of a blocked project without
  wholesale success, fleet retry skipping satisfied projects and
  re-planning on changed preconditions, fleet dry-run skipping
  writes).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id up-app` registers
  L1; `forge upgrade --feature auth` installs auth and writes receipt
  + manifest; `forge upgrade --all` reports `0 success, 0 failure,
  0 blocked, 1 skipped` for the now-satisfied project; `forge upgrade
  --all --dry-run` reports `plan only` without changes; `forge upgrade
  --feature nosuch` exits 1 with `error[unknown-feature]`;
  `forge doctor` still reports the same PASS manifest/profile/
  features/drift/build/deployment after fleet upgrades; postgres
  --dry-run after aging `postgres: 0.0.9` shows `upgrade postgres:
  0.0.9 -> 0.1.0 [package+configuration+codemod+schema]` with the
  irreversible `manifest-repin+manual-schema-review` recovery note.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate project-upgrade-orchestration --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive, includes the promoted
  `spec/project-upgrade-orchestration`).
- `git diff --check`: PASS; staged set reviewed (4 files modified,
  2 files added: implementation, tests, archive and promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Per-service native generation validation stays out of scope here;
  upgrade only repins manifest versions, receipts and runs declared
  policy validators; DriftWatch execution evidence stays deferred to
  v0.3 (`quality-policy-integration` and later).

## Verification evidence (feature-lifecycle, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 115 passed, 0 failed (61 unit incl. 12 new feature
  catalog/resolver/add/remove/upgrade/ownership/section-preservation
  tests, 5 CLI contract, 2 cross-surface regression, 8 doctor contract,
  10 new feature contract incl. catalog discovery, dep-ordered plans,
  conflict/missing/unsupported failures before edits, add-then-upgrade
  agreement, reverse-dep and ownership blocks with preservation,
  exact-reinstall no-op, closure selection in `new`, repeatability,
  12 generate contract, 10 import contract, 7 profile contract).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --feature admin` records
  auth+admin 0.1.0; `feature resolve rust-web --feature billing` plans
  auth,billing in order; `feature add billing` updates manifest+receipt+
  registry; `feature remove auth` exits 1 with
  `error[incompatible-feature]` naming admin,billing dependents and
  preserving files; `forge doctor` still PASSes manifest/profile/
  features-compatible on the feature-modified project.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate feature-lifecycle --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Package installation stays per-service native resolution (scaffolds are
  dependency-free by design); declared policy validators are reported
  per plan while DriftWatch execution evidence stays deferred to v0.3
  (`quality-policy-integration` and later).

## Verification evidence (doctor-maturity-assessment, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 93 passed, 0 failed (49 unit incl. 9 new doctor
  inventory/maturity/stale/unavailable tests, 5 CLI contract, 2
  cross-surface regression, 8 new doctor contract incl. stable
  finding IDs with evidence/remediation classes, unavailable-inspector
  reporting, repeatability without file changes, L2 missing-control
  reporting, L4 recovery denial, L0 nonapplicability, stale-observation
  reporting and generated-project registry freshness, 12 generate
  contract, 10 import contract, 7 profile contract).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web` then `forge doctor`
  reports PASS manifest/profile/features/drift/build/deployment with
  `[UNAVAILABLE] repository` outside a git repo and verdict `not
  healthy`; `--target L2` JSON reports unmet
  L2-auth/admin/ci/driftwatch (deployment met via Dockerfile) and
  L1-structure met after the forge.yaml-presence fix.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate doctor-maturity-assessment --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (10 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Doctor is local inspection in v0.1; DriftWatch execution evidence stays
  deferred to v0.3 (`quality-policy-integration` and later).

## Verification evidence (deterministic-project-generation, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 76 passed, 0 failed (40 unit incl. 11 new generate
  normalization/rendering/failure/cancel/collision/preflight tests, 5 CLI
  contract, 2 cross-surface regression, 10 import contract, 7 profile
  contract, 12 new generate contract incl. explicit-vs-interactive
  equivalence, nonempty/cancel/unknown/incompatible/id-collision failures,
  missing-toolchain unverified reporting, native rust/node/dotnet/flutter
  builds and python compile check).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new` for all five MVP profiles renders, registers
  and lists with L1 maturity; repeat `new` on a nonempty destination exits
  1 with `error[generation-conflict]`; piped interactive answers create the
  same request as flags; `--verify-native` on rust-web reports native
  `cargo build` + `cargo test` success; empty-PATH `--verify-native`
  exits 1 with `error[toolchain-missing]` containing "not tested".
- Native evidence (Forge unavailable): generated rust `cargo build` +
  `cargo test` PASS; `npm run build` + `npm test` PASS; `dotnet build`
  PASS (0 warnings, 0 errors); `flutter test` PASS (1 test); python
  `compileall` PASS while `pytest`/`build` modules are absent, so only
  rendering (not a pytest run) is claimed for python-service.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate deterministic-project-generation --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Generated scaffolds are dependency-free by design (offline-portable);
  per-service framework packages remain per-service resolution, not part
  of the scaffold claim.

## Verification evidence (project-import, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 53 passed, 0 failed (29 unit incl. 8 new import
  detection/adoption tests, 5 CLI contract, 2 cross-surface regression,
  7 profile contract, 10 new import contract incl. read-only proposal,
  ambiguity-before-writes, missing-remote inspection, accept-writes-only-
  manifest, id-collision/unwritable/legacy failures, repeatability,
  unknown-profile refusal, incompatible-manifest gating).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge import` on a Rust fixture reports Language rust,
  Framework axum, suggested rust-web/L1 with high confidence;
  `--accept` writes only `forge.yaml` and `inspect` returns profile
  rust-web maturity L1; mixed rust+flutter exits 1 with
  `error[ambiguous-import]` in human and JSON.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate project-import --strict --no-interactive`: valid
  pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Detection fixtures are synthetic; no new native-toolchain profile
  support is advertised. Native generation validation stays deferred to
  `deterministic-project-generation`.

## Verification evidence (profile-registry, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 35 passed, 0 failed (21 unit incl. 8 new profile
  descriptor/resolver/preflight tests, 5 CLI contract, 2 cross-surface
  regression, 7 new profile contract incl. list/inspect/resolve/preflight,
  flutter+postgres rejection, register gating and repeatability).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge profile list` shows all five MVP IDs at 0.1.0;
  `profile inspect rust-web` JSON carries adapter/build/test metadata;
  `profile resolve flutter-app --feature postgres` exits 1 with
  `error[incompatible-profile]` suggesting a backend boundary; empty-PATH
  `profile preflight rust-web` exits 1 with `error[toolchain-missing]`
  without claiming the profile was tested.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate profile-registry --strict --no-interactive`: valid
  pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- Committed as `9a3b263`; no push performed.
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Profile metadata alone does not establish working templates; native
  toolchain generation validation is deferred to
  `deterministic-project-generation`.

## Prior verification evidence (core-manifest-registry)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 20 passed, 0 failed (13 unit, 5 CLI contract, 2
  cross-surface regression incl. register→restart→inspect roundtrip,
  id/path collisions, unavailable/unknown reporting, dual-manifest and
  unsupported-schema failures leaving files unchanged, stale-pending
  journal reconciliation).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge --version`/`list`/`register`/`inspect` roundtrip
  against `tests/fixtures/valid-full`; unknown id exits 1 with
  `error[unknown-project]`; unknown subcommand exits 2 without creating a
  registry; empty registry lists an empty collection.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive); `openspec validate core-manifest-registry --strict`:
  valid pre-archive.
- `git diff --check`: PASS; staged set reviewed (23 files, implementation
  + tests + fixtures + ADR + README + archive + promoted specs only).
- Committed as `15c225f`; no push performed.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## Implementation cycle

1. Run `node scripts/check-openspec-change-names.mjs` before selection; failure blocks status/instructions and implementation.
2. Run `openspec list`, reconcile roadmap dependencies, and update the single pointer before work.
3. Run `openspec status --change extended-profile-catalog` and `openspec instructions apply --change extended-profile-catalog`; read all selected artifacts and applicable local rules.
4. Follow BFS analysis, structural pass, DFS requirement implementation, then BFS regression/completeness. Check tasks only against evidence.
5. Run the actual local build/test/integration commands and applicable Gate before archive; record exact failures and next actions. Gate FAIL or unresolved REVIEW_REQUIRED blocks completion when a Gate is configured.
6. Run the name checker and `openspec validate --all --strict --no-interactive`; review diffs and original impact surfaces.
7. Archive verified work without `--skip-specs`, inspect promoted canonical specs, and commit only related implementation/tests/archive/specs.
8. Advance `current_spec` to the next active eligible change, or remove the line when no active changes remain; update this evidence, commit HANDOFF separately and stop without push.

Planning-only documentation does not implement, archive or commit the queued changes. Future blockers must identify the exact failed command and next action; they must not be recorded as completion.
