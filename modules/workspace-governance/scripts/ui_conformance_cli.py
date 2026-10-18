"""Command-line and fleet orchestration for the UI conformance checker."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

from ui_conformance import (
    BASELINE_FILE, DEFAULT_NODE_TIMEOUT, DEFAULT_TOKEN_SOURCE, SCHEMA_VERSION,
    ConfigError, Report, _governance_root, _load_declaration, _resolve_inside,
    apply_baseline, census, evaluate, exit_code, load_baseline, parse_ui_policy,
)

def _print_report(report: Report, as_json: bool) -> None:
    if as_json:
        print(json.dumps(report.to_json(), indent=2, sort_keys=True))
        return
    for note in report.notes:
        print(f"  note {report.project}: {note}")
    for finding in report.findings:
        location = f"{finding.file}:{finding.line}" if finding.line else finding.file
        print(f"  {finding.level} {finding.rule_id} {location} {finding.marker}")
    print(
        f"{report.status}: {report.project} ui_files={report.summary['ui_files']} "
        f"errors={report.summary['errors']} review_required={report.summary['review_required']} "
        f"baselined={report.summary['baselined']}"
    )


def _fleet_roots(governance_root: Path, override: str | None = None) -> tuple[Path, list[dict[str, Any]]]:
    registry_path = governance_root / "projects.json"
    if not registry_path.is_file():
        raise ConfigError(f"registry not found: {registry_path}")
    document = _load_declaration(registry_path)
    projects = document.get("projects")
    if not isinstance(projects, list):
        raise ConfigError("projects.json: projects must be a list")
    if override:
        return Path(override).expanduser().resolve(), projects
    workspace_root = os.environ.get("WORKSPACE_ROOT") or document.get("workspace_root") or Path.cwd()
    return Path(str(workspace_root)).expanduser().resolve(), projects


def audit(
    governance_root: Path,
    baseline_path: Path,
    *,
    as_json: bool,
    token_source: Path | None,
    module_path: Path | None,
    node_timeout: int,
    root_override: str | None = None,
) -> int:
    """Run every registered project under the workspace root."""
    workspace_root, projects = _fleet_roots(governance_root, root_override)
    covered = load_baseline(baseline_path)
    reports: list[Report] = []
    worst = 0
    for entry in projects:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            continue
        project_dir = (workspace_root / entry["path"]).resolve()
        if not project_dir.is_dir():
            continue
        try:
            declaration = _load_declaration(project_dir / ".project.json")
            policy = parse_ui_policy(declaration)
        except ConfigError as exc:
            report = Report(schema_version=SCHEMA_VERSION, status="REVIEW_REQUIRED", project=entry["path"])
            report.notes.append(f"configuration: {exc}")
            reports.append(report)
            worst = max(worst, 2)
            continue
        if not census(project_dir):
            continue
        report = evaluate(
            project_dir, policy,
            token_source=token_source, module_path=module_path, node_timeout=node_timeout,
        )
        report.project = entry["path"]
        apply_baseline(report, covered.get(entry["path"], frozenset()))
        reports.append(report)
        worst = max(worst, exit_code(report))

    failing = [r for r in reports if r.status == "FAIL"]
    review = [r for r in reports if r.status == "REVIEW_REQUIRED"]
    if as_json:
        print(json.dumps({
            "schema_version": SCHEMA_VERSION,
            "workspace_root": str(workspace_root),
            "baseline": str(baseline_path),
            "projects_checked": len(reports),
            "failing": len(failing),
            "review_required": len(review),
            "projects": [r.to_json() for r in reports],
        }, indent=2, sort_keys=True))
    else:
        for report in reports:
            if report.status == "PASS" and not report.summary["baselined"]:
                continue
            _print_report(report, False)
        print(
            f"AUDIT: projects={len(reports)} fail={len(failing)} review_required={len(review)} "
            f"baseline={baseline_path.name}"
        )
    return worst


def main(argv: list[str] | None = None) -> int:
    governance_root = _governance_root()
    parser = argparse.ArgumentParser(
        description="UI convention conformance checker",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("project", nargs="?", type=Path, help="project directory; omit with --audit")
    parser.add_argument("--json", action="store_true", help="emit a versioned JSON report on stdout")
    parser.add_argument("--baseline", type=Path, default=governance_root / BASELINE_FILE,
                        help="baseline file covering pre-existing findings (default: %s)" % BASELINE_FILE)
    parser.add_argument("--audit", action="store_true", help="check every registered project under the workspace root")
    parser.add_argument("--root", default=None, help="workspace root for --audit; defaults to WORKSPACE_ROOT or the registry's workspace_root")
    parser.add_argument("--declaration", type=Path, default=None, help="explicit declaration file")
    parser.add_argument("--tokens", type=Path, default=None, help="token source; defaults to the platform layer's tokens.json")
    parser.add_argument("--token-module", type=Path, default=None, help="token-model.mjs path")
    parser.add_argument("--node-timeout", type=int, default=DEFAULT_NODE_TIMEOUT, help="seconds allowed for the token resolver")
    args = parser.parse_args(argv)

    default_source = (governance_root / DEFAULT_TOKEN_SOURCE).resolve()
    token_source = args.tokens.resolve() if args.tokens else default_source

    try:
        if args.audit:
            return audit(
                governance_root, args.baseline, as_json=args.json,
                token_source=token_source, module_path=args.token_module,
                node_timeout=args.node_timeout, root_override=args.root,
            )
        if args.project is None:
            print("a project directory is required unless --audit is given", file=sys.stderr)
            return 2
        project_dir = args.project.resolve()
        if not project_dir.is_dir():
            print(f"project directory not found: {project_dir}", file=sys.stderr)
            return 2
        declaration_path = args.declaration or (project_dir / ".project.json")
        declaration = _load_declaration(declaration_path)
        policy = parse_ui_policy(declaration)
        if policy.token_source:
            token_source = _resolve_inside(project_dir, policy.token_source)
        report = evaluate(
            project_dir, policy,
            token_source=token_source, module_path=args.token_module, node_timeout=args.node_timeout,
        )
        if args.baseline and args.baseline.is_file():
            apply_baseline(report, load_baseline(args.baseline).get(project_dir.name, frozenset()))
        _print_report(report, args.json)
        return exit_code(report)
    except ConfigError as exc:
        print(f"REVIEW_REQUIRED: configuration: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
