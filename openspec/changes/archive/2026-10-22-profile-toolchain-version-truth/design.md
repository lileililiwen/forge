## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The profile catalogue | `forge` | Yes. `src/profile/catalog.rs`. |
| The version rule | `forge` | Yes. `src/profile/version.rs` (new). |
| The profile preflight | `forge` | Yes. `src/profile/resolve.rs`. |
| The toolchain error vocabulary | `forge` | Yes. `src/core/mod.rs`. |
| The readiness matrix | `forge` | Yes. `src/readiness/mod.rs`. |
| The profile inspection output | `forge` | Yes. `src/cli/feature.rs`. |
| The preflight output | `forge` | Yes. `src/cli/setup.rs`. |
| The release-readiness document | `forge` | Yes. `docs/release-readiness.md`. |
| The version-truth contract test | `forge` | Yes. `tests/profile_toolchain_version_truth_contract.rs` (new). |
| The profile registry capability | `forge` | Modified. |
| The release-readiness capability | `forge` | Modified. |
| The doctor | `forge` | Read only. |
| The release lane | `forge` | Read only. |
| The generator's `--verify-native` path | `forge` | Read only. |
| The manifest | `forge` | Read only. |

## The Gap

### The declared version is a number nothing checks

`ProfileDescriptor::toolchain_version` is declared for five of the six
supported profiles:

| Profile | Toolchain | Declared version |
|---|---|---|
| `aspnet-web` | `dotnet` | `10.0` |
| `flutter-app` | `flutter` | `3.22` |
| `nextjs-web` | `node` | `20` |
| `python-service` | `python3` | `3.12` |
| `react-web` | `npm` | `20` |
| `rust-web` | `cargo` | `stable` — a channel, not a version |

It is read in exactly one place in the repository —
`src/cli/feature.rs:629`, which prints it:

```rust
format!(
    "toolchain: {}{}",
    p.toolchain,
    p.toolchain_version
        .as_deref()
        .map(|v| format!("@{v}"))
        .unwrap_or_default()
),
```

No surface compares it with the host. The preflight checks presence and
nothing else:

```rust
// src/profile/resolve.rs
let present = match available {
    Some(set) => set.contains(profile.toolchain.as_str()),
    None => toolchain_on_path(profile.toolchain.as_str()),
};
if !present {
    return Err(ForgeError::ToolchainMissing { .. });
}
Ok(PreflightReport { id, version, toolchain, available: true })
```

So on this host — node v22.23.3, npm 10.9.9, Python 3.14.4, Flutter
3.47.1, dotnet 10.0.400:

```text
$ forge profile preflight nextjs-web
preflight ok: toolchain 'node' available for profile 'nextjs-web'
$ forge profile preflight python-service
preflight ok: toolchain 'python3' available for profile 'python-service'
$ forge profile preflight flutter-app
preflight ok: toolchain 'flutter' available for profile 'flutter-app'
```

Every one of those profiles declares a version the host does not have.
The preflight says ok.

### The readiness matrix records the host's version and never compares it

The matrix is the surface that produces *release* evidence. It already
captures the host's version:

```rust
// src/readiness/mod.rs
let toolchain_version = capture_toolchain_version(&descriptor.toolchain, &native_path);
```

and stores it in the row. The declared version is never read, and the
row's result is decided by the native commands alone:

```rust
if drift.is_empty() { ... }
let build = run_native_command(&descriptor.build_command, &target, &native_path);
if !build.outcome.success { /* failed */ }
```

So a row can report `passed` on a toolchain the profile does not declare,
and the release-readiness document's own rule — "the gate passes only for
profiles the runner qualifies" — has no version in it at all.

### Two of the five declarations contradict the repository's own qualification

The repository states its qualification basis in
`docs/release-readiness.md`:

> | CI (`ubuntu-latest` + Rust/Node/.NET toolchains) | `rust-web`,
> `nextjs-web`, `aspnet-web` | toolchains provisioned by the workflow
> (`dotnet 10.0.x`, matching this qualification; Node 24) |

