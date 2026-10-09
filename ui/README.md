# Platform UI: tokens, the feed, and conformance

## React application templates

`@platform/react-templates` provides three distinct starting layouts: `PublicPageLayout`
for content-first public routes, `UserDashboardLayout` for signed-in user tasks, and
`AdminDashboardLayout` for permission-aware admin navigation. It also provides
`PlatformThemeProvider`, which maps generated light/dark semantic tokens to Ant Design,
and `DataTable` for the complex data display that is not covered by the base primitives.
Dashboard layouts move focus to the page heading when `currentPath` changes. `DataTable`
accepts caller-provided loading, empty, error, and forbidden messages; the retry button only
appears when an `onRetry` callback is supplied.

The platform keeps one implementation of base controls. Use `@platform/react-ui` for
`Button`, `Field`, `Card`, `Badge`, and `State`; use Ant Design through the template
package for data tables and advanced controls. Do not add a second button/input/card
layer or write page-specific palette values.

Consumers must vendor the `@platform/react-templates` tarball and its platform feed
dependencies as described below, and must install a compatible Ant Design peer from
their package source (`antd >=5 <7`). Import the design-token CSS and template CSS once
at the application entry point, along with the React UI primitive stylesheet, then wrap
the application with `PlatformThemeProvider`.
Ant Design itself is a peer and is not copied into the platform tarball.

Example consumer setup (use the exact tarball filenames and digests from the feed):

```json
{
  "dependencies": {
    "@platform/design-tokens": "file:vendor/platform-ui/platform-design-tokens-0.2.0.tgz",
    "@platform/react-ui": "file:vendor/platform-ui/platform-react-ui-0.2.0.tgz",
    "@platform/react-shell": "file:vendor/platform-ui/platform-react-shell-0.2.0.tgz",
    "@platform/react-templates": "file:vendor/platform-ui/platform-react-templates-0.2.0.tgz",
    "antd": "^6.0.0",
    "@types/react": "^18.0.0"
  }
}
```

```tsx
import '@platform/design-tokens/dist/tokens.css';
import '@platform/react-ui/styles.css';
import '@platform/react-templates/styles.css';
import { AdminDashboardLayout, DataTable, PlatformThemeProvider } from '@platform/react-templates';

export function AccountsPage() {
  return (
    <PlatformThemeProvider mode="light">
      <AdminDashboardLayout
        brand="Example"
        title="Accounts"
        user={{ authenticated: true, displayName: 'Admin', permissions: ['accounts.read'] }}
        navigation={[{ id: 'accounts', label: 'Accounts', href: '/admin/accounts', permission: 'accounts.read' }]}
        currentPath="/admin/accounts"
      >
        <DataTable
          label="Accounts"
          rowKey="id"
          rows={[]}
          columns={[{ title: 'Name', dataIndex: 'name', key: 'name' }]}
          emptyMessage="No accounts have been created yet."
        />
      </AdminDashboardLayout>
    </PlatformThemeProvider>
  );
}
```

Use `PublicPageLayout` for visitor-facing content and `UserDashboardLayout` for signed-in
customer tasks. Their props are explicit so an agent does not need to invent a navigation
model or infer authorization from the route. `DataTable` rows and columns are typed by
Ant Design; provide a stable `rowKey` for the consumer's domain record.

AI agents should choose a layout by audience, supply explicit headings/navigation and
loading/empty/error/recovery states, and keep domain data and authorization in the
consumer. Public pages should prioritize content and one primary action; user dashboards
should prioritize the user's current tasks; admin dashboards may use denser tables and
filters. Keep wide tables inside the labeled `DataTable` scroll region. Give every action
a visible text label, use semantic state text as well as color, provide text/table
alternatives for charts, and do not add gradients, one-off palette literals, deeply
nested cards, or hover-only behavior by default.

Before adopting a template, review it at 320, 375, 768, 1024, and 1280 CSS pixels; use
keyboard-only navigation; check the accessibility tree and focus visibility; and review
light/dark mode and reduced motion. Permission-filtered navigation is presentation only:
the application's backend must authorize every protected operation.

This directory is the platform's design-token layer: the single token source, the
generator that turns it into shipped artifacts, the committed feed that makes those
artifacts resolvable from another repository, and the frozen baseline the
workspace conformance check measures against.

Layout:

| Path | What it is |
| --- | --- |
| `tokens/tokens.json` | **Every** design-token value, in DTCG form, light and dark. Single authoritative source. |
| `tokens/emitted-baseline.json` | The name/value set the 0.1.0 generator emitted, captured before the tree walk; re-baselined once — exactly `--font-sans` — by `ui-design-guideline-adoption` (the kit moved to 0.2.0 for it). The additive-compatibility invariant. |
| `scripts/lib/token-model.mjs` | The DTCG walk, naming rule and alias resolver, shared by the generator and the verifier so they cannot disagree. |
| `scripts/generate-tokens.mjs` | Walks the tree and writes `packages/design-tokens/dist/tokens.css`, `tokens.ts`, and the Tailwind projections `dist/tailwind/preset.js` (v3) and `dist/tailwind/theme.css` (v4 `@theme inline`). |
| `scripts/verify-ui.mjs` | The oracle: completeness, no orphan literal, alias resolution, canonical primitive, literal-free Tailwind outputs (`ui-tailwind-incomplete`), and every accessibility guarantee the layer already asserted. |
| `scripts/selftest-verify-ui.mjs` | Proves each of those named diagnostics actually fires, on throwaway copies of the tree. |
| `scripts/pack-ui.mjs` | Deterministic `npm pack` equivalent: `feed/*.tgz` plus `feed/feed.json`. |
| `scripts/verify-ui-feed.mjs` | Feed ⇄ consumer-declaration agreement, including the three path refusals. |
| `feed/` | Committed tarball bytes and the digest manifest. Committed on purpose; see below. |
| `legacy-divergent.json` | The frozen legacy baseline (15 repositories, 55 undeclared-UI repositories, 24 baselined breakpoint values). Data, not code. |
| `tests/conformance-fixtures/` | Fixture repositories used by `scripts/test-ui-distribution.sh`. |

## The token source is singular

`tokens/tokens.json` owns every value. The generator enumerates no token path and
hardcodes no value: adding `{ "color": { "teal": { "500": { "$value": "#14b8a6" } } } }`
emits `--color-teal-500` with no code change. The two defects that made "one source"
false before this change are closed and are now checkable:

- **The generator hand-enumerated tokens.** It read specific paths inside a template
  literal, so a token added to the source produced no output *and no error*.
  `verify-ui.mjs` now fails with `ui-token-dropped`, naming the source path.
- **The dark theme was a second literal palette.** `[data-theme="dark"]` hard-coded
  five hexes (`#93c5fd`, `#bfdbfe`, `#fca5a5`, `#86efac`, `#fcd34d`) that appeared in
  no token file. They are now `color.blue.300`, `color.blue.200`, `color.red.300`,
  `color.green.300` and `color.amber.300` in the source, and the theme's override map
  lives in `theme.dark.color`. Any literal in the generated CSS that is not reachable
  from a `$value` fails with `ui-orphan-literal`, naming the declaration and its line.

### Naming

`--` + the source path, kebab-cased: `color.slate.50` → `--color-slate-50`,
`space.4` → `--space-4`, `semantic.color.mutedForeground` → `--color-muted-foreground`,
`component.button.minHeight` → `--component-button-min-height`. One documented exception
is preserved verbatim from 0.1.0: `semantic.color.focusRing` → `--focus-ring`, because
`react-ui/styles.css` and `Platform.UI.Razor` consume that name and a rename is a contract
change, not a refactor. `ui-design-guideline-adoption` added the color-group bridge
`color.focusRing` → `--color-focus-ring`, declared as `{semantic.color.focusRing}`: it
emits as `var(--focus-ring)`, follows the dark-theme override through it, and is the name
the adopted accessibility rules cite for the always-visible focus indicator
(see [`docs/ui-tokens.md`](../docs/ui-tokens.md)).

`tokens/emitted-baseline.json` records all 38 names the 0.1.0 generator emitted with
their values. `verify-ui.mjs` fails with `ui-token-renamed` if any of them is missing or
changed. The rewrite is **additive**: it emits 50 names today, the 38 baselined ones
(`--font-sans` re-baselined to the Outfit stack by `ui-design-guideline-adoption`'s
contract record) plus 5 dark-capable shades, 4 component tokens, the
`--color-focus-ring` bridge, and the 2 semantic colors
(`semantic.color.onPrimary`, `semantic.color.border`) that
`platform-shared-auth-pages` added after measuring the auth pages and finding the
button label at 1.80:1 on the dark primary fill and the field edge at 1.48:1 on the
light surface. Neither could be expressed with an existing primitive in both modes, so
both are theme-override entries rather than a raw shade.

