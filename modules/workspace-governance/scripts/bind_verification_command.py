#!/usr/bin/env python3
"""Bind each repository's declared verification command into its own Gate manifest.

Ruling D18 in this workspace: *a verifier that no gate runs does not gate; wire
every check into the gate that blocks, or it is documentation.* Every repository
already records how it should be verified, in its own `.project.json`, under
`verification.command`. At fleet scale that record was being ignored: measured on
2026-10-04, 81 of 81 repositories declared one and 6 bound a command into
`.ai-gate/gate.yaml`, leaving 70 whose gate executed nothing.

This tool closes the gap by writing each repository's **own** declared command
into that repository's **own** manifest. It is deliberately not a template: a
template cannot know a sibling repository's verification command, and
`profile_defaults` in `driftwatchdog` selects concern ids only and never invents
a command, by design. See
`driftwatchdog/openspec/changes/bind-declared-verification-command/design.md` §1.

## What it refuses to do

* **Never invents a command.** A repository with no `.project.json`, an
  unparseable one, or no non-empty string `verification.command` is *refused*,
  by name and per condition. The run continues to the next repository.
* **Never reuses another repository's command.** The value always comes from
  the repository being written.
* **Never clobbers operator intent.** An existing `commands.declared-verification`
  that differs from the declared command, or an existing
  `checks.declared-verification` that is not `true`, is reported as a conflict
  and left untouched.
* **Never writes without `--apply`.** The default is a dry run.

## Idempotence

The second run over an already-bound repository finds the declared command
already present and writes nothing, so the file is byte-for-byte unchanged. The
only bytes this tool ever introduces are the two entries below plus their
comment; the rest of the document, comments and ordering included, is preserved.

## Deletion

This host's `rm` is a recoverable-delete shim that prints to stdout and can fail.
Nothing here deletes anything. Test and temp cleanup uses
`Path.unlink(missing_ok=True)` and `shutil.rmtree(..., ignore_errors=True)`.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

# The concern id this tool binds. It is stable, non-exemptable
# (`NON_EXEMPTABLE_CONCERNS` in driftwatchdog), and lives in no closed concern
# vocabulary, so the Gate routes it to the project-runtime text adapter where
# the exit code is the verdict: 0 is PASS, non-zero is FAIL.
CONCERN_ID = "declared-verification"

PROJECT_JSON = ".project.json"
GATE_YAML = ".ai-gate/gate.yaml"

# One line, so the generated entry is identifiable and a reader can tell
# generated policy from hand-written policy without opening the tool.
GENERATED_COMMENT = (
    f"  # Generated from {PROJECT_JSON} verification.command by "
    "workspace-governance/scripts/bind_verification_command.py.\n"
    "  # A verifier that no gate runs does not gate (ruling D18). Re-run the\n"
    "  # generator to update it; do not hand-edit.\n"
)

# Outcomes, so a caller can tell "nothing to do" from "refused" from "conflicted".
BOUND = "BOUND"
ALREADY_BOUND = "ALREADY_BOUND"
REFUSED = "REFUSED"
CONFLICT = "CONFLICT"
NO_MANIFEST = "NO_MANIFEST"
DIRTY = "DIRTY"


@dataclass
class Outcome:
    """What happened to one repository."""

    repo: str
    status: str
    detail: str = ""
    wrote: bool = False
    before: str = ""
    after: str = ""


@dataclass
class Report:
    """The whole run, in a form a caller can assert on and a human can read."""

    outcomes: list[Outcome] = field(default_factory=list)

    def add(self, outcome: Outcome) -> None:
        self.outcomes.append(outcome)

    def count(self, status: str) -> int:
        return sum(1 for o in self.outcomes if o.status == status)

    def names(self, status: str) -> list[str]:
        return [o.repo for o in self.outcomes if o.status == status]


# --- reading the declaration ---------------------------------------


def read_declared_command(project_path: Path) -> tuple[str | None, str]:
    """Return `(command, refusal_reason)` for one repository.

    The command is the repository's own string, read from its own
    `.project.json`, and is returned verbatim. A refusal reason means no
    command may be written: the caller must not substitute anything, because a
    generated command that the repository never declared would assert a
    verification nobody asked for.
    """
    project_json = project_path / PROJECT_JSON
    if not project_json.is_file():
        return None, f"no {PROJECT_JSON}"
    try:
        raw = project_json.read_text(encoding="utf-8")
    except OSError as exc:
        return None, f"cannot read {PROJECT_JSON}: {exc}"
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        return None, f"{PROJECT_JSON} is not parseable JSON: {exc}"
    if not isinstance(data, dict):
        return None, f"{PROJECT_JSON} top level is not an object"
    verification = data.get("verification")
    if verification is None:
        return None, f"{PROJECT_JSON} declares no `verification`"
    if not isinstance(verification, dict):
        return None, f"{PROJECT_JSON} `verification` is not an object"
    if "command" not in verification:
        return None, f"{PROJECT_JSON} `verification` declares no `command`"
    command = verification["command"]
    if not isinstance(command, str):
        return None, f"{PROJECT_JSON} `verification.command` is not a string"
    if not command.strip():
        return None, f"{PROJECT_JSON} `verification.command` is blank"
    return command.strip(), ""


# --- minimal, comment-preserving YAML editing ----------------------


def yaml_scalar(value: str) -> str:
    """Render one command as a single-quoted YAML scalar.

    Single quotes are the right delimiter for a shell command: only `'` itself
    needs escaping (doubled), and backslashes stay literal. A double-quoted
    scalar would reinterpret `\\s`, `$` and friends; a plain scalar would break
    on the `:` and `#` that commands like `rg -e 'password\\s*'` contain. A
    declared command is written back byte-for-byte in meaning, so this must not
    re-interpret it.
    """
    return "'" + value.replace("'", "''") + "'"


def top_level_block_bounds(lines: list[str], key: str) -> tuple[int, int] | None:
    """Return `(header_index, end_index)` for a top-level `key:` block.

    `end_index` is exclusive and stops at the next non-blank line at column 0,
    which is the next top-level key. Comments and blank lines *inside* the block
    belong to it; a comment line at column 0 directly above the next key is
    included, which is harmless because only the block's own entries are read.
    """
    start = None
    for index, line in enumerate(lines):
        if line.startswith(f"{key}:"):
            start = index
            break
    if start is None:
        return None
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if line.strip() and not line[0].isspace():
            end = index
            break
    return start, end


def entry_value(lines: list[str], start: int, end: int, key: str) -> str | None:
    """Read one `  <key>: <value>` entry from a top-level block, or None."""
    needle = f"  {key}:"
    for index in range(start + 1, end):
        line = lines[index]
        if line.startswith(needle):
            return line[len(needle):].strip()
    return None


def render_manifest(text: str, command: str) -> tuple[str, str]:
    """Return `(new_text, status)` for binding `command` into a manifest.

    `status` is `BOUND` when bytes were inserted, `ALREADY_BOUND` when the
    binding is already exactly right, and `CONFLICT` when an existing entry
    disagrees. The conflict is reported rather than resolved, because an
    existing binding is a decision somebody made by hand.
    """
    lines = text.splitlines(keepends=True)
    existing_command = None
    existing_selection = None

    checks = top_level_block_bounds(lines, "checks")
    if checks is not None:
        existing_selection = entry_value(lines, checks[0], checks[1], CONCERN_ID)
    commands = top_level_block_bounds(lines, "commands")
    if commands is not None:
        existing_command = entry_value(lines, commands[0], commands[1], CONCERN_ID)

    # An existing selection that is not `true` is the operator having said no.
    # Do not flip it: a generated tool overriding a recorded decision is the
    # behaviour this tool exists to prevent everywhere else.
    if existing_selection is not None and existing_selection != "true":
        return text, CONFLICT

    if existing_command is not None:
        if existing_command == yaml_scalar(command):
            # Already bound. If the selection is right too, there is nothing to
            # do — and in particular do not append a second copy of the
            # comment, which is what would break byte-for-byte idempotence.
            if existing_selection == "true":
                return text, ALREADY_BOUND
            return "".join(_insert_selection(lines, command)), BOUND
        return text, CONFLICT

    # A command bound with no selection is refused by the manifest parser
    # (`commands.<id> references a check that is not selected`), so there is
    # nothing to gain from writing one, and synthesising a whole `checks`
    # section would be inventing the repository's policy.
    if checks is None:
        return text, REFUSED

    lines = _insert_selection(lines, command)
    lines = _insert_command(lines, command)
    return "".join(lines), BOUND


def _insert_selection(lines: list[str], command: str) -> list[str]:
    """Ensure `checks: declared-verification: true` is present.

    The caller has already established that a `checks` block exists; this
    function only places the entry at the end of it, so the concern lands
    inside the block rather than after the next top-level key.
    """
    block = top_level_block_bounds(lines, "checks")
    if block is None:  # pragma: no cover - guarded by the caller
        return lines
    _start, end = block
    lines.insert(end, f"  {CONCERN_ID}: true\n")
    return lines


def _insert_command(lines: list[str], command: str) -> list[str]:
    """Ensure `commands: declared-verification: '<command>'` is present."""
    block = top_level_block_bounds(lines, "commands")
    entry = f"  {CONCERN_ID}: {yaml_scalar(command)}\n"
    if block is None:
        # Create the block at the end of the document, separated by a blank
        # line so it reads as a section rather than trailing the last key.
        body = [line for line in lines if line.strip()]
        trailing = len(lines) - len(body)
        out = list(lines[: len(lines) - trailing])
        if out and not out[-1].endswith("\n"):
            out[-1] += "\n"
        if out and out[-1].strip():
            out.append("\n")
        out.append("commands:\n")
        out.append(GENERATED_COMMENT)
        out.append(entry)
        out.extend(lines[len(lines) - trailing:])
        return out
    start, end = block
    lines.insert(end, GENERATED_COMMENT)
    lines.insert(end + 1, entry)
    return lines


# --- the per-repository driver -------------------------------------


def is_dirty(repo_path: Path) -> bool:
    """True when the repository has uncommitted work in the manifest.

    The workspace rule is to stop rather than write into a repository that is
    dirty in a file the change must change. Untracked counts: a `gate.yaml` the
    owner has not committed is still their uncommitted state.
    """
    import subprocess

    try:
        completed = subprocess.run(
            ["git", "status", "--porcelain", "--", f".ai-gate/{'gate.yaml'}"],
            cwd=repo_path,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        # If git cannot answer, do not guess that the tree is clean.
        return True
    return completed.stdout.strip() != ""


def bind_repo(repo_path: Path, apply: bool = False, check_clean: bool = True) -> Outcome:
    """Bind one repository's declared command. Never raises."""
    name = repo_path.name
    command, refusal = read_declared_command(repo_path)
    if command is None:
        return Outcome(name, REFUSED, refusal)

    manifest = repo_path / GATE_YAML
    if not manifest.is_file():
        return Outcome(
            name, NO_MANIFEST, f"no {GATE_YAML} to bind into; nothing was written"
        )

    if check_clean and is_dirty(repo_path):
        return Outcome(
            name,
            DIRTY,
            f"{GATE_YAML} has uncommitted work; stopped rather than write into it",
        )

    try:
        before = manifest.read_text(encoding="utf-8")
    except OSError as exc:
        return Outcome(name, REFUSED, f"cannot read {GATE_YAML}: {exc}")

    try:
        after, status = render_manifest(before, command)
    except Exception as exc:  # noqa: BLE001 - a bad manifest must not stop the fleet
        return Outcome(name, REFUSED, f"{GATE_YAML} could not be updated: {exc}")

    if status == CONFLICT:
        return Outcome(
            name, CONFLICT, "an existing entry disagrees with the declared command; left untouched"
        )
    if status == REFUSED:
        return Outcome(
            name,
            REFUSED,
            f"{GATE_YAML} selects no `checks:` block, so a bound command could never run; "
            "nothing was written",
        )
    if status == ALREADY_BOUND:
        return Outcome(name, ALREADY_BOUND, "already bound to the declared command", before=before, after=after)
    if after == before:
        return Outcome(name, ALREADY_BOUND, "declared command is already present", before=before, after=after)
    if not apply:
        return Outcome(name, BOUND, "would bind the declared command", before=before, after=after)

    try:
        manifest.write_text(after, encoding="utf-8")
    except OSError as exc:
        return Outcome(name, REFUSED, f"cannot write {GATE_YAML}: {exc}")
    return Outcome(name, BOUND, "bound the declared command", wrote=True, before=before, after=after)


