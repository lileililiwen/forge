#!/usr/bin/env python3
"""Bind the UI-conformance check into ONE repository's own Gate manifest.

Ruling D18 in this workspace: *a verifier that no gate runs does not gate; wire
every check into the gate that blocks, or it is documentation.* The
`ui_conformance.py` checker exists to report a repository's UI drift against the
shared layer in `dotnet-platform-libs/ui/`. Reporting is not gating. Until a
repository's own `.ai-gate/gate.yaml` selects `ui-conformance` and names a
command for it, the checker is a document.

This tool closes that gap for **one** repository, named on the command line. It
is deliberately not a fleet tool: binding 64 repositories is the owner's
decision, taken one at a time, and a tool that walks the workspace would make a
policy decision look like a keystroke. The sibling `bind_gate_commands.py`
already walks the fleet for concerns the gate itself resolves; this one does not
walk, at all. There is no `--root`, no `--repo`, and no discovery.

## What it writes

```yaml
checks:
  ui-conformance: true
commands:
  ui-conformance: 'python3 ../workspace-governance/scripts/ui_conformance.py .'
```

The command is the one the other bound governance commands in this workspace
already use — the `governance-quality` entry written by
`bind_gate_commands.py` has the same `../workspace-governance/scripts/… .` shape.
It is repository-relative, it resolves from the repository root as working
directory, and it depends on no environment variable. That last property is a
rule here, not a style preference: a CI runner that lacks the variable fails
exactly as a hard-coded path fails, and the shared feed already refuses
env-var-assembled paths for the same reason.

That command reaches another checkout, so binding it also declares the
interface edge in the repository's own `.project.json`:

```json
"interfaces": [
  {
    "id": "ui-conformance",
    "consumes": true,
    "provider": "workspace-governance",
    "mechanism": "sibling",
    "entry": "python3 ../workspace-governance/scripts/ui_conformance.py .",
    "digest": "sha256:…",
    "required": true,
    "evidence_ref": ".ai-gate/gate.yaml"
  }
]
```

An undeclared sibling edge is what `workspace_check.py` charges once the
recorded ramp date passes, so the declaration is written in the same operation
as the binding rather than left for a later run: a repository with no
`.project.json` to declare the edge in is refused by name, and a command this
tool cannot describe as an edge is refused rather than declared falsely. The
entry is never overwritten: an id already declared in the repository's own
file is left exactly as the repository wrote it.

## What it refuses to do

* **Never invents a gate.** A repository whose `.ai-gate/gate.yaml` is missing,
  or is not the governance `rule_pack` shape, is *refused* with the reason
  named. Writing a key into a document whose schema this tool does not
  recognise is how a manifest silently stops parsing.
* **Never clobbers operator intent.** An existing `checks.ui-conformance` that
  is not `true`, or an existing `commands.ui-conformance` that is not the
  command below, is reported as a conflict and left untouched.
* **Never writes without `--apply`.** The default, and `--check`, are dry runs.
* **Never touches another repository.** There is no code path that resolves a
  second path: `bind_repo` is handed one directory and writes only inside it.

## Idempotence

The second run over an already-bound repository finds both entries present and
correct and writes nothing, so the file is byte-for-byte unchanged. The only
bytes this tool ever introduces are the two entries plus their comment; the rest
of the document, its comments and its ordering included, is preserved.

## Deletion

This host's `rm` is a recoverable-delete shim that prints to stdout and can
fail. Nothing here deletes anything.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from bind_gate_commands import (  # noqa: E402
    SIBLING_REACH,
    declared_interface_ids,
    governance_prefix,
    has_declaration,
    sibling_interface,
    write_interfaces,
)

# The check id this tool binds. Like `declared-verification`, it lives in no
# closed concern vocabulary in the gate, so the Gate routes it to the
# project-runtime text adapter where the exit code is the verdict: 0 is PASS,
# non-zero is FAIL.
CHECK_ID = "ui-conformance"

GATE_YAML = ".ai-gate/gate.yaml"

PROJECT_JSON = ".project.json"

# The governance manifest shape this tool will write into. A manifest without
# this is a different document that happens to be called `gate.yaml`.
RULE_PACK_PREFIX = "workspace-governance@"

# Repository-relative, no environment variable, no absolute path, no `..` that
# escapes. See the module docstring.
#
# The path to this module is derived per repository, because this module is
# the Forge-owned governance module at `forge/modules/workspace-governance`:
# a workspace-root repository reaches it through
# `../forge/modules/workspace-governance`, and a repository inside Forge's own
# `modules/` tree reaches it through `../workspace-governance`.
DEFAULT_COMMAND_TEMPLATE = "python3 {governance}/scripts/ui_conformance.py ."


def default_command(repo: Path) -> str:
    """The repository-relative UI-conformance command for `repo`."""
    return DEFAULT_COMMAND_TEMPLATE.format(governance=governance_prefix(repo))

GENERATED_COMMENT = (
    f"  # Generated by workspace-governance/scripts/bind_ui_gate.py. A verifier that\n"
    "  # no gate runs does not gate (ruling D18). Re-run the generator to update it;\n"
    "  # do not hand-edit.\n"
)

# Outcomes, so a caller can tell "nothing to do" from "refused" from "conflicted".
BOUND = "BOUND"
ALREADY_BOUND = "ALREADY_BOUND"
CONFLICT = "CONFLICT"
REFUSED = "REFUSED"
NO_MANIFEST = "NO_MANIFEST"


@dataclass
class Outcome:
    """What happened to one repository."""

    repo: str
    status: str
    detail: str = ""
    wrote: bool = False
    added: tuple[str, ...] = ()


# --- minimal, comment-preserving YAML editing ------------------------


def yaml_scalar(value: str) -> str:
    """Render one command as a single-quoted YAML scalar.

    Single quotes are right for a shell command: only `'` needs escaping
    (doubled) and backslashes stay literal. A double-quoted scalar would
    re-interpret `\\s` and `$`; a plain scalar would break on the `:` and `#`
    that commands contain. The command is written back byte-for-byte in
    meaning, so this must not re-interpret it.
    """
    return "'" + value.replace("'", "''") + "'"


def top_level_block_bounds(lines: list[str], key: str) -> tuple[int, int] | None:
    """Return `(header_index, end_index)` for a top-level `key:` block.

    `end_index` is exclusive and stops at the next non-blank line at column 0,
    which is the next top-level key. Comments and blank lines *inside* the block
    belong to it; a column-0 comment directly above the next key is included,
    which is harmless because only the block's own entries are read.
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


