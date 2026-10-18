# UI convention and the shared design-token layer

Read this **before writing any user interface** in any repository: a login
page, a register page, a logout control, a theme toggle, a colour, a spacing
value. The workspace already ships the layer. Your job is to render through it,
not to write it again.

Adoption lives in the owning repositories; this reference only routes there.

## The one question, and the one command

**Should this repository consume the shared UI layer, and is it doing so?**

## Run

- `python3 scripts/ui_conformance.py <this-repo>` — the whole answer for one
  repository. Read-only: it writes nothing inside a scanned project. Exit `0`
  is PASS, `1` is FAIL, `2` is REVIEW_REQUIRED or a configuration error.
- `python3 scripts/ui_conformance.py <this-repo> --json` — the same report, as
  a versioned JSON document.
- `python3 scripts/bind_ui_gate.py <this-repo> --check` — is the check actually
  bound into your gate? Non-zero when it is not.
- `python3 scripts/bind_ui_gate.py <this-repo> --apply` — bind it. Writes one
  file, in your repository, and nothing anywhere else.

The checker is **fail-closed and never guesses**. It does not infer that you
adopted the layer because you import something that looks like a token, and it
does not infer that you did not because your tree is unfamiliar. The owner
declares; the checker reads. A repository that ships **no** UI surface passes
without adopting anything — that is a real `PASS`, not a skip.

## The finding codes

Seven codes, fixed by the portfolio owner in `scripts/ui_conformance.py`. This
reference states what each one means for the change in hand; it does not
restate or rename the vocabulary.

| code | fires when | what to do |
|---|---|---|
| `UI_AUTH_HANDROLLED` | a UI file is named login / logout / register / signup / sign-in / forgot-password / reset-password / auth, and is not under a declared `consume_paths` | delete the screen and consume the shared layer's auth package. Not a baseline question. |
| `UI_TOKEN_NOT_CONSUMED` | `ui.shared_layer.adopted` is not declared `true` | declare it with a `consume_paths` list, or record why the layer is declined |
| `UI_COLOR_LITERAL` | a colour literal in the declared scope does not resolve to a token | replace the literal with a shared design-token reference. Not "pick a similar colour". |
| `UI_THEME_ABSENT` | no theme mechanism in the scanned files, and not adopted | adopt the shared `ThemeProvider`, or declare a `[data-theme]` / `prefers-color-scheme` mechanism |
| `UI_BRANDING_ABSENT` | no branding document resolvable, and not adopted | ship `branding.json` / `branding.yaml`, or declare `ui.branding_document` |
| `UI_CONTRAST_UNMEASURED` | no measured-contrast artefact resolvable, and not adopted | record a measured contrast run and declare `ui.contrast_evidence`. Build/test success is not contrast evidence. |
| `UI_SCOPE_UNDECLARED` | the repository ships UI files and declares no `ui.scope.roots` | declare the scope. Until you do, the content rules are silent **by declaration**, not by exemption. |

`UI_SCOPE_UNDECLARED` is `REVIEW_REQUIRED`; the rest are `ERROR`. Read the note
line above a finding before you read the finding: the checker reports a
declared silence as a note so it is not mistaken for a pass.

A separate, **frozen baseline** (`ui-conformance-baseline.json`) covers
pre-existing findings so the checker can start from a truthful position. A
baselined finding is reported as baselined, not as a pass, and the baseline
exists to shrink.

## Declaring: the `ui` block in your own `.project.json`

The checker reads an additive `ui` block from **your** `.project.json`. Every
path in it must be project-relative; an absolute path or a `..` is a
configuration error, because a checker that depends on a machine-specific path
fails differently on a CI runner.

```json
{
  "ui": {
    "scope": {
      "roots": ["src"],
      "exclude": [],
      "max_files": 400
    },
    "shared_layer": {
      "adopted": true,
      "consume_paths": ["vendor/platform-ui"]
    },
    "branding_document": "src/MyApp.Web/branding.json",
    "contrast_evidence": "contrast-audit.json"
  }
}
```

- `scope.roots` — the only files the content rules open. Without it they do not
  run, and `UI_SCOPE_UNDECLARED` says so.
- `shared_layer.adopted` — `true` once the feed is vendored and consumed.
- `shared_layer.consume_paths` — your vendored feed directory. Auth files
  under it are the shared pages, not hand-rolled ones.
- `branding_document` / `contrast_evidence` — where those artefacts live.
- `max_files` — the ceiling, default 400. Over it, filename rules still run
  over the whole census and content rules stay inside the declared scope.

The separate `ui-conformance.json` file in the `somedanote` repository belongs
to the **platform layer's** feed verification. It is a different document with a
different owner. Do not merge the two.

## Adopting: the order of operations

1. **Vendor the feed.** Copy the kit's tarballs, its `feed.json`, and its
   `branding.schema.json` into your repository and commit them. The feed is
   vendored, never published to a public npm scope (decision D1). No registry,
   no `npm publish`, no credential.
