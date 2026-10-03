# Proposal: A scaffold starts at high consumption of the registered shared layer

## Why

Forge owns the moment a project exists, and that moment currently produces a
project with **zero** consumption of the registered shared layer — while a
sibling library ships 75 packages that almost nothing uses.

Measured, not estimated:

- `src/generate/mod.rs:559-591` renders the whole `aspnet-web` profile inline.
  Its `Program.cs` body is `WebApplication.CreateBuilder(args)`,
  `app.MapGet("/", () => "hello from {id}")`, `app.Run()`. A workspace grep for
  `Platform.` inside `src/generate/mod.rs` returns **zero** matches, so not one
  of the 75 platform packages reaches a generated ASP.NET Core project.
- `templates/` contains exactly one file, `rust-web-main-rs.txt`. The
  `templates/` directory that exists to hold reusable assets holds one asset.
- `dotnet-platform-libs/src/` holds **75** package projects
  (`Platform.Core` … `Platform.Webhooks.EfCore`). **9** of the workspace's 81
  repositories reference any of them. The two flagship products are the
  weakest: `devloom` references `Platform.AspNetCore` plus `Platform.Core`;
  `kairovia` references only the jobs slice (`Platform.Jobs`,
  `Platform.Jobs.Hangfire`) and has no `NuGet.config` at all.
- The library's own composition root has almost no adoption. Counting distinct
  external consumer repositories per package: `Platform.Core` 7,
  `Platform.AspNetCore` 6, `Platform.Testing` 4, `Platform.RateLimiting` 2,
  `Platform.Idempotency` 2, `Platform.Observability` 2 — but
  `Platform.Web.Composition` 1, `Platform.Web.Telemetry` 1,
  `Platform.Testing.AspNetCore` 1, and `Platform.UI.Razor`,
  `Platform.Http.Resilience`, `Platform.FeatureManagement` and
  `Platform.Tenant.Lifecycle.AspNetCore` **0**.
- The UI story is the same shape twice. `dotnet-platform-libs/ui/` holds a
  complete DTCG token source (`ui/tokens/tokens.json`), a generator
  (`ui/scripts/generate-tokens.mjs`) emitting `tokens.css` and `tokens.ts`, a
  verifier (`ui/scripts/verify-ui.mjs`) that asserts a dark theme, a
  `prefers-reduced-motion` fallback, `--focus-ring`, `aria-busy` / `aria-invalid`
  / `aria-live`, `:focus-visible` and a dialog role, a `48rem` responsive
  breakpoint in `packages/react-ui/styles.css`, and three npm packages
  (`@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell`,
  all `0.1.0`) — consumed by **0**. Forge's own `src/ui_pattern/mod.rs` holds a
  tested catalog of 18 entries (17 active, one deprecated) with per-profile
  `react-web` / `nextjs-web` / `flutter-app` adapters — also consumed by 0;
  `forge ui-pattern` appears outside `forge/` only in the workspace
  `ARCHITECTURE.md`.
- The Rust mirror is the same: `rust-platform-libs/crates/` holds 8 crates with
  **1** external consumer.
- Forge itself reaches 3 of 81 repositories (the three carrying a `forge.yaml`:
  `chronicleaf`, `devloom`, `kairovia`).

The result is a control plane whose central promise — deterministic assembly of
reusable software — is only exercised on the manifest and the container
definition. The reusable layer is the part that ships, and it ships unused.

The workspace's own rule is that a capability enters a shared library only with
at least two real consumers. The same rule read in reverse says a scaffold must
not invent a dependency the library has not proven — which is exactly what
"pre-wire everything" would do. This change therefore pre-wires the **proven**
set and records the unproven set as visible, non-restoring, evidence-gathering
declarations.

## What Changes

- Add a **compiled-in shared-layer kit reference** to the profile registry: a
  versioned descriptor (`id`, `version`, `ecosystem`, `feed`, `tfm`, `packages`,
  `assets`) declared on `ProfileDescriptor` exactly as profiles, features and
  standard packs are declared today — resolved offline, never fetched at
  render time. `forge profile inspect <id>` renders it; nothing new is fetched.