and `.github/workflows/ci.yml` provisions it:

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: "24"
- uses: actions/setup-dotnet@v4
  with:
    dotnet-version: "10.0.x"
```

| Profile | Declared | The repository's qualification | Agree? |
|---|---|---|---|
| `aspnet-web` | `dotnet@10.0` | `dotnet 10.0.x` | yes — the document says "matching this qualification" |
| `python-service` | `python3@3.12` | Python 3.12 (CI's `setup-python`) | yes |
| `nextjs-web` | `node@20` | Node 24 | **no** |
| `react-web` | `npm@20` | Node 24 / npm 11 | **no — and npm has no version 20** |
| `flutter-app` | `flutter@3.22` | no CI qualification | unverifiable |
| `rust-web` | `stable` | Rust `stable` | a channel is not a version: a moving version cannot be pinned, so nothing is judged |

`react-web`'s declaration is not merely stale. npm is at 10/11; there
has never been an npm 20. The number was a **node** version all along:
`profile-build-command-runnable` established that node and npm are one
toolchain in Forge's model, and this change applies that same rule to
the version.

### The version banners are not uniform

A comparison rule has to read a real banner:

| Runtime | `--version` output | The version |
|---|---|---|
| `node` | `v22.23.3` | `22.23.3` |
| `npm` | `10.9.9` | `10.9.9` |
| `python3` | `Python 3.14.4` | `3.14.4` |
| `dotnet` | `10.0.400` | `10.0.400` |
| `flutter` | `Flutter 3.47.1 • channel stable • https://github.com/flutter/flutter.git` | `3.47.1` |
| `cargo` | `cargo 1.99.1 (5f94df478 2026-08-27)` | `1.99.1` |

The first dotted-numeric token of the first line is the version in every
case. A `v` prefix, a product name, a channel label and a build hash all
appear, and none of them is the version.

## The Decision

### The declared version is a requirement, checked by one rule

**A profile declares the version of the runtime that executes its
commands, and the requirement is compared component-wise.**

`src/profile/version.rs` owns the rule:

```rust
/// The numeric version in a toolchain's `--version` banner.
pub fn extract_version(banner: &str) -> Option<String>;

/// Whether the observed version satisfies the declared requirement.
///
/// The declared version names the components that must match: `24` is a
/// major version, `3.12` is a major and a minor, `10.0` is a major and
/// a minor. A component the observed version does not carry cannot be
/// confirmed, so it does not satisfy the requirement.
pub fn version_satisfies(declared: &str, observed: &str) -> bool;

/// The binary whose version carries the ecosystem's runtime.
pub fn runtime_binary(toolchain: &str) -> &str;

/// The runtime's version under the given `PATH`.
pub fn probe_version(runtime: &str, path: &OsStr) -> Option<String>;
```

The comparison is deliberately not a string prefix. A prefix would read
`3.1` as satisfying `3.12`, and `2` as satisfying `24`. The declared
version's component count is the number of components that must match.

### The runtime is the ecosystem's binary

`profile-build-command-runnable` decided that node and npm are one
toolchain, because `react-web` and `nextjs-web` both build with
`npm run build`. The same decision applies to the version: the npm
ecosystem's runtime is `node`, and its version is node's version.

```rust
pub fn runtime_binary(toolchain: &str) -> &str {
    match crate::kit::registry::Ecosystem::for_toolchain(toolchain) {
        // The Node ecosystem's runtime is node; npm and node ship together.
        Ecosystem::Npm => "node",
        Ecosystem::Dotnet => "dotnet",
        Ecosystem::Cargo => "cargo",
        Ecosystem::Pub => "flutter",
        // A toolchain with no registered ecosystem is its own runtime.
        Ecosystem::None => toolchain,
    }
}
```

