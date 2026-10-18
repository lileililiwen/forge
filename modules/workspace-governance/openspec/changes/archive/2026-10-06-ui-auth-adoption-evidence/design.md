# Design: ui-auth-adoption-evidence

## 1. The defect, stated as a rule defect

The archived design made `UI_AUTH_HANDROLLED` a pure path rule on purpose: it
costs nothing across the census and it is the rule that must not be narrowed by
a declaration. Both halves of that decision survive. What does not survive is
the assumption that a file's **name** determines whether the file is a
hand-rolled screen. A Razor transcription of the shared `react-auth`
composition is byte-for-byte the shared composition and is still reported.

## 2. Signals, and what each one is allowed to mean

Three content signals, all matched against the file's own text:

| id | Regex | Reads |
|---|---|---|
| `SHARED_CLASS_RE` | `\bplatform-[a-z0-9]+(?:-[a-z0-9]+)*\b` | the shared class vocabulary |
| `SHARED_LAYER_PATH_RE` | `@platform/`, `platform-ui/`, `/platform/`, `design-tokens/` | the shared layer's own path vocabulary |
| `CSS_REFERENCE_RE` | a quoted `….css` path | a stylesheet reference |

A file shows shared-layer use when it carries a `platform-*` name, or when one
of its stylesheet references names the shared layer. The second covers both
named cases in the proposal — the vendored token stylesheet and a vendored
shared-layer stylesheet — because a path that names the layer and ends in `.css`
is either of them.

Deliberately coarse, deliberately named: these are string shapes over file
content, the same class of test as the existing `THEME_MECHANISM_RE`. They are
not token semantics. The DTCG walk, the alias chain and the canonical name rule
stay in `token-model.mjs`, and `UI_COLOR_LITERAL` continues to consult it over
one bounded `node` subprocess (archived `design.md` §4.4).

### The division of labour with the adopting repository

A `platform-*` **name** is evidence that the file speaks the shared vocabulary,
not that the stylesheet exists. A repository can write the name into a comment
and gain nothing real. That is why the checker's half is a name test and the
adopter's half is a build-time test: `somedanote`'s
`SharedUiContractTests.TheAuthPagesUseOnlyClassNamesTheSharedStylesheetsDefine`
asserts that every `platform-*` name its pages use is **defined** by a
stylesheet the shared feed emitted. The checker sees the claim; the adopter's
own suite proves it. Neither half is asked to do the other's job.

## 3. The composition signal, and the gate around it

A `.cshtml`/`.razor` page can be a handler with no screen: `somedanote`'s
`Platform/Logout.cshtml` is `@page`, `@model`, a `ViewData` assignment, a
Razor comment, and no markup, because the sign-out control that posts to it is
rendered by the shared shell in `Pages/Shared/_Layout.cshtml`.

So a page that renders no markup of its own is judged by the surface it
composes:

1. `@layout = "…"` in the page, resolved against the page's own directory and
   against the Pages root; otherwise
2. `Layout = "…"` in the nearest ancestor `_ViewStart.cshtml`, resolved as
   `<root>/<name>.cshtml` then `<root>/Shared/<name>.cshtml`.

The resolved surface must itself show a shared-layer signal. Gates, all three
required:

- the repository declares `ui.shared_layer.adopted: true`;
- the file is a Razor page (`.cshtml`, `.razor`) — never a `.tsx`, `.jsx`,
  `.vue`, `.svelte`, `.html` or `.dart` file;
- the file renders no markup of its own (`<` followed by a letter, `!`, `?` or
  `/`), and the composed surface resolves **inside** the project and shows a
  shared-layer signal.

Anything else — unresolved layout, unreadable layout, a layout with no shared
signal, no declaration, a non-Razor file — is reported. This is the only place
a declaration silences a finding, and it can only apply to a file that renders
nothing at all.

**Measured consequence:** of the 113 auth-named files in the fleet, six Razor
pages render no markup (`cvunify`, three in `openlearning`, two in
`somedanote`). Neither `cvunify` nor `openlearning` declares adoption, so their
findings are unchanged. The gate is what makes that true, and it is the reason
the gate exists rather than leaving the composition signal open to any
repository with a shared-looking layout.

## 4. Fail-closed, itemised

| Condition | Result |
|---|---|
| file unreadable | reported (the content test could not run) |
| read truncated at `MAX_FILE_BYTES` | reported unless a signal was already in the read prefix |
| no signal, no composition | reported |
| composition unresolvable | reported |
| declared adopter writes a fresh page with markup | reported — the declaration is not consulted |
| `platform-*` named in a comment | silences **this** finding only; the stylesheet still has to define the name, proven by the adopter's own suite |

Every silence is written to `report.notes`, naming the file and the signal. A
finding that stops being reported and is not named is indistinguishable from a
checker that stopped looking, and `AGENTS.md` is explicit that this is worse
than no gate.

## 5. Cost

The auth rule now opens the files it matches. Fleet-wide that is 113 files, all
bounded by `MAX_FILE_BYTES`, and one extra `_ViewStart.cshtml`/`_Layout.cshtml`
read per Razor page that renders no markup. The repo-wide census itself is
unchanged, so the 2067-file `alltools-platform` boundary from archived
`design.md` §7 is unaffected: it emits at most its 6 real auth paths.

## 6. The recorded baseline

`dotnet-platform-libs` is the provider. Its
`ui/packages/react-auth/src/auth-forms.tsx` is the shared layer's own source
and carries `platform-*` names, so its `UI_AUTH_HANDROLLED` count drops from 6
to 5 after this change. The entry's code list is unchanged — the code is still
raised for its other five auth-named paths — but its recorded count is stale,
and `generate_ui_conformance_baseline.py --check` reports stale entries as dead
weight. The baseline is regenerated with its own generator, which only ever
shrinks: the change removes one grandfathered finding and adds nothing.

## 7. What this change does not do

- It does not migrate a repository, declare anything on a repository's behalf,
  or touch a sibling checkout.
- It does not change `UI_COLOR_LITERAL`, the token boundary, the baseline
  shape, the eight-code vocabulary, or the scope rule.
- It does not re-open the archived rulings. Ruling 2 survives in its exact
  form: a declared adoption never suppresses a hand-rolled screen.
