#!/usr/bin/env python3
"""Verify the source history and tracked-tree parity of the .NET platform import."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path
from typing import Mapping


ROOT = Path(__file__).resolve().parents[1]
MODULE_PREFIX = b"modules/dotnet-platform-libs/"
SOURCE_REVISION = "d11fd9b41e48d966f5f807f0087b769ff01214e5"
SOURCE_BASELINE = "c740bd9d625f004978f058578e61ffcdf14bf8c2"
MOVED_PATHS = {
    b".github/workflows/quality.yml": b".github/workflows/platform-dotnet-quality.yml",
    b".github/workflows/release.yml": b".github/workflows/platform-dotnet-release.yml",
}
WHITESPACE_ONLY_ADAPTATIONS = {
    # Remove inherited extra trailing blank lines without allowing other content drift.
    b"openspec/changes/archive/2026-09-08-platform-observability/design.md",
    b"openspec/changes/archive/2026-09-08-platform-observability/proposal.md",
    b"openspec/changes/archive/2026-09-08-platform-observability/specs/platform-observability/spec.md",
    b"openspec/changes/archive/2026-09-08-platform-persistence-multitenancy/design.md",
    b"openspec/changes/archive/2026-09-08-platform-persistence-multitenancy/proposal.md",
    b"openspec/changes/archive/2026-09-08-platform-persistence-multitenancy/specs/platform-persistence-multitenancy/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-consumer-conformance/design.md",
    b"openspec/changes/archive/2026-09-09-platform-consumer-conformance/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-consumer-conformance/specs/platform-consumer-conformance/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-eventing-rabbitmq/design.md",
    b"openspec/changes/archive/2026-09-09-platform-eventing-rabbitmq/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-eventing-rabbitmq/specs/platform-eventing-rabbitmq/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-eventing-rabbitmq/tasks.md",
    b"openspec/changes/archive/2026-09-09-platform-featureflags-resilience/design.md",
    b"openspec/changes/archive/2026-09-09-platform-featureflags-resilience/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-featureflags-resilience/specs/platform-featureflags-resilience/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-identity-host-integration/design.md",
    b"openspec/changes/archive/2026-09-09-platform-identity-host-integration/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-identity-host-integration/specs/platform-identity-host-integration/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-jobs-hangfire/design.md",
    b"openspec/changes/archive/2026-09-09-platform-jobs-hangfire/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-jobs-hangfire/specs/platform-jobs-hangfire/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-mailing-providers/design.md",
    b"openspec/changes/archive/2026-09-09-platform-mailing-providers/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-mailing-providers/specs/platform-mailing-providers/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-quota-aspnetcore/design.md",
    b"openspec/changes/archive/2026-09-09-platform-quota-aspnetcore/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-quota-aspnetcore/specs/platform-quota-aspnetcore/spec.md",
    b"openspec/changes/archive/2026-09-09-platform-realtime/design.md",
    b"openspec/changes/archive/2026-09-09-platform-realtime/proposal.md",
    b"openspec/changes/archive/2026-09-09-platform-realtime/specs/platform-realtime/spec.md",
    b"openspec/specs/adoption-tooling/spec.md",
    b"openspec/specs/domain-primitives/spec.md",
    b"openspec/specs/dotnet-template-pack/spec.md",
    b"openspec/specs/dotnet10-platform-baseline/spec.md",
    b"openspec/specs/efcore-migrator/spec.md",
    b"openspec/specs/platform-aspnetcore-foundation/spec.md",
    b"openspec/specs/platform-consumer-conformance/spec.md",
    b"openspec/specs/platform-contract-conformance/spec.md",
    b"openspec/specs/platform-core-contracts/spec.md",
    b"openspec/specs/platform-entitlement-contracts/spec.md",
    b"openspec/specs/platform-eventing-rabbitmq/spec.md",
    b"openspec/specs/platform-eventing/spec.md",
    b"openspec/specs/platform-featureflags-resilience/spec.md",
    b"openspec/specs/platform-idempotency/spec.md",
    b"openspec/specs/platform-identity-authorization/spec.md",
    b"openspec/specs/platform-identity-host-integration/spec.md",
    b"openspec/specs/platform-jobs-hangfire/spec.md",
    b"openspec/specs/platform-jobs/spec.md",
    b"openspec/specs/platform-mailing-providers/spec.md",
    b"openspec/specs/platform-mailing/spec.md",
    b"openspec/specs/platform-observability/spec.md",
    b"openspec/specs/platform-persistence-efcore/spec.md",
    b"openspec/specs/platform-persistence-multitenancy/spec.md",
    b"openspec/specs/platform-privacy-aspnetcore/spec.md",
    b"openspec/specs/platform-privacy/spec.md",
    b"openspec/specs/platform-quota-aspnetcore/spec.md",
    b"openspec/specs/platform-ratelimiting/spec.md",
    b"openspec/specs/platform-realtime/spec.md",
    b"openspec/specs/platform-repository-foundation/spec.md",
    b"openspec/specs/platform-shared-auth-pages/spec.md",
    b"openspec/specs/platform-test-web-factory/spec.md",
    b"openspec/specs/platform-testing-postgres/spec.md",
    b"openspec/specs/platform-web-edge/spec.md",
    b"openspec/specs/platform-webhooks/spec.md",
    b"openspec/specs/readiness/spec.md",
    b"openspec/specs/sample-matrix/spec.md",
    b"openspec/specs/web-composition/spec.md",
}
ADAPTED_SOURCE_PATHS = WHITESPACE_ONLY_ADAPTATIONS | {
    # The opt-in ASP.NET Core OpenID Connect adapter
    # (`project-oidc-auth-adoption`). The module is the maintained source
    # after the import, so the package, its tests, the promoted canonical spec
    # and the module records that describe them carry the change that the
    # import commit did not contain.
    b"CHANGELOG.md",
    b"Directory.Packages.props",
    b"Platform.sln",
    b"docs/packages.md",
    b"docs/platform-identity.md",
    b"eng/package-manifest.json",
    b"tests/Platform.Architecture.Tests/UiDistributionTests.cs",
    b".project.json",
    b".ai-gate/gate.yaml",
    b"Directory.Build.props",
    b"README.md",
    b"HANDOFF.md",
    b"docs/release-governance.md",
    b"eng/consumer/NuGet.config",
    b"scripts/test-workspace-bootstrap.sh",
    b"tests/Platform.Architecture.Tests/DependencyDirectionTests.cs",
    b"tests/scripts/test_consumer_nuget.py",
    *MOVED_PATHS,
}
ADDED_DESTINATION_PATHS: set[bytes] = {
    # Per-project local files that the sibling checkout kept untracked and
    # that the module owns now that the source lives in Forge.
    b"modules/dotnet-platform-libs/driftwatch.toml",
    b"modules/dotnet-platform-libs/verify-local.sh",
    # The opt-in ASP.NET Core OpenID Connect adapter package, its tests and
    # the promoted canonical spec, added by `project-oidc-auth-adoption`.
    b"modules/dotnet-platform-libs/openspec/changes/archive/2026-10-19-project-oidc-auth-adoption/design.md",
    b"modules/dotnet-platform-libs/openspec/changes/archive/2026-10-19-project-oidc-auth-adoption/proposal.md",
    b"modules/dotnet-platform-libs/openspec/changes/archive/2026-10-19-project-oidc-auth-adoption/specs/project-oidc-auth-adoption/spec.md",
    b"modules/dotnet-platform-libs/openspec/changes/archive/2026-10-19-project-oidc-auth-adoption/tasks.md",
    b"modules/dotnet-platform-libs/openspec/specs/project-oidc-auth-adoption/spec.md",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/Platform.Identity.OpenIdConnect.AspNetCore.csproj",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformIdentityOpenIdConnectAssemblyMarker.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectBackchannelHandler.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectEventBinder.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectFailures.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectIdentityResolver.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectOptions.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectOptionsValidator.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectSecretResolver.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectServiceCollectionExtensions.cs",
    b"modules/dotnet-platform-libs/src/Platform.Identity.OpenIdConnect.AspNetCore/PlatformOpenIdConnectStateReplayGuard.cs",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/LocalOidcTestIssuer.cs",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/OidcTestApp.cs",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests.csproj",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/PlatformOpenIdConnectCallbackTests.cs",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/PlatformOpenIdConnectOptionsTests.cs",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/PlatformOpenIdConnectOwnershipTests.cs",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/PlatformOpenIdConnectSecretResolverTests.cs",
    b"modules/dotnet-platform-libs/tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/PlatformOpenIdConnectStateReplayGuardTests.cs",
}


def git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        ["git", "-C", str(root), *args],
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def source_tree(root: Path, revision: str) -> dict[bytes, tuple[bytes, bytes]]:
    result = git(root, "ls-tree", "-r", "-z", "--full-tree", revision)
    files: dict[bytes, tuple[bytes, bytes]] = {}
    for entry in result.stdout.split(b"\0"):
        if not entry:
            continue
        metadata, path = entry.split(b"\t", 1)
        mode, kind, blob = metadata.split(b" ", 2)
        if kind == b"blob":
            files[path] = (mode, blob)
    return files


def indexed_files(root: Path, paths: tuple[bytes, ...]) -> dict[bytes, tuple[bytes, bytes]]:
    result = git(root, "ls-files", "--stage", "-z", "--", *(os.fsdecode(path) for path in paths))
    files: dict[bytes, tuple[bytes, bytes]] = {}
    for entry in result.stdout.split(b"\0"):
        if not entry:
            continue
        metadata, path = entry.split(b"\t", 1)
        mode, blob, stage = metadata.split(b" ", 2)
        if stage != b"0":
            raise ValueError(f"unmerged index entry for {os.fsdecode(path)}")
        files[path] = (mode, blob)
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
    expected: dict[bytes, tuple[bytes, bytes, bytes]] = {}
    for source_path, (source_mode, source_blob) in source_files.items():
        destination_path = moved_paths.get(source_path, prefix + source_path)
        if destination_path in expected:
            problems.append(f"duplicate destination mapping for {destination_path!r}")
            continue
        expected[destination_path] = (source_mode, source_blob, source_path)

    for path, (source_mode, source_blob, source_path) in expected.items():
        imported = destination_files.get(path)
        if imported is None:
            problems.append(f"missing imported path {path.decode(errors='replace')} from {source_path.decode(errors='replace')}")
            continue
        imported_mode, imported_blob = imported
        if imported_mode != source_mode:
            problems.append(f"mode mismatch for {path.decode(errors='replace')}: source {source_mode.decode()} imported {imported_mode.decode()}")
        if source_path not in adapted_source_paths and imported_blob != source_blob:
            problems.append(f"blob mismatch for {source_path.decode(errors='replace')}")

    expected_paths = set(expected)
    unexpected = set(destination_files) - expected_paths - added_destination_paths
    for path in sorted(unexpected):
        problems.append(f"unexpected imported path {path.decode(errors='replace')}")
    for path in sorted(added_destination_paths - set(destination_files)):
        problems.append(f"missing declared added path {path.decode(errors='replace')}")
    return problems


def normalized_final_newlines(payload: bytes) -> bytes:
    """Remove inherited extra blank lines at EOF without masking other drift."""
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
            continue  # compare_trees reports missing source/destination paths.
        if source_entry[1] == destination_entry[1]:
            continue
        source_content = git(root, "cat-file", "blob", os.fsdecode(source_entry[1])).stdout
        destination_content = git(root, "cat-file", "blob", os.fsdecode(destination_entry[1])).stdout
        if normalized_final_newlines(source_content) != normalized_final_newlines(destination_content):
            problems.append(f"non-whitespace content mismatch for {source_path.decode(errors='replace')}")
    return problems


def check_source_ancestry(
    root: Path,
    *,
    source_revision: str,
    source_baseline: str,
    pending_merge_heads: tuple[str, ...],
) -> list[str]:
    problems: list[str] = []
    baseline_result = git(root, "merge-base", "--is-ancestor", source_baseline, source_revision, check=False)
    if baseline_result.returncode != 0:
        return ["source baseline is not an ancestor of the recorded source revision"]

    candidates = ["HEAD", *pending_merge_heads]
    for message, commit in (("source revision", source_revision),):
        preserved = any(git(root, "merge-base", "--is-ancestor", commit, candidate, check=False).returncode == 0 for candidate in candidates)
        if not preserved:
            problems.append(f"{message} {commit} is not an ancestor of Forge HEAD or a pending merge parent")

    source_only = git(root, "rev-list", source_revision, f"^{source_baseline}").stdout.splitlines()
    for commit_bytes in source_only:
        commit = commit_bytes.decode("ascii")
        preserved = any(git(root, "merge-base", "--is-ancestor", commit, candidate, check=False).returncode == 0 for candidate in candidates)
        if not preserved:
            problems.append(f"source-only commit {commit} is not preserved in Forge history")
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
    problems = check_source_ancestry(
        root,
        source_revision=SOURCE_REVISION,
        source_baseline=SOURCE_BASELINE,
        pending_merge_heads=pending_merge_heads(root),
    )
    try:
        imported = indexed_files(
            root,
            (MODULE_PREFIX.rstrip(b"/"), *MOVED_PATHS.values()),
        )
        source = source_tree(root, SOURCE_REVISION)
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        return [*problems, f"cannot inspect staged import: {error}"]

    problems.extend(
        compare_trees(
            source,
            imported,
            prefix=MODULE_PREFIX,
            moved_paths=MOVED_PATHS,
            adapted_source_paths=ADAPTED_SOURCE_PATHS,
            added_destination_paths=ADDED_DESTINATION_PATHS,
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
    if any(mode == b"160000" for mode, _blob in imported.values()):
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
            print(f"dotnet-platform-libs import: FAIL: {problem}", file=sys.stderr)
        return 1
    count = len(source_tree(root, SOURCE_REVISION))
    source_only_count = len(git(root, "rev-list", SOURCE_REVISION, f"^{SOURCE_BASELINE}").stdout.splitlines())
    print(f"dotnet-platform-libs import: PASS ({count} source files, {source_only_count} source-only commits)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
