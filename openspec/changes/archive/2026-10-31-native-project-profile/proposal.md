# Change: native-project-profile

## Why

Forge cannot inspect or register itself. `forge.yaml` declares
`profile: rust-product`, which is the *governance* profile attached to the
`rust-web` descriptor, not a Forge profile id. `inspect_profile` therefore
returns `unknown-profile`, and two commands fail on the flagship repository:

- `forge doctor .` reports `profile-known` as **FAIL**, leaves every
  descriptor-gated check unavailable, and returns `verdict: not healthy`;
- `forge register .` refuses, so `forge check .` cannot run at all.

CI works around this by generating and registering a scratch project. That
exercises the checker surface but never the real repository, and it hides the
fact that Forge's own project cannot pass the commands Forge ships.

The missing concept is a **native profile**: a project Forge owns and builds
with its own toolchain, which is known to the profile catalog and therefore
registrable and assessable, but which Forge must never scaffold. Native is
neither `Supported` (selectable for generation) nor `Planned` (a future
template): it is a descriptor with no template by design.

## What Changes

- **`ProfileSupportStatus` gains `Native`.** A native descriptor is known to
  `inspect_profile`, excluded from `list_profiles`, and refused by generation
  with a message that says it has no scaffold.
- **`rust-product` becomes a native descriptor.** It declares the Rust
  toolchain, `cargo build --workspace` / `cargo test --workspace`, and the
  Cargo workspace layout. It carries no generated capabilities.
- **Registration and resolution treat native as registrable.** `resolve_profile`
  accepts a native profile with no requested capabilities so `forge register`
  works; requesting a capability is still refused because a native project has
  no generated feature set.
- **Doctor judges a native project.** `profile-known` is `PASS`,
  `features-compatible` is `PASS` when nothing is requested, and the build
  definition is judged against `Cargo.toml`. The `readiness` checker and API
  status do not report a native profile as a planned gap.
- **Forge dogfoods its own commands.** The CI surfaces job registers this
  repository (`forge register .`) and runs `forge check forge` instead of a
  scratch project.
- **The evidence claim matches the export.** `.project.json` moves from
  `evidence_status: verified` to `implemented`: doctor's `release-evidence`
  check names the old claim as assertion-without-evidence because the gate
  export has zero verified fields.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `src/profile/model.rs` | `Native` status |
| `src/profile/catalog.rs`, `resolve.rs` | native descriptor, inspect/refuse rules |
| `src/generate/engine.rs` | generation refuses native |
| `src/doctor/runner.rs`, `probes.rs` | native profile assessment |
| `src/checker/mod.rs`, `src/api/status.rs` | native is not a planned gap |
| `.github/workflows/ci.yml` | surfaces job dogfoods `register`/`check` |
| `openspec/specs/profile-registry/spec.md` | native profile requirement |
| `openspec/specs/doctor-maturity-assessment/spec.md` | native assessment requirement |
| `ROADMAP.md` | the proposed follow-up becomes delivered |

Persistence: the registry gains a row when Forge registers itself; no schema
change.

Failure boundaries: a native profile is never selectable for generation; a
capability request on it is refused; a project with an unknown profile keeps
the existing unknown-profile behaviour.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `profile-registry` | modified | native profile class |
| `doctor-maturity-assessment` | modified | native project assessment |
| `core-manifest-registry` | read only | registration path unchanged |

## Non-Goals

- No native descriptor for every repository Forge owns; only `rust-product`.
- No generation template, feature set or scaffold for a native profile.
- No change to the governance-profile mapping (`rust-web` still maps to
  `rust-product` in `.project.json`); the two names live in different
  namespaces and the descriptor documents that.
