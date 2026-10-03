# Design: A scaffold starts at high consumption of the registered shared layer

## 1. Implementation boundary

Repository `/home/paul/code/forge`, Rust 2021, MSRV 1.87.

- **New module `src/kit/`**: `mod.rs` (exports, `KIT_CONTRACT_VERSION`),
  `registry.rs` (the compiled-in kit descriptors and their lookup),
  `floor.rs` (floor arithmetic, confirmed/provisional classification, exception
  records), `feed.rs` (repo-relative named-feed rendering, feed-value
  validation, feed-version verification and the pack/verify commands),
  `assets.rs` (digest-pinned vendored asset access for the `kits/` tree).
- **New directory `kits/`** beside `templates/`, holding the digest-pinned
  vendored UI artifacts (`tokens.css`, `tokens.ts`, the kit verifier) and the
  digest-pinned committed `.NET` feed (`kits/feed/*.nupkg`) plus a
  `kits/manifest.json` recording the source revision and the sha256 of every
  vendored file. This is deliberately the same shape as `contracts/manifest.json`
  in `platform-contract-consumption` — the digest discipline is already
  established in this repository and is reused rather than reinvented.
- **Extension points** are exactly: `src/profile/mod.rs` (one additive
  `#[serde(default)] kit: Option<KitReference>` field on `ProfileDescriptor`),
  `src/generate/mod.rs` (the `aspnet-web`, `react-web` and `nextjs-web` arms gain
  their pre-wired manifest blocks; the staging and promotion tails gain the
  binary feed vendoring; the `kits/` ownership receipt), `src/registry/mod.rs`
  (one additive observed field carrying the pinned kit id and version),
  `src/core/mod.rs` (six additive typed errors plus their `code()` arms), and
  `src/main.rs` (one additive `--kit-exception <reason>` flag on the existing
  `New` command, one additive block in `forge profile inspect`, and — an
  explicit exception to the rule below — one additive `forge kit` verb carrying
  `pack` and `verify`). No changes to `src/studio/`, `src/publish/`,
  `src/agent/`, `src/api/`, `src/mcp/`, `src/gate/` or `src/portal/`.
- **No new SQLite table and no new migration.** The pinned kit id and version
  are additive fields on the existing project record, which already carries the
  profile, maturity and observation JSON.
- **No new dependency in `Cargo.toml`.** The vendored-asset digest check reuses
  the `sha2` crate already used by `src/standard/mod.rs` and
  `src/contract/`. The feed-value validator is hand-written and needs no XML or
  package-manager library.
- **No new MCP tool, API route or portal section, and no new top-level CLI verb
  except `forge kit`.** The one exception is deliberate and recorded here: the
  feed is committed, so it needs a reproducible way to regenerate it and a check
  that catches drift. Both are covered by `cargo test`, which a shell script
  beside `kits/` is not, and a drift caught only by hand is a drift CI misses —
  which is exactly how the `global.json`-versus-CI defect reached production.
  `forge kit` reads and writes nothing outside this repository and touches no
  network. Its subcommands are `pack`, `verify` and `upgrade`; all three are
  subcommands of that one verb, not three new top-level verbs, and `upgrade`
  exists because the "explicit kit upgrade" requirement is unsatisfiable while
  the only caller is a `#[test]`. A capability no operator can invoke is not an
  explicit path — it is an implicit one, reachable only by whoever edits the
  test.

## 2. Language and runtime

Rust (`cargo` workspace, edition 2021) for Forge. The generated projects keep
their own native toolchains: .NET 10 SDK for `aspnet-web`, Node/npm for
`react-web` and `nextjs-web`, Cargo for `rust-web`, Flutter for `flutter-app`,
Python for `python-service`. Forge performs no restore and no build for a
generated project; it renders text and verifies digests. Verification for this
package is `cargo fmt --all -- --check`, focused `cargo test kit`, the
generator render-fixture tests, and Forge strict OpenSpec validation.

## 3. The kit reference contract

A `KitReference` is a **compiled-in** descriptor, declared on
`ProfileDescriptor` exactly as profiles, features and standard packs are
declared today. Nothing is fetched at render time.

