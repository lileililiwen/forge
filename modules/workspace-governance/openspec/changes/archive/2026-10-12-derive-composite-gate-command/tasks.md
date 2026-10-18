# Tasks: Derive a composite command for a polyglot repository

## 1. BFS — Measure the refusals before changing anything

- [x] Read `gate-debt.json` and group all 44 unresolved checks by reason
- [x] Count how many share one cause: **24 checks in 11 repositories** are
      "N equally plausible entry points"
- [x] Confirm the shape of each, from disk, not from the record's prose:
      `crossify` holds `backend/CourseBridge.slnx` **and** `frontend/package.json`
- [x] Establish that these are not owner decisions: both halves of a polyglot
      repository must run, so the question is not "which one" but "both"
- **Done when** the count is a measurement. It is 24, not an estimate.

## 2. DFS — One coherent contract

- [x] `proposal.md` — the gap, the 11 repositories, and what is *not* changed
- [x] `specs/gate-command-binding/spec.md` — ADDED the composability
      requirement with five scenarios; MODIFIED the ambiguity requirement
- [x] `design.md` — why the tool had only two answers, what the third is, and
      why the test is "different stacks" rather than "two candidates"
- **Done when** `openspec validate derive-composite-gate-command --strict` passes.

### 2.1 Declare the stack, never parse it back out

- [x] `Candidate` gains a `stack` field
- [x] Every generator tags its own candidates: `dotnet`, `rust`, `node`,
      `python`, `flutter`
- [x] The composite decision reads the tag. It never parses `command`, and
      never reads `Shape.label()` — which reports one name for a polyglot
      repository and would have made the decision on the wrong information
- **Done when** no composite decision depends on a string.

### 2.2 The three answers, in order

- [x] One candidate → bind it, unchanged
- [x] A root-level candidate beats a nested one → bind the root, and do **not**
      compose. Dominance runs first, so `linglitch`'s nested `frontend/` does
      not become a second product
- [x] Candidates spanning two or more stacks → join with ` && `
- [x] Two candidates of the same stack → refuse, naming both. Two Flutter
      packages are one product described twice
- **Done when** all four paths are tested.

## 3. BFS — The subshell bug this change introduced, and fixed

- [x] Derive the composites and read them
- [x] **Find the defect before applying anything to the fleet.** A candidate
      may `cd` into its own directory, and `cd` is not scoped to one command
      in an `&&` chain. `alltools-platform` would have received
      `cd rust && cargo build --workspace && cd django_archived && npm run build`
- [x] Prove it in a real shell: `sh -c 'cd rust && pwd && cd django_archived'`
      → `can't cd to django_archived`. The second `cd` ran from inside `rust/`
- [x] Wrap every part in a subshell, uniformly, so the shape does not depend
      on whether a particular part happens to `cd`
- [x] Prove the fix: flat → exit 2, wrapped → exit 0, same directories
- [x] Pin it with `test_each_part_of_a_composite_runs_in_its_own_directory`,
      which runs both forms in a real shell. A string assertion cannot see a
      shell semantics bug, so the test does not use one
- **Done when** the negative control is in the suite, not in a comment.

## 4. BFS — Re-audit every consumer

- [x] `resolve()` has exactly one caller, so the new answer reaches every
      concern without a second site
- [x] `GateOracleTest` still holds. It pins the record set-equal to what the
      real gate reports as unbound, so a composite the gate cannot run turns
      it red rather than passing quietly
- [x] `FleetTest`'s path invariant runs over composite commands too, so a
      joined command naming a path that does not exist fails the suite
- [x] The three tests that asserted the old refusal are **converted, not
      deleted** — each now asserts the composite, and a new test keeps the
      same-stack refusal so the guard stays load-bearing
- **Done when** 137 tests pass, 0 fail.

## 5. BFS — Apply, and regenerate the record

- [x] Apply to the fleet: **10 manifests** written
- [x] Every referenced path checked to exist
- [x] Run one derived composite for real: `opendockify`'s
      `dotnet test OpenDockify.sln` + `python3 -m unittest`. The shell ran both
      halves. The .NET half fails on its own, before and after this change —
      a pre-existing `MSB4018`, not something this change introduced
- [x] `gate-debt.json` regenerated: **44 → 25 checks, 22 → 13 repositories**
- **Done when** the record and the gate agree, which `GateOracleTest` asserts.

### 5.1 A wrong fix, caught and reverted

- [x] The record was being written to the workspace root rather than the
      governance root, so a regeneration looked successful and left the
      authoritative copy stale
- [x] First fix: hard-code the script's own repository as the default path.
      **This was wrong and was reverted.** A default that ignores the `--root`
      the caller passed is not a default, and the first test run overwrote this
      repository's real `gate-debt.json` with one test repository's debt
- [x] The record follows `--root`; `--debt-out` redirects it. `GateOracleTest`
      passing is the evidence the record is right again, and the numbers are
      back to 13 repositories / 25 checks with no test data in the file
- [x] `test_debt_out_redirects_the_record` and
      `test_the_record_follows_the_root_argument` pin the contract, and the
      second asserts the real file was not touched
- **Done when** the mistake is in the record rather than only in this file.

### 5.2 The rule caught this repository's own debt

- [x] With the composite landed, `--self-check` went red:
      `QUALITY_NEW_CHANGE_WITH_GATE_DEBT: opens change
      'derive-composite-gate-command' while required check(s) security are
      unbound`
- [x] That is the rule from `bind-declared-gate-checks` working on the
      repository that wrote it, and the honest response is to close the debt,
      not to add an exception for it
- [x] The cause: this repository imported `jsonschema` and `PyYAML` in its
      tests with **nothing declaring them**, and declared no security tool. So
      the dependency set a `security` check would audit was not the one the
      tests actually used
- [x] Add `requirements-dev.txt` declaring `jsonschema==4.19.2` and
      `PyYAML==6.0.3` — the versions the suite is verified against — plus
      `pip-audit==2.10.1` as the tool itself
- [x] Install `pip-audit` and run it for real before binding:
      `No known vulnerabilities found`, exit 0
- [x] Bind `security: pip-audit`, then regenerate the record
- [x] Record: **13 → 12 repositories, 25 → 24 checks.**
      `--self-check` returns exit 0
- **Done when** the rule that blocks new work is satisfied by closing the debt,
      not by waiving it.

## 6. BFS — Spec, validation and verification

- [x] `openspec validate --all --strict` — 22 passed, 0 failed
- [x] `openspec validate derive-composite-gate-command --strict`
- [x] `python3 -m unittest discover -s tests` — **810 passed, 0 failed**
- [x] `scripts/workspace_check.py --self-check` — exit 0
- [x] The 20 remaining refusals are unchanged and still carry their reasons.
      Each is real work or an owner decision: a checker that does not exist, a
      package with no build step, several `.csproj` and no solution, or a
      Python security tool nobody declared
- **Done when** the numbers above are measurements.

## 7. BFS — Documentation and commit

- [x] `README.md` documents the generator, the three outcomes, the debt
      record, and the exit codes — the gap left open as task 3.7 of
      `bind-declared-gate-checks`
- [x] Commit 1: spec, design, tasks, `bind_gate_commands.py`, the converted and
      new tests, `gate-debt.json`, `README.md`, and the 10 sibling manifests
- [x] Commit 2: `HANDOFF.md` only
- [x] Explicit paths, never `git add -A`
- [x] On `main`. No branch, no worktree, no push.
- [x] Archive this completed change with the owner's authorization. The 20
      remaining refusals are separate recorded debt entries and do not leave
      this already completed composite-command change active.