def is_rule_pack_shape(lines: list[str]) -> tuple[bool, str]:
    """Is this a governance `rule_pack` manifest?

    Returns `(ok, reason)`. The reason names the missing key, so a refusal
    tells the reader what to look at rather than only that something is wrong.
    """
    for required in ("version:", "runtime:", "rule_pack:"):
        if not any(line.startswith(required) for line in lines):
            return False, f"no top-level `{required.rstrip(':')}:` key"
    for line in lines:
        if line.startswith("rule_pack:"):
            value = line.split(":", 1)[1].strip()
            if not value.startswith(RULE_PACK_PREFIX):
                return False, (
                    f"`rule_pack:` is {value!r}, not a {RULE_PACK_PREFIX}<version> "
                    "governance rule pack"
                )
            return True, ""
    return False, "no `rule_pack:` value"


def render_manifest(text: str, command: str) -> tuple[str, str, tuple[str, ...]]:
    """Return `(new_text, status, added_keys)` for binding `command`.

    `status` is `BOUND` when bytes are inserted, `ALREADY_BOUND` when the
    binding is already exactly right, `CONFLICT` when an existing entry
    disagrees, and `REFUSED` when the document is not a shape this tool writes
    into. A conflict is reported rather than resolved because an existing
    binding is a decision somebody made by hand.
    """
    lines = text.splitlines(keepends=True)

    ok, reason = is_rule_pack_shape(lines)
    if not ok:
        return text, REFUSED, ()

    existing_selection = None
    existing_command = None

    checks = top_level_block_bounds(lines, "checks")
    if checks is not None:
        existing_selection = entry_value(lines, checks[0], checks[1], CHECK_ID)
    commands = top_level_block_bounds(lines, "commands")
    if commands is not None:
        existing_command = entry_value(lines, commands[0], commands[1], CHECK_ID)

    # A selection that is not `true` is the operator having said no. Do not
    # flip it: a generated tool overriding a recorded decision is the behaviour
    # this tool exists to prevent everywhere else.
    if existing_selection is not None and existing_selection != "true":
        return text, CONFLICT, ()

    if existing_command is not None and existing_command != yaml_scalar(command):
        return text, CONFLICT, ()

    if existing_selection == "true" and existing_command == yaml_scalar(command):
        # Already bound. Return before any insertion, which is what makes the
        # second run a byte-for-byte no-op rather than a second copy of the
        # comment.
        return text, ALREADY_BOUND, ()

    added: list[str] = []
    lines = _insert_selection(lines, added)
    lines = _insert_command(lines, command, added)
    return "".join(lines), BOUND, tuple(added)


