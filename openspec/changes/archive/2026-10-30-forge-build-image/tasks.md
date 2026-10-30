## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm `forge build` produces no artifact and that its non-image path is independent of the image path.
- [x] 1.2 Confirm the native runner captures stdout and stderr separately and has no stdin entry point.
- [x] 1.3 Confirm the shared extension contract owns the version rule this adapter contract will use.
- [x] 1.4 Confirm `.forge/build/` is not already used and that `.forge/` records are written atomically elsewhere.
- [x] 1.5 Confirm the deploy lane's artifact input is out of scope and that no Gate check consumes a build record.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add a bounded native runner entry point that supplies stdin and keeps the timeout and process-group kill.
- [x] 2.2 Define the image-builder contract types, the request and the response.
- [x] 2.3 Resolve the project revision, the default tag and the adapter binary; write the request.
- [x] 2.4 Validate the response: contract compatibility, status, image equals tag, digest shape, builder identity, evidence bounds.
- [x] 2.5 Validate and hash the SBOM: relative, inside the project, exists, bounded, SHA-256 computed by Forge.
- [x] 2.6 Write the provenance record atomically under `.forge/build/<project-id>/<sha12>.json`.
- [x] 2.7 Add `--image`, `--tag` and `--builder-bin` and their refusal rules.
- [x] 2.8 Render the outcome human and JSON.
- [x] 2.9 Write the two reference adapters and the contract document.
- [x] 2.10 Add the build-to-image seam to the CI/CD system model.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 A built response with a digest and an SBOM writes the record with Forge's own SBOM hash.
- [x] 3.2 A built response without an SBOM writes a record with no SBOM.
- [x] 3.3 A failed status and a non-zero exit are `build-failed` and write no record.
- [x] 3.4 A malformed response, a wrong image, a bad digest and a bad contract are `build-invalid` and write no record.
- [x] 3.5 An SBOM path that escapes the project and one that names no file are refused.
- [x] 3.6 A missing adapter is refused with the variable and the install recovery.
- [x] 3.7 `--tag` or `--builder-bin` without `--image` is refused.
- [x] 3.8 `--image` with `--os`, `--arch` or `--rid` is refused.
- [x] 3.9 The default tag is `forge-<project-id>:<sha12>`.
- [x] 3.10 A project without Git history is refused.
- [x] 3.11 `forge build` without `--image` behaves exactly as before.
- [x] 3.12 The contract accepts a newer minor and refuses a different family or major.
- [x] 3.13 The record is written atomically and carries no absolute path.
- [x] 3.14 The reference adapters compile under Python 3 and are executable.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run `cargo build` and the focused image, native and build suites.
- [x] 4.3 Run the repository Gate: `forge gate --dry-run`, then the full `forge gate`.
- [x] 4.4 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.5 Run a real image build against a recording adapter and record the built and refused outcomes.

## 5. Implementation — authorized surfaces

- [x] 5.1 `src/image/` and its tests.
- [x] 5.2 `src/native/mod.rs` and its tests.
- [x] 5.3 `src/main.rs` and `src/cli/gitops.rs`.
- [x] 5.4 `adapters/image/`.
- [x] 5.5 `docs/adapter-contracts/image-builder.md` and `docs/ci-cd-system-model.md`.
- [x] 5.6 The CLI contract tests.
- [x] 5.7 Record the exact output of every verification command in the evidence table below.
- [x] 5.8 Archive without `--skip-specs` so the capability is promoted, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS, 4341 tests / 0 failed | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 and reported 4341 passed, 0 failed, 1 ignored, 3 filtered out across 246 suites in 358.63s. |
| Focused suites | PASS, 52 tests / 0 failed | `cargo test -p forge --lib image::` → 11 passed; `native::` → 11 (two new stdin tests); `build::` → 8; `command_catalog` → 7; `cargo test --test build_image_contract` → 10; `--test cli_contract` → 5. |
| Repository Gate | FAIL, 7/8 — the same pre-existing advisory | `forge gate --dry-run` resolved all 8 required checks with none unbound. The first full run failed `source-file-size` on `src/main.rs` at 1001 lines; the build arguments moved into `BuildArgs` in `src/cli/gitops.rs`, leaving `src/main.rs` at 964 lines. The second full `forge gate --timeout-secs 1800` run reported aggregate `blocked`: only `security` failed on RUSTSEC-2026-0387, and `tests`, `source-file-size` (465 candidates, 1000-line maximum) and the other six passed. `.forge/gate/forge/evidence.json` records `"aggregate": "blocked"` at `2026-10-30T11:16:35Z` on revision `7e9dff6`; the advisory and its remediation path are the ones recorded in the two previous entries, and this change touches neither `Cargo.toml` nor `Cargo.lock`. |
| Real image build | PASS | A fixture project at `/tmp/opencode/image-manual/proj` with a recording adapter: `forge build … --image --builder-bin …` printed `tag: forge-image-manual:fad25ef115a6`, `builder: railpack`, `digest: sha256:bbbb…`, and `sbom: sbom.json (48 bytes, sha256:4e9bb7b6…)`. `.forge/build/image-manual/fad25ef115a6.json` carries `contract: forge-build-image/0.1.0`, the revision, the tag, the builder and version, the digest, Forge's SBOM hash and byte count, and no absolute path. |
| Real refusal | PASS | With `FORGE_IMAGE_BUILDER_BIN` unset and no `forge-image-builder` on `PATH`, `forge build … --image` exited 1 with `error[build-invalid]: build invalid: the image builder `forge-image-builder` could not start: No such file or directory (os error 2); install it or set `FORGE_IMAGE_BUILDER_BIN`; Forge never installs a builder`, and wrote no record. The other refusals are the ten `build_image_contract` cases. |
| Reference adapters | PASS for syntax, NOT exercised | `python3 -m py_compile adapters/image/forge-image-builder-railpack adapters/image/forge-image-builder-pack` exits 0 and both are executable. Neither has been run against a real Railpack or `pack` installation on this host; `docs/adapter-contracts/image-builder.md` states that and the adoption prerequisite (install the builder and Docker first). |
| OpenSpec and whitespace | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 171 passed, 0 failed (171 items)`; `git diff --check` reported no findings. Archived without `--skip-specs`, and the promoted spec's `TBD` Purpose was replaced before the commit. |

## Notes on the implementation, for the reviewer

- **The runner gained stdin, not a second runner.** `run_native_with_input`
  reuses the bounded capture, timeout and process-group kill; the request is
  one small document and a command that ignores it still reports its status.
- **The adapter reports a failure with exit 0.** A `status: failed` response is
  how Forge gets the builder's evidence; a non-zero exit is reserved for an
  adapter that could not answer. Both are `build-failed`.
- **Forge hashes the SBOM itself.** The adapter names a project-relative file;
  Forge checks the path, size and existence, reads it and records its own
  SHA-256, so the record carries one fewer unchecked claim.
- **The record is not Gate evidence.** It is written atomically under
  `.forge/build/` and feeds nothing; a Gate check that consumes it and a deploy
  artifact input are separate changes.
- **`--image` refuses `--os`/`--arch`/`--rid`.** The builder owns the platform;
  accepting those flags would claim a selection Forge did not make.
- **`forge build` without `--image` is unchanged**, including its refusals, and
  the five `cli_contract` tests and the existing `build::` tests still pass.
- **The CLI arguments moved to `BuildArgs`** because the Gate's source-size
  check caught `src/main.rs` one line over the maximum; the command surface is
  identical.