The profile plane already depends on the kit plane
(`src/profile/catalog.rs:122` uses `crate::kit::registry::KitReference`),
so the ecosystem map is the one place that already knows the Node
ecosystem's two names.

### The catalogue is corrected against the repository's own qualification

| Profile | Before | After | Why |
|---|---|---|---|
| `nextjs-web` | `node@20` | `node@24` | CI provisions Node 24; `docs/release-readiness.md` records Node 24 as the qualification basis. |
| `react-web` | `npm@20` | `npm@24` (the node runtime) | npm has no version 20; under the one-toolchain rule the number is node's, and node 24 is what CI provisions. |
| `python-service` | `python3@3.12` | unchanged | CI's `setup-python` provisions 3.12. |
| `aspnet-web` | `dotnet@10.0` | unchanged | CI provisions `dotnet 10.0.x`; the document already calls it "matching this qualification". |
| `flutter-app` | `flutter@3.22` | unchanged | No CI qualification exists; the document records the Flutter SDK as the prerequisite. |
| `rust-web` | `stable` (a channel) | unchanged | CI pins `stable`; a moving version cannot be pinned, and a channel states no version requirement. |

The toolchain **names** are untouched. `react-web` still pins `npm` and
`nextjs-web` still pins `node`; `profile-build-command-runnable` decided
that and this change does not reopen it. Only the number moves, and only
for the two declarations the repository's own CI contradicts.

`forge profile inspect` names the runtime it declares, so the line is
true for the npm profile:

```text
toolchain: npm (node@24)      # react-web
toolchain: node@24             # nextjs-web
```

### The preflight checks the version

A toolchain that is present but the wrong version is not unavailable.
Saying "the toolchain is unavailable" would be a false statement of the
same class this programme has been repairing, so the mismatch gets its
own typed error:

```rust
#[error("toolchain '{toolchain}' required by profile '{profile}' is version {observed}; \
         the profile declares {declared}; the profile was not tested")]
ToolchainVersionMismatch {
    toolchain: String,
    profile: String,
    declared: String,
    observed: String,
},
```

with the code `toolchain-version-mismatch`. The preflight's report gains
the observed version, so a successful preflight names what it checked:

```text
preflight ok: toolchain 'node' version 24.5.0 satisfies profile 'nextjs-web' (node@24)
```

A profile that states no version requirement is never refused on a
version: the requirement is what the descriptor states, and `rust-web`
states a moving channel (`stable`) rather than a version. A channel cannot
be compared against a host's version, so it is not a requirement.

### The readiness matrix checks the version

A host that presents the wrong version is treated like a host that
presents no toolchain, because in both cases the row is not evidence for
that profile:

- the row is `unverified`, never `passed`;
- the reason names the profile, the declared version and the observed
  version;
- the native commands are not run, exactly as on the absent-toolchain
  path;
- the row records the declaration next to the observation.

The row gains three fields, each a distinct fact:

| Field | Meaning |
|---|---|
| `toolchain_runtime` | the binary whose version was probed (`node` for the npm ecosystem) |
| `toolchain_version_declared` | the profile's declared requirement |
| `toolchain_version_satisfied` | whether the observed version satisfies it; unknown when there is no declaration or no observation |

`failed` keeps its meaning: a native command that failed. A version
mismatch is `unverified`, the state the matrix already uses for "this
host did not present a qualified toolchain".

### The claims are pinned

A declaration that nothing checks drifts, and a check that stops
comparing is invisible. The contract check holds all of it:

