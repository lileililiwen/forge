# Deploy executor contract `forge-deploy-executor/0.1.0`

Status: frozen 2026-09-24 (`jenkins-deploy-adapter-consumption`).
The Forge deploy plane (`src/deploy`) invokes one external
executable through `FORGE_DEPLOYER_BIN` (default
`forge-deployer`). This document is the authoritative boundary
for third-party executors. The bundled reference adapter is
[`adapters/jenkins/forge-deployer-jenkins`](../../adapters/jenkins/forge-deployer-jenkins);
its jenkins-local mapping facts (verbs, exit codes, status shapes,
job URL env) live in
[`adapters/jenkins/jenkins-adapter.md`](../../adapters/jenkins/jenkins-adapter.md).

Distinct version strings, deliberately:

- `forge-deploy-executor/0.1.0` — the **executor wire**
  discriminator carried in the stdin payload and the stdout
  envelope below. This is what a conformant adapter must echo.
- `0.1.0` (`DEPLOY_CONTRACT_VERSION`) — the **Forge-side**
  version recorded inside plans, reports and
  `.forge/deploy/<project-id>/<deploy-id>/state.json`. It is
  Forge's own surface data version and is never sent as the
  wire discriminator.

A bare `0.1.0` in the envelope's `contract` field is a mismatch:
the pre-namespacing value is refused so a stale executor can
never masquerade as conformant.

## Invocation model

- One executable resolved from `FORGE_DEPLOYER_BIN` (or the
  `forge-deployer` PATH default), invoked with **argument
  arrays** — never an interpolated shell string.
- Stdin receives one supplementary JSON payload (see below) and
  is closed so scripts that `cat`/`read` see EOF.
- Stdout carries exactly one JSON envelope. Trailing
  human-readable noise after the envelope is a contract
  violation; put progress on stderr.
- A bounded wait applies (60s per invocation through the CLI
  and API; the provider probe waits 10s). A timed-out or
  unresponsive executor is unavailable.
- The adapter path is operator-installed configuration: same
  trust model as the docs, policy, analytics and release
  adapters.

## Operations

`apply` — run (or rehearse, with `--dry-run`) the named
deployment:

```text
apply --target <name> --kind <local|docker-compose|ssh> \
      --project <id> --revision <sha> \
      [--artifact <project-relative-path>] \
      [--health-kind <docker|http|process> \
       [--health-service <s>|--health-url <u>|--health-process <p>]] \
      [--dry-run]
```

`observe` — read-only health status for a persisted deploy
identity. Observe must never trigger a deployment side effect:

```text
observe --target <name> --project <id> --deploy-id <id> \
        [--health-kind ... (same shape as apply)]
```

`--deploy-id` is Forge's derived identity
(`<project>-<target>-<12hex>` over project, target and source
revision). `ssh` targets are refused Forge-side before any
invocation, so a conformant executor never sees them.

## Stdin payload

```json
{
  "contract": "forge-deploy-executor/0.1.0",
  "project_id": "app",
  "deploy_id": "app-home-1f2e3d4c5b6a",
  "target": { "name": "home", "kind": "local", "host": null,
              "user": null, "path": null, "service": null,
              "note": null },
  "artifact": { "path": "docker-compose.yml",
                "content_hash": "<sha256>", "byte_size": 42 },
  "health": { "kind": "docker", "service": "app", "url": null,
              "process": null, "interval_seconds": null },
  "revision": "<full source sha>",
  "dry_run": false
}
```

`artifact` and `health` are `null` when absent. The payload is
supplementary context; argv is authoritative for the operation.

## Stdout envelope

```json
{
  "contract": "forge-deploy-executor/0.1.0",
  "apply_status": "delivered",
  "apply_note": "human-readable outcome",
  "apply_evidence": ["short attributable lines"],
  "observation_status": "running",
  "observation_detail": "what was observed",
  "observation_evidence": ["short attributable lines"],
  "recovery": ["operator next steps"],
  "source": "forge-deployer-jenkins/0.1.0",
  "source_revision": "<adapter-runtime revision or unknown>",
  "observed_at": "2026-09-24T03:42:24+00:00"
}
```

Required behaviour of each field:

- `contract` — must equal the wire discriminator exactly.
- `apply_status` — one of `delivered`, `skipped`, `disabled`,
  `failed`, `unknown` (Forge stage vocabulary). Default when
  absent: `failed`.
- `apply_note` / `observation_detail` — bounded, redacted
  Forge-side before they reach the state file or transports.
- `apply_evidence` / `observation_evidence` / `recovery` —
  arrays of strings; unknown extra keys (e.g. `observed_at`)
  are tolerated but Forge stamps its own observation timestamps.
- `observation_status` — one of `running`, `failed`, `unknown`.
  Anything outside that set is recorded as `unknown`.
- `source` / `source_revision` — optional self-identification.
  Forge appends `executor=<source>@<revision>` to each stage's
  evidence; an adapter that does not name itself attributes to
  its binary name with `unknown` revision, never a claimed
  version.

## Classification rules

| Observed outcome | Forge records |
| --- | --- |
| exit 0, conformant envelope | the stage outcome exactly as the envelope names it |
| non-zero exit, conformant envelope | the named stage **failed** with the envelope's evidence; a contradictory `delivered` claim is downgraded to `failed` because the exit status is authoritative |
| non-zero exit, no envelope | `deploy-target-unavailable` |
| exit 0, non-JSON or unknown contract | `deploy-target-unavailable`, naming the observed contract |
| bounded-wait overrun | `deploy-target-unavailable` |
| binary missing/unspawnable | `deploy-target-unavailable`, naming the binary |

Prior-state preservation: state persists only when the apply
stage is `delivered`, `skipped` or `disabled` **and** the run
was not a dry-run. Every unavailable outcome and every failed
stage leaves the previous `DeployState` file byte-identical, so
a broken or absent executor never erases real evidence.
`unknown` never overwrites the recorded last-good `running`
observation (`last_observed_running`): disconnected means
unknown, not offline proof.

## Health vocabulary

Forge maps `observation_status` verbatim within
`running` / `failed` / `unknown`. The reference adapter applies
the explicit table in
[jenkins-adapter.md](../../adapters/jenkins/jenkins-adapter.md);
no executor may default an unrecognized state to healthy.

## Security and confinement

- Argument arrays only; no shell interpolation of Forge data.
- Every captured string passes
  `policy::redact_credentials` Forge-side; reference adapters
  scrub first so host secrets never leave the adapter.
- Forge stores neither provider credentials nor absolute host
  paths: adapters replace them (`[REDACTED]`, `<host-path>`)
  before emitting evidence. Job URLs and endpoints stay in
  adapter-side configuration referenced as `scheme://` values.
- A dry-run (`--dry-run`) must never perform or trigger a real
  deployment side effect, on the adapter side as well as in
  Forge's records.

## Versioning

Breaking changes to argv, the envelope schema or the
classification table bump the discriminator
(`forge-deploy-executor/0.2.0`). Additive optional envelope
fields are compatible within `0.1.0`. Forge refuses any
discriminator it does not implement, naming both sides.
