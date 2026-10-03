# Proposal: Make the shipped UI token layer reachable, single-sourced, and checkable

## Why

`dotnet-platform-libs/ui/` is a real, working design-token layer. It is consumed
by **nothing** — including this repository's two flagship products, neither of
which knows it exists. Meanwhile the fleet has independently re-invented the
smallest possible pieces of it, 15 times over, in ways that disagree.

Measured for this proposal (scope stated in **Evidence boundary** below):

| Asset in this repository | State today | Consumers |
|---|---|---|
| `ui/tokens/tokens.json` | DTCG source, `$value`/`$type`, semantic aliases (`{color.slate.50}`) | generator only |
| `ui/scripts/generate-tokens.mjs` | emits `dist/tokens.css` + `dist/tokens.ts` | — |
| `ui/scripts/verify-ui.mjs` | 6 required artifacts; asserts dark theme (`:22`), reduced-motion (`:23`), `aria-busy`/`aria-invalid`/`aria-live` (`:27`), `:focus-visible` (`:28`), dialog role (`:30`) | runs in `design-tokens` `test` |
| `@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell` | 3 packages, all `0.1.0`; the latter two have tests | **0** |
| `src/Platform.UI.Razor` | in `eng/package-manifest.json` at `0.1.0`, ships `platform-tokens.css` + `platform-ui.css` consuming `var(--color-*)` | **0** |

A workspace grep for `@platform/design-tokens`, `@platform/react-ui` and
`@platform/react-shell` outside `dotnet-platform-libs/ui/` returns **zero**
files**. The two flagship products use a different system entirely:
`devloom/web/components.json` is the **shadcn/ui** config
(`ui.shadcn.com` schema, `style: new-york`, `rsc: true`), and `kairovia`
carries an independent `frontend/`. A third-party design system won inside the
kit that exists to prevent exactly that.

The duplication this would absorb is not stylistic. Four semantically
different `.visually-hidden` definitions, all four bodies verified this
session:

| Repo | Location | Divergence |
|---|---|---|
| `openpanel` | `crates/openpanel-web/assets/app.css:141` | `clip: rect(0 0 0 0)`, space-separated `clip` |
| `cvunify` | `src/CvUnify.Web/wwwroot/css/site.css:388` | comma form `clip`, all 9 properties |
| `arivio` | `src/Arivio.Console/src/styles.css:295` | **omits `padding`/`margin`/`border` — leaks a 1px box** |
| `pangoman` | `src/HistoryKB.Web/wwwroot/css/site.css:94` | every declaration `!important` |

`arivio` is a live accessibility defect, not a style disagreement: a
`visually-hidden` element that still occupies a 1×1px box is not hidden.

The reason the abstraction is missing is now measurable and is the sharpest
finding in this proposal: **the generated `tokens.css` defines no
`.visually-hidden` at all** (`grep -c visually-hidden` → `0`). The shared layer
never shipped the primitive, so 15 repos wrote their own. Nothing in the
workspace checks that, so the fifteenth was free.

Two further defects mean "one token source" is not currently true, and this
change must close them before any reachability work is worth doing:

- **The generator hand-enumerates every token.**
  `ui/scripts/generate-tokens.mjs` does not walk the DTCG tree; it reads
  specific paths (`source.color.slate['50'].$value`,
  `source.space['2'].$value`, …) inside a template literal. Adding
  `color.slate.200` to `tokens.json` produces **no** output and **no** error.
  `verify-ui.mjs:19-21` checks that 6 named tokens exist, so a dropped token is
  not caught. `tokens.json` and the generator template are therefore **two**
  sources today, and the second one wins silently.
- **The dark theme is a second literal palette living only in the generator.**
  `[data-theme="dark"], .theme-dark` hard-codes `#93c5fd`, `#bfdbfe`, `#fca5a5`,
  `#86efac`, `#fcd34d` (5 unique values, `bfdbfe` twice). **None** of the 5
  appears in `tokens.json` (each: 0 matches). A dark mode that is not in the
  token source cannot be themed, and nothing detects it.

The second "common" role is filled twice. `forge/src/ui_pattern/mod.rs`
(2137 lines, 17 tested active entries) owns a semantic UI pattern catalog.
Its boundary against this layer is **real, and this session proved it rather
than asserting it**: across all 2137 lines there are **0** hex color literals
and **0** `px`/`rem`/`em` dimension literals. It holds no values, so it is not
a competing value source. What it does hold is a *declared* token vocabulary
that does not resolve against this repository — `standard_spacing()` returns
`token: "design-token-spacing-4"`, `scale: "4,8,12,16,24,32"` (px-shaped),
whereas the authoritative token is `space.4` / `--space-4` (`1rem`). So the
layer split is real and the **binding is undeclared**. Repairing the binding is
a `forge/` change and is out of this change's write boundary; it is recorded as
Non-goal **NG4** with a named follow-up.