- Add a **profile-declared minimum-consumption floor**. Each profile declares
  how many confirmed shared-layer packages a scaffold must start with. When a
  profile cannot meet its floor, `forge new` **refuses before writing
  anything**; there is no warn-and-continue path. The only escape is an
  explicit `--kit-exception <reason>` that records a visible, dated exception
  in the generated manifest and README. A profile whose ecosystem has no proven
  kit declares a **recorded zero** with the missing evidence named — which is
  an honest declared state, not a pass.
- Pre-wire `aspnet-web` concretely with the packages that both meet the
  two-consumer bar and require **no external infrastructure** (their only
  project references are `Platform.Core` and `Platform.Web.Telemetry`):
  `Platform.Core`, `Platform.AspNetCore`, `Platform.Testing`,
  `Platform.RateLimiting`, `Platform.Idempotency`, `Platform.Observability` —
  plus a generated `Directory.Packages.props` that pins every version centrally.
  Packages below the bar (`Platform.Web.Composition`, `Platform.Web.Telemetry`,
  `Platform.Testing.AspNetCore`, `Platform.UI.Razor`, `Platform.Http.Resilience`,
  `Platform.Web.*`, `Platform.FeatureManagement`,
  `Platform.Tenant.Lifecycle.AspNetCore`) are rendered as a **commented,
  non-restoring** block naming each one's missing second consumer, so the
  scaffold tells the truth and produces the adoption the library still needs.
  Infrastructure-bearing families (identity, EF Core persistence, tenant
  lifecycle, caching, jobs, billing, mailing, storage) stay behind the existing
  `--feature` mechanism and are never pre-wired unconditionally.
- **Decide the distribution mechanism and refuse the non-reproducible one.** A
  scaffold declares a *named, versioned* feed whose value is a path **relative to
  the generated project**, and the `.nupkg` bytes are committed into the generated
  tree, so a fresh clone restores with no sibling checkout, no environment
  variable and no secret. Source-mode `ProjectReference` into a sibling checkout
  is refused, and so is any machine-specific absolute path — `kairovia`'s
  restore-from-a-machine-path is named in `design.md` as the known-bad shape
  this ruling exists to prevent. A feed resolved from an environment variable is
  refused for the same reason: a variable a CI runner does not carry is the same
  failure class as a hard-coded absolute path. Restore is the generated
  project's own native toolchain's job, never Forge's.
- **Make the committed feed reproducible and drift-checked.** A `forge kit pack`
  command repacks the confirmed set's restore closure from a checked-out sibling
  library, taking the version from the sibling's own package manifest, so the
  feed is regenerated rather than hand-copied. A `forge kit verify` command
  fails when the committed feed does not match the kit version a project's
  `forge.yaml` declares, when a needed package is missing, or when an undeclared
  package is present — the same drift class as the `global.json`-versus-CI defect,
  which reached production precisely because nothing checked it.
- Pre-wire the presentation layer from **one** token source. `dotnet-platform-libs/ui`
  is the single source of truth for design tokens (it is the only one with a
  generator, a verifier and a dark theme). `forge new` vendors its generated
  `tokens.css` and `tokens.ts` into the project as ordinary, digest-receipted
  source. Forge's `ui_pattern` catalog stays the **behavior** layer (states,
  a11y, navigation, form semantics — `requirement.md` §13: "semantic patterns,
  not merely copied HTML") and its adapters consume the vendored tokens rather
  than defining a second palette. `flutter-app` declares `tokens: none` with a
  reason rather than inventing Dart tokens.
- Preserve the README's determinism claim. Same request, same pinned assets,
  byte-identical output; the rendered kit manifest is verified against the
  compiled-in descriptor digest; a native restore/build is reported
  `unverified` when the toolchain is absent, exactly as `verify_native`
  reports today. A kit version bump **never** mutates an already-scaffolded
  project; it is a reviewable, receipt-checked upgrade or nothing.
- Record the **target-framework blocker** found in the evidence: every platform
  package targets `net10.0` while the `aspnet-web` scaffold targets `net8.0`,
  and a `net8.0` project cannot reference a `net10.0` package. Generation must
  render the TFM the kit requires, and the choice of who moves first is an open
  decision recorded in `design.md`.

