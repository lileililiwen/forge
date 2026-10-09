# scaffold-prewires-shared-layer Specification

## Purpose

Forge owns the moment a project exists, and that moment currently produces a project with **zero** consumption of the registered shared layer — while a sibling library ships 75 packages that almost nothing uses. Measured, not estimated: - `src/generate/mod.rs:559-591` renders the whole `aspnet-web` profile inline. Its `Program.cs` body is `WebApplication.CreateBuilder(args)`, `app.MapGet("/", () => "hello from {id}")`, `app.Run()`. A workspace grep for `Platform.` inside `src/generate/mod.rs` returns **zero** matches, so not one of the 75 platform packages reaches a generated ASP.NET Core project. - `templates/` contains exactly one file, `rust-web-main-rs.txt`. The `templates/` directory that exists to hold reusable assets holds one asset. - `dotnet-platform-libs/src/` holds **75** package projects (`Platform.Core` … `Platform.Webhooks.EfCore`). **9** of the workspace's 81 repositories reference any of them. The two flagship products are the weakest: `devloom` references `Platform.AspNetCore` plus `Platform.Core`; `kairovia` references only the jobs slice (`Platform.Jobs`, `Platform.Jobs.Hangfire`) and has no `NuGet.config` at all. - The library's own composition root has almost no adoption. Counting distinct external consumer repositories per package: `Platform.Core` 7, `Platform.AspNetCore` 6, `Platform.Testing` 4, `Platform.RateLimiting` 2, `Platform.Idempotency` 2, `Platform.Observability` 2 — but `Platform.Web.Composition` 1, `Platform.Web.Telemetry` 1, `Platform.Testing.AspNetCore` 1, and `Platform.UI.Razor`, `Platform.Http.Resilience`, `Platform.FeatureManagement` and `Platform.Tenant.Lifecycle.AspNetCore` **0**. - The UI story is the same shape twice. `dotnet-platform-libs/ui/` holds a complete DTCG token source (`ui/tokens/tokens.json`), a generator (`ui/scripts/generate-tokens.mjs`) emitting `tokens.css` and `tokens.ts`, a verifier (`ui/scripts/verify-ui.mjs`) that asserts a dark theme, a `prefers-reduced-motion` fallback, `--focus-ring`, `aria-busy` / `aria-invalid` / `aria-live`, `:focus-visible` and a dialog role, a `48rem` responsive breakpoint in `packages/react-ui/styles.css`, and three npm packages (`@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell`, all `0.1.0`) — consumed by **0**. Forge's own `src/ui_pattern/mod.rs` holds a tested catalog of 18 entries (17 active, one deprecated) with per-profile `react-web` / `nextjs-web` / `flutter-app` adapters — also consumed by 0; `forge ui-pattern` appears outside `forge/` only in the workspace `ARCHITECTURE.md`. - The Rust mirror is the same: `rust-platform-libs/crates/` holds 8 crates with **1** external consumer. - Forge itself reaches 3 of 81 repositories (the three carrying a `forge.yaml`: `chronicleaf`, `devloom`, `kairovia`). The result is a control plane whose central promise — deterministic assembly of reusable software — is only exercised on the manifest and the container definition. The reusable layer is the part that ships, and it ships unused. The workspace's own rule is that a capability enters a shared library only with at least two real consumers. The same rule read in reverse says a scaffold must not invent a dependency the library has not proven — which is exactly what "pre-wire everything" would do. This change therefore pre-wires the **proven** set and records the unproven set as visible, non-restoring, evidence-gathering declarations.
## Requirements
### Requirement: Profile-declared shared-layer kit reference

Forge SHALL let a profile declare a versioned shared-layer kit reference as a
compiled-in descriptor resolved entirely offline, and `forge new` SHALL render
that reference into the generated project's own native manifest without
contacting a package feed, a sibling checkout or the network. A profile with no
registered kit for its ecosystem SHALL declare a recorded zero naming the
missing evidence, and a rendered scaffold SHALL NOT leave that state implicit.

#### Scenario: Declared kit is rendered into the native manifest

- **WHEN** an operator runs `forge new <path> --profile <id>` for a profile
  whose descriptor declares a kit reference
- **THEN** Forge stages the ecosystem's manifest carrying the kit's package or
  asset references, records the kit id, version, ecosystem and target framework
  in the generated `forge.yaml` `kit` block, and performs no network access and
  no package restore during generation

