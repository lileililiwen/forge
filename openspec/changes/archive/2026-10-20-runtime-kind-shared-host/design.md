## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The runtime-kind vocabulary and the host model | `forge` | Yes. `src/runtime/`. |
| The publish inventory's runtime type, its parser, its report | `forge` | Yes. `src/publish/inventory/`. |
| The fleet's selection rule and its counts | `forge` | Yes. `src/cli/fleet.rs`. |
| The fleet's liveness verdicts and their roster | `forge` | Yes. `src/fleet/online.rs`, `src/cli/inventory.rs`. |
| The inventory document's shape, its spellings and its contract version | `forge` | No. Read only. |
| The publish providers, the release lane, the deploy lane | `forge` | No. Read only. |
| The profile catalog, the manifest contract, the plugin registry | `forge` | No. Read only. |
| `modules/dotnet-platform-libs` and the other sibling modules | their owners | No. Read only. |

## The Vocabulary

`src/runtime/` owns one runtime-kind type and one host model.

```rust
/// How a project's artifact is hosted.
pub enum HostModel {
    /// The artifact is a process of its own, with its own container
    /// and its own deploy target.
    OwnProcess,
    /// The artifact is a module loaded into a host another project
    /// owns. Several modules can share one host.
    SharedHost,
}

/// The kind of runtime a project's artifact is.
pub enum RuntimeKind {
    Web,
    Worker,
    Job,
    Library,
}
```

The four values and their spellings are the ones the inventory contract
already carries, so the vocabulary moves owner and nothing else moves. The
parsing stays the same too: trimmed, case-insensitive, and refused by name
when it is not one of the four.

The table is the vocabulary's front door, and each row says what the kind is
and how it is hosted:

| Kind | Host model | What it is |
|---|---|---|
| `web` | OwnProcess | A public HTTP service with a host process of its own. |
| `worker` | OwnProcess | A long-running background worker with a host process of its own. |
| `job` | OwnProcess | A short-lived job with a host process of its own. |
| `library` | **SharedHost** | A module loaded into a host another project owns. Several modules can share one host. |

`RuntimeKind::host_model()` answers the row, and the shared parser returns
the typed error the rest of Forge uses, so an unknown runtime is refused with
the accepted values named.

## The Convergence

`src/publish/inventory/model.rs` owns a private `RuntimeClass` today, with
its own `as_str`, its own `parse` and its own `Display` in
`runtime_display.rs`. Nothing outside the publish lane can read it.

The change deletes it. `InventoryEntry.runtime` and
`InventoryFleetEntry.runtime` become `crate::runtime::RuntimeKind`; the
`Display` impl moves with the type; the pipeline uses the shared parser.

The reason a shared-host artifact carries moves with the vocabulary too. It
is a fact about the host model, not about the publish lane, so
`crate::runtime::SHARED_HOST_REASON` owns the wording and the publish
inventory re-exports it. One wording, so the publish lane and the observe
lane say the same thing.

The inventory *document* does not change. Its contract string stays
`forge-project-inventory/0.1.0` and its `runtime` field keeps the spelling
`web|worker|job|library`. A consumer of the document sees the same bytes and
the same values. Only the owner of the vocabulary moves, from a private type
in one lane to a shared module in Forge Core.

The pipeline's refusal changes shape, because the shared parser returns the
shared error type. The refusal still names the entry and the accepted
values:

```
entry `app` runtime `svc` is not one of web|worker|job|library
```

## The Fleet Rule

Today the rule is about the compose file:

> only `compose_ready` entries invoke the provider

A `compose_ready` entry is one whose declared Compose file exists at the
declared source path. Under that rule a `library` entry that happens to carry
a Compose file is started as a Mac Docker workload of its own — and the host
that owns the module never learns about it.

The rule becomes about the host model:

> an entry is published when it is compose-ready **and** its runtime kind is
> an own-process kind

A shared-host entry is reported and never published, whatever its compose
file says. Its reason says why:

```
a library is a module; it is hosted by the project that adopts it, so the
fleet never starts it
```

The other classifications are untouched. A `web`, `worker` or `job` entry
that is compose-ready is published exactly as before, including the
non-web own-process kinds, which the canonical capability explicitly
protects.

The counts the CLI prints change with the rule, and the message changes with
them: the fleet no longer says "0/N compose_ready" when it means "0/N
publishable". It says how many entries were publishable and how many were
held back, and it names the host model that held them back.

## The Observe Rule

The fleet has a second surface that reads the same roster: `forge fleet
online` probes the target and verdicts each entry `ONLINE`, `DOWN`,
`NO-ROUTE` or `NOT-DEPLOYED`.

That surface probes `compose_ready` entries. Under the new publishing rule
that would be a false alarm: a compose-ready `library` entry is a module the
fleet never started, so probing it would report `NO-ROUTE` or `NOT-DEPLOYED`
for a workload that does not exist.

The observe lane therefore reads the **same rule** from the same place. The
rule lives in one function — `forge::publish::inventory::is_publishable` —
and both surfaces call it:

- the publish lane asks whether an entry may be published;
- the observe lane asks whether an entry may be probed.

An entry the fleet never published is reported with its classification and
its host model, verdict `SKIPPED`, and the shared-host reason. It is never
probed.

## The Report

Every fleet row carries its host model, so an operator can tell a service
from a module without reading the reason:

```json
{
  "id": "platform-modules",
  "runtime": "library",
  "host_model": "shared_host",
  "classification": "compose_missing",
  "reason": "a library is a module; it is hosted by the project that adopts it, so the fleet never starts it"
}
```

A row whose runtime kind could not be read names neither. The pipeline used
to fill in a placeholder kind there, which read as a claim the entry never
made — a refused entry looked like a module, hosted by someone else, for a
reason that had nothing to do with the refusal. The row now carries the
reason and nothing else:

```json
{
  "id": "platform-modules",
  "classification": "invalid",
  "reason": "entry `platform-modules` runtime `svc` is not one of web|worker|job|library"
}
```

The public routing rule is stated in the host model's words. Only an
own-process kind may declare a public HTTP port; a shared-host module may
not, and the refusal says so:

```
entry `platform-modules` declares public_http but runtime `library` is a
shared-host module; only an own-process runtime may expose HTTP
```

The rule's behaviour is unchanged — only a `web` entry could declare
`public_http` before, and only a `web` entry can now. The vocabulary it is
stated in is the shared one, and the reason names the host model.

## User Experience and Interface

No new command. No new field on the inventory document. The visible change is
in the fleet's report and its refusals.

Before, a module read like a broken service:

```json
{
  "id": "platform-modules",
  "runtime": "library",
  "classification": "compose_missing",
  "reason": null
}
```

After, it reads like what it is:

```json
{
  "id": "platform-modules",
  "runtime": "library",
  "host_model": "shared_host",
  "classification": "compose_missing",
  "reason": "a library is a module; it is hosted by the project that adopts it, so the fleet never starts it"
}
```

And the fleet's summary names the two counts:

```
fleet publish: 3/5 publishable in inventory source `local`; 2 held back (a shared-host module is never started by the fleet)
```

## Behavioral Model

| Actor | Block | Boundary |
|---|---|---|
| A `web`, `worker` or `job` entry that is compose-ready | It is published, exactly as before. | The non-web own-process kinds are still published. |
| A `library` entry that is compose-ready | It is reported and never published. | The host that owns the module owns its lifecycle. |
| A `library` entry with no compose file | It is reported as a shared-host module, with the reason. | It was already never published. |
| An entry that declares `public_http` with a non-`web` runtime | Refused, naming the host model. | The behaviour is unchanged; the vocabulary is shared now. |
| A `library` entry with a public port | Refused. | |
| An inventory document with an unknown runtime value | Refused, naming the entry and the accepted values. | The wording is unchanged. |
| A sibling surface that needs a runtime kind | It reads `crate::runtime::RuntimeKind`. | The private type is gone. |
| `forge fleet online` reads a compose-ready `library` entry | It is reported as `SKIPPED` with the shared-host reason and never probed. | The observe rule is the publish rule. |
| An entry whose runtime kind could not be read | The row names no runtime kind and no host model, and carries the refusal. | The row says only what is true. |

## Contract and Compatibility

The inventory document is unchanged: the same contract string, the same
field names, the same four spellings, the same meaning for each. A consumer
of `forge-project-inventory/0.1.0` sees the same bytes.

The fleet's *report* changes: it gains a `host_model` field per row, and the
reason for a shared-host entry is no longer null. A row whose runtime kind
could not be read names no runtime kind and no host model — the keys are
absent rather than filled with a placeholder. The report is Forge's own
output, not the document, so this is additive.

The liveness report changes the same way: it gains a `host_model` field per
entry. `forge-fleet-liveness/0.1.0` keeps its version — the field is
additive and every existing field keeps its value — and the entry's verdict
for an entry the fleet never published becomes `SKIPPED` with the
shared-host reason as detail, where it used to read `NO-ROUTE` or
`NOT-DEPLOYED`.

The publishing rule changes in one case: a compose-ready `library` entry is no
longer started as a workload. That is the change's point, and it is the only
behavioural difference.