## Package Boundary and Split Assessment

Single outcome: **a project created by `forge new` starts already consuming the
registered shared layer, and says so honestly in its own tree.**

Included surfaces: the profile registry descriptor, the generator
(`src/generate/mod.rs` and a new `kits/` vendored-asset directory alongside
`templates/`), the generated project manifests, and the `forge.yaml` `kit`
block. The scaffold is the only place this is applied.

Excluded surfaces, and who owns them: the 81-repository retrofit, the Gate
semantics that would later police adoption, and any adoption gate. Those are
separate changes in their owning repositories. This package defines what a
*scaffold* emits; it does not touch one existing repository.

Split signals considered: kit reference, floor enforcement, feed declaration and
token vendoring are each independently testable, but they share one owner
(Forge), one moment (generation), one contract (the pinned kit descriptor plus
its receipt) and one oracle (`forge new` on a clean host produces a project
whose manifest resolves). Splitting them yields scaffolds that are wired but
unverifiable, or floors that are enforceable but have nothing to count.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `scaffold-prewires-shared-layer` | `forge new` emits a project that already consumes the registered shared layer, deterministically and reproducibly | Forge, Rust; generated .NET / Node / Dart / Rust projects | profile-declared kit reference (id, version, ecosystem, feed, tfm, packages, assets) + ownership receipt + `forge.yaml` `kit` block | `deterministic-project-generation`, `profile-registry`, `standard-pack-registry-and-snapshots`, `semantic-ui-patterns` | byte-equal re-render, digest match against the compiled-in descriptor, floor refusal/exit code, and a `dotnet restore`/`npm install` attempt that is reported `unverified` rather than passed |

Dependency order: `deterministic-project-generation`, `profile-registry` and
`standard-pack-registry-and-snapshots` are archived and promoted;
`scaffold-prewires-shared-layer` consumes them and redefines none of their
contracts. It is independently adoptable and does not depend on the fleet
retrofit, the Gate or the adoption gate.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Deterministic generation | `src/generate/mod.rs` `template_files`, `CreationRequest`, `verify_native`; `templates/rust-web-main-rs.txt` | Pinned assets, staged ownership, byte-equal re-render, offline build/test | Emits no shared-layer reference at all; `templates/` holds one asset | Forge release | **extend shared owner** |
| Profile registry | `src/profile/mod.rs` `ProfileDescriptor` (additive `#[serde(default)] kit`) | Compiled-in, versioned, offline-resolved descriptors with compatibility and preflight | No field expresses a shared-layer kit | Forge release | **extend shared owner** |
| Standard-pack snapshots | `src/standard/mod.rs` receipt, `asset_digest`, `diff`/`upgrade` | Ownership receipt, per-file digest, conflict-on-modified upgrade | Scoped to `.standard/`; no kit packages | Forge release | **adopt** (reuse the receipt and upgrade machinery, do not invent a second ownership system) |
| Contract digest vendoring | `contracts/manifest.json` digest check (`platform-contract-consumption`) | Digest-pinned vendored bytes verified offline, no sibling checkout, no network | Contract schemas only, not UI/token assets | Forge release | **adapt through a generic adapter** (same digest discipline for `kits/`) |
| `dotnet-platform-libs` .NET packages | `src/Platform.*` (75), `Directory.Build.props` (`VersionPrefix 0.1.0`, `TreatWarningsAsErrors`), `Directory.Packages.props` | Auth/identity, idempotency, rate limiting, health, error boundaries, testing fixtures, persistence, tenancy, feature management, observability | `net10.0` only; 9 of 81 consumers; best composition root has 1 consumer | dotnet-platform-libs release | **adopt** (consume a pinned, named subset) |
| `dotnet-platform-libs/ui` kit | `ui/tokens/tokens.json`, `ui/scripts/generate-tokens.mjs`, `ui/scripts/verify-ui.mjs`, `ui/packages/*` | DTCG tokens, generated `tokens.css`/`tokens.ts`, dark theme, reduced-motion guard, `48rem` breakpoint, React shell/UI | 0 consumers; not published to a registry; no .NET token path | dotnet-platform-libs release | **adapt through a generic adapter** (vendor the generated artifacts; the DTCG file stays upstream-owned) |
| Forge semantic UI catalog | `src/ui_pattern/mod.rs` `ui_pattern_catalog()`, `install_pattern`, per-profile adapters | 17 tested pattern contracts + adapters, ordinary-source installation, ownership conflicts | 0 consumers; defines no token values | Forge release | **extend shared owner** (behavior layer only, consumes vendored tokens) |
| `rust-platform-libs` crates | `crates/` (8), `trailCrew/Cargo.toml` | Rust equivalents of core, identity, testing, audit | 1 consumer — no crate meets the two-consumer bar | rust-platform-libs release | **defer** (declared zero with the missing evidence named) |
| Flutter / Python kits | — | None registered | No kit exists for either ecosystem | Unowned | **keep local** (declared zero with a reason) |