#### Scenario: Kit reference is not in the compiled-in registry

- **WHEN** a profile declares a kit id or a kit version that the compiled-in
  registry does not contain
- **THEN** Forge refuses with `kit-unknown` naming the id, the version and the
  registry it consulted, and stages no file and registers no project

#### Scenario: Kit does not match the profile ecosystem

- **WHEN** a profile declares a .NET package list on a Node or Dart profile, or
  any other ecosystem mismatch
- **THEN** Forge refuses with `kit-ecosystem-mismatch` naming both ecosystems
  before any file is written

#### Scenario: Profile has no registered kit

- **WHEN** an operator scaffolds a profile whose ecosystem has no registered
  kit
- **THEN** Forge renders the project normally, records
  `kit.minimum_packages: 0` with a `zero_reason` naming the missing evidence,
  prints the reason as a `WARN`, and does not report the state as a floor
  failure

#### Scenario: Generation never resolves a feed

- **WHEN** a scaffold completes for any profile
- **THEN** Forge has performed no restore, no `npm install`, no `dotnet
  restore` and no `cargo fetch`, and reports native build state as `unverified`
  unless it actually observed a successful native build

### Requirement: Profile-declared minimum consumption floor

Each profile SHALL declare a minimum number of confirmed shared-layer packages
that a scaffold must start with, and `forge new` SHALL refuse before writing
anything when the profile cannot meet it. Forge SHALL NOT offer a
warn-and-continue default, and a refusal SHALL leave no directory, no staged
file and no registered project behind.

#### Scenario: Profile meets its floor

- **WHEN** an operator scaffolds a profile whose confirmed kit set meets or
  exceeds the declared minimum
- **THEN** Forge renders the project, records the confirmed set in the `kit`
  block, and exits zero

#### Scenario: Profile cannot meet its floor

- **WHEN** an operator scaffolds a profile whose confirmed kit set is below the
  declared minimum
- **THEN** Forge refuses with `kit-floor-not-met` naming the profile, the
  declared minimum, the confirmed set it would have rendered and the
  provisional set it would have rendered commented, writes nothing, and does
  not downgrade the refusal to a warning

#### Scenario: Refusal leaves no partial scaffold

- **WHEN** a floor refusal occurs
- **THEN** the destination directory, the staging area and the project registry
  are byte-identical and row-identical to their state before the command

#### Scenario: Declared zero is not a floor failure

- **WHEN** a profile declares `minimum_packages: 0` with a `zero_reason` and the
  operator scaffolds it
- **THEN** Forge renders normally, the manifest and the console output both
  distinguish the recorded zero from a floor failure, and neither reports the
  profile as having met its floor

### Requirement: Explicit and recorded floor exception

An operator SHALL be able to override an unmet floor only by supplying an
explicit non-empty reason, and Forge SHALL record that exception visibly in the
generated project with the floor it bypassed and the set actually declared. An
exception SHALL never be inferred, defaulted or generated by Forge.

#### Scenario: Exception is recorded with its reason

- **WHEN** an operator runs `forge new <path> --profile <id> --kit-exception
  "deliberate greenfield spike"`
- **THEN** Forge renders the project, records `kit.exception` carrying the
  reason, the declared minimum and the declared set in the generated
  `forge.yaml` and the generated README, and prints a `WARN` naming the profile
  and the bypassed floor

#### Scenario: Exception without a reason

- **WHEN** an operator passes `--kit-exception` with an empty or whitespace-only
  reason
- **THEN** Forge refuses with `kit-exception-reason-required` and writes nothing

#### Scenario: Exception cannot be applied to a met floor

- **WHEN** an operator passes `--kit-exception` for a profile that already meets
  its floor
- **THEN** Forge renders the project without recording an exception and reports
  that the floor was met, so an exception is never recorded for a floor that did
  not fail

### Requirement: Named versioned repo-relative feed, never a machine-specific source

A scaffold SHALL declare its shared-layer feed as a **named, versioned source
whose value is a path relative to the generated project**, resolved by the
generated project's own toolchain at restore time with no environment variable,
no machine state and no sibling checkout. Forge SHALL refuse a source-mode
project reference into a sibling checkout and SHALL refuse any feed value that
is a machine-specific absolute path, escapes the project through a `..` segment
or a `file://` URL, or contains a shell metacharacter or secret-shaped string. A
scaffolded project SHALL NOT be rendered with a restore path that only exists on
the machine that generated it, because a runner that does not carry that
machine's state — a CI runner, for instance — is the same failure class as a
hard-coded absolute path.