def discover_repos(root: Path) -> list[Path]:
    """Sibling git repositories, in stable sorted order."""
    return sorted(
        (child for child in root.iterdir() if (child / ".git").exists() and child.is_dir()),
        key=lambda p: p.name,
    )


def run(
    root: Path,
    apply: bool = False,
    only: list[str] | None = None,
    check_clean: bool = True,
) -> Report:
    """Bind across the workspace. Never raises.

    `check_clean` defaults to True, so a fleet run stops for any repository
    that is dirty in the manifest rather than writing into somebody's
    uncommitted state. Tests pass False to exercise the binding itself.
    """
    report = Report()
    for repo_path in discover_repos(root):
        if only and repo_path.name not in only:
            continue
        report.add(bind_repo(repo_path, apply=apply, check_clean=check_clean))
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Bind each repository's declared verification command into its Gate manifest."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="workspace root holding the sibling repositories",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="write the bindings; without this the run is a dry run",
    )
    parser.add_argument(
        "--repo",
        action="append",
        dest="only",
        help="restrict the run to these repository names (repeatable)",
    )
    args = parser.parse_args(argv)

    root = args.root.resolve()
    if not root.is_dir():
        print(f"workspace root is not a directory: {root}", file=sys.stderr)
        return 2

    report = run(root, apply=args.apply, only=args.only)
    for outcome in report.outcomes:
        marker = "wrote" if outcome.wrote else outcome.status.lower()
        detail = f" :: {outcome.detail}" if outcome.detail else ""
        print(f"{outcome.repo:32} {marker:14}{detail}")

    print()
    print(f"mode:            {'APPLY' if args.apply else 'DRY RUN'}")
    print(f"repositories:    {len(report.outcomes)}")
    for status in (BOUND, ALREADY_BOUND, CONFLICT, REFUSED, NO_MANIFEST, DIRTY):
        print(f"{status + ':':18}{report.count(status)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
