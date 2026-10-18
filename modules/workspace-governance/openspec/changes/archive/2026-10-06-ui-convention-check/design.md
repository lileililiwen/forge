# Design: ui-convention-check

## 1. Implementation boundary

Repository `/home/paul/code/workspace-governance`, Python 3 standard library
only, plus one bounded `node` subprocess for the token resolver.

New files, and nothing else:

- `scripts/ui_conformance.py` — the checker and its CLI.
- `scripts/generate_ui_conformance_baseline.py` — the baseline writer.
- `ui-conformance-baseline.json` — the recorded snapshot.
- `tests/test_ui_conformance.py` — the tests.

**Not modified:** `scripts/workspace_check.py`, `scripts/product_code_quality.py`,
`vocabulary.json`, `projects.json`, `schemas/project.schema.json`,
`gate-debt*.json`, and every file outside this repository. The checker is
invoked standalone or by `--audit`; wiring it into `validate_project()` is a
separate change, because that file is not owned here.

**Not touched by another agent:** `skills/workspace-governance/**` and
`docs/ui-convention.md` are owned by a parallel agent and are left alone.

Read-only, like `product_code_quality.py`: the checker writes nothing inside a
scanned project. The only writes are the two `--out`-shaped outputs of the
baseline generator, which is a separate entry point.

## 2. The UI surface

A **UI file** is a regular file whose suffix is one of

```
.tsx .jsx .vue .svelte .css .scss .razor .cshtml .html .dart
```

under a repository root, pruning these directory names at any depth:

```
target/ node_modules/ obj/ bin/ dist/ build/ .git/ __pycache__/ coverage/ .next/
```

The census is one `os.walk` pass with in-place directory pruning, not one
`rglob` per extension: a single pass is what keeps a 2067-file repository cheap
to enumerate. Symlinked files are skipped; a symlinked directory is not
followed. That is the fail-closed direction — an unreadable path is a counted
coverage gap, never a silent pass.

**Ships UI** is `len(census) > 0`. A repository with no UI file is out of the
capability entirely and is never reported against.

## 3. The declaration surface

Read from the project's own `.project.json`. Every block is additive; a project
that declares none of them gets the fail-closed findings, which is the point.

```json
{
  "ui": {
    "scope": {
      "roots": ["apps/web/src"],
      "exclude": ["apps/web/src/legacy"],
      "max_files": 400
    },
    "shared_layer": {
      "adopted": true,
      "evidence_ref": "apps/web/package.json",
      "consume_paths": ["node_modules/@platform/react-auth"]
    },
    "branding_document": "docs/branding.json",
    "contrast_evidence": "docs/contrast-audit.json",
    "token_source": "../dotnet-platform-libs/ui/tokens/tokens.json"
  }
}
```

`roots` entries must be project-relative and must resolve inside the project
root. An absolute entry, a `..` escape, or a root that resolves outside the
project is a **configuration error**, not a finding — it is exit 2, because a
scope that does not apply cannot produce a verdict and must not be reported as
a pass.

`max_files` defaults to `DEFAULT_SCOPE_CEILING = 400`. The default is a
committed constant in `ui_conformance.py`, never an environment variable:
`AGENTS.md` forbids a checker whose behaviour depends on an environment
variable being set, because a CI runner that lacks it fails differently from
one that has it.

## 4. The token boundary — the rule this change must not get wrong

`UI_COLOR_LITERAL` asks whether a colour literal in UI source *resolves to a
token* in `dotnet-platform-libs/ui/tokens/tokens.json`. That repository owns
the resolution semantics, and it implements them in
`ui/scripts/lib/token-model.mjs`:

- `readTokens(source)` walks the DTCG tree.
- `resolveAliases(tokens)` follows `{a.b.c}` alias chains and **throws
  `TokenCheckError` on an unresolved alias or a cycle**.
- `declaredLiterals(tokens)` returns every literal a generated artifact is
  allowed to contain.

