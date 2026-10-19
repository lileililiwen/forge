#!/usr/bin/env python3
"""Verify the imported Workspace Governance tree and preserved Git history."""

from __future__ import annotations

import argparse
import os
import stat
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "modules" / "workspace-governance"
ALLOWED_SOURCE_ADAPTATIONS = {
    b"README.md",
    b"scripts/bind_gate_commands.py",
    b"scripts/product_code_quality.py",
    b"scripts/ui_conformance.py",
    b"scripts/workspace_check.py",
    b"scripts/generate_ui_conformance_baseline.py",
    b"ui-conformance-baseline.json",
    b"projects.json",
    b".project.json",
    b"tag-taxonomy.json",
    b"scripts/axis_migration.py",
    b"tests/test_identity_and_taxonomy.py",
    b"tests/test_bind_gate_commands.py",
    b"tests/test_capability_adoption_gate.py",
    b"tests/test_cross_repo_adoption_grant.py",
    b"tests/test_promote_project.py",
    b"tests/test_ui_conformance.py",
    b"tests/test_workspace_verification_baseline.py",
    b"MATURITY_PROGRESS_SUMMARY.md",
    b"openspec/changes/archive/2026-10-01-standalone-project-boundary/proposal.md",
    b"openspec/changes/archive/2026-10-12-bind-declared-gate-checks/proposal.md",
    b"openspec/changes/archive/2026-10-12-bind-declared-gate-checks/tasks.md",
    b"openspec/specs/deployable-batch-onboarding/spec.md",
    b"openspec/specs/forge-governance-adapter/spec.md",
    b"openspec/specs/legacy-gate-debt-remediation/spec.md",
    b"openspec/specs/platform-capability-manifest/spec.md",
    b"openspec/specs/readiness/spec.md",
    b"openspec/specs/registry-integrity/spec.md",
    b"openspec/specs/rules-baseline/spec.md",
    b"openspec/specs/ui-convention-check/spec.md",
    b"openspec/specs/ui-convention-guidance/spec.md",
    b"openspec/specs/workspace-decision-register/spec.md",
    b"openspec/specs/workspace-verification-baseline/spec.md",
}
WHITESPACE_ONLY_ADAPTATIONS = {
    b"MATURITY_PROGRESS_SUMMARY.md",
    b"openspec/changes/archive/2026-10-01-standalone-project-boundary/proposal.md",
    b"openspec/changes/archive/2026-10-12-bind-declared-gate-checks/proposal.md",
    b"openspec/changes/archive/2026-10-12-bind-declared-gate-checks/tasks.md",
    b"openspec/specs/deployable-batch-onboarding/spec.md",
    b"openspec/specs/forge-governance-adapter/spec.md",
    b"openspec/specs/legacy-gate-debt-remediation/spec.md",
    b"openspec/specs/platform-capability-manifest/spec.md",
    b"openspec/specs/readiness/spec.md",
    b"openspec/specs/registry-integrity/spec.md",
    b"openspec/specs/rules-baseline/spec.md",
    b"openspec/specs/ui-convention-check/spec.md",
    b"openspec/specs/ui-convention-guidance/spec.md",
    b"openspec/specs/workspace-decision-register/spec.md",
    b"openspec/specs/workspace-verification-baseline/spec.md",
}
ADDED_MODULE_FILES = {
    b"gate.toml",
    b"scripts/bind_gate_commands_declarations.py",
    b"scripts/bind_gate_commands_discovery.py",
    b"scripts/bind_gate_commands_execution.py",
    b"scripts/product_code_quality_claims.py",
    b"scripts/product_code_quality_scope.py",
    b"scripts/ui_conformance_cli.py",
    b"scripts/workspace_check_capabilities.py",
    b"scripts/workspace_check_cli.py",
    b"scripts/workspace_check_interfaces.py",
    b"scripts/workspace_check_openspec.py",
    b"scripts/workspace_check_project.py",
    b"scripts/workspace_check_projects.py",
    b"scripts/workspace_check_quality.py",
    b"scripts/workspace_check_standalone.py",
    b"tests/test_gate_manifest_contract.py",
    b"migration/forge-monorepo-path-reconciliation.json",
    b"tests/test_platform_module_paths.py",
    b"tests/workspace_root.py",
}
SOURCE_REVISION = "2c766c64eb54560fa898115dec27c32abbf4968e"
SOURCE_ONLY_COMMITS = (
    "b8808c4bcfb8c461f314ce85fb2dc3e2d6b7a86c",
    "d48ea3fc3a1f1f804ed4afb4219e43dd2740f9e3",
    "abf5c373184deb7d66ee14e9c36fbb626714dbf7",
    "a55fbb700c2a86e84e0d93ecba4000773df2ab23",
    "d6b628e72d478280908de47eda570e542820e91d",
    "2c766c64eb54560fa898115dec27c32abbf4968e",
)


