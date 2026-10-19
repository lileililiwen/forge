# UI Token Kit — brand adoption, accessibility rules, and Tailwind outputs

Source of truth: [`ui/tokens/tokens.json`](../ui/tokens/tokens.json) (DTCG 3-tier:
primitive → semantic → component, plus the `theme.dark` override). Generated artifacts:
`ui/packages/design-tokens/dist/{tokens.css,tokens.ts,tailwind/preset.js,tailwind/theme.css}`,
shipped as the `@platform/design-tokens` tarball in `ui/feed/` (kit version **0.2.0**).
Adopted from the owner-approved root brand guideline `DESIGN.md` ("Kira AI") by the
archived OpenSpec change `ui-design-guideline-adoption`; the merge decisions, including
the values that were **rejected**, are recorded there and repeated below.

## Accessibility rules (non-negotiables adopted from the guideline)

These are kit rules, not suggestions. Components built on this kit must satisfy them.

1. **Contrast target: WCAG 2.2 AA.** Text pairs must measure ≥ 4.5:1 (≥ 3:1 for text at
   ≥ 24px, or ≥ 18.66px bold); non-text indicators — the focus ring and control borders —
   must measure ≥ 3:1 (SC 1.4.11). Every shipped pair is measured below; a new pair is
   measured before it merges.
2. **A always-visible focus indicator.** `:focus-visible` must never be suppressed
   without a replacement. Cite the documented token `var(--color-focus-ring)` (it
   aliases the baselined `--focus-ring` and follows the dark-theme override). The kit
   exposes the canonical `.visually-hidden` primitive so focus text can be hidden
   visually but not from the accessibility tree.
3. **Component state set.** Every component and every component guidance defines, at
   minimum: **default, hover, focus-visible, active, disabled, loading, error**. The
   loading and error states carry text or ARIA status (`aria-busy`, `aria-invalid`,
   `aria-live`), never color alone. `@platform/react-ui` primitives ship these states;
   guidance lives in `ui/README.md` and the canonical `platform-ui-design-system` spec.
4. **Reduced motion.** `tokens.css` emits the `prefers-reduced-motion: reduce` fallback;
   consumers must not remove it.
5. **Semantic tokens only.** Components consume `var(--…)` names; a raw hex/`rgb()` value
   in a generated or packed artifact fails verification (`ui-orphan-literal`,
   `ui-tailwind-incomplete`).

## Typography: what was adopted, what was rejected

- **Adopted:** Outfit as the primary family. `--font-sans` resolves to
  `Outfit, system-ui, sans-serif`. **Loading the font is the consumer's job** —
  self-hosted `@font-face` or `next/font`; the kit ships no font binary and requests no
  network. Without the file, text renders through `system-ui` exactly as before.
- **Rejected (recorded deviations):** the guideline's `14px` base size and its
  `2px–10px` spacing scale — too cramped for the Razor/admin surfaces this kit serves;
  the kit keeps its `1rem` base and the `0.25rem`-stepped space scale. The guideline's
  oklch color notation is also rejected: its values are near-neutral and already covered
  by the slate scale, so the slate/blue palette identity and hex notation stay.

## Contrast audit (measured 2026-10-09)

Method: WCAG 2.2 relative luminance on the resolved hex values; 4.5:1 for text roles,
3:1 for non-text. **All 38 pairs passed before any adjustment, so no primitive hex was
changed** — the table is the evidence, and the tightest margins are annotated in the
primitive `$description`s.

Light theme (background `#f8fafc`, surface `#ffffff`, muted `#f1f5f9`):

