# Tasks: Declaration-first plugin resolution and Forge's own interface block

## Phase 1 — BFS baseline

- [ ] 1.1 Read the current resolution path end to end (`src/plugins/mod.rs` `builtin_descriptor`, `builtin_from_kits_manifest`, `builtin_hardcoded`, `src/cli/plugins.rs`, `src/cli/cap.rs`) and record the exact call sites and rendered rows.
- [ ] 1.2 Record the versioned-surface inventory as the declaration's source: every constant in the `CONTRACTS` table (`src/contract/mod.rs:44`) and every adapter contract constant used by a plugin id.
- [ ] 1.3 Confirm which project files Forge already parses (`.project.json` via `contract::emit_capability`, `forge.yaml` via `Manifest`) so the new read reuses an existing reader rather than adding a second one.
- [ ] 1.4 Inventory Forge's retained copies and their digests: `scripts/governance/{product_code_quality.py,vocabulary.py}`, `scripts/vocabulary.json`, `contracts/**` + `contracts/manifest.json`, `kits/**` + `kits/manifest.json`, and the third `secret-field-substrings.json` copy.
- [ ] 1.5 Enumerate the tests that assert the current builtin mapping, so each is either preserved or deliberately updated.
- **Done when** every changed surface, every source rank and every affected test is named.

## Phase 2 — DFS implementation

- [ ] 2.1 Add declaration reading: parse `interfaces[]`, `capabilities`, `integrations` from the inspected project, with typed refusals for an unparseable declaration and for each invalid field this change names.
- [ ] 2.2 Implement the precedence chain (descriptor → declaration → kit manifest → builtin) and record the answering source per resolved row.
- [ ] 2.3 Refuse a descriptor/declaration contradiction by name instead of resolving it silently.
- [ ] 2.4 Render the provenance in `forge plugins list` and keep `forge cap` counts unchanged for the same id set.
- [ ] 2.5 Derive the published declaration values from `CONTRACTS` and the adapter constants; add a test asserting the derived set equals the table.
- [ ] 2.6 Write `interfaces[]` into `forge/.project.json`, recording the two agent-runtime entries **without** a version (the real state) rather than inventing one, and naming the vendored copies as `consume` edges with their manifest digests.
- [ ] 2.7 Reduce the third secret-field vocabulary copy to a single declared edge against its owner's artifact.
- **Done when** a declared row and a builtin row resolve identically in kind and capabilities, every refusal names its field, and no id loses resolution.

## Phase 3 — BFS regression and completeness

- [ ] 3.1 Regression: `forge plugins list`, `forge cap list|inspect`, the API shipping-reads section and the command catalog tests pass with the same id set; the only difference is the provenance value.
- [ ] 3.2 Prove behavioural equality: with a declaration present the row is `declared`; with it removed the row is `builtin`; kind and capabilities are identical in both.
- [ ] 3.3 Prove no silent invention: the agent-runtime interfaces appear without a contract version and the audit reports them, in the same run.
- [ ] 3.4 Check the no-sibling invariant: every case runs with no sibling checkout present.
- [ ] 3.5 Documentation: `docs/architecture.md` and `docs/provider-evidence.md` name the declaration as the source of the plugin boundary, and the provenance column is documented.
- **Done when** every observation agrees with this change's scenarios and unrelated dirty files remain excluded.

## Phase 4 — Verification

- [ ] 4.1 `cargo fmt --check`, `cargo build`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] 4.2 Targeted suites plus the workspace suite; record results.
- [ ] 4.3 `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, `git diff --check`.
- [ ] 4.4 `forge gate --dry-run`, then the full `forge gate` with a bounded timeout; record the verdict, attributing any pre-existing failure with its remediation path.
- [ ] 4.5 Record exact verification results in `HANDOFF.md`, archive without `--skip-specs`, and commit code, tests and specs first, `HANDOFF.md` second.

## Dependency

- [ ] Requires `workspace-governance` change `interface-declaration-and-boundary-findings` to land first: this change reads the `interfaces[]` block that change defines, and its refusal rules mirror that schema.
