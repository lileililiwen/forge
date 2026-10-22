# Change: profile-toolchain-version-truth

## Why

Every profile descriptor declares a required toolchain version. Nothing in
Forge checks it.

```rust
// src/profile/catalog.rs
descriptor(
    "nextjs-web",
    "0.1.0",
    "adapter-nextjs",
    "typescript",
    "node",
    Some("20"),          // <- the declared toolchain version
    ...
```

`toolchain_version` is read in exactly one place in the whole repository —
`forge profile inspect` prints it:

```text
toolchain: node@20
```

No surface compares it with the host's version. `forge profile preflight`
reports the toolchain as available after checking only that a file with
that name exists on `PATH`:

```text
$ forge profile preflight nextjs-web
preflight ok: toolchain 'node' available for profile 'nextjs-web'
```

That is true and useless. The profile declares `node@20`; this host runs
node v22.23.3; the preflight says ok. The same holds for every profile
with a declared version, and the readiness matrix — the surface that
produces *release* evidence — records the host's version next to the
declared one and never compares them:

```text
readiness matrix (contract 0.1.0): passed=6 failed=0 unverified=0 ready=true
```

**Two of the five declarations are also incoherent with the repository's
own qualification.**

| Profile | Declared | The repository's own qualification | Agree? |
|---|---|---|---|
| `aspnet-web` | `dotnet@10.0` | `dotnet 10.0.x` (CI) | yes |
| `python-service` | `python3@3.12` | Python 3.12 (CI) | yes |
| `nextjs-web` | `node@20` | Node 24 (CI, `node-version: "24"`) | **no** |
| `react-web` | `npm@20` | Node 24 / npm 11 (CI) | **no — and npm has no version 20** |
| `flutter-app` | `flutter@3.22` | no CI qualification at all | unverifiable |
| `rust-web` | `stable` | Rust `stable` | a channel is not a version, so nothing is pinned |

`docs/release-readiness.md` states the qualification basis in its own
words — "`dotnet 10.0.x`, matching this qualification; Node 24" — so the
repository already knows that Node 24 is the version the Node profiles
are qualified on. The catalogue says 20. And `react-web`'s `npm@20` is
not a version npm can have at all: npm is at 10/11. Under Forge's own
model node and npm are one toolchain — `profile-build-command-runnable`
established that — so the number was always a node version, and it is
stale for the same reason.

So the declaration is a number nothing checks, and where it is checked by
a human against the repository's own CI, two of the five are wrong.

## What Changes

- **The declared version becomes a requirement.** A profile declares the
  version of the runtime that executes its commands, and the comparison is
  one shared rule: the numeric version extracted from the tool's banner,
  compared component-wise against the declared version. A declaration
  that names a moving channel rather than a version — `rust-web` tracks
  `stable` — states no version requirement and is never refused on a
  version.
- **The runtime is the ecosystem's binary.** For the npm ecosystem the
  runtime is `node`, so `react-web` and `nextjs-web` declare and check
  the same node version — the rule `profile-build-command-runnable`
  already established for the build command's program.
- **The catalogue is corrected against the repository's own
  qualification.** `nextjs-web` and `react-web` declare the Node version
  CI provisions. `python-service`, `aspnet-web`, `flutter-app` and
  `rust-web` keep what they declare.
- **The preflight checks the version.** A host whose runtime version does
  not satisfy the declaration is refused with a new typed error that names
  the toolchain, the profile, the declared version and the observed
  version. "The toolchain is unavailable" would be a false statement, so
  it gets its own error.
- **The readiness matrix checks the version.** A host that presents the
  wrong version is treated like a host that presents no toolchain: the
  row is `unverified`, the mismatch is named, and the native commands are
  not run. The row records the declared version next to the observed one.
- **The claims are pinned.** A repository contract check holds the
  catalogue, the preflight and the matrix to the version rule, so a
  declaration cannot drift back into a version the toolchain cannot have
  or a check that stops comparing.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The profile catalogue | `forge` | Yes | `src/profile/catalog.rs`: `nextjs-web` and `react-web` declare the version the repository's qualification provisions. |
