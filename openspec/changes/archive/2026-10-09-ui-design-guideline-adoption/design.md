# Design: adopt root DESIGN.md into the token kit, Tailwind outputs, 0.2.0 kit

## D-1  Font adoption is a value change, not a new tier

`tokens.json` has no separate `semantic.font.*` tier: `font.sans` **is** the kit's
font-family contract (`--font-sans`, consumed by `Platform.UI.Razor`, react-templates and
the Tailwind outputs). DTCG purity would add a semantic alias group for one member; the
adopted decision is to change the declared value of `font.sans` to
`Outfit, system-ui, sans-serif`. The system-ui fallback stays, so a consumer that does not
load Outfit renders exactly as before — the change is additive at render time and only a
contract change for byte-pinners. **Loading Outfit is the consumer's job** (self-hosted
`@font-face` or `next/font`); the kit ships no font binary and gains no network dependency
(feed design NG6 unchanged).

## D-2  Rejected DESIGN.md values are recorded deviations

DESIGN.md's `font.size.base=14px` and `space.1..8 = 2px..10px` are rejected: 14px body and
a 2–10px step are too cramped for the Razor/admin surfaces this kit serves, and the kit's
1rem base plus 0.25rem-stepped scale is already consumed by three shipped packages. The
deviation is documented in `docs/ui-tokens.md` and cited from the canonical spec so the
divergence from the guideline is a decision, not drift. Likewise the guideline's oklch
values (`oklch(0.145 0 0)` etc.) are near-neutral and already covered by the slate scale;
adopting oklch would re-cut every primitive for no measured gain and is refused
(palette identity stays slate/blue, hex).

## D-3  Contrast audit: measured, recorded, no value changed

Method: WCAG 2.2 relative luminance on the resolved light and dark hex values
(`srgb` → linear → L), pairs listed per text role and per non-text role (focus ring and
border at 3:1 per SC 1.4.11, text at 4.5:1 per SC 1.4.3). All 38 measured pairs pass
before any adjustment, so **no primitive hex is touched**:

Light (background `#f8fafc`, surface `#ffffff`, muted `#f1f5f9`):

| Pair | Ratio | Need | Result |
| --- | --- | --- | --- |
| foreground `#0f172a` on background / surface / muted | 17.06 / 17.85 / 16.30 | 4.5 | pass |
| muted-foreground `#334155` on background / surface / muted | 9.90 / 10.35 / 9.45 | 4.5 | pass |
| on-primary `#ffffff` on primary `#1d4ed8` / primary-hover `#1e40af` | 6.70 / 8.72 | 4.5 | pass |
| danger `#b91c1c` on surface / background / muted | 6.47 / 6.18 / 5.91 | 4.5 | pass |
| success `#15803d` on surface / background / muted | 5.02 / 4.79 / 4.58 | 4.5 | pass |
| warning `#b45309` on surface / background / muted | 5.02 / 4.80 / 4.58 | 4.5 | pass |
| focus ring `#1d4ed8` on background | 6.41 | 3.0 | pass |
| border `#64748b` on background | 4.55 | 3.0 | pass |

Dark (background `#0f172a`, surface/muted `#334155`):

| Pair | Ratio | Need | Result |
| --- | --- | --- | --- |
| foreground `#f8fafc` on background / surface | 17.06 / 9.90 | 4.5 | pass |
| muted-foreground `#f1f5f9` on background / surface | 16.30 / 9.45 | 4.5 | pass |
| on-primary `#0f172a` on primary `#93c5fd` / primary-hover `#bfdbfe` | 9.90 / 12.56 | 4.5 | pass |
| danger `#fca5a5` on surface / background | 5.46 / 9.41 | 4.5 | pass |
| success `#86efac` on surface / background | 7.37 / 12.71 | 4.5 | pass |
| warning `#fcd34d` on surface / background | 7.18 / 12.38 | 4.5 | pass |
| focus ring `#bfdbfe` on background | 12.56 | 3.0 | pass |
| border `#cbd5e1` on background | 12.02 | 3.0 | pass |