A feed value read from an environment variable SHALL be refused even when the
variable is documented, because a variable a CI runner does not have is not a
reproducible source.

#### Scenario: Named repo-relative feed is rendered

- **WHEN** Forge renders a scaffold for a profile whose kit declares a named
  feed
- **THEN** the generated package-manager configuration clears inherited
  sources, declares exactly that feed name with a value relative to the
  generated project, declares the neutral public source, and pins every package
  version

#### Scenario: No environment variable is required to restore

- **WHEN** Forge renders a scaffold for a profile whose kit declares a named
  feed
- **THEN** no file in the generated tree resolves the feed through an
  environment variable, a build argument, or a restore-time source override,
  and no file in the generated tree names a feed source that is absent from the
  project

#### Scenario: Machine-specific absolute path is refused

- **WHEN** a kit descriptor or a generated manifest would carry a feed value
  that is an absolute filesystem path
- **THEN** Forge refuses with `kit-feed-invalid` naming the offending shape and
  writes nothing, because a restore that resolves only on the generating machine
  is not reproducible

#### Scenario: Escaping or malformed feed value is refused

- **WHEN** a feed value is not relative to the project — it escapes through a
  `..` segment or a `file://` URL — or contains a shell metacharacter or a
  secret-shaped string
- **THEN** Forge refuses with `kit-feed-invalid` before staging and never echoes
  the value back

#### Scenario: Source-mode project reference is refused

- **WHEN** a scaffold would reference the shared layer as a source-mode project
  or path dependency into a sibling checkout
- **THEN** Forge refuses with `kit-feed-invalid` naming the offending reference,
  so the generated project cannot stop working when the sibling library is
  absent

### Requirement: The feed contents are committed and reproducible

The generated project SHALL carry the `.nupkg` bytes for every package its
pre-wired set resolves, inside the feed directory its own configuration names,
so a fresh clone restores with no sibling checkout, no environment variable and
no secret. Those bytes SHALL be reproducible rather than hand-copied: a `forge`
command SHALL repack the confirmed set from a checked-out sibling library into
the vendored feed, using the sibling's own manifest as the version source of
truth, and SHALL report the confirmed set it packed.

Forge SHALL NOT build, sign or publish the sibling library, and SHALL NOT touch
the sibling checkout beyond reading it and invoking the sibling's own documented
pack step.

#### Scenario: A fresh clone restores with no sibling checkout

- **WHEN** a generated project is checked out at a path that is not the path it
  was generated at, with no environment variable set and no sibling
  `dotnet-platform-libs` present
- **THEN** the project's own `dotnet restore` resolves every pre-wired platform
  package from the committed feed directory its configuration names

#### Scenario: The feed is repacked from the sibling's own manifest

- **WHEN** an operator runs the pack command against a checked-out sibling
  library
- **THEN** Forge reads the sibling's package manifest as the version source of
  truth, packs the confirmed set at that version into the vendored feed, records
  one digest per packed file, and prints the package names and versions it
  packed

#### Scenario: Packing without a sibling checkout is refused

- **WHEN** the pack command is run and the sibling library or its package
  manifest is absent
- **THEN** Forge refuses naming what is missing, changes no vendored file, and
  leaves the existing feed exactly as it was

#### Scenario: The sibling checkout is not modified

- **WHEN** the pack command runs against a sibling library checkout
- **THEN** no file in the sibling checkout is written, moved or removed

### Requirement: The committed feed is verified against the declared kit version

A verification step SHALL fail when the committed feed contents do not match the
kit version a project declares in its `forge.yaml` `kit.version`, or the kit
version the compiled-in descriptor pins. Drift between a declared version and
the committed bytes is the same class of defect as a pinned SDK the CI runner
does not have, so it SHALL be caught by a check rather than by a later restore
failure.

#### Scenario: A matching feed passes

- **WHEN** the verification step runs and every committed feed package is
  declared, present at the declared version, and matches its recorded digest
- **THEN** it reports the kit, the version and the package set it verified, and
  exits zero

#### Scenario: The feed does not match the declared kit version

- **WHEN** a committed feed package's version differs from the `kit.version` a
  project's `forge.yaml` declares, or from the version the compiled-in
  descriptor pins
- **THEN** the verification step fails, naming the package, the declared version
  and the version actually committed, and the drift is never reported as a pass