def _insert_selection(lines: list[str], added: list[str]) -> list[str]:
    """Ensure `checks: ui-conformance: true` is present.

    A command bound with no selection is refused by the gate's manifest reader
    (`commands.<id> references a check that is not selected`), and synthesising
    a whole `checks:` section would be inventing the repository's policy. So a
    manifest with no `checks:` block is refused by the caller.
    """
    block = top_level_block_bounds(lines, "checks")
    if block is None:  # pragma: no cover - guarded by the caller
        return lines
    _start, end = block
    lines.insert(end, f"  {CHECK_ID}: true\n")
    added.append(f"checks.{CHECK_ID}")
    return lines


def _insert_command(lines: list[str], command: str, added: list[str]) -> list[str]:
    """Ensure `commands: ui-conformance: '<command>'` is present."""
    block = top_level_block_bounds(lines, "commands")
    entry = f"  {CHECK_ID}: {yaml_scalar(command)}\n"
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
        added.append(f"commands.{CHECK_ID}")
        return out
    start, end = block
    lines.insert(end, GENERATED_COMMENT)
    lines.insert(end + 1, entry)
    added.append(f"commands.{CHECK_ID}")
    return lines


# --- the per-repository driver ---------------------------------------


def bind_repo(
    repo_path: Path,
    apply: bool = False,
    command: str | None = None,
) -> Outcome:
    """Bind one repository's manifest. Never raises. Touches only `repo_path`."""
    name = repo_path.name

    if command is None:
        command = default_command(repo_path)

    manifest = repo_path / GATE_YAML
    if not manifest.is_file():
        return Outcome(
            name, NO_MANIFEST, f"no {GATE_YAML} to bind into; nothing was written"
        )

    try:
        before = manifest.read_text(encoding="utf-8")
    except OSError as exc:
        return Outcome(name, REFUSED, f"cannot read {GATE_YAML}: {exc}")

    # A manifest with no `checks:` block cannot be given one by this tool, so
    # refuse it here with the reason rather than after a partial edit.
    if top_level_block_bounds(before.splitlines(keepends=True), "checks") is None:
        return Outcome(
            name,
            REFUSED,
            f"{GATE_YAML} selects no `checks:` block, so a bound command could "
            "never run; nothing was written",
        )

    try:
        after, status, added = render_manifest(before, command)
    except Exception as exc:  # noqa: BLE001 - a bad manifest must not stop a run
        return Outcome(name, REFUSED, f"{GATE_YAML} could not be updated: {exc}")

    # The command reaches another checkout, so binding it is declaring an
    # interface edge: `mechanism: sibling`, the provider it names, and a
    # digest of the invocation. The declaration goes into the repository's
    # own `.project.json` in the same operation as the manifest write, on the
    # same rule as the `quality` block `bind_gate_commands.py` writes: an
    # undeclared sibling edge is the state this exists to remove, so a
    # repository that cannot carry the declaration is refused by name rather
    # than left with one.
    reaches_sibling = bool(SIBLING_REACH.search(command))
    described = sibling_interface(command) if reaches_sibling else None
    if reaches_sibling and described is None:
        return Outcome(
            name,
            REFUSED,
            f"`{command}` reaches another checkout in a shape this tool cannot "
            "describe as a declared interface (a provider and the entry it "
            "invokes, with a digest); nothing was written",
        )
    declaration_key = ""
    if described is not None:
        provider, entry = described
        if not has_declaration(repo_path):
            return Outcome(
                name,
                REFUSED,
                f"`{command}` reaches {provider} and a sibling edge is lawful only "
                f"when it is declared, but this repository has no {PROJECT_JSON} to "
                "declare it in; the declaration is the registry's document and this "
                "tool never creates one, so nothing was written",
            )
        if entry["id"] not in declared_interface_ids(repo_path):
            declaration_key = f"interfaces.{entry['id']}"

    if status == CONFLICT:
        return Outcome(
            name,
            CONFLICT,
            f"an existing `{CHECK_ID}` entry disagrees with this tool's; left untouched",
        )
    if status == REFUSED:
        return Outcome(
            name,
            REFUSED,
            f"{GATE_YAML} is not a governance rule_pack manifest; nothing was written",
        )

    # The declaration lands before the manifest, and before the early return
    # that an already-bound manifest takes: the edge is undeclared until it
    # lands, and leaving it for a re-run is how it stays undeclared.
    declaration_written = False
    if declaration_key and apply:
        try:
            write_interfaces(repo_path, [described[1]])
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            return Outcome(
                name,
                REFUSED,
                f"cannot declare the sibling edge in {PROJECT_JSON}: {exc}",
            )
        declaration_written = True
    declared_note = ""
    if declaration_key:
        verb = "declared" if declaration_written else "would declare"
        declared_note = f"; {verb} {declaration_key} in {PROJECT_JSON}"

    if status == ALREADY_BOUND or after == before:
        return Outcome(
            name,
            ALREADY_BOUND,
            f"already bound to `{command}`; no change{declared_note}",
            wrote=declaration_written,
            added=(declaration_key,) if declaration_key else (),
        )
    if not apply:
        return Outcome(
            name,
            BOUND,
            f"would bind `{command}`{declared_note}",
            added=(*added, declaration_key) if declaration_key else added,
        )

    try:
        manifest.write_text(after, encoding="utf-8")
    except OSError as exc:
        return Outcome(name, REFUSED, f"cannot write {GATE_YAML}: {exc}")
    return Outcome(
        name,
        BOUND,
        f"bound `{command}`{declared_note}",
        wrote=True,
        added=(*added, declaration_key) if declaration_key else added,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__.splitlines()[0],
    )
    parser.add_argument(
        "repo",
        type=Path,
        help=(
            "the one repository to bind. This tool has no fleet mode: there is no "
            "--root and no discovery, because binding a workspace is the owner's "
            "decision taken one repository at a time"
        ),
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="write the binding; without this the run is a dry run",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="dry run that exits non-zero when the binding is missing or wrong",
    )
    parser.add_argument(
        "--command",
        default=None,
        help=(
            "the command to bind (default: the repository-relative governance "
            "command that reaches the Forge-owned governance module from the "
            "repository being bound). Refused if it is absolute or reads "
            "an environment variable"
        ),
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="emit the outcome as JSON instead of text",
    )
    args = parser.parse_args(argv)

    if args.check and args.apply:
        print(
            "--check cannot be combined with --apply: --check exists precisely so "
            "a run that writes nothing can still fail",
            file=sys.stderr,
        )
        return 2

    repo = args.repo.resolve()
    if not repo.is_dir():
        print(f"not a directory: {repo}", file=sys.stderr)
        return 2

    command = args.command if args.command is not None else default_command(repo)

    problem = command_problem(command)
    if problem:
        print(f"ui-gate-command-invalid: {problem}", file=sys.stderr)
        return 2

    outcome = bind_repo(repo, apply=args.apply, command=command)

    if args.json:
        print(
            json.dumps(
                {
                    "repo": outcome.repo,
                    "status": outcome.status,
                    "detail": outcome.detail,
                    "wrote": outcome.wrote,
                    "added": list(outcome.added),
                    "command": command,
                },
                indent=2,
            )
        )
    else:
        marker = "wrote" if outcome.wrote else outcome.status.lower()
        print(f"{outcome.repo:32} {marker:14} :: {outcome.detail}")
        if outcome.added:
            for key in outcome.added:
                print(f"{'':32} + {key}")
        print()
        print(f"mode:            {'APPLY' if args.apply else 'DRY RUN'}")
        print(f"command:         {command}")
        print(f"repositories:    1")

    if outcome.status == BOUND:
        return 1 if args.check else 0
    if args.check and outcome.status == CONFLICT:
        # Already bound, but not to what this tool would write. A check run must
        # surface that rather than pass on the strength of a wrong command.
        return 1
    if args.check and outcome.added:
        # The command half is bound and the declaration is not: the edge is
        # still undeclared, which is what this tool's check mode exists to
        # surface rather than pass.
        return 1
    if outcome.status in (CONFLICT, REFUSED, NO_MANIFEST):
        return 1
    return 0


