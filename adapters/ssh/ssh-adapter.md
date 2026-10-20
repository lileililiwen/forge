# SSH adapter mapping facts

The reference adapter
[`forge-deployer-ssh`](forge-deployer-ssh) translates the
[`forge-deploy-executor/0.1.0`](../../docs/adapter-contracts/deploy-executor.md)
contract onto an SSH transfer. It contains no deployment logic of
its own: the artifact is copied to the named host under a staged
name and renamed into place, and the health check runs on the
host, because the health check describes the target's runtime. This
file records the exact mapping so a host owner can adopt or fork
the adapter without contract archaeology.

The transport facts are the publish lane's, so the repository has
one set of SSH facts: `/bin/mkdir`, `/bin/mv` and the `.forge-new`
staged suffix. The adapter is a separate process and cannot import
them, so the values are restated in the adapter and must stay
equal.

## Executor mapping

| Executor call | Adapter invocation | Meaning |
| --- | --- | --- |
| `apply --dry-run` | `ssh <target> /bin/mkdir -p <dir>`, `scp <artifact> <target>:<path>.forge-new`, `ssh <target> /bin/mv -f <path>.forge-new <path>` — none of them | Local preview only: the adapter names the transfer it would have made, reports `delivered` with a rehearsal note and contacts no host. |
| `apply` | the same three commands, in that order | The real transfer: the remote directory is created, the artifact is copied under the staged name, and the staged file is renamed into place. |
| `observe` | `ssh <target> <health command>` | Read-only: the health check runs on the host. Never re-applies. |

The order matters. The copy lands on the staged name, so a
half-written artifact is never visible under the final name, and
the rename is one atomic step on the host's filesystem.

The target's `host`, `user` and `path` travel in the executor
payload, which the contract already delivers on stdin; the argv
carries the operation, the kind and the health arguments. The
remote target is `user@host` when the target declares a user and
`host` otherwise.

The remote path, the host and the user travel as plain argv words.
Only characters that are inert in a remote shell word and
unambiguous in an `scp` remote spec are accepted; a declared value
outside that set is refused before anything runs, so a declared
path can never become a remote command.

## Exit-code mapping (apply)

| Step | Observed result | Executor result |
| --- | --- | --- |
| `apply` | all three commands exit 0 | `delivered`; observation `unknown` — health is proven later by `forge deploy observe` |
| `apply` | `mkdir`, `scp` or `mv` exits non-zero | `failed` with the transfer lines and the failing command's redacted output, and a non-zero exit, so Forge records a failed stage with this envelope's evidence |
| `apply` | a command exceeds the adapter's bound (30s for `ssh`, 60s for `scp`) | `failed` — an overrun is a failure, never a hang |
| `apply` | the kind is not `ssh` | `failed` with the reason. Forge never routes another kind here, but the adapter refuses it rather than serving it. |
| `apply` | the payload names no host or no path | `failed` with the reason. Forge refuses this target earlier, in the configuration validator, with `deploy-invalid`. |
| `apply --dry-run` | — | `delivered` with a rehearsal note; no host is contacted |

Any failure keeps the previously recorded `DeployState`
byte-identical (contract rule), and the command's output is
attributed as the adapter's own evidence.

The remote path never appears in the evidence: it is reported as
`<host-path>`. The local artifact is reported by its
project-relative name. Credential-shaped strings are scrubbed from
every emitted line, and Forge redacts again on the way in.

## Health vocabulary (observe)

The health check runs **on the host**, because the health check
describes the target's runtime:

| Health kind | Command on the host | Observation |
| --- | --- | --- |
| `http` | `curl -fsS -o /dev/null <url>` | `running` on exit 0; `failed` when the host answers with a non-2xx status |
| `docker` | `docker ps --filter name=<service> --format {{.Status}}` | `running` when a status contains `Up`; `failed` when the service is visible and not up; `unknown` when the service is not visible |
| `process` | `pgrep -f <process>` | `running` on exit 0; `failed` otherwise |
| none declared | — | `unknown` with the detail "no health check declared for this target" |

A host that cannot be reached is `unknown`, never `failed`:
disconnected means unknown, not offline proof. A missing
`health.service` for a `docker` check is `unknown`, not `failed`.

There is no optimistic default: only a literal healthy answer
observes healthy. An `unknown` observation never erases the last
good `running` observation recorded in Forge's state.

## Configuration

| Variable | Purpose |
| --- | --- |
| `FORGE_DEPLOYER_SSH_BIN` | Points Forge at this adapter (or a fork of it). Unset → Forge uses `forge-deployer-ssh`. |
| `FORGE_SSH_BIN` | The `ssh` program the adapter invokes. Default `ssh`. Set it to record or route the connection. |
| `FORGE_SCP_BIN` | The `scp` program the adapter invokes. Default `scp`. |
| `FORGE_DEPLOYER_SSH_REVISION` | Optional override of the `source_revision` attribution; otherwise the adapter records `unknown`. |

The host's own credentials (a key, a known-hosts file, an agent)
stay in the operator's SSH configuration. They never transit this
adapter's arguments, payload or envelope: the adapter only names a
host and a user, and the `ssh` program resolves the rest.

## Production promotion (the host owner owns this checklist)

This change ships the contract and the reference adapter as a
tested executable; production deployment evidence is explicitly
**not** claimed from a fixture round trip. Adoption by a host
owner:

1. Install the adapter on the machine that runs Forge, or point
   `FORGE_DEPLOYER_SSH_BIN` at a fork of it.
2. Give the Forge account passwordless access to the host, and
   confirm the host's `sshd` accepts the key.
3. Declare the target with `kind: ssh`, a `host`, a `path` and the
   `user` the account runs as; a target that does not state `host`
   and `path` is refused before any adapter runs.
4. Run one controlled `forge deploy apply --confirm --target <t>`
   and a follow-up `forge deploy observe`; record the outcome as
   live provider evidence.
5. Only then label the `ssh` row production-supported — the
   promotion decision and its evidence stay with the host owner.