#### Scenario: A package the confirmed set needs is missing from the feed

- **WHEN** a package the confirmed set resolves is absent from the committed
  feed
- **THEN** the verification step fails naming the missing package, because a
  feed that is merely nonempty is not a feed that restores

#### Scenario: A committed package the kit does not declare

- **WHEN** the committed feed carries a package the kit does not declare for
  that version
- **THEN** the verification step fails naming the undeclared package, so the
  feed cannot silently drift into a different set

#### Scenario: A tampered committed package

- **WHEN** a committed feed package's bytes differ from the digest recorded for
  it
- **THEN** the verification step fails naming the file, the expected digest and
  the actual digest

### Requirement: Pre-wired consumption for the .NET profile

The `aspnet-web` profile SHALL pre-wire the shared-layer packages that both have
at least two distinct external consumer repositories in the compiled-in kit
registry and require no external infrastructure, and SHALL render every other
kit package as a commented, non-restoring declaration naming the evidence it is
missing. Packages that require a database, broker, cache or provider account
SHALL NOT be pre-wired unconditionally.

#### Scenario: Confirmed .NET set is restored by default

- **WHEN** Forge renders an `aspnet-web` scaffold
- **THEN** the generated project manifest references the confirmed kit set
  (`Platform.Core`, `Platform.AspNetCore`, `Platform.Testing`,
  `Platform.RateLimiting`, `Platform.Idempotency`, `Platform.Observability`),
  pins each version centrally, and a restore of the scaffold requires no
  database, broker, cache or provider account

#### Scenario: Below-bar packages render as non-restoring declarations

- **WHEN** Forge renders an `aspnet-web` scaffold
- **THEN** every kit package below the two-consumer bar
  (`Platform.Web.Composition`, `Platform.Web.Telemetry`,
  `Platform.Testing.AspNetCore`, `Platform.UI.Razor`,
  `Platform.Http.Resilience`, `Platform.Web.OpenApi`, `Platform.Web.Cors`,
  `Platform.Web.Resilience`, `Platform.Web.Versioning`,
  `Platform.FeatureManagement`, `Platform.Tenant.Lifecycle.AspNetCore`) appears
  only inside a comment block that names its current external consumer count, so
  it neither restores nor counts toward the floor

#### Scenario: Infrastructure-bearing packages are not pre-wired

- **WHEN** Forge renders an `aspnet-web` scaffold
- **THEN** no identity, persistence, tenancy, caching, jobs, billing, mailing,
  storage or AI package is referenced unless the operator explicitly requested
  the corresponding feature, and referencing a package never grants the
  capability it implements

#### Scenario: Provisional package does not satisfy the floor

- **WHEN** an `aspnet-web` render is evaluated against the declared floor
- **THEN** only packages in the confirmed set count toward the minimum, and a
  confirmed set that is short of the minimum still refuses with
  `kit-floor-not-met`

### Requirement: One design-token source for the presentation layer

A scaffold SHALL receive its design tokens from exactly one registered token
source, vendored as digest-pinned ordinary source with an ownership receipt, and
the semantic UI pattern catalog SHALL supply pattern behavior without defining
any colour, spacing, radius or typography value of its own. A profile with no
registered token kit SHALL declare that absence rather than synthesize a
replacement palette.

#### Scenario: Token artifacts are vendored and receipted

- **WHEN** Forge renders a `react-web` or `nextjs-web` scaffold
- **THEN** the generated tree contains the token artifacts from the registered
  kit under the owned subtree, the ownership receipt records one digest per
  owned file, and the project's own build and test commands continue to run
  offline with no registry dependency added

#### Scenario: Pattern catalog supplies behavior, not tokens

- **WHEN** a semantic UI pattern is installed into a scaffolded project
- **THEN** the installed pattern consumes the project's vendored tokens and
  Forge emits no second palette, spacing scale or typography definition

#### Scenario: No token kit for the profile

- **WHEN** an operator scaffolds `flutter-app`, for which no token kit is
  registered
- **THEN** Forge records a token absence with a reason, keeps the pattern layer
  available through its existing `flutter-app` adapter, and does not synthesize
  Dart tokens or substitute copied web markup

#### Scenario: Edited owned token file conflicts on upgrade

- **WHEN** an owned token artifact was edited and a kit version bump would
  replace it
- **THEN** the upgrade refuses with the existing ownership-conflict code and
  leaves the edited file untouched

### Requirement: Deterministic rendering and asset digest verification