## The canonical visually-hidden primitive

`tokens.css` defines `.visually-hidden` in the full nine-property form, with no
`!important`:

```css
.visually-hidden {
  position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
  overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; border: 0;
}
```

It is emitted by the generator so it cannot drift, and `verify-ui.mjs` fails with
`ui-primitive-incomplete` if a property is missing or any declaration is `!important`,
and with `ui-duplicate-primitive` if a second definition appears in any other generated
artifact. It exists because the shared layer never shipped the primitive: 15 repositories
wrote their own. One of them, `arivio`, omits `padding`, `margin` and `border`, so a
screen-reader-only element still occupies a 1×1px box — a measured accessibility defect,
not a style disagreement. See the frozen list below.

## The feed: how a consumer outside this repository resolves the layer

```
ui/feed/
  platform-design-tokens-0.2.0.tgz
  platform-react-ui-0.2.0.tgz
  platform-react-shell-0.2.0.tgz
  platform-react-auth-0.2.0.tgz
  feed.json          # kit version, per-package name/version/sha256/members
```

A consumer commits those bytes into its own tree and declares a `file:` specifier:

```jsonc
// package.json
"dependencies": {
  "@platform/design-tokens": "file:vendor/platform-ui/platform-design-tokens-0.2.0.tgz"
}
```

```jsonc
// ui-conformance.json, at the consumer repository root
{
  "schema": 1,
  "state": "consumes-feed",     // or "exempt" with reason + owner + reviewBy
  "kitVersion": "0.2.0",
  "feed": {
    "path": "vendor/platform-ui",
    "packages": [{ "name": "@platform/design-tokens", "version": "0.2.0", "sha256": "<64 hex>" }]
  }
}
```

There is no registry, no `npm publish` and no credential. The bytes are committed, the
digest is declared, and the check compares the two.

### Three refusals, stated as rules

These are rules, not folklore. `ui/scripts/verify-ui-feed.mjs` fails each with
`ui-feed-path-invalid`, naming the specifier, **before anything is resolved or installed**:

1. **An absolute path.** The tree stops resolving when it is relocated or cloned
   somewhere else. Use a repository-relative path.
2. **A `..`-escaping path.** A specifier that leaves the consumer repository is only
   resolvable next to a sibling checkout of this repository — the exact dependency this
   mechanism exists to remove.
3. **A path assembled from an environment variable.** A variable a CI runner does not
   carry fails identically to a hard-coded path, so it is the same failure class and gets
   the same diagnostic, naming the variable.

The same three were refused for the .NET feed in `forge`'s
`scaffold-prewires-shared-layer`, which also refused an env-var feed
(`NUGET_PLATFORM_FEED`) for the same reason.

### Why a tarball and not a directory

npm has no file-backed *source* concept: its unit of resolution is a specifier string in
`package.json`. A `file:` specifier over a **directory** is satisfied by npm with a
symlink, which makes the vendored copy ordinary editable source — it can drift with no
manifest change and no digest to compare, and it makes the consumer a second silent
editor of the shared layer. A **tarball** is a checksummed byte artifact, like the
committed `.nupkg` precedent, so "these bytes are the kit's bytes" is checkable.

### Determinism

`pack-ui.mjs` normalises the three things that would otherwise leak a build clock or a
checkout user into the bytes: members are sorted by tar path, mtime is fixed at epoch 0
with uid/gid 0, empty uname/gname and mode 0644, and the archive is a single gzip stream
at a fixed level. Two runs produce byte-identical tarballs; prove it with
`node ui/scripts/pack-ui.mjs && sha256sum ui/feed/*.tgz` twice.

## Version policy (D15, moved by ui-design-guideline-adoption)

The kit is at **0.2.0**. `ui-shared-layer-reachability` stayed at 0.1.0 because its work
was additive — D15: the digest, not the version, is the load-bearing check, and a bump
would signal a contract change that did not happen. `ui-design-guideline-adoption` *is*
such a signal: it re-baselined `--font-sans` to the adopted brand stack (a value change
for byte-pinners), added `--color-focus-ring`, and shipped two new generated artifacts.
Because `pack-ui.mjs` records exactly one version across the feed, all five packages,
the tarball names and `feed.json` moved together. Consumers still pin the tarball **and**
its digest, so bytes that change without a version bump are caught by
`ui-feed-digest-mismatch` rather than absorbed.

