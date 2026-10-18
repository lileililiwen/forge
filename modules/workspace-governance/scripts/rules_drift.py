#!/usr/bin/env python3
"""Report drift from the canonical `.ai-rules` baseline.

`templates/project/.ai-rules/*` is the single source of truth for the shared
rules. Repositories are free to document those rules in their own words,
language and structure, so this is a **triage report**, not a rewrite: it names
the projects that do not visibly carry a canonical rule and never modifies a
project file. A project that documents the rules differently can opt out of a
file with the marker `<!-- workspace-governance:rules-customized -->`.

Usage:
    python3 scripts/rules_drift.py --root .. --check
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]
RULE_FILES = (".ai-rules/workflow.md", ".ai-rules/completion.md")
HANDOFF = "HANDOFF.md"
OPT_OUT = "<!-- workspace-governance:rules-customized -->"
SCAFFOLD_MARKERS = (
    "Replace this section with current implementation and verification evidence",
    "Replace this section with implementation and verification evidence",
)

# The invariant clauses of the shared baseline, kept short and stable so they
# survive re-wrapping; they are the machine-checkable shadow of the templates.
CLAUSES: dict[str, tuple[tuple[str, str], ...]] = {
    ".ai-rules/workflow.md": (
        ("BFS workflow model", "BFS"),
        ("two-commit rule", "two commits"),
        ("OpenSpec name preflight", "check-openspec-change-names"),
        # Keyed to the rule's subject rather than its full phrasing: it is the
        # highest-carry needle that still survives re-wrapping, so a finding
        # means the rule is genuinely absent rather than worded differently.
        # See openspec/changes/work-on-main-baseline-rule/design.md section 5.
        ("work-on-main rule", "on `main`"),
    ),
    ".ai-rules/completion.md": (
        ("two-commit rule", "two commits"),
        ("evidence boundary", "release evidence"),
    ),
}


def read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def scaffold_handoff(project_path: Path) -> bool:
    text = read_text(project_path / HANDOFF)
    return text is not None and any(marker in text for marker in SCAFFOLD_MARKERS)


def drift(project_path: Path) -> list[dict]:
    """Return the canonical clauses a project's rule files do not visibly carry."""
    findings: list[dict] = []
    for relative, clauses in CLAUSES.items():
        path = project_path / relative
        if not path.is_file():
            continue
        text = read_text(path)
        if text is None:
            findings.append({"code": "RULES_UNREADABLE", "file": relative, "label": "unreadable file"})
            continue
        if OPT_OUT in text:
            continue
        for label, needle in clauses:
            if needle not in text:
                findings.append({"code": "RULES_DRIFT", "file": relative, "label": label})
    return findings


def adopted_projects(registry: dict, workspace_root: Path) -> list[tuple[str, Path]]:
    projects: list[tuple[str, Path]] = []
    for entry in registry.get("projects", []):
        if entry.get("adoption") != "adopted":
            continue
        projects.append((str(entry.get("id")), workspace_root / str(entry.get("path"))))
    return projects


def check(workspace_root: Path, registry_path: Path) -> list[dict]:
    findings: list[dict] = []
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    for project_id, project_path in adopted_projects(registry, workspace_root):
        for item in drift(project_path):
            findings.append({
                "level": "WARNING",
                "code": item["code"],
                "project": project_id,
                "message": f"{item['file']} does not carry the {item['label']}",
            })
        if scaffold_handoff(project_path):
            findings.append({
                "level": "WARNING",
                "code": "HANDOFF_SCAFFOLD",
                "project": project_id,
                "message": "HANDOFF.md still carries the initializer scaffold",
            })
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default="..", help="workspace root")
    parser.add_argument("--registry", default=None, help="registry path; defaults to the governance registry")
    parser.add_argument("--check", action="store_true", help="report drift (the default action)")
    parser.add_argument("--json", action="store_true", dest="as_json")
    args = parser.parse_args(argv)
    workspace_root = Path(args.root).expanduser().resolve()
    registry_path = Path(args.registry).expanduser().resolve() if args.registry else GOVERNANCE_ROOT / "projects.json"

    findings = check(workspace_root, registry_path)
    if args.as_json:
        print(json.dumps({"summary": {"drift": len(findings)}, "findings": findings}, indent=2))
    else:
        for item in findings:
            print(f"{item['level']} {item['code']}: [{item['project']}] {item['message']}")
        print(f"Summary: drift_findings={len(findings)}")
    return 1 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
