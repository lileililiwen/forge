# Design: UI shared-layer reachability, single token source, and conformance

Capability: `platform-ui-token-distribution`.
Consumed unchanged: `platform-ui-design-system`.
Repository: `dotnet-platform-libs`. This change writes **only** inside this
repository. `forge/` is read-only here.

---

## 1. Ownership and dependency direction

| Owner | Owns | Direction |
|---|---|---|
| `ui/tokens/tokens.json` | **every** design-token value, in DTCG form, light **and** dark | → generator |
| `ui/scripts/generate-tokens.mjs` | DTCG walk + alias resolution + primitive emission | reads tokens.json, writes `dist/` |
| `ui/scripts/verify-ui.mjs` | completeness and no-orphan-literal oracle over `dist/` | reads `dist/` |
| `ui/scripts/pack-ui.mjs` | deterministic tarballs into `ui/feed/` | reads `packages/*` |
| `ui/scripts/verify-ui-feed.mjs` | feed ⇄ consumer-declaration agreement | reads consumer `ui-conformance.json` |
| `scripts/check-ui-conformance.sh` | the fleet-wide negative scan | reads all repos |
| `tests/Platform.Architecture.Tests/UiDistributionTests.cs` | frozen-list and byte-stability guards | reads repo state |
| `forge/src/ui_pattern` | **behavior** layer only (states, a11y, navigation, form semantics) | consumes tokens it does not define (NG4) |
| consumer repo | the committed feed bytes + `ui-conformance.json` | resolves its own native toolchain |

Rules:

- Values flow **one way**: `tokens.json` → generator → `dist/` → tarball →
  consumer. No consumer and no generator may hold a value that is not
  reachable from a `$value` in `tokens.json`.
- The conformance check **depends on** the generator's output (it reads the
  emitted primitive names) and never on a consumer's own CSS.
- The check is **outward-reading only**. It reports on other repositories; it
  never writes to them.

---

## 2. Decision: reachability mechanism

### 2.1 Statement

How does a repository outside this one resolve
`@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell`?

### 2.2 Options

| # | Mechanism | Verdict |
|---|---|---|
| 1 | Publish to an npm registry | **Rejected.** No `@platform` scope, no credentials, no release pipeline, and a network dependency at install time — the opposite of what the precedent requires. Also NG6. |
| 2 | `file:../../../dotnet-platform-libs/ui/packages/design-tokens` | **Rejected.** Requires a sibling checkout. This is `..`-escaping, which `scaffold-prewires-shared-layer` explicitly refused. Fails on a fresh clone of one repo — the exact failure the precedent exists to prevent. |
| 3 | `.npmrc` feed path from an environment variable | **Rejected, and this is the precedent's own rejected first attempt.** The `forge` record is explicit: the first implementation used `NUGET_PLATFORM_FEED` and the owner **rejected** it because a variable a CI runner does not carry is the same failure class as a hard-coded absolute path. |
| 4 | Vendor the package **directory**, `file:vendor/platform-ui/design-tokens` | **Rejected.** npm satisfies a directory `file:` specifier with a **symlink**. The vendored directory is then ordinary editable source, so its bytes can drift with no manifest change and no digest to compare — the check would pass on a tree that no longer matches the kit. It also makes the consumer a second, silent editor of the shared layer. |
| 5 | **Commit the packed tarball; `file:` specifier over the tarball** | **Chosen.** |

### 2.3 Why option 5 is a different mechanism from the .NET one, and why that is correct

The instruction asks for the same *treatment* and permits a different
*mechanism* for npm with a stated reason. The treatment is identical; the
mechanism is not, and the difference is forced by the two toolchains:

- **NuGet** has a *source* concept: `NuGet.config` names a directory
  (`<add key="platform" value="./feed" />`) and NuGet scans it for `.nupkg`.
  The source is named, versioned and project-relative; the bytes are committed.
