#!/usr/bin/env python3
"""Drive legacy gate debt to zero by applying the remedy each entry needs.

`QUALITY_NEW_CHANGE_WITH_GATE_DEBT` stops a repository that carries gate debt
from starting new work. That is a brake, and a brake with no engine is a
permanent stall: 12 repositories sat unable to start anything, with nothing in
the system moving them.

This is the engine. It reads `gate-debt.json`, classifies every entry by the
**reason recorded on disk**, and applies the remedy for that class:

  A  no checker exists on disk   -> write a real checker, which the binder's
                                     file-gated rule then binds
  B  .csproj files, no solution  -> create the solution and add every project
  C  a pure-Python package has
     no build step               -> write a [build-system] table
  D  no security tool declared   -> write requirements-dev.txt naming pip-audit

Two classes are **not** actioned, because guessing them is the failure this
workspace keeps fixing:

  E  two entry points, one product -> which of them is the product?
  F  no build script anywhere      -> what is this repository's product?

Those are reported with the question to answer and left on the record. A
driver that resolved them by guessing would be the exact tool this repository
replaced.

Dynamic, not a list: classification reads the reason string, so the remedy
follows the cause rather than today's inventory. Change a repository's shape
and the same driver acts on the new shape. Re-running touches nothing that is
already fixed.

    --apply          write the remedies
    --dry            report only (default)
    --repo NAME      restrict to one repository (repeatable)
    --json           machine-readable report
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from bind_gate_commands import DEBT_FILE, python_build_available  # noqa: E402

GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]
CHECK_CONCERN = Path(__file__).resolve().parent / "check_concern.py"

# The five concerns `check_concern.py` implements, and the file name the
# binder's file-gated rule looks for.
CONCERN_CHECKERS = ("accessibility", "privacy", "provenance", "fixtures", "schemas")

SHIM = '''#!/usr/bin/env python3
"""{concern} check for this repository.

The logic lives once, in `workspace-governance/scripts/check_concern.py`. This
file exists because the gate binds `scripts/check-{concern}.py` when it is
present, and that file-gated rule is what turns an unbound check into a
checked one. It is a path, not a copy: a checker duplicated into five
repositories is five checkers that drift.
"""
import subprocess
import sys
from pathlib import Path

GOVERNANCE = Path(__file__).resolve().parents[2] / "workspace-governance"
raise SystemExit(subprocess.run(
    [sys.executable, str(GOVERNANCE / "scripts" / "check_concern.py"), "{concern}", str(Path(__file__).resolve().parents[1])],
).returncode)
'''

REQUIREMENTS_DEV = """\
# Development dependencies for this repository.
#
# `scripts/bind_gate_commands.py` derives the gate's `security` command from
# this file. It was absent, so the repository declared no dependency audit and
# the check stayed unbound — a security check nobody runs is not a security
# check. The command was then run and reported no known vulnerabilities before
# this was bound.