```text
KitReference {
  id: String,                  // e.g. "platform-dotnet", "platform-ui-web", "none"
  version: String,             // e.g. "0.1.0"; "none" carries no version
  ecosystem: Ecosystem,        // Dotnet | Npm | Cargo | Pub | None
  feed: Option<FeedRef>,       // named source; None for a declared zero
  tfm: Option<String>,         // "net10.0" for Dotnet; None otherwise
  confirmed: Vec<String>,      // packages with >= 2 distinct external consumers
  provisional: Vec<String>,    // 0-1 consumer; rendered commented and non-restoring
  assets: Vec<AssetRef>,       // digest-pinned vendored artifacts (tokens, feed)
  minimum_packages: usize,     // the declared consumption floor
  zero_reason: Option<String>, // set only when minimum_packages == 0
}
```

`FeedRef { name, kind, path }` names a feed and gives its location **as a path
relative to the generated project** (`packages/platform-feed`). It never carries
a machine path, and it never carries an environment variable: the first
implementation's `env: String` field was removed with its mechanism. The value
the generated project's own toolchain resolves is the path plus the committed
bytes, both of which travel with the repository.

A kit is **confirmed** when the compiled-in registry records at least two
distinct external consumer repositories for the package. This mirrors the
workspace's own two-consumer rule and is what keeps the scaffold honest: Forge
counts consumers, not the package's existence. The counts this package ships
as the initial registry are the ones measured in the proposal.

## 4. What each profile pre-wires

| Profile | Kit | Confirmed set (floor) | Provisional / declared |
|---|---|---|---|
| `aspnet-web` | `platform-dotnet@0.1.0`, `net10.0` | `Platform.Core` (7), `Platform.AspNetCore` (6), `Platform.Testing` (4), `Platform.RateLimiting` (2), `Platform.Idempotency` (2), `Platform.Observability` (2) — floor **6** | Commented block naming `Platform.Web.Composition` (1), `Platform.Web.Telemetry` (1), `Platform.Testing.AspNetCore` (1), `Platform.UI.Razor` (0), `Platform.Http.Resilience` (0), `Platform.FeatureManagement` (0), `Platform.Tenant.Lifecycle.AspNetCore` (0) and each one's missing second consumer |
| `react-web` | `platform-ui-web@0.1.0` | vendored `tokens.css` + `tokens.ts` — floor **1** asset set | `@platform/react-ui` / `react-shell` stay commented (npm packages, 0 consumers) |
| `nextjs-web` | `platform-ui-web@0.1.0` | same vendored token assets — floor **1** asset set | same |
| `flutter-app` | `none` | floor **0**, `zero_reason`: no registered token kit for `pub` | `ui_pattern`'s `flutter-app` adapter remains the behavior layer |
| `rust-web` | `none` | floor **0**, `zero_reason`: no `rust-platform-libs` crate has two external consumers (1 consumer: `trailCrew`) | the 8 crates stay commented-out `Cargo.toml` entries |
| `python-service` | `none` | floor **0**, `zero_reason`: no registered kit for this ecosystem | none |

`aspnet-web` deliberately excludes `Platform.Identity.AspNetCore` (3 consumers)
and `Platform.Persistence.EfCore` (4 consumers) from the confirmed set even
though both clear the two-consumer bar: both require a store, and a scaffold
that references a persistence package without a database is exactly the
"invented dependency" this change exists to prevent. They stay behind the
existing `--feature` mechanism.

The confirmed set was chosen so that **every** member's only project references
are `Platform.Core` and `Platform.Web.Telemetry`. That means a restored
`aspnet-web` scaffold needs no database, no broker, no cache server and no
provider account — which is what keeps the offline-build story and the
"portable generated project" boundary intact.

## 5. The distribution decision

**Ruling: a scaffold declares a named, versioned feed whose value is a path
relative to the generated project, and the feed's bytes are committed into the
generated tree. Source-mode `ProjectReference`, machine-specific absolute paths
and environment-variable-resolved feeds are refused.**

The candidate mechanisms, with the evidence:

| Mechanism | Precedent in this workspace | Verdict |
|---|---|---|
| Published feed (nuget.org / npm registry / crates.io) | none — the library packs at `0.1.0` with no evidence of a published feed or a release entry that proves one | the destination, not today's answer |
| Committed local feed at a repo-relative path | `therapist-commons/nuget.config` → `<clear />` + `<add key="local-platform" value=".packages/local-feed" />` + nuget.org, 21 committed `.nupkg` and a `scripts/pack-platform.sh` to regenerate; `citylens/nuget.config` → `artifacts/platform-feed`, 6 committed `.nupkg` | **selected** — the only multi-consumer precedent, and byte-reproducible |
| Source-mode `ProjectReference` | `mewo/Directory.Build.props` | **refused** — couples a generated project to a sibling checkout, so the project stops working when `dotnet-platform-libs` is absent |
| Machine-specific absolute path | `kairovia` (no `NuGet.config`; restores from a path that exists on one machine) | **refused, explicitly** — this is non-reproducible: two operators running the same scaffold get different trees, and CI cannot resolve it |
| Named feed resolved from an environment variable | **this package's first implementation**; no precedent in the workspace | **rejected — see below** |

### The environment-variable mechanism: considered and rejected

The first implementation of this package wrote a `NuGet.config` carrying the
neutral public source and declared the named `platform` feed in
`Directory.Packages.props` with
`<RestoreAdditionalProjectSources Include="$(NUGET_PLATFORM_FEED)" />`.

**The owner rejected it.** The owner's requirement is that a project must not
rely on a physical project path, and that a concrete project must still build
when it does not reside on that machine — a CI runner being the named case.

The rejection reason, recorded so it is not re-proposed: an environment variable
a CI runner does not carry is the *same failure class* as a hard-coded absolute
path. Both make the restore depend on state that exists on one machine and
nowhere else. The variable was not a portability mechanism; it was a portable
spelling of the same defect, and it deferred the failure from a visible missing
path to an invisible unset variable. The existing consumers do not do this:
`therapist-commons` and `citylens` both commit the `.nupkg` bytes into the
repository and name a repo-relative directory. Matching the two consumers that
already work was the correct answer, and the workspace convention was already
the answer to a question this package had re-opened.

The mechanism, its constant, its `props` block and the comments justifying it
were removed rather than left dormant, so no stale justification survives.

What the ruling means concretely per ecosystem:

- **.NET** — `forge new` writes a `NuGet.config` with `<clear />`, one named
  source whose value is the project-relative feed directory
  (`packages/platform-feed`), and the neutral public source, plus a
  `Directory.Packages.props` with `ManagePackageVersionsCentrally` and one
  `PackageVersion` per kit package. The `.nupkg` bytes for the whole restore
  closure are written into that directory. A path that is absolute, escapes the
  project through `..` or a `file://` URL, contains a shell metacharacter or a
  secret-shaped string, or is read from an environment variable is refused with
  `kit-feed-invalid` before staging.
- **npm** — `package.json` pins exact versions; the token artifacts are
  **vendored as ordinary source** rather than added as registry dependencies,
  so the existing offline `npm run build` / `npm test` contract is preserved and
  no scaffold acquires a new network requirement to build.
- **Cargo** — `[workspace.dependencies]` pins `=` exact versions. A path
  dependency into `rust-platform-libs` is refused by the same feed validator:
  that is the `kairovia` failure mode in a different language.

**Source-mode references are checked where they are rendered, not only where
they are described.** The feed-value validator classifies a *feed value*; it
cannot see a `ProjectReference` or a `file:` dependency, which is why the
original shape left `refuse_source_reference` with no production caller and the
requirement unimplemented while it looked done. `find_source_reference` scans
every rendered manifest in `template_files` before anything is staged, and
refuses a reference that is absolute, `~`-rooted, Windows-absolute, or escapes
the project through `..`. A reference that stays **inside** the generated
project is allowed: a solution with its own test project is portable, and
refusing it would make the profile unscaffoldable.
- **pub** — no kit; `flutter-app` declares a recorded zero.

Forge performs no restore. `dotnet restore`, `npm install` and `cargo fetch`
are the generated project's own steps, run by its own toolchain, and Forge
reports them `unverified` when it did not observe them.

### The restore closure is larger than the confirmed set

