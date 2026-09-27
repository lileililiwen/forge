# Provider integration evidence

`forge provider matrix|run|inspect` (`provider-integration-evidence`,
contract `0.1.0`) is the opt-in harness that attributes controlled
round trips to the external adapter boundaries without ever fabricating
a healthy result.

## Providers

| Id | Boundary | Live binary (default) | Override |
| --- | --- | --- | --- |
| `driftwatch-policy` | `check --dry-run --format json` (`gate --format json` for gate-managed projects — whose only side effect is a `gate_runs` row in the project's own `.driftwatch/` store), project as working directory; ordered binary probe `driftwatchdog` → `driftwatch` | `driftwatchdog` / `driftwatch` | `FORGE_DRIFTWATCH_BIN` |
| `gate-runtime` | `gate --dry-run --format json` against the resolved gate runtime with the project as the working directory; the real sibling's dry-run surface is a side-effect-free plan preview (no JSON composition at `25811ed`), so a responding plan, a parseable status document, or the honest "nothing to gate" answer records `supported` while a missing binary (resolution: explicit `FORGE_GATE_BIN`, else the ordered PATH probe), timeout or refused invocation records `unavailable`; the probe never runs the real gate and never claims a gate pass | `driftwatchdog` / `driftwatch` | `FORGE_GATE_BIN` |
| `oidc-identity` | in-memory challenge/callback/claims/mint/validate/terminate | none (no external binary) | n/a |
| `analytics` | `health --provider <p> --project <id> --project-ref <ref> --plane <plane>` | `forge-analytics-adapter` | `FORGE_ANALYTICS_BIN` |
| `deploy` | `apply --target <t> --kind <k> --project <id> --revision <rev> --dry-run` under the `forge-deploy-executor/0.1.0` envelope contract ([contract](adapter-contracts/deploy-executor.md), [reference adapter](../adapters/jenkins/jenkins-adapter.md)) | `forge-deployer` | `FORGE_DEPLOYER_BIN` |
| `release` | `publish --stage <s> --project <id> --revision <rev> --dry-run` (package + container) | `forge-package-publisher` | `FORGE_PACKAGE_BIN` |

Row statuses: `supported` (controlled round trip with provenance),
`unavailable` (attempted, no success recorded), `not-run` (never
attempted; the default), `disabled` (manifest-disabled semantics of the
underlying adapter).

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
`<workspace-root>/workspace-governance/scripts/forge_governance_adapter.py`
resolved from `--workspace-root`/`FORGE_WORKSPACE_ROOT` only, verified as an
existing executable regular file, with the resolved adapter path and the
workspace root stored under `.forge/providers.yaml` and re-supplied to the
adapter as `WORKSPACE_ROOT` on every run. This is the governance plane's own
surface, not a `forge provider matrix` row.

Live status (this host, 2026-09-24, sibling `workspace-governance` at
`204d140`): the real candidate exists at the real portfolio root but the
sibling committed the adapter as git mode `100644` while its other scripts
are `100755`, so the direct-exec v0.1.0 boundary honestly refuses it —
recorded verbatim in `tests/fixtures/governance-audit/NOTES.md`. **Exact next
action:** the sibling sets the execute bit on
`scripts/forge_governance_adapter.py` (e.g. `git update-index
--chmod=+x scripts/forge_governance_adapter.py`) to match its own README
invocation; until then the live row stays honestly not-run for the preset
path, and the documented explicit-adapter remedy (`--adapter <path>
--workspace-root <root>`) carries any non-executable checkout.

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
changes this for exactly one surface: the `gate` job installs the
declared gate runtime and initializes its store in the runner workspace,
so `forge gate .` executes for real there and the job mirrors the
runtime's verdict. Every other row stays `not-run` in CI — no OIDC
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
