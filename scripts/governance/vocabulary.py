#!/usr/bin/env python3
"""Single source for the portfolio vocabulary owned by Workspace Governance.

`vocabulary.json` at the governance root is the only machine-readable source
for the canonical `roles` and `stacks` (the two independent project
classification axes), the canonical `profiles`, canonical `kinds`,
`placeholder_markers` and `secret_field_substrings`, and the closed
`readme_claim_surfaces` / `readme_absence_phrases` pair used by the
`readme-claim-verification` rule. Consumers
(`workspace_check.py`, `product_code_quality.py`, `maturity_audit.py`) load it
through this module.

The loader is fail-closed: a missing, unparseable, duplicate-bearing or
empty-list vocabulary raises `VocabularyError` naming the file and the
problem. Consumers must surface that as a non-zero exit, never fall back to
an empty list (an empty secret list would silently disable the scan, and an
empty `roles`/`stacks` list would mark every repository in the workspace
unknown). See `openspec/specs/identity-and-taxonomy/spec.md`.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any


VOCABULARY_FILENAME = "vocabulary.json"
REQUIRED_KEYS = ("profiles", "kinds", "placeholder_markers", "secret_field_substrings")
# The two independent classification axes (see
# `openspec/specs/identity-and-taxonomy/spec.md`). Required and fail-closed for
# the same reason as the required keys: an empty axis set would mark every
# project unknown, so the checker must exit non-zero rather than evaluate the
# portfolio against nothing.
AXIS_ENUM_KEYS = ("roles", "stacks")
# Additive, fail-closed enums for the standalone-project-boundary capability.
# Each must be a non-empty list of distinct strings; the loader raises
# `VocabularyError` if any value is missing, empty, duplicated, or non-string.
OPTIONAL_ENUM_KEYS = ("standalone_statuses", "integration_required_states")
# The closed README-claim surface set and the absence-phrase vocabulary for the
# `readme-claim-verification` capability. Optional keys for the same reason as
# the enums above, and fail-closed when present but unusable: an empty
# `readme_claim_surfaces` would make every README claim vacuously true, which is
# a silent pass on the one thing the check exists to catch. The vocabulary is
# never widened per repository — see `design.md` §10 ruling 3.
README_CLAIM_KEYS = ("readme_claim_surfaces", "readme_absence_phrases")


class VocabularyError(ValueError):
    """Raised when `vocabulary.json` is missing or unusable."""


_CACHE: dict[str, dict[str, Any]] = {}


def vocabulary_path(governance_root: Path) -> Path:
    return Path(governance_root) / VOCABULARY_FILENAME


def load_vocabulary(path: Path) -> dict[str, Any]:
    """Load and validate the vocabulary file at `path`.

    Returns `{"schema_version": int, "roles": frozenset, "stacks":
    frozenset, "profiles": frozenset, "kinds": frozenset,
    "placeholder_markers": tuple, "secret_field_substrings": tuple}`, plus
    the optional `readme_claim_surfaces` / `readme_absence_phrases` tuples when
    the file carries them.
    Raises `VocabularyError` naming the file on any problem.
    """
    path = Path(path)
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise VocabularyError(f"vocabulary file missing: {path}")
    except OSError as exc:
        raise VocabularyError(f"cannot read vocabulary file {path}: {exc}")
    except json.JSONDecodeError as exc:
        raise VocabularyError(f"vocabulary file is not valid JSON: {path}: {exc}")
    if not isinstance(raw, dict):
        raise VocabularyError(f"vocabulary file must contain a JSON object: {path}")
    if raw.get("schema_version") != 1:
        raise VocabularyError(f"vocabulary schema_version must be 1: {path}")
    vocabulary: dict[str, Any] = {"schema_version": 1}
    for key in REQUIRED_KEYS + AXIS_ENUM_KEYS:
        value = raw.get(key)
        if not isinstance(value, list) or not value:
            raise VocabularyError(f"vocabulary {key!r} must be a non-empty list: {path}")
        for entry in value:
            if not isinstance(entry, str) or not entry:
                raise VocabularyError(f"vocabulary {key!r} entries must be non-empty strings: {path}")
        if len(set(value)) != len(value):
            raise VocabularyError(f"vocabulary {key!r} contains a duplicate entry: {path}")
        vocabulary[key] = frozenset(value) if key in ("roles", "stacks", "profiles", "kinds") else tuple(value)
    for key in OPTIONAL_ENUM_KEYS:
        if key not in raw:
            continue
        value = raw[key]
        if not isinstance(value, list) or not value:
            raise VocabularyError(f"vocabulary {key!r} must be a non-empty list: {path}")
        for entry in value:
            if not isinstance(entry, str) or not entry:
                raise VocabularyError(f"vocabulary {key!r} entries must be non-empty strings: {path}")
        if len(set(value)) != len(value):
            raise VocabularyError(f"vocabulary {key!r} contains a duplicate entry: {path}")
        vocabulary[key] = frozenset(value)
    for key in README_CLAIM_KEYS:
        if key not in raw:
            continue
        value = raw[key]
        if not isinstance(value, list) or not value:
            raise VocabularyError(f"vocabulary {key!r} must be a non-empty list: {path}")
        for entry in value:
            if not isinstance(entry, str) or not entry:
                raise VocabularyError(f"vocabulary {key!r} entries must be non-empty strings: {path}")
        if len(set(value)) != len(value):
            raise VocabularyError(f"vocabulary {key!r} contains a duplicate entry: {path}")
        # A tuple, not a frozenset: the surface set is reported in a stable
        # order so a finding stream and a coverage metric are reproducible.
        vocabulary[key] = tuple(value)
    return vocabulary


def load_vocabulary_for_governance_root(governance_root: Path) -> dict[str, Any]:
    """Load the vocabulary for a governance root, caching per resolved path."""
    key = str(Path(governance_root).resolve())
    cached = _CACHE.get(key)
    if cached is None:
        cached = load_vocabulary(vocabulary_path(governance_root))
        _CACHE[key] = cached
    return cached


def secret_pattern(substrings: tuple[str, ...]) -> "re.Pattern[str]":
    """Compile a case-insensitive substring pattern for secret-shaped keys."""
    ordered = sorted(substrings, key=len, reverse=True)
    return re.compile("(" + "|".join(re.escape(item) for item in ordered) + ")", re.IGNORECASE)


def clear_cache() -> None:
    _CACHE.clear()
