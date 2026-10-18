#!/usr/bin/env python3
"""Plan and apply the `## Purpose` backfill for the workspace fleet.

Every spec file produced by archiving a change carries a TBD marker:

    ## Purpose
    TBD - created by archiving change <name>. Update Purpose after archive.

The archiver is third-party (`@fission-ai/openspec`), so the defect cannot be
fixed where it is born. This tool is the durable half of the fix; the
`SPEC_PURPOSE_PLACEHOLDER` rule in `scripts/workspace_check.py` is the half that
stops the next one from appearing.

What it does:

- reads every `openspec/specs/**/spec.md` in every sibling repository and finds
  the `## Purpose` + TBD marker pair, in both TBD marker generations the
  archiver has emitted;
- resolves the TBD marker's change name to the change's own archive directory.
  The TBD marker holds the **bare** change name while the archive directory is
  **date-prefixed** (`archive/2026-10-03-work-on-main-baseline-rule`), so the two
  are joined by stripping `YYYY-MM-DD-` from the directory name, not by
  concatenation. A name that resolves to more than one archive directory is
  refused, never guessed;
- prefers the archived change's own `## Why` prose, and falls back to the
  requirement-derived sentence otherwise. Both derived sentences are preserved
  byte-for-byte from `openlearning/scripts/fill_spec_purposes.py` so this tool's
  output for a spec without a recoverable `## Why` is the exact string that
  prior art produced;
- records the source used on every plan row, so a reviewer can see which
  sentences are the author's and which are derived;
- prints the plan by default and writes only with `--apply`.

Usage:
    python3 scripts/spec_purpose_backfill.py                    # dry run: print the plan
    python3 scripts/spec_purpose_backfill.py --json             # dry run, machine-readable
    python3 scripts/spec_purpose_backfill.py --check            # is the backfill applied?
    python3 scripts/spec_purpose_backfill.py --write-plan       # refresh the committed plan
    python3 scripts/spec_purpose_backfill.py --apply            # WRITE the fleet (authorized)

Exit codes: 0 plan computed / backfill complete, 1 refused or inconsistent,
3 backfill not yet applied (`--check` only). Python 3.11 standard library only.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]
CHANGE_ID = "spec-purpose-backfill-gate"
PLAN_RELATIVE = f"migration/{CHANGE_ID}.json"

# The TBD marker carries the bare change name; an archive directory is
# date-prefixed. Seven archive directories in the fleet are NOT date-prefixed
# and 47 TBD markers name a date-prefixed change, so both spellings are
# accepted on both sides rather than assuming one.
DATE_PREFIX_RE = re.compile(r"^\d{4}-\d{2}-\d{2}-")

# Both generations: the current form and the older short form. The marker must be
# the whole *section* body, not merely a line that starts with it, so a real
# sentence appended after the marker makes the section a real purpose and this
# tool must not overwrite it. That is why the change name is bound to `[\w.-]+`:
# a change name has no spaces, so trailing prose can never be absorbed into it,
# which a bare `[^\n]*` would happily do. `(?:[ \t]*\n)*` after the heading
# absorbs the blank line 7 files in the fleet carry, which the prior art's
# pattern silently missed.
#
# Measured: the section-strict form matches the same 1,234 files fleet-wide as the
# looser line form and no others, so this is free on the real corpus.
#
# Detection here and in `scripts/workspace_check.py` MUST stay identical — a
# marker this tool resolves but the rule cannot see is a spec that migrates and
# then still reports ERROR.
PLACEHOLDER_RE = re.compile(
    r"^##[ \t]+Purpose[ \t]*\n(?:[ \t]*\n)*"
    r"(TBD - created by archiving change [\w.-]+?\.(?:[ \t]+Update Purpose after archive\.)?)[ \t]*\n"
    r"(?:[ \t]*\n)*(?=^#{1,6}[ \t]|\Z)",
    re.MULTILINE,
)
PLACEHOLDER_NAME_RE = re.compile(
    r"^TBD - created by archiving change (?P<name>[\w.-]+?)\.(?:[ \t]+Update Purpose after archive\.)?$"
)

# Verbatim from `openlearning/scripts/fill_spec_purposes.py`; both derived
# sentences are unchanged so the fleet write is reproducible against what that
# prior art already committed in its one repository.
REQUIREMENT_RE = re.compile(r"^### Requirement:\s*(.+?)\s*$", re.MULTILINE)
CAPABILITY_RE = re.compile(r"^#\s+(.+?)\s+Specification\s*$", re.MULTILINE)

WHY_SECTION_RE = re.compile(r"^##[ \t]+Why[ \t]*$(.*?)(?=^##[ \t]|\Z)", re.MULTILINE | re.DOTALL)

# A `## Why` shorter than this is treated as absent and the row falls back to the
# requirement-derived sentence. The boundary is a judgment call, so it is a named
# constant, recorded in the committed plan, and asserted by a test rather than
# buried in a branch. At 120 the fleet yields 1,091 usable sections; the 12 it
# excludes are fragments rather than a purpose. A reviewer who disagrees changes
# this one number in this one place.
MIN_WHY_CHARS = 120

SOURCE_AMBIGUOUS = "ambiguous-archive"
SOURCE_WHY = "archived-change-why"
SOURCE_REQUIREMENT = "first-requirement"
SOURCE_CAPABILITY = "capability-name"


class BackfillError(RuntimeError):
    """The plan cannot be computed, or applying it would break an invariant."""


@dataclass
class SpecPlan:
    """One spec file whose TBD marker can be resolved mechanically.

    `purpose` is `None` only for a row an owner must decide: a change name that
    resolves to more than one archive directory. Such a row is planned and
    visible, never guessed, and the apply refuses to write while any exists.
    """

    project: str
    spec: str
    change: str
    source: str
    purpose: str | None
    evidence: str | None = None
    marker: str = ""
    requires_owner_decision: bool = False

    def as_dict(self) -> dict[str, Any]:
        record: dict[str, Any] = {
            "project": self.project,
            "spec": self.spec,
            "change": self.change,
            "source": self.source,
            "purpose": self.purpose,
        }
        if self.evidence:
            record["evidence"] = self.evidence
        if self.marker:
            record["marker"] = self.marker
        if self.requires_owner_decision:
            record["requires_owner_decision"] = True
        return record


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise BackfillError(f"cannot read {path}: {exc}") from exc


def collapse(text: str) -> str:
    """Whitespace-collapse a Markdown section to one paragraph."""
    return " ".join(text.split())


def derive_purpose(spec_text: str, capability: str) -> str:
    """The requirement-derived fallback. Preserved from the prior art verbatim."""
    match = REQUIREMENT_RE.search(spec_text)
    if match is None:
        return (
            f"The {capability} specification defines the behaviors, contracts, and "
            "scenarios that govern this capability across the platform."
        )
    topic = match.group(1).strip().rstrip(".")
    topic_lower = topic[0].lower() + topic[1:] if topic else topic
    return (
        f"The {capability} specification covers {topic_lower} and the related "
        "scenarios that govern this capability across the platform."
    )


def capability_name(spec_path: Path, spec_text: str) -> str:
    """The capability the spec is about: its title, else its directory name."""
    match = CAPABILITY_RE.search(spec_text)
    return match.group(1).strip() if match else spec_path.parent.name


def index_archive(project_path: Path) -> dict[str, list[Path]]:
    """Index a repository's archive directories by every name they answer to.

    A date-prefixed directory is indexed under both `2026-09-18-foo` and `foo`,
    because the TBD marker may carry either spelling. An unprefixed directory is
    indexed only under its own name.
    """
    archive = project_path / "openspec" / "changes" / "archive"
    index: dict[str, list[Path]] = {}
    if not archive.is_dir():
        return index
    for child in sorted(archive.iterdir()):
        if not child.is_dir() or child.name.startswith("."):
            continue
        keys = {child.name}
        stripped = DATE_PREFIX_RE.sub("", child.name, count=1)
        if stripped != child.name:
            keys.add(stripped)
        for key in keys:
            index.setdefault(key, []).append(child)
    return index


def resolve_change(index: dict[str, list[Path]], name: str) -> tuple[Path | None, list[Path]]:
    """The one archive directory for `name`, or the ambiguous set.

    Returns `(None, [])` when nothing matches, which is a fallback row, and
    `(None, matches)` when the name is ambiguous, which is a refusal.
    """
    keys = [name]
    stripped = DATE_PREFIX_RE.sub("", name, count=1)
    if stripped != name:
        keys.append(stripped)
    found: dict[Path, None] = {}
    for key in keys:
        for child in index.get(key, []):
            found[child] = None
    if len(found) == 1:
        return next(iter(found)), []
    if not found:
        return None, []
    return None, sorted(found)


def read_why(archive_dir: Path) -> str | None:
    """The archived change's own `## Why` prose, or `None` when unusable."""
    proposal = archive_dir / "proposal.md"
    if not proposal.is_file():
        return None
    try:
        text = proposal.read_text(encoding="utf-8")
    except OSError:
        return None
    match = WHY_SECTION_RE.search(text)
    if match is None:
        return None
    why = collapse(match.group(1))
    return why if len(why) >= MIN_WHY_CHARS else None


