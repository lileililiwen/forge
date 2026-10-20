## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The deploy target kind registry: the wired flag, the required fields, the adapter variable and the adapter default | `forge` | Yes. `src/extension/kinds.rs`. |
| The deploy lane's adapter resolution | `forge` | Yes. `src/deploy/model.rs`, `src/deploy/engine/stages.rs`. |
| The configuration validation of a declared target | `forge` | Yes. `src/deploy/model.rs`. |
| The reference SSH adapter and its mapping facts | `forge` | Yes. `adapters/ssh/`. |
| The deploy executor's documentation | `forge` | Yes. `docs/adapter-contracts/deploy-executor.md`. |
| The deploy contract's capability text | `forge` | Yes. `openspec/specs/adapter-deployment/spec.md`. |
| The executor contract's argv, envelope schema and classification table | `forge` | No. Read only. The `ssh` kind needs no new operation. |
| The manifest's `deployment` block and its field names | `forge` | No. Read only. No field is added. |
| The plugin registry, the publish lane's SSH transport, the fleet probe | `forge` | No. Read only. |
| The release, delivery and publish lanes | `forge` | No. Read only. |
| Every sibling module | its owner | No. Not edited. |

## The Kind Registry Gains Two Declarations

`src/extension/kinds.rs` owns the registry, and the registry is where a kind
says what it is. Today a kind is a name and a wired flag. A branch needs two
more things, and both belong next to the kind rather than in a validator:

```rust
pub struct DeployTargetKind {
    pub kind: &'static str,
    /// False when the kind is known but not wired into this build.
    pub wired: bool,
    pub summary: &'static str,
    /// The target fields a target of this kind must state.
    pub required_fields: &'static [&'static str],
    /// The variable that names this kind's own adapter binary.
    pub adapter_env: Option<&'static str>,
    /// The adapter binary this kind uses when nothing else names one.
    pub default_adapter: &'static str,
}
```

| Kind | Wired | Required fields | Adapter variable | Adapter default |
|---|---|---|---|---|
| `local` | yes | — | — | `forge-deployer` |
| `docker-compose` | yes | — | — | `forge-deployer` |
| `jenkins` | yes | — | — | `forge-deployer` |
| `mac-runtime` | yes | — | — | `forge-deployer` |
| `ssh` | **yes** | `host`, `path` | `FORGE_DEPLOYER_SSH_BIN` | `forge-deployer-ssh` |

The `ssh` kind requires `host` and `path`: a target without a host is not a
target, and a target without a remote path has nowhere to put the artifact.
`user` stays optional — the SSH client already has a well-defined default
for it — and `service` stays optional because only a Compose health check
reads it.

The kinds that name no adapter of their own keep using the shared binary, so
the resolution rule is one rule and not one rule per kind.

## The Adapter Resolution

Today one binary serves every kind. The deploy lane resolves it once, from
`FORGE_DEPLOYER_BIN`, and the reference Jenkins adapter refuses the kinds it
does not know — so a project with a Jenkins target and an SSH target cannot
be served today.

The resolution becomes per kind, in one order:

1. the kind's own variable, when it is set and not blank;
2. the shared variable, when it is set and not blank;
3. the kind's default binary, from the registry.

`DeployAdapterConfig` therefore carries two things instead of one:

```rust
pub struct DeployAdapterConfig {
    /// The shared adapter binary. Empty when the shared variable is
    /// unset, so the resolution can tell "unset" from "set to the
    /// default".
    pub deployer_bin: String,
    /// The adapter binary each kind named for itself.
    pub kind_bins: BTreeMap<String, String>,
}
```

`bin_for_kind(kind)` applies the order. The resolved binary is what the lane
spawns, what the spawn failure names and what the evidence attributes to
when the adapter does not name itself.

The order is chosen so that one adapter can serve every kind when an
operator wants that: setting `FORGE_DEPLOYER_BIN` alone still serves `ssh`,
and setting `FORGE_DEPLOYER_SSH_BIN` overrides it for `ssh` only. Nothing
else changes for the kinds that name no adapter of their own — their
resolved binary is the shared binary, exactly as today.