## Conformance: absence, not presence

`scripts/check-ui-conformance.sh` scans the workspace and fails a repository that
**declares a shared primitive itself**. A repository is conformant by either consuming
the feed or declaring an exemption, and a repository that ships no user interface passes
without adopting anything. Named diagnostics:

| Diagnostic | Condition |
| --- | --- |
| `ui-divergent-primitive` | A repository outside the frozen list declares `.visually-hidden` / `.sr-only` itself. Names repository, file, line and the canonical rule. |
| `ui-conformance-undeclared` | A UI-bearing repository ships a user interface and declares nothing. |
| `ui-breakpoint-undeclared` | A width-based media query value that is neither the documented kit scale (`30rem`, `36rem`, `48rem`, `64rem`, `80rem`) nor part of the frozen baseline. |
| `ui-legacy-list-grew` | The frozen baseline grew. Raised by the architecture guard; the list is closed by construction. |
| `ui-feed-*` | Anything `verify-ui-feed.mjs` reports for a `consumes-feed` repository. |

The check is **read-only** over consumer repositories: it reports, it never writes,
never installs and never reformats. Run it with `scripts/test-ui-distribution.sh`, which
also proves each failure path on fixtures and proves the portability claim with a real
`npm install` at an unrelated path, offline, with the registry pointed at an unreachable
address.

### The frozen legacy list (D16, D17)

`legacy-divergent.json` is a **data file**, not a script constant, so it is reviewable as
a diff and no code path can extend it. It records:

- `divergentPrimitive` — the 15 repositories that declare a shared visually-hidden
  primitive today, with file, line, selector, the specific divergence, and a migration
  order. `arivio` is order 1 because its copy is a measured accessibility defect; the
  other fourteen may wait for their own schedule. Follow-up: `ui-fleet-adoption`.
- `undeclaredUiRepos` — the 55 repositories that ship a user interface and declare
  nothing, so the check can start from a truthful baseline and fail the *next* one.
- `flutterNotCoveredRepos` — the 8 Dart repositories. Not covered, never failed: the
  platform ships no Dart artifact (NG5).
- `mediaQueryBaselinedValues` — the 24 width values present today, including the
  off-by-one pairs `640px`/`641px` and `768px`/`769px`, which are reported as known
  divergence and do not fail.

The list can only shrink. A repository that migrates leaves it. There is no flag,
comment or argument anywhere that adds an entry, and
`tests/Platform.Architecture.Tests/UiDistributionTests.cs` fails if the list grows.

## What runs in the gate (D18)

`scripts/quality-gate.sh` runs `node ui/scripts/verify-ui.mjs` and
`node ui/scripts/selftest-verify-ui.mjs` alongside `dotnet` and `openspec`. This
overrides the earlier recommendation in this change's own history, which left the wiring
to the owner. The reason is that a verifier no gate runs does not gate: the repo gate was
`dotnet` + `openspec` + `git diff --check`, so without this a token regression would not
block it. The diagnosed example of that failure class in this workspace is
`forge/scripts/contract-parity.sh`, which performs zero comparisons and still prints
`contract-parity: OK`.

## Known boundaries

- **Razor** consumers use the same committed feed: commit `platform-tokens.css` into the
  consumer's web assets. `Platform.UI.Razor` already exists at 0.1.0 and needs no new
  delivery vehicle.
- **`Platform.UI.Razor`'s own `platform-tokens.css` is hand-authored**, not generated.
  `ui-design-guideline-adoption` value-synced it (Outfit stack, the `--color-focus-ring`
  alias; the palette itself was unchanged by the contrast audit), but consolidating it
  into a generated artifact is still out of scope: it is a .NET static asset, not a
  generated artifact, and its package version and manifest entry stay untouched.
- **`scripts/check-public-api.sh` does not cover `Platform.UI.Razor`** (D14) and
  `eng/public-api-baseline.txt` was not regenerated. The gap is recorded, not closed: a
  regenerated baseline could silently absorb a breaking change in a package this change
  does not touch. `UiDistributionTests` fails if that gap is ever closed without
  regenerating the baseline in the same change.
- **No consumer repository is migrated by this change.** This directory makes the layer
  resolvable and checkable; adopting it is each repository's own change.