## BFS Impact Map

- **Actors:** the operator running `forge new`; Forge Core/CLI; the profile
  registry; the compiled-in kit registry; the generated project's own native
  toolchain (`dotnet`, `npm`, `cargo`, `flutter`), which owns resolution and
  restore; the owner of `dotnet-platform-libs` / `rust-platform-libs`, who own
  the package bytes.
- **State:** kit descriptor `declared → confirmed floor → provisional
  (below the two-consumer bar) → recorded-zero`. A scaffold is
  `staged → floor-checked → rendered`; a floor failure is
  `refused (nothing staged)`. A kit version bump is
  `pinned-in-project → upgrade-planned → upgraded | refused-on-conflict`, and is
  never an implicit rewrite.
- **Persistence:** the generated `forge.yaml` gains an additive, schema-bumped
  `kit:` block (`id`, `version`, `ecosystem`, `tfm`, `confirmed_packages[]`,
  `provisional_packages[]`, `feed` with its project-relative `path`, `exception?`);
  the generated tree gains the ecosystem manifest
  (`Directory.Packages.props`, `package.json` dependencies,
  `[workspace.dependencies]`, `NuGet.config`), a `packages/platform-feed/`
  directory carrying the committed `.nupkg` bytes, and a `.platform/` owned
  subtree carrying the vendored token artifacts plus a receipt with one digest
  per owned file. **No new SQLite table and no new migration** — the kit id and
  version are additive fields on the existing project record. No change to
  `forge.yaml`'s existing keys.
- **Security:** the generated feed is a *named* source whose value is a path
  relative to the generated project. An absolute, machine-specific path, a value
  escaping the project through `..` or a `file://` URL, a shell metacharacter, a
  secret-shaped value, or a value read from an environment variable in a kit
  descriptor is refused before any file is written. Forge performs no network
  access and no package restore during generation. The rendered feed carries no
  credentials. Vendored assets — the token artifacts and the `.nupkg` bytes — are
  digest-checked on render, so a tampered kit asset fails generation rather than
  producing a project that silently differs from its descriptor.
- **Failure:** typed and non-empty — `kit-unknown` (id or version not in the
  compiled-in registry), `kit-ecosystem-mismatch` (a .NET package list on a
  Node profile), `kit-feed-invalid` (absolute, escaping or environment-resolved
  feed value), `kit-feed-version-mismatch` (committed feed version differs from
  the declared `kit.version`), `kit-feed-incomplete` (a needed package missing or
  an undeclared one committed), `kit-pack-unavailable` (no sibling checkout or
  sibling manifest),
  `kit-floor-not-met` (declared floor unmet, nothing written),
  `kit-exception-reason-required` (`--kit-exception` without a reason),
  `kit-digest-mismatch` (vendored bytes differ from the descriptor digest).
  None of them produce a half-wired project.
