# Tasks: Product-code quality gates

## 1. BFS — Baseline and impact coverage

- [ ] Inventory project declaration loading, existing finding levels, JSON
  output, secret redaction, and `auto_gate.py` callers.
- [ ] Inventory product/source roots and language profiles in the registry and
  identify projects that already have executable Gate commands.
- [ ] Map every requirement and scenario to checker, declaration, adapter,
  fixture, and documentation surfaces.
- [ ] Add fixtures for Rust `#[cfg(test)]`, forbidden Rust product test code,
  Python/TypeScript/C# test leakage, marker counts, exclusions, and unreadable
  files before implementation.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Add strict quality-policy parsing with required product roots, language
  profile validation, bounded exclusions, and integer threshold validation.
- [ ] Implement path classification and language-aware test-code findings,
  including lexical Rust `#[cfg(test)]` containment.
- [ ] Implement bounded marker scanning, per-file/count summaries, threshold
  comparison, and fail-closed unreadable-file handling.
- [ ] Implement stable human/JSON output, exit-code mapping, remediation text,
  and secret redaction.
- [ ] Integrate the checker with Workspace Governance audit and add explicit
  `.ai-gate` command bindings for the first migrated project set.

## 3. BFS — Cross-surface regression and completeness

- [ ] Re-run registry discovery, project adoption, workspace audit, and
  `auto_gate.py` to prove existing findings and deployment manifest behavior
  remain unchanged.
- [ ] Exercise full-scope and changed-scope scans, nested exclusions, symlink
  and path traversal boundaries, empty roots, and non-product documentation.
- [ ] Verify every migrated declaration has a native build/test command and a
  product-quality command; unverified projects remain explicit.
- [ ] Update README, project declaration schema guidance, migration notes, and
  the active roadmap with the exact evidence boundary.

## 4. Verification

- [ ] `python3 -m unittest discover -s tests -v`.
- [ ] `python3 scripts/workspace_check.py --self-check`.
- [ ] Run the checker against each migrated fixture and capture JSON/exit-code
  evidence for pass, fail, and review-required states.
- [ ] Run `node scripts/check-openspec-change-names.mjs` when available and
  `openspec validate --changes --strict --no-interactive`.
- [ ] Record unavailable project/toolchain checks with the exact next action;
  do not represent planning artifacts as enforcement evidence.
