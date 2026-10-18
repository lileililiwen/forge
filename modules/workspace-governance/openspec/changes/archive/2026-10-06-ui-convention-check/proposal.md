# Proposal: Fail a repository that hand-rolls what the shared UI layer already provides

## Why

`dotnet-platform-libs/ui` is a complete, tested design-token and component
layer. Its adoption count across the portfolio is **zero**, and nothing in
Workspace Governance can see that. Measured today, with the surface census
fixed in `design.md` §2:

| Measured fact | Value |
|---|---|
| Registered repositories | 81 |
| Repositories shipping UI surface | 59 |
| UI files across the fleet | 4280 |
| Repositories with an auth-named UI file | 22 |
| Auth-named UI files | 113 |
| Repositories declaring adoption of the shared `ui` layer | 0 |
| Repositories declaring a UI scan scope | 0 |

Two governance checks already exist and neither one looks at this:
`product_code_quality.py` reads the `quality` block and looks for test code in
product scope; the `readme-status-claim` rule reads `README.md` and checks it
against the tree. A repository can satisfy both while shipping its own login
form, its own colour literals, and no theme mechanism at all, and the fleet
stays green.

The failure mode this change targets is concrete: **an agent asked to "add a
login page" writes a login page.** That is the compliant outcome under the
current rules, because the repo has no declaration saying the shared layer
owns authentication, and no gate that fails when it ships one. The result is
113 hand-written auth files across 22 repositories, each of which will need to
be edited separately forever.

## What changes

Add one standalone, read-only checker, `scripts/ui_conformance.py`, plus a
recorded baseline and its generator. The checker answers one question per
repository — *does this repo consume the shared UI conventions, or reimplement
them?* — through eight fixed finding codes.

The codes are fixed by the portfolio owner and are not negotiable in this
change:

```
UI_AUTH_HANDROLLED       ERROR            hand-rolled auth screen outside a declared consumption path
UI_TOKEN_NOT_CONSUMED    ERROR            UI shipped, shared ui layer adoption undeclared
UI_COLOR_LITERAL         ERROR            colour literal not reachable from the token source
UI_THEME_ABSENT          ERROR            UI shipped, no light/dark theme mechanism
UI_BRANDING_ABSENT       ERROR            UI shipped, no branding document
UI_CONTRAST_UNMEASURED   ERROR            UI shipped, no measured-contrast artefact
UI_SCOPE_UNDECLARED      REVIEW_REQUIRED  UI shipped, no scan scope declared
```

`ERROR` blocks. `REVIEW_REQUIRED` is the fail-closed "a human must look"
state, and a missing declaration is never turned into a guessed pass.

## The combination that matters

Rules 5-7 are suppressed by a declared adoption of the shared layer, because a
repository that adopts `@platform/react-ui` genuinely does not need its own
`branding.json` or its own `prefers-color-scheme` block. `UI_AUTH_HANDROLLED`
is **not** suppressed by adoption. A repository that declares the shared layer
and then hand-writes a new login page fails, and that combination is the
reason this checker exists. It is the case an adoption-only check would pass
and it has an explicit test.

## The scope problem, stated rather than skipped

`alltools-platform` ships 2067 UI files. It is a tooling platform, not a
consumer. Letting it emit one finding per colour literal would produce a
thousands-of-lines report that nobody reads, and skipping it silently would
produce a green gate over the largest UI surface in the portfolio.

Neither is acceptable, so the split is by *what a rule has to read*:

- **Filename-level rules run on the repo-wide census.** `UI_AUTH_HANDROLLED`
  matches on a path. It costs nothing on 2067 files and it is exactly the rule
  that must not be narrowed.
- **Content-level rules run only inside the repository's declared
  `ui.scope.roots`.** `UI_COLOR_LITERAL` and `UI_THEME_ABSENT` have to open the
  file. A repository that declares no scope therefore has its content rules
  silent *by declaration*, not by exemption, and `UI_SCOPE_UNDECLARED` fires so
  the silence is visible.
- **A ceiling makes the large repositories a named, recorded decision.** Any
  repository whose census exceeds `ui.scope.max_files` (default 400) is
  reported with its exact count and the ceiling it exceeded, and is entered
  into the baseline with `scope_oversize: true`. One repository in the fleet is
  over the ceiling today: `alltools-platform`, at 2067.

## Why a baseline is required for day one

The gate has to go live without turning 59 repositories red on the same
afternoon. `ui-conformance-baseline.json` freezes today's measured state, is
modelled on `gate-debt-baseline.json`, and **only shrinks**: adopting the layer
removes an entry permanently, and a `--check` mode reports drift so a stale
entry is visible rather than quietly load-bearing.

The baseline is not an opt-out switch. The default run, with no `--baseline`,
fails closed on every finding. The baseline only converts a recorded, dated,
drift-checked finding into a reported, counted, non-blocking one.

## Boundary

`dotnet-platform-libs/ui` resolves tokens through
`ui/scripts/lib/token-model.mjs`. This change **does not re-implement that
resolver in Python**; it calls the module and consumes its
`declaredLiterals()` output. Where the module cannot be consulted, the checker
fails closed and says so rather than guessing at token semantics. See
`design.md` §4.