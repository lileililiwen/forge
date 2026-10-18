# UI convention conformance

The workspace ships one design-token and component layer. This document
explains what the conformance checker enforces, why a frozen baseline exists,
how to read a finding, and how to discharge one.

This is the human-facing document. The agent-facing one is
`skills/workspace-governance/references/ui-convention.md`. They state the same
codes, the same policies, and the same scope; if they ever disagree, the
checker is the authority and one of them is a bug.

## Why this exists

The layer is built, tested and packaged. For most of its life nothing adopted
it. Measured on 2026-10-06:

| Measured fact | Count |
|---|---|
| Hand-written login/register/logout page files | 44 |
| Repositories holding those files | 17 |
| Repositories each implementing their own theme switching | 22 |
| Repositories shipping a UI | 64 |
| Repositories with hand-rolled auth files | 21 |
| `dotnet-platform-libs/ui` adopters | 1 (`somedanote`) |

The counts come from `ui-adoption-migration.json`, whose evidence block names
the frozen source and the exact exclusions used. Two of them are 2026-10-03
figures read from the platform layer's frozen baseline; they are a floor, not a
live count. Re-measure before quoting them as current.

The cost was never the code. It was that no repository could *find out* it was
drifting. The checker's job is to make that answer cheap and unignorable.

## What the checker enforces

`scripts/ui_conformance.py` is read-only over a scanned project. It writes
nothing inside it, installs nothing, and reformats nothing.

Its subject is **absence, not presence**. A repository that ships no user
interface passes without adopting anything — that is a real `PASS`, not a
skip. A repository that ships a UI is expected to have declared what it does
with the layer.

Everything is driven by an additive `ui` block in the repository's own
`.project.json`:

```json
{
  "ui": {
    "scope": { "roots": ["src"], "exclude": [], "max_files": 400 },
    "shared_layer": { "adopted": true, "consume_paths": ["vendor/platform-ui"] },
    "branding_document": "src/MyApp.Web/branding.json",
    "contrast_evidence": "contrast-audit.json"
  }
}
```

The design rule is **the owner declares, the checker never guesses**. It does
not infer adoption from an import that looks like a token, and it does not
infer its absence from an unfamiliar tree. Absent a declaration, the
fail-closed findings fire, and that is the intended reading of silence.

## The seven finding codes

| code | severity | fires when | the discharge |
|---|---|---|---|
| `UI_AUTH_HANDROLLED` | ERROR | a UI file named login / logout / register / signup / sign-in / forgot-password / reset-password / auth is not under a declared `consume_paths` | delete the screen; consume the shared auth package; add the shared pages' directory to `consume_paths` |
| `UI_TOKEN_NOT_CONSUMED` | ERROR | `ui.shared_layer.adopted` is not `true` | declare it with `consume_paths`, or record why the layer is declined |
| `UI_COLOR_LITERAL` | ERROR | a colour literal in the declared scope does not resolve to a token | replace the literal with the token reference |
| `UI_THEME_ABSENT` | ERROR | no theme mechanism in the scanned files, and not adopted | adopt the shared `ThemeProvider`, or declare a `[data-theme]` / `prefers-color-scheme` mechanism |
| `UI_BRANDING_ABSENT` | ERROR | no branding document resolvable, and not adopted | ship one, or declare `ui.branding_document` |
| `UI_CONTRAST_UNMEASURED` | ERROR | no measured-contrast artefact resolvable, and not adopted | record a measured contrast run and declare `ui.contrast_evidence` |
| `UI_SCOPE_UNDECLARED` | REVIEW_REQUIRED | the repository ships UI files and declares no `ui.scope.roots` | declare the scope |

Exit codes: `0` PASS, `1` FAIL, `2` REVIEW_REQUIRED or a configuration error.

## Two properties worth knowing

**Fail-closed token comparison.** `UI_COLOR_LITERAL` asks whether a literal
resolves to a declared token. That resolution is *not* reimplemented in Python.
The checker spawns one bounded `node` process, calls the platform's own
`ui/scripts/lib/token-model.mjs`, and compares against its output. When that
module cannot be consulted, the check fails closed and names the cause. A
literal is never declared token-reachable by a comparison that did not run.

**Declared silence is visible.** Content rules read only files inside
`ui.scope.roots`, because they have to open the file. A repository that
declares no scope has its content rules silent *by declaration* rather than by
exemption — and `UI_SCOPE_UNDECLARED` fires so the silence is visible rather
than being read as a pass. `somedanote` carries this finding today.

## Why a frozen baseline exists

The checker started against a workspace where the drift was the norm, not the
exception. Starting with every pre-existing finding failing would have made the
first run uninformative: a repository that has been wrong for two years looks
exactly like one that regressed today.

So `ui-conformance-baseline.json` records the findings that pre-date the
checker. Its rules:

- A baselined finding is **reported as baselined, not as a pass**. The report
  distinguishes the two, so the baseline never hides that the debt exists.
- The baseline **may only shrink**. A repository that discharges a finding
  leaves it. Adding an entry to silence a finding is not adoption.
- The baseline lives at the governance root, is read as a committed data file,
  and is regenerated by a separate tool. It is never a script constant.

A new finding in a repository that already has baselined findings still fails.
The baseline covers the past, never the future.

## Discharging a finding

In order, and not interchangeably:

1. **Do the work.** Delete the hand-rolled screen, replace the literal with a
   token, declare the scope.
2. **Declare it.** Update the repository's own `ui` block. A discharge that
   changes no declaration usually means the finding was silenced rather than
   fixed.
3. **Re-run** `python3 scripts/ui_conformance.py <this-repo>` and read the
   result. Do not assume.
4. **Re-measure contrast** if the discharge touched colour. A finding that
   claimed a token fix and left contrast unmeasured is not discharged.
5. **Bind it to the gate**, once, by name: `python3 scripts/bind_ui_gate.py
   <this-repo> --apply`. A checker no gate runs does not gate.

## The decided policies

Recorded 2026-10-06 in `ui-adoption-migration.json`. These are rulings, not open
questions, and this document does not re-open them.

- **D1 — the feed is vendored.** Per repository, committed as bytes. Never
  published to a public npm scope; that is an account-level action under the
  owner's npm identity, not a step in adoption.
- **D2 — two different actors.** The shared auth package ships a local
  username/password form for site users. forge's OIDC with PKCE is for the
  operator portal. Different actors, not competing designs.
- **D3 — Flutter/Dart is a permanent exemption.** There is no Dart token
  artifact and this decision does not create one. The Dart repositories are not
  counted as drift.
- **D4 — `branding.json` is repo-owned.** Each repository owns the document; the
  layer owns its shape, and the shape is validated at build time. A product
  changes its own logo without a governance round trip.

## Out of scope

- **Migrating a repository is that repository's own change**, with its own spec
  and its own commits. This repository documents the path; it does not walk it.
- **This repository does not edit another repository's source.** Consuming a
  committed artifact is consumer-side; editing a sibling's source needs scope
  authorization.
- **The gate binding is per repository.** `scripts/bind_ui_gate.py` has no
  fleet mode, no `--root`, and no discovery. Binding the workspace is the
  owner's decision, one repository at a time.
- **The baseline is not extended.** It is not a registry of excuses, and this
  change adds nothing to it.

## Related

- `ui-adoption-migration.json` — the measured problem, the waves, and the four
  rulings with their reasons.
- `skills/workspace-governance/references/ui-convention.md` — the agent-facing
  version, with the per-stack adoption path and the worked example.
- `docs/adoption-grant.md` — the bounded permission to consume a shared
  provider.