def plan_spec(project: str, spec_path: Path, workspace_root: Path, index: dict[str, list[Path]],
              *, resolve_ambiguous_as_derived: bool = False) -> SpecPlan | None:
    """Resolve one spec, or return `None` when it holds no TBD marker."""
    try:
        text = spec_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise BackfillError(f"cannot read {spec_path}: {exc}") from exc
    match = PLACEHOLDER_RE.search(text)
    if match is None:
        return None
    line = match.group(1).strip()
    name_match = PLACEHOLDER_NAME_RE.match(line)
    if name_match is None:
        raise BackfillError(
            f"{project}: {spec_path.name} holds an unrecognised Purpose TBD marker: {line!r}"
        )
    change = name_match.group("name")
    archive_dir, ambiguous = resolve_change(index, change)
    relative = spec_path.relative_to(workspace_root).as_posix()
    if ambiguous and not resolve_ambiguous_as_derived:
        # Recorded, not resolved, and never guessed: the output is a published
        # sentence, and two archive directories under one change name are two
        # different changes. The row stays in the plan so the other 1,234 are
        # reviewable, and `--apply` refuses to write while it exists.
        return SpecPlan(
            project=project,
            spec=relative,
            change=change,
            source=SOURCE_AMBIGUOUS,
            purpose=None,
            evidence="|".join(
                f"openspec/changes/archive/{item.name}/proposal.md" for item in ambiguous
            ),
            marker=line,
            requires_owner_decision=True,
        )

    capability = capability_name(spec_path, text)
    if archive_dir is not None:
        why = read_why(archive_dir)
        if why is not None:
            return SpecPlan(
                project=project,
                spec=relative,
                change=change,
                source=SOURCE_WHY,
                purpose=why,
                evidence=f"openspec/changes/archive/{archive_dir.name}/proposal.md#why",
                marker=line,
            )
    source = SOURCE_REQUIREMENT if REQUIREMENT_RE.search(text) else SOURCE_CAPABILITY
    # With the owner opt-in, an ambiguous change name no longer blocks the write,
    # but it must still be visible in the record: the candidates stay in `evidence`
    # so a reader can see which archives were declined, and why the sentence is
    # derived from the spec's own requirements rather than from any `## Why`.
    return SpecPlan(
        project=project,
        spec=relative,
        change=change,
        source=source,
        purpose=derive_purpose(text, capability),
        evidence=(
            "declined-ambiguous-archive: "
            + "|".join(
                f"openspec/changes/archive/{item.name}/proposal.md" for item in (ambiguous or [])
            )
            if resolve_ambiguous_as_derived and ambiguous
            else None
        ),
        marker=line,
    )