This checker does **not** re-implement that walk, the alias chain, or the
canonical name rule in Python. It spawns one bounded `node` process per run,
imports the module, and reads its `declaredLiterals()` output:

```js
import { readTokens, resolveAliases, declaredLiterals } from '<module>';
const src = readFileSync(process.argv[2], 'utf8');
process.stdout.write(JSON.stringify([...declaredLiterals(resolveAliases(readTokens(src)))]));
```

Consequences, each of them deliberate:

1. **One spawn per run, not per file.** The literal set is loaded once and
   compared in Python. The subprocess is bounded by a timeout
   (`DEFAULT_NODE_TIMEOUT = 30` seconds) and its stderr is bounded before it can
   reach a finding message.
2. **The token source is a committed, repo-relative input.** Default is
   `DEFAULT_TOKEN_SOURCE = "../dotnet-platform-libs/ui/tokens/tokens.json"`
   relative to the governance root, overridable per repository by
   `ui.token_source` and by `--tokens`. No environment variable.
3. **Only *colour* literals are compared in Python.** `declaredLiterals()`
   returns space, radius and font values too. Filtering them to the
   `#hex` / `rgb()` / `hsl()` class is a shape test on a string, not a token
   semantic, and it is stated here so the boundary is auditable.
4. **Normalisation is spelling, not semantics.** Literals are lowercased and
   3-digit hex is expanded to 6-digit before comparison, so `#FFF` and `#ffffff`
   are the same literal. Nothing else is rewritten — no colour conversion, no
   perceptual distance, no "close enough" threshold.
5. **When the module cannot be consulted, the checker fails closed.** No `node`,
   a missing module, a missing token source, an unresolved alias, or a timeout
   all produce: every colour literal found is reported as
   `UI_COLOR_LITERAL`, plus a `REVIEW_REQUIRED` note naming the exact cause.
   A literal is never declared token-reachable by a comparison that did not
   run.

## 5. Per-rule semantics

| Code | Reads | Fires when | Suppressed by |
|---|---|---|---|
| `UI_SCOPE_UNDECLARED` | census | ships UI, no `ui.scope.roots` | nothing |
| `UI_AUTH_HANDROLLED` | census (paths) | auth-named UI file outside `consume_paths` | nothing |
| `UI_TOKEN_NOT_CONSUMED` | declaration | ships UI, `ui.shared_layer.adopted` is not true | adoption |
| `UI_COLOR_LITERAL` | declared scope only | colour literal not in `declaredLiterals()` | nothing |
| `UI_THEME_ABSENT` | declared scope only | no theme mechanism in scope, no adoption | adoption |
| `UI_BRANDING_ABSENT` | declaration | ships UI, no branding doc resolved | adoption |
| `UI_CONTRAST_UNMEASURED` | declaration | ships UI, no contrast artefact resolved | adoption |

**Auth filename pattern** (case-insensitive, matched against the file *name*
only, never the directory path, so `src/authors/Article.tsx` does not fire):

```
login | logout | register | signup | sign-in | signin | forgot.?password | reset.?password | auth
```

**Theme mechanism** — any of: a `[data-theme=...]` attribute selector, a
`prefers-color-scheme` media query, or a theme provider / hook
(`ThemeProvider`, `useTheme`, `theme_provider.dart`).

**Branding document** — `ui.branding_document` if declared and resolvable,
otherwise `branding.json` / `branding.yaml` / `branding.yml` at the repository
root. A declared path that does not resolve is a finding, not a fallback to the
default name.

**Contrast artefact** — `ui.contrast_evidence` if declared and resolvable,
otherwise `contrast-audit.json` / `ui-contrast.json` at the repository root.

## 6. Rulings

1. **A missing declaration is REVIEW_REQUIRED, never a guessed pass.** The
   checker does not infer scope from directory names and does not infer
   adoption from a lockfile.
2. **Adoption suppresses 5-7 but never `UI_AUTH_HANDROLLED`.** Declaring the
   shared layer is a statement about what the repo consumes; shipping a login
   form is a statement about what it built. Both are reported.
