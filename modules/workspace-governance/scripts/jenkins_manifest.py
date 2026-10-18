#!/usr/bin/env python3
"""Generate the Jenkins-safe deployable project list."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

from workspace_check import load_json, run


JOB = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]*$")


def build_manifest(root: Path, registry_path: Path) -> tuple[dict, list[str]]:
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    errors: list[str] = []
    projects: list[dict] = []
    for entry in registry.get("projects", []):
        path = root / entry["path"]
        declaration_path = path / ".project.json"
        if not declaration_path.is_file():
            continue
        declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
        deployment = declaration.get("deployment", {})
        if not deployment.get("deployable"):
            continue
        job = deployment.get("jenkins_job")
        compose = deployment.get("compose_file")
        if not isinstance(job, str) or not JOB.fullmatch(job):
            errors.append(f"{entry['id']}: invalid Jenkins job name")
            continue
        if not isinstance(compose, str) or not (path / compose).is_file():
            errors.append(f"{entry['id']}: missing declared Compose file")
            continue
        projects.append({"id": entry["id"], "path": entry["path"], "jenkins_job": job, "compose_file": compose, "verification_command": declaration["verification"]["command"]})
    return {"schema_version": 1, "generated_by": "workspace-governance/scripts/jenkins_manifest.py", "generated_at": datetime.now(timezone.utc).isoformat(), "projects": projects}, errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=None)
    parser.add_argument("--registry", default=None)
    parser.add_argument("--output", default=None)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    here = Path(__file__).resolve().parents[1]
    registry_path = Path(args.registry) if args.registry else here / "projects.json"
    registry = load_json(registry_path, [], "REGISTRY")
    if registry is None:
        return 1
    root = Path(args.root or os.environ.get("WORKSPACE_ROOT") or registry.get("workspace_root") or Path.cwd()).expanduser().resolve()
    check_args = argparse.Namespace(registry=str(registry_path), root=str(root), project=None, audit=False, self_check=True, as_json=False)
    findings, summary = run(check_args)
    for entry in registry.get("projects", []):
        declaration_path = root / entry["path"] / ".project.json"
        if not declaration_path.is_file():
            continue
        declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
        if declaration.get("deployment", {}).get("deployable"):
            project_args = argparse.Namespace(registry=str(registry_path), root=str(root), project=entry["id"], audit=False, self_check=False, as_json=False)
            project_findings, project_summary = run(project_args)
            findings.extend(project_findings)
            summary["errors"] += project_summary["errors"]
            summary["warnings"] += project_summary["warnings"]
    manifest, errors = build_manifest(root, registry_path)
    if errors:
        findings.extend(type("Finding", (), {"level": "ERROR", "code": "JENKINS_MANIFEST", "message": item, "project": None})() for item in errors)
        summary["errors"] += len(errors)
    if args.check:
        for item in findings:
            print(f"{item.level} {item.code}: {item.message}", file=sys.stderr)
        print(f"Jenkins manifest check: projects={len(manifest['projects'])} errors={summary['errors']}")
        return 1 if summary["errors"] else 0
    output = Path(args.output) if args.output else here / "deployment" / "projects.json"
    output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {output} ({len(manifest['projects'])} deployable projects)")
    return 1 if summary["errors"] else 0


if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    raise SystemExit(main())