def build_plan(workspace_root: Path, governance_root: Path, *, verify_committed: bool = True,
               resolve_ambiguous_as_derived: bool = False) -> dict[str, Any]:
    """Plan every TBD marker in the fleet, or raise `BackfillError`.

    `verify_committed` is the drift refusal, and it is a *plan-time* guard: it
    exists so a write never lands on a fleet nobody inspected. `--check` turns it
    off, because a fully applied fleet is exactly the state in which every
    planned marker is legitimately absent, and reporting that as drift would make
    "is the backfill applied?" the one question the tool cannot answer.
    """
    plans: list[SpecPlan] = []
    scanned = 0
    contributing: set[str] = set()
    for child in sorted(workspace_root.iterdir(), key=lambda item: item.name):
        if child.name.startswith(".") or child.is_symlink() or not child.is_dir():
            continue
        specs_root = child / "openspec" / "specs"
        if not specs_root.is_dir():
            continue
        scanned += 1
        index = index_archive(child)
        for spec in sorted(specs_root.rglob("spec.md")):
            if not spec.is_file() or spec.is_symlink():
                continue
            plan = plan_spec(child.name, spec, workspace_root, index,
                              resolve_ambiguous_as_derived=resolve_ambiguous_as_derived)
            if plan is not None:
                plans.append(plan)
                contributing.add(child.name)
    if not plans:
        # An empty plan is a valid state, not an error: it is what `--check` sees
        # after a successful apply, and what a dry run on an already-migrated
        # fleet sees. Raising here would make "is the backfill applied?" the one
        # question the tool cannot answer.
        pass

    sources: dict[str, int] = {}
    for plan in plans:
        sources[plan.source] = sources.get(plan.source, 0) + 1

    record: dict[str, Any] = {
        "schema_version": 1,
        "change": CHANGE_ID,
        "policy": {
            "min_why_chars": MIN_WHY_CHARS,
            "sources": [SOURCE_WHY, SOURCE_REQUIREMENT, SOURCE_CAPABILITY, SOURCE_AMBIGUOUS],
            "note": (
                "A `## Why` shorter than `min_why_chars` is treated as absent and the row falls back. "
                "The fallback sentences are preserved byte-for-byte from "
                "`openlearning/scripts/fill_spec_purposes.py`."
            ),
            "owner_decision": (
                "A row with `requires_owner_decision` names a change that matches more than one "
                "archive directory. Its `purpose` is null and `--apply` refuses to write while any "
                "such row exists; the tool does not choose between two changes."
            ),
        },
        "status": {
            "applied": False,
            "authority": (
                "scripts/spec_purpose_backfill.py --check reads the live fleet; this block is a "
                "record, not the source of truth."
            ),
            "note": (
                "The fleet write is a cross-repo source edit and is not applied by the change that "
                "ships the tool and the rule. Apply it with "
                "`python3 scripts/spec_purpose_backfill.py --apply` under explicit authorization. "
                "Setting SPEC_PURPOSE_AUDIT_ENABLED = True additionally requires the "
                "`governance-quality` concern's declared code set to include "
                "SPEC_PURPOSE_PLACEHOLDER in driftwatchdog (design.md §6); until then the bound "
                "gate would report PASS while the checker fails."
            ),
        },
        "totals": {
            "projects_scanned": scanned,
            "projects_with_specs": len(contributing),
            "specs": len(plans),
            "resolvable": sum(1 for plan in plans if plan.purpose is not None),
            "requires_owner_decision": sum(1 for plan in plans if plan.requires_owner_decision),
            "by_source": dict(sorted(sources.items())),
        },
        "specs": [plan.as_dict() for plan in plans],
    }

    if verify_committed:
        _assert_matches_committed_plan(record, workspace_root, governance_root)
    return record


