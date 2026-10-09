# platform-ui-token-distribution Specification Delta

## ADDED Requirements

### Requirement: The token kit SHALL adopt the approved brand typography and SHALL record every rejected guideline value as an intentional deviation

The declared value of the kit's font-family token SHALL be the approved brand stack
`Outfit, system-ui, sans-serif`, retaining the system fallback so a consumer that does not
load the font renders unchanged. Font loading (self-hosted `@font-face` or `next/font`)
SHALL remain the consumer's responsibility and the kit SHALL NOT ship a font binary or
gain a network dependency. Guideline values the kit rejects — a 14px base size, a 2–10px
spacing scale, and an oklch color representation — SHALL be recorded as documented
deviations with their reasons, not silently diverged.

#### Scenario: The font-family token is emitted

- **WHEN** the generator writes the token artifacts from `ui/tokens/tokens.json`
- **THEN** `--font-sans` resolves to `Outfit, system-ui, sans-serif`
- **AND** the CSS, TypeScript and Tailwind outputs all reference that one declaration

#### Scenario: A consumer loads no Outfit font file

- **WHEN** a consumer installs the kit without self-hosting or bundling Outfit
- **THEN** text renders through the `system-ui` fallback
- **AND** no artifact requests a remote font

#### Scenario: A rejected guideline value reappears undocumented

- **WHEN** a change alters the base font size, the space scale, or the color notation
  away from the recorded deviations
- **THEN** the change is refused unless its design record re-states the deviation and
  its reason

### Requirement: Every text-carrying and non-text token pair SHALL meet WCAG 2.2 AA and each measurement SHALL be recorded

Each semantic color pair that carries text (foreground and muted-foreground on
background, surface and muted; on-primary on primary and primary-hover; and the danger,
success and warning text roles on their surfaces) SHALL measure at least 4.5:1 in both
light and dark themes, and the focus indicator and control border SHALL measure at least
3:1 against their surfaces. The audit SHALL be recorded with its measured ratios; where a
pair fails, the minimally adjusted primitive SHALL keep the recorded palette identity.

#### Scenario: A text pair fails AA at audit time

- **WHEN** the contrast audit measures a text-carrying pair below 4.5:1
- **THEN** the primitive behind it is adjusted minimally and the new ratio is recorded
- **AND** the change re-baselines the emitted contract rather than hiding the delta

#### Scenario: The audit passes without adjustment

- **WHEN** every measured pair already meets its threshold
- **THEN** no primitive value changes
- **AND** the recorded ratios are the evidence that the palette is compliant

#### Scenario: Status text on a muted chip

- **WHEN** danger, success or warning text is drawn on the muted surface in either theme
- **THEN** the measured ratio is at least 4.5:1
- **AND** the tightest recorded margins are noted in the token descriptions so future
  shade changes are measured, not guessed

### Requirement: The kit SHALL expose a documented, always-visible focus indicator under a name that consumers can cite

The generated CSS SHALL define `--color-focus-ring` in addition to the legacy
`--focus-ring` it is baselined to, SHALL document the focus indicator as a non-negotiable
accessibility rule, and the alias SHALL resolve through the legacy token so it follows
every theme override without a second declaration.

#### Scenario: A consumer styles its focus indicator through the documented name

- **WHEN** a consumer uses `var(--color-focus-ring)` for its `:focus-visible` outline
- **THEN** the value resolves to the theme's focus ring in light and dark
- **AND** verification fails if either name is absent from the generated CSS

#### Scenario: The dark theme re-points the focus ring

- **WHEN** the dark theme override re-points the semantic focus-ring token
- **THEN** `--color-focus-ring` resolves to the overridden shade
- **AND** the pair still meets the non-text contrast threshold

### Requirement: The generator SHALL emit Tailwind-compatible outputs from the same token source and the feed SHALL ship them

The generator SHALL additionally write a Tailwind v3 preset and a Tailwind v4 `@theme`
block to `dist/tailwind/` from `ui/tokens/tokens.json` alone, mapping every semantic
color, the font family, the space scale and the radius scale to the kit's CSS custom
properties. Both outputs SHALL contain no color literal, SHALL be listed as required
artifacts, SHALL ship inside the design-token tarball, and verification SHALL fail with a
named diagnostic when a semantic token is missing from either output or a literal appears
in either.

#### Scenario: A Tailwind v3 consumer uses the preset

- **WHEN** a consumer imports `@platform/design-tokens/dist/tailwind/preset.js` into its
  Tailwind v3 configuration
- **THEN** its color, font, spacing and radius utilities resolve to `var(--…)` references
  of the kit tokens and follow the dark theme at runtime

#### Scenario: A Tailwind v4 consumer imports the theme block

- **WHEN** a consumer imports `dist/tailwind/theme.css` after `dist/tokens.css`
- **THEN** the `@theme` block is declared `inline` so Tailwind does not re-emit competing
  custom properties and `tokens.css` remains the single source per theme mode

#### Scenario: A literal leaks into a Tailwind output

- **WHEN** either Tailwind artifact contains a hex or `rgb()` color literal, or is missing
  a semantic color token mapping
- **THEN** verification fails with the named `ui-tailwind-incomplete` diagnostic
- **AND** the self-test proves the diagnostic fires

### Requirement: A deliberate token-value contract change SHALL move the whole kit version and re-baseline the V6 record

Because the packer records one kit version across the feed, a contract change to any
package SHALL move all five package versions, the feed tarball names, `feed.json`
digests, the fixture consumers, the distribution test fixtures, the architecture guards
and the emitted-token baseline together. The baseline SHALL re-baseline exactly the
entries this change's design record authorizes; any further value drift SHALL still fail
`ui-token-renamed`.

#### Scenario: The kit moves to the new version

- **WHEN** the feed is repacked after an authorized contract change
- **THEN** `feed.json` records kit version `0.2.0`, every package at `0.2.0`, matching
  tarball filenames and fresh digests
- **AND** the conformance fixture and distribution tests resolve against the new bytes

#### Scenario: An unauthorized value change rides along

- **WHEN** an emitted token value changes that the change record did not authorize
- **THEN** verification fails with `ui-token-renamed` naming the token
- **AND** repacking or re-baselining cannot make the failure pass