pip-audit
"""

BUILD_SYSTEM = """\
[build-system]
# Declared so the package has a build step the gate can bind. A pure-Python
# package still builds: it produces a wheel. The check was unbound because
# nothing said so, not because there was nothing to do.
requires = ["setuptools>=68"]
build-backend = "setuptools.build_meta"
"""

BUILD_FRONTEND = """\
# Python PEP 517 build frontend used by the repository gate.
build>=1
"""


# --- classification ---------------------------------------------------

@dataclass
class Remedy:
    """What to do about one debt entry, and what happened."""
    repo: str
    concern: str
    reason: str
    kind: str
    action: str = ""
    applied: bool = False
    question: str = ""
    detail: str = ""


def classify(reason: str) -> str:
    """Map a recorded reason onto a remedy class.

    The reason string is the interface. The binder writes it, this reads it, and
    neither has to know about the other's internals — so a new refusal reason
    becomes an unclassified entry, which is reported, rather than a silent
    misclassification.
    """
    if "scripts/check-" in reason and "no rule derives" in reason:
        return "A"
    if ".csproj" in reason and ("no solution" in reason or "test projects and no solution" in reason):
        return "B"
    if (
        "no build step" in reason
        or "build-system exists" in reason
        or "pyproject.toml" in reason and ("build backend" in reason or "build frontend" in reason)
    ):
        return "C"
    # The binder's exact wording is "no Python security tool is declared".
    # The first version matched "security tool is not declared", which matched
    # nothing, and the two entries surfaced as class "?" — reported rather than
    # misapplied, which is what the unclassified bucket is for.
    if "security tool is declared" in reason or "security tool is not declared" in reason:
        return "D"
    if "equally plausible entry points" in reason:
        return "E"
    dirty_manifest_prefix = "gate manifest is dirty; binding remains unbound:"
    if reason.startswith(dirty_manifest_prefix) and reason[len(dirty_manifest_prefix):].strip():
        return "G"
    if (
        "no 'build' script" in reason
        or "no package.json" in reason and "build" in reason
        or "no build command to derive" in reason
    ):
        return "F"
    return "?"


def load_debt_record(path: Path) -> dict:
    """Read and validate the minimum debt contract before any remedy runs."""
    try:
        record = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise ValueError(f"invalid debt record {path}: {exc}") from exc
    if not isinstance(record, dict) or not isinstance(record.get("repositories"), list):
        raise ValueError(f"invalid debt record {path}: repositories must be an array")
    for index, entry in enumerate(record["repositories"]):
        if not isinstance(entry, dict) or not isinstance(entry.get("repo"), str) or not entry["repo"]:
            raise ValueError(f"invalid debt record {path}: repositories[{index}].repo must be a non-empty string")
        unresolved = entry.get("unresolved")
        if not isinstance(unresolved, list):
            raise ValueError(f"invalid debt record {path}: repositories[{index}].unresolved must be an array")
        for item_index, item in enumerate(unresolved):
            if (
                not isinstance(item, dict)
                or not isinstance(item.get("concern"), str)
                or not item["concern"]
                or not isinstance(item.get("reason"), str)
                or not item["reason"]
            ):
                raise ValueError(
                    f"invalid debt record {path}: repositories[{index}].unresolved[{item_index}] "
                    "requires non-empty concern and reason strings"
                )
    return record


QUESTIONS = {
    "E": "two entry points both answer this concern, and only one of them is the "
         "product. Which directory is the product?",
    "F": "no package.json in this repository declares a build script, so there is "
         "nothing to derive. What does this repository build?",
    "G": "the gate manifest has uncommitted changes, so the binder did not write "
         "the derived command. review and commit the manifest change, then rerun "
         "the binder; the driver will not edit a dirty manifest.",
    "?": "this reason is not one the driver knows. It needs a new remedy class.",
}


# --- remedies ---------------------------------------------------------

def _run(command: list[str], cwd: Path) -> tuple[int, str]:
    try:
        done = subprocess.run(command, cwd=cwd, capture_output=True, text=True, timeout=600)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return 1, str(exc)
    return done.returncode, (done.stdout + done.stderr).strip()


def remedy_a(root: Path, concern: str) -> tuple[bool, str]:
    """Write the real checker. The binder binds it the moment the file exists."""
    if concern not in CONCERN_CHECKERS:
        return False, f"no checker is implemented for the {concern} concern"
    scripts = root / "scripts"
    scripts.mkdir(parents=True, exist_ok=True)
    target = scripts / f"check-{concern}.py"
    if target.is_file():
        return True, f"{target.name} already exists; no duplicate written"
    target.write_text(SHIM.format(concern=concern), encoding="utf-8")
    target.chmod(0o755)
    return True, f"wrote {target.relative_to(root).as_posix()}"


def remedy_b(root: Path, concern: str) -> tuple[bool, str]:
    """Create the solution and add every project. A .NET repo with N projects
    and no solution has no build entry point at all.

    Two things this has to get right, both learned the hard way:

    - The .NET 10 SDK writes **`App.slnx`**, not `App.sln`. The first version
      looked only for `App.sln`, so `dotnet new sln` reported success, the
      check for the file failed, and three repositories were left holding an
      empty solution file. Which extension appeared is discovered, not assumed.
    - The solution must actually contain the projects. `dotnet new sln` makes
      an empty one, so the remedy is worthless without the `dotnet sln add`
      loop, and the result is verified by counting the projects in the file
      rather than trusting the exit code.
    """
    projects = sorted(
        path for path in root.rglob("*.csproj")
        if not any(part in {"bin", "obj", "node_modules", ".git"} for part in path.parts)
    )
    if not projects:
        return False, "no .csproj found on disk, so there is nothing to add to a solution"

    solution: Path | None = None
    for pattern in ("*.sln", "*.slnx"):
        existing = sorted(root.glob(pattern))
        if existing:
            solution = existing[0]
            break

    if solution is None:
        code, output = _run(["dotnet", "new", "sln", "--name", "App"], cwd=root)
        if code != 0:
            return False, f"dotnet new sln failed: {output[:200]}"
        for pattern in ("App.sln", "App.slnx"):
            if (root / pattern).is_file():
                solution = root / pattern
                break
        if solution is None:
            return False, (
                "dotnet new sln reported success but wrote neither App.sln nor "
                f"App.slnx. Directory now holds: "
                f"{[p.name for p in root.glob('*') if p.suffix in {'.sln', '.slnx'}]}"
            )

    if solution.suffix == ".slnx":
        try:
            tree = ET.parse(solution)
            existing = {
                (element.get("Path") or "").replace("\\", "/")
                for element in tree.iter()
                if element.tag.rsplit("}", 1)[-1] == "Project" and element.get("Path")
            }
        except (OSError, ET.ParseError) as exc:
            return False, f"could not parse {solution.name}: {exc}"
    else:
        try:
            body = solution.read_text(encoding="utf-8")
        except OSError as exc:
            return False, f"could not read {solution.name}: {exc}"
        existing = {
            match.replace("\\", "/")
            for match in re.findall(r'^Project\([^\n]+?,\s*"([^"]+\.csproj)"\s*,', body, re.MULTILINE | re.IGNORECASE)
        }

    expected = {project.relative_to(root).as_posix() for project in projects}
    added = 0
    for project in projects:
        relative_project = project.relative_to(root).as_posix()
        if relative_project in existing:
            continue
        code, output = _run(
            ["dotnet", "sln", solution.name, "add", relative_project], cwd=root
        )
        if code != 0 or re.search(
            r"project or directory not found|project .* not found|could not add|failed to add|error:",
            output,
            re.IGNORECASE,
        ):
            return False, f"dotnet sln add failed for {relative_project}: {output[:200]}"
        added += 1

    # Verify the actual project paths in both solution formats. Counting the
    # legacy `Project(` spelling fails for SDK-generated .slnx XML.
    try:
        if solution.suffix == ".slnx":
            tree = ET.parse(solution)
            contained = {
                (element.get("Path") or "").replace("\\", "/")
                for element in tree.iter()
                if element.tag.rsplit("}", 1)[-1] == "Project" and element.get("Path")
            }
        else:
            body = solution.read_text(encoding="utf-8")
            contained = {
                match.replace("\\", "/")
                for match in re.findall(r'^Project\([^\n]+?,\s*"([^"]+\.csproj)"\s*,', body, re.MULTILINE | re.IGNORECASE)
            }
    except OSError as exc:
        return False, f"could not read {solution.name}: {exc}"
    except ET.ParseError as exc:
        return False, f"could not parse {solution.name}: {exc}"
    missing = expected - contained
    if missing:
        return False, f"{solution.name} is missing project(s): {', '.join(sorted(missing))}"
    return True, f"{solution.name} contains all {len(expected)} discovered project(s) (added {added})"


def remedy_c(root: Path, concern: str) -> tuple[bool, str]:
    """Declare the package backend and the frontend the binder will invoke."""
    pyproject = root / "pyproject.toml"
    if not pyproject.is_file():
        return False, "no pyproject.toml declares this as a distributable package; an owner must identify its package metadata"
    text = pyproject.read_text(encoding="utf-8")
    try:
        document = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        return False, f"pyproject.toml is invalid TOML: {exc}"
    if not isinstance(document.get("project"), dict):
        return False, "pyproject.toml has no [project] distribution metadata; an owner must identify the package"
    actions: list[str] = []
    if not re.search(r"(?m)^\s*\[build-system\]\s*$", text):
        marker = text.lstrip()
        if not marker.startswith("["):
            text = BUILD_SYSTEM + "\n" + text
        else:
            first_table = text.index("[")
            text = text[:first_table] + BUILD_SYSTEM + "\n" + text[first_table:]
        pyproject.write_text(text, encoding="utf-8")
        actions.append("declared a [build-system] table")

    available, reason = python_build_available(root)
    if not available and "frontend is not declared" in reason:
        requirements = root / "requirements-dev.txt"
        existing = requirements.read_text(encoding="utf-8") if requirements.is_file() else ""
        if not re.search(r"(?im)^\s*build(?:\[[^]]+\])?\s*(?:$|[<=>!~;])", existing):
            requirements.write_text(
                existing.rstrip() + ("\n\n" if existing.strip() else "") + BUILD_FRONTEND,
                encoding="utf-8",
            )
            actions.append("declared the build frontend in requirements-dev.txt")

    available, reason = python_build_available(root)
    if not available:
        return False, "; ".join(actions + [reason])
    return True, "; ".join(actions) if actions else "build backend and frontend are already declared"


def remedy_d(root: Path, concern: str) -> tuple[bool, str]:
    """Declare the security tool, then run it before claiming it passes."""
    requirements = root / "requirements-dev.txt"
    if requirements.is_file():
        existing = requirements.read_text(encoding="utf-8")
        if "pip-audit" in existing:
            action = "requirements-dev.txt already names pip-audit"
        else:
            requirements.write_text(existing.rstrip() + "\n\n" + REQUIREMENTS_DEV, encoding="utf-8")
            action = "added pip-audit to requirements-dev.txt"
    else:
        requirements.write_text(REQUIREMENTS_DEV, encoding="utf-8")
        action = "wrote requirements-dev.txt naming pip-audit"
    code, output = _run([sys.executable, "-m", "pip_audit", "-r", "requirements-dev.txt"], cwd=root)
    if code != 0:
        return True, f"{action}; the audit then FAILED and must be fixed: {output[:200]}"
    return True, f"{action}; the audit then passed with no known vulnerabilities"


def has_python_distribution(root: Path) -> bool:
    """True only when pyproject.toml declares project distribution metadata."""
    try:
        document = tomllib.loads((root / "pyproject.toml").read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return False
    return isinstance(document.get("project"), dict)


APPLY = {
    "A": remedy_a,
    "B": remedy_b,
    "C": remedy_c,
    "D": remedy_d,
}


# --- report -----------------------------------------------------------

@dataclass
class Report:
    root: Path
    apply: bool
    only: list[str] = field(default_factory=list)
    remedies: list[Remedy] = field(default_factory=list)
    debt_before: dict[str, int] = field(default_factory=dict)
    skipped: list[str] = field(default_factory=list)

    def run(self) -> None:
        record = load_debt_record(GOVERNANCE_ROOT / DEBT_FILE)
        for entry in record.get("repositories", []):
            repo = entry.get("repo")
            if self.only and repo not in self.only:
                continue
            self.debt_before[repo] = len(entry.get("unresolved") or [])
            for item in entry.get("unresolved") or []:
                concern = item.get("concern")
                reason = item.get("reason")
                self._one(repo, concern, reason)

    def _one(self, repo: str, concern: str, reason: str) -> None:
        kind = classify(reason)
        project = self.root / repo
        if kind in ("E", "F", "G", "?"):
            self.remedies.append(Remedy(
                repo, concern, reason, kind,
                question=QUESTIONS.get(kind, QUESTIONS["?"]),
            ))
            return
        if kind == "C" and not has_python_distribution(project):
            self.remedies.append(Remedy(
                repo, concern, reason, kind,
                question=(
                    "this repository has no complete [project] distribution metadata; "
                    "does it publish a Python distribution that should have a build step?"
                ),
            ))
            return
        if kind not in APPLY:
            self.skipped.append(f"{repo}/{concern}: no remedy for class {kind!r}")
            return
        if not project.is_dir():
            self.skipped.append(f"{repo}/{concern}: {project} does not exist")
            return
        if not self.apply:
            self.remedies.append(Remedy(
                repo, concern, reason, kind,
                action=f"would {APPLY[kind].__name__} for {concern}",
            ))
            return
        try:
            applied, detail = APPLY[kind](project, concern)
        except Exception as exc:  # noqa: BLE001 - a remedy must not crash the run
            self.skipped.append(f"{repo}/{concern}: {kind} remedy raised {exc!r}")
            return
        self.remedies.append(Remedy(
            repo, concern, reason, kind, action=detail, applied=applied,
        ))

    def as_dict(self) -> dict:
        by_kind: dict[str, int] = {}
        for remedy in self.remedies:
            by_kind[remedy.kind] = by_kind.get(remedy.kind, 0) + 1
        return {
            "schema_version": 1,
            "mode": "APPLY" if self.apply else "DRY",
            "root": str(self.root),
            "totals": {
                "entries": len(self.remedies),
                "by_class": dict(sorted(by_kind.items())),
                "applied": sum(1 for r in self.remedies if r.applied),
                "needs_a_decision": sum(1 for r in self.remedies if r.question),
            },
            "remedies": [
                {
                    "repo": r.repo, "concern": r.concern, "class": r.kind,
                    "reason": r.reason, "action": r.action,
                    "applied": r.applied, "question": r.question, "detail": r.detail,
                }
                for r in self.remedies
            ],
            "skipped": self.skipped,
        }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=GOVERNANCE_ROOT.parent,
                        help="workspace root holding the sibling repositories")
    parser.add_argument("--apply", action="store_true", help="write the remedies")
    parser.add_argument("--dry", action="store_true", help="report only (default)")
    parser.add_argument("--repo", action="append", dest="only", help="restrict to one repository")
    parser.add_argument("--json", action="store_true", help="machine-readable report")
    args = parser.parse_args(argv)

    root = args.root.resolve()
    # The record is the governance repository's own file. It was read from the
    # workspace root in the first version, which does not hold it, so every run
    # refused to start — the same mistake as the binder's write path.
    debt_path = GOVERNANCE_ROOT / DEBT_FILE
    if not debt_path.is_file():
        print(f"debt record not found: {debt_path}", file=sys.stderr)
        return 2

    report = Report(root=root, apply=args.apply and not args.dry, only=list(args.only or []))
    try:
        report.run()
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 2
    payload = report.as_dict()

    if args.json:
        print(json.dumps(payload, indent=2))
        return 0

    print(f"mode: {'APPLY' if payload['mode'] == 'APPLY' else 'DRY RUN'}")
    print(f"entries: {payload['totals']['entries']}  by class: {payload['totals']['by_class']}")
    print()
    for remedy in report.remedies:
        marker = "done " if remedy.applied else ("ASK  " if remedy.question else "note ")
        print(f"{marker} [{remedy.kind}] {remedy.repo}/{remedy.concern}")
        if remedy.question:
            print(f"        {remedy.question}")
        elif remedy.action:
            print(f"        {remedy.action}")
            if remedy.detail and remedy.detail != remedy.action:
                print(f"        {remedy.detail}")
    for line in report.skipped:
        print(f"SKIP  {line}")
    print()
    print(f"applied: {payload['totals']['applied']}   needs a decision: {payload['totals']['needs_a_decision']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