The canonical capability `forge-independent-project-inventory-fleet` is
modified: its "Container fleet publishing" requirement stated the old rule
("every valid Compose-ready inventory entry ... including non-web runtime
classes"), and its "Complete inventory reporting" requirement now names the
host model.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The inventory names a runtime that is not one of the four | The entry is refused, naming the entry and the accepted values. |
| The inventory names no runtime at all | The entry is refused, naming the entry and the four values. |
| A `library` entry is compose-ready | It is reported and never published. |
| A `library` entry is compose-ready and the fleet observes it | It is reported as `SKIPPED` with the shared-host reason and never probed. |
| A `library` entry declares `public_http` | Refused, naming the host model. |
| A `web` entry declares `public_http` without a port | Refused, unchanged. |
| A `web` entry declares `public_http` with a port | It is routed, unchanged. |
| A `worker` or `job` entry is compose-ready | It is published without a public route, unchanged. |

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

- a `library` entry is never published, whatever its compose file says;
- a `library` entry is never probed, whatever its compose file says;
- a `web`, `worker` or `job` entry is published exactly as before;
- every fleet row names its host model;
- a shared-host entry's reason names the host and the rule;
- the public-routing refusal names the host model;
- an unknown runtime is refused with the accepted values;
- the inventory document's shape and spellings are unchanged.

## Implementation Plan

**The vocabulary.** A new `src/runtime/` module: `RuntimeKind`, `HostModel`,
the kind table, the shared parser, the shared-host reason and the `Display`
impl, with its own test suite.

**The convergence.** `src/publish/inventory/model.rs` drops `RuntimeClass`
and reads the shared type; `runtime_display.rs` is deleted with it; the
pipeline uses the shared parser.

**The rule.** `src/publish/inventory/model.rs` owns the rule in one function
— `is_publishable` — and its `compose_ready()` iterator (now
`publishable()`) and `src/cli/fleet.rs`'s eligibility filter both call it;
the report carries the host model and the reason.

**The observe rule.** `src/fleet/online.rs` and `src/cli/inventory.rs` read
the same `is_publishable` function, so the observe lane never probes an
entry the fleet never published. The liveness unit suite moves to
`src/fleet/online_tests.rs` so the surface itself stays under the
repository's 1000-line source cap.

**The counts.** `src/cli/fleet.rs` reports how many entries were publishable
and how many were held back, and names the rule that held them back.

**The documentation.** `src/publish/inventory/mod.rs` states the host model
and the rule; `src/cli/commands.rs` and `src/cli/constants.rs` state the rule
in the operator-facing help.

**The tests.** The runtime module's own suite; the inventory's suite; the
fleet CLI's suite; the liveness contract suite.

## Decision Ledger

- **Resolved:** the shared vocabulary keeps the inventory contract's four
  values and their spellings. The contract does not move, so no consumer
  breaks and the contract version stays `forge-project-inventory/0.1.0`.
- **Resolved:** the shared-host kind is `library`. It is the only kind in the
  vocabulary that is a module rather than a process, and it is the shape the
  repository's .NET platform library set already has.
- **Resolved:** the fleet's rule becomes "compose-ready **and** own-process",
  not "compose-ready". A compose file is a deployment fact; the host model is
  a lifecycle fact, and only the second one decides who starts the workload.
- **Resolved:** the report gains the host model and the reason. The
  classification vocabulary does not change: `compose_missing` still means
  "no compose file at the declared source path", which is true of a module
  and is a fact rather than a verdict.
- **Resolved:** the public-routing refusal changes wording, not behaviour.
  Only a `web` entry could declare `public_http` before, and only a `web`
  entry can now.
- **Resolved:** the .NET workspace is not edited and no .NET code ships. The
  shared-host kind is a vocabulary row and a fleet rule; the .NET module
  that instantiates it has its own governance and its own verification.
- **Resolved:** the manifest's `runtime:` block, the registry's runtime
  string and the profile catalog are not touched. Each converges in its own
  change, one surface at a time.
- **Resolved:** the observe lane reads the same rule from the same place. The
  publish rule lives in one function, `forge::publish::inventory::is_publishable`,
  and the liveness surface calls it. A module the fleet never started is
  never probed, so `forge fleet online` cannot report a false alarm.
- **Resolved:** a row whose runtime kind could not be read names no runtime
  kind and no host model. The pipeline used to fill in a placeholder kind
  there; with a host model attached, that placeholder read as a lifecycle
  claim the entry never made. The keys are absent instead.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `extension-point-contract` landed the shared contract and `deploy-target-ssh`
  was the first branch to plug into it.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| One runtime-kind vocabulary with a host model | Forge Core owns the kinds and says how each is hosted | An operator reads the kind and its host model | The `library` row is the shared-host row | `src/runtime/` | Every kind; an unknown value | 2.1–2.4, 3.1–3.2 | The runtime module's suite |
| The publish inventory converges onto it | One vocabulary, one owner | The document's bytes do not change | The owner moves, the contract does not | `src/publish/inventory/` | The same document, parsed | 2.5–2.7, 3.3 | The inventory's suite |
| The fleet judges a module by its host model | A module is never started by the fleet | The report names the model and the reason | Compose-ready **and** own-process | `src/publish/inventory/model.rs`, `src/cli/fleet.rs` | A library entry; an own-process entry | 2.8–2.9, 3.4–3.6 | The fleet CLI's suite |
| The observe lane reads the same rule | A module the fleet never started is never probed | `forge fleet online` reports it as skipped | One rule, one function, two callers | `src/fleet/online.rs`, `src/cli/inventory.rs` | A compose-ready module; an own-process entry | 3.9–3.10 | The liveness contract suite |
| A refused entry names no runtime kind | The row says what is true | The reason is the whole truth of the row | Absent keys, not a placeholder | `src/publish/inventory/pipeline.rs` | An entry with an unknown runtime | 3.11 | The inventory's suite |