def git(*args: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        ["git", "-C", str(ROOT), *args],
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def source_tree(revision: str) -> dict[bytes, tuple[bytes, bytes]]:
    result = git("ls-tree", "-r", "-z", "--full-tree", revision)
    files: dict[bytes, tuple[bytes, bytes]] = {}
    for entry in result.stdout.split(b"\0"):
        if not entry:
            continue
        metadata, path = entry.split(b"\t", 1)
        mode, kind, blob = metadata.split(b" ", 2)
        if kind != b"blob":
            continue
        files[path] = (mode, blob)
    return files


def normalized_whitespace(payload: bytes) -> bytes:
    lines = [line.rstrip(b" \t\r") for line in payload.splitlines()]
    return b"\n".join(lines).rstrip(b"\n") + b"\n"


def source_blob(entry: tuple[bytes, bytes]) -> bytes:
    return git("cat-file", "blob", entry[1].decode("ascii")).stdout


def imported_tree(module: Path) -> dict[bytes, tuple[bytes, bytes]]:
    files: dict[bytes, tuple[bytes, bytes]] = {}
    for path in module.rglob("*"):
        if ".driftwatch" in path.parts or "__pycache__" in path.parts or path.suffix == ".pyc":
            continue
        if not path.is_file() and not path.is_symlink():
            continue
        relative = os.fsencode(path.relative_to(module).as_posix())
        info = path.lstat()
        if stat.S_ISLNK(info.st_mode):
            mode = b"120000"
            payload = os.fsencode(os.readlink(path))
            blob = subprocess.run(
                ["git", "-C", str(ROOT), "hash-object", "--stdin"],
                input=payload,
                check=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
        else:
            mode = b"100755" if info.st_mode & 0o111 else b"100644"
            blob = git("hash-object", "--no-filters", str(path)).stdout.strip()
        files[relative] = (mode, blob)
    return files


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-revision", default=SOURCE_REVISION)
    parser.add_argument("--ancestry-revision", default="HEAD")
    parser.add_argument("--module-root", type=Path, default=MODULE)
    args = parser.parse_args()
    expected = source_tree(args.source_revision)
    actual = imported_tree(args.module_root.resolve())
    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected) - ADDED_MODULE_FILES)
    missing_additions = sorted(ADDED_MODULE_FILES - set(actual))
    changed = []
    for path in sorted(set(expected) & set(actual)):
        if expected[path] == actual[path]:
            continue
        if path not in ALLOWED_SOURCE_ADAPTATIONS or expected[path][0] != actual[path][0]:
            changed.append(path)
            continue
        if path in WHITESPACE_ONLY_ADAPTATIONS:
            original = source_blob(expected[path])
            imported = (args.module_root.resolve() / os.fsdecode(path)).read_bytes()
            if normalized_whitespace(original) != normalized_whitespace(imported):
                changed.append(path)
    if missing or extra or missing_additions or changed:
        for label, paths in (("missing", missing), ("extra", extra), ("mode/blob mismatch", changed)):
            for path in paths:
                print(f"{label}: {os.fsdecode(path)}", file=sys.stderr)
        for path in missing_additions:
            print(f"missing declared monorepo addition: {os.fsdecode(path)}", file=sys.stderr)
        return 1

    not_ancestors = []
    for commit in (args.source_revision, *SOURCE_ONLY_COMMITS):
        result = git("merge-base", "--is-ancestor", commit, args.ancestry_revision, check=False)
        if result.returncode != 0:
            not_ancestors.append(commit)
    if not_ancestors:
        for commit in not_ancestors:
            print(f"source commit is not an ancestor of HEAD: {commit}", file=sys.stderr)
        return 1
    print(
        f"PASS: {len(expected)} source paths match, with {len(ALLOWED_SOURCE_ADAPTATIONS)} "
        f"declared adaptations ({len(WHITESPACE_ONLY_ADAPTATIONS)} whitespace-only) "
        f"and {len(ADDED_MODULE_FILES)} additions; "
        "all 6 source-only commits are ancestors"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
