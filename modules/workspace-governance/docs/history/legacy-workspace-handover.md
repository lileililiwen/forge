# Historical handover (superseded)

This file preserves an older workspace snapshot. Its counts, uncommitted-file lists, commands, and next-step instructions are stale and must not be followed. Current repository state is recorded in the root `HANDOFF.md`, current policy in `AGENTS.md` and `.ai-rules/`, and current work in `openspec list`.

---

# Workspace architect handover — read this first

This is the state of an in-flight workspace governance refactor. It is not
done. The next agent inherits a working build that needs structural changes
to land, plus a backlog of small jobs and known traps. Read the whole file
before doing anything. Most of what follows is **not** in `git log` or any
test report — it is what was learned in the room and not committed.

> **The file exists in three places now** — `/tmp/HANDOVER.md`,
> `/home/paul/HANDOVER.md`, and `/home/paul/code/workspace-governance/HANDOVER.md`.
> The `/tmp` copy is what the user asked for; the other two are durable
> copies that survive a reboot or a `rm -rf /tmp/*`.

---

## 0. The one-paragraph brief

`/home/paul/code` holds **81 sibling git repositories**. One of them,
`workspace-governance`, is the governance repository. Another, `driftwatchdog`,
is the gate runtime. A third, `forge`, is the orchestrator. The other 78 are
products. The work of this and the next session is to make every product
conform to the governance this fleet already enforces in principle, by
**driving debt down to zero mechanically** rather than by parking decisions
on a person.

---

## 1. The standing rules — these are not negotiable

Every agent on this workspace is bound by these. The user is enforcing them
and will catch violations. Skim, do not skip.

1. **Work on `main`. Never create a branch or worktree.** No
   `git checkout -b`, no `git worktree add`. This rule is now in
   `AGENTS.md` of every repo.
2. **OpenSpec-first, two-commit protocol.** A spec change must exist
   *before* implementation. Commit 1 = spec + impl + tests + docs.
   Commit 2 = `HANDOFF.md` only. Then stop. Do not archive without
   authorisation.
3. **Never `rm`.** There is a `rm` shim at
   `/home/paul/.minimax/shims/rm` that prints to **stdout** and may
   fail. Use `Path.unlink(missing_ok=True)`,
   `shutil.rmtree(p, ignore_errors=True)`, or
   `tempfile.TemporaryDirectory()`.
4. **Never commit pre-existing uncommitted work.** Stage explicit
   paths, never `git add -A`. The fleet has dozens of dirty trees owned
   by other things — leave them.
5. **Never fabricate attribution.** No `Co-Authored-By` trailers.
6. **Never weaken a test to make it green.** A red gate is the correct
   report; filtering it out is the bug this workspace keeps catching.
7. **Never trust a subagent's report.** Verify independently, including
   negative controls. Specifically: re-run the failing case the report
   claims to have fixed.
8. **Do not push unless asked.** The user commits and pushes; you do
   not.
9. **Never write secrets.** No tokens, PATs, passwords in files or
   commits.
10. **`cd` explicitly in every shell call.** The cwd resets between
    calls. This caused many wrong-directory failures.
11. **OpenSpec CLI quirk:** never `openspec list --changes --strict`. Use
    `openspec list --changes` and `openspec validate --changes --strict`.
12. **`driftwatchdog` on PATH is a stale 2026-10-01 build that reports
    0.** Never use it as an oracle. Use
    `driftwatchdog/target/debug/driftwatchdog` after `cargo build`.
13. **The user is committing concurrently.** Check `git log` for mid-run
    commits. Never revert them. They are not yours.
14. **Reports are short, plain English, classified.** The user reads
    English as a second language. Done / Needs you / Blocked / Next.
    One line each.

The user profile adds: never ask the user about resolvable engineering
problems. Decide, act, report. `ask_user` is disabled.

---

## 2. The fleet at a glance

| Repo | Role | Notes |
|---|---|---|
| `workspace-governance` | governance tool itself | what you usually edit |
| `driftwatchdog` | gate runtime (Rust) | profile/concern logic lives here |
| `forge` | orchestrator | writes the fleet, runs gates |
| 78 products | the fleet being governed | see Section 7 for current pass/fail |

