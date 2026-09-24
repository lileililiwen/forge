# Jenkins adapter mapping facts

The reference adapter
[`forge-deployer-jenkins`](forge-deployer-jenkins) translates the
[`forge-deploy-executor/0.1.0`](../../docs/adapter-contracts/deploy-executor.md)
contract onto the workspace's existing **jenkins-local** scripts.
It contains no deployment logic of its own: sync, compose, port
allocation, job registration, shared PostgreSQL and host paths all
stay in jenkins-local. This file records the exact mapping so the
jenkins-local repository can adopt or fork the adapter without
contract archaeology.

## Executor mapping

| Executor call | jenkins-local invocation | Meaning |
| --- | --- | --- |
| `apply --dry-run` | `project.sh <project> deploy --dry-run` | Local preview only: jenkins-local prints `Would sync, configure shared PostgreSQL when needed, register the Jenkins job, and trigger it.` and exits **before** the Mac handoff. No side effect, no host contact. |
| `apply` | `project.sh <project> deploy` | The real trigger: syncs to the Mac, registers the Jenkins job and starts the `Production/<project>` pipeline build (`ACTION=deploy`). |
| `observe` | `project-action.sh <project> status` | Read-only status (the same verb the Jenkins status job runs): prints the `project-status.sh` table filtered to the project row. Never re-applies. |

Project names are validated with jenkins-local's own grammar
(`^[A-Za-z0-9][A-Za-z0-9_-]*$`) before anything runs. Forge's
kebab-case project ids satisfy it.

## Exit-code mapping (apply)

Observed `project.sh` outcomes map to executor statuses:

| `project.sh` exit | Observed output | Executor result |
| --- | --- | --- |
| 0 | `Triggered Jenkins: Production/<project> (deploy)` | `delivered`; observation `unknown` — the pipeline just started, health is proven later by `forge deploy observe` |
| 1 | sync, port-preparation or job-registration failure | `failed` with the redacted output lines |
| 2 | invalid project name or option | refused pre-invocation: non-zero exit without an envelope → `unavailable` |
| 3 | `Project configuration was created...` / first-time shared-database migration required | `failed` + recovery: complete the Mac-side step named by the output, then rerun |
| 4 | `Jenkins credentials are not initialized on the Mac` | `failed` + recovery: create the protected credential file, then rerun |
| 5 | Workspace Governance not installed at the expected root | `failed` + recovery: install/sync Workspace Governance, then rerun |
| 6 | Workspace Governance rejected the deployment | `failed` + recovery: resolve the governance check findings |

Any failure keeps the previously recorded `DeployState`
byte-identical (contract rule), and the guidance lines from the
script are attributed as the adapter's own evidence.

## Health vocabulary (observe)

`project-status.sh` prints one row per project:
`PROJECT CONFIG CONTAINERS URL`. The adapter reads the
`CONTAINERS` column for the matching project row:

| `CONTAINERS` cell | Observation |
| --- | --- |
| `running` | `running` — the only healthy state |
| `stopped` | `failed` — deployment down |
| `partial` | `unknown` — some containers up is not health proof |
| `not-created` | `unknown` — the stack was never brought up; observe cannot distinguish "never deployed" from "not visible here" |
| project row absent / any unrecognized token | `unknown` |
| status command exits non-zero | `unknown` ("unknown is not offline proof") |

There is no optimistic default: only a literal `running` cell
observes healthy. An `unknown` observation never erases the last
good `running` observation recorded in Forge's state.

## Configuration

| Variable | Purpose |
| --- | --- |
| `FORGE_DEPLOYER_BIN` | Points Forge at this adapter (or a fork of it). |
| `FORGE_JENKINS_LOCAL_DIR` | Path to the jenkins-local checkout that owns `project.sh` / `project-action.sh`. Unset or missing → the adapter exits non-zero **without** an envelope, which Forge classifies `unavailable`. |
| `FORGE_JENKINS_STATUS_CMD` | Optional replacement for the status verb executable (still invoked as `<cmd> <project> status`). Use it when observation must be routed through SSH or the Jenkins status job instead of a local script. |
| `FORGE_JENKINS_REVISION` | Optional override of the `source_revision` attribution; otherwise the adapter records the jenkins-local checkout's short git HEAD (or `unknown`). |

Jenkins connection facts (`JENKINS_URL`, `JENKINS_PUBLIC_URL`,
`JENKINS_USER`, `JENKINS_TOKEN`) are consumed by jenkins-local
on the Mac from its protected credential file
(`$SECRETS_ROOT/jenkins.env`); they never transit this adapter's
arguments, payload or envelope. The adapter scrubs
credential-shaped strings and absolute host paths from all
evidence anyway, and Forge redacts again.

## Production promotion (jenkins-local owns this checklist)

This change ships the contract and the reference adapter as a
tested executable; production deployment evidence is explicitly
**not** claimed from a fixture round trip. Adoption by
jenkins-local:

1. Fork or vendor this adapter into the jenkins-local repo (or
   keep pointing `FORGE_DEPLOYER_BIN` at the Forge-tree copy).
2. Run `forge provider run deploy --fixture <adapter>` with
   `FORGE_JENKINS_LOCAL_DIR` set; the row must reach `supported`
   with `sandbox: fixture` (dry-run only).
3. Against the real Mac host, run one controlled
   `forge deploy apply --confirm --target <t>` per project and
   a follow-up `forge deploy observe`; record the outcome as
   live provider evidence.
4. Only then label the `deploy` row production-supported — the
   promotion decision and its evidence stay in jenkins-local.