## The Configuration Validation

`DeployConfig::from_manifest_meta` refuses a target that does not state the
fields its kind requires, and the refusal names the kind and the fields:

```
deployment target `vps` of kind `ssh` must declare `host` and `path`
```

The check sits next to the kind check, so a target is validated against its
kind in one place. The legacy `deployment.type` form cannot express a host
and therefore cannot express an `ssh` target: it is refused with the same
sentence, which is the honest answer rather than a later failure inside an
adapter.

## The Reference SSH Adapter

`adapters/ssh/forge-deployer-ssh` maps the executor contract onto an SSH
transfer. It contains no deployment logic of its own beyond the transfer and
the health check, and it uses the transport facts the publish lane already
uses: a staged copy and an atomic rename, so a half-copied artifact is never
visible on the host.

The apply path is three commands, in this order:

```text
ssh <target> /bin/mkdir -p <dirname of the remote path>
scp <local artifact> <target>:<remote path>.forge-new
ssh <target> /bin/mv -f <remote path>.forge-new <remote path>
```

The staged suffix and the two remote binaries are the publish lane's
constants, so the repository has one set of SSH facts.

The observe path runs the health check **on the host**, because the health
check describes the target's runtime:

| Health kind | Command on the host | Observation |
|---|---|---|
| `http` | `curl -fsS -o /dev/null <url>` | `running` on exit 0, `failed` on a non-2xx answer, `unknown` when the host cannot be reached |
| `docker` | `docker ps --filter name=<service> --format {{.Status}}` | `running` when a status contains `Up`, `failed` when the service is known and not up, `unknown` when the service is not visible |
| `process` | `pgrep -f <process>` | `running` on exit 0, `failed` otherwise |
| none | — | `unknown` ("no health check declared for this target") |

A host that cannot be reached is `unknown`, never `failed`: disconnected
means unknown, not offline proof. That is the deploy contract's own rule and
the adapter follows it.

The adapter reads the target's `host`, `user` and `path` from the stdin
payload, which the contract already delivers, and it never writes an
absolute host path into the evidence: the remote path is reported as
`<host-path>` and the local artifact as its project-relative name. Forge
redacts again on the way in.

Two variables make the adapter testable without a host:

| Variable | Purpose |
|---|---|
| `FORGE_SSH_BIN` | The `ssh` program. Default `ssh`. |
| `FORGE_SCP_BIN` | The `scp` program. Default `scp`. |

A test points them at a local script that records its argv, so the transfer's
argv, order and failure handling are all checkable without a network.

Two refusals the adapter owns. It refuses a kind it does not serve, the way
the reference Jenkins adapter refuses the kinds it does not serve, so a
misrouted invocation never becomes a transfer. And it refuses a declared
host, user or remote path that contains anything outside
`[A-Za-z0-9._+/=,@-]`, before anything runs: the remote side of a transfer is
a shell word on the far end and an ambiguous spec in `scp`, and a path with a
space in it cannot be addressed unambiguously by either.

## User Experience and Interface

No new command. No new manifest field. The visible change is that an `ssh`
target is served instead of refused.

Before, an `ssh` target was refused wherever it was read:

```
error[deploy-invalid]: deploy invalid: deployment target kind `ssh` is known but not
yet wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime`
```

After, the same manifest is served:

```yaml
deployment:
  artifact: docker-compose.yml
  default: vps
  targets:
    - name: vps
      kind: ssh
      host: home-server-01
      user: deploy
      path: /srv/app/docker-compose.yml
```

```text
$ forge deploy apply --target vps --confirm
stages:
  - apply target=`vps` delivered: artifact transferred to the host
      evidence: scp <artifact> <host>:<host-path>.forge-new
      evidence: ssh <host> /bin/mv -f <host-path>.forge-new <host-path>
      evidence: executor=forge-deployer-ssh/0.1.0@unknown
  - observe target=`vps` running: the host reports the service is up
      evidence: docker ps --filter name=app: Up 4 minutes
      evidence: executor=forge-deployer-ssh/0.1.0@unknown