The fleet gate runs per-repo via
`driftwatchdog/target/debug/driftwatchdog gate --project <name>`. Run
cold-cache builds at **parallelism 2** on this 14GB box — four concurrent
.NET builds get OOM-killed. The measuring harness is
`/tmp/runfleet.sh`.

---

## 3. What this session shipped (committed)

All on `main`, never pushed. The user is the publisher. The user
committed several of these during the conversation; the agent did not.
Verify with `git log` in each repo.

### 3.1 `workspace-governance`

| Commit | What it does |
|---|---|
| `0611298` | The new-change-with-gate-debt rule. Blocks a repo with debt from opening a new change. Reads `gate-debt.json`, `gate-debt-baseline.json` (ratchet for pre-existing work), and `gate-debt-exceptions.json` (recorded escape). All three fail closed. |
| `d9a9c25` | HANDOFF only. |
| `9105360` | Hardens the promote-test fixture so it cannot fall behind a new required governance file. |
| `452ae8f` | HANDOFF only. |
| `aa87592` | The composite command. A polyglot repo (e.g. .NET backend + npm frontend) joins the two entry points with `&&` wrapped in subshells, instead of refusing as ambiguous. |
| `228109d` | HANDOFF only. |
| `1d19cc2` | Anchors the TypeScript test-code match on the qualifier. `/re/.test(v)` is a regex, not a test. 15 false positives in 4 repos → 0. |
| `8b7733b` | HANDOFF only. |
| `efd060c` | Corrects the suite-count claim in HANDOFF (812, not 813). |
| `22638ae` | Admits cross-cutting shared ids in every profile (workspace-governance capability work — see Section 5.4). |
| `d34ec55` | HANDOFF only. |
| `af095e8` | Drives legacy gate debt repairs — the `close_gate_debt.py` driver, the `check_concern.py` checkers, and the matching tests. (Committed by the user after the agent left them in `??`.) |
| `483d912` | HANDOFF only, with the legacy-debt-repair evidence. |

### 3.2 `driftwatchdog`

| Commit | What it does |
|---|---|
| `546df58` | Declares `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` on the `governance-quality` concern so the gate can see the rule. |
| `6f0691d` | HANDOFF only. |

### 3.3 Test counts

| Suite | Result | Note |
|---|---|---|
| `workspace-governance` | 812+ passed | exact count may have changed; run `--dry` to confirm |
| `driftwatchdog` | 909 passed, 0 failed | the +1 is the declaration test |

---

## 4. What is uncommitted right now

### 4.1 `driftwatchdog`

```
 M src/commands/gate.rs
 M src/gate/aigate.rs
 M src/gate/manifest.rs
?? openspec/changes/mandatory-governance-quality-all-profiles/
```

The manifest.rs change is real and substantial — it is **not** the simple
one-line arm the agent drafted before stopping. It:
- Adds the 10 workspace profile names (`typescript-product`,
  `python-product`, `dotnet-product`, `dotnet-library`,
  `typescript-monorepo`, `rust-platform`, `tooling`, `deployment`,
  `inventory`, `governance`) as built-in profiles returning
  `Some(&[GOVERNANCE_QUALITY])`.
- Adds the same 10 names to `SUPPORTED_PROFILES`.
- Augments `profile_defaults_for` so **every** profile's defaults
  receive `governance-quality` automatically, including
  project-declared profiles from `[profiles.<name>]` sections. This is
  the better design — a new project with a brand-new profile name
  still gets governance for free.

**This change is built.** `target/debug/driftwatchdog` is dated
`2026-10-05 21:46:15`, after the manifest.rs change at `21:45:19`.

**This change is not committed.** It needs:
- The spec to be finished
  (`openspec/changes/mandatory-governance-quality-all-profiles/` has
  `proposal.md`, `design.md`, `tasks.md`, and a `specs/` dir — verify
  it is complete, and `openspec validate --changes --strict` passes).
- `cargo test` to pass on it
- Two commits on `main` (spec + impl + tests + docs, then `HANDOFF.md`
  only).

