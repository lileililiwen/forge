# Proposal: Judge a login page on what it renders, not on what it is called

## Why

`UI_AUTH_HANDROLLED` fires on a **filename pattern alone**. The file is read by
nothing. That was the correct narrowing in the archived design (`§6` ruling 2:
adoption must never suppress it) and it is wrong for the one repository in the
workspace that actually did the work.

`somedanote` vendored the shared layer, transcribed its auth composition into
Razor, and rendered every screen through it. It still fails:

```
FAIL: somedanote ui_files=15 errors=5 review_required=1 baselined=0
  UI_AUTH_HANDROLLED src/SomadaNote.Web/Pages/Platform/Logout.cshtml
  UI_TOKEN_NOT_CONSUMED .project.json ui.shared_layer.adopted is not declared true
  UI_BRANDING_ABSENT  .project.json no branding document resolvable
  UI_CONTRAST_UNMEASURED .project.json no measured-contrast artefact resolvable
```

`Login.cshtml` carries `platform-auth-card`, `platform-auth-title`,
`platform-auth-description`, `platform-auth-form`, `platform-auth-header`,
`platform-auth-logo`, `platform-button`, `platform-control`, `platform-shell`
and `platform-state`. That is the shared layer, rendered server-side. The name
of the file is the only thing the rule looked at.

Meanwhile a genuine non-adopter is correctly frozen. The ratchet is right for
58 repositories and wrong for the one that migrated, which means a repository
is punished for succeeding and the finding code cannot tell a *migrated* screen
from a *hand-rolled* one.

## What changes

`UI_AUTH_HANDROLLED` keeps its pattern and its ERROR level, and stops being
decided by the pattern alone. Before reporting, the checker asks whether the
file **demonstrably renders the shared layer**:

| Signal | What it reads | Example |
|---|---|---|
| shared class name | one or more `platform-*` names in the file | `class="platform-auth-card"` |
| shared token stylesheet | a `.css` reference whose path names the shared layer | `~/css/platform/design-tokens/dist/tokens.css` |
| shared-layer stylesheet | a `.css` reference whose path names the shared layer | `~/css/platform/react-auth/styles.css` |

A server-rendered page that renders **no markup of its own** is then judged by
the surface it composes — its `@layout`, or the `Layout` its nearest
`_ViewStart.cshtml` assigns. That is how a POST-only `Logout.cshtml` is
accounted for: the screen is rendered by the shared shell, and the shell links
the shared stylesheets.

## What this does not become

- **Not a switch.** Adoption never silences a page that renders a screen of its
  own. The three signals are read from the file, not from the declaration.
- **Not a port of the token resolver.** No DTCG walk, no alias chain, no token
  semantics move into Python. This is string matching over file content, in the
  same class as the existing theme-mechanism regex.
- **Not a migration.** `somedanote`'s declaration is that repository's own
  change, with its own spec and its own commits.
- **Not a change to the other 58 repositories.** Measured: every registered
  repository that ships UI and declares no adoption keeps exactly the findings
  it has today.

## Impact

| Surface | Before | After |
|---|---|---|
| Registries declaring adoption | 0 | 0 (this change declares none) |
| Repositories whose `UI_AUTH_HANDROLLED` count changes | — | 1 (`dotnet-platform-libs`, the provider, −1) |
| Non-adopter repositories whose report changes | — | 0 |
| Finding codes added or renamed | 0 | 0 |
| Files opened by the auth rule | 0 | one per auth-named file, bounded by `MAX_FILE_BYTES` |

## Non-goals

No change to `UI_COLOR_LITERAL`, the baseline shape, the eight-code vocabulary,
the scope rule, or any repository outside this one.
