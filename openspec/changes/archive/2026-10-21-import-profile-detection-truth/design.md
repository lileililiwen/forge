## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The import detection | `forge` | Yes. `src/import/detect.rs`. |
| The import proposal and its display | `forge` | Read only. |
| The import contract test | `forge` | Yes. `tests/import_profile_detection_contract.rs`. |
| The existing import contract test | `forge` | Read only. |
| The profile catalogue | `forge` | Read only. It is the authority the lane's table is pinned against. |
| The generator | `forge` | Read only. It already writes the markers the detectors read. |
| The doctor | `forge` | Read only. |
| The manifest, the registry | `forge` | Read only. |

## The Gap

**Four tables, two profiles missing.**

```rust
// src/import/detect.rs, before this change
fn profile_language(profile: &str) -> &str {        // five profiles
fn profile_package_manager(profile: &str) -> &str { // five profiles
fn detect_framework(dir: &Path, profile: &str) {    // five profiles
fn top_level_candidates(dir: &Path) -> Vec<Candidate> // five detectors
```

`react-web` is in none of them. `nextjs-web` is in three of them, and its
detector is a text coincidence.

**A supported profile is invisible to the import.**

```console
$ forge import ./my-react-app
Suggested profile:
unknown
Confidence: none

$ forge import ./my-react-app --accept
error[import-conflict]: import conflict: directory './my-react-app' has no
recognizable profile; re-run with --profile <id>; source and conflicting
metadata left unchanged
```

**And naming the profile by hand writes a lie into the manifest.**

```console
$ forge import ./my-react-app --profile react-web --accept
imported my-react-app (./my-react-app)
$ grep -A1 runtime forge.yaml
runtime:
  language: unknown

$ forge doctor ./my-react-app
[WARN] dependency-drift (automatic)
  evidence: manifest runtime language 'unknown' disagrees with profile language 'typescript'
```

The import lane wrote the drift the doctor then reported. The two lanes
disagree about the same project because they read two different tables,
and only one of them is pinned.

**The npm detector answers from the project's name.**

```rust
if exists(dir, "package.json") {
    let text = scan_text(&dir.join("package.json"));
    if text.contains("\"next\"") || text.contains("nextjs") {
        ...
        out.push(Candidate { profile: "nextjs-web", score });
    }
}
```

Forge's own Next.js scaffold is dependency-free by contract: its
`package.json` carries the `build` and `test` scripts and nothing else.
So the scaffold is detected only when the project's file name carries the
word:

```console
$ forge import ./webapp             # generated nextjs-web project
Suggested profile:
unknown
$ forge import ./sweep-nextjs-web   # the same project, named after the profile
Suggested profile:
nextjs-web
```

A detector whose answer depends on the file name is not a detector.

**Nothing keeps the lane and the catalogue in step.** The catalogue lists
the supported profiles in `src/profile/catalog.rs`; the lane lists them in
`src/import/detect.rs`. Both are hand-written and only one is checked —
and the lane's copy was already two profiles short.

## The Decision

### The lane's profile knowledge is one table

The language and the package manager come from one declared list, so the
lane cannot know a profile's language in one place and not know it in
another:

```rust
/// `(profile, language, package manager)` for every profile the import
/// lane knows.
const IMPORT_PROFILES: [(&str, &str, &str); 6] = [
    ("aspnet-web", "csharp", "dotnet"),
    ("flutter-app", "dart", "pub"),
    ("nextjs-web", "typescript", "npm"),
    ("python-service", "python", "pip"),
    ("react-web", "typescript", "npm"),
    ("rust-web", "rust", "cargo"),
];
```

`profile_language` and `profile_package_manager` read it, and
`import_profiles()` returns the ids. A profile that is not in the table
is `unknown`, exactly as before — the difference is that the table is
now the single place a profile enters the lane.

### The npm ecosystem is detected by the markers each stack leaves

A `package.json` project is one of two stacks, and each leaves its own
markers:

| Stack | Markers |
|---|---|
| Next.js | the `next` dependency in `package.json`, or a `next.config.js\|mjs\|cjs\|ts` file |
| React | the `react` or `react-dom` dependency in `package.json`, or a `vite.config.js\|mjs\|ts` file, or a root `index.html` |

```rust
if exists(dir, "package.json") {
    let text = scan_text(&dir.join("package.json"));
    let nextjs = text.contains("\"next\"")
        || text.contains("nextjs")
        || has_nextjs_config(dir);
    let react = text.contains("\"react\"")
        || text.contains("\"react-dom\"")
        || has_vite_config(dir)
        || exists(dir, "index.html");
    if nextjs {
        out.push(Candidate { profile: "nextjs-web", score });
    } else if react {
        out.push(Candidate { profile: "react-web", score });
    }
}
```

