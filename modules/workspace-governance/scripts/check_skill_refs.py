#!/usr/bin/env python3
"""Reference-integrity check for the governance front-door skill.

Walks `skills/workspace-governance/SKILL.md` and
`skills/workspace-governance/references/*.md`, extracts every backticked
repo path and every `python3 scripts/*.py` command, and exits non-zero
naming the reference and the missing path when one does not exist.

Only backticked spans are checked: prose names (sibling repositories,
external tools such as `openspec` or `forge`) are descriptive and are
skipped. Spans starting with `~`, `/`, or `http` are install paths or URLs
outside the repository and are skipped as well.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


SKILL_DIRNAME = Path("skills/workspace-governance")
SKILL_FILENAME = "SKILL.md"
REFERENCES_DIRNAME = "references"

BACKTICK_SPAN = re.compile(r"`([^`\n]+)`")
PYTHON_SCRIPT_COMMAND = re.compile(r"python3\s+(scripts/[^\s`]+)")
CHECKED_SUFFIXES = (".py", ".md", ".json", ".yaml", ".yml", ".mjs", ".toml", ".txt")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def skill_files(root: Path) -> tuple[Path, list[Path], list[str]]:
    """Return (skill_file, reference_files, errors)."""
    errors: list[str] = []
    skill_dir = root / SKILL_DIRNAME
    skill_file = skill_dir / SKILL_FILENAME
    if not skill_file.is_file():
        errors.append(f"missing skill entry: {skill_dir.as_posix()}/{SKILL_FILENAME}")
        return skill_file, [], errors
    references_dir = skill_dir / REFERENCES_DIRNAME
    if not references_dir.is_dir():
        errors.append(f"missing references directory: {(skill_dir / REFERENCES_DIRNAME).as_posix()}/")
        return skill_file, [], errors
    references = sorted(references_dir.glob("*.md"))
    if not references:
        errors.append(f"no references in {(skill_dir / REFERENCES_DIRNAME).as_posix()}/")
    return skill_file, references, errors


def cited_paths(text: str) -> list[str]:
    """Extract checkable repo paths from backticked spans in `text`."""
    found: list[str] = []
    for span in BACKTICK_SPAN.findall(text):
        candidate = span.strip()
        command = PYTHON_SCRIPT_COMMAND.search(candidate)
        if command:
            found.append(command.group(1))
            continue
        if not candidate or " " in candidate or candidate.startswith(("~", "/", "http", "$", "<")):
            continue
        if candidate.startswith("scripts/") and candidate.endswith(".py"):
            found.append(candidate)
            continue
        if "/" in candidate and candidate.endswith(CHECKED_SUFFIXES):
            found.append(candidate)
            continue
    return found


def check_file(path: Path, root: Path) -> list[str]:
    errors: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        return [f"{path.relative_to(root)}: cannot read reference: {exc}"]
    display = path.relative_to(root).as_posix()
    for candidate in cited_paths(text):
        target = (root / candidate).resolve()
        try:
            inside = target.is_relative_to(root.resolve())
        except AttributeError:  # Python < 3.9 fallback, unreachable on 3.11
            inside = str(target).startswith(str(root.resolve()))
        if not inside or not target.exists():
            errors.append(f"{display}: cited path does not exist: `{candidate}`")
    return errors


def check(root: Path | None = None) -> list[str]:
    root = root or repo_root()
    skill_file, references, errors = skill_files(root)
    if errors:
        return errors
    errors.extend(check_file(skill_file, root))
    for reference in references:
        errors.extend(check_file(reference, root))
    return errors


def main(argv: list[str] | None = None) -> int:
    del argv
    root = repo_root()
    errors = check(root)
    if errors:
        for error in errors:
            print(f"skill-ref: {error}", file=sys.stderr)
        return 1
    skill_count = len(list((root / SKILL_DIRNAME / REFERENCES_DIRNAME).glob("*.md")))
    print(f"skill refs ok: SKILL.md + {skill_count} references")
    return 0


if __name__ == "__main__":
    sys.exit(main())
