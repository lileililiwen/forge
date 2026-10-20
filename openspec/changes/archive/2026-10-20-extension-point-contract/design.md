## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The extension contract: the version rule, the state vocabulary, the descriptor | `forge` | Yes. `src/extension/`. |
| The deploy target kind vocabulary and its one refusal wording | `forge` | Yes. `src/extension/kinds.rs`. |
| The shared contract verification every kind runs | `forge` | Yes. `src/extension/contract.rs` and `src/extension/extension_tests.rs`. |
| The deploy executor boundary: the declared contract and the adapter's self-identification | `forge` | Yes. `src/deploy/engine/model.rs`, `src/provider/runner.rs`, `src/provider/matrix.rs`. |
| The deploy surface's kind validation and the plan's kind refusal | `forge` | Yes. `src/deploy/prepare.rs`, `src/deploy/model.rs`, `src/deploy/engine/stages.rs`. |
| The deploy executor's wire format, its argv and its timeout | `forge` | No. Read only. Only the rule that judges the declared version changes. |
| The manifest contract and `project.schema.json` | `forge` | No. Read only. No field is added. |
| The plugin registry, its descriptor and its discovery order | `forge` | No. Read only. The plugin kinds converge in a later change. |
| The publish, governance, identity, analytics and evidence providers | `forge` | No. Read only. Each converges in a later change. |
| The standard packs, the identity packs, the kits, the components and the UI patterns | `forge` | No. Read only. Each converges in a later change. |
| The `platform-contracts` contract registry | its owner | No. Read only. |
| Every sibling module | its owner | No. Not edited. |

## Why a Port and Not a Helper

The repository has nine extension boundaries and no shared contract between them.
Each one invented its own descriptor, its own version rule and its own state
vocabulary. The consequence is that a newer extension is refused exactly like an
unrelated one, because every version check in the repository is exact string
equality.

The deploy executor is the one boundary that already behaves like a port: an
external process declares the contract it speaks, and Forge checks it. It is
therefore the first thing to migrate, and the shared contract is built around
what that boundary already needs:

- a way to **read** a declared contract version;
- a way to **judge** it against a required one;
- a way to **report** the outcome in one vocabulary;
- a way to **test** a new kind against the rules.

The other eight surfaces converge onto the same module afterwards, one at a
time. This change does not migrate them.

## The Contract Version

`src/extension/version.rs` owns one type:

```rust
pub struct ContractVersion {
    pub family: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
```

A declared value is read as `family/major.minor.patch`. The bare
`major.minor.patch` form carries no family and is therefore unreadable as a
contract: it is refused, which preserves the deploy executor's existing rule
that the pre-namespacing `0.1.0` can never masquerade as conformant.

Reading fails, with the offending value named, when the family is empty, when
the family contains whitespace, when the version does not have exactly three
numeric parts, or when any part is not a number.

### The One Rule

```rust
pub fn is_compatible_with(&self, required: &ContractVersion) -> bool
pub fn incompatibility_detail(&self, required: &ContractVersion) -> Option<String>
```

A declared version satisfies a requirement when **the families are equal and
the majors are equal**. The minor and the patch are reported but do not decide.

| Declared | Required | Verdict | Detail |
|---|---|---|---|
| `forge-deploy-executor/0.1.0` | `forge-deploy-executor/0.1.0` | compatible | — |
| `forge-deploy-executor/0.2.0` | `forge-deploy-executor/0.1.0` | compatible | — |
| `forge-deploy-executor/0.1.9` | `forge-deploy-executor/0.1.0` | compatible | — |
| `forge-deploy-executor/1.0.0` | `forge-deploy-executor/0.1.0` | incompatible | the major versions differ (1 and 0) |
| `forge-deployer/0.1.0` | `forge-deploy-executor/0.1.0` | incompatible | the families differ (`forge-deployer` and `forge-deploy-executor`) |
| `0.1.0` | `forge-deploy-executor/0.1.0` | unreadable | the version `0.1.0` does not name a contract family; expected `family/major.minor.patch` |

