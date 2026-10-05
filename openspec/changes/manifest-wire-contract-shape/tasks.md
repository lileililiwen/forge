# Tasks: manifest-wire-contract-shape

Phase order is BFS baseline → DFS requirement implementation → BFS
regression/completeness → Verification, per `.ai-rules/workflow.md`. Checkbox
state reflects only work actually completed.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Reproduce the defect from the producer's own constants rather than
  from a hypothesis: read the pinned schema at
  `/home/paul/code/platform-contracts/schemas/public-portfolio-manifest.schema.json`
  and the three emitting sites, and confirm the required type/pattern for
  `schema_family`, `schema_version` and `manifest_revision`.
- [x] 1.2 Map every reader of the three fields. `MANIFEST_SCHEMA_FAMILY` and
  `MANIFEST_SCHEMA_VERSION` are read only at `src/portfolio/share/manifest.rs:74-75`
  and in its own tests; `grep -rn "portfolio-manifest" src/` shows no second
  consumer. `draft.body.manifest_revision` is read at `src/main.rs:10560,10586`
  and `src/api/mod.rs:2889`, none of which does arithmetic on it.
- [x] 1.3 Map every reader of the *internal* revision and record them as
  out-of-scope: `manifest_revision INTEGER NOT NULL`
  (`src/registry/share/mod.rs:88`), `PublishReport`/`PublishContext`/
  `AdapterRequest`/`PublicationAttempt` (`i64`), and the audit query
  (`src/registry/share/audit.rs:187-210`). Establish that these read
  `approval.revision` or the stored row, never the body field.
- [x] 1.4 Map the deterministic-hash design and confirm it survives the change:
  `ManifestBody` carries the contract's fields only, `generated_at` and
  `manifest_sha256` are outside it, and projects sort by id and surfaces by
  `(label, url)`.
- [x] 1.5 Find the affected tests by searching the shape rather than by name:
  `tests/portfolio_share_cli_contract.rs:313-314` assert the two fields,
  `src/portfolio/share/manifest.rs:392-394` assert the third,
  `tests/portfolio_share_cli_contract.rs:876-898` assert the key *set* only,
  and `tests/portfolio_share_cross_surface.rs` /
  `tests/portfolio_share_api_contract.rs` read `manifest_sha256` alone.
- [x] 1.6 Record the three further schema mismatches found during the map
  (`visibility`, `status_evidence`, `id`) as declared non-goals rather than
  widening this change's scope.

## 2. DFS — Requirement implementation

- [x] 2.1 `platform.public-portfolio-manifest` family: `MANIFEST_SCHEMA_FAMILY`
  emits the full contract name, with its doc comment corrected to name the
  contract and the qualified family.
- [x] 2.2 `"1.0.0"` schema version: `MANIFEST_SCHEMA_VERSION` becomes a `&str`
  carrying the full semantic version, and its doc comment stops claiming it is a
  major.
- [x] 2.3 Revision encoding: `wire_manifest_revision(u32) -> String` added, used
  by `ManifestBody::new` and the published document, with the match against the
  contract's own pattern proven in a test.
- [x] 2.4 `ManifestBody` and `PublicPortfolioManifest` carry
  `schema_version: String` and `manifest_revision: String`; `build_manifest` and
  every persistence-facing type keep the integer.
- [x] 2.5 `src/main.rs:10560,10586` and `src/api/mod.rs:2889` required **no**
  edit: `format!` and `serde_json::json!` accept the new `String`, and neither
  site does arithmetic on the revision. Confirmed by `git diff --name-only`,
  which lists neither file.

## 3. BFS — Regression and completeness

- [x] 3.1 Update the two stale assertions in
  `tests/portfolio_share_cli_contract.rs` to the new wire shape.
- [x] 3.2 Update the in-module test
  `the_document_embeds_the_hash_and_the_emission_time` and add
  `the_document_matches_the_contract_shape`, which pins all three fields and
  matches them against the schema's enum and patterns.
- [x] 3.3 Add `manifest_revision_encoding_is_stable_and_schema_shaped`,
  covering `0`, `1`, `4`, `10` and `u32::MAX`, and asserting the encoding is
  deterministic and injective.
- [x] 3.4 Add the acceptance test `tests/manifest_wire_contract.rs`, which
  builds a document through `build_manifest` / `ManifestDraft::document` and
  validates the serialized bytes against the pinned sibling schema with
  `jsonschema`.
- [x] 3.5 Confirm no persistence, audit, report or adapter type changed: the
  diff touches no file under `src/registry/`, and the `i64` / `INTEGER`
  declarations are byte-identical to the parent commit.
- [x] 3.6 Confirm the parked change `contract-parity-gate-real-digests` and
  `contracts/**` are untouched by the diff.
- [x] 3.7 Record the approval consequence: a pre-existing approval is bound to
  the old hash and is refused until re-approved. Stated in `design.md` §4.

## 4. Verification

- [x] 4.1 `cargo build`
- [x] 4.2 `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  **2294 passed / 0 failed / 3 ignored**. The one skipped test is the
  pre-existing hang recorded in HANDOFF; the three ignored tests are the
  pre-existing `contract::tests::parity_walk`, the pre-existing
  `packing_leaves_the_sibling_checkout_byte_identical`, and this change's own
  acceptance test, run separately by 4.3.
- [x] 4.3 `cargo test --test manifest_wire_contract -- --ignored --nocapture`:
  **1 passed**. `jsonschema accepted the Forge-produced manifest: valid: 1
  project(s), revision 'rev_1', schema_version '1.0.0'`. The consumer's own
  `validate_manifest.py --strict` was additionally run against a document
  published by the built `target/debug/forge` binary: `manifest OK … (schema
  1.0.0, 1 project(s), revision rev_1)`, exit 0.
- [x] 4.4 `node scripts/check-openspec-change-names.mjs`
- [x] 4.5 `openspec validate --all --strict --no-interactive`
- [x] 4.6 `cargo fmt --check`
- [x] 4.7 `cargo clippy --workspace --all-targets`
- [x] 4.8 `git diff --check`
- [ ] 4.9 Archive. Deliberately not run: the owner authorized this change to
  proceed alongside the parked `contract-parity-gate-real-digests`, and
  `.ai-rules/workflow.md` allows one active change at a time. The change is left
  active with every implementation task evidenced.