- **Compatibility:** `forge new --profile <id>` stays deterministic — identical
  inputs produce byte-identical output, and the README's "Rendering is verified
  against the asset version" claim is preserved and extended to the kit assets.
  `react-web`'s offline, dependency-free `npm run build` / `npm test` contract
  is preserved: token assets are vendored as ordinary source rather than added
  as registry dependencies, so no scaffold acquires a new network requirement
  to build. `npm install` is never run implicitly. Generated projects keep
  building and operating through their native tools with Forge absent. The five
  v0.1 profile ids stay valid.
- **Verification:** byte-equal re-render across repeated `forge new` runs;
  descriptor-digest agreement tests for every compiled-in kit (a drifted
  `kits/` byte fails the test, as `platform-contract-consumption` does for
  `contracts/`); a completeness test failing when a profile declares a kit with
  no registry row; floor tests for met, unmet, exception-recorded and
  exception-without-reason; feed tests proving an absolute path, a `..`-escaping
  path and an environment-resolved path are refused, that a project-relative
  named feed is rendered, and that no generated file resolves the feed through an
  environment variable; a **feed-version agreement test** over the committed
  `kits/feed/` proving the check passes when it matches and fails on a version
  mismatch, a missing package and an undeclared package; a **portability proof**
  that renders a scaffold, copies it to an unrelated path, and runs the
  project's own `dotnet restore` there with no environment variable and no
  sibling library; per-profile render fixtures asserting the exact
  pre-wired package set, including that provisional packages are commented and
  do not restore; a token-vendoring test asserting the vendored `tokens.css` /
  `tokens.ts` match the upstream digests and that no second palette is emitted;
  a "no Forge at runtime" test proving the generated project builds with the
  Forge binary absent; and `openspec validate --all --strict`.

## Capabilities

### New Capabilities

- `scaffold-prewires-shared-layer`: a profile declares a versioned shared-layer
  kit reference; `forge new` renders that reference into the generated project's
  native manifest; each profile declares a minimum-consumption floor that is
  enforced by refusal; the feed is a named, versioned source at a path relative
  to the generated project whose bytes are committed, and never a
  machine-specific path or an environment variable; the committed feed is
  reproducible through `forge kit pack` and checked against the declared kit
  version by `forge kit verify`; design tokens reach a scaffold from exactly one
  source; a kit version bump never mutates an existing project.

### Modified Capabilities (declared, not delta'd in this package)

This package's delta is **strictly additive** to the new capability. The
existing capabilities below are *consumed* and their behaviour is *narrowed*,
not rewritten; a follow-up package may promote these into explicit
`## MODIFIED Requirements` blocks once the scaffold surface has landed.

- `deterministic-project-generation`: "Portable generated projects" is
  narrowed — a scaffold now carries a pinned shared-layer reference **and the
  committed feed bytes that reference resolves**, so the "works without Forge"
  scenario must additionally hold at a project path the project was not
  generated at, with no sibling library and no environment variable present.
- `profile-registry`: the descriptor vocabulary gains one optional
  `kit` field; every existing field and id keeps its current meaning.
- `extended-profile-catalog`: the five v0.1 profile ids remain valid; the new
  `kit` field is what distinguishes "supported profile" from "supported profile
  with a proven shared layer".
- `semantic-ui-patterns`: pattern installation gains a token dependency — a
  pattern adapter consumes the project's vendored tokens; the catalog still
  defines no colour, spacing or typography value of its own.

### Unchanged consumers

`site-studio-preview-refinement`, `react-web-live-preview`,
`standard-pack-registry-and-snapshots`, `platform-contract-consumption` and the
publish/portal/portfolio surfaces are consumed unchanged; their contracts,
routes, typed errors and envelope shapes are untouched.

### Phase

Post-`v0.5` lifecycle foundation, **scaffold surface only**. It depends on
`deterministic-project-generation` (archived), `profile-registry` (archived)
and `standard-pack-registry-and-snapshots` (archived), and it is a prerequisite
for — but does not contain — any fleet-wide retrofit or adoption gate.

### Source requirement sections

`requirement.md` §3.1 (deterministic first, packages and generators before AI),
§3.2 (generate ordinary source; a generated project stays usable if Forge is
removed), §3.3 (reuse at multiple levels of granularity: profile → feature →
pattern → capability), §9 (Profiles), §13 (UI Standard Library — semantic
patterns, not copied HTML; typography, spacing and responsive rules as common
design rules), §14 (Project Creation), §15 (Deterministic Generation),
§34 (CLI Requirements), §39 (MVP) and §44 (Non-Goals).