```

And a target that does not state what its kind needs is refused by name:

```
error[deploy-invalid]: deploy invalid: deployment target `vps` of kind `ssh` must
declare `host` and `path`
```

## Behavioral Model

| Actor | Block | Boundary |
|---|---|---|
| A manifest naming an `ssh` target with a host and a path | The kind is accepted and the deploy reaches the SSH adapter. | |
| A manifest naming an `ssh` target without a host | The target is refused, naming the kind and the field. | The refusal happens before any adapter runs. |
| A manifest naming an `ssh` target through the legacy `type` form | The target is refused for the same missing fields. | The legacy form cannot state a host. |
| A manifest naming a kind that is not known | The kind is refused, naming the supported kinds. | Unchanged. |
| A manifest naming a kind that is known but not wired | The kind is refused as not yet wired. | No kind is in that state after this change; the sentence stays for the next one. |
| An operator with only `FORGE_DEPLOYER_BIN` set | Every kind, `ssh` included, is served by that binary. | One adapter can serve every kind. |
| An operator with `FORGE_DEPLOYER_SSH_BIN` set | The `ssh` kind is served by that binary; the other kinds keep the shared binary. | |
| An operator with nothing set | The `ssh` kind uses `forge-deployer-ssh`; the other kinds use `forge-deployer`. | |
| An adapter that names itself `forge-deployer-ssh/0.1.0` | The stage evidence attributes to that family and version. | Judged by the shared version rule. |
| A host that cannot be reached | The observation is `unknown`. | Disconnected is unknown, not offline proof. |
| A dry run against an `ssh` target | Nothing is copied and no host is contacted. | |
| A half-copied artifact | It is never visible on the host: the copy lands under a staged name and is renamed. | |

## Contract and Compatibility

The executor contract is unchanged. The argv, the envelope schema and the
classification table are untouched, so the discriminator stays
`forge-deploy-executor/0.1.0`. The `ssh` kind needs no new operation: apply
and observe are the operations a remote target has.

The manifest is unchanged. No field is added, and the `host`, `user` and
`path` fields keep their names and their meaning. What changes is that the
`ssh` kind now *depends* on two of them.

The kind vocabulary changes in one place: `ssh` moves from "known but not
wired" to "wired". The one refusal sentence for an unwired kind stays in the
code, because the next kind will need it.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The `ssh` kind is named without a host or a path | Refused with `deploy-invalid`, naming the kind and the fields. |
| The kind's adapter binary is missing | `deploy-target-unavailable`, naming the binary — the same as any other kind. |
| The adapter is invoked with a kind it does not serve | It refuses the kind, the way the reference Jenkins adapter refuses the kinds it does not serve. |
| A declared host, user or remote path holds a character outside `[A-Za-z0-9._+/=,@-]` | The adapter refuses it before anything runs, naming the value. |
| The adapter's envelope names a different contract | Refused by the shared version rule, naming both sides. |
| The host cannot be reached during apply | The stage is `failed` with the adapter's evidence; the previous state stands. |
| The host cannot be reached during observe | The observation is `unknown`; the last good `running` observation is kept. |
| The remote directory does not exist | It is created by the adapter's `mkdir -p` step. |
| The copy fails half way | The staged file is left, the final path is untouched, and the stage is `failed`. |
| The evidence contains a credential or an absolute host path | The adapter scrubs first; Forge redacts again. |
| A dry run | No host is contacted and no file is copied. |

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

- an `ssh` target with a host and a path reaches the adapter;
- an `ssh` target without a host is refused by name;
- the adapter for a kind is resolved in the stated order;
- the SSH adapter copies to a staged name and renames it;
- the SSH adapter's health vocabulary maps to `running`, `failed` and
  `unknown`;
- a host that cannot be reached observes `unknown`;
- a dry run contacts no host.

## Implementation Plan

**The registry.** `src/extension/kinds.rs` gains the three fields, the
`ssh` entry's values, and the two accessors the deploy surface reads:
`required_target_fields(kind)` and `default_adapter_for_kind(kind)`.

**The resolution.** `src/deploy/model.rs` gives `DeployAdapterConfig` the
`kind_bins` field, the `from_env` that reads each kind's variable, and
`bin_for_kind`. `src/deploy/engine/stages.rs` resolves the binary once per
stage and uses it for the spawn, the failure messages and the attribution.

**The validation.** `src/deploy/model.rs` refuses a target that does not
state its kind's required fields, next to the kind check.

**The adapter.** `adapters/ssh/forge-deployer-ssh` and
`adapters/ssh/ssh-adapter.md`.

**The docs.** `docs/adapter-contracts/deploy-executor.md` records the wired
kinds, the adapter resolution, the SSH target fields and the SSH argv.
`src/deploy/mod.rs` and `src/core/manifest.rs` stop saying that `ssh` is
planned.

**The tests.** The shared contract suite covers the new kind automatically;
new tests cover the refusal, the resolution order, the transfer's argv and
the health vocabulary.

## Decision Ledger

- **Resolved:** the transfer lives in the adapter, not in Forge Core.
  `requirement.md` §30 says deployment is adapter-based, and the publish
  lane's `SshTransport` is a publish-lane contract with its own scope.
- **Resolved:** the `ssh` kind requires `host` and `path`. A target without a
  host is not a target; a target without a remote path has nowhere to put
  the artifact. `user` is optional because the SSH client has a defined
  default for it.
- **Resolved:** the requirement lives in the registry, not in a validator.
  A validator that hard-codes "ssh needs a host" is the same divergence the
  kind registry was created to remove, one field later.
- **Resolved:** the adapter resolution is per kind, in the order "the kind's
  own variable, the shared variable, the kind's default". The shared
  variable stays in the order so that one adapter can serve every kind.
- **Resolved:** `deployer_bin` becomes empty when the shared variable is
  unset. The resolution has to tell "unset" from "set to the default", and
  the field is only read in the deploy lane.
- **Resolved:** the executor contract does not change. The `ssh` kind needs
  no new operation, so the discriminator stays `forge-deploy-executor/0.1.0`.
- **Resolved:** the health check runs on the host, because the health check
  describes the target's runtime. The adapter's document states what the
  host needs (`curl`, `docker`, `pgrep`).
- **Resolved:** the remote path is copied under a staged name and renamed,
  using the publish lane's constants, so a half-copied artifact is never
  visible on the host.
- **Resolved:** the adapter refuses a kind it does not serve, and refuses a
  declared host, user or path that holds a character outside
  `[A-Za-z0-9._+/=,@-]`. The remote side of a transfer is a shell word on
  the far end and an ambiguous spec in `scp`; a value that cannot be
  addressed unambiguously is refused, not escaped.
- **Resolved:** a legacy `deployment.type: ssh` target is refused for the
  missing fields. The legacy form cannot state a host, and a later failure
  inside an adapter would be a worse answer.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `extension-point-contract` landed the port this change plugs into.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| The `ssh` deploy target is a served kind | The kind moves from known-but-unwired to wired | The same manifest is served instead of refused | The registry owns the wired flag | `src/extension/kinds.rs` | A served target; an unwired kind stays refused | 2.1–2.3, 3.1 | Unit tests, a real run |
| A kind declares the target fields it needs | The requirement lives next to the kind | A target that does not state them is refused by name | The registry carries the requirement; the validator reads it | `src/extension/kinds.rs`, `src/deploy/model.rs` | A missing field; the legacy form | 2.4, 3.2–3.3 | Unit tests, a real run |
| A kind may name its own adapter | The adapter is resolved per kind | An operator can install a Jenkins adapter and an SSH adapter | One order: the kind's variable, the shared variable, the kind's default | `src/deploy/model.rs`, `src/deploy/engine/stages.rs` | Each rung of the order | 2.5–2.6, 3.4–3.5 | Unit tests |
| The reference SSH adapter ships the artifact and observes the host | A staged copy and an atomic rename, and a host-side health check | The stage evidence names the copy and the rename | The publish lane's SSH facts, reused | `adapters/ssh/` | A delivered copy; an unreachable host; a dry run | 2.7–2.8, 3.6–3.9 | The adapter's own tests, a real run |
