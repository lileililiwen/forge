# Tasks: Import validated ideas from Hypora

Contract: `forge-graduation-import/0.1.0`. Input contract:
`platform.idea-graduation/0.1.0` (major `0`). Read `design.md` for the
exact key sets, deny lists, bounds, types, grammar, output shapes and
named test list; this file is the execution order. All tasks are checked;
the evidence is recorded in `HANDOFF.md` and the two new suites.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Capture the pre-change baselines before editing anything:
  `cargo fmt --all -- --check` drift locations and
  `cargo clippy --all-targets -- -D warnings` locations, using
  `git stash push -u -- src tests` and a `comm` diff. Record the exact
  pre-change test counts
  (`cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`).
- [x] 1.2 Re-read and confirm every reused symbol at its current path:
  `crate::import::build_manifest_text` (reused) and the
  `crate::import::derive_project_id` kebab pattern (mirrored for the
  title-derived id),
  `crate::core::manifest::{Manifest, resolve_manifest_path}`,
  `crate::core::validate_project_id`, `crate::profile::inspect_profile`,
  `Registry::{register, check_identity_available}`,
  `crate::policy::redact_credentials`,
  `portfolio::interest::validation::{looks_like_secret, classify_refused_key}`
  and `portfolio::interest::redact_interest_text` as the pattern to
  mirror. Confirm `platform.idea-graduation` and `AppSpec` still do not
  exist in the tree. (Evidenced in `design.md`, "Implementation
  boundary" and "Ownership and shared code".)
- [x] 1.3 Confirm the impact map: no new dependency, no manifest field,
  no registry table/migration, no API route, no MCP tool, no portal
  change; `forge import` untouched.
- [x] 1.4 Confirm the naming decision (`src/graduation/`, `forge
  graduation`) and the producer-contract pinning strategy are recorded
  in the design's decision ledger before implementing.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `src/graduation/mod.rs`: `GRADUATION_CONTRACT_VERSION`,
  `IDEA_GRADUATION_CONTRACT`, `SUPPORTED_IDEA_GRADUATION_MAJOR`,
  `SUPPORTED_IDEA_GRADUATION_REVISIONS`, `GRADUATION_DIR`, all bounds,
  the closed key sets, the four deny lists, `GraduationSource`,
  `GraduationSuccessMetric`, `GraduationBrief`, `GraduationEvidence`,
  `GraduationExperiment`, `GraduationImport`, `GraduationRefusal`, the
  `refusal` module, `redact_graduation_text`, `looks_like_secret`,
  `looks_like_email`, `is_raw_url_with_query`, and its unit tests.
- [x] 2.2 Add `src/graduation/validation.rs`: `read_artifact` (path or
  stdin, `MAX_GRADUATION_BYTES` enforced before parse), `parse_artifact`
  (decode into a closed-map record; family/major/revision check first),
  and `validate_graduation` (deny lists → closed key sets → provenance →
  brief → experiment → `validated == true` → bounds → scrub), with its
  unit tests.
- [x] 2.3 Add `src/graduation/import.rs`: `GraduationProposal`,
  `GraduationPreview`, `GraduationAdoption`, `build_proposal`
  (read-only: profile via `inspect_profile`, destination checks, id
  derivation) and `adopt_graduation` (minimal manifest via
  `build_manifest_text`, `Manifest::parse` self-check, receipt write,
  `register`, file rollback on failure), plus `render_preview_human` and
  its unit tests.
- [x] 2.4 Add `ForgeError::GraduationInvalid { reason }` and
  `ForgeError::GraduationConflict { reason }` with codes
  `graduation-invalid` and `graduation-conflict` in `src/core/mod.rs`,
  and their `code()` arms. Do not add any other Core variant.
- [x] 2.5 Add `pub mod graduation;` to `src/lib.rs` (alphabetical).
- [x] 2.6 Add `GraduationCommands` (`Preview`, `Import`) and
  `Commands::Graduation { command }` in `src/main.rs`, the dispatch arm,
  `cmd_graduation`, `cmd_graduation_preview`, `cmd_graduation_import`
  and the human/JSON renderers with the exact flags, defaults, bounds
  and envelopes from the design. `import` without `--confirm` must be a
  dry run that writes nothing and exits `0`.
- [x] 2.7 Implement the receipt exactly as specified: write
  `.forge/graduation/<id>/import.json` through the closed
  `GraduationReceipt` shape (brief + source + `evidence_count` +
  `imported_at` + `actor`), and prove no evidence excerpt and no extra
  key can reach it.
- [x] 2.8 Add the unit tests named in `design.md` under "Verification
  oracle" (module, validation and import groups).
- [x] 2.9 Add `tests/graduation_cli_contract.rs` with every named CLI
  and in-process-library test, including the preview/dry-run equality,
  the typed refusals with empty stdout, stdin, and both JSON envelopes.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Add `tests/graduation_cross_surface.rs`: prove a preview and
  a confirmed import make no network call and no `gh` invocation, and
  scaffold/deploy/approve nothing. A preview spawns no process; a
  confirmed import's only child process is the registry's existing
  best-effort `git` probe, shared with `forge import`. Prove the
  registry gains exactly one project and one journal row.
- [x] 3.2 Prove no evidence excerpt, no original artifact byte and no
  field outside the closed set reaches disk, and that the original
  artifact is never copied into the workspace.
- [x] 3.3 Prove an unrelated registry and a project created before this
  package are read as-is, and that `forge import`, the manifest schema
  and the existing contract suites pass without edits.
- [x] 3.4 Exercise the boundary cases end to end: unsupported major,
  unknown revision, every deny-list class, credential/email/query-URL
  values, oversized and malformed files, missing/invalid `validated`,
  every per-field bound, unknown profile, manifest-bearing destination,
  reserved and registered ids, stdin, and the no-`--confirm` path.

## 4. Verification

- [x] 4.1 Run formatting, build, clippy, focused unit tests, the two
  new suites, the full suite, the name preflight, strict OpenSpec
  validation and `git diff --check`. Diff `cargo fmt` and `cargo clippy`
  against the captured baselines and confirm zero new locations; restore
  any incidentally reformatted pre-existing file with `git checkout --`.
- [x] 4.2 Run the live CLI smoke paths and record the exact bytes/exit
  codes: preview writes nothing; `import` without `--confirm` writes
  nothing; `import --confirm` creates one project and one receipt; each
  refusal is a typed error with empty stdout.
- [x] 4.3 Record that no Hypora endpoint was contacted, no credential
  was exchanged, no project was scaffolded or deployed, and no gate was
  approved; this package claims no end-to-end Hypora proof.
- [x] 4.4 Record the deferred claim explicitly: end-to-end adoption
  waits for the Hypora producer change and the `contracts/` vendoring
  package; this package's oracle is its local fixtures.
- [x] 4.5 Set the single `current_spec` pointer to this change before
  implementing, update `HANDOFF.md` with the evidence, and archive only
  after every requirement above is evidenced.
