#!/usr/bin/env python3
"""Add the minimum governance declaration to an existing or new repository."""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime
from pathlib import Path

from repository_metadata import build_discoverability
from bind_gate_commands import (
    DIRTY,
    PROFILE_DEFAULTS,
    REFUSED,
    debt_concerns,
    format_detail,
    run as run_gate_binder,
)


PROJECT_ID = re.compile(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$")
CONVERTIBLE_NAME = re.compile(r"^[A-Za-z0-9_-]+$")
GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]
TEMPLATE_DIR = GOVERNANCE_ROOT / "templates" / "project"
TEXT_TEMPLATES = (
    "README.md",
    "ROADMAP.md",
    "AGENTS.md",
    "HANDOFF.md",
    ".ai-rules/workflow.md",
    ".ai-rules/completion.md",
    ".ai-gate/gate.yaml",
)


def mit_license() -> str:
    year = datetime.now().year
    return f"""MIT License

Copyright (c) {year} Project contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""


def derive_id(directory_name: str) -> str:
    """Return the canonical kebab-case project id for a directory name.

    ``hestiaLab`` -> ``hestia-lab``. Directory names that are not directly
    convertible (for example a dotted host name) still produce a candidate so
    the caller can decide whether to accept an explicit alias.
    """
    spaced = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "-", directory_name)
    return re.sub(r"[^A-Za-z0-9]+", "-", spaced).strip("-").lower()


def resolve_project_id(directory_name: str, requested: str | None) -> tuple[str | None, str | None]:
    """Resolve the declaration id, returning (id, error).

    The directory name is the source of truth when it is convertible to a
    valid id: an explicit ``--id`` that differs is refused. A directory name
    that is not convertible (for example ``lileililiwen.github.io``) may use
    an explicitly recorded alias instead.
    """
    derived = derive_id(directory_name)
    convertible = bool(CONVERTIBLE_NAME.fullmatch(directory_name))
    if requested:
        if not PROJECT_ID.fullmatch(requested):
            return None, f"id must match {PROJECT_ID.pattern}: {requested!r}"
        if convertible and requested != derived:
            return None, f"--id {requested!r} conflicts with directory name; use {derived!r}"
        return requested, None
    if not PROJECT_ID.fullmatch(derived):
        return None, f"cannot derive a valid id from {directory_name!r}; pass --id explicitly"
    return derived, None


def load_templates() -> dict[str, str]:
    """Read the canonical templates from disk.

    `templates/project/` is the single source of truth; there is deliberately
    no embedded copy in this script, so changing a template changes the
    initializer's output.
    """
    templates: dict[str, str] = {}
    for relative in TEXT_TEMPLATES:
        path = TEMPLATE_DIR / relative
        if not path.is_file():
            raise FileNotFoundError(f"missing canonical template: {path}")
        templates[relative] = path.read_text(encoding="utf-8")
    return templates


def render(content: str, project_id: str, profile: str, verification_command: str) -> str:
    return (content
            .replace("__PROFILE__", profile)
            .replace("__CURRENT_SPEC__", "bootstrap-governance")
            .replace("__PROJECT_ID__", project_id)
            .replace("__VERIFICATION_COMMAND__", verification_command))


def apply_local_metadata(target: Path, metadata: dict, license_spdx: str) -> list[str]:
    """Add missing local metadata without replacing existing values or files."""
    changed: list[str] = []
    declaration_path = target / ".project.json"
    declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
    if not isinstance(declaration, dict):
        raise ValueError(".project.json must contain an object")
    license_path = target / "LICENSE"
    if not license_path.exists() and license_spdx != "MIT":
        raise ValueError(f"no bundled license template for {license_spdx}; add LICENSE manually")
    if "discoverability" not in declaration:
        declaration["discoverability"] = metadata
        declaration_path.write_text(json.dumps(declaration, indent=2) + "\n", encoding="utf-8")
        changed.append(".project.json")
    elif declaration["discoverability"] != metadata:
        raise ValueError("discoverability already exists and differs; refusing to overwrite it")
    if not license_path.exists():
        license_path.write_text(mit_license(), encoding="utf-8")
        changed.append("LICENSE")
    return changed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("target", type=Path)
    parser.add_argument("--id", default=None, help="explicit id; must match the derived id unless the directory name is not convertible")
    parser.add_argument("--profile", required=True)
    parser.add_argument("--kind", default="product")
    parser.add_argument("--lifecycle", default="active")
    parser.add_argument("--verification-command", default="./scripts/verify.sh")
    parser.add_argument("--force", action="store_true", help="kept for automation compatibility; existing files are never overwritten")
    parser.add_argument("--apply-metadata", action="store_true", help="add explicit discoverability metadata and a missing license")
    parser.add_argument("--description")
    parser.add_argument("--keywords", nargs="+")
    parser.add_argument("--topics", nargs="+")
    parser.add_argument("--audience", nargs="+")
    parser.add_argument("--use-case", dest="use_cases", action="append")
    parser.add_argument("--search-phrase", dest="search_phrases", action="append")
    parser.add_argument("--homepage")
    parser.add_argument("--license-spdx", default="MIT")
    parser.add_argument("--visibility", choices=["public", "private", "internal"])
    args = parser.parse_args()
    target = args.target.resolve()
    if args.profile not in PROFILE_DEFAULTS:
        supported = ", ".join(sorted(PROFILE_DEFAULTS))
        print(
            f"init_project refused: unsupported gate profile {args.profile!r}; "
            f"supported profiles: {supported}",
            file=sys.stderr,
        )
        return 2
    project_id, error = resolve_project_id(target.name, args.id)
    if project_id is None:
        print(f"init_project refused: {error}", file=sys.stderr)
        return 2
    try:
        templates = load_templates()
    except FileNotFoundError as exc:
        print(f"init_project refused: {exc}", file=sys.stderr)
        return 2
    target.mkdir(parents=True, exist_ok=True)
    created: list[str] = []
    declaration = target / ".project.json"
    if not declaration.exists():
        declaration.write_text(json.dumps({
            "schema_version": 1,
            "id": project_id,
            "kind": args.kind,
            "profile": args.profile,
            "lifecycle": args.lifecycle,
            "verification": {"command": args.verification_command, "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
        }, indent=2) + "\n", encoding="utf-8")
        created.append(".project.json")
    for relative, content in templates.items():
        path = target / relative
        if path.exists():
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(render(content, project_id, args.profile, args.verification_command), encoding="utf-8")
        created.append(relative)
    if args.apply_metadata:
        try:
            metadata = build_discoverability(
                description=args.description or "",
                keywords=args.keywords or [],
                topics=args.topics or [],
                audience=args.audience or [],
                use_cases=args.use_cases or [],
                search_phrases=args.search_phrases or [],
                homepage=args.homepage or "",
                license_spdx=args.license_spdx,
                visibility=args.visibility or "",
            )
            created.extend(apply_local_metadata(target, metadata, args.license_spdx))
        except (ValueError, OSError, json.JSONDecodeError) as exc:
            print(f"init_project refused metadata repair: {exc}", file=sys.stderr)
            return 2
    created = list(dict.fromkeys(created))
    scaffold_files_created = {".project.json", ".ai-gate/gate.yaml"}.issubset(created)
    try:
        binder = run_gate_binder(
            target.parent,
            apply=True,
            only=[target.name],
            check_clean=not scaffold_files_created,
        )
    except Exception as exc:  # noqa: BLE001 - binder errors must fail initialization
        print(f"init_project failed to run gate binder for {target}: {exc}", file=sys.stderr)
        return 2
    if binder.names(REFUSED) or binder.names(DIRTY):
        detail = "; ".join(
            f"{outcome.repo}: {outcome.detail}"
            for outcome in binder.outcomes
            if outcome.status in (REFUSED, DIRTY)
        )
        print(
            "init_project failed: gate binder refused to update the manifest "
            f"({detail}); scaffold files are preserved",
            file=sys.stderr,
        )
        return 2
    unresolved = binder.unresolved()
    if unresolved:
        details = "; ".join(
            format_detail(outcome, verbose=True)
            for outcome in binder.outcomes
            if debt_concerns(outcome)
        )
        print(f"Gate binder completed with {len(unresolved)} unresolved check(s): {details}")
        print("The scaffold remains blocked until those checks are bound.")
    print(f"Initialized {project_id} at {target}")
    print("Created: " + (", ".join(created) if created else "nothing; existing files preserved"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