Generation SHALL remain deterministic: identical inputs and pinned assets SHALL
produce byte-identical output, and Forge SHALL verify every rendered and
vendored kit asset against the digest recorded in the compiled-in descriptor
before it reports success. Forge SHALL fail its own tests when a vendored asset
digest drifts, when a profile declares a kit with no registry row, or when a kit
declares a package the registry does not classify.

#### Scenario: Repeated render is byte-identical

- **WHEN** the same `forge new` request is run twice, and once more through
  interactive input with equivalent answers
- **THEN** the two flag-driven outputs and the interactive output are
  byte-identical except documented identity fields, including the rendered kit
  manifest and the ownership receipt

#### Scenario: Vendored asset digest drift

- **WHEN** a vendored kit asset's bytes differ from the digest recorded in the
  kit manifest
- **THEN** generation fails with `kit-digest-mismatch` naming the file, the
  expected digest and the actual digest, and stages no project

#### Scenario: Unregistered kit or unclassified package

- **WHEN** a profile declares a kit with no registry row, or a kit declares a
  package the registry does not classify as confirmed or provisional
- **THEN** Forge's own completeness test fails naming the profile and the kit
  rather than letting the surface go unaccounted

#### Scenario: Generated project operates without Forge

- **WHEN** a generated project is built and tested with its own native
  toolchain and the Forge binary absent
- **THEN** the build and applicable tests succeed, and the project carries no
  Forge runtime dependency

### Requirement: Kit version upgrade is explicit and never implicit

A scaffolded project SHALL pin its shared-layer kit version at generation time
and SHALL NOT be rewritten by a later kit version, a Forge command, a doctor
check, or any background process. Moving a project to a new kit version SHALL
require an explicit upgrade that produces a reviewable diff and refuses when an
owned file was edited.

#### Scenario: New kit version does not touch an existing project

- **WHEN** a new kit version is registered and an already-scaffolded project is
  inspected, listed or health-checked
- **THEN** the project still reports its originally pinned kit version and its
  generated tree is byte-identical to its state before the version change

#### Scenario: Explicit upgrade produces a reviewable diff

- **WHEN** an operator runs the explicit kit upgrade for a project whose owned
  files are unmodified
- **THEN** Forge shows a per-file diff against the new descriptor, applies the
  change on confirmation, and records the new pinned version

#### Scenario: Modified owned file refuses the upgrade

- **WHEN** an owned kit file was edited and an upgrade would change it
- **THEN** the upgrade refuses with the existing ownership-conflict code and
  leaves the edited file untouched

#### Scenario: Upgrade path unavailable

- **WHEN** no upgrade path is available for a project's ecosystem
- **THEN** Forge reports the upgrade as unavailable with a reason, and the
  project remains pinned to its current version rather than being reported as
  current or silently updated

### Requirement: Vendored token mirror is revision-synced with the kit source

The vendored `platform-ui-web` mirror SHALL be one atomic record of one kit
source revision: the per-file bytes under `kits/`, the `kits/manifest.json`
records (source revision, released kit version, per-file digests), the
compiled-in descriptor digests and the registered kit version constant SHALL
all describe the same kit release. After a resync the vendored kit verifier
SHALL pass over the new bytes, and any disagreement between the mirrored
bytes and their manifest or descriptor records SHALL fail generation with
`kit-digest-mismatch` naming the file and both digests.

#### Scenario: Resynced mirror verifies

- **WHEN** the token artifacts are re-copied from a new kit release and the
  manifest, descriptor digests and kit version constant are updated together
- **THEN** `node kits/scripts/verify-tokens.mjs` passes over the vendored pair,
  the vendored-asset digest check passes, and a `react-web` render receipts the
  new bytes with their newly measured digests

#### Scenario: Half-applied resync fails closed

- **WHEN** mirrored bytes are updated but a manifest or compiled-in descriptor
  record still names the previous revision's digest
- **THEN** generation and Forge's own digest verification fail with
  `kit-digest-mismatch` naming the drifted file, the expected digest and the
  actual digest, and no project is staged

#### Scenario: Pinned version matches the generation version

- **WHEN** a `react-web` or `nextjs-web` project is scaffolded after the resync
- **THEN** the rendered `forge.yaml` `kit.version` and the ownership receipt
  `version` equal the kit release the vendored artifacts were generated at, and
  `forge kit upgrade --to platform-ui-web@<that version>` resolves against the
  compiled-in registry
