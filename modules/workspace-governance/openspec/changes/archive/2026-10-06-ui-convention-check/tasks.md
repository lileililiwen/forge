# Tasks: ui-convention-check

## 1. BFS — measure and pin

- [x] 1.1 Measure the UI census across the 81 registered repositories with the
  suffix set and prune set fixed in `design.md` §2: **59 repositories ship UI,
  4280 UI files**, 22 repositories hold **113** auth-named UI files,
  `alltools-platform` is the only repository over the 400 ceiling at **2067**.
- [x] 1.2 Confirm the adoption count is zero and no repository declares a `ui`
  block, so every declaration rule fires fleet-wide on day one and the baseline
  is what makes the gate live.
- [x] 1.3 Read the token boundary before writing a line of the checker:
  `ui/scripts/lib/token-model.mjs` exports `readTokens`, `resolveAliases`,
  `declaredLiterals`, and `resolveAliases` throws on an unresolved alias.
  Decide: call it, never re-implement it.
- [x] 1.4 Confirm the structural template: `scripts/product_code_quality.py`
  (fail-closed, exit 0/1/2, no writes, no per-run spawn) and
  `scripts/gate_debt_baseline.py` + `gate-debt-baseline.json` (the generator
  and snapshot pair).

## 2. DFS — implement the checker

- [x] 2.1 `scripts/ui_conformance.py`: constants (`UI_EXTENSIONS`,
  `SKIP_DIRS`, `AUTH_FILENAME_RE`, `THEME_MECHANISM_RE`, the eight codes,
  `DEFAULT_SCOPE_CEILING = 400`, `DEFAULT_TOKEN_SOURCE`,
  `DEFAULT_NODE_TIMEOUT = 30`), `ConfigError`, `Finding`, `Report`,
  `census()`, `parse_ui_policy()`, `load_declaration()`.
- [x] 2.2 Scope parsing: reject absolute entries, `..` segments and roots
  resolving outside the project, each as a `ConfigError`. Default `max_files`
  to 400; never read an environment variable for it.
- [x] 2.3 `UI_SCOPE_UNDECLARED` at `REVIEW_REQUIRED` when a shipping
  repository declares no `roots`, plus the over-ceiling note naming the exact
  count and the ceiling.
- [x] 2.4 `UI_AUTH_HANDROLLED` over the whole census, file-name match only,
  exempting `consume_paths`; **not** suppressible by adoption.
- [x] 2.5 Token resolver: one bounded `node` subprocess calling
  `declaredLiterals(resolveAliases(readTokens(source)))`; keep only
  `#hex`/`rgb()`/`hsl()` values; lowercase and expand 3-digit hex; every
  unavailable path (no `node`, no module, unreadable source, unresolved alias,
  timeout) fails closed with the literal reported and the cause named.
- [x] 2.6 `UI_COLOR_LITERAL` and `UI_THEME_ABSENT` read only inside the
  declared scope. Colour findings are bounded per file so one file cannot
  flood the report; the bound is recorded in the note, not hidden.
- [x] 2.7 `UI_TOKEN_NOT_CONSUMED`, `UI_BRANDING_ABSENT`,
  `UI_CONTRAST_UNMEASURED`, and adoption suppressing 5-7 but not
  `UI_AUTH_HANDROLLED`.
- [x] 2.8 Baseline application: covered code downgraded and counted, uncovered
  code still fails, `--baseline` optional so the default run fails closed.
- [x] 2.9 CLI `ui_conformance.py <project-dir> [--json] [--baseline ...]
  [--audit] [--declaration ...] [--tokens ...] [--root ...]`, exit 0/1/2, plus
  `--audit` fleet mode over `projects.json`.

## 3. DFS — the baseline

- [x] 3.1 `scripts/generate_ui_conformance_baseline.py`: collect one entry per
  shipping project carrying `ui_files`, the measured counts per code, and the
  code list; refuse to write an empty ratchet when no repository resolved, the
  failure mode `gate_debt_baseline.py` exists to prevent.
- [x] 3.2 `record_role()` for `alltools-platform` (`tooling`) and
  `dotnet-platform-libs` (`provider`) so their entries read as what they are,
  with `scope_oversize` set for the repositories over the ceiling.
- [x] 3.3 `render()` with `schema_version`, `generator`, `recorded_at`, `note`
  and entries keyed by project; `--check` reports drift and writes nothing.
- [x] 3.4 Write `ui-conformance-baseline.json` from the measured fleet state.

## 4. Tests

- [x] 4.1 `tests/test_ui_conformance.py`, unittest, scratch repositories built
  with `tempfile.TemporaryDirectory()`; no shell delete, ever.
- [x] 4.2 Scope parsing: valid, absolute, `..` escape, missing root, default
  ceiling, ceiling override.
- [x] 4.3 Census: suffix set, pruned directories, symlinked file skipped.
- [x] 4.4 Each of the eight codes fires in isolation.
- [x] 4.5 The combination: adoption declared **and** a hand-rolled login
  shipped still reports `UI_AUTH_HANDROLLED`, while 5-7 stay silent.
- [x] 4.6 A consume path exempts the shared package's own login screen; a
  directory named `authors` does not match.
- [x] 4.7 Colour literal: declared literal passes, `#FFF` normalises to
  `#ffffff`, an undeclared literal fires, and the token-source-unavailable path
  fails closed.
- [x] 4.8 Baseline: covered code suppressed, uncovered code still fails, drift
  detected.
- [x] 4.9 CLI exit 0 / 1 / 2 on three scratch repositories.
- [x] 4.10 The load-bearing proof: add a hand-rolled `LoginPage.tsx` to a
  conformant scratch repository, assert the run flips from 0 to 1.

## 5. BFS — verify

- [x] 5.1 `python3 -m unittest discover -s tests -v` (the declared command).
- [x] 5.2 `python3 scripts/ui_conformance.py <conformant scratch>` → 0.
- [x] 5.3 `python3 scripts/ui_conformance.py <non-conformant fleet repo>` → 1
  with real findings.
- [x] 5.4 Load-bearing proof outside the test suite: copy a scratch repo,
  add a login page, show exit 1, revert, show exit 0 again.
- [x] 5.5 `python3 scripts/workspace_check.py --self-check`.
- [x] 5.6 `openspec validate --changes --strict` and
  `openspec validate --specs --strict`.
- [x] 5.7 `node scripts/check-openspec-change-names.mjs`.
- [x] 5.8 `python3 scripts/check_skill_refs.py`.
- [x] 5.9 `python3 scripts/generate_ui_conformance_baseline.py --check` → 0.

## 6. Not done

- [ ] 6.1 Wiring `ui_conformance` into `validate_project()` in
  `scripts/workspace_check.py`. That file is not owned by this change; the
  checker is standalone and callable.
- [ ] 6.2 Migrating any repository onto the shared layer. Adoption is the
  separate `cross-repo-adoption-grant` change.
- [ ] 6.3 `skills/workspace-governance/**` and `docs/ui-convention.md`. Owned
  by a parallel agent.