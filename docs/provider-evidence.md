# Provider integration evidence

`forge provider matrix|run|inspect` (`provider-integration-evidence`,
contract `0.1.0`) is the opt-in harness that attributes controlled
round trips to the external adapter boundaries without ever fabricating
a healthy result.

## Providers

| Id | Boundary | Live binary (default) | Override |
| --- | --- | --- | --- |
| `driftwatch-policy` | `check --dry-run --format json` (`gate --format json` for gate-managed projects — whose only side effect is a `gate_runs` row in the project's own `.driftwatch/` store), project as working directory; ordered binary probe `driftwatchdog` → `driftwatch` | `driftwatchdog` / `driftwatch` | `FORGE_DRIFTWATCH_BIN` |
| `gate-runtime` | `forge gate --dry-run` using the bundled engine; validates the resolved baseline and project extensions without running checks or persisting a verdict. A valid plan records `supported`; invalid project configuration or an unavailable Forge execution records `unavailable`. This provider row describes command support only and never claims a gate pass. | `forge` | none |
| `oidc-identity` | in-memory challenge/callback/claims/mint/validate/terminate | none (no external binary) | n/a |
| `analytics` | `health --provider <p> --project <id> --project-ref <ref> --plane <plane>` | `forge-analytics-adapter` | `FORGE_ANALYTICS_BIN` |
| `deploy` | `apply --target <t> --kind <k> --project <id> --revision <rev> --dry-run` under the `forge-deploy-executor/0.1.0` envelope contract ([contract](adapter-contracts/deploy-executor.md), [reference adapter](../adapters/jenkins/jenkins-adapter.md)) | `forge-deployer` | `FORGE_DEPLOYER_BIN` |
| `release` | `publish --stage <s> --project <id> --revision <rev> --dry-run` (package + container) | `forge-package-publisher` | `FORGE_PACKAGE_BIN` |

Row statuses: `supported` (controlled round trip with provenance),
`unavailable` (attempted, no success recorded), `not-run` (never
attempted; the default), `disabled` (manifest-disabled semantics of the
underlying adapter).

## Declared boundaries and provenance

A provider boundary in Forge is not only probed — it is **declared**, and
the declaration is read before anything compiled in.

- The registry resolves a plugin id from the inspected project's workspace
  declaration (`interfaces[]`, then `capabilities`/`integrations`), then
  from `kits/manifest.json` and finally from the builtin table, with the
  operator's `providers.yaml` descriptor speaking first. Each row carries
  the source that answered it (`descriptor` | `declared` | `manifest` |
  `builtin`), rendered by `forge plugins list` and by the API's
  `/v1/admin/projects/{id}/plugins` projection, and shown by
  `forge cap inspect` beside each Ready plugin. A reviewer can therefore
  tell a provider's own claim from a compiled-in guess.
- The declaration is `workspace-governance`'s format
  (`schemas/project.schema.json`); Forge reads the project directory it was
  pointed at and writes nothing. A declaration that cannot be read — an
  unparseable file, a mechanism outside
  `invoke|consume|observe|sibling`, a version-less contract on a non-
  `sibling` interface, a `consume`/`observe` edge without a digest — refuses
  with `plugin-boundary-refused`, naming the file or field. A configured
  descriptor that contradicts the declaration refuses naming the id and
  both values. An id nothing declares still resolves through the builtin
  table and reports `builtin`.
- The **evidence** for a boundary and the **declaration** of it are
  different things, and this document keeps them apart: the matrix rows
  above say what has been *observed*, while a declared interface says what
  the boundary *is* and what version it speaks. A declared interface
  promotes no matrix row: an unprobed provider stays `not-run`, and a
  declaration is never presented as a successful round trip.
- Forge's own boundaries are declared in `forge/.project.json` with
  contract versions derived from the versioned-surface inventory
  (`CONTRACTS` in `src/contract/mod.rs`, the adapter contract constants and
  `PLATFORM_FAMILY_VERSIONS`) rather than restated; a boundary with no
  contract version today (the `ariadex` and `sisyphusfy` agent runtimes) is
  recorded as an **integration** with an `evidence_ref`, never as an
  interface with an invented version. See
  [architecture.md](architecture.md#forges-own-declaration).

## Opt-in rule

- `forge provider matrix` reports every row as `not-run`. A `not-run`
  row never becomes `supported`.
- `forge provider run <id> [--live] [--fixture <path>] [TARGET]`
  performs exactly one controlled round trip: `--fixture` labels the
  row `sandbox: fixture` (supplemental harness proof, never provider
  support); `--live` requires `FORGE_PROVIDER_LIVE=1` and labels the
  row `sandbox: live`. Without either flag the row is `not-run` and
  nothing is contacted.
- Binary probes always pass `--dry-run` where the adapter supports it,
  run with a 10s bounded wait, and parse only the envelope shape; a
  probe never performs a real remote write.
- The identity probe exercises the real `crate::identity` lifecycle
  and terminates the probe session; cross-project, expired, revoked
  and non-admin presentations stay refusals. A real OIDC issuer round
  trip remains a downstream step.

## Secret boundary

Secrets reach Forge only through the runner environment (binary paths,
fixture files). Manifests and the repository never carry provider
secrets. Every captured string — receipts, evidence, version probes,
diagnostics — passes through `policy::redact_credentials` before it
reaches stdout, JSON, the journal or a state file.

## Provenance and teardown

Each `supported` / `unavailable` row carries provider, sandbox,
source, project id, VCS revision (absent for temp dirs, reported as
`unversioned` rather than invented), timestamp, tool version, redacted
receipt and `teardown`. Binary probes run in disposable temp dirs that
are removed afterwards; targeted runs attribute the real project id in
the journal. The matrix and untargeted runs journal under the
synthetic `__provider__` id and invent no registered project.

## Workspace Governance adapter consumption

`governance use workspace-governance` carries a packaged adapter preset
(`workspace-governance-adapter-consumption`): the candidate
`<workspace-root>/forge/modules/workspace-governance/scripts/forge_governance_adapter.py`
resolved from `--workspace-root`/`FORGE_WORKSPACE_ROOT` only, verified as an
existing executable regular file, with the resolved adapter path and the
workspace root stored under `.forge/providers.yaml` and re-supplied to the
adapter as `WORKSPACE_ROOT` on every run. This is the governance plane's own
surface, not a `forge provider matrix` row.

Historical live status (2026-09-24, before the in-tree import): the sibling
candidate was mode `100644` and the direct-exec boundary refused it. That
observation is superseded by the 2026-10-19 consolidation and origin
retirement. The current preset points into the Forge-owned module and never
searches the archived sibling checkout. Contract tests prove that the Forge
candidate wins when both paths exist and that the legacy sibling path alone
is refused.

The consumption loop itself was proven live end to end: a scratch portfolio
holding a byte-identical copy of the sibling adapter (execute bit on the
copy only; the real tree untouched) ran the full audit-shaped matrix —
`pass`/`fail`/`blocked`/`unknown` for `forge`, `crossalheart`, an adoption-gap
copy of `argoset` and an unregistered id — through `governance use` +
`governance status`, and a later removed checkout surfaced `unavailable` while
local commands continued unchanged. Fixture stubs mirror every captured
document; tests use `sandbox: fixture` paths only.

## Providers not run (this host, 2026-09-21)

No live sandbox is configured in the local environment, so every live
claim below is `not-run` by design. CI (`artifact-and-ci-baseline`)
runs Forge's bundled gate directly; it does not install a separate gate
runtime. Other rows stay `not-run` in CI — no OIDC
issuer, analytics adapter, deploy target or release publisher is
provisioned, and the `surfaces` job runs only `forge provider matrix`
without `--live`, which never claims support.

- Real `driftwatch` binary: the policy plane probes the cargo name
  `driftwatchdog` first and then the npm launcher alias `driftwatch`.
  Live status (this host, 2026-09-24): `forge provider run
  driftwatch-policy <project> --live` round-tripped `supported`
  (`sandbox: live`, `source: live:driftwatchdog`) through both the
  `FORGE_DRIFTWATCH_BIN` override and the ordered PATH probe against a
  release build of the sibling at commit `25811ed`
  (`driftwatch-checker/0.1.0` document, `check --dry-run --format json`
  persisting nothing); the stale `~/.cargo/bin/driftwatchdog` install
  predates the envelope and is honestly classified `unavailable`. On
  hosts without either name the row stays `unavailable`; tests use
  fixture scripts and PATH-controlled stubs (`sandbox: fixture`).
- Real OIDC issuer: no issuer URL, client or test subject; identity
  evidence uses the in-memory fixture lifecycle.
- Real analytics sources (`unified-content`, `github-analytics`): no
  adapter binary or project ref; evidence uses fixture scripts.
- Real deploy targets (`local`, `docker-compose`): no live Jenkins
  round trip is claimed; evidence uses the reference adapter
  `adapters/jenkins/forge-deployer-jenkins` against stubbed
  jenkins-local trees with `--dry-run`. Production promotion is a
  jenkins-local adoption step (see its checklist).
- Real release registries (package, container, notes): no publisher
  binaries; evidence uses fixture scripts with `--dry-run`.

## Gate evidence export consumption

`forge gate evidence [TARGET]` (`gate-evidence-export-consumption`)
consumes Driftwatchdog's `gate evidence-export --format json` verb
and persists a versioned `release-evidence/0.1.0` record under
`.forge/gate/<project-id>/release-evidence.json`. The sibling owns
the export entry point, its field semantics and its refusal rules;
Forge validates against the consumed governance vocabulary (nine fields,
four non-blocked states) and enforces standing boundary rules:
revision mismatch → every field `unverified`; unknown field name
or out-of-vocabulary state → refused; `verified` publication
without digests → refused as contradictory; absent export →
`unavailable`. `forge gate evidence status [TARGET]` reads the
persisted record without side effects.

Live status (this host, 2026-09-27, sibling `driftwatchdog` at
`221faeca`): the sibling's export entry point exists and produces a
parseable document; this checkout's `.driftwatch/` store was initialized
(`driftwatchdog init --no-config`); the first export shows all nine
fields as `unverified` because no check command was declared; the
consumption was verified live (`forge gate evidence .` exit 0, persisted
record at `.forge/gate/forge/release-evidence.json`).

What stays `not-run`: the gate verdict surface (`forge gate status`)
is unchanged; no MCP tool, API route or portal control was added;
`forge fleet` gains no healthy state from consumed evidence;
`deployable` is never set by consumption; and generated project
templates carry no `release_evidence` block.

## Portfolio interest import consumption

`forge portfolio interest import` (`portfolio-interest-snapshots`,
contract `forge-portfolio-interest/0.1.0`) consumes *already-aggregated*
records from an approved analytics source. It is the consuming half of
an analytics pipeline: Forge owns the privacy boundary (closed key set,
allowlisted counts, rejection rather than redaction), the append-only
store, the overlap rule and the read model, while the provider owns
collection, identity policy and credentials. No verb under `forge
provider matrix` produces aggregate snapshots — the `analytics` row is
the `health` probe only.

Live status (this host, 2026-09-28): **no analytics provider was
contacted and no provider credential exists.** Every verification
fixture in that package is a local JSON document or an in-process call;
there is no adapter subprocess, no `FORGE_*_BIN` override and no network
request on the interest code path. Forge therefore claims nothing about
how a figure was produced beyond the `privacy_mode` (`exact-count` /
`lower-bound` / `undeclared`) and `coverage` (`complete` / `partial`)
the source declared, and a `partial` source reporting all zeros is
refused rather than stored as "nobody was interested". The public share
manifest, the private portfolio projection and the fleet list carry no
interest data.

**Exact next action:** the analytics adapter boundary exposes only
`health --provider <p> --project <id> --project-ref <ref> --plane <p>`;
it has no verb that emits aggregate snapshots. A producer must add one —
the provider owns its shape, its credentials and its collection policy —
and it must emit a document Forge accepts at
`contract: forge-portfolio-interest/0.1.0`. Until that exists, live
interest evidence stays `not-run`, `interest-snapshots` makes no
live-aggregate claim, and no reviewed aggregate exists to satisfy the
`portfolio-activation-readiness` gate. What is missing is a producer,
not a consumer: the import verb and its store are complete and verified
locally.