def _committed_plan_path(governance_root: Path) -> Path:
    return governance_root / PLAN_RELATIVE


def _assert_matches_committed_plan(plan: dict[str, Any], workspace_root: Path, governance_root: Path) -> None:
    """Refuse to plan when the fleet has drifted from the committed plan.

    The committed plan is the record of what was decided to write. A spec that no
    longer holds the planned TBD marker, a TBD marker naming a different
    change, a purpose that no longer derives to the planned sentence, or a
    TBD marker the plan never recorded are all drift: they mean the fleet moved
    under the plan, and writing on top of it would resolve a file nobody decided
    to resolve.
    """
    path = _committed_plan_path(governance_root)
    if not path.is_file():
        return
    previous = load_json(path)
    if not isinstance(previous, dict) or not isinstance(previous.get("specs"), list):
        return
    planned = {str(record.get("spec")): record for record in previous["specs"]}
    derived = {str(record.get("spec")): record for record in plan["specs"]}
    issues: list[str] = []
    for spec, record in sorted(planned.items()):
        if spec not in derived:
            issues.append(f"{spec}: planned TBD marker is no longer present")
            continue
        current = derived[spec]
        if current["change"] != record.get("change"):
            issues.append(
                f"{spec}: TBD marker names change {current['change']!r}, plan records "
                f"{record.get('change')!r}"
            )
        elif current["source"] != record.get("source"):
            issues.append(
                f"{spec}: source is now {current['source']!r}, plan records {record.get('source')!r}"
            )
        elif current["purpose"] != record.get("purpose"):
            issues.append(f"{spec}: derived Purpose differs from the committed plan")
    for spec in sorted(set(derived) - set(planned)):
        issues.append(f"{spec}: a new TBD marker is not covered by the committed plan")
    if issues:
        preview = "; ".join(issues[:5])
        more = f" (+{len(issues) - 5} more)" if len(issues) > 5 else ""
        raise BackfillError(
            f"the fleet has drifted from {PLAN_RELATIVE}: {len(issues)} row(s) differ: {preview}{more}"
        )


