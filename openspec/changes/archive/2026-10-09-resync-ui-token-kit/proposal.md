# Proposal: Resync the vendored UI token kit mirror to kit 0.2.0

## Why

The `platform-ui-web` kit moved to `0.2.0` in `dotnet-platform-libs` at
`6bf3946fcb1a9d2294b13d0af4347f6cf0d8e566` (`ui-design-guideline-adoption`),
while Forge's vendored mirror in `kits/` still carries the bytes, manifest
records, compiled-in descriptor digests and version constant of `0.1.0` at
source revision `c740bd9d625f004978f058578e61ffcdf14bf8c2` (synced
2026-10-02). Measured drift on `main`:

- `kits/tokens/tokens.css` is at sha256 `720d0bc7…`, the kit now emits
  `7ca6b795…`: the 0.2.0 token source adds the blue/red/green/amber
  dark-mode primitives, `--color-on-primary`, `--color-border`,
  `--color-focus-ring`, component-layer tokens
  (`--component-button-*`, `--component-control-*`), the `Outfit` font stack,
  an expanded dark-theme block and the canonical `.visually-hidden` primitive.
- `kits/tokens/tokens.ts` is at `98dfd642…`, the kit now emits `7e0633d3…`:
  the 0.2.0 artifact adds the `tokenValues` name→value map,
  `semanticColorValues` light/dark theme maps and the
  `TokenName`/`SemanticColorName` types.
- The upstream verifier `ui/scripts/verify-ui.mjs` grew from 32 to 341 lines
  with the same older assertions (token presence, dark theme, reduced motion,
  the focus-ring name) plus tree-walk completeness; Forge's scoped vendored
  derivative `kits/scripts/verify-tokens.mjs` still quotes revision
  `c740bd9…` and asserts only the pre-0.2.0 evidence.
- `PLATFORM_UI_KIT_VERSION` is `"0.1.0"` while the vendored bytes would then
  claim a version they were not generated at — a pinned project would render,
  receipt and upgrade-plan against a version that does not describe its
  artifacts.

## What Changes

- Re-copy `kits/tokens/tokens.css` and `kits/tokens/tokens.ts` byte-identically
  from the kit 0.2.0 artifacts, verified against the kit's own recorded digests
  (`7ca6b795…`, `7e0633d3…` measured here, not trusted).
- Re-derive the scoped `kits/scripts/verify-tokens.mjs` from the new upstream
  verifier: same scoping rule (assert the vendored pair and nothing else), plus
  the `--color-focus-ring` name assertion and the `tokenValues` /
  `semanticColorValues` export assertions the 0.2.0 pair must carry; provenance
  comment repointed at `6bf3946…`.
- Update `kits/manifest.json`: top-level `revision` to `6bf3946…`, a new
  explicit `version` field recording `0.2.0` (modeled in `KitManifest` so
  `forge kit pack` preserves it), new per-file sha256 for the three UI files,
  new `synced_at`; the `feed/*.nupkg` entries are untouched (the .NET kit is
  still 0.1.0 — this change resyncs the UI token kit only).
- Bump `PLATFORM_UI_KIT_VERSION` to `"0.2.0"` and re-record the three digests
  in the compiled-in descriptor `src/kit/assets/verification.rs`
  `token_assets()`.
- Update the contract inventory row for `PLATFORM_UI_KIT_VERSION` in
  `src/contract/mod.rs` and change the `inventory_agreement` lib test to check
  that row against the compiled-in constant instead of a blanket `0.1.0`.
- Update the `platform-ui-web@0.1.0` literals in `tests/kit_contract/upgrade.rs`
  and `tests/kit_contract/tokens_and_assets.rs` to the new pinned version.

## BFS Impact Map

- **Capabilities:** `scaffold-prewires-shared-layer` (one design-token source,
  digest-pinned vendoring, deterministic rendering); `kit` upgrade flow
  (`forge kit upgrade --to platform-ui-web@0.2.0` resolves against the
  registry).
- **Users/flows:** generated `react-web` / `nextjs-web` projects receive the
  0.2.0 pair with a receipt recording the new digests and version; operators
  planning a kit upgrade see the new bytes in the reviewable diff.
- **Contracts/data:** `kits/manifest.json` gains a top-level `version` string;
  `KitManifest` gains the matching `#[serde(default)] version` field so the
  only writer (`record_feed_digests`) preserves it. No generated-project
  schema change: `forge.yaml` `kit.version` and `.platform/receipt.json`
  `version` are rendered from `PLATFORM_UI_KIT_VERSION`.
- **Integrations:** none — no network, no npm feed consumption; `kits/feed/`
  `.nupkg` bytes and the `platform-dotnet` descriptor are untouched; the kit
  repo's new `dist/tailwind/*` artifacts and npm tarballs are not vendored
  (Non-goal).
- **Tests:** `cargo test --test kit_contract` (tokens_and_assets, upgrade,
  tampering, manifest, feed all read the mirror dynamically or via constants),
  `cargo test --lib contract` (inventory_agreement), the vendored
  `node kits/scripts/verify-tokens.mjs`, name checker, strict validation.
- **Compatibility:** existing pinned projects keep rendering and verifying
  against their committed receipt bytes; `inspect_kit` refuses
  `platform-ui-web@0.1.0` after the bump, which is the designed single-version
  registry behavior (upgrade review text names the known version); the CLI
  help already examples `platform-ui-web@0.2.0`.

## Capabilities (delta surface)

- One design-token source for the presentation layer (unchanged requirement,
  new artifacts).
- Added: vendored token mirror revision-sync invariant across bytes, manifest,
  descriptor and version constant.

## Non-goals

- No vendoring of `dist/tailwind/preset.js` / `dist/tailwind/theme.css` or any
  npm tarball: no forge consumer, kit descriptor, receipt or scaffold surface
  references them today; extending the owned subtree to them is a scaffold
  feature change, not a resync.
- No `.nupkg` feed repack and no `platform-dotnet` version change.
- No change to the generation, receipt or upgrade machinery beyond the
  manifest `version` field and the contract-inventory test that pins it.