`src/commands/gate.rs` and `src/gate/aigate.rs` are also touched. The
agent did not write them. Read them; they are likely the test or harness
adjustments the mandatory change requires. Do not revert them.

### 4.2 `workspace-governance`

```
?? driftwatch.toml
```

`driftwatch.toml` is a known parked item that has been untracked for the
whole session. It belongs to no current change. Leave it untracked.

### 4.3 Sibling repos

The three empty `App.slnx` files in `aegify`, `hestiaLab`, and `smotoox`
from the first draft of `close_gate_debt.py` are gone — the agent
removed them with `Path.unlink(missing_ok=True)` during the cleanup
before the user committed `af095e8`. Confirm:

```
ls aegify/App.slnx hestiaLab/App.slnx smotoox/App.slnx 2>&1
```

If any of them is still there, delete it. A solution with no projects
is a lie.

Other untracked files in sibling repos (`scripts/check-*.py`,
`requirements-dev.txt`, the `post-mvp-readiness/` change dirs,
`Cargo.toml.bak-profile`, etc.) belong to the user or to in-flight
work. **Do not touch them.** Stage explicit paths only when the
matching spec exists.

---

## 5. The four requirements and where each stands

From the most recent user message. These are the four things the user is
asking for, in priority order. All four must be done to call the
mechanism complete.

| # | Requirement | Status |
|---|---|---|
| 1 | Legacy projects must be able to fix governance (a driver, not just a stop) | **DONE** — `close_gate_debt.py` shipped in `af095e8`, with tests |
| 2 | New projects must comply from day one | **NOT DONE** |
| 3 | It must be mandatory, not opt-in | **UNCOMMITTED BUT BUILT** — the `_ => Some(&[GOVERNANCE_QUALITY])` arm is in `manifest.rs` but uncommitted |
| 4 | It must be dynamic | **PARTLY DONE** — debt is read from disk, but the ratchet (a repo that gains debt fails immediately) is not yet enforced |

### 5.1 Legacy debt driver — what was shipped

`scripts/close_gate_debt.py` reads `gate-debt.json`, classifies every
entry by its reason string, and applies the remedy:

- **A** (10 checks): writes a real checker into the repository. The
  binder's file-gated rule binds `scripts/check-<concern>.*` the
  moment the file exists.
- **B** (7 checks): creates the solution and adds every project.
- **C** (3 checks): writes a `[build-system]` table.
- **D** (2 checks): writes a `requirements-dev.txt` naming
  `pip-audit`.
- **E**, **F** (2 checks): refuses and asks — those are the genuine
  decisions, and they remain on the record.

Current state after `af095e8`:

| Repo | Concerns left |
|---|---|
| `hestiaLab` | `build`, `security` |
| `jenkins-local` | `build` |
| `sisyphusfy` | `build`, `security` |

That is **5 of 24** remaining. 19 are closed.

### 5.2 Why the 5 are still open — they are real decisions

- `hestiaLab/build` and `hestiaLab/security`: 2 csproj and no solution.
  The driver attempted to create `App.slnx`; the first version
  reported success without verifying the projects were added. The
  bug was fixed, but the empty `App.slnx` files were removed before
  re-application. Either re-run the driver now that the verification
  is correct, or ask the owner to create the solution.
- `jenkins-local/build`: a pure-Python package has no build step.
  Genuinely ambiguous — the owner picks the packaging command.
- `sisyphusfy/build`: no `package.json` declares a build script.
  Genuinely ambiguous — the owner picks what this repository builds.
- `sisyphusfy/security`: 2 equally plausible `npm audit` entry
  points (`.opencode/`, `npm/`). Genuinely ambiguous — the owner
  picks which is the product.

### 5.3 New-project compliance — what to build

`init_project.py` still writes a template gate.yaml with 4 checks
selected and no commands. An empty repo has no source for the binder to
derive from. The current behaviour is a deadlock: the new repo cannot
start a feature because it has unbound checks, and it cannot bind the
checks because it has no source.