## What Changes

- **Reachability — a committed, repo-relative npm feed.** Add
  `ui/scripts/pack-ui.mjs` (deterministic `npm pack` equivalent over the three
  packages into `ui/feed/*.tgz`) and a committed-feed consumption contract: a
  consumer commits the tarball bytes into its own tree and declares
  `"@platform/design-tokens": "file:vendor/platform-ui/design-tokens-0.1.0.tgz"`.
  No sibling checkout, no absolute path, no environment variable. This is the
  npm translation of the mechanism `forge`'s `scaffold-prewires-shared-layer`
  already settled for the .NET kit, and the reason it is *not* byte-identical
  is in `design.md` §2: npm has no `file:` registry — `NuGet.config`'s
  `<add key value="./feed" />` names a directory NuGet scans for `.nupkg`,
  while npm's unit of resolution is a specifier string inside `package.json`.
  The faithful equivalent of "a named, versioned, project-relative source whose
  bytes are committed" is therefore a `file:` specifier over a committed
  **tarball** — a checksummed byte artifact, matching the committed `.nupkg`
  precedent — not a `file:` directory, which npm satisfies with a symlink whose
  contents can drift without the manifest changing.
- **Drift checking.** `ui/scripts/verify-ui-feed.mjs` fails when a consumer's
  committed feed does not match the kit version its `ui-conformance.json`
  declares, when a needed package is absent, or when an undeclared package is
  present — the same drift class as the `global.json`-versus-CI defect.
- **Make the token source actually singular.** Replace the hand-enumerated
  generator with a DTCG tree walk that resolves `{alias}` references, so every
  `$value` in `tokens.json` is emitted; move the 5 dark-theme literals into
  `tokens.json`; and extend `verify-ui.mjs` to fail when the generated CSS
  contains a literal color/space/radius/font value not reachable from a
  `$value` or a documented `var()` alias. This is a prerequisite, not
  decoration: distributing a file that can silently drop tokens would ship the
  defect fleet-wide.
- **Add the missing primitive.** Emit a canonical `.visually-hidden` (the full
  9-property form, no `!important`) into the generated CSS, so the four
  divergent copies have something correct to converge on.
- **Conformance as absence, not presence.** The check
  `scripts/check-ui-conformance.sh` scans the workspace and **fails a
  repository that declares a shared primitive itself**, subject to one
  explicitly frozen, closed legacy list. A repo is compliant by declaring
  `consumes-feed` (feed path, kit version, tarball digest) or `exempt` (reason,
  owner, review-by) in a one-file `ui-conformance.json`; a UI-bearing repo with
  no declaration fails. The legacy list is **closed at 15** and a guard asserts
  it has not grown — that is the mechanism by which a *new* divergent copy
  fails while the existing 15 are not retroactively blocked. This change adds
  **no** new .NET package: the check is a shell script plus test methods in the
  existing `tests/Platform.Architecture.Tests` project.
- **Cover the Razor majority explicitly.** 5 of the 15 divergent
  `.visually-hidden` definitions are in Razor-Pages repos
  (`cvunify`, `pangoman`, `singleatee`, `stylify`, `testify`) that already
  reference this repository. Razor is therefore **in scope** for the token
  artifact and the conformance check, and needs no new delivery vehicle:
  `platform-tokens.css` is a plain CSS file.
- **State the Flutter boundary in Non-goals**, not by implication. The kit
  ships no Dart tokens, and `forge`'s own scaffold declares
  `flutter-app: tokens: none` for the same reason. 8 Flutter repos ship UI and
  this change does not serve them.

## Package Boundary and Split Assessment

**Single outcome: a consumer outside this repository can resolve the shipped UI
token layer from bytes committed in its own tree, and the workspace can prove
that no repository other than the frozen legacy list declares a shared UI
primitive itself.**

