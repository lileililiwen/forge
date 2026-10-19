#!/usr/bin/env python3
"""Verify provenance and tracked-tree parity of the Rust platform module."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path
from typing import Mapping


ROOT = Path(__file__).resolve().parents[1]
MODULE_PREFIX = b"modules/rust-platform-libs/"
SOURCE_REVISION = "4762746e35b3886b21e2c87bf602aed14bc9dea8"
SOURCE_COMMIT_COUNT = 30
MOVED_PATHS = {
    b".github/workflows/ci.yml": b".github/workflows/platform-rust-quality.yml",
    b".github/workflows/publish.yml": b".github/workflows/platform-rust-publish.yml",
}
WHITESPACE_ONLY_ADAPTATIONS = {
    b"openspec/specs/platform-ai/spec.md",
    b"openspec/specs/rust-identity-audit-permission/spec.md",
    b"openspec/specs/rust-platform-foundation/spec.md",
    b"openspec/specs/rust-platform-publication/spec.md",
}
ADAPTED_SOURCE_PATHS = {
    b".project.json",
    b".ai-gate/gate.yaml",
    b"Cargo.toml",
    b"README.md",
    b"HANDOFF.md",
    b"ROADMAP.md",
    b"docs/consumer-baseline.md",
    b"docs/publication.md",
    b"docs/usage.md",
    b"crates/platform-contracts/src/lib.rs",
    b"crates/platform-contracts/tests/fixture_parity.rs",
    b"crates/platform-contracts/tests/typed_payload_parity.rs",
    *WHITESPACE_ONLY_ADAPTATIONS,
    *MOVED_PATHS,
}


def git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        check=False,
        capture_output=True,
    )
    if check and result.returncode != 0:
        detail = result.stderr.decode(errors="replace").strip()
        raise subprocess.CalledProcessError(result.returncode, result.args, result.stdout, result.stderr)
    return result


def source_tree_from_repository(root: Path, revision: str) -> dict[bytes, tuple[bytes, bytes]]:
    result = git(root, "ls-tree", "-rz", "--full-tree", "-r", revision)
    files: dict[bytes, tuple[bytes, bytes]] = {}
    for record in result.stdout.split(b"\0"):
        if not record:
            continue
        metadata, path = record.split(b"\t", 1)
        mode, object_type, object_id = metadata.split(b" ", 2)
        if object_type not in (b"blob", b"commit"):
            continue
        files[path] = (mode, object_id)
    return files


def source_tree(root: Path, revision: str) -> dict[bytes, tuple[bytes, bytes]]:
    return source_tree_from_repository(root, revision)


def indexed_files(root: Path, prefixes: tuple[bytes, ...]) -> dict[bytes, tuple[bytes, bytes]]:
    result = git(root, "ls-files", "--stage", "-z", "--", *(os.fsdecode(p) for p in prefixes))
    files: dict[bytes, tuple[bytes, bytes]] = {}
    for record in result.stdout.split(b"\0"):
        if not record:
            continue
        metadata, path = record.split(b"\t", 1)
        mode, object_id, _stage = metadata.split(b" ", 2)
        files[path] = (mode, object_id)
    return files


def compare_trees(
    source_files: Mapping[bytes, tuple[bytes, bytes]],
    destination_files: Mapping[bytes, tuple[bytes, bytes]],
    *,
    prefix: bytes,
    moved_paths: Mapping[bytes, bytes],
    adapted_source_paths: set[bytes],
    added_destination_paths: set[bytes],
) -> list[str]:
    problems: list[str] = []
    expected_destinations: set[bytes] = set(added_destination_paths)
    for source_path, source_entry in sorted(source_files.items()):
        destination_path = moved_paths.get(source_path, prefix + source_path)
        expected_destinations.add(destination_path)
        destination_entry = destination_files.get(destination_path)
        if destination_entry is None:
            problems.append(f"missing imported path {source_path.decode(errors='replace')}")
            continue
        if source_entry[0] != destination_entry[0]:
            problems.append(f"mode mismatch for {source_path.decode(errors='replace')}")
        if source_entry[1] != destination_entry[1] and source_path not in adapted_source_paths:
            problems.append(f"blob mismatch for {source_path.decode(errors='replace')}")

    for destination_path in sorted(set(destination_files) - expected_destinations):
        problems.append(f"unexpected imported path {destination_path.decode(errors='replace')}")
    for destination_path in sorted(added_destination_paths - set(destination_files)):
        problems.append(f"missing declared added path {destination_path.decode(errors='replace')}")
    return problems


def normalized_final_newlines(payload: bytes) -> bytes:
    """Remove only extra blank lines at EOF, preserving one final newline."""
    return payload.rstrip(b"\n") + b"\n" if payload else payload


def check_whitespace_adaptations(
    root: Path,
    source_files: Mapping[bytes, tuple[bytes, bytes]],
    destination_files: Mapping[bytes, tuple[bytes, bytes]],
    *,
    prefix: bytes,
    moved_paths: Mapping[bytes, bytes],
) -> list[str]:
    problems: list[str] = []
    for source_path in sorted(WHITESPACE_ONLY_ADAPTATIONS):
        source_entry = source_files.get(source_path)
        destination_path = moved_paths.get(source_path, prefix + source_path)
        destination_entry = destination_files.get(destination_path)
        if source_entry is None or destination_entry is None:
            continue
        if source_entry[1] == destination_entry[1]:
            continue
        source_content = git(root, "cat-file", "blob", os.fsdecode(source_entry[1])).stdout
        destination_content = git(root, "cat-file", "blob", os.fsdecode(destination_entry[1])).stdout
        if normalized_final_newlines(source_content) != normalized_final_newlines(destination_content):
            problems.append(f"non-whitespace content mismatch for {source_path.decode(errors='replace')}")
    return problems


def check_source_ancestry(root: Path, *, source_revision: str) -> list[str]:
    revision = git(root, "rev-parse", "--verify", f"{source_revision}^{{commit}}", check=False)
    if revision.returncode != 0:
        return [f"recorded source revision {source_revision} is unavailable"]
    candidates = ["HEAD", *pending_merge_heads(root)]
    commits = git(root, "rev-list", source_revision).stdout.splitlines()
    if len(commits) != SOURCE_COMMIT_COUNT:
        return [f"expected {SOURCE_COMMIT_COUNT} source commits, found {len(commits)}"]
    problems: list[str] = []
    for commit_bytes in commits:
        commit = commit_bytes.decode("ascii")
        preserved = any(
            git(root, "merge-base", "--is-ancestor", commit, candidate, check=False).returncode == 0
            for candidate in candidates
        )
        if not preserved:
            problems.append(f"source commit {commit} is not preserved in Forge history")
    return problems


def pending_merge_heads(root: Path) -> tuple[str, ...]:
    path_result = git(root, "rev-parse", "--git-path", "MERGE_HEAD")
    merge_path = Path(os.fsdecode(path_result.stdout.strip()))
    if not merge_path.is_absolute():
        merge_path = root / merge_path
    if not merge_path.exists():
        return ()
    return tuple(line.strip() for line in merge_path.read_text().splitlines() if line.strip())


def verify(root: Path) -> list[str]:
    problems = check_source_ancestry(root, source_revision=SOURCE_REVISION)
    try:
        imported = indexed_files(root, (MODULE_PREFIX.rstrip(b"/"), *MOVED_PATHS.values()))
        source = source_tree(root, SOURCE_REVISION)
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        return [*problems, f"cannot inspect imported tree: {error}"]

    problems.extend(
        compare_trees(
            source,
            imported,
            prefix=MODULE_PREFIX,
            moved_paths=MOVED_PATHS,
            adapted_source_paths=ADAPTED_SOURCE_PATHS,
            added_destination_paths=set(),
        )
    )
    problems.extend(
        check_whitespace_adaptations(
            root,
            source,
            imported,
            prefix=MODULE_PREFIX,
            moved_paths=MOVED_PATHS,
        )
    )
    if b".gitmodules" in indexed_files(root, (b".gitmodules",)):
        problems.append("Forge source import must not add a Git submodule")
    if any(mode == b"160000" for mode, _object_id in imported.values()):
        problems.append("imported module contains a Git submodule entry")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="Forge repository root")
    args = parser.parse_args()
    root = args.root.expanduser().resolve()
    problems = verify(root)
    if problems:
        for problem in problems:
            print(f"rust-platform-libs import: FAIL: {problem}", file=sys.stderr)
        return 1
    print(
        "rust-platform-libs import: PASS "
        f"({len(source_tree(root, SOURCE_REVISION))} source files, {SOURCE_COMMIT_COUNT} source commits)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
