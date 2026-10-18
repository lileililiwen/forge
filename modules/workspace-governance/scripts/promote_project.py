#!/usr/bin/env python3
"""Promote or demote a project in the deployment ledger.

Promotion is the only path that flips `deployment.deployable` to `true`. It
runs the project audit before and after the mutation, refuses to overwrite
conflicting manual values without `--force`, and appends a tamper-evident
entry to `deployment/promotions.json`. Demotion is the symmetric reverse
and records a `demote` entry; both kinds coexist in the ledger for history.

The script never runs a project verification command. It only records the
evidence reference the operator supplies, so the audit boundary stays with
the project's own verification pipeline.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from types import SimpleNamespace
from typing import Any

from workspace_check import Finding, run as run_workspace_check  # noqa: E402


SCHEMA_VERSION = "promotions/1"
COMPOSE_CANDIDATES = ("docker-compose.yml", "compose.yml")
EXIT_OK = 0
EXIT_REJECTED = 2


class PromotionError(Exception):
    """Raised when a promotion is rejected before any state change."""


class LedgerTamperError(Exception):
    """Raised when the on-disk ledger does not match the recorded sidecar."""


def ledger_path(governance_root: Path) -> Path:
    return governance_root / "deployment" / "promotions.json"


def ledger_sidecar_path(governance_root: Path) -> Path:
    return governance_root / "deployment" / "promotions.sha256"


def compute_ledger_hash(entries: list[dict]) -> str:
    canonical = json.dumps(entries, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def load_ledger(governance_root: Path, allow_create: bool = True) -> dict:
    path = ledger_path(governance_root)
    sidecar = ledger_sidecar_path(governance_root)
    if not path.is_file():
        if allow_create:
            return {"schema": SCHEMA_VERSION, "entries": []}
        raise PromotionError(f"missing ledger file: {path}")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PromotionError(f"invalid ledger JSON {path}: {exc}")
    if not isinstance(payload, dict) or payload.get("schema") != SCHEMA_VERSION:
        raise PromotionError(f"ledger schema mismatch in {path}")
    entries = payload.get("entries")
    if not isinstance(entries, list):
        raise PromotionError(f"ledger entries must be a list in {path}")
    if sidecar.is_file():
        expected = sidecar.read_text(encoding="utf-8").strip()
        actual = compute_ledger_hash(entries)
        if expected != actual:
            raise LedgerTamperError(
                f"ledger entries do not match recorded sidecar at {sidecar}; "
                "refusing to read or append to a tampered ledger"
            )
    return payload


def write_ledger(governance_root: Path, payload: dict) -> None:
    path = ledger_path(governance_root)
    sidecar = ledger_sidecar_path(governance_root)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    tmp.replace(path)
    sidecar.write_text(compute_ledger_hash(payload["entries"]), encoding="utf-8")


def append_promotion(governance_root: Path, entry: dict) -> dict:
    payload = load_ledger(governance_root, allow_create=True)
    payload["entries"].append(entry)
    write_ledger(governance_root, payload)
    return payload


def detect_compose_file(project_path: Path) -> str | None:
    """Return the first existing compose candidate in the project root, else None."""
    for candidate in COMPOSE_CANDIDATES:
        if (project_path / candidate).is_file():
            return candidate
    return None


def detect_git_revision(project_path: Path) -> str:
    """Return the project's HEAD revision if .git is present, else 'unknown'."""
    head = project_path / ".git" / "HEAD"
    if not head.is_file():
        return "unknown"
    raw = head.read_text(encoding="utf-8").strip()
    # Detached HEAD form
    if raw and not raw.startswith("ref:"):
        return raw
    # Symbolic ref form: "ref: refs/heads/main"
    if raw.startswith("ref:"):
        ref = raw.split(":", 1)[1].strip()
        ref_path = project_path / ".git" / ref
        if ref_path.is_file():
            return ref_path.read_text(encoding="utf-8").strip()
    return "unknown"


def resolve_workspace_root(governance_root: Path, registry: dict) -> Path:
    configured = os.environ.get("WORKSPACE_ROOT") or registry.get("workspace_root")
    if configured:
        return Path(configured).expanduser().resolve()
    return Path.cwd().resolve()