The confirmed set is `6` packages, but a restore resolves the **transitive
closure**, not the direct references. Measured from
`dotnet-platform-libs/eng/package-manifest.json`, the closure of the six
confirmed packages is **9** packages: the six, plus `Platform.Billing.Contracts`,
`Platform.Eventing` and `Platform.Web.Telemetry`, which arrive as project
references of `Platform.Testing` and `Platform.Observability`. A feed holding
only the six would fail restore, so the feed carries the closure and the pack
command packs the closure. The floor still counts the confirmed set only:
`Platform.Billing.Contracts` and `Platform.Eventing` remain below the bar and
are still rendered commented, never as direct references.

The nine packages total about 268 KB of `.nupkg`, against the
`therapist-commons` precedent of 21 committed packages. The cost is a binary
blob per scaffold, and it is accepted: a blob that is present is what makes a
fresh clone restore. Symbol packages (`.snupkg`) are not committed — they are
not needed to restore, and `citylens` commits none.

### The pack command

The feed is reproducible, not hand-copied. `forge kit pack` reads a checked-out
sibling library, takes the version from the sibling's own
`eng/package-manifest.json` (its declared source of truth), and packs the
closure into the vendored feed. It writes only inside this repository, invokes
`dotnet pack` with an argument array, and refuses — changing nothing — when the
sibling or its manifest is absent.

The alternative, a `scripts/pack-platform.sh` beside `kits/`, was not taken: a
shell script cannot be covered by `cargo test`, and the drift this package must
catch is exactly the drift a test catches. The command is a deliberate
exception to `design.md` §1's "no new top-level CLI verb", recorded there.

### The explicit upgrade, and where the pin lives

`forge kit upgrade <path> --to <kit@version>` is the one sanctioned way a pinned
project moves between kit versions. Without `--confirm` it writes nothing and
prints the reviewable per-file diff, so the default is review and applying is
the explicit act. `--force` is the only way to replace an owned file the
operator edited; without it the upgrade reuses the existing ownership-conflict
refusal and leaves the edited file exactly as written. A project with no kit to
move — a declared zero, or a target this build does not register — is reported
as **unavailable with a reason** and exits non-zero, because "nothing to do,
already current" is the single outcome that state must never produce.

Three files carry the pin and all three move together: the owned asset files,
`.platform/receipt.json`, and the `kit.version` line in the project's own
`forge.yaml`. The manifest line is rewritten as a **targeted line edit**, never
a YAML round-trip — the manifest is a deterministic render, and re-serializing
it to change one field would reorder and reformat everything else. It matters
because `forge kit verify <path>` deliberately reads the version the project
*says* it pins: an upgrade that moved the bytes and the receipt but left the
declared line behind would hand back a project that fails its own drift gate.
`src/registry/mod.rs` is not a fourth writer — its `kit_id`/`kit_version`
columns are an observation of the manifest, and the existing `observe` path
picks the new value up.

### The drift check

`forge kit verify` is the anti-drift gate, and `tests/kit_contract.rs` runs it,
because a drift that is only checked by hand is a drift CI will not catch —
which is precisely how the `global.json`-versus-CI defect reached production. It
fails when a committed package's version differs from the declared `kit.version`,
when a needed package is missing, when an undeclared package is present, or when
a committed byte differs from its recorded digest.

The digest arm is the one that needed care. It only runs where a digest is
actually **recorded**, which is the vendored `kits/` tree: a generated project
commits its own `.nupkg` bytes and records no digest for them. The original
shape — an `if feed_dir.starts_with(kits_dir())` guard with no reporting — meant
a project feed was version-checked and the byte check was skipped in silence, so
a green result said nothing about whether the bytes had been verified. The
record is now a parameter (`verify_committed_feed_with_digests`) and the report
carries `digests_verified`, so a skipped check is visible instead of implied.

### The sibling is read-only, by construction

`forge kit pack` does not build in the sibling. It copies the sibling's `src/`
tree — without any `obj/`, `bin/` or other build-output directory — plus every
regular file at the sibling's root, into a scratch tree outside both
checkouts, and runs `dotnet pack` there. The scratch tree is removed on success
and on refusal.

Redirecting MSBuild's output roots
(`BaseIntermediateOutputPath`, `MSBuildProjectExtensionsPath`, `BaseOutputPath`)
was implemented first and **rejected on evidence**: it does not work, because
the default `**/*.cs` item glob still reads the sibling's own `obj/`. On a
sibling carrying a stale `net8.0` output the build then compiles two copies of
the same generated assembly attributes and fails with duplicate-attribute
errors. Copying the tree removes the problem rather than working around it, and
makes "no file in the sibling is written, moved or removed" true by
construction instead of by remembering enough flags.

