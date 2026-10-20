# Change: deploy-target-ssh

## Why

`requirement.md` §30 names "SSH server" as an initial deployment target and
says "Deployment should be adapter-based." The `ssh` deploy target kind is
therefore not an optional extra — it is a target the product promised and
Forge has never served.

The `extension-point-contract` change gave the repository one extension
contract: one version rule, one state vocabulary, one owner for the deploy
target kinds and one contract test every kind runs. The `ssh` kind is the
first branch that plugs into it, and three things are still missing.

**The kind is refused.** The shared registry knows `ssh` and marks it as not
wired, so every entry point refuses it in one sentence. That is honest, but it
means the promised target does not exist.

**A kind cannot say what it needs.** The manifest's target entry carries
`host`, `user`, `path` and `service`, and nothing validates any of them:
`host` and `user` are never read, `path` and `service` are read only by the
Compose health check. An `ssh` target with no host is not a target, and the
refusal should say so at validation time instead of failing later inside an
adapter.

**One adapter serves every kind.** `DeployAdapterConfig` resolves a single
binary from `FORGE_DEPLOYER_BIN` for every kind. A project that deploys to a
Jenkins job and to a plain host cannot be served: the reference Jenkins
adapter refuses the kinds it does not know, so the operator has to choose
which adapter to install and lose the other target.

The three gaps are the same gap. A branch is a specialisation behind a
declared extension point, and an extension point that cannot declare its own
requirements or own its own implementation is a switch, not a port.

## What Changes

- **The `ssh` kind becomes a served kind.** The shared registry marks it wired
  and the one refusal sentence stops naming it. The shared contract test
  covers it automatically; nothing restates the rules for it.
- **A kind declares the target fields it needs.** The registry entry carries
  the fields a target of that kind must state, and the configuration
  validator refuses a target that does not state them, naming the kind and
  the fields. The `ssh` kind requires `host` and `path`. The mechanism is
  general, so the next branch inherits it.
- **A kind may name its own adapter.** The registry entry carries the kind's
  adapter variable and its default binary, and the deploy lane resolves the
  adapter per kind: the kind's own binary, then the shared binary, then the
  kind's default. A Jenkins adapter and an SSH adapter can both be
  installed.
- **A reference SSH adapter ships.** `adapters/ssh/forge-deployer-ssh` maps
  the executor contract onto an SSH transfer, using the transport facts the
  publish lane already uses: a staged copy and an atomic rename, so a
  half-copied artifact is never visible on the host. `adapters/ssh/ssh-adapter.md`
  records the facts.
- **The deploy executor's documentation states the new facts**: the wired
  kinds, the adapter resolution, the SSH target fields and the SSH argv.
- **No other extension surface moves.** The manifest shape, the plugin
  registry, the other kinds, the release and delivery lanes, the publish
  lane and every sibling module are untouched.

## Package Boundary

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `deploy-target-ssh` | forge | Rust + Python | implemented | this package |
| `adapter-deployment` | forge | Rust | modified | the deploy contract's requirement text and purpose |
| `extension-point-contract` | forge | Rust | read only | the shared contract this change plugs into |
| `platform-contract-consumption` | its owner | contract | read only | |
| `forge-publish-plugin-orchestration` | its owner | Rust | read only | the publish lane's SSH transport is not touched |
| Every sibling module | its owner | — | read only | not edited |

## Non-Goals

- No SSH transport inside Forge Core. The transfer belongs to the adapter,
  because `requirement.md` §30 says deployment is adapter-based and because
  the publish lane's `SshTransport` is a publish-lane contract with its own
  scope.
- No key management, no agent installation, no remote command execution
  beyond the artifact transfer and the health check.
- No change to the executor contract's argv, envelope schema or
  classification table. The `ssh` kind needs no new operation, so the
  discriminator stays `forge-deploy-executor/0.1.0`.
- No new manifest field. The `host`, `user` and `path` fields already exist;
  this change makes the `ssh` kind depend on them.
- No convergence of the other extension surfaces onto the shared contract.
  That is a later change, one surface at a time.