**Next.js wins when both are present.** A Next.js application depends on
React, so a project with both markers is a Next.js project. That is a
precedence, not a tie: the ambiguity rule — two detectors with equal
evidence and no explicit selection — is untouched and still fails with
`ambiguous-import`.

**A `package.json` with neither marker proposes no profile.** A plain npm
library is not a `nextjs-web` and not a `react-web` project, and the lane
does not guess. That is the same honest answer the lane gives a
directory it does not recognise.

The scoring keeps the shape of the other detectors: a base score of 3, a
bonus for a lockfile (`package-lock.json`, `pnpm-lock.yaml`,
`yarn.lock`), and a bonus for the stack's own source root (`app/` or
`pages/` for Next.js, `src/` for React).

### The React framework arm

`detect_framework` learns React the way it learns Next.js — from the
dependency the project's own manifest declares:

```rust
"react-web" => {
    let text = scan_text(&dir.join("package.json"));
    if text.contains("\"react\"") || text.contains("\"react-dom\"") {
        Detection::detected("react", vec!["package.json"])
    } else {
        unknown()
    }
}
```

A React project that names no React dependency has no framework evidence,
and the lane says `unknown` rather than guessing. The Rust and Python
arms keep their behaviour: the generated Rust project's `Cargo.toml` names
no framework, and that stays `unknown`.

### The lane is pinned against the catalogue, in both directions

A table can be complete and still drift from the catalogue. The contract
check asserts three things:

| Check | Fails when |
|---|---|
| The lane's profile list is the catalogue's supported list | A profile is added to the catalogue and the lane does not learn it, or the lane keeps a profile the catalogue dropped. |
| Every supported profile is detectable | A profile the lane claims to know cannot be proposed for a project shaped like it. |
| A detected profile carries a known language and package manager | The lane proposes a profile and answers "unknown" for its language, as it did for `react-web`. |

The second check is end to end: for each supported profile, a project
shaped like that profile is inspected through the built binary and the
proposal must name that profile, with no other profile in its
alternatives.

## What Is Not Touched, And Why

| Surface | Why it is not in this package |
|---|---|
| The proposal model (`src/import/model.rs`) | No field is added. The proposal already carries a language, a framework and a package manager, each with its `unknown`/`missing`/`detected` state. |
| The ambiguity rule | Two detectors that tie still fail with `ambiguous-import` until an explicit `--profile` selects the outcome. Next.js over React is a precedence. |
| The adoption write (`adopt_import`) | The manifest is still written only on an explicit accept, only after the identity is pre-checked. |
| The registry | Untouched. |
| The Rust, Flutter, Python and .NET detectors | Their markers and their scores are already right. |
| The doctor | Its build-definition judgement and its drift rule are change 17's business. This change stops the import from writing the false drift. |
| The generator | It already writes the markers the detectors read: `next.config.mjs` and `app/` for Next.js, `vite.config.js` and `index.html` for React. |
| The manifest schema | No field is added. `runtime.language` is filled with the profile's real language instead of `unknown`. |
| The profile catalogue | It is the authority the lane is pinned against; it is right. |

## User Experience and Interface

Before, a React repository was invisible:

```console
$ forge import ./my-react-app
Language: unknown
Framework: unknown
Package manager: unknown
...
Suggested profile:
unknown
Confidence: none
```

After, the lane sees it:

```console
$ forge import ./my-react-app
Language: typescript
Framework: react
Package manager: npm
...
Suggested profile:
react-web
Suggested maturity:
L1
Confidence: high
```

The accepted manifest carries the profile's language, so the doctor stops
reporting a drift the import wrote:

```console
$ forge import ./my-react-app --accept
imported my-react-app (./my-react-app)
$ forge doctor ./my-react-app
[PASS] dependency-drift (automatic)
  evidence: manifest runtime agrees with on-disk dependency evidence
```

And a generated Next.js project is detected by its own markers, whatever
its name:

```console
$ forge import ./webapp
Suggested profile:
nextjs-web
Confidence: high
```

## Behavioral Model

