#!/usr/bin/env python3
"""Write the grandfathered-work ratchet for `QUALITY_NEW_CHANGE_WITH_GATE_DEBT`.

The precondition in `scripts/workspace_check.py` fails a repository that
carries unresolved gate debt and opens a *new* OpenSpec change. On the day it
lands, 22 repositories already carry open changes, and failing those fails the
legacy fleet for exactly the state the rule was written to stop growing. So the
work that predates the rule is recorded once, here, from disk:

    python3 scripts/gate_debt_baseline.py            # write gate-debt-baseline.json
    python3 scripts/gate_debt_baseline.py --check    # report drift, write nothing

Generated, never hand-written: the pairs come from `gate-debt.json` and each
repository's own `openspec/changes` directories, read through the same two
helpers the rule uses (`load_gate_debt_record` and `unarchived_changes`), so
the ratchet and the rule cannot disagree about what a change is or which
repositories are debted.

The debt-closing change is never written: it is exempt in the rule itself, and
an entry for it would be dead weight on every future run.

Nothing here is automatic. The file is committed and shrinks by hand as
repositories close changes and discharge debt; a stale entry is dead weight,
not a live exemption, because a repository that leaves `gate-debt.json` is not
consulted at all. The checker reads these files and never writes them.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import workspace_check  # noqa: E402
from workspace_check import (  # noqa: E402
    GATE_DEBT_BASELINE_FILE,
    GATE_DEBT_CLOSING_CHANGES,
    GateDebtRecordError,
    load_gate_debt_record,
    unarchived_changes,
)

class BaselineRootError(ValueError):
    """The resolved workspace root does not hold the debted repositories."""


def collect(debt: dict[str, tuple[str, ...]], workspace_root: Path) -> list[dict[str, str]]:
    """One entry per `(repository, change)` that is open and not debt-closing.

    Refuses a root that holds none of the debted repositories. A wrong root
    would otherwise write an empty ratchet, and an empty ratchet grandfathers
    nothing, so the whole legacy fleet would go red on the next run — the exact
    failure this file exists to prevent, produced by a typo in a path.
    """
    present = [repo for repo in sorted(debt) if (workspace_root / repo).is_dir()]
    if debt and not present:
        raise BaselineRootError(
            f"none of the {len(debt)} debted repositories exist under {workspace_root}; "
            f"pass --root to the workspace root (the one holding them)"
        )
    entries: list[dict[str, str]] = []
    for repo in present:
        changes = workspace_root / repo
        for change in unarchived_changes(changes):
            if change in GATE_DEBT_CLOSING_CHANGES:
                continue
            entries.append({"project": repo, "change": change})
    return entries


def render(entries: list[dict[str, str]]) -> str:
    # A `generator` field, not a comment banner: JSON has no comments, and the
    # loader reads this file with `json.loads`. The sibling record names its
    # generator the same way.
    document = {
        "schema_version": 1,
        "generator": "scripts/gate_debt_baseline.py",
        "recorded_at": "generated",
        "note": (
            "The OpenSpec changes that were already open on the repositories in "
            "gate-debt.json when QUALITY_NEW_CHANGE_WITH_GATE_DEBT landed. A pair listed "
            "here is pre-existing legacy work and is not new work, so the precondition "
            "stays silent for it; every change opened after this snapshot is reported. "
            "The list only shrinks: as a change is archived its entry becomes dead weight, "
            "and as a repository discharges its debt it leaves gate-debt.json and is not "
            "consulted at all."
        ),
        "findings": entries,
    }
    return json.dumps(document, indent=2) + "\n"


def resolve_workspace_root(governance_root: Path, override: str | None) -> Path:
    """The workspace root, resolved the way `workspace_check.run` resolves it.

    `--root`, then `WORKSPACE_ROOT`, then the registry's own `workspace_root`,
    then the current directory. `projects.json` leaves the field null in this
    workspace, so the registry answer alone would not be enough.
    """
    if override:
        return Path(override).expanduser().resolve()
    configured = os.environ.get("WORKSPACE_ROOT")
    if not configured:
        registry = json.loads((governance_root / "projects.json").read_text(encoding="utf-8"))
        configured = registry.get("workspace_root")
    if not configured:
        return Path.cwd().resolve()
    return Path(str(configured)).expanduser().resolve()


def main(argv: list[str] | None = None) -> int:
    governance_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", help="workspace root; defaults to the registry's workspace_root")
    parser.add_argument("--governance-root", default=str(governance_root))
    parser.add_argument("--out", help=f"where to write the ratchet (default: <governance-root>/{GATE_DEBT_BASELINE_FILE})")
    parser.add_argument("--check", action="store_true", help="report drift and write nothing")
    args = parser.parse_args(argv)

    root = Path(args.governance_root)
    try:
        debt = load_gate_debt_record(root)
    except GateDebtRecordError as exc:
        print(f"ERROR {exc}", file=sys.stderr)
        return 2
    workspace_root = resolve_workspace_root(root, args.root)
    out = Path(args.out) if args.out else root / GATE_DEBT_BASELINE_FILE

    try:
        entries = collect(debt, workspace_root)
    except BaselineRootError as exc:
        print(f"ERROR {exc}", file=sys.stderr)
        return 2
    text = render(entries)
    if args.check:
        current = out.read_text(encoding="utf-8") if out.is_file() else ""
        if current == text:
            print(f"{out} is up to date: {len(entries)} grandfathered pair(s)")
            return 0
        print(f"{out} is stale: it does not match the open changes of {len(debt)} debted repository(s)", file=sys.stderr)
        return 1
    out.write_text(text, encoding="utf-8")
    print(f"wrote {out}: {len(entries)} grandfathered pair(s) across {len(debt)} debted repository(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