Why the major and not the minor: a major bump is a deliberate break of the wire
format, and a minor bump is an addition the old reader can ignore. The rule
says exactly that, and nothing else. A repository with no negotiation at all
cannot distinguish the two, so it refuses both.

## The State Vocabulary

`src/extension/state.rs` owns one type:

```rust
pub enum ExtensionState {
    Ready,
    Disabled { reason: String },
    Unavailable { reason: String },
    Invalid { reason: String },
}
```

It is the plugin surface's `PluginState`, promoted from one surface's private
type to the repository's, with one change: every non-ready state carries a
reason, because the contract says a refusal names the extension and the reason.

| State | Meaning |
|---|---|
| `ready` | enabled, well-formed, compatible and executable here |
| `disabled` | switched off by the operator |
| `unavailable` | the declared command is not executable on this host |
| `invalid` | the declaration is malformed, or names a version that does not satisfy the requirement |

The descriptor is the shape every extension's declaration takes:

```rust
pub struct ExtensionDescriptor {
    pub id: String,
    pub contract: ContractVersion,
    pub enabled: bool,
}
```

and the shared judgement is one function:

```rust
pub fn state(&self, required: &ContractVersion, executable: bool) -> ExtensionState
```

A disabled extension is `disabled`. An extension that cannot run here is
`unavailable`. An incompatible or malformed declaration is `invalid`, and the
reason names the extension and the mismatch:

```
forge-deployer-jenkins declares the contract `forge-deploy-executor/1.0.0`, which is
incompatible with the required `forge-deploy-executor/0.1.0`: the major
versions differ (1 and 0)
```

## The Deploy Target Kind Registry

`src/extension/kinds.rs` owns the list. Today the list is spread over four
places with two different contents, and `ssh` is advertised by one validator,
accepted by another and refused by a third.

```rust
pub struct DeployTargetKind {
    pub kind: &'static str,
    /// False when the kind is known but not wired into this build.
    pub wired: bool,
    pub summary: &'static str,
}

pub const DEPLOY_TARGET_KINDS: &[DeployTargetKind] = &[ ... ];
```

| Kind | Wired | Summary |
|---|---|---|
| `local` | yes | The adapter runs on this host. |
| `docker-compose` | yes | The adapter drives a Compose project. |
| `jenkins` | yes | The adapter drives a Jenkins job. |
| `mac-runtime` | yes | The adapter runs here and sends runtime operations to the target host. |
| `ssh` | no | The SSH host adapter is a later change. |

One function answers the kind's state, and one renderer answers the list:

```rust
pub fn deploy_target_kind_state(kind: &str) -> ExtensionState
pub fn supported_target_kinds() -> String
```

The refusal wording exists once:

