# Tasks: Forge post-MVP readiness

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Record the current README section/command map and identify which v0.1 surface each section does and does not describe.
- [x] 1.2 Confirm the real behavior and limits of `forge import`, `list`, `inspect`, `new` and `doctor`, and of `forge.yaml` and the registries, against the source and a scratch project.
- [x] 1.3 Inventory the active overlapping proposals (`artifact-and-ci-baseline`, `gate-evidence-export-consumption`, `governance-vocabulary-consumption`) and record the single-writer decisions for `LICENSE` and packaging.
- [x] 1.4 Reconcile the README install statement against `artifact-and-ci-baseline`: what exists today versus what is planned.
- [x] 1.5 Check the siblings `workspace-governance` and `driftwatchdog` and record the keep-local default, the `workspace-governance` adapter execute-bit state, and the packaging shape used as reference.
- [x] 1.6 Select quickstart demo scenarios and the capture format; confirm no host path or secret enters the capture.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Rewrite the README command map so each v0.1 surface (manifest, registries, import/list/inspect/new/doctor) states its task, behavior and honest limits.
- [x] 2.2 Present the install section through the delivered packaging path: `artifact-and-ci-baseline` archived first (2026-09-27), so the README names its delivered digest-verified package/install scripts and owning change instead of the proposal's original "no packaging yet" plan wording (spec requirement amended accordingly).
- [x] 2.3 Produce the committed quickstart transcript and terminal capture from real runs and reference it from the README.
- [x] 2.4 Confirm the `LICENSE` file matching the declared MIT expression exists exactly once: `artifact-and-ci-baseline` created it on 2026-09-27, so this package verifies (never duplicates) the single writer.
- [x] 2.5 Document the governance keep-local default (`local` with no `.forge/providers.yaml`), the `workspace-governance` preset invocation, the no-parent-search/no-network boundary, and the sibling-owned execute-bit prerequisite.
- [x] 2.6 Confirm the README no longer overstates or understates any implemented surface.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run every documented README command against the built binary and confirm output and behavior match the text.
- [x] 3.2 Confirm the transcript and command map cover import/list/inspect/new/doctor and the default `local` governance path, with no fabricated output.
- [x] 3.3 Confirm no packaging, publication, tag, push or deploy claim is introduced.
- [x] 3.4 Confirm no command, flag, exit code, document, registry schema or provider contract changed, and `.project.json` is untouched.
- [x] 3.5 Confirm the declared `driftwatchdog` gate runtime and the `workspace-governance` sibling are untouched.

## 4. Verification

- [x] 4.1 `grep -n 'license' Cargo.toml` and confirm `LICENSE` exists and matches; confirm no duplicate writer with `artifact-and-ci-baseline`.
- [x] 4.2 `cargo build` then run `--version`, import, list, inspect, new and doctor; record verbatim output and exit codes.
- [x] 4.3 `cargo test --workspace` (or `--all-targets`), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check` stay clean.
- [x] 4.4 `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive` and `git diff --check` pass.
- [x] 4.5 Record PASS/FAIL/BLOCKED with exact commands, including the sibling-owned blockers; do not archive with unresolved mandatory failures.