The measurements are repeated in `docs/ui-tokens.md` (the consumer-facing record) and the
tightest margins (success/warning on muted at 4.58:1) are noted in the primitive token
`$description`s so a future shade change is measured, not guessed.

## D-4  `--color-focus-ring` is a documented alias, `--focus-ring` stays the base name

DESIGN.md makes a visible focus indicator a non-negotiable and requires a named token.
The baseline pins `semantic.color.focusRing` → `--focus-ring` (legacy name exception,
consumed by `react-ui/styles.css` and `Platform.UI.Razor`); renaming it is a contract
change this adoption does not need. Instead the color group gains one bridge token
`color.focusRing` → `--color-focus-ring`, declared as `{semantic.color.focusRing}`. It
emits as `var(--focus-ring)` and therefore follows the dark-theme override for free
(blue-200 `#bfdbfe`, 12.56:1 on the dark background). Both names are listed in the
verifier's required-token set and documented in `docs/ui-tokens.md`.

## D-5  Tailwind outputs are pure projections of `tokens.json`

`generate-tokens.mjs` writes, beside `tokens.css`/`tokens.ts`:

- `dist/tailwind/preset.js` — Tailwind v3 preset, **ESM default export** (the package is
  `"type": "module"`; `tailwind.config.mjs`/`createRequire` consumers get the same
  object — a `.cjs` sidecar is refused as a second artifact to keep in sync). Maps
  `theme.extend.colors` from every semantic color token, `fontFamily.sans`, `spacing`
  from the space group, `borderRadius` from the radius group. Every value is
  `var(--…)` — the kit's custom properties hold hex, not channels, so the
  `rgb(var(…) / <alpha-value>)` pattern is not available without re-cutting every color
  token; opacity modifiers are a documented limitation of the v3 preset.
- `dist/tailwind/theme.css` — Tailwind v4 `@theme inline { … }` block mapping the same
  names (`--color-*`, `--font-sans`, `--spacing-*`, `--radius-*`) onto the kit's runtime
  custom properties. `inline` is load-bearing: without it Tailwind re-emits the `@theme`
  variables into `:root`, which would fight `tokens.css` (and the dark-theme override)
  for the same names; with it the utilities reference the kit variables directly and
  `tokens.css` remains the single source, in both themes.

Both artifacts stay literal-free: `verify-ui.mjs` fails `ui-tailwind-incomplete` if a
semantic token name is missing from either output or if a hex/`rgb()` literal appears in
either, and the self-test proves the diagnostic fires. They ship in the design-tokens
tarball automatically (`files: ["dist"]`).

## D-6  Version 0.2.0 for the whole kit, one reason

`pack-ui.mjs` refuses a feed whose packages disagree on version (one kit version, one
declaration). The font change is a value change to a baselined contract, and two new
generated artifacts ship — under D15's own logic ("a version number cannot distinguish
today's bytes from drifted bytes; the digest is load-bearing") the digest check still
gates, but 0.1.0-pinned consumers must see a version that says "token bytes changed on
purpose". All five packages, the feed, the fixture consumers, `test-ui-distribution.sh`
(which previously faked a mismatch with the string `0.2.0` — it now fakes `0.3.0`) and
`UiDistributionTests` move to 0.2.0 together.

## D-7  The V6 baseline re-baselines exactly one entry

`emitted-baseline.json` records that a *changed* entry is a contract change. This change
is that contract change: `--font-sans` is re-baselined to
`"Outfit, system-ui, sans-serif"` and the file's `$description` names this change as the
record. No other entry moves (D-3 measured zero failures). The verifier logic is
untouched — a second silent value change would still fail `ui-token-renamed`.

## D-8  Razor asset stays hand-authored but value-synced

`Platform.UI.Razor/wwwroot/platform-tokens.css` is documented as hand-authored and is not
generated by this change (consolidation stays a recorded boundary). Leaving it on the old
`system-ui` contract while the kit ships Outfit would re-open the "two palettes" defect
class for typography, so its literals are synced to the adopted values and the
`--color-focus-ring` alias is added. The manifest, `check-public-api.sh` scope (D14 gap)
and the Razor package version stay untouched, as `UiDistributionTests` continues to
assert.