| Claim | The contract check asserts |
|---|---|
| the catalogue | every declared version is a version the profile's runtime can have, and it matches the versions the repository's own CI provisions |
| the rule | `24` is satisfied by `24.5.0` and not by `22.23.3`; `3.12` is satisfied by `3.12.4` and not by `3.14.4`; `10.0` is satisfied by `10.0.400`; `3.1` is not satisfied by `3.12.4` |
| the banner | `v22.23.3`, `Python 3.14.4`, `Flutter 3.47.1 • channel stable • …` and `cargo 1.99.1 (5f94df478 …)` all yield their version |
| the preflight | a mismatched version is refused with `toolchain-version-mismatch`, and a matching one is accepted |
| the matrix | a row on a mismatched version is `unverified` with the mismatch named, and never `passed` |

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The doctor | It never claimed anything about the toolchain. It has no toolchain finding to repair. |
| The generator's `--verify-native` path | It checks that the toolchain is present and runs the native commands; a missing toolchain is `toolchain-missing` and stays. The native commands are the same commands the matrix runs, and the matrix is where the version is judged. |
| The `failed` matrix state | It keeps its meaning: a native command failed. |
| The readiness gate's qualified subset | Which profiles a runner qualifies for is still the operator's selection. Only each row's verdict becomes honest about the toolchain it ran on. |
| The toolchain names | `react-web` pins `npm`, `nextjs-web` pins `node`; `profile-build-command-runnable` decided that. |
| The profile set | No profile is added, promoted or retired. |
| The manifest | No field is added, renamed or defaulted. |
| The release lane | It runs the doctor, not the preflight. |
| The kit registry's package pins | Unchanged; only the kit's ecosystem mapping is read. |

## User Experience and Interface

Before, a host on the wrong toolchain version was told everything was
fine:

```text
$ forge profile preflight nextjs-web
preflight ok: toolchain 'node' available for profile 'nextjs-web'
```

and the matrix reported a pass on that same host:

```text
readiness matrix (contract 0.1.0): passed=6 failed=0 unverified=0 ready=true
```

After, the preflight names both versions and refuses the mismatch:

```text
$ forge profile preflight nextjs-web
error[toolchain-version-mismatch]: toolchain 'node' required by profile 'nextjs-web' is
version 22.23.3; the profile declares 24; the profile was not tested
```

and the matrix reports the row as unverified, with the declaration next
to the observation:

```text
profile=nextjs-web result=unverified toolchain=node
  reason: toolchain 'node' for profile 'nextjs-web' is version 22.23.3; the profile
  declares node@24; the profile was not tested
```

On a host that meets the declaration, the preflight says what it
checked:

```text
preflight ok: toolchain 'node' version 24.5.0 satisfies profile 'nextjs-web' (node@24)
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| The profile states no version requirement | The preflight checks presence only, as before. The matrix row records the observed version and is not judged on it. A profile that declares a moving channel rather than a version (`rust-web` tracks `stable`) states no version requirement, so it is judged the same way. |
| The host's runtime version satisfies the declaration | The preflight passes and names the version it checked. The matrix row is judged on the native commands, as before. |
| The host's runtime version does not satisfy the declaration | The preflight is refused with `toolchain-version-mismatch`, naming the profile, the declared version and the observed version. The matrix row is `unverified` with the mismatch named, and the native commands are not run. |
| The runtime's version cannot be read | The version is unknown, no comparison is made, and the row records the unknown. The absence of a version is never read as a mismatch. |
| The npm ecosystem's profile is judged | The runtime probed is `node`, and the version compared is node's — the rule `profile-build-command-runnable` established for the build command's program. |
| The toolchain is absent | Unchanged: the preflight refuses with `toolchain-missing` and the matrix row is `unverified`. |
| A planned profile is selected | Unchanged: refused with `unsupported-profile` before anything is probed. |

## Failure and Boundary Policy

- **A declared requirement is never reported as met without a
  comparison.** The preflight and the matrix compare whenever the host's
  version can be read.
- **A present toolchain is never reported as missing.** A version
  mismatch is its own error, not `toolchain-missing`.
- **A version that cannot be read is not a mismatch.** An unreadable
  version is unknown, and unknown is recorded as unknown.
- **The check is read-only.** It runs the runtime's `--version` and
  reads the catalog. It writes nothing to the inspected project.
- **The matrix's vocabulary is unchanged.** `passed`, `failed` and
  `unverified` keep their meanings; only the row's evidence grows.

## Verification Oracle

```bash
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
./target/debug/forge gate --dry-run
./target/debug/forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
git diff --check
```

Four real runs:

1. `forge profile preflight nextjs-web` on a host whose node version does
   not match the declaration: refused with `toolchain-version-mismatch`,
   naming both versions.
2. `forge profile preflight rust-web`: unchanged, because the profile
   declares no version.
3. `forge readiness matrix`: the row for a mismatched host is
   `unverified` with the mismatch named, and the row records the
   declaration next to the observation.
4. `forge profile inspect react-web`: the toolchain line names the
   runtime it declares.

## Implementation Plan

**The rule.** `src/profile/version.rs`: the banner extraction, the
component-wise comparison, the runtime resolution and the version probe.

**The catalogue.** `src/profile/catalog.rs`: `nextjs-web` and
`react-web` declare the version the repository's qualification
provisions.

**The preflight.** `src/profile/resolve.rs`: the version is checked and
the report carries the observed version.

**The error.** `src/core/mod.rs`: `ToolchainVersionMismatch` and its
code.

**The matrix.** `src/readiness/mod.rs`: the version is checked, the row
records the declaration, the observation and the verdict, and the
existing `capture_toolchain_version` becomes the shared probe.

**The output.** `src/cli/feature.rs`: the toolchain line names the
runtime. `src/cli/setup.rs`: a successful preflight names the version it
checked.

**The document.** `docs/release-readiness.md`: the qualification basis
and the row states.

**The tests.** `tests/profile_toolchain_version_truth_contract.rs` pins
the rule, the catalogue, the preflight and the matrix;
`src/profile/version.rs` carries the rule's own unit tests.

## Decision Ledger

- **Resolved:** the declared version is a requirement on the version of
  the runtime that executes the profile's commands.
- **Resolved:** the comparison is component-wise on the declared
  version's component count, never a string prefix.
- **Resolved:** the runtime is the ecosystem's binary; the npm
  ecosystem's runtime is `node`, so `react-web` and `nextjs-web` declare
  and check the same node version.
- **Resolved:** the catalogue's two stale declarations are corrected
  against the repository's own CI qualification (Node 24); the other
  three and the one absent declaration are unchanged.
- **Resolved:** a version mismatch is refused by the preflight with a
  new `toolchain-version-mismatch` error, because a present toolchain is
  never unavailable.
- **Resolved:** a version mismatch makes the matrix row `unverified`,
  not `failed`, because `failed` keeps its meaning of a native command
  that failed.
- **Resolved:** the native commands are not run on a mismatched host,
  exactly as on the absent-toolchain path.
- **Resolved:** the doctor's inventory is untouched; it never claimed
  anything about the toolchain.
- **Implementation authorized (2026-10-22):** the owner directed the
  trunk repair programme and authorized this package in the approved
  order, after `doctor-claim-evidence` closed the claim-evidence gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| A declared toolchain version is a checked requirement | The preflight and the matrix compare the host's version | The mismatch is named | The version rule | `src/profile/version.rs`, `src/profile/resolve.rs`, `src/readiness/mod.rs` | A mismatch is refused and unverified; a match is accepted | 3.1, 3.2, 3.5 | The contract test; the real runs |
| The declarations match the repository's own qualification | Two stale declarations are corrected | `profile inspect` names the runtime | The catalogue correction | `src/profile/catalog.rs`, `src/cli/feature.rs` | The declared version is one the runtime can have | 3.3 | The contract test |
| A present toolchain is never reported as missing | The mismatch has its own error | The error names both versions | The new error | `src/core/mod.rs` | A mismatch is `toolchain-version-mismatch` | 3.4 | The contract test |
| The rule is pinned | A declaration cannot drift | The claims are held | The contract check | `tests/profile_toolchain_version_truth_contract.rs` | The rule, the catalogue, the preflight and the matrix are held | 3.6 | The contract test |
