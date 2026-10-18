#!/usr/bin/env python3
"""Bind every gate check a repository declares to a command derived from disk.

Ruling D18 in this workspace: *a verifier that no gate runs does not gate;
wire every check into the gate that blocks, or it is documentation.* Measured
on 2026-10-04, 76 of 81 repositories declared gate checks, 69 of them declared
at least one required check with no command bound, and 283 required checks were
unbound in total. The Gate's own wording for that state is `UNBOUND` — "no
command declared for this concern; no execution was attempted" — and it counts
the project as failing.

`bind_verification_command.py` closed the same gap for the one concern whose
command a repository declares for itself. It cannot close this one, because a
`verification.command` is not a per-concern command: `kairosa`'s is
`openspec validate --all --strict --no-interactive`, which is not a test
command, and it is already bound there. This tool is about the other concerns.

## Why this walks the disk and never reads the label

`.project.json` `stack` disagrees with the disk for 27 of the 81 repositories
by shape alone. `linglitch` declares `typescript` and carries a .NET solution
under `backend/` with its Node manifest under `frontend/`. `alltools-platform`
declares `python` and has no Python at its root. `loreMix` declares `typescript`
and has `LoreMix.slnx` at its root. 19 repositories carry **no `*.sln` at all**
and use `*.slnx` instead, which a `.sln` glob cannot see.

So `stack` is not an argument of any function here. It is not read, and it
cannot leak in later.

## What the gate selects, and why the debt record had to learn it

A manifest selects a check two ways: by listing it under `checks:`, and by
naming a `profile:` whose default set the gate schedules anyway. The gate
fails on an unbound *required* check either way. This tool originally read
only the first, so a repository whose only unbound checks came from its
profile dropped out of `repositories` altogether — 30 repositories and 122
checks, while the gate was failing on all of them. A debt record naming 48
of 170 gaps reads as complete, which is worse than no record at all.

`PROFILE_DEFAULTS` and `gate_required` now mirror the gate's own resolution,
and `GateOracleTest` in `tests/test_bind_gate_commands.py` checks that
mirror against the real binary: the (repository, concern) pairs in this
record MUST equal the set `driftwatchdog gate --dry-run` reports as unbound.
The table is a copy of `profile_defaults` in
`driftwatchdog/src/gate/manifest.rs`; the test is what keeps the copy honest,
and it skips rather than fails when the binary has not been built.

A check only the profile scheduled is **derived first**. The gate's
`.ai-gate` reader refuses a manifest whose `commands:` names a check the
manifest does not select (`aigate.rs:213`), so this tool used to record such
a check as debt and write nothing. That was the right refusal guarding the
wrong boundary: a repository naming `profile: product` has already committed
to the concerns that profile schedules, and the gate already fails it for not
binding them, so the correct write is **both** a `checks: <id>: true` entry
and a `commands: <id>` entry. It is done in one operation, and only when a
checker is genuinely reachable — see `governance_concern`. Where none is, the
concern stays in the debt record, because a selection with no command behind
it is a worse state than the gap it replaced.

## The gate protocol, and which checkers can satisfy it

`driftwatchdog/src/commands/gate.rs:206-208` builds
`AdapterInput { program: "sh", args: ["-c", cmd], working_dir: proj.root }`
and `checker/runner.rs:103` gives the child `Stdio::null()`. **A bound command
gets no stdin.** The environment is not cleared (`runner.rs:99`), so it
inherits this process's. A command must therefore work from argv and the
inherited environment and report through stdout and its exit code.

That protocol is what decides the producers:

* `workspace_check.py --json` emits `{findings: [{code, level, message,
  project}], summary}` — exactly `parse_report`'s shape at
  `governance/mod.rs:141`, with exit 0/1 expected. **Bound.**
* `product_code_quality.py` is fail-closed on a `quality` block and exits 2
  without one. It emits `schema_version`, not the `version` `parse_envelope`
  requires (`adapters.rs:1040`), so it always takes the text-mode exit-code
  path — verified, not assumed. **Bound, and the `quality` block it reads is
  derived from disk** — see `derive_quality_block`.
* `forge_governance_adapter.py` reads `sys.stdin.read()` and emits an
  observation (`status`/`detail`/`evidence`), not a findings report. With no
  stdin it reads an empty input, and the report parser would not recognise the
  output. **Never bound** — it is a one-input adapter, not a checker.

`workspace_check.py --root ..` passes the workspace root explicitly on
purpose. Its default is the directory holding `projects.json`, which is this
repository, and from a sibling that default makes all 81 registered projects
report `PROJECT_MISSING` against the sibling's own subdirectories. 81 false
findings is not a pass.

## What it refuses to do

* **Never invents a command.** An unknown shape, a `package.json` without the
  script the concern needs, an undetectable test runner, a pure-Python
  `build`, and two equally plausible entry points are all *unresolved*, with
  the reason and the evidence. A wrong command is worse than an honest gap.
* **Derives policy where policy is derivable, and judges only where it must.**
  `placeholder-threshold` and `product-code-boundary` need a `quality` block
  in the repository's own `.project.json`. Its `product_roots` and
  `languages` are facts about where the code is, so they are derived from
  disk with the manifest each came from attached as evidence. Its
  `placeholder_threshold` is a judgement, and is written as `0` — the
  deliberately strict default both hand-written blocks chose, which a project
  may relax on purpose in its own file. The block and the bound command are
  written in one operation, and a hand-written block is never touched.
* **Never invents an `exclude` list or a marker list.** Both are judgements
  about what is not product, and a guessed exclusion hides a real finding
  rather than reporting one. The marker vocabulary is the checker's own.
* **Never names a product root it did not see.** A root is derived only from a
  manifest on disk whose `src` directory exists and holds a file of that
  manifest's language. No root means refusal by name, not a fallback to `.`.
* **Never binds a concern the gate does not require.** Only a check the
  manifest selected, or one its `profile:` scheduled and it did not disable,
  exempt, or already bind. A disabled, exempted, `"optional"`, or
  unselected check is left alone whether or not a command could be derived.
* **Never overwrites an operator's binding.** An existing `commands.<concern>`
  that differs from the derived one is a `CONFLICT` and is left exactly as it
  is. An identical one is `ALREADY_BOUND`.
* **Never mines prose.** A command written in a manifest's `notes:` is not
  recovered from it; a check named in free text is not a declaration this tool
  can act on.
* **Never writes without `--apply`.** The default is a dry run.
* **Never exits clean while debt remains.** Exit 3, with no flag to suppress it.

## Idempotence

The second run over an already-bound repository derives the same commands,
finds them already present, and writes nothing, so the file is byte-for-byte
unchanged. The generated comment is written once, guarded by its own marker
text, so a partial binding does not accumulate a second copy of it.

## Deletion

This host's `rm` is a recoverable-delete shim that prints to stdout and can
fail. Nothing here deletes anything.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from bind_verification_command import (  # noqa: E402
    entry_value,
    discover_repos,
    is_dirty,
    top_level_block_bounds,
    yaml_scalar,
)

PROJECT_JSON = ".project.json"
GATE_YAML = ".ai-gate/gate.yaml"
DEBT_FILE = "gate-debt.json"

# One line a reader can grep for to tell generated policy from hand-written
# policy without opening this file. Doubled quotes stay literal inside a
# double-quoted Python string only when escaped, so it is written plainly.
GENERATED_MARKER = "workspace-governance/scripts/bind_gate_commands.py"
GENERATED_COMMENT = (
    f"  # Generated by {GENERATED_MARKER} from the files on disk in this\n"
    "  # repository, for the concerns this manifest already selects under\n"
    "  # `checks:`. A verifier that no gate runs does not gate (ruling D18).\n"
    "  # Re-run the generator to update these; do not hand-edit. Concerns it\n"
    "  # could not derive are recorded in gate-debt.json, not guessed.\n"
)

# Directories that hold a dependency's files rather than this repository's.
# `alethefy`'s only `conftest.py` is `.venv/…/site-packages/numpy/conftest.py`;
# reading it would make a vendored file look like pytest configuration.
SKIP_DIRS = frozenset(
    {
        ".git",
        "node_modules",
        "target",
        "bin",
        "obj",
        ".venv",
        "venv",
        "dist",
        "build",
        ".next",
        ".nuxt",
        "site-packages",
        ".dart_tool",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        "__pycache__",
        ".gradle",
        "vendor",
        "TestResults",
    }
)

MAX_DEPTH = 6

SOLUTION_SUFFIXES = (".sln", ".slnx")
PROJECT_SUFFIX = ".csproj"

# Per-concern status.
BOUND = "BOUND"
ALREADY_BOUND = "ALREADY_BOUND"
CONFLICT = "CONFLICT"
UNRESOLVED = "UNRESOLVED"

# Per-repository roll-up.
RESOLVED = "RESOLVED"
PARTIAL = "PARTIAL"
NO_MANIFEST = "NO_MANIFEST"
DIRTY = "DIRTY"
REFUSED = "REFUSED"

# A concern is only debt when it has no command at all. A `CONFLICT` has a
# command: a person wrote it, the gate will run it, and this tool leaves it
# alone. Counting that as debt would report 123 wired checks as unbound,
# which is the opposite of what the Gate sees.
NOT_RESOLVED = (UNRESOLVED,)

# Concerns whose command is a repository-wide property rather than a
# per-language one. `labrys` binds its `repository_integrity` to exactly
# this, and `therapist-commons` binds its `repository` to the same command.
REPOSITORY_CONCERNS = ("repository", "repository_integrity")

# The three concern ids three repositories use for the same OpenSpec change-name
# preflight. One script, three labels, because the labels are per-repository
# vocabulary and the command is the same file in all of them.
OPENSPEC_NAME_CONCERNS = (
    "openspec_names",
    "openspec_change_names",
    "openspec_name_preflight",
)

OPENSPEC_NAMES_SCRIPT = "scripts/check-openspec-change-names.mjs"

# Concern id -> repository script, for the checks a repository implements as its
# own script rather than as a build-system command. The path is exact; a
# repository that has not written the script does not get a command.
SCRIPT_CONCERNS = {
    "product_quality": ("python3 scripts/product_code_quality.py .", "scripts/product_code_quality.py"),
}

SCRIPT_SUFFIXES = (".sh", ".mjs", ".js", ".py")

# The concerns a build system answers. A concern outside this set is a
# repository's own check, and is looked for as a script instead.
SHAPE_CONCERNS = ("build", "tests", "security", "format", "lint")

# --- what a `profile:` selects -----------------------------------

# The concerns a built-in gate profile schedules on its own, mirroring
# `profile_defaults` in driftwatchdog's `src/gate/manifest.rs` (checked
# against the built binary by `GateOracleTest`, which fails if this table
# and the gate ever disagree).
#
# This is the selection a manifest makes by naming a profile rather than by
# listing a check. `aesthra` lists six concerns and says `profile: product`;
# the gate then schedules four more it never wrote down, and fails on the
# three that bind no command. A binder that reads only the `checks:` block
# cannot see them, which is exactly how 30 repositories whose only unbound
# checks were profile-scheduled went missing from the debt record while the
# gate was failing on them.
PRODUCT_PROFILE_CONCERNS = (
    "governance-quality",
    "placeholder-threshold",
    "product-code-boundary",
    "source-file-size",
)
RELEASE_GATE_CONCERNS = ("capability-conformance", "release-evidence")
DEPLOYABLE_CONCERNS = ("ci-contract", "compose-contract")

PROFILE_DEFAULTS: dict[str, tuple[str, ...]] = {
    "backend": ("api-contract", "migration", "secret-scan"),
    "frontend": ("a11y", "api-contract", "responsive"),
    "full": ("a11y", "api-contract", "migration", "responsive", "secret-scan"),
    "minimal": (),
    "product": PRODUCT_PROFILE_CONCERNS,
    "rust-product": PRODUCT_PROFILE_CONCERNS,
    "release": RELEASE_GATE_CONCERNS,
    "deployable": DEPLOYABLE_CONCERNS,
    # Workspace profiles declared by Driftwatchdog. Their extra checks are
    # supplied by the mandatory governance-quality default at runtime.
    "typescript-product": ("governance-quality",),
    "python-product": ("governance-quality",),
    "dotnet-product": ("governance-quality",),
    "dotnet-library": ("governance-quality",),
    "typescript-monorepo": ("governance-quality",),
    "rust-platform": ("governance-quality",),
    "tooling": ("governance-quality",),
    "deployment": ("governance-quality",),
    "inventory": ("governance-quality",),
    "governance": ("governance-quality",),
}

# The gate runs this one itself, in-process, and never counts it unbound
# (`is_source_file_size_concern`). Deriving a command for it would hand the
# gate a binding for a check it never asks a command for, and recording it
# as debt would name a failure that does not exist.
IN_PROCESS_CONCERNS = frozenset({"source-file-size"})

# The .NET command for each concern, given the resolved entry point. The
# dependency-audit form matches the one .NET security binding already
# proven in this fleet (`therapist-commons`), minus `--format json`, which
# needs SDK 9.0.200+ and would be a portability hazard for a repository
# this tool has not read a `global.json` for.
DOTNET_COMMANDS = {
    "build": lambda entry: f"dotnet build {entry}",
    "tests": lambda entry: f"dotnet test {entry}",
    "security": lambda entry: (
        f"dotnet list {entry} package --vulnerable --include-transitive --no-restore"
    ),
    "format": lambda entry: f"dotnet format {entry} --verify-no-changes",
}

# Declared Python security tooling -> the command that runs it. A Python
# dependency audit is not in the standard library, so unlike `npm audit` it is
# bound only when the repository declares the tool in its own dependencies.
PYTHON_SECURITY_TOOLS = {
    "pip-audit": "pip-audit",
    "pip_audit": "pip-audit",
    "bandit": "bandit -r .",
    "safety": "safety check",
}

DEPENDENCY_FILES = ("requirements.txt", "requirements-dev.txt", "requirements-test.txt", "Pipfile")

# A module-level bare test function starts at column 0. An indented `def
# test_x(self):` is a `unittest.TestCase` method, and treating those as bare
# functions would make every unittest suite in the fleet look like a pytest
# suite: `workspace-governance`'s own 24 test files are all TestCase methods.
BARE_TEST_DEF = re.compile(r"^def\s+test_", re.MULTILINE)
# The same shape at any indent, used only to ask whether a file defines a test
# at all. `alethefy` ships an empty `tests/test_cli_json_contract.py`, and an
# empty file is not evidence for or against any runner.
ANY_TEST_DEF = re.compile(r"^\s*def\s+test_", re.MULTILINE)
IMPORT_PYTEST = re.compile(r"^\s*(import\s+pytest|from\s+pytest\s+import)", re.MULTILINE)
IMPORT_UNITTEST = re.compile(r"^\s*(import\s+unittest|from\s+unittest\s+import)", re.MULTILINE)


# --- reading the manifest's own declarations ------------------


# Historical CLI/import path; implementation is grouped by concern below.
if __name__ == "__main__":
    sys.modules.setdefault("bind_gate_commands", sys.modules[__name__])
import bind_gate_commands_discovery as _discovery
globals().update({key: value for key, value in vars(_discovery).items() if not key.startswith("__")})
import bind_gate_commands_declarations as _declarations
globals().update({key: value for key, value in vars(_declarations).items() if not key.startswith("__")})
import bind_gate_commands_execution as _execution
globals().update({key: value for key, value in vars(_execution).items() if not key.startswith("__")})
for _part in (_discovery, _declarations, _execution):
    _part.__dict__.update({key: value for key, value in globals().items() if not key.startswith("__")})


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="workspace root holding the sibling repositories",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="write the bindings and the debt record; without this the run is a dry run",
    )
    parser.add_argument(
        "--debt-only",
        action="store_true",
        help=(
            "write only the debt record, from a run that cannot write a manifest. "
            "Regenerating the record must never be the thing that writes the "
            "fleet, so this forces the run itself dry while still persisting the "
            "record; --apply is ignored and this cannot be combined with it"
        ),
    )
    parser.add_argument(
        "--debt-out",
        type=Path,
        default=None,
        help=(
            f"where to write the debt record (default: this repository's own "
            f"{DEBT_FILE}). The record belongs to the governance repository, "
            f"not beside the sibling repositories it describes — "
            f"`workspace_check.py` reads it from the governance root, so "
            f"writing it under `--root` put the authoritative copy somewhere "
            f"nothing reads while the real one went stale"
        ),
    )
    parser.add_argument(
        "--repo",
        action="append",
        dest="only",
        help="restrict the run to these repository names (repeatable)",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print the derived command and its on-disk evidence per concern",
    )
    args = parser.parse_args(argv)

    if args.debt_only and args.apply:
        print(
            "--debt-only cannot be combined with --apply: --debt-only exists "
            "precisely so a record is regenerated without writing the fleet",
            file=sys.stderr,
        )
        return 2

    root = args.root.resolve()
    if not root.is_dir():
        print(f"workspace root is not a directory: {root}", file=sys.stderr)
        return 2

    # `--debt-only` threads `apply=False` all the way into `run`, so no code
    # path in the run can reach a manifest write. Only the record is persisted.
    report = run(
        root,
        apply=args.apply and not args.debt_only,
        only=args.only,
        check_clean=True,
    )
    for outcome in report.outcomes:
        marker = "wrote" if outcome.wrote else outcome.status.lower()
        detail = format_detail(outcome, args.verbose)
        if not detail:
            detail = f" :: {outcome.detail}" if outcome.detail else ""
        print(f"{outcome.repo:32} {marker:12}{detail}")

    debt = build_debt(report, root)
    # `--debt-out` is the only thing that redirects the record, and the default
    # is the workspace root the caller named. Hard-coding the script's own
    # repository instead looked more correct and was much worse: a test
    # pointing `--root` at a temporary workspace overwrote the real
    # `gate-debt.json` in this repository, because the default no longer
    # followed the argument the caller gave. A default that ignores its own
    # flag is not a default. The stale-record problem this was meant to solve
    # is a caller problem, and `--debt-out` is the fix.
    debt_path = args.debt_out or (root / DEBT_FILE)
    writing_debt = args.apply or args.debt_only
    if writing_debt:
        write_debt_record(debt, debt_path)

    unresolved = report.unresolved()
    conflicts = report.conflicts()
    print()
    mode = "DEBT RECORD ONLY (manifest run is dry)" if args.debt_only else ("APPLY" if args.apply else "DRY RUN")
    print(f"mode:            {mode}")
    print(f"repositories:    {len(report.outcomes)}")
    for status in (RESOLVED, PARTIAL, UNRESOLVED, NO_MANIFEST, DIRTY, REFUSED):
        names = report.names(status)
        print(f"{status + ':':18}{len(names):<5}{' '.join(names[:6])}{' ...' if len(names) > 6 else ''}")
    print(f"{'unresolved checks:':18}{len(unresolved)}  (declared, unbound, and underivable)")
    print(f"{'left in place:':18}{len(conflicts)}  (a person wrote a different command; not clobbered)")
    print(f"{'debt record:':18}{debt_path}{' (written)' if writing_debt else ' (not written; dry run)'}")
    if unresolved:
        print()
        print("Unresolved concerns, by reason:")
        for group in debt["by_reason"]:
            print(f"  {group['checks']:>3}  {group['reason']}")
    return 3 if unresolved else 0


if __name__ == "__main__":
    raise SystemExit(main())