| Case | Wording |
|---|---|
| unknown kind | `deployment target kind `X` is not supported; expected one of `local`, `docker-compose`, `jenkins`, and `mac-runtime`` |
| known but unwired kind | `deployment target kind `ssh` is known but not yet wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime`` |

Both name the same list, rendered by the same function, so the manifest, the
configuration and the plan cannot drift apart again.

### What Changes for `ssh`

Today `ssh` is **accepted** by the configuration validator and refused later
by the plan and by the observe path, in two different sentences, under two
different error codes. After this change it is refused at every entry point,
in one sentence, under one error code.

The error code changes from `deploy-target-unavailable` to `deploy-invalid`
for the plan and observe paths. A kind that this build does not implement is
an invalid declaration, not an unreachable target; the plan and the observe
path now say the same thing the configuration validator says.

## The Deploy Executor Boundary

`src/deploy/engine/model.rs` judges the adapter's envelope with the shared
rule instead of with string equality.

Before:

```rust
if contract != DEPLOY_EXECUTOR_CONTRACT { /* refused */ }
```

After:

```rust
let required = deploy_executor_contract();
let declared = ContractVersion::parse(&contract)?;
let descriptor = ExtensionDescriptor { id, contract: declared, enabled: true };
match descriptor.state(&required, true) { ... }
```

The required contract stays where the deploy surface reads it —
`DEPLOY_EXECUTOR_CONTRACT` is still `forge-deploy-executor/0.1.0` — and gains a
parsed form next to it.

The refusal names the extension, both contracts and the reason:

```
forge-deployer-jenkins declares the contract `forge-deploy-executor/1.0.0`, which is
incompatible with the required `forge-deploy-executor/0.1.0`: the major
versions differ (1 and 0)
```

### The Second Reader of the Same Contract

The provider probe (`src/provider/runner.rs`) runs the same deploy adapter as
a dry run and checks the same envelope's `contract` field with its own exact
string comparison. It is the same boundary, so it is the same rule: a probe
against an adapter at a newer minor of the same family records `supported`,
and a probe against a different family or a different major records
`unavailable` naming the mismatch.

Leaving that check on string equality would give the boundary two rules — the
deploy lane accepting an adapter the probe refuses — which is the exact class
of divergence this change exists to remove.

### The Adapter's Self-Identification

The envelope's optional `source` (`forge-deployer-jenkins/0.1.0`) is read
with the same parser into a family and a version, and the stage evidence
records both:

```
executor=forge-deployer-jenkins/0.1.0@cafe1234
```

A source that is present but cannot be read is a malformed self-identification
and the adapter is refused. A source that is absent is not a claim and is
still allowed: the evidence attributes to the configured binary with an
`unknown` revision, exactly as today.

The asymmetry is deliberate. An absent source says "I do not claim an
identity". A malformed source claims an identity Forge cannot check, which is
worse than no claim at all.

## The Shared Contract Verification

`src/extension/contract.rs` holds the one verification every kind runs:

```rust
pub fn verify_extension(
    descriptor: &ExtensionDescriptor,
    required: &ContractVersion,
    executable: bool,
) -> ExtensionState
```

`src/extension/extension_tests.rs` holds the suite that runs it over every
registered kind, so a new kind is held to the rules by the next test run
without anyone restating them. The suite covers the state vocabulary and the
version rule:

- a wired kind is `ready`;
- an unwired kind is `invalid` with the shared wording;
- a disabled declaration is `disabled`;
- a non-executable declaration is `unavailable`;
- a newer minor of the same family is compatible;
- a different major, a different family and an unreadable version are not.

The deploy surface adds its own test that every kind in the registry is
accepted or refused by the configuration validator with the shared wording, so
the two lists cannot drift apart.

## User Experience and Interface

No new command, no new field, no new output section. The visible change is
in the refusals.

An adapter that speaks a newer minor of the same contract is accepted. Before,
it was refused exactly like a stranger:

```
error[deploy-target-unavailable]: deploy target unavailable: deploy adapter contract
`forge-deploy-executor/0.2.0` does not match expected `forge-deploy-executor/0.1.0`
```

After, a major break is refused with the reason spelled out:

```
error[deploy-target-unavailable]: deploy target unavailable: forge-deployer-jenkins declares
the contract `forge-deploy-executor/1.0.0`, which is incompatible with the required
`forge-deploy-executor/0.1.0`: the major versions differ (1 and 0)
```

And the `ssh` kind is refused in one wording everywhere it appears:

```
error[deploy-invalid]: deploy invalid: deployment target kind `ssh` is known but not yet
wired; v0.1.0 supports `local`, `docker-compose`, `jenkins`, and `mac-runtime`
```

## Behavioral Model

| Actor | Block | Boundary |
|---|---|---|
| An adapter that declares the executor family at the same major | The adapter is accepted. | The wire format is unchanged. |
| An adapter that declares a newer minor of the same family | The adapter is accepted. | The minor is reported, not gating. |
| An adapter that declares a different major | The adapter is refused, and the refusal names both majors. | No state is written. |
| An adapter that declares a different family | The adapter is refused, and the refusal names both families. | No state is written. |
| An adapter that declares an unreadable version | The adapter is refused, and the refusal names the value. | No state is written. |
| An adapter that names itself with a readable family and version | The stage evidence records the family and the version. | The evidence is redacted before it is stored. |
| An adapter that names itself with an unreadable value | The adapter is refused. | A malformed claim is not a claim. |
| An adapter that does not name itself | The evidence attributes to the configured binary with an `unknown` revision. | Never a claimed version. |
| A provider probe against an adapter at a newer minor of the same family | The probe records `supported`. | The same rule as the deploy lane. |
| A provider probe against an adapter at a different major or family | The probe records `unavailable`, naming the mismatch. | |
| A manifest naming a wired kind | The kind is accepted. | |
| A manifest naming an unknown kind | The kind is refused, naming the supported kinds. | The same wording in the manifest, the configuration and the plan. |
| A manifest naming `ssh` | The kind is refused as known but not yet wired, in the same words everywhere. | |
| A project with no extension at all | Nothing changes. | The extension contract is not required by any other surface yet. |

## Contract and Compatibility

No wire format changes. The adapter still receives `forge-deploy-executor/0.1.0`
in its stdin payload and still echoes a `contract` field; only the rule that
judges the echoed value changes.

No manifest changes. No schema version change. No registry row change.

Two error codes change behaviour in one place: the `ssh` refusal now reports
`deploy-invalid` instead of `deploy-target-unavailable`, in the plan and the
observe path, and it is refused by the configuration validator as well.

The deploy executor's documentation
(`docs/adapter-contracts/deploy-executor.md`) states the rule the code now
applies: the `contract` field must name the executor family at the required
major, a newer minor is accepted, and a `source` that is present must be
readable as `family/major.minor.patch`.

## Failure and Boundary Policy

| Situation | Outcome |
|---|---|
| The adapter's envelope is not JSON | Refused, as today. |
| The adapter's envelope is not a JSON object | Refused, as today. |
| The adapter's `contract` cannot be read | Refused, naming the value. |
| The adapter's `contract` is a different family or major | Refused, naming both. |
| The adapter's `source` cannot be read | Refused, naming the value. |
| The adapter's `source` is absent | The binary name and `unknown` are recorded. |
| The evidence contains a credential | The evidence is redacted before it reaches the state file. |
| A deploy target kind is unknown | Refused, naming the supported kinds. |
| A deploy target kind is known but unwired | Refused, naming the supported kinds. |
| A sibling module reads a target kind constant | The constant keeps its name and its value. |

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

- `forge-deploy-executor/0.2.0` is accepted where `0.1.0` was required.
- `forge-deploy-executor/1.0.0` is refused, and the refusal names both majors.
- `forge-deployer/0.1.0` is refused, and the refusal names both families.
- `0.1.0` is refused as unreadable, and the refusal names the value.
- `ssh` is refused in one wording by the configuration validator and by the
  plan, and both refusals are `deploy-invalid`.
- An unknown kind is refused with the supported list.
- The stage evidence records the adapter's family and version.

## Implementation Plan

**The version.** A new `src/extension/version.rs`: `ContractVersion`, the
parser and the compatibility rule.

**The state.** A new `src/extension/state.rs`: `ExtensionState`,
`ExtensionDescriptor` and the shared judgement.

**The kinds.** A new `src/extension/kinds.rs`: the registry, the state lookup
and the one list renderer.

**The verification.** A new `src/extension/contract.rs` with the shared
verification, and a new `src/extension/extension_tests.rs` with the suite that
runs it over every kind.

**The deploy surface.** `src/deploy/contract.rs` re-exports the kind
constants from the extension module and gains the parsed executor contract.
`src/deploy/prepare.rs` and `src/deploy/model.rs` validate a kind through the
registry. `src/deploy/engine/stages.rs` refuses an unwired kind through the
registry, in one wording, under one code. `src/deploy/engine/model.rs` judges
the declared contract with the shared rule and reads the self-identification.
`src/provider/runner.rs` judges the same envelope with the same rule. Its
boundary description in `src/provider/matrix.rs` states the rule too, so the
owner reading the provider matrix sees what the probe actually accepts.

**The module registration.** `src/lib.rs` gains `pub mod extension;`.

**The doc.** `docs/adapter-contracts/deploy-executor.md` states the rule the
code applies.

**The tests.** The extension module's own suite, the deploy surface's kind
test, and the deploy executor boundary's version tests.

## Decision Ledger

- **Resolved:** the compatibility boundary is the family plus the major. The
  minor and the patch are reported. A major bump is a deliberate break of the
  wire format; a minor bump is an addition the old reader ignores.
- **Resolved:** the bare `major.minor.patch` form is unreadable as a contract.
  It has no family, and a contract without a family cannot be compared with
  one that has.
- **Resolved:** the shared contract lives in Forge Core, in `src/extension/`.
  Every surface that would use it is in Forge Core, and the boundary must not
  be owned by the first surface that happens to migrate.
- **Resolved:** the deploy target kinds move into the extension module. The
  list is a contract, and the deploy surface is its first reader, not its
  owner.
- **Resolved:** the `ssh` refusal becomes `deploy-invalid` everywhere. A kind
  this build does not implement is an invalid declaration, and one code for
  one situation beats two codes for one situation.
- **Resolved:** the configuration validator refuses `ssh` too. Today it
  accepts a kind the plan refuses, which is the inconsistency this change
  exists to remove.
- **Resolved:** a malformed `source` is refused, an absent `source` is not. A
  malformed claim is worse than no claim.
- **Resolved:** no other extension surface migrates here. The convergence is
  one surface per change, so each one's blast radius stays reviewable.
- **Resolved:** the plugin registry is not changed. Its `PluginState` is the
  model the shared vocabulary came from; the registry keeps its own type
  until the plugin surface converges.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order. The
  authorized surfaces are the files listed in the proposal.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| One contract-version rule | `ContractVersion` reads a declared version and judges it against a required one | The owner reads a refusal that names the extension, both contracts and the reason | Family plus major is the boundary; the minor is reported | `src/extension/version.rs` | A newer minor; a major break; another family; an unreadable version | 2.1–2.2 | Unit tests |
| One state vocabulary | `ExtensionState` and `ExtensionDescriptor` | The owner reads one state name and one reason | The plugin state promoted to the repository's, with a reason on every refusal | `src/extension/state.rs` | Ready; disabled; unavailable; invalid | 2.3 | Unit tests, the shared suite |
| One owner for the deploy target kinds | The registry holds the list and the one wording | The owner is refused by name, the same way, at every entry point | One list, one renderer, one code for the unwired refusal | `src/extension/kinds.rs` | An unknown kind; an unwired kind; a wired kind | 2.4–2.5, 3.1–3.2 | Unit tests, the deploy surface's test |
| The deploy executor boundary uses the shared contract | The adapter's declared contract is judged by the shared rule and its self-identification is read | A newer minor is accepted; a major break is refused with the reason | The port is the first migration | `src/deploy/engine/model.rs`, `src/deploy/contract.rs`, `src/provider/runner.rs` | A newer minor; a different major; the self-identification; a malformed self-identification | 2.6–2.8, 3.3–3.5 | Unit tests, the deploy contract test, a real run |
| One contract test every kind runs | The shared suite runs the shared verification over every kind | The owner trusts that a new kind inherits the rules | The suite lives beside the contract, not beside the surface | `src/extension/contract.rs`, `src/extension/extension_tests.rs` | A kind is verified against the shared contract | 2.9, 3.6 | The shared suite |