Root files are copied wholesale rather than named. The sibling's
`Directory.Build.props` sets `PackageReadmeFile`, so a copy without the root
`README.md` fails with `NU5039`; a hand-kept list of required root files is a
list that goes stale silently. The reported sibling path stays the real one, so
provenance in `kits/manifest.json` still names the library that was packed.


## 6. The floor and its enforcement

A profile's `minimum_packages` is a hard input to `forge new`.

- **Met** — generation proceeds normally.
- **Unmet** — `forge new` refuses with `kit-floor-not-met` **before anything is
  staged or written**, and the message names the profile, the floor, the
  confirmed set it would have rendered, and the provisional packages it would
  have rendered commented. There is no warn-and-continue path. A warn-only
  default would restore exactly the silent zero this change removes.
- **`--kit-exception <reason>`** — records a visible, dated exception in the
  generated `forge.yaml` (`kit.exception: {reason, recorded_at, floor,
  declared}`) and in the generated README, and prints a `WARN`. The reason is
  mandatory: `--kit-exception` without a non-empty reason is
  `kit-exception-reason-required`, because an exception nobody wrote down is
  the same as no exception.
- **A declared zero is not a floor failure.** `flutter-app`, `rust-web` and
  `python-service` declare `minimum_packages: 0` with a `zero_reason` naming the
  missing evidence. `forge new` renders them normally and prints the reason. The
  two states are distinct in the manifest and are never conflated in output.

**Open decision 2 — the floor value.** The number `6` for `aspnet-web` and `1`
asset set for the JS profiles is a proposal, not a derivation. The
implementation must make the value a single declared field so the owner can
change it without touching the mechanism. A floor of `6` means "every confirmed
package with no infrastructure requirement"; a stricter owner may prefer `3`
(the three with 4+ consumers), a looser one may accept the 2-consumer set plus
`Platform.Web.Composition` as provisional-but-rendering.

## 7. The presentation layer: one token source

There are two candidate token sources and one pattern source. The rule is that
**there is exactly one token source and the pattern catalog defines no token
value.**

- `dotnet-platform-libs/ui` is the token source of truth: DTCG
  `ui/tokens/tokens.json`, a generator emitting `tokens.css` and `tokens.ts`, a
  verifier asserting the dark theme, the `prefers-reduced-motion` fallback,
  `--focus-ring`, the aria markers, `:focus-visible` and the dialog role, and a
  `48rem` breakpoint in `packages/react-ui/styles.css`. It is the only candidate
  with a generator and a verifier, so it wins on evidence rather than on
  preference.
- `forge/src/ui_pattern/mod.rs` is the **behavior** source: 18 catalog entries
  (17 active, one deprecated) covering states, accessibility, navigation, form
  semantics and the `react-web` / `nextjs-web` / `flutter-app` adapters. Per
  `requirement.md` §13 — "the UI registry should describe semantic patterns, not
  merely copied HTML" — it defines no colour, spacing, radius or typography
  value.
- `forge new` **vendors the generated artifacts**, not the DTCG file, into
  `.platform/tokens/{tokens.css,tokens.ts}` and records one digest per owned
  file in a receipt. The DTCG source stays upstream-owned; the vendored copy is
  what the project compiles against. This mirrors the
  `deterministic-project-generation` ownership model and keeps the project
  buildable offline.
- A pattern adapter installed into a scaffolded project consumes the vendored
  tokens. A profile with no token kit (`flutter-app`) declares the reason and
  keeps only the pattern layer, rather than inventing Dart tokens — this is the
  same "declared zero" rule as §6.
- A fourth token source is prohibited: Forge never synthesises a palette, and a
  scaffold's `forge.yaml` records exactly one `kit.assets` token source.

**Open decision 4 — one receipt or two.** This design recommends reusing the
existing `.standard/` receipt, digest, `diff` and `upgrade` machinery for the
`.platform/` subtree rather than introducing a second ownership system. The
obstacle is that `dotnet-platform-libs` does not yet expose a standard-pack
descriptor that Forge could select by `--standard-pack`, so reuse requires a
sibling-side descriptor. If the owner prefers no sibling dependency, a separate
`.platform/receipt.json` is a small, contained alternative.