2. **Declare it** in your `.project.json`: `shared_layer.adopted: true` and the
   `consume_paths` you just vendored into.
3. **Verify the digests** with the sync script in check mode. It must pass with
   no sibling checkout present — that is the property being proven.
4. **Wire `branding.json`.** Name, logo, navigation, footer links, locale,
   default theme, validated at build time against the vendored schema
   (decision D4). You own this file; the layer owns its shape.
5. **Replace the hand-rolled page.** Delete your own login/register/logout and
   render the shared one, then add the directory to `consume_paths` so the
   shared pages are not themselves reported.
6. **Record measured contrast** and declare `ui.contrast_evidence`.
7. **Make it gate.** `python3 scripts/bind_ui_gate.py <this-repo> --apply`
   selects `ui-conformance` in your `.ai-gate/gate.yaml` and binds the command.
   A checker no gate runs does not gate (ruling D18).

### Per stack

| stack | what you write | what proves it |
|---|---|---|
| **Razor Pages / ASP.NET** | commit the `platform-*.css` files into your web assets; transcribe the shared markup into a `Pages/Platform/` directory; read branding through a loader | a test asserting the hand-rolled pages are gone and that every shared class your pages use is one the shared stylesheet defines |
| **React / Next.js** | a `file:` specifier per package pointing at your committed tarball; import from `react-ui` and `react-auth` | your existing test command, plus the digest check |
| **Django / Jinja** | no shared path ships today. Wave 4 of the UI adoption migration owns it. | not yet available — do not hand-roll a private token set in the meantime |
| **Rust maud / Hugo** | no shared path ships today. Wave 4 owns it. | not yet available |
| **Flutter / Dart** | nothing. There is no Dart artifact and there will not be one under this decision (D3). | not applicable — a permanent exemption, not a gap |

## The worked example: `somedanote`

It is the only real consumer in the workspace. Read these before inventing
anything. The paths below are inside the `somedanote` repository, relative to
its root — they are written as prose rather than as repo-relative paths because
they are not paths in *this* repository.

| path in `somedanote` | what it is |
|---|---|
| vendor/platform-ui/ | the vendored feed: 4 tarballs, feed.json, branding.schema.json |
| scripts/sync-platform-ui.mjs | `--check` verifies committed bytes with no sibling needed; `--sync` copies from the kit then verifies. It refuses absolute, `..`-escaping, and env-var paths before resolving anything. |
| ui-conformance.json | the platform layer's feed declaration, with a digest per package |
| src/SomadaNote.Web/branding.json | the repo-owned branding document (D4) |
| scripts/verify-branding.mjs | validates it against the **vendored** schema, not a copy of the rules |
| src/SomadaNote.Web/Branding/ | the loader; changing the logo is a data change, not a code change |
| src/SomadaNote.Web/Pages/Platform/ | the shared auth pages, transcribed for Razor |
| tests/SomadaNote.Tests.Unit/SharedUi/ | the tests that stop the transcription drifting into a private accessibility contract |

The last row is the part people skip. A transcription is a copy; the tests are
what make it a transcription.

Run the checker against it and read the result before you copy anything:

```
python3 scripts/ui_conformance.py ../somedanote
```

A repository that has adopted still carries findings until it declares its
scope. `somedanote` today reports `UI_SCOPE_UNDECLARED` and two
`UI_AUTH_HANDROLLED` findings, because it ships the shared pages under a path
its `.project.json` does not yet declare. That is the declaration work
remaining, and it is the same work you will have.

## Decided. Do not re-open.

These are rulings recorded 2026-10-06. Read them as facts, not as questions.

| id | decision |
|---|---|
| **D1** | The feed is **vendored** per repository. Never published to a public npm scope. Publishing under the owner's npm identity is an account-level action, not a step in adoption. |
| **D2** | The shared auth package ships a **local username/password form for site users**. forge's OIDC+PKCE is for the **operator portal**. Different actors, not competing designs. Do not replace one with the other. |
| **D3** | **Flutter/Dart is a permanent exemption.** No Dart token artifact. The 8 Dart repositories are not counted as drift. |
| **D4** | `branding.json` is owned by **each repository**, schema-validated at build time. A product changes its own logo without a governance round trip. |

## Out of scope here

- **Migrating a repository is that repository's own change**, with its own spec
  and its own commits. This reference tells you how; it does not do it for you.
- **This repository does not edit another repository's source.** Consuming a
  committed artifact is a consumer-side change; editing a sibling's source needs
  scope authorization. See `platform-adoption.md`.
- **The baseline is not yours to edit.** It is a data file, it exists to shrink,
  and adding an entry to silence a finding is not adoption.
- **The gate binding is per repository.** `bind_ui_gate.py` has no fleet mode by
  design. Binding the workspace is the owner's decision, one repository at a
  time.
