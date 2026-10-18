#!/usr/bin/env python3
"""Plan and apply the identity/taxonomy axis migration for the workspace fleet.

The migration itself is a fleet-wide cross-repo source edit: 81 sibling
`.project.json` declarations plus the registry entries that govern them. It is
therefore **not** applied by the change that ships the schema, the validators,
and this tool. It is applied in a separately authorized step, and this script
is the reviewable artifact for that step.

What it does:

- reads the decided rows from `design.md` §5.2 / §5.3 (the owner's
  per-repository table, including the rulings it carries) instead of keeping a
  second copy of them, and refuses to plan if the fleet has drifted from it;
- resolves every remaining declaration with the deterministic §5.1 order
  (stack-qualified `kind` -> stack-qualified declaration `profile` ->
  stack-qualified registry `profile` -> bounded offline census -> `none`);
- refuses to write anything unless every existing registry `id`/`path` pair
  survives byte-identical (id immutability, `design.md` §5.5);
- prints the plan by default and writes only with `--apply`.

Usage:
    python3 scripts/axis_migration.py                      # dry run: print the plan
    python3 scripts/axis_migration.py --json               # dry run, machine-readable
    python3 scripts/axis_migration.py --check              # is the migration applied?
    python3 scripts/axis_migration.py --write-plan         # refresh the committed plan
    python3 scripts/axis_migration.py --apply              # WRITE the fleet (authorized)

Exit codes: 0 plan computed / migration complete, 1 refused or inconsistent,
3 migration not yet applied (`--check` only). The census is a read-only,
bounded, offline walk (`design.md` §5.1 rule 4): it is a migration aid, its
result is recorded in the plan for a human to confirm, and the audit never
re-derives an axis from a file tree.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

_SCRIPTS = Path(__file__).resolve().parent
if str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))


GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]
CHANGE_ID = "identity-and-taxonomy-axis-split"
DESIGN_RELATIVE = f"openspec/changes/archive/2026-10-12-{CHANGE_ID}/design.md"
PLAN_RELATIVE = f"migration/{CHANGE_ID}.json"
REGISTRY_RELATIVE = "projects.json"

# §5.1 rule 4: the bounded, read-only census. The excluded directories are
# exactly the ones the design lists; nothing outside the repository is read and
# no project command is executed.
CENSUS_EXCLUDE = (
    "node_modules", "dist", "build", "target", "bin", "obj", "vendor",
    "coverage", ".git", "__pycache__", ".venv", ".dart_tool", ".next", "out",
    ".turbo",
)
# A file budget per repository, so the census is bounded in time as well as in
# scope. The counts actually seen are recorded in the plan.
CENSUS_FILE_BUDGET = 20000
CENSUS_EXTENSIONS = {
    ".cs": "dotnet", ".fs": "dotnet", ".vb": "dotnet",
    ".rs": "rust",
    ".ts": "typescript", ".tsx": "typescript", ".js": "typescript",
    ".jsx": "typescript", ".mjs": "typescript", ".cjs": "typescript",
    ".py": "python",
    ".dart": "flutter",
}

# `shared-library` is ruled out of the role set by owner ruling 1: the three
# repositories that wrote it take `role: platform`, because the
# library-versus-service distinction that carries weight already lives in the
# frozen `profile` values and no consumer branches on the role.
ROLE_OVERRIDES = {"shared-library": "platform"}
# A composite whose trailing word is a layout, not a role. `design.md` §9
# defers a `layout` axis, so the word must not become a role; the role census in
# `design.md` §4 pins the value that remains legal.
LAYOUT_ROLES = {"monorepo"}
LAYOUT_ROLE_FALLBACK = "product"

ROW_KEY = re.compile(r"^`([^`]+)`(?:\s*\(`([^`]+)`\))?$")
ROW_CELL = re.compile(r"^`([^`]+)`$")


class MigrationError(RuntimeError):
    """The plan cannot be computed, or applying it would break an invariant."""


@dataclass
class DecidedRow:
    """One decided row of `design.md` §5.2 or §5.3."""

    directory: str
    registry_id: str | None
    kind: str | None
    registry_profile: str | None
    declared_profile: str | None
    census: str | None
    roles: list[str]
    stacks: list[str]
    rule: str
    section: str


@dataclass
class DeclarationPlan:
    directory: str
    project_id: str
    declaration: str
    kind: str
    role: str
    stack: str
    rule: str
    source: str
    evidence: str | None = None
    owner_confirmation_required: bool = False
    census: dict[str, int] = field(default_factory=dict)

    def as_dict(self) -> dict[str, Any]:
        record: dict[str, Any] = {
            "directory": self.directory,
            "id": self.project_id,
            "declaration": self.declaration,
            "declared_kind": self.kind,
            "role": self.role,
            "stack": self.stack,
            "rule": self.rule,
            "source": self.source,
        }
        if self.evidence:
            record["evidence"] = self.evidence
        if self.census:
            record["census"] = self.census
        if self.owner_confirmation_required:
            record["owner_confirmation_required"] = True
        return record


@dataclass
class RegistryPlan:
    project_id: str
    path: str
    action: str
    role: str | None = None
    stack: str | None = None
    rule: str = ""
    source: str = ""

    def as_dict(self) -> dict[str, Any]:
        return {
            "id": self.project_id,
            "path": self.path,
            "action": self.action,
            "role": self.role,
            "stack": self.stack,
            "rule": self.rule,
            "source": self.source,
        }


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise MigrationError(f"cannot read {path}: {exc}") from exc


def split_composite(value: Any, stacks: frozenset[str]) -> tuple[str | None, str | None]:
    """Split `<stack>-<rest>` into `(stack, rest)`; a bare value is `(None, value)`."""
    if not isinstance(value, str) or not value:
        return (None, None)
    for stack in sorted(stacks):
        if value == stack:
            return (stack, stack)
        if value.startswith(f"{stack}-"):
            return (stack, value[len(stack) + 1:])
    return (None, value)


def _cell_value(cell: str) -> str | None:
    match = ROW_CELL.match(cell)
    return match.group(1) if match else None


def _cell_values(cell: str) -> list[str]:
    """A decided cell names one value, or two when the owner wrote `a` or `b`.

    Two candidates are not a decision: the resolver picks the candidate the
    recorded evidence supports and marks the row `owner_confirmation_required`,
    so the choice is visible to the reviewer instead of silent.
    """
    parts = [part for part in re.findall(r"`([^`]+)`", cell) if part]
    return parts or [cell]


def parse_decided_rows(design_path: Path) -> dict[str, DecidedRow]:
    """Read the decided per-repository rows out of `design.md` §5.2 / §5.3.

    The owner's table is the authority for a decided row, so it is read from the
    change directory rather than restated here. A row keyed by directory name
    with an explicit registry id (`` `lileililiwen.github.io` `` /
    `` `lileililiwen-site` ``) keeps both spellings, because the registry `path`
    is a resolution detail and the `id` is the identity key. The result is keyed
    by both, so an eight-entry `campusMesh`-style alias resolves either way.
    """
    if not design_path.is_file():
        raise MigrationError(f"design table missing: {design_path}")
    section = ""
    rows: dict[str, DecidedRow] = {}
    for line in design_path.read_text(encoding="utf-8").splitlines():
        heading = re.match(r"^###\s+5\.(\d+)", line)
        if heading:
            section = f"5.{heading.group(1)}"
            continue
        if section not in {"5.2", "5.3"} or not line.startswith("| `"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        key = ROW_KEY.match(cells[0]) if cells else None
        if not key or len(cells) < 6:
            continue
        directory, registry_id = key.group(1), key.group(2)
        number = re.match(r"^(\d)", cells[5])
        row = DecidedRow(
            directory=directory,
            registry_id=registry_id,
            kind=_cell_value(cells[1]),
            registry_profile=_cell_value(cells[2]) if section == "5.2" else None,
            declared_profile=_cell_value(cells[2]) if section == "5.3" else None,
            census=_cell_value(cells[6]) if section == "5.3" and len(cells) > 6 else None,
            roles=_cell_values(cells[3]),
            stacks=_cell_values(cells[4]),
            rule=f"§{section} rule {number.group(1) if number else '?'}: {cells[5]}",
            section=section,
        )
        rows[directory] = row
        if registry_id:
            rows[registry_id] = row
    if not rows:
        raise MigrationError(f"no decided rows parsed from {design_path}")
    return rows


def census_stack_counts(project_path: Path) -> dict[str, int]:
    """Bounded, read-only extension census (§5.1 rule 4). Advisory only."""
    counts: dict[str, int] = {}
    seen = 0
    for dirpath, dirnames, filenames in os.walk(project_path):
        dirnames[:] = sorted(
            name for name in dirnames if name not in CENSUS_EXCLUDE and not name.startswith(".")
        )
        for filename in sorted(filenames):
            stack = CENSUS_EXTENSIONS.get(Path(filename).suffix.lower())
            if stack is None:
                continue
            counts[stack] = counts.get(stack, 0) + 1
            seen += 1
        if seen >= CENSUS_FILE_BUDGET:
            break
    return counts


def census_majority(counts: dict[str, int]) -> str | None:
    if not counts:
        return None
    return max(sorted(counts), key=lambda key: counts[key])


def resolve_role(kind: Any, stacks: frozenset[str], roles: frozenset[str]) -> tuple[str, str]:
    """The role word of a stack-qualified `kind`, or the bare `kind`.

    Returns `(role, reason)`; raises `MigrationError` rather than inventing a
    value the role set does not contain.
    """
    _, word = split_composite(kind, stacks)
    if not word:
        raise MigrationError(f"declaration declares no kind to take a role from: {kind!r}")
    if word in ROLE_OVERRIDES:
        return ROLE_OVERRIDES[word], f"§10 ruling 1: {word!r} is not a role; {ROLE_OVERRIDES[word]!r} is"
    if word in LAYOUT_ROLES:
        return LAYOUT_ROLE_FALLBACK, f"§9 defers a layout axis: {word!r} is a layout, not a role"
    if word not in roles:
        raise MigrationError(f"role word {word!r} is outside the canonical role set {sorted(roles)}")
    return word, "role word of the declaration kind"


def build_plan(workspace_root: Path, governance_root: Path) -> dict[str, Any]:
    """Resolve every declaration and registry entry, or raise `MigrationError`."""
    import vocabulary

    vocab = vocabulary.load_vocabulary_for_governance_root(governance_root)
    roles = frozenset(vocab["roles"])
    stacks = frozenset(vocab["stacks"])
    decided = parse_decided_rows(governance_root / DESIGN_RELATIVE)

    registry = load_json(governance_root / REGISTRY_RELATIVE)
    entries = [entry for entry in (registry.get("projects") or []) if isinstance(entry, dict)]
    by_path = {str(entry.get("path")): entry for entry in entries}

    declaration_plans: list[DeclarationPlan] = []
    registry_plans: list[RegistryPlan] = []

    for child in sorted(workspace_root.iterdir(), key=lambda item: item.name):
        if child.name.startswith(".") or child.is_symlink() or not child.is_dir():
            continue
        declaration_path = child / ".project.json"
        if not declaration_path.is_file():
            continue
        declaration = load_json(declaration_path)
        if not isinstance(declaration, dict):
            raise MigrationError(f"declaration is not a JSON object: {declaration_path}")
        entry = by_path.get(child.name)
        plan = _plan_declaration(
            child=child,
            declaration=declaration,
            entry=entry,
            kind=declaration.get("kind"),
            declared_profile=declaration.get("profile"),
            registry_profile=entry.get("profile") if entry else None,
            row=decided.get(child.name) or decided.get(str(declaration.get("id"))),
            roles=roles,
            stacks=stacks,
            workspace_root=workspace_root,
        )
        declaration_plans.append(plan)

        if entry is None:
            # §5.3: an active declaration with no registry entry is registered,
            # which clears the existing DISCOVERED_UNREGISTERED finding. No new
            # finding code is introduced for it.
            registry_plans.append(RegistryPlan(
                project_id=plan.project_id,
                path=child.name,
                action="add",
                role=plan.role,
                stack=plan.stack,
                rule=plan.rule,
                source=f"design.md §5.3: register {plan.project_id!r} (clears DISCOVERED_UNREGISTERED)",
            ))
        else:
            registry_plans.append(RegistryPlan(
                project_id=plan.project_id,
                path=str(entry.get("path")),
                action="axes",
                role=plan.role,
                stack=plan.stack,
                rule=plan.rule,
                source=plan.source,
            ))

    planned_registry_ids = {plan.project_id for plan in registry_plans}
    workspace_root_resolved = workspace_root.resolve()
    for entry in entries:
        path_value = str(entry.get("path"))
        project_path = (workspace_root / path_value).resolve()
        try:
            project_path.relative_to(workspace_root_resolved)
        except ValueError as exc:
            raise MigrationError(f"registry path escapes workspace root: {path_value!r}") from exc
        if project_path.is_dir():
            # The portfolio registry may point at ordinary nested monorepo
            # members. Their identity/path rows are already authoritative; keep
            # them in the plan instead of misclassifying them as absent merely
            # because the workspace's top-level walk does not descend.
            if path_value not in {child.name for child in workspace_root.iterdir()}:
                declaration_path = project_path / ".project.json"
                if declaration_path.is_file() and str(entry.get("id")) not in planned_registry_ids:
                    registry_plans.append(RegistryPlan(
                        project_id=str(entry.get("id")),
                        path=path_value,
                        action="axes",
                        role=str(entry.get("role") or ""),
                        stack=str(entry.get("stack") or ""),
                        rule="registered nested project path",
                        source="existing registry path resolves to an in-workspace nested project declaration",
                    ))
                    planned_registry_ids.add(str(entry.get("id")))
            continue
        # §5.4: a ghost entry is removed or archived. It already reports ERROR
        # PROJECT_MISSING; the remediation reuses that code, invents no second
        # one, and never recreates the directory.
        registry_plans.append(RegistryPlan(
            project_id=str(entry.get("id")),
            path=path_value,
            action="remove",
            rule="§5.4",
            source=f"design.md §5.4: no directory at {path_value!r} (clears PROJECT_MISSING)",
        ))

    _assert_id_immutability(entries, registry_plans)

    return {
        "schema_version": 1,
        "change": CHANGE_ID,
        "generated_from": {
            "design": DESIGN_RELATIVE,
            "sections": ["5.1", "5.2", "5.3", "5.4", "5.5"],
            "registry": REGISTRY_RELATIVE,
        },
        "status": {
            "applied": False,
            "authority": "scripts/axis_migration.py --check reads the live fleet; this block is a record, not the source of truth.",
            "note": (
                "The fleet write is a cross-repo source edit and is not applied by the change that "
                "ships the schema and the validators. Apply it with "
                "`python3 scripts/axis_migration.py --apply` under explicit authorization, then set "
                "TAXONOMY_AUDIT_ENABLED = True in scripts/workspace_check.py in the same commit "
                "(design.md §10, ruling 2)."
            ),
        },
        "totals": {
            "declarations": len(declaration_plans),
            "registry_axes": sum(1 for plan in registry_plans if plan.action == "axes"),
            "registry_added": sum(1 for plan in registry_plans if plan.action == "add"),
            "registry_removed": sum(1 for plan in registry_plans if plan.action == "remove"),
            "existing_ids_unchanged": len(entries) - sum(1 for plan in registry_plans if plan.action == "remove"),
        },
        "roles": _tally(plan.role for plan in declaration_plans),
        "stacks": _tally(plan.stack for plan in declaration_plans),
        "declarations": [plan.as_dict() for plan in declaration_plans],
        "registry": [plan.as_dict() for plan in registry_plans],
    }


def _assert_id_immutability(entries: list[dict[str, Any]], registry_plans: list[RegistryPlan]) -> None:
    """`id` is the join key: no existing id or path may move (§5.5, ruling 3)."""
    before = {(str(entry.get("id")), str(entry.get("path"))) for entry in entries}
    removed = {(plan.project_id, plan.path) for plan in registry_plans if plan.action == "remove"}
    added = {(plan.project_id, plan.path) for plan in registry_plans if plan.action == "add"}
    after = {(plan.project_id, plan.path) for plan in registry_plans if plan.action != "remove"}
    lost = sorted(before - after - removed)
    if lost:
        raise MigrationError(f"id immutability violation: existing id/path pairs would change: {lost}")
    unexpected = sorted(after - before - added)
    if unexpected:
        raise MigrationError(f"id immutability violation: undeclared id/path pairs introduced: {unexpected}")


def _plan_declaration(
    *,
    child: Path,
    declaration: dict[str, Any],
    entry: dict[str, Any] | None,
    kind: Any,
    declared_profile: Any,
    registry_profile: Any,
    row: DecidedRow | None,
    roles: frozenset[str],
    stacks: frozenset[str],
    workspace_root: Path,
) -> DeclarationPlan:
    project_id = str(declaration.get("id") or (entry or {}).get("id") or child.name)
    relative = (workspace_root / child.name / ".project.json").relative_to(workspace_root).as_posix()
    counts: dict[str, int] = {}
    evidence: str | None = None
    needs_confirmation = False

    if row is not None:
        role_candidates = list(row.roles)
        stack_candidates = list(row.stacks)
        rule = row.rule
        source = f"design.md {row.rule}"
        _assert_row_matches_fleet(project_id, row, kind, declared_profile, registry_profile, entry)
    else:
        kind_stack, _ = split_composite(kind, stacks)
        declared_stack, _ = split_composite(declared_profile, stacks)
        registry_stack, _ = split_composite(registry_profile, stacks)
        rule = "§5.1 rules 1-3: stack-qualified kind, else declaration profile, else registry profile"
        source = "design.md §5.1 resolution order applied to the declaration"
        qualified = [item for item in (kind_stack, declared_stack, registry_stack) if item]
        if qualified:
            stack_candidates = [qualified[0]]
        else:
            counts = census_stack_counts(child)
            majority = census_majority(counts)
            if majority is None:
                # Rule 5: recorded explicitly, never as a silent fallback.
                stack_candidates = ["none"]
                rule = "§5.1 rule 5: no implementation language, recorded explicitly"
                evidence = "bounded census found no file in the census extension set"
            else:
                stack_candidates = [majority]
                rule = "§5.1 rule 4: bounded offline census (advisory; confirm before applying)"
                evidence = "census: " + _census_phrase(counts)
        role, reason = resolve_role(kind, stacks, roles)
        role_candidates = [role]
        evidence = "; ".join(filter(None, [evidence, reason]))

    role = role_candidates[0]
    if role not in roles:
        raise MigrationError(f"{project_id}: resolved role {role!r} is outside the canonical role set {sorted(roles)}")

    stack = stack_candidates[0]
    if len(stack_candidates) > 1:
        if not counts:
            counts = census_stack_counts(child)
        supported = [candidate for candidate in stack_candidates if counts.get(candidate, 0) > 0]
        if len(supported) == 1:
            stack = supported[0]
        needs_confirmation = True
        evidence = "; ".join(filter(None, [
            evidence,
            f"the decided cell offers {' or '.join(repr(item) for item in stack_candidates)}; "
            f"chosen {stack!r} on the census evidence, owner confirmation required",
        ]))
    if stack not in stacks:
        raise MigrationError(f"{project_id}: resolved stack {stack!r} is outside the canonical stack set {sorted(stacks)}")

    return DeclarationPlan(
        directory=child.name,
        project_id=project_id,
        declaration=relative,
        kind=str(kind),
        role=role,
        stack=stack,
        rule=rule,
        source=source,
        evidence=evidence,
        owner_confirmation_required=needs_confirmation,
        census=dict(sorted(counts.items())),
    )


def _assert_row_matches_fleet(
    project_id: str,
    row: DecidedRow,
    kind: Any,
    declared_profile: Any,
    registry_profile: Any,
    entry: dict[str, Any] | None,
) -> None:
    """A decided row must still describe the repository it was decided for."""
    if row.kind and row.kind != kind:
        # The decided table records the value the repository carried when the row
        # was decided. The migration writes the DERIVED composite, so on every
        # run after a successful apply the declaration carries the derived value
        # instead. Accepting both keeps the migration idempotent: without this the
        # tool would report "the fleet has drifted" about its own output and could
        # never be re-run. Any other value really is drift and is refused.
        accepted = {row.kind}
        for role in row.roles or ():
            for stack in row.stacks or ("none",):
                accepted.add(role if stack == "none" else f"{stack}-{role}")
        if kind not in accepted:
            raise MigrationError(
                f"{project_id}: design.md {row.rule} records kind {row.kind!r} but the declaration says "
                f"{kind!r}; the fleet has drifted from the decided table"
            )
    if row.registry_profile and registry_profile is not None and row.registry_profile != registry_profile:
        raise MigrationError(
            f"{project_id}: design.md {row.rule} records registry profile {row.registry_profile!r} but the "
            f"registry says {registry_profile!r}; the fleet has drifted from the decided table"
        )
    if row.declared_profile and row.declared_profile != declared_profile:
        raise MigrationError(
            f"{project_id}: design.md {row.rule} records declaration profile {row.declared_profile!r} but the "
            f"declaration says {declared_profile!r}; the fleet has drifted from the decided table"
        )
    if row.registry_profile and entry is None:
        raise MigrationError(
            f"{project_id}: design.md {row.rule} records a registry profile but no registry entry exists; "
            "the fleet has drifted from the decided table"
        )


def _census_phrase(counts: dict[str, int]) -> str:
    return ", ".join(f"{key} {value}" for key, value in sorted(counts.items(), key=lambda item: (-item[1], item[0])))


def _tally(values: Any) -> dict[str, int]:
    counts: dict[str, int] = {}
    for value in values:
        counts[value] = counts.get(value, 0) + 1
    return dict(sorted(counts.items()))


def outstanding_rows(plan: dict[str, Any], workspace_root: Path, governance_root: Path) -> list[str]:
    """Rows the migration has not reached, read from the live fleet."""
    issues: list[str] = []
    for record in plan["declarations"]:
        path = workspace_root / record["declaration"]
        if not path.is_file():
            issues.append(f"{record['id']}: declaration missing at {record['declaration']}")
            continue
        declaration = load_json(path)
        if declaration.get("role") != record["role"] or declaration.get("stack") != record["stack"]:
            issues.append(
                f"{record['id']}: declaration role/stack "
                f"{(declaration.get('role'), declaration.get('stack'))} != planned ({record['role']}, {record['stack']})"
            )
    registry = load_json(governance_root / REGISTRY_RELATIVE)
    entries = {str(entry.get("id")): entry for entry in registry.get("projects") or [] if isinstance(entry, dict)}
    for record in plan["registry"]:
        entry = entries.get(record["id"])
        if record["action"] == "remove":
            if entry is not None:
                issues.append(f"{record['id']}: ghost entry is still registered")
            continue
        if entry is None:
            issues.append(f"{record['id']}: registry entry is missing")
        elif entry.get("role") != record["role"] or entry.get("stack") != record["stack"]:
            issues.append(
                f"{record['id']}: registry role/stack {(entry.get('role'), entry.get('stack'))} != "
                f"planned ({record['role']}, {record['stack']})"
            )
    planned_ids = {record["id"] for record in plan["registry"]}
    for project_id in sorted(set(entries) - planned_ids):
        issues.append(f"{project_id}: registry entry is not covered by the plan")
    return issues


def apply_plan(plan: dict[str, Any], workspace_root: Path, governance_root: Path) -> list[str]:
    """Write the migration. Reached only from `--apply`, after a full validation."""
    import vocabulary

    roles = frozenset(vocabulary.load_vocabulary_for_governance_root(governance_root)["roles"])
    stacks = frozenset(vocabulary.load_vocabulary_for_governance_root(governance_root)["stacks"])
    for record in plan["declarations"]:
        if record["role"] not in roles or record["stack"] not in stacks:
            raise MigrationError(f"{record['id']}: planned axes are not canonical; refusing to write")

    written: list[str] = []
    for record in plan["declarations"]:
        path = workspace_root / record["declaration"]
        declaration = load_json(path)
        derived_kind = record["role"] if record["stack"] == "none" else f"{record['stack']}-{record['role']}"
        if (
            declaration.get("role") == record["role"]
            and declaration.get("stack") == record["stack"]
            and declaration.get("kind") == derived_kind
        ):
            continue
        updated: dict[str, Any] = {}
        for key, value in declaration.items():
            updated[key] = value
            if key == "kind":
                updated["role"] = record["role"]
                updated["stack"] = record["stack"]
                # `kind` is the derived, deprecated composite and is never
                # hand-authored (ruling 5). This migration is the one pass that
                # stops it being hand-authored, so it writes the derived value.
                #
                # The previous hand-authored value is NOT lost: it is recorded as
                # `declared_kind` on this row in the committed plan, so the
                # measurement survives the write. Leaving `kind` stale instead
                # would put 81 permanent `TAXONOMY_LEGACY_VALUE` warnings on the
                # fleet forever, and a warning nobody can ever clear is noise that
                # teaches people to ignore the checker.
                updated["kind"] = derived_kind
        if "role" not in updated:
            updated["role"] = record["role"]
            updated["stack"] = record["stack"]
            updated["kind"] = derived_kind
        path.write_text(json.dumps(updated, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        written.append(
            f"{record['declaration']}: role={record['role']} stack={record['stack']} "
            f"kind={record.get('declared_kind')}->{derived_kind}"
        )

    registry_path = governance_root / REGISTRY_RELATIVE
    registry = load_json(registry_path)
    plans = {record["id"]: record for record in plan["registry"]}
    declaration_by_id = {record["id"]: record for record in plan["declarations"]}
    kept: list[dict[str, Any]] = []
    for entry in registry["projects"]:
        record = plans.get(str(entry.get("id")))
        if record is None:
            kept.append(entry)
            continue
        if record["action"] == "remove":
            written.append(f"registry: removed {entry.get('id')} ({record['source']})")
            continue
        updated: dict[str, Any] = {}
        for key, value in entry.items():
            updated[key] = value
            if key == "profile":
                updated["role"] = record["role"]
                updated["stack"] = record["stack"]
        if "role" not in updated:
            updated["role"] = record["role"]
            updated["stack"] = record["stack"]
        kept.append(updated)
        written.append(f"registry: {entry.get('id')} role={record['role']} stack={record['stack']}")
    for record in plan["registry"]:
        if record["action"] != "add":
            continue
        declaration = load_json(workspace_root / declaration_by_id[record["id"]]["declaration"])
        kept.append({
            "id": record["id"],
            "path": record["path"],
            "profile": str(declaration.get("profile") or "inventory"),
            "lifecycle": str(declaration.get("lifecycle") or "active"),
            "adoption": "adopted",
            "role": record["role"],
            "stack": record["stack"],
        })
        written.append(f"registry: added {record['id']} (role={record['role']} stack={record['stack']})")
    registry["projects"] = kept
    registry_path.write_text(json.dumps(registry, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return written


def print_plan(plan: dict[str, Any], stream: Any) -> None:
    totals = plan["totals"]
    print(
        f"identity-and-taxonomy axis migration: {totals['declarations']} declarations, "
        f"{totals['registry_axes']} registry axes, {totals['registry_added']} registry entries to add, "
        f"{totals['registry_removed']} ghost entry to remove; "
        f"{totals['existing_ids_unchanged']} existing ids unchanged",
        file=stream,
    )
    print("roles: " + ", ".join(f"{key} {value}" for key, value in plan["roles"].items()), file=stream)
    print("stacks: " + ", ".join(f"{key} {value}" for key, value in plan["stacks"].items()), file=stream)
    print("", file=stream)
    for record in plan["declarations"]:
        flag = "  [owner confirmation required]" if record.get("owner_confirmation_required") else ""
        print(
            f"  {record['declaration']:46s} id={record['id']:24s} role={record['role']:10s} "
            f"stack={record['stack']:10s} {record['rule']}{flag}",
            file=stream,
        )
    print("", file=stream)
    for record in plan["registry"]:
        if record["action"] == "axes":
            continue
        print(
            f"  registry {record['action']:6s} {record['id']:24s} role={record['role']} stack={record['stack']}  {record['source']}",
            file=stream,
        )


def write_plan(plan: dict[str, Any], governance_root: Path) -> Path:
    target = governance_root / PLAN_RELATIVE
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
                        help="governance root holding vocabulary.json, the design table and the registry; "
                             "defaults to this repository. Point it at a copy to rehearse --apply.")
    parser.add_argument("--json", action="store_true", dest="as_json", help="print the plan as JSON (dry run)")
    parser.add_argument("--check", action="store_true", help="report whether the migration has been applied")
    parser.add_argument("--apply", action="store_true", help="WRITE the migration to the fleet (authorized step)")
    parser.add_argument("--write-plan", action="store_true", help="refresh the committed plan file from a dry run")
    args = parser.parse_args(argv)

    workspace_root = Path(args.root).expanduser().resolve()
    governance_root = Path(args.governance_root).expanduser().resolve() if args.governance_root else GOVERNANCE_ROOT
    try:
        plan = build_plan(workspace_root, governance_root)
    except MigrationError as exc:
        print(f"refused: {exc}", file=sys.stderr)
        return 1

    if args.write_plan:
        target = write_plan(plan, governance_root)
        print(f"wrote {target.relative_to(governance_root)}", file=sys.stderr)

    if args.apply:
        try:
            written = apply_plan(plan, workspace_root, governance_root)
        except MigrationError as exc:
            print(f"refused: {exc}", file=sys.stderr)
            return 1
        print(f"applied {len(written)} writes", file=sys.stderr)
        for line in written:
            print(f"  {line}", file=sys.stderr)
        print("now set TAXONOMY_AUDIT_ENABLED = True in scripts/workspace_check.py in the same commit", file=sys.stderr)
        return 0

    if args.check:
        issues = outstanding_rows(plan, workspace_root, governance_root)
        if issues:
            print(f"migration not applied: {len(issues)} row(s) outstanding", file=sys.stderr)
            for issue in issues[:20]:
                print(f"  {issue}", file=sys.stderr)
            if len(issues) > 20:
                print(f"  ... and {len(issues) - 20} more", file=sys.stderr)
            print("TAXONOMY_AUDIT_ENABLED must stay False until every row is applied", file=sys.stderr)
            return 3
        print("migration applied: every declaration and registry entry carries the planned axes")
        print("TAXONOMY_AUDIT_ENABLED may now be set to True in the same commit")
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