## 8. Compatibility and determinism

- **Determinism.** The kit reference is compiled in, exactly like a profile or a
  feature. Identical inputs produce byte-identical output. Nothing is fetched,
  resolved or discovered at render time, so the README's "Rendering is verified
  against the asset version" claim holds and is extended: the rendered kit
  manifest is additionally verified against the compiled-in descriptor digest.
- **Digest drift.** `kits/manifest.json` records the source revision and the
  sha256 of every vendored file. A digest test fails naming the file, the
  expected digest and the actual digest — the same discipline as
  `platform-contract-consumption`. A tampered asset fails generation with
  `kit-digest-mismatch` rather than producing a project that silently differs
  from its descriptor.
- **Completeness.** A test fails when a profile declares a kit with no registry
  row, and when a kit declares a package the registry does not classify. This is
  the "unregistered versioned surface" discipline from
  `platform-contract-consumption` applied to kits.
- **Native verification.** Rendering a manifest is not restoring it. `forge new`
  reports `native_verified: false` with the reason unless it actually observed a
  successful native build, exactly as `verify_native` reports today.
- **Product boundary.** A generated project still builds and operates through
  `dotnet build` / `npm run build` / `cargo build` / `flutter analyze` with Forge
  absent. The kit reference is a manifest entry plus committed feed bytes, not a
  Forge runtime dependency. The feed travels with the repository, so the project
  restores at any path with no sibling library and no environment variable; if
  the feed bytes are nonetheless removed or corrupted, the project's own tooling
  fails in its own terms — Forge is not in the path, and Forge does not claim
  the build passed.
- **Kit version bump flow.** A scaffolded project pins its kit version at
  generation time and is **never** mutated by a bump. The flow is:
  1. The library publishes a new version.
  2. `forge upgrade` on an existing project produces a **reviewable diff**
     against the new descriptor and checks every owned-file digest.
  3. If an owned file was edited, the upgrade refuses with the existing
     ownership-conflict code and leaves the file untouched.
  4. Nothing happens implicitly: no `forge new`, no `forge doctor`, no CI run
     and no background process rewrites a pinned project.
- **Registry observation.** The project record gains the pinned kit id and
  version as additive observed fields. No existing field changes meaning and no
  table migrates.

## 9. The target-framework blocker — **RESOLVED**

Every project in `dotnet-platform-libs/src/` declares
`<TargetFramework>net10.0</TargetFramework>`, and
`Directory.Packages.props` pins `Microsoft.Extensions.*` at `10.0.0`. The
`aspnet-web` scaffold declared `net8.0`. **A `net8.0` project cannot reference a
`net10.0` package**, so a scaffold that declared the confirmed set as-is would
not restore.

The kit reference therefore carries `tfm`, and generation renders the TFM the
kit requires rather than the profile's historical hard-coded value.

**Owner ruling — open decision 3 is closed.** The workspace is .NET 10 and SDK 8
is being removed:

- `/home/paul/code/global.json` pins `10.0.400` with `rollForward: latestFeature`.
- `/home/paul/code/Directory.Build.props` baseline is `net10.0`.
- All 75 `dotnet-platform-libs` packages are `net10.0` and build clean.

**Option A was taken.** The `aspnet-web` descriptor's TFM became `net10.0`,
along with its `toolchain_version` and the scaffold `Dockerfile`'s SDK base
image. `dotnet-platform-libs` was **not** multi-targeted down: that would push 75
packages' support matrix into a sibling repository the owner has ruled is
moving to .NET 10 anyway. `deterministic-project-generation` requires
determinism, not a fixed TFM, so this does not break that contract — it does
change bytes existing operators may have pinned, and that change is recorded
here rather than absorbed silently. The blocker that was recorded as a hard
stop before this package is **discharged**.

The `aspnet-saas` planned descriptor still names `8.0`. It is a roadmap
placeholder that refuses generation, is outside the scaffold surface, and is
deliberately left alone: changing it is out of scope for this package.

## 10. Failure and boundary policy