| The version rule | `forge` | Yes | `src/profile/version.rs`: one extraction, one comparison, one runtime resolution. |
| The profile preflight | `forge` | Yes | `src/profile/resolve.rs`: the preflight checks the version. |
| The toolchain error vocabulary | `forge` | Yes | `src/core/mod.rs`: a new `toolchain-version-mismatch` error, so a present toolchain is never reported as missing. |
| The readiness matrix | `forge` | Yes | `src/readiness/mod.rs`: the matrix checks the version and records the declaration. |
| The profile inspection output | `forge` | Yes | `src/cli/feature.rs`: the toolchain line names the runtime it declares. |
| The preflight output | `forge` | Yes | `src/cli/setup.rs`: a successful preflight names the version it checked. |
| The release-readiness document | `forge` | Yes | `docs/release-readiness.md`: the qualification basis and the row states. |
| The version-truth contract test | `forge` | Yes | `tests/profile_toolchain_version_truth_contract.rs`. |
| The profile registry capability | `forge` | Modified | The preflight requirement names the version it checks. |
| The release-readiness capability | `forge` | Modified | The matrix requirement names the version it checks. |
| The doctor | `forge` | Read only | It never claimed anything about the toolchain. |
| The manifest | `forge` | Read only | No field is added, renamed or defaulted. |
| The release lane | `forge` | Read only | It runs the doctor, not the preflight. |
| The profile catalogue's kit pins | `forge` | Read only | The toolchain *name* is unchanged; change 19's decision stands. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §23 asks the doctor to inspect the project; the profile and readiness requirements are the repository's own canonical capabilities. |
| Contracts | The descriptor's field names are unchanged. The matrix row gains three fields (`toolchain_runtime`, `toolchain_version_declared`, `toolchain_version_satisfied`), the preflight report gains the observed version, and one error code (`toolchain-version-mismatch`) is added. The `failed` and `unverified` states keep their meanings. |
| Callers | `forge profile preflight`, `forge profile inspect`, `forge readiness matrix` and `forge readiness check` read the change. The generator's `--verify-native` path keeps its own `toolchain-missing` refusal. |
| Persistence | Nothing is persisted. |
| Integrations | None. No new toolchain, network or credential surface. |
| Failures | One new error code. A toolchain that is present but the wrong version is never reported as missing. |
| Tests | A new `tests/profile_toolchain_version_truth_contract.rs` pins the rule, the catalogue, the preflight and the matrix. |
| Dependencies | None added. |
| Concerns | Quality: a declared requirement is now checked, and the declarations agree with the repository's own qualification evidence. Compatibility: the catalogue's toolchain names, the profile set and the manifest are unchanged. Security: no new file, process, network or credential surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `profile-toolchain-version-truth` | forge | Rust | added | this package |
| `profile-registry` | forge | Rust | modified | the preflight requirement names the version it checks |
| `profile-and-release-readiness` | forge | Rust | modified | the matrix requirement names the version it checks |

## Non-Goals

- **The doctor gains no toolchain finding.** It never claimed anything
  about the toolchain, so there is nothing to repair there.
- **The `failed` matrix state keeps its meaning.** A native command that
  fails is `failed`. A host that presents the wrong toolchain version is
  `unverified`, exactly like a host that presents no toolchain.
- **The toolchain names are unchanged.** `react-web` still pins `npm` and
  `nextjs-web` still pins `node`; `profile-build-command-runnable` decided
  that, and this change does not reopen it.
- **The readiness gate's qualified subset is unchanged.** Which profiles a
  runner qualifies for is still the operator's selection; this change only
  makes each row's verdict honest about the toolchain it ran on.
- **The generator's `--verify-native` refusal is unchanged.** It reports a
  missing toolchain, which is what it checks.
- **The manifest is unchanged.**