| Situation | Outcome |
|---|---|
| A React repository with the `react` dependency | The proposal names `react-web`, `typescript`, `npm` and the framework `react`. |
| A Next.js repository with the `next` dependency | The proposal names `nextjs-web`, `typescript`, `npm` and the framework `next.js`. |
| A repository with both markers | `nextjs-web`: a Next.js application depends on React. No ambiguity is raised. |
| A generated Next.js project (no dependency, a `next.config.*` file) | The proposal names `nextjs-web`. |
| A generated React project (the `react` dependency, a `vite.config.*` file, a root `index.html`) | The proposal names `react-web`. |
| A `package.json` project with neither marker | No profile is proposed; the language, framework and package manager stay `unknown`. |
| A React repository with no `react` dependency | The profile is `react-web` when a `vite.config.*` file or a root `index.html` says so; the framework stays `unknown` when no React dependency is named. |
| A monorepo whose subdirectories disagree | Unchanged: `ambiguous-import` until an explicit `--profile` selects the outcome. |
| A Rust, Flutter, Python or .NET repository | Unchanged: the same profile, language, package manager and framework as before. |
| A directory with no recognisable markers | Unchanged: `unknown` everywhere, and an explicit accept refuses with `import-conflict`. |
| An accepted import | The manifest carries the profile's language, and the doctor reports no drift on it. |

## Failure and Boundary Policy

- **The lane never guesses.** A project with no markers stays
  unrecognised, and the refusal is the existing `import-conflict`.
- **The framework stays `unknown` when no framework is named.** Only the
  profile, the language and the package manager are guaranteed for every
  supported profile.
- **Next.js over React is a precedence, documented and pinned.** A project
  with both markers is a Next.js project.
- **The detection is read-only.** Nothing in this package writes to the
  inspected repository; the manifest is still written only by an explicit
  accept.
- **The detectors read the same bounded files as before.** The bounded
  scan limit and the subdirectory limit are unchanged.
- **The pinned table is the friction.** Adding a profile to the catalogue
  without teaching the lane fails the contract check, naming the
  profile. That is the intended friction.

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

Three real runs:

1. `forge import` on a React project names `react-web`, `typescript`,
   `npm` and the framework `react`.
2. `forge import --accept` on that project writes `runtime.language:
   typescript`, and `forge doctor` on it reports no dependency drift.
3. `forge import` on a generated Next.js project names `nextjs-web`
   whatever the project is called.

## Implementation Plan

**The table.** `src/import/detect.rs` gains `IMPORT_PROFILES`, and
`profile_language`, `profile_package_manager` and a new
`import_profiles()` read it.

**The npm detector.** `src/import/detect.rs`: the `package.json` branch
proposes `nextjs-web` or `react-web` from the markers each stack leaves.

**The framework.** `src/import/detect.rs`: `detect_framework` learns
`react-web`.

**The tests.** `tests/import_profile_detection_contract.rs` pins the
lane's profile list against the catalogue and inspects a project shaped
like every supported profile through the built binary.

## Decision Ledger

- **Resolved:** the lane's language and package-manager knowledge is one
  table, and the profile list is derived from it.
- **Resolved:** the npm ecosystem is detected by the `next` dependency or a
  `next.config.*` file, and by the `react` dependency, a `vite.config.*`
  file or a root `index.html`.
- **Resolved:** Next.js wins when both markers are present. A Next.js
  application depends on React, so both markers are one stack, not two.
- **Resolved:** a `package.json` project with neither marker proposes no
  profile. The lane does not guess.
- **Resolved:** the React framework is detected from the `react`
  dependency; without it the framework stays `unknown`.
- **Resolved:** the guard pins the lane's profile list against the
  supported catalogue in both directions and checks that every supported
  profile is detectable.
- **Resolved:** the other four profiles' detectors, the ambiguity rule,
  the adoption write and the manifest schema are untouched.
- **Implementation authorized (2026-10-22):** the owner directed the
  trunk repair programme and authorized this package in the approved
  order, after `doctor-build-definition-truth` closed the doctor's
  build-definition gap.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| The import lane knows every supported profile | A React repository is detected and imported | The proposal names the profile, language and package manager | One table; the npm markers | `src/import/detect.rs` | A React project is detected; a plain npm project is not | 3.1, 3.2, 3.3 | The contract test; the real runs |
| The npm ecosystem is detected by its markers | A generated Next.js project is detected whatever its name | The proposal names `nextjs-web` | The marker rules; Next.js wins | `src/import/detect.rs` | Both markers → Next.js; neither → no profile | 3.2 | The contract test |
| The lane's profile list is pinned against the catalogue | A profile cannot be added without the lane learning it | The check fails and names the profile | The pinned table and the end-to-end detection | `tests/import_profile_detection_contract.rs` | The lists agree; every profile is detectable | 3.4 | The contract test |
| The detection requirement states the rule | The contract names the rule | The report is consistent | `project-import` gains a scenario | `src/import/detect.rs` | Every supported profile is detected with its language | 3.1 | The contract test |