| Condition | Typed error | State written |
|---|---|---|
| Kit id or version absent from the compiled-in registry | `kit-unknown` | nothing |
| .NET package list on a Node profile (or any ecosystem mismatch) | `kit-ecosystem-mismatch` | nothing |
| Feed value is absolute, escapes the project through `..` or `file://`, contains a metacharacter, or is secret-shaped | `kit-feed-invalid` | nothing |
| Feed value is read from an environment variable rather than a project-relative path | `kit-feed-invalid` | nothing |
| Declared floor unmet | `kit-floor-not-met` | nothing; the message names the floor, the confirmed set and the provisional set |
| `--kit-exception` without a reason | `kit-exception-reason-required` | nothing |
| Vendored asset digest differs from the descriptor | `kit-digest-mismatch` | nothing; names file, expected and actual digest |
| Committed feed version differs from the declared `kit.version` | `kit-feed-version-mismatch` | nothing; names package, declared version and committed version |
| Committed feed is missing a package the confirmed set resolves, or carries an undeclared one | `kit-feed-incomplete` | nothing; names the package |
| Pack requested with no sibling checkout or no sibling package manifest | `kit-pack-unavailable` | nothing; names what is missing |
| Profile has a declared zero | *(none)* | normal render plus the recorded `zero_reason` and a `WARN` |

Every refusal is a refusal **before** staging, so a failed `forge new` never
leaves a half-wired directory or a registered project row. A declared zero and
a refused floor are always distinguishable in both the manifest and the console
output.

## 11. Verification oracle

- **Render fixtures per profile.** `aspnet-web`, `react-web` and `nextjs-web`
  fixtures assert the exact confirmed package set, assert that every
  provisional package appears inside a comment block, and assert the generated
  `NuGet.config` clears inherited sources, carries the named source with a
  project-relative value, and declares no environment variable.
- **Portability, end to end.** Render an `aspnet-web` scaffold into one
  directory, copy the directory to an unrelated path, and run the project's own
  `dotnet restore` there with the feed environment variable removed and no
  sibling library present. This is the oracle for the owner's requirement, and
  it is reported with the commands and their output rather than as a claim.
- **Feed version agreement.** A test runs the feed verification over the
  committed `kits/feed/` and the compiled-in descriptor, and a test proves the
  verification fails on a version mismatch, a missing package and an undeclared
  package. This is the anti-drift gate; it runs in `cargo test` precisely so CI
  catches what a hand-run check would not.
- **Determinism.** `render_files` byte-equality across repeated calls and
  across flag/interactive inputs, matching the existing
  `deterministic-project-generation` invariant. The binary feed bytes are
  digest-pinned vendored files, so repeated generation is byte-identical there
  too.
- **Digest agreement.** A test over `kits/manifest.json` fails naming the file,
  the expected digest and the actual digest; a profile with a kit and no
  registry row fails completeness.
- **Floor behaviour.** Four tests: met (renders), unmet (typed refusal, nothing
  written), exception with reason (renders, records the exception, prints
  `WARN`), exception without reason (typed refusal).
- **Feed validation.** Absolute path, `file://` escape, metacharacter and
  secret-shaped values each refuse; a named feed renders.
- **Single token source.** A test asserts the vendored `tokens.css` and
  `tokens.ts` match the upstream digests, that a `react-web` scaffold contains
  no second palette definition, and that `ui_pattern` emits no colour value.
- **No-Forge boundary.** A generated project builds and tests with the Forge
  binary absent, using its own toolchain. Where the toolchain is missing, the
  result is reported `unverified` and is never counted as a pass.
- **No network at generation.** The generation path is asserted to perform no
  outbound access; only a text render and a local digest check occur.

## 12. Decision ledger

**Resolved.** A kit reference is a compiled-in, versioned, offline descriptor
declared on the profile. `forge new` renders it into the generated project's
native manifest. A profile declares a minimum-consumption floor; an unmet floor
refuses before anything is written, and the only exception is an explicit,
dated, reasoned one. A declared zero (no proven kit for the ecosystem) is a
first-class, honest state distinct from a floor failure. The feed is named and
versioned; source-mode `ProjectReference` and machine-specific absolute paths
are refused, and `kairovia`'s machine-specific restore is the named example of
what is being refused. The token source of record is `dotnet-platform-libs/ui`,
vendored as digest-pinned ordinary source; `ui_pattern` supplies behaviour and
never a token value; `flutter-app` declares a token zero rather than inventing
Dart tokens. Determinism and the "rendering is verified against the asset
version" claim are preserved and extended to the kit assets. A kit version bump
never mutates an existing project.

