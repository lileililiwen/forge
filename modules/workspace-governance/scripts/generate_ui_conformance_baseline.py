#!/usr/bin/env python3
"""Write the grandfathered-surface ratchet for `ui-convention-check`.

The checker in `scripts/ui_conformance.py` fails a repository that ships UI and
declares neither a scan scope nor adoption of the shared `ui` layer. On the day
it lands, 59 repositories ship UI (4280 files), 22 of them hold 113 auth-named
UI files, and the adoption count is zero. Failing those fails the fleet for
exactly the state the rule was written to catch, so the work that predates the
rule is recorded once, here, from disk:

    python3 scripts/generate_ui_conformance_baseline.py            # write the baseline
    python3 scripts/generate_ui_conformance_baseline.py --check    # report drift, write nothing

Generated, never hand-written. The entries come from the same `evaluate()` the
checker uses, so the ratchet and the rule cannot disagree about what a finding
is or which repositories have one.

Nothing here is automatic. The file is committed and shrinks by hand as
repositories adopt the shared layer; a stale entry is dead weight, not a live
exemption, because a repository whose findings have cleared is reported by
`--check` as an entry that no longer matches. The checker reads this file and
never writes it.

`record_role()` exists because two repositories in the fleet are not consumers
and their entries must not read as if they were: `alltools-platform` is a
tooling platform (2067 UI files, the only repository over the 400-file
ceiling) and `dotnet-platform-libs` is the provider that ships the layer
itself.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import ui_conformance  # noqa: E402
from ui_conformance import (  # noqa: E402
    BASELINE_FILE,
    ConfigError,
    evaluate,
    load_baseline,
    parse_ui_policy,
    _fleet_roots,
    _governance_root,
    _load_declaration,
    census,
)

# Repositories that are not consumers. Recorded explicitly so the entry reads
# as what it is rather than as one more repo that has not adopted the layer.
ROLES = {
    "alltools-platform": "tooling",
    "dotnet-platform-libs": "provider",
}


def record_role(project: str) -> str:
    return ROLES.get(project, "consumer")


class BaselineRootError(ValueError):
    """The resolved workspace root does not hold the measured repositories."""


def collect(
    workspace_root: Path,
    *,
    token_source: Path,
    module_path: Path | None,
    node_timeout: int,
) -> list[dict[str, object]]:
    """One entry per registered repository that ships UI.

    Refuses to write an empty ratchet when nothing resolved. An empty ratchet
    grandfathers nothing, so the whole fleet would go red on the next run —
    the exact failure this file exists to prevent, produced by a typo in a
    path.
    """
    _, projects = _fleet_roots(_governance_root())
    entries: list[dict[str, object]] = []
    scanned = 0
    for entry in projects:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            continue
        project = entry["path"]
        project_dir = (workspace_root / project).resolve()
        if not project_dir.is_dir():
            continue
        scanned += 1
        try:
            declaration = _load_declaration(project_dir / ".project.json")
            policy = parse_ui_policy(declaration)
        except ConfigError as exc:
            raise BaselineRootError(f"{project}: {exc}") from exc
        ui_files = census(project_dir)
        if not ui_files:
            continue
        report = evaluate(
            project_dir, policy,
            token_source=token_source, module_path=module_path, node_timeout=node_timeout,
        )
        counts: dict[str, int] = {}
        for finding in report.findings:
            counts[finding.rule_id] = counts.get(finding.rule_id, 0) + 1
        entries.append({
            "project": project,
            "role": record_role(project),
            "ui_files": len(ui_files),
            "scope_ceiling": policy.max_files,
            "scope_oversize": len(ui_files) > policy.max_files,
            "scope_declared": policy.scope_declared,
            "codes": sorted(counts),
            "counts": dict(sorted(counts.items())),
        })
    if scanned == 0:
        raise BaselineRootError(
            f"no registered repository directory exists under {workspace_root}; "
            "pass --root to the workspace root (the one holding them)"
        )
    if not entries:
        raise BaselineRootError(
            f"no repository under {workspace_root} ships a UI surface; "
            "an empty baseline would grandfather nothing and turn the fleet red"
        )
    return entries


def render(entries: list[dict[str, object]]) -> str:
    # A `generator` field, not a comment banner: JSON has no comments, and the
    # loader reads this file with `json.loads`. The sibling record names its
    # generator the same way.
    document = {
        "schema_version": 1,
        "generator": "scripts/generate_ui_conformance_baseline.py",
        "recorded_at": "generated",
        "note": (
            "The UI findings that were already true of every registered repository "
            "when ui-convention-check landed. A code listed here for a project is a "
            "measured pre-existing state, so the gate goes live without turning the "
            "whole fleet red on day one; the finding is still reported and counted as "
            "baselined. The list only shrinks: a repository that adopts the shared ui "
            "layer, declares its scan scope, or resolves its branding and contrast "
            "artefacts drops the corresponding code, and a repository that clears every "
            "code leaves this file and is not consulted at all. An entry that no longer "
            "matches the measured state is dead weight and --check reports it. This file "
            "is not a switch: a run without --baseline fails closed on every finding, and "
            "a code that is not listed here fails normally."
        ),
        "entries": entries,
    }
    return json.dumps(document, indent=2) + "\n"


def resolve_workspace_root(override: str | None) -> Path:
    if override:
        return Path(override).expanduser().resolve()
    root, _ = _fleet_roots(_governance_root())
    return root


def main(argv: list[str] | None = None) -> int:
    governance_root = _governance_root()
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--root", help="workspace root; defaults to WORKSPACE_ROOT or the registry's workspace_root")
    parser.add_argument("--governance-root", default=str(governance_root))
    parser.add_argument("--out", help=f"where to write the ratchet (default: <governance-root>/{BASELINE_FILE})")
    parser.add_argument("--tokens", type=Path, default=None, help="token source for the measurement pass")
    parser.add_argument("--token-module", type=Path, default=None, help="token-model.mjs path")
    parser.add_argument("--node-timeout", type=int, default=ui_conformance.DEFAULT_NODE_TIMEOUT)
    parser.add_argument("--check", action="store_true", help="report drift and write nothing")
    args = parser.parse_args(argv)

    root = Path(args.governance_root)
    token_source = args.tokens.resolve() if args.tokens else (root / ui_conformance.DEFAULT_TOKEN_SOURCE).resolve()
    try:
        workspace_root = resolve_workspace_root(args.root)
    except ConfigError as exc:
        print(f"ERROR {exc}", file=sys.stderr)
        return 2
    out = Path(args.out) if args.out else root / BASELINE_FILE

    try:
        entries = collect(
            workspace_root,
            token_source=token_source,
            module_path=args.token_module,
            node_timeout=args.node_timeout,
        )
    except (BaselineRootError, ConfigError) as exc:
        print(f"ERROR {exc}", file=sys.stderr)
        return 2

    text = render(entries)
    oversize = [str(e["project"]) for e in entries if e["scope_oversize"]]
    if args.check:
        current = out.read_text(encoding="utf-8") if out.is_file() else ""
        if current == text:
            print(
                f"{out} is up to date: {len(entries)} grandfathered project(s), "
                f"{len(oversize)} over the scope ceiling"
            )
            return 0
        stale = _stale_projects(out, entries)
        if stale:
            print(f"{out} is stale: these entries no longer match the measured state: "
                  f"{', '.join(stale)}", file=sys.stderr)
        else:
            print(f"{out} is stale: it does not match the measured state of {len(entries)} project(s)",
                  file=sys.stderr)
        return 1
    out.write_text(text, encoding="utf-8")
    print(
        f"wrote {out}: {len(entries)} grandfathered project(s), "
        f"{len(oversize)} over the scope ceiling"
    )
    return 0


def _stale_projects(out: Path, measured: list[dict[str, object]]) -> list[str]:
    """Recorded projects whose measured codes no longer match the file.

    A stale entry is dead weight, not a live exemption. This is the difference
    from a hand-maintained exemption list, and it is the reason the generator
    ships with `--check`.
    """
    if not out.is_file():
        return []
    recorded = load_baseline(out)
    now = {str(entry["project"]): frozenset(entry["codes"]) for entry in measured}  # type: ignore[index]
    stale = [project for project, codes in sorted(recorded.items()) if now.get(project) != codes]
    return stale


if __name__ == "__main__":
    sys.exit(main())