3. **A consume path is a path exemption, not a content exemption.** A file
   inside `consume_paths` is not a hand-rolled screen because it is the shared
   package's own screen. The pattern is still reported for any file outside
   those paths.
4. **Filename-level and content-level rules have different scopes.** See
   §7. This is why `alltools-platform` does not emit thousands of findings
   without being skipped.
5. **The census decides coverage; the declaration decides content.** The census
   is repo-wide and cheap. Content is read only where the repo said to look.
6. **A symlink, an unreadable file, or a truncated read is a coverage gap.**
   It is recorded in `summary.unreadable` and, if it could hide a finding,
   raises the run to `REVIEW_REQUIRED`.

## 7. The scope rule, stated in one line

> The census runs repo-wide; content is read only inside declared
> `ui.scope.roots`; a repository over `ui.scope.max_files` (default 400) with no
> declared scope is reported with its exact count, is fired
> `UI_SCOPE_UNDECLARED`, and is recorded in the baseline with
> `scope_oversize: true`.

`alltools-platform` therefore emits at most: the declaration findings, plus the
auth-named file paths it really has (24 today), plus one note naming 2067 and
the 400 ceiling. It is not skipped, and it does not drown the report. Today it
is the only repository over the ceiling; `rust-ecommerce` (305) and
`openlearning` (239) are the next largest and sit under it.

`dotnet-platform-libs` is the **provider**, not a consumer: it ships the layer
itself, so its own auth files are the layer's source. It is entered in the
baseline with `role: "provider"` so that record reads as what it is.

## 8. Exit codes and baseline semantics

| Exit | Meaning |
|---|---|
| 0 | conformant, or every finding is covered by the supplied baseline |
| 1 | at least one blocking `ERROR` finding outside the baseline |
| 2 | configuration error, or a `REVIEW_REQUIRED` finding outside the baseline |

A baseline entry lists, per project, the finding **codes** that were measured
at snapshot time. Suppressed findings are counted in
`summary.baselined` and named in the notes; they are never deleted from the
report, only downgraded. A finding whose code is *not* in the entry fires
normally, which is what makes the baseline a ratchet rather than a switch: a
new `UI_COLOR_LITERAL` in a repository whose entry only lists
`UI_AUTH_HANDROLLED` still exits 1.

`--check` on the generator recomputes the entries and reports drift. A stale
entry — a repository that no longer has the baselined finding — is dead weight
and is reported as such, the same shape as `gate_debt_baseline.py --check`.

## 9. Tests

`tests/test_ui_conformance.py`, unittest, matching the style of
`tests/test_product_code_quality.py`:

- scope parsing: valid, absolute entry, `..` escape, missing root, default
  ceiling, override ceiling
- census: suffix set, pruned directories, symlink skip, symlinked directory not
  followed
- each of the eight codes fires in isolation
- adoption suppresses 5-7 and **not** `UI_AUTH_HANDROLLED` — the combination,
  stated as its own test
- a consume path exempts the shared package's own login screen
- colour literal: in the token set, `#FFF` normalising to `#ffffff`, not in the
  token set, and the token-source-unavailable path failing closed
- baseline: covered code suppressed, uncovered code still fires, drift detected
- CLI: exit 0 / 1 / 2 on three scratch repositories built with
  `tempfile.TemporaryDirectory()`
- the load-bearing proof: a conformant scratch repository gains a hand-rolled
  `LoginPage.tsx` and the run flips from 0 to 1

## 10. What this change does not do

- It does not migrate a repository. Adoption is a separate change with its own
  grant (`cross-repo-adoption-grant`).
- It does not wire itself into `validate_project()`. See §1.
- It does not define the conventions themselves. `docs/ui-convention.md` and
  the skill are owned by a parallel agent.
- It does not decide whether a repo *should* adopt the layer; that is a
  recorded portfolio decision.