- **npm has no file-backed source concept.** npm's unit of resolution is a
  *specifier string* in `package.json`. There is no equivalent of a scanned
  local directory that carries a name and a version.

So the faithful translation of "a named, versioned, project-relative source
whose bytes are committed" into npm is: a `file:` specifier in `package.json`
(that's where npm's resolution lives) pointing at a committed **`.tgz`**
(that's where the versioned, checksummed bytes live), with a small
`ui-conformance.json` carrying the kit version and the tarball digest (that's
where the "named, versioned" part lives, since npm has nowhere else to put it).

A tarball is also the only one of the four local options that is
**byte-stable**: `npm pack` produces a deterministic artifact, so the same kit
version always yields the same digest, and a digest is something a check can
compare. That property is the whole point, so it is worth the one extra
indirection over option 4.

### 2.4 The contract

Producing side (this repository):

```
ui/feed/
  platform-design-tokens-0.1.0.tgz
  platform-react-ui-0.1.0.tgz
  platform-react-shell-0.1.0.tgz
  feed.json          # kit version, per-package name/version/sha256
```

Consuming side (a consumer repo, later — **not** in this change):

```jsonc
// package.json
"dependencies": {
  "@platform/design-tokens": "file:vendor/platform-ui/platform-design-tokens-0.1.0.tgz"
}
```

```jsonc
// ui-conformance.json  (repo root)
{
  "schema": 1,
  "state": "consumes-feed",          // or "exempt"
  "kitVersion": "0.1.0",
  "feed": {                          // path relative to this file's repo
    "path": "vendor/platform-ui",
    "packages": [
      { "name": "@platform/design-tokens", "version": "0.1.0",
        "sha256": "<64 hex>" }
    ]
  }
}
```

Refused at check time, with a named diagnostic each: an absolute
`file:` path, a `..`-escaping `file:` path, a `file:` value assembled from an
environment variable, a declared `consumes-feed` with no committed tarball, a
digest mismatch, a version mismatch, a missing needed package, an undeclared
extra package.

---

## 3. Decision: which token source is authoritative

### 3.1 Ruling

**`ui/tokens/tokens.json` is the single authoritative source for every design
token value.** `forge/src/ui_pattern` is the **behavior** layer and stays
value-free.

### 3.2 The boundary is proved, not asserted

The obvious way this ruling goes wrong is that "primitives vs patterns" is
hand-waved and both sources quietly keep values. That is testable, so it was
tested:

| Probe | Result | Meaning |
|---|---|---|
| hex color literals in `forge/src/ui_pattern/`, `src/component/`, `src/semantic/` | **0** | no competing palette |
| `px`/`rem`/`em` dimension literals in `ui_pattern/mod.rs` (2137 lines) | **0** | no competing scale |
| `@media` breakpoints in `ui_pattern/mod.rs` | **0** | no competing responsive rule |
| `.cshtml`/`.razor`/`.html` template literals in `forge/src/ui_pattern/` | not a value site | out of scope for a value source |

`forge` therefore does **not** compete for authority and needs no change to
comply. That is the proof the ruling rests on.

### 3.3 The boundary is *not* yet bound, and that is NG4

`ui_pattern` declares token **names** in its contracts, and those names do not
resolve here:

| | `forge` `standard_spacing()` | this repository |
|---|---|---|
| token name | `design-token-spacing-4` | `space.4` / `--space-4` |
| scale | `4,8,12,16,24,32` (px-shaped) | `0.25, 0.5, 0.75, 1, 1.5, 2` rem |

So the layer split is real and the **binding is missing**. Binding the
vocabularies is a `forge/` change and is out of this change's write boundary
(**NG4**, follow-up `ui-pattern-token-binding`). It is recorded here rather
than silently ignored, because a reader of the ruling above would otherwise
assume the two sides already agree.

### 3.4 "One source" is currently false in **this** repository — two defects to close first

The ruling is only meaningful once these are fixed, and neither is optional:

- **D-A: the generator hand-enumerates tokens.** `generate-tokens.mjs` reads
  specific paths (`source.color.slate['50'].$value`) inside a template
  literal instead of walking the DTCG tree. A token added to `tokens.json` is
  silently dropped, and `verify-ui.mjs:19-21` checks only 6 named tokens, so
  the drop is invisible. **Fix:** walk the tree, resolve `{alias}` references,
  and assert in the verifier that the emitted token-name set equals the
  `$value`-bearing set in `tokens.json`.
- **D-B: the dark theme is a second literal palette in the generator.**
  `[data-theme="dark"], .theme-dark` hard-codes 5 unique hexes
  (`#93c5fd`, `#bfdbfe`, `#fca5a5`, `#86efac`, `#fcd34d`); **0 of 5** appear
  in `tokens.json`. **Fix:** move them into `tokens.json` as dark-capable
  primitives, and have the verifier fail on any literal in the generated CSS
  that is not reachable from a `$value` or a documented `var()` alias.

D-A and D-B are the reason this change is not only a feed. Shipping a
distributable that can drop a token and a dark theme that cannot be themed
would scale the existing defect to every consumer.

---

## 4. Decision: how conformance is expressed

### 4.1 The problem, stated precisely

The abstraction is missing, so 15 repos wrote `.visually-hidden` themselves and
nothing failed. A check of the form "does the repo use the shared thing?"
cannot work: most repos that ship UI do not use it, and a repo shipping no UI
at all must not be forced to. **Conformance is therefore expressed as the
absence of a second definition, not the presence of the first.**

### 4.2 The three states

| State | Meaning | Check result |
|---|---|---|
| `consumes-feed` | resolves the committed feed; digests agree | pass |
| `exempt` | ships UI, declares a non-consuming reason + owner + review-by | pass, and **visible** |
| *(no declaration)* + UI detected | silent divergence | **fail** |
| *(no declaration)* + no UI detected | not a UI repo | pass vacuously |
| `legacy-divergent` | in the **frozen** list (§4.4) | pass, **cannot grow** |

### 4.3 The canonical primitive

`tokens.css` gains `.visually-hidden` in the full 9-property form
(`position:absolute; width:1px; height:1px; padding:0; margin:-1px;
overflow:hidden; clip:rect(0 0 0 0); white-space:nowrap; border:0`) with **no**
`!important`. It is emitted by the generator, so it cannot drift from the kit,
and `verify-ui.mjs` asserts it.

`arivio` is the justification for shipping it rather than documenting it: its
copy omits `padding`/`margin`/`border` and leaks a 1×1px box, which is a
defect, not a preference. `pangoman`'s every-declaration-`!important` copy and
`openpanel`'s space-separated `clip` are style disagreements. One canonical
rule settles all three.

### 4.4 The frozen legacy list — how a **new** divergent copy fails

This is the crux. A blanket ban fails today and teaches nothing; a blanket
pass-through changes nothing. The mechanism is a **closed** list:

1. At implementation time, `scripts/check-ui-conformance.sh` **generates** the
   baseline by scanning for a shared-primitive declaration outside any vendored
   feed directory. This session's measurement gives **15** repos.
2. That list is committed as a frozen constant, and a guard in
   `UiDistributionTests.cs` asserts the count has **not grown** and that no
   listed repo has been removed without its state being updated.
3. Any repo *not* on the list that declares `.visually-hidden` or `.sr-only`
   itself → **fail**, named `ui-divergent-primitive`.
4. The list can only shrink. A legacy repo that migrates leaves the list; the
   list is never extended, and there is no suppression flag that adds to it.

Net effect: the 15 known copies are visible and blocking nothing today, and
the sixteenth — the one this change is actually for — cannot be written
without failing the check.

### 4.5 What a repo must do to be compliant

1. Ship the committed feed bytes at the path its `ui-conformance.json` names,
   with digests matching, **or** declare `exempt` with a reason, an owner and
   a review-by date.
2. Declare **no** shared primitive (`.visually-hidden`, `.sr-only`) outside the
   feed directory.
3. Introduce **no** hand-authored breakpoint outside the documented kit scale
   without a declared reason. The 118 existing non-scale queries are baselined
   the same frozen way, so this fails *new* divergence without demanding the
   289-repository rewrite this change explicitly does not authorise (NG8).
4. Pass the token-completeness oracle for anything it vendors.

### 4.6 Why the check is a script and not a .NET package

A `Platform.UI.Conformance` package would let a consumer assert compliance at
build time — which sounds right and is wrong here, on this repository's own
evidence:

- The check's subject is **other repositories' text files**. A referenced
  assembly cannot observe them; the consumer would have to hand its own file
  paths to the package, which makes the package a wrapper around a file walk.
- The workspace rule is two confirmed consumers before a capability enters the
  shared library. The three existing UI packages have **0** consumers and
  `Platform.UI.Razor` has **0** (per `forge`'s own inventory).
- Cost: a 76th `eng/package-manifest.json` entry, a `PackageReference` to
  version centrally, a `docs/packages.md` row, and public-API surface — in a
  repository whose recorded concern is over-factoring.

**Decision: no new .NET package.** The check is `scripts/check-ui-conformance.sh`
plus new test methods in the existing `tests/Platform.Architecture.Tests`
project. Package count stays **75**; the manifest stays byte-identical.

---

## 5. Contracts

| Contract | Producer | Consumer | Oracle |
|---|---|---|---|
| Token source | `ui/tokens/tokens.json` (DTCG) | `generate-tokens.mjs` | every `$value` appears in `tokens.css`; no orphan literal in `tokens.css` |
| Generated artifacts | `generate-tokens.mjs` | react-ui, react-shell, `Platform.UI.Razor`, consumers | `verify-ui.mjs` green; emitted token-name set unchanged from 0.1.0 |
| Canonical primitive | `generate-tokens.mjs` | every consumer | `.visually-hidden` present, 9 properties, no `!important` |
| Feed bytes | `pack-ui.mjs` → `ui/feed/*.tgz` + `feed.json` | a consumer's committed `vendor/platform-ui/` | sha256 of each tarball == `feed.json` == consumer's `ui-conformance.json` |
| Compliance declaration | consumer `ui-conformance.json` | `check-ui-conformance.sh` | exactly one of `consumes-feed` / `exempt`; no shared-primitive declaration outside the frozen list |
| Frozen baseline | `check-ui-conformance.sh` constants + `UiDistributionTests.cs` | the check itself | legacy list length unchanged and non-extensible |

Additive-compatibility invariant: the generator rewrite **must not rename any
emitted token**. `verify-ui.mjs`'s 6-token assertion and the existing
`react-ui`/`react-shell` tests are the regression net for that.

---

## 6. Failure boundaries

| Condition | Named outcome | Behaviour |
|---|---|---|
| Token added to `tokens.json`, not emitted | `ui-token-dropped` | `verify-ui.mjs` fails, names the token path |
| Literal in `tokens.css` unreachable from a `$value` | `ui-orphan-literal` | fails, names the declaration and line |
| `tokens.json` has an unresolvable `{alias}` | `ui-alias-unresolved` | fails, names the alias and the referencing path |
| Consumer `file:` specifier is absolute / `..`-escaping / env-var | `ui-feed-path-invalid` | fails; refused before any install |
| `consumes-feed` declared, tarball absent | `ui-feed-incomplete` | fails, names the missing package |
| Tarball digest ≠ `feed.json` ≠ declaration | `ui-feed-digest-mismatch` | fails, names package and both digests |
| Kit version in `feed.json` ≠ declared `kitVersion` | `ui-feed-version-mismatch` | fails, names both versions |
| Undeclared extra package in the feed dir | `ui-feed-undeclared` | fails, names the extra tarball |
| UI-bearing repo, no `ui-conformance.json` | `ui-conformance-undeclared` | fails, names the detected UI signal |
| Shared primitive declared outside the frozen list | `ui-divergent-primitive` | fails, names repo, file, line, and the canonical rule to use instead |
| Legacy list grew | `ui-legacy-list-grew` | guard fails; the list is closed by construction |
| Non-scale breakpoint introduced without a reason | `ui-breakpoint-undeclared` | fails, names the value and the file |
| `forge` catalog gains a literal value | *(not checked here)* | NG4 follow-up owns it |

None of these produce a half-applied state: the check is read-only and reports;
it never rewrites a consumer.

---

## 7. Migration

Nothing in this change migrates anything. Stated as a boundary, not a plan:

- **This change:** makes the layer resolvable and checkable, and freezes an
  honest baseline. Consumer trees are untouched.
- **Follow-up `ui-fleet-adoption`:** one OpenSpec change per consuming
  repository, sequenced under this repository's one-active-change rule
  (`AGENTS.md` workflow), each doing the same three moves: commit the feed
  bytes, add `ui-conformance.json`, delete the local primitive. Sequenced
  `arivio` first (measured defect, not preference), then the 4-property
  variants, then the remainder.
- **Follow-up `ui-pattern-token-binding` (in `forge`):** map
  `design-token-spacing-4` → `space.4` and the scale vocabulary onto the
  authoritative namespace, then make the value-free property checkable from
  both sides.

An already-`file:`-consuming consumer is unaffected: this change adds no
`npm publish`, so nothing it has today is withdrawn.

---

## 8. Compatibility

- **.NET:** no public type added, changed or removed. No new `.csproj`, no new
  `PackageReference`, no new `ProjectReference`. `eng/package-manifest.json`
  is **byte-identical** and `generate-package-manifest.sh --check` stays green
  because the manifest's inputs are unchanged. `Platform.UI.Razor` is
  untouched at `0.1.0`. `eng/public-api-baseline.txt` is unchanged;
  `scripts/check-public-api.sh` scans 6 packages and none of them changes.
  `Platform.UI.Razor` is **not** in that scan list — a pre-existing coverage
  gap, recorded here and **not** closed by this change (closing it would
  regenerate the baseline for a package this change does not touch).
- **Architecture guards:** additive test methods in an existing project. No
  guard deleted, renamed, or weakened. The recorded guard count is stated two
  ways in the proposal (137 methods / 355 executed cases) and this change does
  not re-measure it by running the suite.
- **npm:** strictly additive within `0.1.0`. `tokens.css` gains declarations
  and the canonical primitive; it renames nothing, so a hypothetical 0.1.0
  consumer keeps resolving. Package versions are unchanged because their
  public surface is unchanged; consumers pin the tarball **and** its digest, so
  a version that is not bumped but whose bytes changed is caught by
  `ui-feed-digest-mismatch` rather than silently absorbed. That is the reason
  the digest, not the version, is the load-bearing check.
- **Existing `ui` tests:** `react-ui/tests/components.test.mjs` and the
  `react-shell` tests must stay green. They are named as gates, not reported
  as passing — no test was executed while writing this change.
- **OpenSpec:** new capability only. `platform-ui-design-system` is consumed
  unchanged; a later change may promote its requirements to `## MODIFIED`
  blocks once distribution has landed.

---

## 9. Verification oracle

One oracle per contract. Each is a command or a named assertion — none is
"sounds right".

| # | Oracle | How it is checked |
|---|---|---|
| V1 | Token completeness | `node ui/scripts/verify-ui.mjs` fails when the emitted `--token-name` set ≠ the `$value`-bearing set in `tokens.json` |
| V2 | No orphan literal | same run fails on any literal in `tokens.css` unreachable from a `$value` or a documented `var()` alias — this is what closes D-B |
| V3 | Alias resolution | same run fails on an unresolvable `{alias}` |
| V4 | Canonical primitive | same run asserts `.visually-hidden` exists, has all 9 properties, and contains no `!important` |
| V5 | Accessibility markers preserved | the existing `verify-ui.mjs:22-30` assertions (dark theme, reduced-motion, `aria-busy`/`aria-invalid`/`aria-live`, `:focus-visible`, dialog role) stay green — the generator rewrite is a regression, not a licence |
| V6 | No token renamed | emitted token-name set is byte-equal to the pre-change `tokens.css` name set |
| V7 | Feed reproducible | `pack-ui.mjs` run twice yields byte-identical tarballs; sha256 stable across runs |
| V8 | Feed ⇄ declaration | `verify-ui-feed.mjs` passes on agreement; fails on digest mismatch, version mismatch, missing package, undeclared package |
| V9 | Feed path refusals | absolute, `..`-escaping and env-var `file:` values each produce their named diagnostic |
| V10 | Portability | a consumer tree is copied to an unrelated path and resolves with **no** sibling checkout, **no** environment variable, and **no** network |
| V11 | Conformance scan | `check-ui-conformance.sh` exits 0 on the current workspace; a fixture repo declaring `.visually-hidden` itself exits non-zero with `ui-divergent-primitive` |
| V12 | Legacy list frozen | `UiDistributionTests.cs` fails if the legacy count is not exactly the recorded 15, and there is no code path that adds an entry |
| V13 | New-breakpoint failure | a fixture with an undeclared `min-width: 823px` fails with `ui-breakpoint-undeclared`; a fixture with only baselined values passes |
| V14 | Vacuous pass | a repo with no UI signal and no declaration passes without a `ui-conformance.json` |
| V15 | .NET manifest untouched | `scripts/generate-package-manifest.sh --check` reports `Manifest matches source.`; `scripts/package-inventory.sh` still enumerates **75** |
| V16 | Public API untouched | `scripts/check-public-api.sh` produces a clean diff against the unchanged `eng/public-api-baseline.txt` |
| V17 | Guards green | `dotnet test tests/Platform.Architecture.Tests` passes; no existing guard removed or weakened |
| V18 | Repo gate | `scripts/quality-gate.sh` passes end to end, and `openspec validate ui-shared-layer-reachability --strict` passes |

**Evidence discipline for the implementing session:** V5, V6, V7, V15, V16 and
V17 require running `node`, `npm`/`dotnet`, which this proposal did **not** do.
V1–V4 and V11–V14 are implementable without a network and without a .NET
build. Any oracle blocked by a missing toolchain SHALL be reported as
`unverified` with its exact command, never as passing.

---

## 10. Decision ledger

| # | Decision | Options | Choice | Rationale | Status |
|---|---|---|---|---|---|
| D1 | npm reachability mechanism | registry publish · sibling `file:` path · env-var `.npmrc` · vendored dir `file:` · **committed tarball `file:`** | **committed tarball** | The only local option that is byte-stable; dir `file:` becomes a symlink that drifts silently. Reuses the settled precedent's three refusals. | **Decided** |
| D2 | Why not byte-identical to the .NET feed | — | npm has no file-backed *source*; the equivalent is a specifier + committed bytes + a declared version/digest | Forced by the toolchain, not preference. | **Decided** |
| D3 | Authoritative token source | `dotnet-platform-libs/ui` · `forge/ui_pattern` · keep both | **`ui/tokens/tokens.json`** | Proved: `forge` holds 0 hex and 0 dimension literals across its 2137-line `ui_pattern/mod.rs`, so it is not a value competitor. | **Decided** |
| D4 | forge-side binding | fix here · NG4 | **NG4** | `forge/` is outside this change's write boundary; hard rule forbids touching it. | **Decided** |
| D5 | Conformance polarity | require adoption · require no divergence · warn | **require no divergence, with a frozen closed list** | "Require adoption" fails 30+ repos today and proves nothing; warn-only changes nothing. A closed list makes the *next* copy fail. | **Decided** |
| D6 | Legacy baseline size | blanket ban · no baseline · **frozen list of 15** | **frozen list** | Blanket ban is unreviewable at this size; no baseline makes copy #16 free. | **Decided** |
| D7 | Conformance implementation | new `Platform.UI.Conformance` package · shell script + existing test project | **script + existing project** | The subject is other repos' text files; a referenced assembly cannot see them. 0 consumers of every existing UI package; manifest stays byte-identical at 75. | **Decided** |
| D8 | Ship `.visually-hidden` or document it | document · **ship canonical** | **ship** | `arivio` is a measured defect (1px leak). A pattern with no canonical form has 15 copies. | **Decided** |
| D9 | Retrofit existing repos | now · **NG2** | **NG2** | Prohibited by this change's scope; `arivio` first in `ui-fleet-adoption`. | **Decided** |
| D10 | Flutter coverage | include · **NG5** | **NG5** | No Dart artifact exists; `forge` already declares `flutter-app: tokens: none`. Claiming coverage would be false. | **Decided** |
| D11 | Razor coverage | exclude · **include (token + check only)** | **include** | `Platform.UI.Razor` already ships at `0.1.0`; 5 of 15 divergent copies are Razor repos; `platform-tokens.css` needs no new vehicle. | **Decided** |
| D12 | Responsive unification | now · **NG8** | **NG8** | 118 existing non-scale queries; collapsing them is fleet adoption, not distribution. New divergence still fails. | **Decided** |
| D13 | Generator rewrite timing | after feed · **before feed** | **before** | Distributing an artifact that can drop tokens scales the defect. V6 protects the rename invariant. | **Decided** |
| D14 | Close the `check-public-api.sh` gap for `Platform.UI.Razor` | now · record only | **record only** | Would regenerate the baseline for a package this change does not touch; scope creep with manifest/baseline churn. | **Open for owner** |
| D15 | Kit version bump policy on `0.1.0` | bump to `0.2.0` · keep `0.1.0` + digest | **keep `0.1.0`, digest is load-bearing** | Surface is unchanged (additive); a bump would imply a contract change that did not happen. Consumers pin tarball **and** digest, so byte drift is caught regardless. | **Open for owner** |
| D16 | Should the frozen list live in a script constant or a checked-in data file | constant · data file | **data file** (`ui/legacy-divergent.json`) | A data file is reviewable as a diff and cannot be edited by a code path; a constant invites in-code edits. V12 asserts its length. | **Open for owner** |

### Open decisions left to the owner

1. **D14** — extend `scripts/check-public-api.sh` to cover `Platform.UI.Razor`
   (and regenerate `eng/public-api-baseline.txt`), or leave the pre-existing
   6-package coverage gap closed-over. Recommended: leave it; it is unrelated
   to this change and regenerating the baseline adds review noise.
2. **D15** — confirm the three UI packages stay at `0.1.0` with digest-pinning,
   or take a `0.2.0` for the added `.visually-hidden` primitive and dark-theme
   tokens. Recommended: stay at `0.1.0`; the change is additive and the digest
   already catches drift.
3. **D16** — confirm the frozen legacy list ships as
   `ui/legacy-divergent.json` (data file) rather than a script constant.
4. **Sequencing** — confirm `ui-fleet-adoption` and `ui-pattern-token-binding`
   are the accepted follow-up names, and which of `arivio` (the measured
   defect) goes first.
5. **Whether `verify-ui.mjs` should also be wired into
   `scripts/quality-gate.sh`.** It is not today (the gate is `dotnet` +
   `openspec` + `git diff --check`), so a token regression does not block the
   repo gate. Recommended: wire it in, but that is a gate-semantics change and
   is left to the owner.