## Non-goals

- **No retrofit of the 81 existing repositories.** Rewriting `devloom`,
  `kairovia`, `mewo`, `citylens`, `therapist-commons` and the rest to
  pre-wired manifests is explicitly out of scope and is not authorized by this
  package. It is a separate change in its owning repository.
- **No Gate change and no adoption gate.** Nothing here alters `forge gate`
  semantics, gate evidence, or the rule that blocks a repository without a kit
  reference. Those are separate changes owned elsewhere.
- **No publishing, signing or CI for the shared layer.** Forge does not publish,
  sign or release `dotnet-platform-libs`, `rust-platform-libs` or the UI kit, and
  it does not gain a release pipeline. Those libraries own their own feeds.
  Forge *does* read a checked-out sibling to pack a local feed and verify a
  committed one, which is a read plus a `dotnet pack` invocation, not a release.
- **No new dependency in Forge's own `Cargo.toml`**, no new SQLite table, no new
  MCP tool, no API route, no portal section. The surface is `forge new`,
  `forge profile inspect`, `forge kit pack`, `forge kit verify` and the
  generated tree.
- **No change to what a project may do.** Pre-wiring a dependency is not a
  feature grant: a scaffolded project does not gain authentication, a database,
  billing, tenancy or deployment because a package is referenced. Referencing
  `Platform.Idempotency` does not make the project idempotent.
- **No network access at generation time**, and no implicit `npm install`,
  `dotnet restore` or `cargo fetch`.
- **No replacement of DriftWatch, the PTY agent manager, the intent planner or
  the agent runtime**, and no change to Studio, publish, delivery or the
  project-to-production workflow.

## Open decisions — now resolved

These were material and are **not** silently fixed; `design.md` section 12
records the full statement, the options, the choice taken and the rationale for
each. Decision 3 (who moves the target framework first) was ruled by the owner:
the workspace is .NET 10, SDK 8 is being removed, and Forge raises the
`aspnet-web` TFM to `net10.0` rather than multi-targeting `dotnet-platform-libs`
down. The original statements are kept below unchanged so the reasoning is
still readable.

1. **Kit distribution mechanism.** **Ruled by the owner.** This package rules
   *out* source-mode `ProjectReference`, machine-specific absolute paths, and
   feeds resolved from an environment variable. The scaffold writes a
   `NuGet.config` with `<clear />`, one named source pointing at a path
   relative to the generated project, and nuget.org; the `.nupkg` bytes are
   committed into that directory, matching `therapist-commons` and `citylens`.
   The first implementation used `NUGET_PLATFORM_FEED` with
   `RestoreAdditionalProjectSources`; **the owner rejected it**, because a
   variable a CI runner does not carry is the same failure class as a hard-coded
   absolute path. `design.md` §5 records the full statement, the options, the
   rejection and the rationale, so the option is not silently re-proposed.
2. **The minimum-consumption floor value.** This package proposes `6` for
   `aspnet-web` and `0`-with-a-recorded-reason for the four ecosystems with no
   proven kit, and it specifies the *refusal* mechanism. The number itself is a
   product judgement, not a technical derivation.
3. **Who moves the target framework first.** Every platform package is `net10.0`
   and the `aspnet-web` scaffold is `net8.0`. Either Forge renders `net10.0`
   (changing an existing profile's output) or `dotnet-platform-libs`
   multi-targets down (a change in another repository).
4. **Whether the kit's owned subtree reuses the existing `.standard/` receipt and
   upgrade machinery** (this package recommends reuse) or introduces a separate
   `.platform/` ownership system. **Taken: the contained `.platform/receipt.json`,
   because reuse needs a sibling-side standard-pack descriptor that is outside
   this repository's write boundary. The shapes, the diff vocabulary and the
   ownership-conflict refusal are deliberately identical to the standard
   receipt, so it is a second instance of one discipline rather than a second
   design.**
