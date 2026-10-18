#!/usr/bin/env python3
"""Check or explicitly publish repository metadata through the GitHub CLI."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Callable
from urllib.parse import urlparse


class GitHubCommandError(RuntimeError):
    pass


Runner = Callable[[list[str], Path], tuple[int, str, str]]


def run_command(command: list[str], cwd: Path) -> tuple[int, str, str]:
    completed = subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)
    return completed.returncode, completed.stdout, completed.stderr


def require_authentication(project_path: Path, runner: Runner = run_command) -> None:
    code, _, stderr = runner(["gh", "auth", "status"], project_path)
    if code != 0:
        raise GitHubCommandError(f"GITHUB_AUTH_REQUIRED: run `gh auth login` manually ({stderr.strip() or 'gh auth status failed'})")


def build_edit_command(repository: str, description: str, homepage: str, topics: list[str]) -> list[str]:
    command = ["gh", "repo", "edit", repository, "--description", description, "--homepage", homepage]
    for topic in topics:
        command.extend(["--add-topic", topic])
    return command


def _remote_slug(project_path: Path, runner: Runner) -> str | None:
    code, stdout, _ = runner(["git", "remote", "get-url", "origin"], project_path)
    if code != 0:
        return None
    remote = stdout.strip()
    if remote.startswith("git@github.com:"):
        return remote.removeprefix("git@github.com:").removesuffix(".git")
    parsed = urlparse(remote)
    if parsed.hostname == "github.com":
        return parsed.path.strip("/").removesuffix(".git")
    return None


def _load_metadata(project_path: Path) -> tuple[str, dict]:
    declaration = json.loads((project_path / ".project.json").read_text(encoding="utf-8"))
    metadata = declaration.get("discoverability")
    if not isinstance(metadata, dict):
        raise GitHubCommandError("DISCOVERABILITY_MISSING: local metadata must be repaired before GitHub publication")
    return str(metadata.get("visibility", "")), metadata


def build_create_command(repository: str, project_path: Path, visibility: str) -> list[str]:
    if visibility not in {"public", "private", "internal"}:
        raise GitHubCommandError("GITHUB_IDENTITY_AMBIGUOUS: visibility must be public, private, or internal")
    return ["gh", "repo", "create", repository, "--source=.", "--remote=origin", "--push", f"--{visibility}"]


def check_repository(project_path: Path, runner: Runner = run_command) -> dict:
    require_authentication(project_path, runner)
    visibility, metadata = _load_metadata(project_path)
    repository = _remote_slug(project_path, runner)
    if not repository:
        raise GitHubCommandError("GITHUB_REMOTE_MISSING: origin must point to a GitHub repository")
    code, stdout, stderr = runner(
        ["gh", "repo", "view", repository, "--json", "nameWithOwner,description,homepageUrl,visibility,repositoryTopics,defaultBranchRef"],
        project_path,
    )
    if code != 0:
        raise GitHubCommandError(f"GITHUB_REPOSITORY_UNAVAILABLE: {stderr.strip() or 'gh repo view failed'}")
    remote = json.loads(stdout)
    remote_topics = sorted(
        topic.get("name", "") if isinstance(topic, dict) else str(topic)
        for topic in remote.get("repositoryTopics", [])
    )
    differences = {}
    if remote.get("description") != metadata["description"]:
        differences["description"] = {"local": metadata["description"], "remote": remote.get("description")}
    if (remote.get("homepageUrl") or "") != metadata["homepage"]:
        differences["homepage"] = {"local": metadata["homepage"], "remote": remote.get("homepageUrl")}
    local_topics = sorted(metadata["topics"])
    if not set(local_topics).issubset(remote_topics):
        differences["topics"] = {"local": local_topics, "remote": remote_topics}
    if str(remote.get("visibility", "")).lower() != visibility:
        differences["visibility"] = {"local": visibility, "remote": remote.get("visibility")}
    return {"repository": repository, "differences": differences, "remote": remote, "state": "metadata_verified" if not differences else "drift"}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--apply", action="store_true")
    mode.add_argument("--publish", action="store_true")
    parser.add_argument("--owner")
    parser.add_argument("--repository")
    parser.add_argument("--json", action="store_true", dest="as_json")
    args = parser.parse_args(argv)
    project_path = args.project.resolve()
    try:
        require_authentication(project_path)
        visibility, metadata = _load_metadata(project_path)
        repository = _remote_slug(project_path, run_command)
        if args.publish:
            owner = args.owner or (repository.split("/", 1)[0] if repository else None)
            name = args.repository or (repository.split("/", 1)[1] if repository and "/" in repository else project_path.name)
            if not owner or not name:
                raise GitHubCommandError("GITHUB_IDENTITY_AMBIGUOUS: pass --owner and --repository")
            target = f"{owner}/{name}"
            if repository and repository != target:
                raise GitHubCommandError("GITHUB_IDENTITY_AMBIGUOUS: existing origin conflicts with requested repository")
            if not repository:
                code, _, stderr = run_command(build_create_command(target, project_path, visibility), project_path)
                if code != 0:
                    raise GitHubCommandError(stderr.strip() or "gh repo create failed")
            else:
                code, _, stderr = run_command(["git", "push", "-u", "origin", "main"], project_path)
                if code != 0:
                    raise GitHubCommandError(stderr.strip() or "git push failed")
            repository = target
        elif not repository:
            raise GitHubCommandError("GITHUB_REMOTE_MISSING: origin must point to a GitHub repository")
        if args.apply or args.publish:
            command = build_edit_command(repository, metadata["description"], metadata["homepage"], metadata["topics"])
            code, _, stderr = run_command(command, project_path)
            if code != 0:
                raise GitHubCommandError(stderr.strip() or "gh repo edit failed")
        result = check_repository(project_path)
        if (args.apply or args.publish) and not result["differences"]:
            declaration_path = project_path / ".project.json"
            declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
            declaration["publication"] = {
                "provider": "github",
                "owner": result["repository"].split("/", 1)[0],
                "repository": result["repository"].split("/", 1)[1],
                "default_branch": result["remote"].get("defaultBranchRef", {}).get("name", "main"),
                "state": "metadata_verified",
                "evidence_ref": "HANDOFF.md",
            }
            declaration_path.write_text(json.dumps(declaration, indent=2) + "\n", encoding="utf-8")
        if args.as_json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"{result['state']}: {result['repository']}")
            if result["differences"]:
                print(json.dumps(result["differences"], indent=2, sort_keys=True))
        return 1 if result["differences"] else 0
    except (GitHubCommandError, OSError, json.JSONDecodeError) as exc:
        print(str(exc), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
