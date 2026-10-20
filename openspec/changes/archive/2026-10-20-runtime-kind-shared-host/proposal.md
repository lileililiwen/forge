# Change: runtime-kind-shared-host

## Why

The publish inventory's contract already carries a runtime vocabulary —
`"runtime": "web|worker|job|library"` — and the vocabulary says what a project
*is* without ever saying how it is *hosted*. The repository's own instance of
that missing shape is `modules/dotnet-platform-libs`: 80 independently
adoptable .NET packages that all run inside **one** CLR host inside the
consuming application, declared `role: platform` and `deployable: false`.

The fleet judged a module by its Compose file. An entry was published when it
was `compose_ready` — when a Compose file existed at the declared source
path — so a `library` entry that happened to carry one was started as its own
Mac Docker workload, and the host that owns the module never learned about
it. A module read as a service that failed to start. The vocabulary also had
no owner: `RuntimeClass` was a private type in the publish lane, so no other
surface could read it and nothing validated it against a shared owner.

## What Changes

- **Forge Core gains one runtime-kind vocabulary.** A new `src/runtime/`
  module owns `RuntimeKind` — `web`, `worker`, `job`, `library` — and the
  **host model** each kind declares: an **own-process** kind runs as a
  process of its own, and a **shared-host** kind is a module loaded into a
  host another project owns, where several modules can share one host. The
  `library` row is the shared-host row.
- **The publish inventory converges onto it.** The private `RuntimeClass` is
  deleted; the inventory's model, pipeline, report and tests read the shared
  type. The contract's spelling (`web|worker|job|library`) does not change —
  only its owner moves.
- **The fleet judges a module by its host model, not by its compose file.** A
  shared-host entry is never started as a workload of its own and never
  receives a public route, whatever its compose file says, and the report
  names the host model and the reason. Every other entry is published exactly
  as before.
- **The observe lane reads the same rule.** `forge fleet online` probed every
  compose-ready entry, so a module the fleet never started would read
  `NO-ROUTE` or `NOT-DEPLOYED` — a false alarm about a workload that does not
  exist. The rule now lives in one function that both lanes call, and a
  module is reported as skipped with the reason that names the host that owns
  it.
- **The report names the host model.** Every fleet row carries it, so an
  operator can see at a glance which entries are services and which are
  modules inside somebody else's host. A row whose runtime kind could not be
  read names neither — it carries the refusal, not a placeholder.
- **The public routing rule is stated in terms of the host model.** Only an
  own-process kind may declare a public HTTP port; a shared-host module may
  not, and the refusal says so in the host model's words.

## Package Boundary

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `runtime-kind-shared-host` | forge | Rust | implemented | this package |
| `forge-independent-project-inventory-fleet` | forge | Rust | modified | the fleet's publishing rule and its report |
| `fleet-liveness-status` | forge | Rust | modified | the observe lane reads the same rule |
| `decoupled-remote-publish` | forge | Rust | modified | the roster filter and the journal rule it states |
| `extension-point-contract` | forge | Rust | read only | the shared extension contract |
| `forge-project-inventory/0.1.0` | forge | contract | unchanged | the document's shape and spelling do not change |
| `forge-fleet-liveness/0.1.0` | forge | contract | additive | the report gains a `host_model` field per entry |
| `modules/dotnet-platform-libs` | its own workspace | .NET | read only | the repository's instance of the shared-host kind; not edited |
| The publish providers, the release lane, the deploy lane, the profile catalog, the manifest contract | forge | Rust | read only | not edited |
| Every sibling module | its owner | — | read only | not edited |

## Non-Goals

- No new manifest field. The manifest's `runtime:` block keeps its shape.
- No change to the inventory document's shape, its spellings, or its contract
  version. The vocabulary moves owner; the contract does not move.
- No .NET code. The shared-host kind is a vocabulary row and a fleet rule;
  the .NET workspace that instantiates it is not edited.
- No change to the deploy lane, the release lane, the profile catalog, the
  plugin registry or any other extension surface.
- No convergence of the other runtime-shaped vocabularies — the manifest's
  `runtime.language` and the registry's runtime string stay where they are.
  Each converges in its own change.