| Pair | Ratio | Need | Result |
| --- | --- | --- | --- |
| foreground `#0f172a` on background | 17.06:1 | 4.5:1 | pass |
| foreground on surface | 17.85:1 | 4.5:1 | pass |
| foreground on muted | 16.30:1 | 4.5:1 | pass |
| muted-foreground `#334155` on background | 9.90:1 | 4.5:1 | pass |
| muted-foreground on surface | 10.35:1 | 4.5:1 | pass |
| muted-foreground on muted | 9.45:1 | 4.5:1 | pass |
| on-primary `#ffffff` on primary `#1d4ed8` | 6.70:1 | 4.5:1 | pass |
| on-primary on primary-hover `#1e40af` | 8.72:1 | 4.5:1 | pass |
| danger `#b91c1c` on surface / background / muted | 6.47 / 6.18 / 5.91 | 4.5:1 | pass |
| success `#15803d` on surface / background / muted | 5.02 / 4.79 / 4.58 | 4.5:1 | pass |
| warning `#b45309` on surface / background / muted | 5.02 / 4.80 / 4.58 | 4.5:1 | pass |
| focus ring `#1d4ed8` (`--color-focus-ring`) on background | 6.41:1 | 3:1 | pass |
| border `#64748b` on background | 4.55:1 | 3:1 | pass |

Dark theme (background `#0f172a`, surface/muted `#334155`):

| Pair | Ratio | Need | Result |
| --- | --- | --- | --- |
| foreground `#f8fafc` on background | 17.06:1 | 4.5:1 | pass |
| foreground on surface / muted | 9.90:1 | 4.5:1 | pass |
| muted-foreground `#f1f5f9` on background | 16.30:1 | 4.5:1 | pass |
| muted-foreground on surface / muted | 9.45:1 | 4.5:1 | pass |
| on-primary `#0f172a` on primary `#93c5fd` | 9.90:1 | 4.5:1 | pass |
| on-primary on primary-hover `#bfdbfe` | 12.56:1 | 4.5:1 | pass |
| danger `#fca5a5` on surface / background | 5.46 / 9.41 | 4.5:1 | pass |
| success `#86efac` on surface / background | 7.37 / 12.71 | 4.5:1 | pass |
| warning `#fcd34d` on surface / background | 7.18 / 12.38 | 4.5:1 | pass |
| focus ring `#bfdbfe` (`--color-focus-ring`) on background | 12.56:1 | 3:1 | pass |
| border `#cbd5e1` on background | 12.02:1 | 3:1 | pass |

## Tailwind outputs

Both files are generated from `tokens.json` and ship in `@platform/design-tokens`:

**Tailwind v3 — `dist/tailwind/preset.js`** (ESM default export; the package is
`"type": "module"`, so import it from a `tailwind.config.mjs`):

```js
import preset from '@platform/design-tokens/dist/tailwind/preset.js';

export default { presets: [preset] };
```

**Tailwind v4 — `dist/tailwind/theme.css`**, imported after the token CSS:

```css
@import '@platform/design-tokens/dist/tokens.css';
@import '@platform/design-tokens/dist/tailwind/theme.css';
```

The block is `@theme inline` on purpose: `inline` stops Tailwind from re-emitting the
theme variables into `:root`, where they would compete with `tokens.css` and its
`[data-theme="dark"]` override for the same names. Either way, utilities resolve
through the kit custom properties and follow the theme at runtime.

Mapped surfaces: `colors` from every semantic color (`bg-primary`, `text-muted-foreground`,
`border-danger`, …), `fontFamily.sans`, `spacing` (`p-1` … `p-8`), and `borderRadius`
(`rounded-sm|md|lg|full`).

**Limitation (v3 preset):** the kit's color custom properties hold hex, not channel
triples, so the `rgb(var(--x) / <alpha-value>)` pattern is unavailable and kit colors do
not take Tailwind opacity modifiers (`bg-primary/50`). Use a solid token, or
`color-mix()` in consumer CSS — the kit deliberately did not re-cut every color token
into channels for it.

## Verification

`node ui/scripts/verify-ui.mjs` is the oracle: it fails on a dropped or renamed baselined
token, an orphan literal, a missing focus-ring name, and — since this adoption — an
incomplete or literal-carrying Tailwind output (`ui-tailwind-incomplete`).
`node ui/scripts/selftest-verify-ui.mjs` proves each diagnostic fires. The kit version,
feed digests and the `dist/tailwind/*` tarball members are guarded by
`tests/Platform.Architecture.Tests/UiDistributionTests.cs`.
