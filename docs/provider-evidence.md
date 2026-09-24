# Provider integration evidence

`forge provider matrix|run|inspect` (`provider-integration-evidence`,
contract `0.1.0`) is the opt-in harness that attributes controlled
round trips to the external adapter boundaries without ever fabricating
a healthy result.

## Providers

| Id | Boundary | Live binary (default) | Override |
| --- | --- | --- | --- |
| `driftwatch-policy` | `check --project <dir> --format json` | `driftwatch` | `FORGE_DRIFTWATCH_BIN` |
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

## Providers not run (this host, 2026-09-21)

No live sandbox is configured in the local environment, so every live
claim below is `not-run` by design:

- Real `driftwatch` binary: not installed; policy evidence uses
  fixture scripts (`sandbox: fixture`).
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

To attempt live evidence, install the sandbox binaries, export
`FORGE_PROVIDER_LIVE=1` (plus the per-adapter `FORGE_*_BIN` paths and
any sandbox credentials through the runner's secret mechanism, never
the repository), and run `forge provider run <id> <project> --live`.