def find_registry_entry(registry: dict, project_id: str) -> dict | None:
    for entry in registry.get("projects", []):
        if entry.get("id") == project_id:
            return entry
    return None


def load_project_declaration(project_path: Path) -> dict:
    declaration_path = project_path / ".project.json"
    if not declaration_path.is_file():
        raise PromotionError(f"missing declaration: {declaration_path}")
    return json.loads(declaration_path.read_text(encoding="utf-8"))


def write_declaration(project_path: Path, declaration: dict) -> None:
    declaration_path = project_path / ".project.json"
    tmp = declaration_path.with_suffix(declaration_path.suffix + ".tmp")
    tmp.write_text(json.dumps(declaration, indent=2) + "\n", encoding="utf-8")
    tmp.replace(declaration_path)


def backup(project_path: Path, governance_root: Path) -> dict[str, Path]:
    """Snapshot files we may need to roll back."""
    backups: dict[str, Path] = {}
    declaration = project_path / ".project.json"
    backups[str(declaration)] = Path(shutil.copy2(declaration, declaration.with_suffix(".json.bak")))
    ledger = ledger_path(governance_root)
    if ledger.is_file():
        backups[str(ledger)] = Path(shutil.copy2(ledger, ledger.with_suffix(".json.bak")))
    return backups


def snapshot(governance_root: Path, *paths: Path) -> dict[str, Path]:
    """Backup the given paths in their current state (if they exist).

    The returned dict maps original paths to backup paths. If a file did not
    exist, the backup value is `None` and rollback will delete the file
    rather than restore from a missing source.
    """
    backups: dict[str, Path] = {}
    for original in paths:
        original = Path(original)
        if not original.is_file():
            backups[str(original)] = None
            continue
        backup_path = original.with_suffix(original.suffix + ".bak")
        backups[str(original)] = Path(shutil.copy2(original, backup_path))
    return backups


def rollback(backups: dict[str, Path | None]) -> None:
    for original, backup_path in backups.items():
        target = Path(original)
        if backup_path is None:
            # The file did not exist before; remove the post-write version.
            target.unlink(missing_ok=True)
        else:
            shutil.copy2(backup_path, target)


def cleanup_backups(backups: dict[str, Path | None]) -> None:
    for backup_path in backups.values():
        if backup_path is None:
            continue
        Path(backup_path).unlink(missing_ok=True)


def audit_project(registry: str, workspace_root: Path, project_id: str) -> tuple[list[Finding], dict[str, int]]:
    return run_workspace_check(
        SimpleNamespace(
            registry=registry,
            root=str(workspace_root),
            project=project_id,
            audit=False,
            self_check=False,
            as_json=False,
        )
    )