def outstanding_rows(plan: dict[str, Any], workspace_root: Path) -> list[str]:
    """Rows the backfill has not reached, read from the live fleet."""
    issues: list[str] = []
    for record in plan["specs"]:
        path = workspace_root / str(record["spec"])
        if not path.is_file():
            issues.append(f"{record['spec']}: spec file missing")
            continue
        text = path.read_text(encoding="utf-8")
        match = PLACEHOLDER_RE.search(text)
        if match is None:
            continue
        if match.group(1).strip() != str(record.get('marker', '')):
            issues.append(f"{record['spec']}: still holds a different TBD marker")
        else:
            issues.append(f"{record['spec']}: TBD marker still present")
    return issues


def apply_plan(plan: dict[str, Any], workspace_root: Path) -> list[str]:
    """Write the backfill. Reached only from `--apply`, after a full validation.

    All-or-nothing by design: a row an owner must decide has a null `purpose`, and
    the apply refuses before the first write rather than migrating 1,234 of 1,236
    and leaving a half-applied fleet that the closed fence cannot see.
    """
    undecided = [record for record in plan["specs"] if record.get("requires_owner_decision")]
    if undecided:
        names = "; ".join(
            f"{record['spec']} (change {record['change']!r} matches {str(record.get('evidence')).count('|') + 1} "
            f"archive directories)"
            for record in undecided
        )
        raise BackfillError(
            f"{len(undecided)} row(s) need an owner decision before any write: {names}. "
            f"Decide the Purpose by hand, or remove the duplicate archive directory, then re-run"
        )
    written: list[str] = []
    for record in plan["specs"]:
        path = workspace_root / str(record["spec"])
        if not path.is_file():
            raise BackfillError(f"{record['spec']}: spec file missing at apply time")
        text = path.read_text(encoding="utf-8")
        match = PLACEHOLDER_RE.search(text)
        if match is None:
            continue
        if match.group(1).strip() != str(record.get('marker', '')):
            raise BackfillError(
                f"{record['spec']}: TBD marker changed since the plan was computed; refusing to write"
            )
        # A function replacement, not a string: the Purpose is archived prose
        # and may legitimately contain a backslash, which `re.sub` would
        # otherwise read as an escape (`\s` is "bad escape" and raises). The
        # text must be inserted literally.
        replacement = f"## Purpose\n\n{record['purpose']}\n"
        updated = PLACEHOLDER_RE.sub(
            lambda _match, value=replacement: value, text, count=1,
        )
        path.write_text(updated, encoding="utf-8")
        written.append(f"{record['spec']}: Purpose from {record['source']}")
    return written


def print_plan(plan: dict[str, Any], stream: Any) -> None:
    totals = plan["totals"]
    print(
        f"spec Purpose backfill: {totals['specs']} spec file(s) across "
        f"{totals['projects_with_specs']} repositories "
        f"({totals['projects_scanned']} scanned)",
        file=stream,
    )
    print("by source: " + ", ".join(f"{key} {value}" for key, value in totals["by_source"].items()), file=stream)
    print(f"min_why_chars: {plan['policy']['min_why_chars']}", file=stream)
    if totals.get("requires_owner_decision"):
        print(
            f"{totals['requires_owner_decision']} row(s) need an owner decision: "
            f"{totals['resolvable']} are resolvable now, and --apply refuses to write while any "
            f"undecided row exists",
            file=stream,
        )
    print("", file=stream)
    for record in plan["specs"]:
        evidence = record.get("evidence") or "-"
        print(
            f"  {record['spec']:64s} change={record['change']:36s} "
            f"source={record['source']:20s} {evidence}",
            file=stream,
        )
        if record.get("requires_owner_decision"):
            print("      Purpose: OWNER DECISION REQUIRED (no sentence derived)", file=stream)
        else:
            print(f"      Purpose: {record['purpose']}", file=stream)