**Recommended fix.** Make `init_project.py` run
`bind_gate_commands.py --apply` immediately after creating the
manifest, then re-render the manifest with whatever the binder
derived. An empty repo will have nothing to derive, but the call is
in place. When the repo later adds source, the next binder run binds
what now exists.

Also: add `--profile` validation that rejects profile names the
binder doesn't know. A typo like `typescript-produc` should fail the
scaffold, not silently schedule `governance-quality` for an unknown
profile.

### 5.4 Mandatory — what is in the uncommitted driftwatchdog change

`src/gate/manifest.rs` has a real fix:
- The 10 workspace profile names are added as built-in profiles.
- `profile_defaults_for` adds `governance-quality` to **every**
  profile's defaults automatically.

Build it. Test it. Commit it with two commits on `main`.

### 5.5 The ratchet — still missing

The debt record is a snapshot. A repo that gains debt between runs
(profile change, new manifest check, etc.) does not fail until the
next regeneration. The fix is in `validate_new_change_with_gate_debt`:
- Read the gate's current state for the project at run time.
- Compare to the recorded state.
- Fail closed if the project now has *more* debt than the record
  shows.

Pin with a regression test: a repo recorded as clean is given a new
unbound check in flight (by editing the manifest without the binder),
and the rule must fire on the next new change.

---

## 6. The exact next steps, in order

### Step 1 — finish the mandatory-governance-quality commit (req 3)

```
cd /home/paul/code/driftwatchdog
# 1a. Verify the spec is complete
openspec validate --changes --strict
# Expected: "✓ change/mandatory-governance-quality-all-profiles"
# If anything fails, read it and fix it before going further.

# 1b. Build and run the existing suite
cargo build --release 2>&1 | tail -10
cargo test --quiet 2>&1 | grep -E "^test result" | tail -5

# 1c. Confirm the gate now schedules governance-quality for every profile
./target/debug/driftwatchdog gate --project actoria 2>&1 | grep -E "governance-quality|UNBOUND"
./target/debug/driftwatchdog gate --project sisyphusfy 2>&1 | grep -E "governance-quality|UNBOUND"

# 1d. Stage explicitly and commit
git add src/commands/gate.rs src/gate/aigate.rs src/gate/manifest.rs openspec/changes/mandatory-governance-quality-all-profiles/
git status --short   # confirm exactly the four paths are staged
git commit -m "feat(gate): admit cross-cutting shared ids in every profile

..."  # describe, don't promise
# Then commit 2: HANDOFF only.
git add HANDOFF.md
git commit -m "HANDOFF: ..."
```

### Step 2 — drive the last 5 to a real decision (req 1, finishing)

After step 1 lands, the binder will re-classify every project. For the
remaining 3 repos:

- `hestiaLab`: re-run the driver now that the `App.slnx` verification
  is correct. If the solution ends up empty, delete it and ask the
  owner.
- `jenkins-local`, `sisyphusfy`: these are owner decisions. Write them
  up in HANDOFF with the exact question and the `gate-debt.json`
  entry pointing to it. Do not invent commands.

### Step 3 — new-project compliance (req 2)

See Section 5.3. Make `init_project.py` run the binder after scaffold.

### Step 4 — the ratchet (req 4)

See Section 5.5. The change is in
`validate_new_change_with_gate_debt` plus a regression test.

### Step 5 — the one-line summary

When the session is truly finished, this number must be **0**:

```
python3 /home/paul/code/workspace-governance/scripts/close_gate_debt.py \
  --root /home/paul/code --dry
```

Plus 0 repos in `gate-debt.json`'s `repositories` list (other than the
2 owner-decisions that remain). Until both are true, the work is not
done.

---

## 7. The reports that already exist (do not duplicate)

- `/tmp/fleet_results.txt` — last full-fleet gate measurement,
  pipe-delimited `repo|exit|duration|summary|failing-checks`. The
  pre-`af095e8` snapshot showed 19 PASS, 44 FAIL, 7 no-manifest. Run
  again after step 1 to refresh.
- `/tmp/runfleet.sh` — the harness.
- `/tmp/debt_new.json`, `/tmp/debt_backup.json` — scratch from the
  binder debugging. Ignore.

---

## 8. Things to never say