def main(args: argparse.Namespace | str | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project_id", help="registered project id to promote or demote")
    parser.add_argument("--evidence-ref", default="", help="evidence pointer for the promotion (required unless --demote)")
    parser.add_argument("--compose-file", default=None, help="override the detected compose file name")
    parser.add_argument("--jenkins-job", default=None, help="override the Jenkins job name (defaults to the project id)")
    parser.add_argument("--actor", default=os.environ.get("USER", "agent"), help="decision maker recorded in the ledger")
    parser.add_argument("--force", action="store_true", help="overwrite conflicting manual deployment values")
    parser.add_argument("--demote", action="store_true", help="reverse a previous promotion and record a demote entry")
    parser.add_argument("--registry", default=None, help="path to projects.json (defaults to governance registry)")
    parser.add_argument("--governance-root", default=None, help="workspace-governance repository root (defaults to scripts/..)")
    parser.add_argument("--workspace-root", default=None, help="workspace root (defaults to WORKSPACE_ROOT or registry.workspace_root)")
    parser.add_argument("--quiet", action="store_true", help="suppress non-error output")
    if isinstance(args, argparse.Namespace):
        # Direct call from tests; skip argparse parsing.
        return _run(args)
    parsed = parser.parse_args(args)
    return _run(parsed)


def _run(args: argparse.Namespace) -> int:
    governance_root = Path(args.governance_root).expanduser().resolve() if args.governance_root else Path(__file__).resolve().parents[1]
    registry_path = Path(args.registry).expanduser().resolve() if args.registry else governance_root / "projects.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    workspace_root = Path(args.workspace_root).expanduser().resolve() if args.workspace_root else resolve_workspace_root(governance_root, registry)
    entry = find_registry_entry(registry, args.project_id)
    if entry is None:
        if not args.quiet:
            print(f"unknown project id: {args.project_id}", file=sys.stderr)
        return EXIT_REJECTED
    if entry.get("adoption") != "adopted":
        if not args.quiet:
            print(f"project is not adopted: {args.project_id}", file=sys.stderr)
        return EXIT_REJECTED
    project_path = workspace_root / entry["path"]
    declaration_path = project_path / ".project.json"
    if not declaration_path.is_file():
        if not args.quiet:
            print(f"missing declaration: {declaration_path}", file=sys.stderr)
        return EXIT_REJECTED
    declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
    deployment = declaration.get("deployment")
    if not isinstance(deployment, dict):
        if not args.quiet:
            print(f"deployment block missing or invalid: {declaration_path}", file=sys.stderr)
        return EXIT_REJECTED

    # Pre-audit: refuse on existing errors
    pre_findings, pre_summary = audit_project(str(registry_path), workspace_root, args.project_id)
    if pre_summary["errors"]:
        if not args.quiet:
            print(f"project audit fails for {args.project_id}; refusing to promote", file=sys.stderr)
            for finding in pre_findings:
                if finding.level == "ERROR":
                    print(f"  {finding.code}: {finding.message}", file=sys.stderr)
        return EXIT_REJECTED

    backups = backup(project_path, governance_root)
    try:
        if args.demote:
            return _demote(args, governance_root, project_path, declaration, deployment, registry_path, workspace_root, backups)
        return _promote(args, governance_root, project_path, declaration, deployment, registry_path, workspace_root, backups)
    except PromotionError as exc:
        rollback(backups)
        cleanup_backups(backups)
        if not args.quiet:
            print(f"promotion rejected: {exc}", file=sys.stderr)
        return EXIT_REJECTED
    except LedgerTamperError as exc:
        rollback(backups)
        cleanup_backups(backups)
        if not args.quiet:
            print(f"ledger tamper detected: {exc}", file=sys.stderr)
        return EXIT_REJECTED


def _promote(
    args: argparse.Namespace,
    governance_root: Path,
    project_path: Path,
    declaration: dict,
    deployment: dict,
    registry_path: Path,
    workspace_root: Path,
    backups: dict[str, Path],
) -> int:
    if not args.evidence_ref.strip():
        raise PromotionError("--evidence-ref is required for promotion")
    compose_file = args.compose_file or detect_compose_file(project_path)
    if compose_file is None:
        raise PromotionError(
            f"no compose file found in {project_path}; pass --compose-file to override"
        )
    if not (project_path / compose_file).is_file():
        raise PromotionError(f"declared compose file is missing: {compose_file}")
    jenkins_job = args.jenkins_job or args.project_id
    if not _is_safe_jenkins_job(jenkins_job):
        raise PromotionError(f"invalid Jenkins job name: {jenkins_job!r}")

    current_deployable = bool(deployment.get("deployable"))
    current_compose = deployment.get("compose_file")
    current_job = deployment.get("jenkins_job")
    target_deployment = {
        "deployable": True,
        "jenkins_job": jenkins_job,
        "compose_file": compose_file,
    }
    if not args.force:
        # A "manual" declaration is anything other than the promotion-default
        # initial state of (False, None, None). The conflict guard exists to
        # protect human-edited values, not just currently-deployable ones:
        # a project that was hand-declared with a different compose file or
        # Jenkins job must not be silently overwritten by the detected pair.
        has_manual_value = bool(current_deployable) or current_compose is not None or current_job is not None
        conflicts = []
        if has_manual_value and current_compose and current_compose != compose_file:
            conflicts.append(f"compose_file {current_compose!r} != detected {compose_file!r}")
        if has_manual_value and current_job and current_job != jenkins_job:
            conflicts.append(f"jenkins_job {current_job!r} != default {jenkins_job!r}")
        if conflicts:
            raise PromotionError(
                "conflicting manual deployment values; pass --force to overwrite: "
                + "; ".join(conflicts)
            )

    revision = detect_git_revision(project_path)

    # Idempotency: refuse to append a duplicate promote entry for the same
    # project_id+revision+evidence_ref combination.
    ledger = load_ledger(governance_root, allow_create=True)
    for existing in ledger["entries"]:
        if (
            existing.get("project_id") == args.project_id
            and existing.get("kind") == "promote"
            and existing.get("revision") == revision
            and existing.get("evidence_ref") == args.evidence_ref
        ):
            # Already promoted to the same state; no work to do.
            cleanup_backups(backups)
            if not args.quiet:
                print(
                    f"{args.project_id}: already promoted at revision {revision} with this evidence ref; ledger is unchanged",
                )
            return EXIT_OK

    # Mutate the declaration
    deployment.clear()
    deployment.update(target_deployment)
    write_declaration(project_path, declaration)

    # Snapshot the ledger right before appending so a post-audit rollback
    # can restore the pre-append state even if the ledger did not exist when
    # `_run` took the initial backup.
    ledger_snapshots = snapshot(governance_root, ledger_path(governance_root))

    # Append the ledger entry
    entry = {
        "project_id": args.project_id,
        "decided_at": datetime.now(timezone.utc).isoformat(),
        "actor": args.actor,
        "revision": revision,
        "evidence_ref": args.evidence_ref,
        "manifest_generated": True,
        "kind": "promote",
        "compose_file": compose_file,
        "jenkins_job": jenkins_job,
    }
    append_promotion(governance_root, entry)

    # Post-audit: roll back on any new ERROR (e.g. missing release_evidence)
    post_findings, post_summary = audit_project(str(registry_path), workspace_root, args.project_id)
    if post_summary["errors"]:
        rollback(backups)
        rollback(ledger_snapshots)
        cleanup_backups(ledger_snapshots)
        if not args.quiet:
            print(
                f"post-promotion audit fails for {args.project_id}; rolled back",
                file=sys.stderr,
            )
            for finding in post_findings:
                if finding.level == "ERROR":
                    print(f"  {finding.code}: {finding.message}", file=sys.stderr)
        return EXIT_REJECTED

    cleanup_backups(ledger_snapshots)
    if not args.quiet:
        print(
            f"promoted {args.project_id}: deployable=true compose={compose_file} job={jenkins_job} revision={revision}",
        )
        print("next: regenerate the manifest with `python3 scripts/jenkins_manifest.py`")
    return EXIT_OK


def _demote(
    args: argparse.Namespace,
    governance_root: Path,
    project_path: Path,
    declaration: dict,
    deployment: dict,
    registry_path: Path,
    workspace_root: Path,
    backups: dict[str, Path],
) -> int:
    revision = detect_git_revision(project_path)
    deployment["deployable"] = False
    write_declaration(project_path, declaration)
    ledger_snapshots = snapshot(governance_root, ledger_path(governance_root))
    entry = {
        "project_id": args.project_id,
        "decided_at": datetime.now(timezone.utc).isoformat(),
        "actor": args.actor,
        "revision": revision,
        "evidence_ref": args.evidence_ref or "demote",
        "manifest_generated": True,
        "kind": "demote",
    }
    append_promotion(governance_root, entry)

    post_findings, post_summary = audit_project(str(registry_path), workspace_root, args.project_id)
    if post_summary["errors"]:
        rollback(backups)
        rollback(ledger_snapshots)
        cleanup_backups(ledger_snapshots)
        if not args.quiet:
            print(
                f"post-demotion audit fails for {args.project_id}; rolled back",
                file=sys.stderr,
            )
        return EXIT_REJECTED

    cleanup_backups(ledger_snapshots)
    if not args.quiet:
        print(f"demoted {args.project_id}: deployable=false (ledger entry appended)")
    return EXIT_OK


def _is_safe_jenkins_job(name: str) -> bool:
    import re
    return bool(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]*", name))


if __name__ == "__main__":
    raise SystemExit(main())