| Aspect | Assessment |
|---|---|
| New .NET package | **None.** Package count stays **75**. See below. |
| New npm package | **None.** The same 3 packages at `0.1.0` are packed and committed; no package is authored. |
| `eng/package-manifest.json` | **Byte-identical.** The manifest records package identity, `packageReferences`, `projectReferences` and target frameworks. This change adds no `.csproj`, no `PackageReference` and no `ProjectReference`; `ui/` is not a .NET project and is not inventoried. |
| `eng/public-api-baseline.txt` | **Unchanged.** No public .NET type is added or altered. `Platform.UI.Razor` is not in `scripts/check-public-api.sh`'s 6-package scan list, and this change does not alter its surface, so no baseline regeneration is required or performed. |
| Architecture guards | Additive test methods in the existing `Platform.Architecture.Tests` project (which already has 137 `[Fact]`/`[Theory]` methods). No guard is deleted, renamed or weakened. |
| Touches `forge/` | **No.** Referenced only. |

**Why no new .NET package.** The obvious candidate is
`Platform.UI.Conformance`, referenced by consumers so they can assert
compliance at build time. Rejected on this repository's own evidence: the
workspace rule is that a capability enters a shared library only with at least
two confirmed consumers, and this one has **zero** consumers of the three
existing UI packages and zero of `Platform.UI.Razor` (`forge`'s
`scaffold-prewires-shared-layer` records `Platform.UI.Razor` at 0). The
conformance contract is also not a library concern — it is a check over
*other repositories' text files*, which a referenced .NET assembly cannot
observe. A package would add a 76th manifest entry to solve a problem a shell
script solves exactly, against a repo where over-factoring is a live concern.

**Split signals considered and rejected.** *Feed packing* and *conformance
scanning* are separately testable, but they share one owner (this repository),
one artifact (the committed feed bytes), one version coordinate (the kit
version) and one oracle (a consumer resolves a feed whose digests match). Split
them and you get a feed nobody can check or a check that verifies nothing.

## BFS Impact Map

- **Capabilities:** new `platform-ui-token-distribution`. `platform-ui-design-system`
  is **consumed, not delta'd** — its shipped requirements concern cross-client
  token equivalence and component interaction states, and none of their wording
  changes. A later change may promote them once distribution has landed
  (`design.md` §8).
- **Code (this repository only):** `ui/scripts/generate-tokens.mjs` (rewritten
  as a DTCG walk), `ui/tokens/tokens.json` (5 dark literals added),
  `ui/scripts/verify-ui.mjs` (completeness + no-orphan-literal assertions,
  canonical primitive), new `ui/scripts/pack-ui.mjs`, new
  `ui/scripts/verify-ui-feed.mjs`, new `scripts/check-ui-conformance.sh`,
  `ui/feed/*.tgz` (new committed bytes), `ui/README.md` and `docs/packages.md`
  (UI rows only).
- **Tests:** `tests/Platform.Architecture.Tests/UiDistributionTests.cs` (new
  file in an existing project) asserting the frozen legacy list has not grown,
  the kit's generated CSS is byte-stable, and no new .NET project references a
  UI npm package. Existing `ui/packages/react-ui/tests/components.test.mjs` and
  the `react-shell` tests must stay green — the generator rewrite must not
  change emitted token *names*.
- **No consumer repository is modified by this change.** This is the single
  most important line in this map. `devloom`, `kairovia`, `openpanel`,
  `cvunify`, `arivio`, `pangoman` and the other 14 are **read** to build the
  legacy baseline and are otherwise untouched. They are not fixed here.
- **Failure surface:** a token in `tokens.json` the generator drops; an orphan
  literal in the generated CSS; a feed whose tarball digest ≠ the declared kit
  version; a consumer declaring `file:` with an absolute or `..`-escaping path
  or an env-var-resolved path; a consumer declaring `consumes-feed` with no
  committed tarball; a UI-bearing repo with no `ui-conformance.json`; a
  divergent primitive outside the frozen list. Each has a named, specified
  outcome in `specs/platform-ui-token-distribution/spec.md`.
- **Compatibility:** additive. `tokens.css` gains declarations and never
  renames one, so an existing consumer of `@platform/design-tokens` 0.1.0 — of
  which there are none — would still resolve. `Platform.UI.Razor` 0.1.0 is
  unchanged. Nothing is deprecated or removed by this change.
- **Out of scope but adjacent:** the shadcn/ui migration of `devloom/web` and
  the `kairovia/frontend` consolidation (NG1); the `arivio` 1px-box defect
  repair and the other 14 legacy repairs (NG2); the `openaccounting` dead
  Tailwind utility defect (NG3); the `forge` token-name binding (NG4).

## Capabilities

### New Capabilities

