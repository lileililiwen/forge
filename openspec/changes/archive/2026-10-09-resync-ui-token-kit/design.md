# Design: Resync the vendored UI token kit mirror

## Ownership

The token source of truth stays in `dotnet-platform-libs/ui/` (generator
`ui/scripts/generate-tokens.mjs`, source `ui/tokens/tokens.json`, artifacts
`ui/packages/design-tokens/dist/`). Forge only mirrors published-at-a-revision
bytes; it never edits them. This change consumes the kit revision
`6bf3946fcb1a9d2294b13d0af4347f6cf0d8e566` (`ui-design-guideline-adoption`,
package version `0.2.0`). `86f9ae0` is the kit's handoff-only follow-up (no
`ui/` bytes differ), so the recorded source revision is `6bf3946`.

## What the mirror tracks, and why Tailwind does not enter it

The `platform-ui-web` descriptor (`src/kit/registry.rs`,
`src/kit/assets/verification.rs::token_assets`) owns exactly three vendored
files — `tokens/tokens.css`, `tokens/tokens.ts`,
`scripts/verify-tokens.mjs` — staged into a generated project's
`.platform/tokens/` owned subtree with one receipt digest per file. The
manifest tracks the same three plus the nine `platform-dotnet` feed `.nupkg`
bytes. The kit's new `dist/tailwind/preset.js` and `dist/tailwind/theme.css`
ship inside the `@platform/design-tokens` npm tarball (`ui/feed/`); Forge
consumes no npm feed, no scaffold, descriptor or test references those files,
and the offline dependency-free build contract of `react-web`/`nextjs-web`
does not load Tailwind. Vendoring them would mean extending the owned subtree
and the render contract — a feature change with its own spec, not part of a
resync. Decision: mirror tracks tokens.css, tokens.ts, verify-tokens.mjs;
Tailwind artifacts and npm tarballs are deferred with an explicit non-goal.

## Digest verification must be re-measured, not trusted

The kit records its digests (tokens.css `7ca6b795…`, tarball `e48284…`), but
the three-way agreement Forge enforces (file bytes ↔ `kits/manifest.json` ↔
compiled-in descriptor) is computed here with `sha256sum` after copying; the
copy is byte-identical (`diff` empty) before any record is rewritten. The
tarball digest is not used by Forge (no npm feed vendoring).

## Manifest schema decision: an explicit `version` field

`kits/manifest.json` today records `schema_version` (document shape),
`source`, `revision`, `synced_at`, and per-file `path/sha256/source/revision`.
The kit semantic version lived only in `PLATFORM_UI_KIT_VERSION`. A manifest
that names the source revision but not the released version makes the mirror's
own story ("bytes generated at kit 0.2.0") reconstructible only via the
sibling checkout. Decision: add a top-level `"version": "0.2.0"` and model it
as `#[serde(default)] pub(super) version: String` in `KitManifest`, because
`record_feed_digests` (`forge kit pack`) re-serializes the whole struct — an
unmodeled field would be silently dropped at the next pack, which is exactly
the drift this manifest exists to prevent. Default-empty is tolerated: the
loader accepts pre-resync manifests; Forge's committed manifest always carries
the value.

## Vendored verifier re-derivation

The scoped derivative keeps its contract: run offline inside a generated
project and assert the evidence over the vendored pair. From the grown
upstream verifier it adopts what is visible in the pair: the added
`--color-focus-ring` name assertion (the documented focus-indicator alias) and
export-shape assertions on `tokens.ts` (`tokenValues` map,
`semanticColorValues` light+dark). Repo-wide completeness, template-package
and Razor surfaces stay upstream-only, matching the original scoping comment.

## Version bump surfaces

`PLATFORM_UI_KIT_VERSION` is the compiled-in fact a scaffold pins
(`forge.yaml kit.version`), receipts (`receipt.json version`) and resolves
upgrade targets against (`inspect_kit`). The contract inventory row in
`src/contract/mod.rs` records the same version for auditability; its blanket
`inventory_agreement` test (`every row == "0.1.0"`) would report a false alarm
on the first legitimate bump — decision: the UI-kit row is checked
against the actual `kit::registry::PLATFORM_UI_KIT_VERSION` constant, all
other rows keep the `0.1.0` baseline, so the check still compares two live
surfaces instead of hard-coding twice.

## Failure boundaries

A half-applied resync (new bytes, old record or old descriptor) fails
`kit-digest-mismatch` naming the file with expected/actual digests — the
existing tamper tests prove both directions after the bump because they copy
the live tree. Existing pinned projects are untouched by the bump; their
receipts verify their own committed bytes.
