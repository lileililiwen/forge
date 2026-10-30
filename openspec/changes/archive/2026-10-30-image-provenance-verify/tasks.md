## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the build record is write-only today and that nothing reads it.
- [x] 1.2 Confirm the record shape and the validation rules the previous change wrote.
- [x] 1.3 Confirm a Gate check is a bound process whose exit status is the verdict, and that no Gate engine change is needed.
- [x] 1.4 Confirm no default check or pack has to change for a project to opt in.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Read the record for the project and the revision at `HEAD`, with the relative path in the refusal.
- [x] 2.2 Validate the record's contract, project, revision and digest.
- [x] 2.3 Re-read and re-hash the SBOM, refusing a missing file and a changed hash or length.
- [x] 2.4 Add `--verify-image` and refuse every incompatible flag.
- [x] 2.5 Render the verified record as JSON and human output.
- [x] 2.6 Document verification and the two-line Gate opt-in.
- [x] 2.7 State the build-to-check seam in the CI/CD system model and the README.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 A fresh record verifies and writes nothing.
- [x] 3.2 A moved `HEAD` fails with the revision and the rebuild recovery.
- [x] 3.3 An edited SBOM fails on the hash.
- [x] 3.4 A tampered digest fails.
- [x] 3.5 A record naming another project or revision fails.
- [x] 3.6 A missing record fails.
- [x] 3.7 An unsupported record contract fails.
- [x] 3.8 Every exclusive-flag combination is refused.
- [x] 3.9 Verification starts no builder and writes no file.
- [x] 3.10 `forge build --image` is unchanged.
- [x] 3.11 The bound-check documentation names the exact check entry and script.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run `cargo build` and the focused image, CLI and build suites.
- [x] 4.3 Run the repository Gate: `forge gate --dry-run`, then the full `forge gate`.
- [x] 4.4 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.5 Run a real build-then-verify, then verify again after a commit.

## 5. Implementation — authorized surfaces

- [x] 5.1 `src/image/mod.rs` and its tests.
- [x] 5.2 `src/main.rs` and `src/cli/gitops.rs`.
- [x] 5.3 `docs/adapter-contracts/image-builder.md`, `docs/ci-cd-system-model.md` and `README.md`.
- [x] 5.4 The CLI contract tests.
- [x] 5.5 Record the exact output of every verification command in the evidence table below.
- [x] 5.6 Archive without `--skip-specs` so the capability is promoted, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS, 4348 tests / 0 failed | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 and reported 4348 passed, 0 failed, 1 ignored, 3 filtered out across 246 suites in 378.58s. |
| Focused suites | PASS, 41 tests / 0 failed | `cargo test -p forge --lib image::` → 11 passed; `cargo test --test build_image_contract` → 17 (seven new verification cases); `--test cli_contract` → 5; `build::` → 8. |
| Repository Gate | FAIL, 7/8 — the same pre-existing advisory | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run reported aggregate `blocked`: only `security` failed on RUSTSEC-2026-0387, and `tests`, `source-file-size` and the other six passed. `.forge/gate/forge/evidence.json` records `"aggregate": "blocked"` at `2026-10-30T12:01:19Z` on revision `60c9341`; the advisory and its remediation path are recorded in the previous three entries, and this change touches neither `Cargo.toml` nor `Cargo.lock`. |
| Real build-then-verify | PASS | For the record built in the previous change's fixture, `forge build … --verify-image` printed `verified: yes`, the revision, the tag, the builder, the digest and the SBOM hash. |
| Real stale verify | PASS | After a commit moved `HEAD` to `01d9530d…`, the same command exited 1 with `error[build-invalid]: build invalid: no readable image build record at `.forge/build/image-manual/01d9530d433a.json` for revision `01d9530d433a5a6b1350c953dd4cebad2ee52ace` (No such file or directory (os error 2)); run `forge build --image` at this revision`. After rebuilding and editing the SBOM, it exited 1 with `error[build-failed]: build command failed: the SBOM `sbom.json` no longer matches the recorded build: recorded sha256:4e9bb7b6… (48 bytes), observed sha256:58bc4076… (42 bytes); rebuild with `forge build --image``. |
| Verification is read-only | PASS | `verification_writes_nothing` compares the record bytes and the project's whole file list before and after `--verify-image`; both are unchanged. |
| OpenSpec and whitespace | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 172 passed, 0 failed (172 items)`; `git diff --check` reported no findings. Archived without `--skip-specs`, and the promoted spec's `TBD` Purpose was replaced before the commit. |

## Notes on the implementation, for the reviewer

- **The record shape is consumed, not changed.** `verify_image_build` reads the
  document the previous change writes, applies the same contract, project,
  revision, digest and SBOM rules, and returns the record it trusted.
- **A moved `HEAD` is an identity failure, not a hash failure.** The record is
  looked up by the current revision, so a later commit has no record and the
  refusal names the path and the revision rather than comparing digests across
  revisions.
- **The SBOM is re-hashed, not trusted.** A changed hash or byte count is
  `build-failed`; a missing file is `build-invalid`. Both sentences name the
  recorded and observed values and the rebuild command.
- **No Gate engine, baseline or pack change.** The verb exits zero or non-zero
  like any bound check; the two-line opt-in is documented, and a project that
  does not build images is unaffected.
- **`--verify-image` is exclusive.** It refuses `--image`, `--tag`,
  `--builder-bin`, `--os`, `--arch` and `--rid` before reading anything.

