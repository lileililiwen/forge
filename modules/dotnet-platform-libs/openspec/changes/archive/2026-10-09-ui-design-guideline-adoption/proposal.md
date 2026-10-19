# Proposal: Adopt the root DESIGN.md brand guideline into the UI token kit and ship Tailwind outputs

## Why

`/home/paul/code/DESIGN.md` ("Kira AI") is a brand/design guideline with WCAG 2.2 AA
targets, a typography stack, and non-negotiable accessibility rules that currently live
outside this repository with no binding to the shipped token kit. Meanwhile
`ui/tokens/tokens.json` is the single authoritative token source for
`@platform/design-tokens`, but its font contract is `system-ui, sans-serif` — the brand
font Outfit is reachable from no token — and its accessibility rules are scattered across
generator assertions with no documented state/contrast contract a consumer can cite.
The owner has authorized adopting DESIGN.md with deliberate merge decisions; this change
records those decisions in tokens, docs, and verification, and closes them.

The kit also has no Tailwind surface. Consumers that use Tailwind (v3 config-based or v4
CSS-based) hand-write a second mapping from the kit's CSS custom properties, which is
exactly the "second silent palette" the feed design exists to prevent. This change emits
`dist/tailwind/preset.js` (v3) and `dist/tailwind/theme.css` (v4 `@theme`) from the same
`tokens.json`, inside the same tarball.

## What changes

1. **Font.** `font.sans` becomes `Outfit, system-ui, sans-serif` (system-ui fallback
   kept). Self-hosting / `next/font` loading is the consumer's job and is documented.
2. **Base size and spacing kept.** DESIGN.md's 14px base and 2–10px spacing scale are
   rejected as too cramped; the kit keeps its 1rem base and existing space scale. This is
   recorded as an intentional deviation, not a silent divergence.
3. **Accessibility rules adopted.** WCAG 2.2 AA contrast targets, an always-visible focus
   indicator with a documented `--color-focus-ring` token name, and the seven component
   states (default, hover, focus-visible, active, disabled, loading, error) become kit
   rules in `docs/ui-tokens.md` and in the canonical specs.
4. **Contrast audit.** Every text-carrying semantic pair (foreground/muted-foreground on
   background/surface/muted, onPrimary on primary/primaryHover, danger/success/warning
   text roles, focus ring and border at 3:1) is measured in light and dark against
   AA 4.5:1 (3:1 for non-text). All 38 pairs pass with the current slate/blue palette, so
   **no primitive hex is changed**; each measurement is recorded in `docs/ui-tokens.md`
   and the token `$description`s. Palette identity (slate/blue) is kept.
5. **Tailwind outputs.** The generator additionally writes
   `packages/design-tokens/dist/tailwind/preset.js` and
   `packages/design-tokens/dist/tailwind/theme.css`, both derived only from
   `tokens.json`, both CSS-variable-backed (no literals), shipped in the tarball, and
   gated by a new `ui-tailwind-incomplete` diagnostic in `verify-ui.mjs` plus a
   self-test case.
6. **Kit version 0.1.0 → 0.2.0.** The font value is a breaking change for consumers who
   pin token bytes, and `pack-ui.mjs` enforces one version across the kit, so all five
   packages, the feed tarballs, `feed.json`, the fixtures, and
   `UiDistributionTests` move to 0.2.0. The V6 baseline
   (`ui/tokens/emitted-baseline.json`) is re-baselined for the single changed value
   (`--font-sans`); this OpenSpec change is the contract-change record the baseline
   requires.

## Impact on public APIs

- npm kit: contract change (one token value, two new generated artifacts, five package
  versions 0.1.0 → 0.2.0, feed digests change).
- .NET: no package added, no manifest change, no public .NET type change.
  `Platform.UI.Razor`'s hand-authored `platform-tokens.css` is value-synced (font + the
  documented `--color-focus-ring` name); consolidating it into a generated artifact
  remains out of scope.

## Non-goals

- No switch to DESIGN.md's oklch palette or its 14px / 2–10px scales (rejected above).
- No registry, no `npm publish`, no credential (unchanged from NG6).
- No consumer-repository or forge/kits mirror resync; that is separate work in other
  repositories.
- No Dart/Flutter artifact (unchanged from NG5).
- No new `.visually-hidden` definition or renaming of the legacy `--focus-ring` name.
