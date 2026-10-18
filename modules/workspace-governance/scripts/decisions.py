#!/usr/bin/env python3
"""Single source for the workspace toolchain decision register.

`decisions.json` at the governance root is the only machine-readable source
for toolchain decisions D1.. with status lifecycle, owner project, decision
text, policy boundary notes, and downstream references to registered
repositories plus their planned OpenSpec change names.

The loader is fail-closed: a missing, unparseable, duplicate-bearing, or
shape-violating register raises `DecisionError` naming the file and the
problem. Consumers must surface that as a non-zero exit, never fall back to
an empty list (an empty decision list would silently disable the audit).
Unknown top-level keys are fatal; unknown entry-level keys are tolerated
(forward compatibility for additive fields decided in design L4).
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


DECISIONS_FILENAME = "decisions.json"
DECISION_FORMAT_VERSION = 1
DECISION_TOP_LEVEL_KEYS = frozenset({"version", "decisions"})
DECISION_STATUSES = frozenset({"proposed", "accepted", "rejected", "superseded"})
DECISION_REQUIRED_FIELDS = (
    "id",
    "title",
    "status",
    "date",
    "owner_project",
    "decision",
    "downstream",
)
DECISION_KNOWN_ENTRY_FIELDS = frozenset(DECISION_REQUIRED_FIELDS) | {"policy_boundary"}

DECISION_ID_PATTERN = re.compile(r"^D[0-9]{1,3}$")
KEBAB_PATTERN = re.compile(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$")
ISO_DATE_PATTERN = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$")


class DecisionError(ValueError):
    """Raised when `decisions.json` is missing or unusable."""


@dataclass(frozen=True)
class Decision:
    """A single toolchain decision loaded from the register."""

    id: str
    title: str
    status: str
    date: str
    owner_project: str
    decision: str
    downstream: tuple[dict[str, str], ...] = field(default_factory=tuple)
    policy_boundary: str | None = None
    extra_fields: frozenset[str] = field(default_factory=frozenset)


def decisions_path(governance_root: Path) -> Path:
    return Path(governance_root) / DECISIONS_FILENAME


def _require(entry: dict[str, Any], field_name: str, entry_index: int, path: Path) -> Any:
    if field_name not in entry:
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}] missing required field {field_name!r}"
        )
    return entry[field_name]


def _parse_entry(raw: Any, entry_index: int, path: Path) -> Decision:
    if not isinstance(raw, dict):
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}] must be a JSON object"
        )
    for field_name in DECISION_REQUIRED_FIELDS:
        if field_name not in raw:
            raise DecisionError(
                f"decisions register {path}: entry[{entry_index}] missing required field {field_name!r}"
            )
    decision_id = _require(raw, "id", entry_index, path)
    if not isinstance(decision_id, str) or not DECISION_ID_PATTERN.fullmatch(decision_id):
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].id must match ^D[0-9]{{1,3}}$; got {decision_id!r}"
        )
    title = raw["title"]
    if not isinstance(title, str) or not title:
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].title must be a non-empty string"
        )
    status = raw["status"]
    if status not in DECISION_STATUSES:
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].status must be one of "
            f"{sorted(DECISION_STATUSES)}; got {status!r}"
        )
    date = raw["date"]
    if not isinstance(date, str) or not ISO_DATE_PATTERN.fullmatch(date):
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].date must be ISO-8601 (YYYY-MM-DD); got {date!r}"
        )
    owner_project = raw["owner_project"]
    if not isinstance(owner_project, str) or not owner_project:
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].owner_project must be a non-empty string"
        )
    decision_text = raw["decision"]
    if not isinstance(decision_text, str) or not decision_text:
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].decision must be a non-empty string"
        )
    downstream_raw = raw["downstream"]
    if not isinstance(downstream_raw, list):
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].downstream must be an array"
        )
    downstream: list[dict[str, str]] = []
    for downstream_index, item in enumerate(downstream_raw):
        if not isinstance(item, dict):
            raise DecisionError(
                f"decisions register {path}: entry[{entry_index}].downstream[{downstream_index}] must be an object"
            )
        if "repo" not in item or not isinstance(item["repo"], str) or not item["repo"]:
            raise DecisionError(
                f"decisions register {path}: entry[{entry_index}].downstream[{downstream_index}].repo must be a non-empty string"
            )
        if "change" not in item or not isinstance(item["change"], str) or not item["change"]:
            raise DecisionError(
                f"decisions register {path}: entry[{entry_index}].downstream[{downstream_index}].change must be a non-empty string"
            )
        if not KEBAB_PATTERN.fullmatch(item["change"]):
            raise DecisionError(
                f"decisions register {path}: entry[{entry_index}].downstream[{downstream_index}].change "
                f"must be kebab-case; got {item['change']!r}"
            )
        downstream.append({"repo": item["repo"], "change": item["change"]})
    policy_boundary = raw.get("policy_boundary")
    if policy_boundary is not None and not isinstance(policy_boundary, str):
        raise DecisionError(
            f"decisions register {path}: entry[{entry_index}].policy_boundary must be a string when present"
        )
    extra = frozenset(
        key for key in raw.keys() if key not in DECISION_KNOWN_ENTRY_FIELDS
    )
    return Decision(
        id=decision_id,
        title=title,
        status=status,
        date=date,
        owner_project=owner_project,
        decision=decision_text,
        downstream=tuple(downstream),
        policy_boundary=policy_boundary,
        extra_fields=extra,
    )


def load_decisions(path: Path) -> list[Decision]:
    """Load and validate the decisions register at `path`.

    Returns a list of `Decision`. Raises `DecisionError` naming the file on
    any problem; callers must surface the error and never substitute an
    empty list.
    """
    path = Path(path)
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise DecisionError(f"decisions register missing: {path}")
    except OSError as exc:
        raise DecisionError(f"cannot read decisions register {path}: {exc}")
    except json.JSONDecodeError as exc:
        raise DecisionError(f"decisions register is not valid JSON: {path}: {exc}")
    if not isinstance(raw, dict):
        raise DecisionError(f"decisions register must contain a JSON object: {path}")
    if raw.get("version") != DECISION_FORMAT_VERSION:
        raise DecisionError(
            f"decisions register {path}: version must be {DECISION_FORMAT_VERSION}; got {raw.get('version')!r}"
        )
    unknown_top = sorted(key for key in raw.keys() if key not in DECISION_TOP_LEVEL_KEYS)
    if unknown_top:
        raise DecisionError(
            f"decisions register {path}: unknown top-level keys {unknown_top}; allowed: {sorted(DECISION_TOP_LEVEL_KEYS)}"
        )
    decisions_raw = raw.get("decisions")
    if not isinstance(decisions_raw, list):
        raise DecisionError(f"decisions register {path}: 'decisions' must be an array")
    seen: set[str] = set()
    decisions: list[Decision] = []
    for index, entry in enumerate(decisions_raw):
        parsed = _parse_entry(entry, index, path)
        if parsed.id in seen:
            raise DecisionError(
                f"decisions register {path}: duplicate id {parsed.id!r} (ids are permanent)"
            )
        seen.add(parsed.id)
        decisions.append(parsed)
    return decisions


def validate_against_registry(
    decisions: list[Decision], registry_ids: set[str] | frozenset[str]
) -> list[str]:
    """Return a sorted list of unknown `owner_project` / `downstream[].repo` references.

    Empty list means every reference is known to the registry. The function
    does not raise; the caller decides how to surface offenders.
    """
    offenders: set[str] = set()
    for entry in decisions:
        if entry.owner_project not in registry_ids:
            offenders.add(entry.owner_project)
        for downstream in entry.downstream:
            if downstream["repo"] not in registry_ids:
                offenders.add(downstream["repo"])
    return sorted(offenders)
