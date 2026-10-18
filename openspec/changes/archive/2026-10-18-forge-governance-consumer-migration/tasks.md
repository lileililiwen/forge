# Tasks: hide governance internals behind Forge

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Record all active Workspace Governance path/provider references in
  Forge-owned product manifests, scripts, docs and generated consumer assets;
  exclude archived change history.
- [x] 1.2 Capture the current concern-check JSON/exit contracts and the Forge
  source/installed resource resolution behavior.
- [x] 1.3 Map each in-tree crate Gate check before removing the portfolio audit;
  prove native build/test/security and other project-specific checks remain.
- [x] 1.4 Confirm external sibling repositories are a separate migration
  package and remain untouched here.
- [x] 1.5 Confirm proposal/design/spec agree and that no user-facing browser
  scope is omitted.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `forge concern check <id> [root]`, register it in Forge's command
  catalog, and preserve checker exit/output forwarding.
- [x] 2.2 Package/install all canonical checker resources and verify source-tree
  and installed-binary lookup.
- [x] 2.3 Replace in-tree product wrapper internals with calls to the Forge
  concern command; remove sibling path derivation.
- [x] 2.4 Migrate consolidated crate Gate identity to `forge-baseline/1.0.0` and
  remove workspace-wide `governance-quality` commands from concrete projects.
- [x] 2.5 Repoint internal contract sync to the in-tree source and update
  current product-facing docs without rewriting archived provenance.
- [x] 2.6 Confirm no Workspace Governance source edits were needed, so the
  existing import parity allowlist requires no change.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check all seven concern IDs and negative/operational cases through the
  source-tree and installed Forge binary.
- [x] 3.2 Re-run consolidated crate Gate dry-runs and confirm every remaining
  required check is bound and no native command was removed.
- [x] 3.3 Search live concrete product surfaces for `workspace-governance`
  paths, `workspace-governance@1` and provider identity; distinguish internal
  Forge implementation from product contracts and history.
- [x] 3.4 Verify CI/pre-commit standard templates still invoke the same Forge
  Gate and no project requires the Workspace Governance sibling checkout.
- [x] 3.5 Verify external sibling repositories and remote state were not
  changed; record the portability package as the next migration.

## 4. Verification

- [x] 4.1 Run focused Forge CLI/resource contracts and release package/install
  smoke.
- [x] 4.2 Run consolidated native crate builds/tests and Gate rehearsals.
- [x] 4.3 Run Workspace Governance module self-check/suite; no module Gate is
  configured and the module source was unchanged. Keep its known source-size
  candidate gap for the separate conformance package.
- [x] 4.4 Run Forge Gate dry-run then bounded full Gate and record the verdict
  and any unrelated failures in HANDOFF.
- [x] 4.5 Run name preflight, strict OpenSpec validation, import verifier,
  `git diff --check` and `git diff --cached --check`.
- [x] 4.6 Review impact rows, archive without `--skip-specs`, promote the
  canonical spec, commit implementation/archive, make the HANDOFF-only commit,
  and stop without pushing.