- "I'll follow up" — if there is a follow-up, do it now.
- "Looks good" — without a measurement, the next agent inherits the
  guess.
- "Probably fine" — same.
- "Should work" — never. Run it. Read the output. Quote it.
- "The user can decide" — if the choice is yours, make it; if it is
  theirs, ask once, briefly, and accept the first answer.

---

## 9. The single piece of advice that matters

When something fails and you do not know why, **do not patch the
symptom and move on**. The first three bugs I introduced in
`close_gate_debt.py` were all of the same shape — a wrong default, a
wrong exit-code assumption, a wrong file-format count — and I fixed
each in turn while the next one was waiting. The cost was the user's
attention and a half-finished mechanism. Read the error message, find
the actual mistake, fix it once, verify with a control, then move on.

---

## 10. Where the human feedback came from — for context

The user said: "do not write here, all your tip should write into
HANDOVER because this conversation will be closed." They mean: do not
add another long chat reply on top of this. Every tip belongs in this
file. If a new tip emerges from a new session, append it here; do not
start a new conversation-thread reply that re-states what is already
here.

The next agent will be picked up from this file alone, not from a chat
scrollback. The earlier agents (and the user) will not see your reply.
Write the way you would write a chapter, not the way you would answer a
ticket. Sections 1–9 are the chapter; section 10 onward is your
appendix. Stay short, stay testable, stay honest.

---

## 11. The three traps the next agent is most likely to fall into

### Trap 1 — "The test passes, ship it"

Two examples from this session:

- The TypeScript checker was matching
  `re.search(r"\b(describe|it|test)\s*\(")` with no qualifier check.
  I added the qualifier and it "passed" the suite. I did not measure
  against the real repositories until **after** I had decided the
  change was done — and the measurement found 15 false positives I
  would have shipped.

- The first version of `close_gate_debt.py`'s class-B remedy
  reported `dotnet new sln` exit 0 as success, did not check that
  the solution existed, did not check that the projects were added,
  and counted `Project(` in the file (the old `.sln` format) when
  the SDK now writes `.slnx`. The "test" for it was a dry-run
  message saying "done".

**The rule:** the only test that matters for a checker or a
remediation is "does it find what is real and miss nothing that is
not." A green suite is not that test. A control run against a real
fleet-state is.

### Trap 2 — "The exit code is non-zero, so it failed"

`dotnet sln add <bad-path>` exits 0 with an error message in the
output. The binder and the driver have to read the message text, not
the exit code. Pin a regression test that proves it:

```
$ dotnet sln App.slnx add does-not-exist.csproj
$ echo $?
0
```

If you ever see `dotnet` exit 0 on a missing path, trust the
message, not the code.

### Trap 3 — "I will fix the documentation to make the check pass"

The temptation arises when a checker reports a finding that is true
but uncomfortable (missing `THIRD-PARTY-NOTICES`, a denied
e-signature seam in a README). Editing the repo's documentation to
silence the check is the same defect as an unbound check wearing a
costume. Surface the finding to the user, file it as a real defect
to fix, and do not touch the documentation.

---

## 12. The two things to verify **before** writing the OpenSpec spec

The next agent will be tempted to write the spec first because that is
the standing rule. The standing rule is right, but writing the spec
before measuring is the trap. Two measurements are non-negotiable
first:

1. **The current 5 entries classified.** Run the driver in dry-run
   mode and confirm every entry resolves to one of {A, B, C, D, E, F}.
   The current 5 are: hestiaLab/build, hestiaLab/security
   (both class B), jenkins-local/build (class C),
   sisyphusfy/build (class F), sisyphusfy/security (class E). If
   that changes (a binder reason string is reworded, a new refusal is
   added), the classifier must catch up **before** the spec claims
   it covers everything.

2. **The empty `App.slnx` files handled.** A solution with no
   projects is not a solution; it is a lie. Either the file contains
   every project on disk, or the file does not exist. Do not commit
   the intermediate state.

---

## 13. The one-line summary of the open work