- `platform-ui-token-distribution`: the shipped UI token layer is resolvable by
  an out-of-repository consumer from bytes committed in the consumer's own
  tree; the token source is singular and its generated artifacts are complete
  and free of orphan literals; the shared UI primitives include a canonical
  visually-hidden rule; and a repository is UI-conformant by **not** declaring a
  shared primitive itself, with the legacy divergence list frozen closed.

### Consumed, unchanged

- `platform-ui-design-system` — the token and component contract this change
  distributes. No requirement wording changes.

## Non-goals

- **NG1 — No adoption of the layer in any existing repository.** Nothing in
  `devloom`, `kairovia` or any other consumer changes. Migrating
  `devloom/web` off shadcn/ui is a separate change in `devloom`'s repository and
  is not authorized here. This change makes the layer *reachable*; it does not
  make it *used*.
- **NG2 — No retroactive repair of the 15 divergent `.visually-hidden` copies,
  including `arivio`'s live 1px-box defect.** They are recorded in a frozen
  legacy list so the check can start from a truthful baseline, and each repair
  is its own change. **Follow-up: `ui-fleet-adoption`** — one OpenSpec change
  per consuming repository, sequenced under this repository's one-active-change
  rule, beginning with `arivio` because it is the only measured *defect*
  rather than a style disagreement.
- **NG3 — No repair of the `openaccounting` dead-utility defect.** 100 files
  (`.html`/`.cshtml` scope) use Tailwind `sm:`/`md:`/`lg:` utilities with no
  `tailwind.config.*`, no `postcss.config.*`, and no Tailwind directive in any
  stylesheet, so those utilities resolve to nothing and the layout does not
  respond. Fixing it means adding a real build step to that repository, which
  is a change there, not here. This change makes such a state *detectable*; it
  does not repair it.
- **NG4 — No change to `forge/`.** `forge`'s `ui_pattern` catalog is the
  behavior layer and is **not** a competing value source (proved: 0 hex, 0
  dimension literals across 2137 lines), so nothing here needs to change it.
  What *is* broken is its binding: `standard_spacing()` emits
  `design-token-spacing-4` with scale `4,8,12,16,24,32` where this
  repository's token is `space.4` / `--space-4`. Binding those names to the
  authoritative namespace is a `forge/` change. **Follow-up:
  `ui-pattern-token-binding` in `forge`**, which is also the change that would
  make the layer boundary machine-checked from both sides.
- **NG5 — Flutter is not served.** 8 Flutter repos ship UI. The kit ships no
  Dart token artifact, and `forge`'s scaffold already declares
  `flutter-app: tokens: none` rather than inventing Dart tokens. No Dart
  codegen, no Flutter package, no Flutter conformance rule is in this change,
  and no coverage of Flutter repos is claimed.
- **NG6 — No registry publish, no CI release, no signing.** The feed is
  committed bytes, not a published npm package. There is no `@platform` scope
  on any registry, no `npm publish`, and no token.
- **NG7 — No Razor delivery vehicle is added.** `Platform.UI.Razor` already
  exists at `0.1.0` and is already in the manifest. Razor consumers reach the
  token artifact by committing `platform-tokens.css`; no new .NET package,
  project, or MSBuild integration is introduced.
- **NG8 — No responsive-scale unification, no dark-mode unification, no
  z-index scale, no skip-link component consolidation.** Measured and real
  (see Evidence boundary), but collapsing them is fleet adoption work, and
  NG2/NG3 already place that outside this change.
- **NG9 — No visual redesign, no page composition, no component catalogue
  growth.** The kit stays a token and primitive layer.

## Sibling reconnaissance

