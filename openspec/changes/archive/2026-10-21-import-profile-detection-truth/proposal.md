# Change: import-profile-detection-truth

## Why

`forge import` inspects an existing repository and suggests the profile it
belongs to. The import lane keeps its profile knowledge in four
hand-written places, and two of the six supported profiles are missing
from them:

```rust
// src/import/detect.rs, before this change
fn profile_language(profile: &str) -> &str {
    match profile {
        "aspnet-web" => "csharp",
        "rust-web" => "rust",
        "nextjs-web" => "typescript",
        "flutter-app" => "dart",
        "python-service" => "python",
        _ => "unknown",
    }
}
```

`react-web` is a supported profile with a tested native scaffold, a
catalogue row and a generator arm. The import lane has never heard of it.
So a real React repository is invisible to the lane:

```console
$ forge import ./my-react-app
Detected:

Language: unknown
Framework: unknown
Package manager: unknown
...
Suggested profile:
unknown

Confidence: none
```

and accepting the import is refused outright:

```console
$ forge import ./my-react-app --accept
error[import-conflict]: import conflict: directory './my-react-app' has no
recognizable profile; re-run with --profile <id>; source and conflicting
metadata left unchanged
```

The operator is told to name a profile by hand for a profile Forge itself
sells. `requirement.md` §8 asks the import to detect the language, the
framework, the package manager and to suggest a profile; for a React
project the lane answers "unknown" to all four.

Handing the lane the profile id does not fix it. The lane then looks the
profile up in the same table that has no row for it, so it writes the
manifest with a language it does not know:

```console
$ forge import ./my-react-app --profile react-web --accept
imported my-react-app (./my-react-app)
$ cat forge.yaml
schema: 1
project:
  id: my-react-app
  name: my-react-app
  profile: react-web
  ...
runtime:
  language: unknown          # <- the profile's language, unknown
```

and the manifest then contradicts the profile the doctor reads:

```console
$ forge doctor ./my-react-app
[WARN] dependency-drift (automatic)
  evidence: manifest runtime language 'unknown' disagrees with profile language 'typescript'
```

So the gap is not one missing row. It is that the lane's profile
knowledge is scattered, incomplete and unpinned: a profile can be added
to the catalogue, generated and tested without the import lane ever
learning it.

The npm ecosystem is the second half of the gap. The lane proposes
`nextjs-web` for a `package.json` project only when the file's text
happens to name "next" or "nextjs":

```rust
if exists(dir, "package.json") {
    let text = scan_text(&dir.join("package.json"));
    if text.contains("\"next\"") || text.contains("nextjs") {
        ...
        out.push(Candidate { profile: "nextjs-web", score });
    }
}
```

A real Next.js repository lists the `next` dependency, so it is found. But
Forge's own Next.js scaffold is dependency-free by contract — its
`package.json` carries only the `build` and `test` scripts — so the
scaffold is found only when the project's own name happens to contain the
word:

```console
$ forge import ./webapp            # a generated nextjs-web project
Suggested profile:
unknown                            # the text names neither "next" nor "nextjs"

$ forge import ./sweep-nextjs-web  # the same project, named after the profile
Suggested profile:
nextjs-web
```

A detector whose answer depends on the project's file name is not a
detector.

## What Changes

- **The import lane knows `react-web`.** The profile's language is
  `typescript`, its package manager is `npm`, and its framework is
  detected as `react` from the project's `package.json`.
- **The npm ecosystem is detected by the markers each stack leaves.** A
  `package.json` project is `nextjs-web` when it names the `next`
  dependency or holds a `next.config.*` file, and `react-web` when it
  names the `react` dependency, holds a `vite.config.*` file or holds a
  root `index.html`. Next.js wins when both are present, because a
  Next.js application depends on React. A `package.json` project with
  neither marker still proposes no profile.
- **The lane's profile knowledge is one table.** The language and the
  package manager come from one declared list of profiles, so the lane's
  knowledge of a profile and its ability to propose that profile cannot
  drift apart.
- **The lane's profile list is pinned against the catalogue.** A
  repository contract check fails, naming the profile, when the lane and
  the supported catalogue disagree in either direction, and when a
  supported profile cannot be detected at all.
- **An imported project carries its profile's language.** The accepted
  manifest names the language the profile declares, so the doctor stops
  reporting a drift the import wrote itself.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The import detection | `forge` | Yes | `src/import/detect.rs`: the profile table, the language and package manager, the npm detector and the React framework arm. |
| The import proposal | `forge` | Read only | `src/import/model.rs` keeps its shape; no field is added. |
| The import display | `forge` | Read only | `src/import/display.rs` renders what the proposal carries. |
| The import contract test | `forge` | Yes | `tests/import_profile_detection_contract.rs`: the pinned table and the detection of every supported profile. |
| The existing import contract test | `forge` | Read only | `tests/import_contract.rs` keeps its assertions. |
| The profile catalogue | `forge` | Read only | `src/profile/catalog.rs` is the authority the lane's table is pinned against. |
| The generator | `forge` | Read only | `src/generate/engine.rs` already writes the markers the detectors read. |
| The doctor | `forge` | Read only | Change 17 made it honest; this change stops the import from writing the false drift. |
| The build lane, the test lane | `forge` | Read only | Their contracts do not change. |
| The registry and the adoption write | `forge` | Read only | The manifest is still written only on an explicit accept. |
| The `project-import` capability | `forge` | Modified | The detection requirement states that every supported profile is detected. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md:365` (§8 Project Import) asks the import to detect the language, the framework and the package manager and to suggest a profile; §9 and §14 make the profile the unit of the catalogue. |
| Contracts | The proposal's shape does not change: the same fields, the same `unknown` versus `missing` distinction, the same ambiguity rule. No field is added, renamed or defaulted. |
| Callers | `forge import` (CLI), the MCP import method and the API route render the same proposal. |
| Persistence | The manifest gains no field. `runtime.language` is filled with the profile's real language instead of `unknown`. |
| Integrations | None. No new toolchain, network or credential surface. The import stays read-only until an explicit accept. |
| Failures | No new error code. The existing `import-conflict` refusal for an unrecognisable directory stays; more directories become recognisable. |
| Tests | A new `tests/import_profile_detection_contract.rs` pins the table and the detection of every supported profile. |
| Dependencies | None added. |
| Concerns | Quality and compatibility: the lane's profile list is pinned against the catalogue in both directions. Security: the import stays read-only until an explicit accept, and the detectors read the same bounded files as before. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `import-profile-detection-truth` | forge | Rust | added | this package |
| `project-import` | forge | Rust | modified | the detection requirement states that every supported profile is detected |

## Non-Goals

- **The import lane does not become a framework analyser.** A profile
  whose markers name no known framework stays `unknown`: the generated
  Rust project's `Cargo.toml` names no framework, and that is honest.
- **A `package.json` project with neither marker stays undetected.** A
  plain npm library is not a `nextjs-web` or a `react-web` project, and
  the lane does not guess.
- **The other four profiles' detection is untouched.** The Rust,
  Flutter, Python and .NET detectors keep their markers and their scores.
- **The ambiguity rule is untouched.** Two detectors that tie still fail
  with `ambiguous-import` until an explicit `--profile` selects the
  outcome. Next.js winning over React is a precedence, not a tie.
- **The import lane does not read `forge.yaml`.** Detection stays
  independent of the manifest the lane itself may write.
- **The doctor is not changed.** Its build-definition judgement and its
  drift rule stay as change 17 left them.
- **The manifest gains no field.** The profile's language is written into
  the field the manifest already has.
