#!/usr/bin/env python3
"""Run the portfolio governance gate without agent intervention."""

from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from jenkins_manifest import build_manifest
from workspace_check import Finding, run


def run_gate(root: Path, registry: Path, check_deployment: bool = True, fail_on_warnings: bool = False, self_check: bool = False) -> tuple[dict[str, Any], int]:
    args = argparse.Namespace(
        registry=str(registry),
        root=str(root),
        project=None,
        audit=True,
        self_check=self_check,
        as_json=False,
    )
    findings, summary = run(args)
    deployment: dict[str, Any] = {"checked": check_deployment, "projects": 0, "errors": []}
    if check_deployment:
        manifest, deployment_errors = build_manifest(root, registry)
        deployment["projects"] = len(manifest["projects"])
        deployment["errors"] = deployment_errors
        findings.extend(Finding("ERROR", "JENKINS_MANIFEST", message) for message in deployment_errors)
        summary["errors"] += len(deployment_errors)
    status = "PASS" if summary["errors"] == 0 and (not fail_on_warnings or summary["warnings"] == 0) else "FAIL"
    report = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "status": status,
        "summary": summary,
        "deployment": deployment,
        "findings": [asdict(item) for item in findings],
    }
    return report, 0 if status == "PASS" else 1


def main() -> int:
    here = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=None, help="workspace root; defaults to WORKSPACE_ROOT or the current directory")
    parser.add_argument("--registry", default=str(here / "projects.json"))
    parser.add_argument("--report", type=Path, help="write the machine-readable gate report")
    parser.add_argument("--no-deployment", action="store_true", help="skip Jenkins manifest validation")
    parser.add_argument("--fail-on-warnings", action="store_true")
    parser.add_argument("--json", action="store_true", dest="as_json")
    parser.add_argument("--self-check", action="store_true", help="gate only this governance repository")
    args = parser.parse_args()
    root = here if args.self_check else Path(args.root or os.environ.get("WORKSPACE_ROOT") or Path.cwd()).expanduser().resolve()
    registry = Path(args.registry).expanduser().resolve()
    report, exit_code = run_gate(root, registry, check_deployment=not args.no_deployment, fail_on_warnings=args.fail_on_warnings, self_check=args.self_check)
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if args.as_json:
        print(json.dumps(report, indent=2))
    else:
        print(f"Workspace auto-gate: {report['status']} errors={report['summary']['errors']} warnings={report['summary']['warnings']} deployment_projects={report['deployment']['projects']}")
        if args.report:
            print(f"Report: {args.report}")
    return exit_code


if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    raise SystemExit(main())