| Sibling | Relationship | Finding | Action |
|---|---|---|---|
| `forge` — `scaffold-prewires-shared-layer` (active) | Decides the same reachability problem for the .NET kit | Ruled: committed `.nupkg` bytes in a project-relative `NuGet.config` source; **explicitly refused** an env-var feed (`NUGET_PLATFORM_FEED`) and any machine-specific absolute path, on the grounds that a variable a CI runner lacks is the same failure class as a hard-coded path. Also ruled `dotnet-platform-libs/ui` the single token source and `ui_pattern` the behavior layer. | **Adopted**, translated to npm (`design.md` §2). Same three refusals. |
| `forge` — `src/ui_pattern/mod.rs` | Second source of the "UI common" role | 0 hex literals, 0 dimension literals in 2137 lines ⇒ **not** a value competitor; boundary is real. Token *names* it emits do not resolve against this repo. | Referenced only (**NG4**). |
| `dotnet-platform-libs` — `platform-ui-design-system` (archived 2026-09-08) | Owns the token/component contract | Shipped the token source, generator and verifier. Said nothing about **resolution** or **conformance** — which is why 0 adoption was possible. | Extended by a new capability, not rewritten. |
| `dotnet-platform-libs` — `shared-test-web-factory` (active) | House precedent for an unreachable-but-real shared asset | Same failure shape (a shipped class 63 local copies bypassed) solved by making the shared thing *usable*, and it split its work into BFS baseline → DFS → BFS regression → Verification. | Same task shape adopted (`tasks.md`). |
| `dotnet-platform-libs` — `platform-consumer-bootstrap` (archived) | Ruled the .NET distribution mechanism | Feeds declared per consuming project, never a machine path. | Consistent; no conflict. |
| `devloom/web` | Flagship consumer | Ships shadcn/ui `components.json` (`style: new-york`, `rsc: true`) — a third-party system inside the kit. | Read only (**NG1**). |
| `kairovia` | Flagship consumer | Independent `frontend/`, no reference to the shared layer. | Read only (**NG1**). |
| 15 repos defining `.visually-hidden` | Baseline for the frozen legacy list | 4 semantically different bodies, verified individually. | Read only (**NG2**). |

## Evidence boundary

- **Measurement scope.** Two scopes are used and are never mixed. **CSS scope**
  = `.css .scss .less`, excluding `node_modules/ .git/ obj/ bin/ target/
  dist/ .next/ .dart_tool/ vendor/ .venv/ site-packages/`; **wide scope** =
  CSS scope plus `.ts .tsx .html .vue .svelte .razor .cshtml`. Files over
  200 KB, or under 5 lines and over 20 KB, are skipped as minified vendor
  bundles. This exclusion is deliberate: an unfiltered workspace grep returns
  a vendored Bootstrap bundle that inflates the media-query count roughly
  twenty-fold, and any figure quoted without a scope is not usable.
- **Counts at this scope** (85 directories scanned; `dotnet-platform-libs`
  included): UI-bearing repos **45**; repos defining `.visually-hidden`/
  `sr-only` **15**; dark-mode repos **20**; skip-link repos **17**; `z-index`
  declarations **202**. Responsive: **118** `@media` width queries across 43
  files in 32 repos with **24** distinct values (CSS scope); **139** queries /
  **31** values (wide scope). `640px` (11 repos) and `641px` (1 repo) both
  occur — the off-by-one pair is real. `768px` (10) and `769px` (1) both occur.
  `48rem` is the only rem-denominated value in the top 15 by repo spread.
  These are lower than the briefing figures (289 / 35) because this scope
  excludes vendor bundles; the discrepancy is a scope difference, not a
  correction, and both are stated rather than reconciled to one number.
- **Framework split at this scope:** **14** repos contain `.cshtml`/`.razor`
  (**13** excluding this repository, which has its own templates) and **8**
  contain `.dart`.
- **Guard count is stated two ways and the discrepancy is not resolved here.**
  `tests/Platform.Architecture.Tests/` contains **137** `[Fact]`/`[Theory]`
  methods with 11 `[InlineData]` rows and 4 `[MemberData]`/`[ClassData]`
  sources; the previously recorded figure of **355** is the *executed case*
  count including theory expansion. This change's oracle is
  "no guard is deleted or weakened and the suite still passes", which is
  invariant to which number is correct. The count is **not** re-measured by
  running the suite in this change.
- **Nothing is compiled, installed, packed, restored or executed in this
  change.** No `npm`, `dotnet build`, `dotnet test` or `quality-gate.sh` run
  occurred while writing it. Every statement about the packages' behaviour is
  read from `package.json`, source and committed `dist/` output. The
  `react-ui`/`react-shell` test suites are named as gates to keep green, not
  reported as green.
- **`.visually-hidden` divergence is confirmed by reading all four bodies**
  (table above). The 15-repo list is a selector grep, so it may under-count a
  copy that names the rule differently; the frozen list is therefore a
  **lower bound**, which makes it safe to close but not safe to treat as
  exhaustive.
- **`openaccounting` defect is confirmed by absence**: no
  `tailwind.config.*`, no `postcss.config.*`, no Tailwind directive in any
  stylesheet, 100 files using `sm:`/`md:`/`lg:`. Exact file count is
  `.html`/`.cshtml`-scoped and will differ under a wider scope.
