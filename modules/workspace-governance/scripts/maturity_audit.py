#!/usr/bin/env python3
"""Maturity audit for the workspace portfolio.

Scans all registered projects and reports maturity indicators:
- documentation completeness
- CI coverage
- evidence status
- release evidence
- deployable status
- OpenSpec adoption

Re-run anytime to track progress.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from collections import Counter
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent))


def load_project_json(project_path: Path) -> dict | None:
    p = project_path / ".project.json"
    if not p.exists():
        return None
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def check_docs(project_path: Path) -> dict[str, bool]:
    return {
        "README.md": (project_path / "README.md").exists(),
        "ROADMAP.md": (project_path / "ROADMAP.md").exists(),
        "HANDOFF.md": (project_path / "HANDOFF.md").exists(),
        "AGENTS.md": (project_path / "AGENTS.md").exists(),
    }


def check_ci(project_path: Path) -> bool:
    return (project_path / ".github" / "workflows").is_dir()


def check_openspec(project_path: Path) -> bool:
    return (project_path / "openspec").is_dir()


def check_release_evidence(declaration: dict) -> dict[str, str]:
    re = declaration.get("release_evidence", {})
    if not isinstance(re, dict):
        return {}
    return {k: v.get("state", "missing") for k, v in re.items()}


def audit_workspace(workspace_root: Path) -> dict:
    projects = []
    findings = []

    for child in sorted(workspace_root.iterdir()):
        if not child.is_dir() or child.name.startswith("."):
            continue
        pj = load_project_json(child)
        if pj is None:
            continue

        docs = check_docs(child)
        ci = check_ci(child)
        openspec = check_openspec(child)
        re = check_release_evidence(pj)
        evidence = pj.get("verification", {}).get("evidence_status", "unknown")
        lifecycle = pj.get("lifecycle", "unknown")
        deployable = pj.get("deployment", {}).get("deployable", False)
        kind = pj.get("kind", "unknown")
        profile = pj.get("profile", "unknown")

        missing_docs = [name for name, present in docs.items() if not present]
        if missing_docs:
            findings.append(f"{child.name}: missing docs: {', '.join(missing_docs)}")

        if not ci:
            findings.append(f"{child.name}: no CI workflow")

        if evidence == "planned":
            findings.append(f"{child.name}: evidence_status is 'planned' (not executed)")

        projects.append({
            "id": child.name,
            "kind": kind,
            "profile": profile,
            "lifecycle": lifecycle,
            "evidence_status": evidence,
            "deployable": deployable,
            "ci": ci,
            "openspec": openspec,
            "docs": docs,
            "release_evidence": re,
        })

    # Summary stats
    total = len(projects)
    ci_count = sum(1 for p in projects if p["ci"])
    openspec_count = sum(1 for p in projects if p["openspec"])
    evidence_planned = sum(1 for p in projects if p["evidence_status"] == "planned")
    evidence_verified = sum(1 for p in projects if p["evidence_status"] in ("verified", "implemented", "mvp-validated"))
    deployable_count = sum(1 for p in projects if p["deployable"])
    docs_complete = sum(1 for p in projects if all(p["docs"].values()))

    return {
        "summary": {
            "total_projects": total,
            "ci_coverage": f"{ci_count}/{total}",
            "openspec_adoption": f"{openspec_count}/{total}",
            "evidence_planned": evidence_planned,
            "evidence_verified": evidence_verified,
            "deployable": deployable_count,
            "docs_complete": docs_complete,
        },
        "projects": projects,
        "findings": findings,
    }


def print_human(result: dict) -> None:
    s = result["summary"]
    print("=" * 60)
    print("WORKSPACE MATURITY AUDIT")
    print("=" * 60)
    print(f"Total projects:      {s['total_projects']}")
    print(f"CI coverage:         {s['ci_coverage']}")
    print(f"OpenSpec adoption:   {s['openspec_adoption']}")
    print(f"Evidence 'planned':  {s['evidence_planned']}")
    print(f"Evidence verified:   {s['evidence_verified']}")
    print(f"Deployable:          {s['deployable']}")
    print(f"Docs complete:       {s['docs_complete']}")
    print(f"Audit errors:        {s['errors']}")
    print(f"Audit warnings:      {s['warnings']}")
    print(f"Vocabulary version:  {s.get('vocabulary_version', 'unknown')}")
    print()
    if result["findings"]:
        print("Findings:")
        for f in result["findings"]:
            print(f"  - {f}")
    else:
        print("No findings — all projects pass basic checks.")
    print("=" * 60)


def authoritative_summary(root: Path, registry: str = "projects.json") -> dict[str, int]:
    """Return the authoritative error/warning counts from workspace_check."""
    import workspace_check
    _, summary = workspace_check.run(SimpleNamespace(
        registry=registry,
        root=str(root),
        project=None,
        audit=True,
        self_check=False,
        as_json=False,
    ))
    return summary


def vocabulary_version() -> Any:
    """Report the vocabulary schema version (display only)."""
    try:
        import vocabulary
        governance_root = Path(__file__).resolve().parents[1]
        return vocabulary.load_vocabulary_for_governance_root(governance_root)["schema_version"]
    except Exception:
        return "unknown"


def main() -> int:
    parser = argparse.ArgumentParser(description="Workspace maturity audit")
    parser.add_argument("--root", default=".", help="Workspace root directory")
    parser.add_argument("--registry", default="projects.json", help="registry path; defaults to the governance registry")
    parser.add_argument("--json", action="store_true", help="Output JSON only")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    result = audit_workspace(root)

    # The authoritative error/warning counts come from workspace_check so this
    # report can never disagree with the audit it summarises.
    try:
        audit_summary = authoritative_summary(root, args.registry)
    except Exception as exc:  # pragma: no cover - defensive
        print(f"maturity audit cannot read the authoritative audit: {exc}", file=sys.stderr)
        return 1
    result["summary"]["errors"] = audit_summary["errors"]
    result["summary"]["warnings"] = audit_summary["warnings"]
    result["summary"]["vocabulary_version"] = vocabulary_version()

    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print_human(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
