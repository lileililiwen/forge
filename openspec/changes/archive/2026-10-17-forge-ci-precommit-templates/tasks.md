# Tasks: Forge CI and pre-commit templates

## BFS baseline

- [x] Confirm current standard-pack generation, install, upgrade, receipt and conflict behavior; map compatible profiles and all callers.
- [x] Record the immutable Forge Gate source revision and CI toolchain prerequisites.
- [x] Add the new OpenSpec requirements and review the impact map against project generation, standard CLI operations and Forge Gate extension behavior.

## DFS requirement implementation

- [x] Add immutable `baseline-service@1.3.0` service assets and `baseline-client@1.0.0` Gate-only client assets with provenance and digests.
- [x] Register the new supported pack versions without changing older version descriptors or rendered bytes.
- [x] Default all six compatible new-project profiles to their current service/client pack while honoring explicit supported versions and preserving unsupported-profile behavior.
- [x] Extend CLI help and user documentation with local Forge and pre-commit prerequisites plus legacy install/upgrade commands.
- [x] Add focused descriptor, render, generation for all six profiles, CI/hook parity, legacy adoption and conflict verification.

## BFS regression/completeness

- [x] Recheck generation callers, interactive and explicit CLI paths, all supported profiles, explicit version pins, install and upgrade conflict boundaries.
- [x] Confirm both entry points run the same Forge Gate command, Gate extension failures propagate, and CI cannot publish or deploy.
- [x] Review all new files, provenance digests, receipts and generated project diffs; confirm no sibling repository files were changed.

## Verification

- [x] Run focused standard-pack/generation verification and required workspace checks; record exact commands and results in HANDOFF. Standard snapshot tests passed 20/20; generation unit tests passed 15/15 with the separate Rust native scaffold linker failure excluded; standard contracts passed 16/16; generation contracts passed 11/11 with the stalled Flutter native test excluded; workspace build passed. Full Gate tests report 10 existing failures in untouched `crates/driftwatchdog`, tracked by the roadmap's test-reconciliation package.
- [x] Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, `git diff --check`, and relevant status/instruction checks. Strict validation passed 109/0.
- [x] Run `forge gate --dry-run` then full `forge gate --timeout-secs 600`; dry-run passed. Full Gate aggregate is blocked by 10 existing DriftWatchdog tests; security passed with existing unmaintained `paste` and `unsafe-libyaml` advisories. No change-attributable failure appeared.
- [x] Archive without `--skip-specs`, promote canonical requirements, commit only related implementation/spec files, then update and commit HANDOFF alone.
