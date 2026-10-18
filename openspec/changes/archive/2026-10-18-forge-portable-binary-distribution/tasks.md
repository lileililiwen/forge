# Tasks: portable Forge binary distribution

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Reconcile `ROADMAP.md`, `HANDOFF.md`, `openspec list`, artifact baseline and standard-pack versions; map release job dependencies and all existing package/install callers.
- [x] 1.2 Add the stable publication and generated-consumer requirements; map each requirement to the GitHub workflow, scripts, standard-pack descriptors, receipts and exact tests.
- [x] 1.3 Review the implementation-handoff gate: Rust 1.88, existing POSIX package scripts, Linux x86_64 target, GitHub Actions and the `gh` release boundary; confirm no sibling source changes are required.
- [x] 1.4 Pin the contract-parity source to the revision recorded in `contracts/manifest.json` and prove the manifest revision exists in the upstream checkout without modifying it.
- [x] 1.5 Record UI impact and release/consumer failure states; verify proposal, design and spec agree before implementation.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add release-tag validation and a tag-only publication job to Forge CI, dependent on every existing CI job; scope `contents: write` to that job and refuse duplicate asset replacement.
- [x] 2.2 Make hosted contract parity check out the immutable source revision from Forge's contract mirror manifest; keep the local parity script offline and behavior-compatible.
- [x] 2.3 Add service `baseline-service@1.4.0` and client `baseline-client@1.1.0` immutable release-installer and CI assets with provenance and computed asset digests.
- [x] 2.4 Register both packs and advance new-project defaults while retaining every prior descriptor/rendered byte; preserve standard install/upgrade conflict behavior.
- [x] 2.5 Add offline installer tests for success, missing assets/environment, malformed checksum, checksum mismatch, wrong filename and invalid/missing archive binary; verify no Gate invocation or path exposure on failure.
- [x] 2.6 Document supported target, version tag, release assets, first-release prerequisite, generated workflow install and recovery behavior; correct the stale roadmap row.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Exercise all six profile defaults and explicit old-pack pins; compare generated workflow/hook parity, receipts, upgrade plans, hand-edited conflict refusals and old snapshot stability.
- [x] 3.2 Review every release job dependency, trigger, version check, artifact name, permission, duplicate-release behavior and contract-source checkout; prove branch/PR runs cannot publish.
- [x] 3.3 Rehearse the installer with the exact locally packaged target asset and fake downloads; confirm digest verification precedes extraction and Gate.
- [x] 3.4 Review sibling repository states; confirm no sibling file, remote tag, GitHub Release, repository visibility or credential was changed.

## 4. Verification

- [x] 4.1 Run focused standard/release tests, workspace build, changed-Rust formatting and generated installer checks; record exact commands and outcomes.
- [x] 4.2 Run `RUSTC_WRAPPER= CARGO_INCREMENTAL=0 timeout 600 scripts/check-tests.sh`; rehearse `forge gate --dry-run` and run full Gate with a bounded timeout; record the exact verdict and any unrelated failure.
- [x] 4.3 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, `git diff --check`, status/instructions checks; review every added file and promoted requirement.
- [x] 4.4 Archive without `--skip-specs`, commit only related implementation/tests/archive/promoted specs, update roadmap state and HANDOFF evidence, commit HANDOFF alone, then stop without pushing or publishing.
