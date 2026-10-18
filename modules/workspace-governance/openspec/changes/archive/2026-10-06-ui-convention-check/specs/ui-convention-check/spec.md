## ADDED Requirements

### Requirement: Enumerate the UI surface from a pruned census

The checker MUST treat a regular file as UI surface when its suffix is one of
`.tsx`, `.jsx`, `.vue`, `.svelte`, `.css`, `.scss`, `.razor`, `.cshtml`,
`.html`, `.dart`, and MUST prune `target/`, `node_modules/`, `obj/`, `bin/`,
`dist/`, `build/`, `.git/`, `__pycache__/`, `coverage/` and `.next/` at any
depth. It MUST NOT follow symlinked directories and MUST NOT read a symlinked
file. A repository with no UI file MUST NOT be reported against.

#### Scenario: Pruned build output

- **WHEN** a repository holds `app.tsx` and `node_modules/pkg/index.js`
- **THEN** the census contains one UI file and the vendored tree is pruned

#### Scenario: Symlinked UI file

- **WHEN** a UI-suffix path inside the project is a symlink
- **THEN** it is not read, and if it could hide a finding the run is
  `REVIEW_REQUIRED` with the path named

### Requirement: Read a declared scope, and never infer one

The checker MUST read `ui.scope.roots` and `ui.scope.exclude` from the
project's own `.project.json`. It MUST reject an absolute root, a root
containing `..`, and a root that resolves outside the project as a
configuration error, and MUST NOT infer a scope from directory names. A
repository that ships UI and declares no `ui.scope.roots` MUST be reported
`UI_SCOPE_UNDECLARED` at `REVIEW_REQUIRED`.

#### Scenario: Declared scope

- **WHEN** a repository ships UI and declares `ui.scope.roots`
- **THEN** content-level rules read only the files under those roots

#### Scenario: No declared scope

- **WHEN** a repository ships UI and declares no `ui.scope.roots`
- **THEN** the result carries `UI_SCOPE_UNDECLARED` as `REVIEW_REQUIRED` and no
  content-level rule is evaluated

#### Scenario: Escaping scope root

- **WHEN** `ui.scope.roots` contains an absolute path or a `..` segment
- **THEN** the run exits 2 with a configuration error naming the entry

### Requirement: Bound large surfaces by a recorded ceiling

The checker MUST compare the repo-wide census count against
`ui.scope.max_files`, defaulting to 400, and MUST report the exact count and the
ceiling for any repository over it. It MUST run filename-level rules on the
whole census regardless of the ceiling, and MUST NOT let the ceiling suppress,
narrow or hide a finding. A repository over the ceiling with no declared scope
MUST still be reported `UI_SCOPE_UNDECLARED`.

#### Scenario: Repository over the ceiling

- **WHEN** a repository's census exceeds `ui.scope.max_files` and it declares no
  scope
- **THEN** the report names the measured count and the ceiling, records
  `UI_SCOPE_UNDECLARED`, and still evaluates the auth filename rule over the
  whole census

#### Scenario: Tooling platform

- **WHEN** `alltools-platform` is checked with 2067 UI files and no declared
  scope
- **THEN** the report is bounded, names 2067 against the 400 ceiling, and does
  not emit a colour-literal finding for every file

### Requirement: Reject a hand-rolled auth screen

The checker MUST report `UI_AUTH_HANDROLLED` for a UI file whose file name
matches, case-insensitively, `login`, `logout`, `register`, `signup`,
`sign-in`, `signin`, `forgot.?password`, `reset.?password` or `auth`, when that
file is not inside a declared `ui.shared_layer.consume_paths` entry. The match
MUST be against the file name only. A declared adoption of the shared layer
MUST NOT suppress this finding.

#### Scenario: Hand-rolled login with no adoption

- **WHEN** a repository ships `pages/LoginPage.tsx` and declares no shared-layer
  adoption
- **THEN** the checker reports `UI_AUTH_HANDROLLED` as `ERROR`

#### Scenario: Adoption plus a new hand-rolled login

- **WHEN** a repository declares `ui.shared_layer.adopted: true` and still ships
  a login screen outside `consume_paths`
- **THEN** the checker reports `UI_AUTH_HANDROLLED` as `ERROR` while the codes
  the adoption does suppress stay silent

#### Scenario: Shared package's own screen

- **WHEN** a login-named UI file lies inside a declared
  `ui.shared_layer.consume_paths` directory
- **THEN** no `UI_AUTH_HANDROLLED` finding is emitted for it

#### Scenario: Directory named auth

- **WHEN** a file `Article.tsx` lives under `src/authors/`
- **THEN** no `UI_AUTH_HANDROLLED` finding is emitted, because the pattern is
  matched against the file name

### Requirement: Consult the owning token resolver, not a Python copy

