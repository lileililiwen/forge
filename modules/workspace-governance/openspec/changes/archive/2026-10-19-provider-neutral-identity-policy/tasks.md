## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map the REG-005 requirement to decision-register fields, internal/public scope, consumers, and negative cases; preserve D2 history and verify D9 is the next permanent ID.
- [x] 1.2 Confirm the .NET, Forge, Rust, platform-contracts, and runtime owner boundaries in the package map; record no implementation authority for siblings.
- [x] 1.3 Confirm proposal, design, and spec agree on vendor neutrality, shared-admin identity, isolated product users, and public-site exclusion.
- [x] 1.4 Baseline the current register: ids D1–D8, every entry `accepted`, D2 `policy_boundary` text, `docs/toolchain-decisions.md` rows, and the register-lifecycle assertions in `tests/test_decisions.py`. Record the files that change.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add D9 as the provider-neutral OIDC decision, mark D2 superseded without rewriting its history, and retain D2's public-site exclusion in D9.
- [x] 2.2 Record that shared administrator authentication does not share application sessions or grant authorization.
- [x] 2.3 Record product end-user population isolation as the default and link downstream consumers to this policy.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Check all decision-register references for stale single-vendor requirements and confirm no runtime migration was introduced.
- [x] 3.2 Check public static-site exclusions and product-user isolation across governance documentation and consuming OpenSpecs.
- [x] 3.3 Confirm unrelated policy, identity provider code, and deployment configuration remain unchanged.

## 4. Verification

- [x] 4.1 Run the in-tree governance module's strict OpenSpec validation and repository policy audit; preserve exact command output.
- [x] 4.2 Review the resulting decision and canonical spec delta; the package is archived only after section 5 is evidenced.

## 5. Implementation — authorized surfaces (2026-10-19)

- [x] 5.1 `decisions.json`: set D2 `status` to `superseded` and leave its other fields byte-identical; append the D9 entry with the field values fixed by the design.
- [x] 5.2 `docs/toolchain-decisions.md`: mark D2 superseded, add the D9 table row and `## D9` prose section, and add the D9 register rule.
- [x] 5.3 `tests/test_decisions.py`: narrow the all-accepted assertion to the accepted lifecycle rule, and add D9 policy, D2 history-retention and D9 downstream-reference tests.
- [x] 5.4 Run `python3 -m unittest discover -s tests -v`, `python3 scripts/workspace_check.py --self-check`, `python3 scripts/workspace_check.py --decisions`, `openspec validate --all --strict --no-interactive` and `git diff --check`; record the exact output.
- [x] 5.5 Run the workspace `forge gate --dry-run` rehearsal and the full `forge gate`, and record the verdict.
- [x] 5.6 Archive without `--skip-specs` so REG-005 is promoted into `openspec/specs/workspace-decision-register/spec.md`, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Module test suite | PASS | `python3 -m unittest discover -s tests -v` ran 1031 tests, `OK (skipped=2)`. |
| Policy self-check | PASS | `python3 scripts/workspace_check.py --self-check` printed `Summary: errors=0 warnings=0`. |
| Register summary | PASS | `python3 scripts/workspace_check.py --decisions` lists D1–D9 with D2 `superseded` and D9 `accepted`. |
| Strict OpenSpec validation | PASS | `openspec validate --all --strict --no-interactive` reported `Totals: 27 passed, 0 failed (27 items)`. |
| Whitespace | PASS | `git diff --check` reported no findings. |
| Forge Gate | PASS | Dry-run resolved 8 required checks with none unbound; the full gate returned `aggregate: passed` for all 8 checks at revision `79158c13cf3c`, observed `2026-10-19T12:14:18Z`; evidence in `.forge/gate/forge/evidence.json`. |