def command_problem(command: str) -> str:
    """Why `command` may not be bound, or `""` when it may.

    Two refusals, not three. An absolute path stops resolving when the tree is
    relocated, and a variable a CI runner does not carry fails identically to a
    hard-coded path — so both are refused by name rather than discovered by a
    broken run.

    A `..` segment is *not* refused, because the house form of a governance
    command is sibling-relative from the repository root: the `governance-quality`
    entry `bind_gate_commands.py` writes into this workspace's own manifests is
    `python3 ../workspace-governance/scripts/workspace_check.py ...`, and
    refusing that would refuse the existing, working convention. What matters is
    that the path is written relative to the repository root and names a
    repository, not that it contains no relative segment.
    """
    import os

    if not command.strip():
        return "the command is blank"
    if command.startswith("/") or (len(command) > 1 and command[1] == ":"):
        return "an absolute path stops resolving when the tree is relocated"
    for name in os.environ:
        if f"${name}" in command or f"${{{name}}}" in command:
            return (
                f"the command reads the environment variable {name}, which a CI "
                "runner may not carry"
            )
    # Catch `$SOMETHING_ELSE` too: an unset variable is not in `os.environ` on
    # the machine doing the binding, but it is exactly the case that fails on a
    # runner, so refusing by shape is the point.
    unresolved = re.search(r"\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?", command)
    if unresolved:
        return (
            f"the command reads the environment variable {unresolved.group(1)}, "
            "which a CI runner may not carry"
        )
    return ""


if __name__ == "__main__":
    raise SystemExit(main())