The checker MUST determine whether a colour literal resolves to a token by
calling `declaredLiterals(resolveAliases(readTokens(...)))` from
`dotnet-platform-libs/ui/scripts/lib/token-model.mjs` in a bounded `node`
subprocess, and MUST NOT re-implement the token walk, the alias chain, or the
canonical name rule in Python. It MUST load the literal set once per run. It
MAY restrict the comparison to values spelled as `#hex`, `rgb()` or `hsl()`, and
MAY lowercase and expand 3-digit hex, because those are string-shape and
spelling operations. It MUST NOT convert between colour spaces or apply a
similarity threshold.

#### Scenario: Literal declared in the token source

- **WHEN** a UI file contains `#0f172a` and the token source declares that
  literal
- **THEN** no `UI_COLOR_LITERAL` finding is emitted for it

#### Scenario: Literal not in the token source

- **WHEN** a UI file contains `#ff00aa` and the token source declares no such
  literal
- **THEN** the checker reports `UI_COLOR_LITERAL` as `ERROR`

#### Scenario: Token source cannot be consulted

- **WHEN** `node` is unavailable, the token module is missing, the token source
  is unreadable, an alias is unresolved, or the subprocess times out
- **THEN** every colour literal found is reported as `UI_COLOR_LITERAL`, a note
  names the exact cause, the run records a coverage gap, and the process exits
  non-zero. The run is never a pass

### Requirement: Require the layer, the theme, the brand and the measurement

A repository that ships UI and declares no `ui.shared_layer.adopted: true` MUST
be reported `UI_TOKEN_NOT_CONSUMED`. A repository that ships UI and declares no
theme mechanism inside its scope, and no adoption, MUST be reported
`UI_THEME_ABSENT`. A repository that ships UI with no resolvable branding
document MUST be reported `UI_BRANDING_ABSENT`. A repository that ships UI with
no resolvable measured-contrast artefact MUST be reported
`UI_CONTRAST_UNMEASURED`. A declared adoption of the shared layer MUST suppress
`UI_THEME_ABSENT`, `UI_BRANDING_ABSENT` and `UI_CONTRAST_UNMEASURED`, and MUST
NOT suppress the others.

#### Scenario: Nothing declared

- **WHEN** a repository ships UI and declares no `ui` block at all
- **THEN** `UI_TOKEN_NOT_CONSUMED`, `UI_THEME_ABSENT`, `UI_BRANDING_ABSENT` and
  `UI_CONTRAST_UNMEASURED` are all reported

#### Scenario: Adoption suppresses the three declarations

- **WHEN** a repository ships UI and declares `ui.shared_layer.adopted: true`
- **THEN** `UI_TOKEN_NOT_CONSUMED`, `UI_THEME_ABSENT`, `UI_BRANDING_ABSENT` and
  `UI_CONTRAST_UNMEASURED` are silent

#### Scenario: Declared branding path does not resolve

- **WHEN** `ui.branding_document` names a path that does not exist
- **THEN** `UI_BRANDING_ABSENT` is reported and the default file names are not
  used as a fallback

### Requirement: Report a shrinking baseline without becoming a switch

The checker MUST accept a baseline file whose entries list, per project, the
finding codes measured at snapshot time, and MUST downgrade exactly those
findings to non-blocking while still counting and naming them. A finding whose
code is not listed MUST fail normally. A run without `--baseline` MUST fail
closed on every finding. The baseline generator MUST support `--check` and MUST
report an entry whose measured state no longer holds as stale.

#### Scenario: Covered code

- **WHEN** a project's baseline entry lists `UI_AUTH_HANDROLLED` and that is the
  only finding raised
- **THEN** the run exits 0, the finding is counted as baselined, and the note
  names the project and the code

#### Scenario: New code not in the entry

- **WHEN** a project's baseline entry lists `UI_AUTH_HANDROLLED` only and the
  checker raises `UI_COLOR_LITERAL`
- **THEN** the run exits 1 on the colour finding

#### Scenario: Baseline drift

- **WHEN** the generator's `--check` recomputes an entry that no longer matches
  the measured state
- **THEN** it exits 1 and reports the stale entries, writing nothing

### Requirement: Fail closed on a configuration error

The checker MUST exit 0 when conformant or fully baselined, 1 when a blocking
`ERROR` is outside the baseline, and 2 on a configuration error or a
`REVIEW_REQUIRED` finding outside the baseline. It MUST read nothing outside
the scanned project except the declared token source, and MUST write nothing
inside a scanned project.

#### Scenario: Conformant repository

- **WHEN** a repository declares a scope, declares adoption, and ships no
  hand-rolled auth screen
- **THEN** the process exits 0

#### Scenario: Blocking finding

- **WHEN** a repository ships a hand-rolled auth screen and no baseline covers
  it
- **THEN** the process exits 1 and the finding names the file

#### Scenario: Configuration error

- **WHEN** the project directory does not exist, or `.project.json` is not a
  JSON object
- **THEN** the process exits 2 and writes a diagnostic to stderr