def write_plan(plan: dict[str, Any], governance_root: Path) -> Path:
    target = _committed_plan_path(governance_root)
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.is_file():
        # The applied-state is a record, not the authority: `--check` reads the
        # live fleet. Refreshing the plan must not silently clear a record.
        previous = load_json(target)
        if isinstance(previous, dict) and isinstance(previous.get("status"), dict):
            plan["status"] = {**plan["status"], **previous["status"]}
    target.write_text(json.dumps(plan, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return target


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", default="..", help="workspace root holding the sibling repositories")
    parser.add_argument("--governance-root", default=None,
                        help="governance root holding the committed plan; defaults to this "
                             "repository. Point it at a copy to rehearse --apply.")
    parser.add_argument("--json", action="store_true", dest="as_json", help="print the plan as JSON (dry run)")
    parser.add_argument("--check", action="store_true", help="report whether the backfill has been applied")
    parser.add_argument("--apply", action="store_true", help="WRITE the Purpose to the fleet (authorized step)")
    parser.add_argument("--write-plan", action="store_true", help="refresh the committed plan file from a dry run")
    parser.add_argument("--resolve-ambiguous-as-derived", action="store_true",
                        dest="resolve_ambiguous",
                        help="owner decision: when a change name matches more than one archive "
                             "directory, derive the Purpose from the spec's own requirements and "
                             "record the declined candidates, instead of refusing the write")
    args = parser.parse_args(argv)

    workspace_root = Path(args.root).expanduser().resolve()
    governance_root = Path(args.governance_root).expanduser().resolve() if args.governance_root else GOVERNANCE_ROOT
    # `--check` reports state and must never be refused for drift: a fully applied
    # fleet is the state in which every planned marker is legitimately gone.
    # `--write-plan` alone with `--check` is the one combination that still wants
    # the refusal, because it refreshes the record the refusal guards.
    verify_committed = not args.check or args.write_plan
    try:
        plan = build_plan(workspace_root, governance_root, verify_committed=verify_committed,
                          resolve_ambiguous_as_derived=args.resolve_ambiguous)
    except BackfillError as exc:
        print(f"refused: {exc}", file=sys.stderr)
        return 1

    if args.write_plan:
        target = write_plan(plan, governance_root)
        print(f"wrote {target.relative_to(governance_root)}", file=sys.stderr)

    if args.apply:
        try:
            written = apply_plan(plan, workspace_root)
        except BackfillError as exc:
            print(f"refused: {exc}", file=sys.stderr)
            return 1
        print(f"applied {len(written)} writes", file=sys.stderr)
        for line in written:
            print(f"  {line}", file=sys.stderr)
        print("", file=sys.stderr)
        print(
            "now set SPEC_PURPOSE_AUDIT_ENABLED = True in scripts/workspace_check.py in the same commit",
            file=sys.stderr,
        )
        print(
            "only after SPEC_PURPOSE_PLACEHOLDER is in the `governance-quality` concern's declared "
            "code set in driftwatchdog (design.md §6); flipping the flag before that lands makes "
            "this repository's CI red while the bound gate reports PASS",
            file=sys.stderr,
        )
        return 0

    if args.check:
        issues = outstanding_rows(plan, workspace_root)
        if issues:
            print(f"backfill not applied: {len(issues)} row(s) outstanding", file=sys.stderr)
            for issue in issues[:20]:
                print(f"  {issue}", file=sys.stderr)
            if len(issues) > 20:
                print(f"  ... and {len(issues) - 20} more", file=sys.stderr)
            print("SPEC_PURPOSE_AUDIT_ENABLED must stay False until every row is applied", file=sys.stderr)
            return 3
        print("backfill applied: no spec holds a Purpose TBD marker")
        print("SPEC_PURPOSE_AUDIT_ENABLED may now be set to True, subject to the driftwatchdog dependency")
        return 0

    if args.as_json:
        print(json.dumps(plan, indent=2, ensure_ascii=False))
    else:
        print_plan(plan, sys.stdout)
        print("", file=sys.stderr)
        print("dry run: nothing was written. Re-run with --apply under explicit authorization.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