```
finish the mandatory-governance-quality commit (it is built, not committed),
decide or drive the last 5 debt entries to a real answer,
make new projects comply at scaffold time,
make the new-change rule read live state not the snapshot,
land everything on main with two commits per change.
```

If the next agent reads no further than this section, they know the
shape. If they read sections 1–9, they know the details. If they read
sections 10–12, they know the traps. The order is intentional.

---

## 14. Where the file is kept (so it is not lost)

This file is in three places:

1. `/tmp/HANDOVER.md` — what was asked for.
2. `/home/paul/HANDOVER.md` — durable copy under the user's home.
3. `/home/paul/code/workspace-governance/HANDOVER.md` — in the
   governance repo, durable, version-controlled.

If the user asks again for the file, point them at the third
location. It survives reboots, `rm -rf /tmp/*`, and a fresh checkout.

## 15. Continuation status — 2026-10-05

Five governance packages are implemented on `main`, each with its OpenSpec and
two-commit protocol: mandatory governance quality (`9426fed`, `9477457`),
legacy debt repair (`af095e8`, `483d912`), scaffold binding (`e17615d`,
`471d800`), live debt ratchet (`8ab77e6`, `f897a91`), and dirty-manifest
classification (`ab466b3`, `0c00b39`). All remain active and unarchived.

The original 11-entry debt snapshot covered 8 repositories. The full binder
apply had also left governance-quality changes pending in sibling manifests;
those changes and 20 other dirty sibling manifests were preserved until the
user explicitly requested their resolution. Five repositories had no
manifest. The latest pre-resolution workspace check reported 849 tests passed,
1 skipped, strict OpenSpec 14/14, and clean self/project checks.

Do not commit the pre-existing untracked `workspace-governance/HANDOVER.md` or
`driftwatch.toml`.

## 16. Gate debt closure follow-up — 2026-10-06

The remaining 11 recorded entries were resolved in the existing active
`close-unbound-gate-checks` OpenSpecs, one repository at a time. Each product
repo has an implementation/spec commit followed by a HANDOFF-only commit:

| Repository | Implementation | HANDOFF |
|---|---|---|
| `hestiaLab` | `d33995c` | `40d058f` |
| `jenkins-local` | `7b170e7` | `6fa8727` |
| `sisyphusfy` | `8e66b5d` | `75948c7` |
| `alethefy` | `4c97a73` | `f14d2c2` |
| `ariadex` | `07b93e3` | `e24e255` |
| `catchen` | `27c9cc9` | `d3b2bbf` |
| `linglitch` | `4bc4cef` | `567a045` |
| `platform-contracts` | `1c95354` | `2d62fce` |

Jenkins-local build compiles Python adapters; its product image builds remain
on the Mac runtime. Sisyphusfy builds its root Python distribution and audits
the published npm launcher; ignored `.opencode/` package files are outside
clean-checkout product scope. Catchen and Linglitch have sequential commands
covering backend and frontend tests, but those suites were not run in this
pass. Platform Contracts schema and fixture checkers passed with no findings.

The authoritative snapshot is `workspace-governance/gate-debt.json`, refreshed
with an explicit `--debt-out` path. The debt driver reports **0 entries and 0
decisions needed**. Snapshot/task commit: `829c1be`; HANDOFF-only commit:
`5d59491`. Workspace scan: 81 repos, 49 resolved, 0 unresolved, 5 without
manifests, 27 with other dirty manifest work preserved. No changes were
archived or pushed.

Verification correction — Alethefy's build was run with
`UV_CACHE_DIR=/tmp/alethefy-uv-cache uv run --with build python -m build`.
uv selected Python 3.14.4, satisfying `>=3.12`, and produced both the sdist
and wheel. The gate now runs the build through uv. Alethefy follow-up commits:
`03d7d03` and HANDOFF-only `fc0c01e`. Platform Contracts' pip-audit could not
establish an advisory result through the restricted PyPI proxy; that live
result remains unverified. HestiaLab's workspace checker still reports three project
metadata errors: missing LICENSE, discoverability metadata, and verified
publication evidence. Its build, API tests (146), web tests (129), and
vulnerability scan passed during earlier work.

No OpenSpec changes were archived and no commits were pushed.