**Open, left to the owner.** (3) and (4) are now closed; see below.

### Decision 1 — kit distribution mechanism: **committed repo-relative feed**

**Superseded.** The first implementation took "Option B": a `NuGet.config`
carrying the neutral public source, with the named `platform` feed declared in
`Directory.Packages.props` and resolved from `$(NUGET_PLATFORM_FEED)` through
`RestoreAdditionalProjectSources`. **The owner rejected that mechanism** and the
ruling now reads: a `NuGet.config` with `<clear />`, one named source whose value
is a path relative to the generated project, the neutral public source, and the
`.nupkg` bytes committed into the generated tree.

*Why the rejected option lost:* the owner's requirement is that a project must
build when it does not reside on the machine that generated it. An environment
variable a CI runner does not carry is the same failure class as a hard-coded
absolute path — both make restore depend on state that exists in exactly one
place. The variable deferred the failure instead of removing it, and the two
consumers that already work in this workspace (`therapist-commons`,
`citylens`) commit the bytes and name a repo-relative directory. The
environment-variable mechanism, its constant, its MSBuild block and the comments
that justified it were removed, not left dormant; §5 records the full statement
so the reasoning is not re-litigated.

*What was kept from the rejected option:* the refusal behaviour, which is
identical either way, and the digest discipline. What was lost: a text-only
generated tree. The cost is real and accepted — about 268 KB of `.nupkg` per
`.NET` scaffold — because a blob that is present is the only thing that makes a
fresh clone restore.


### Decision 2 — minimum-consumption floor: **6 / 1 / 0-with-reason, as proposed**

`aspnet-web` declares `6`, which is exactly the confirmed set: every package
with two or more distinct external consumers and no database, broker, cache or
provider account. `react-web` and `nextjs-web` declare `1`, the single
confirmed unit being the digest-pinned vendored token pair. `flutter-app`,
`rust-web` and `python-service` declare `0` with a `zero_reason`.

*Rationale:* `6` is the derived count, not an arbitrary product number, and
making it the exact confirmed-set size means the floor can only be met by
rendering the whole evidence-backed set. The value lives in one declared field
per descriptor, so the owner can change it without touching the mechanism.

### Decision 3 — who moves the target framework first: **Option A, owner-ruled**

Closed by the owner: the workspace is .NET 10, SDK 8 is being removed, and
`dotnet-platform-libs` stays `net10.0`. Forge raises `aspnet-web` rather than
multi-targeting the library down. See §9.

### Decision 4 — one receipt or two: **a `.platform/receipt.json`, mirroring the standard receipt**

The design recommends reusing the `.standard/` receipt, digest, `diff` and
`upgrade` machinery. That reuse requires a standard-pack descriptor owned by
`dotnet-platform-libs`, which is outside this repository's write boundary, so
the sibling-side descriptor cannot be added here.

*Rationale:* the contained alternative was taken, but the shapes are
deliberately identical — the same `ReceiptFile` digest-per-owned-file form, the
same `DiffChange` vocabulary (`added` / `updated` / `unchanged` / `modified` /
`foreign` / `orphaned`), the same read-only-diff-then-confirm apply, the same
rollback-on-failed-write, and the same existing ownership-conflict refusal code.
It is a second instance of one ownership discipline, not a second design. The
generated `README.md` names the subtree so an operator is never confused about
which tree is protected.

### Registry observation — implementation note

The intent recorded in §8 ("no new table, additive observed fields") is
honoured: the pinned kit id and version are two more observed scalars on the
existing `projects` row, read from the manifest's `kit` block. The wording "no
new migration" is honoured in spirit but not literally — a new column needs
the repository's own established additive `ALTER TABLE` mechanism (the same
one that added `operations.idempotency_key`), which is a no-op for a registry
created after this change. No existing field changed meaning and no new table
exists.

**Deferred.** The fleet-wide retrofit of the 81 existing repositories; any Gate
or adoption-gate change; publishing, signing or CI for the shared layer; a
`rust-platform-libs` and a `pub`/Flutter kit; and a kit for `python-service`.
None of these is authorized by this package.
