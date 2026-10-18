#!/usr/bin/env python3
"""Fail-closed governance checks for the local project portfolio."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from dataclasses import dataclass, asdict
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any


PROJECT_ID = re.compile(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$")
CHANGE_ID = PROJECT_ID
OPENSPEC_CHANGE_SERIALITY = "OPENSPEC_CHANGE_SERIALITY"
LIFECYCLES = {"active", "planning", "reference", "inventory", "archived"}


# Decision register codes (see scripts/decisions.py and the
# `workspace-decision-register` canonical spec).
DECISIONS_REGISTER_MISSING = "DECISIONS_REGISTER_MISSING"
DECISIONS_REGISTER_ABSENT = "DECISIONS_REGISTER_ABSENT"
DECISIONS_REGISTER_INVALID = "DECISIONS_REGISTER_INVALID"
DECISIONS_REGISTRY_REFERENCE = "DECISIONS_REGISTRY_REFERENCE"

# Standalone-project-boundary finding codes. Errors block the
# donation contract; warnings surface non-fatal signals. See
# `openspec/specs/standalone-project-boundary/spec.md`.
STANDALONE_BLOCKED = "STANDALONE_BLOCKED"
STANDALONE_STATUS_INVALID = "STANDALONE_STATUS_INVALID"
STANDALONE_SHAPE = "STANDALONE_SHAPE"
STANDALONE_ABSOLUTE_PATH = "STANDALONE_ABSOLUTE_PATH"
STANDALONE_PATH_ESCAPE = "STANDALONE_PATH_ESCAPE"
STANDALONE_REQUIRED_SIBLING = "STANDALONE_REQUIRED_SIBLING"
STANDALONE_REQUIRED_SERVICE = "STANDALONE_REQUIRED_SERVICE"
STANDALONE_OPTIONAL = "STANDALONE_OPTIONAL"
STANDALONE_INTEGRATION_PENDING = "STANDALONE_INTEGRATION_PENDING"
STANDALONE_EVIDENCE_MISSING = "STANDALONE_EVIDENCE_MISSING"
STANDALONE_SECRET_VALUE = "STANDALONE_SECRET_VALUE"
STANDALONE_DETECTED_SIBLING = "STANDALONE_DETECTED_SIBLING"

# The canonical kind/profile sets and the secret scan pattern live in
# vocabulary.json (see scripts/vocabulary.py). This module keeps no local
# duplicate list: SECRET_WORDS and PROFILES resolve lazily from the
# vocabulary so `from workspace_check import SECRET_WORDS` (used by the
# Forge governance adapter) keeps working.


def _default_governance_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _vocabulary_for(governance_root: Path | None) -> dict[str, Any]:
    import vocabulary

    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    return vocabulary.load_vocabulary_for_governance_root(root)


RULES_DRIFT_BASELINE_FILE = "rules-drift-baseline.json"
RULES_DRIFT_BASELINE_KEYS = ("project", "file", "label")


class RulesDriftBaselineError(ValueError):
    """The ratchet baseline is unusable. Fail closed; never fall back to empty."""


def load_rules_drift_baseline(governance_root: Path | None = None) -> frozenset[tuple[str, str, str]]:
    """Load the ratchet baseline as a set of `(project, file, label)` keys.

    Fail-closed, for the same reason the vocabulary loader does: an empty or
    defaulted baseline would make every existing finding "new" and fail the whole
    fleet for the wrong reason. Every one of these conditions raises
    `RulesDriftBaselineError` naming the file and the reason:

    - the file is missing;
    - it is not valid JSON;
    - the root is not an object;
    - `findings` is absent, not a list, or empty;
    - an entry is not an object, is missing a key, carries a non-string value, or
      carries an unknown key.

    An unknown key is rejected rather than ignored, so a typo in a hand edit fails
    loudly instead of silently dropping a baselined finding.
    """
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = root / RULES_DRIFT_BASELINE_FILE
    if not path.is_file():
        raise RulesDriftBaselineError(f"missing rules drift baseline: {path}")
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise RulesDriftBaselineError(f"unreadable rules drift baseline {path}: {exc}") from exc
    if not isinstance(document, dict):
        raise RulesDriftBaselineError(f"rules drift baseline {path} is not a JSON object")
    entries = document.get("findings")
    if not isinstance(entries, list) or not entries:
        raise RulesDriftBaselineError(f"rules drift baseline {path} has no non-empty 'findings' array")

    keys: set[tuple[str, str, str]] = set()
    for index, entry in enumerate(entries):
        where = f"rules drift baseline {path} entry {index}"
        if not isinstance(entry, dict):
            raise RulesDriftBaselineError(f"{where} is not a JSON object")
        unknown = sorted(set(entry) - set(RULES_DRIFT_BASELINE_KEYS))
        if unknown:
            raise RulesDriftBaselineError(f"{where} has unknown key(s): {', '.join(unknown)}")
        missing = [name for name in RULES_DRIFT_BASELINE_KEYS if name not in entry]
        if missing:
            raise RulesDriftBaselineError(f"{where} is missing key(s): {', '.join(missing)}")
        if not all(isinstance(entry[name], str) and entry[name] for name in RULES_DRIFT_BASELINE_KEYS):
            raise RulesDriftBaselineError(f"{where} has a non-string or empty value")
        keys.add(tuple(entry[name] for name in RULES_DRIFT_BASELINE_KEYS))  # type: ignore[arg-type]
    return frozenset(keys)


# The debt record `scripts/bind_gate_commands.py` writes, the ratchet of
# already-open work that predates the precondition, and the deliberate
# exceptions to it. All three are read here, never re-derived: a second parser
# over manifests would be a second source of truth for a fact `GateOracleTest`
# already pins set-equal to the gate's own unbound set, and would drift the
# first time a profile scheduled a check `checks:` never listed.
#
# The ratchet is a ratchet, not a policy: `projects.json`'s
# `discovery.unregistered_policy` is the codebase's answer for a fleet-wide
# default, and `rules-drift-baseline.json` is the answer for a recorded
# per-repository finding. Without it the rule is not a ratchet at all but a
# deadlock: 22 repositories already carry open changes, and failing work that
# predates the rule is failing the fleet for the state the rule was written to
# stop growing. So the pairs that were open when the rule landed are
# grandfathered once, in a committed list, and every pair after it is new work.
#
# The exceptions record is the same shape one step further on: a deliberate
# decision to let a specific new change through while the debt is still open.
GATE_DEBT_BASELINE_FILE = "gate-debt-baseline.json"
GATE_DEBT_BASELINE_KEYS = ("project", "change")
GATE_DEBT_EXCEPTIONS_FILE = "gate-debt-exceptions.json"
GATE_DEBT_EXCEPTIONS_KEYS = ("project", "change", "reason")


class GateDebtRecordError(ValueError):
    """The debt record is unusable. Fail closed; never fall back to empty."""


class GateDebtBaselineError(ValueError):
    """The grandfathered-work ratchet is unusable. Fail closed."""


class GateDebtExceptionsError(ValueError):
    """The deliberate-exception record is unusable. Fail closed."""


def _load_string_entries(
    path: Path, label: str, list_key: str, keys: tuple[str, ...], error: type[ValueError]
) -> list[dict[str, str]]:
    """Read `list_key`'s entries from a governance record, fail closed.

    Shared by both records so their shape rules cannot drift apart. Every one
    of these conditions raises `error` naming the file, the entry and the
    reason, and the caller then honours no entry from the file at all:

    - the file is missing, unreadable, or is not valid JSON;
    - the root is not an object, or `list_key` is absent or not a list;
    - an entry is not an object, carries a non-string or empty value, is
      missing a key, or carries a key this record does not define.

    An unknown key is refused rather than ignored, so a typo in a hand edit
    fails loudly instead of silently dropping a recorded entry. Whether an
    *empty* list is valid is the caller's rule, not this function's: the drift
    baseline requires one entry, the exception record's normal state is none.
    """
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise error(f"unreadable {label} {path}: {exc}") from exc
    if not isinstance(document, dict):
        raise error(f"{label} {path} is not a JSON object")
    entries = document.get(list_key)
    if not isinstance(entries, list):
        raise error(f"{label} {path} has no {list_key!r} array")
    validated: list[dict[str, str]] = []
    for index, entry in enumerate(entries):
        where = f"{label} {path} entry {index}"
        if not isinstance(entry, dict):
            raise error(f"{where} is not a JSON object")
        unknown = sorted(set(entry) - set(keys))
        if unknown:
            raise error(f"{where} has unknown key(s): {', '.join(unknown)}")
        missing = [name for name in keys if name not in entry]
        if missing:
            raise error(f"{where} is missing key(s): {', '.join(missing)}")
        if not all(isinstance(entry[name], str) and entry[name] for name in keys):
            raise error(f"{where} has a non-string or empty value")
        validated.append({name: entry[name] for name in keys})
    return validated


def load_gate_debt_record(governance_root: Path | None = None) -> dict[str, tuple[str, ...]]:
    """The unbound required checks `gate-debt.json` records, by repository.

    This reads the record the generator writes; it does not re-derive it. The
    file name comes from the generator's own `DEBT_FILE`, so the two cannot
    disagree about where the record lives.

    A repository absent from the returned mapping has no recorded debt, and is
    not reported by the precondition. `GateDebtRecordError` means the record
    could not be read at all, which the caller reports once for the whole run
    rather than reading as "no repository has debt".
    """
    from bind_gate_commands import DEBT_FILE  # local import keeps this module standalone

    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = root / DEBT_FILE
    if not path.is_file():
        raise GateDebtRecordError(f"missing gate debt record: {path}")
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise GateDebtRecordError(f"unreadable gate debt record {path}: {exc}") from exc
    if not isinstance(document, dict):
        raise GateDebtRecordError(f"gate debt record {path} is not a JSON object")
    repositories = document.get("repositories")
    if not isinstance(repositories, list):
        raise GateDebtRecordError(f"gate debt record {path} has no 'repositories' array")

    debt: dict[str, tuple[str, ...]] = {}
    for index, entry in enumerate(repositories):
        where = f"gate debt record {path} entry {index}"
        if not isinstance(entry, dict):
            raise GateDebtRecordError(f"{where} is not a JSON object")
        repo = entry.get("repo")
        unresolved = entry.get("unresolved")
        if not isinstance(repo, str) or not repo:
            raise GateDebtRecordError(f"{where} has no non-string 'repo'")
        if not isinstance(unresolved, list):
            raise GateDebtRecordError(f"{where} has no 'unresolved' array")
        concerns: list[str] = []
        for item in unresolved:
            if not isinstance(item, dict) or not isinstance(item.get("concern"), str) or not item["concern"]:
                raise GateDebtRecordError(f"{where} has an unresolved entry with no non-string 'concern'")
            concerns.append(item["concern"])
        if concerns:
            debt[repo] = tuple(concerns)
    return debt


def load_gate_debt_baseline(governance_root: Path | None = None) -> frozenset[tuple[str, str]]:
    """The `(project, change)` pairs that were already open when the rule landed.

    The ratchet, in the same shape and with the same refusals as
    `load_rules_drift_baseline`: missing, unreadable, not an object, no
    `findings` array, an entry that is not an object, a missing key, a
    non-string or empty value, or an unknown key each raise, and the caller then
    honours no entry at all. An empty `findings` array is valid and means every
    open change is new work.

    Written from disk by `scripts/gate_debt_baseline.py`, never by hand and
    never by a checker run: the checker reads, it does not write. The file only
    shrinks as repositories close changes and discharge debt, which is what
    makes a stale entry harmless rather than a permanent exemption.
    """
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = root / GATE_DEBT_BASELINE_FILE
    if not path.is_file():
        raise GateDebtBaselineError(f"missing gate debt baseline: {path}")
    entries = _load_string_entries(
        path, "gate debt baseline", "findings", GATE_DEBT_BASELINE_KEYS, GateDebtBaselineError
    )
    return frozenset((entry["project"], entry["change"]) for entry in entries)


def load_gate_debt_exceptions(governance_root: Path | None = None) -> dict[tuple[str, str], str]:
    """The recorded deliberate exceptions, keyed by `(project, change)`.

    An empty `exceptions` array is valid and is the normal state; every other
    malformed shape raises `GateDebtExceptionsError` naming the file, the entry
    and the reason, and the caller then honours no exception at all. There is
    no environment variable and no flag: an exception is a reviewable line in
    a committed file, which is the only exit a blocking rule may have.
    """
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = root / GATE_DEBT_EXCEPTIONS_FILE
    if not path.is_file():
        raise GateDebtExceptionsError(f"missing gate debt exceptions record: {path}")
    entries = _load_string_entries(
        path, "gate debt exceptions record", "exceptions", GATE_DEBT_EXCEPTIONS_KEYS, GateDebtExceptionsError
    )
    return {(entry["project"], entry["change"]): entry["reason"] for entry in entries}


def canonical_profiles_for(governance_root: Path | None = None) -> frozenset[str]:
    return _vocabulary_for(governance_root)["profiles"]


def canonical_kinds_for(governance_root: Path | None = None) -> frozenset[str]:
    return _vocabulary_for(governance_root)["kinds"]


def canonical_roles_for(governance_root: Path | None = None) -> frozenset[str]:
    """The canonical `role` axis. Six values; `shared-library` is not a role."""
    return _vocabulary_for(governance_root)["roles"]


def canonical_stacks_for(governance_root: Path | None = None) -> frozenset[str]:
    """The canonical `stack` axis. Single-valued; there is no `stacks` array."""
    return _vocabulary_for(governance_root)["stacks"]


def derive_kind(role: str, stack: str) -> str:
    """The derived, deprecated `kind` composite for one axis pair.

    `f"{stack}-{role}"`, or the bare `role` when the stack is `none` (ruling 5,
    `design.md` §4/§10). The derivation is permanent and is the authority a
    hand-authored `kind` is measured against.
    """
    if stack == "none":
        return role
    return f"{stack}-{role}"


def split_kind(kind: Any, stacks: frozenset[str] | set[str]) -> tuple[str | None, str | None]:
    """Decompose a stack-qualified composite back into `(stack, role)`.

    A stack-qualified value round-trips exactly: `product` + `dotnet` ->
    `dotnet-product` -> `("dotnet", "product")`. A *bare* value decomposes to
    `(None, word)`, which is the one case the composite cannot carry: when the
    stack is `none` the derived value is the bare role, so it cannot say
    whether the stack was `none` or simply unstated. Returns `(None, None)` for
    a value that is not a string.
    """
    if not isinstance(kind, str) or not kind:
        return (None, None)
    for stack in sorted(stacks):
        if kind == stack:
            return (stack, kind)
        if kind.startswith(f"{stack}-"):
            return (stack, kind[len(stack) + 1:])
    return (None, kind)


def secret_words_for(governance_root: Path | None = None) -> "re.Pattern[str]":
    import vocabulary

    return vocabulary.secret_pattern(_vocabulary_for(governance_root)["secret_field_substrings"])


def __getattr__(name: str) -> Any:
    if name == "SECRET_WORDS":
        return secret_words_for()
    if name == "PROFILES":
        return set(canonical_profiles_for())
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

# Capability and release-evidence vocabulary (additive, opt-in per project).
CAPABILITY_OWNERS = {"platform", "product", "shared", "external"}
CAPABILITY_EVIDENCE_STATES = {"declared", "configured", "verified", "unverified", "blocked"}
RELEASE_EVIDENCE_STATES = {"declared", "configured", "verified", "unverified"}
RELEASE_EVIDENCE_FIELDS = (
    "revision",
    "version",
    "toolchain",
    "artifacts",
    "digests",
    "sbom",
    "provenance",
    "checks",
    "publication",
)
DEPLOYABLE_RELEASE_REQUIRED_FIELDS = ("revision", "artifacts", "digests", "provenance", "checks", "publication")

# Capability adoption gate finding codes (see
# `openspec/changes/capability-adoption-gate/` and `docs/capabilities.md`).
# The measurement is derived from three sources — the capability registry,
# the consumer's own manifests, and the consumer's `consumes[]` record — so a
# repository cannot mark itself compliant by writing a declaration.
CAPABILITY_REGISTRY_INVALID = "CAPABILITY_REGISTRY_INVALID"
CAPABILITY_ADOPTION_MISSING = "CAPABILITY_ADOPTION_MISSING"
CAPABILITY_ADOPTION_UNEVIDENCED = "CAPABILITY_ADOPTION_UNEVIDENCED"
CAPABILITY_ADOPTION_EVIDENCE_STALE = "CAPABILITY_ADOPTION_EVIDENCE_STALE"
CAPABILITY_ADOPTION_DECLINE_UNRECORDED = "CAPABILITY_ADOPTION_DECLINE_UNRECORDED"
CAPABILITY_ADOPTION_UNRESOLVED = "CAPABILITY_ADOPTION_UNRESOLVED"

# Interface declaration and boundary findings (see
# `openspec/changes/interface-declaration-and-boundary-findings/` and
# `docs/capabilities.md`). A project MAY declare the boundaries it provides
# and consumes in an optional `interfaces[]` array; an absent array is valid
# and silent, and a declaration that fails validation yields
# `INTERFACE_DECLARATION_INVALID` and no interface assertions at all.
INTERFACE_DECLARATION_INVALID = "INTERFACE_DECLARATION_INVALID"
INTERFACE_MECHANISM_UNKNOWN = "INTERFACE_MECHANISM_UNKNOWN"
INTERFACE_CONTRACT_UNVERSIONED = "INTERFACE_CONTRACT_UNVERSIONED"
INTERFACE_PROVIDER_UNDECLARED = "INTERFACE_PROVIDER_UNDECLARED"
INTERFACE_CONSUMED_UNUSED = "INTERFACE_CONSUMED_UNUSED"
INTERFACE_SIBLING_PATH_UNPINNED = "INTERFACE_SIBLING_PATH_UNPINNED"
INTERFACE_VENDORED_WITHOUT_DIGEST = "INTERFACE_VENDORED_WITHOUT_DIGEST"
CAPABILITY_PROVIDER_UNVERIFIED = "CAPABILITY_PROVIDER_UNVERIFIED"

# The closed mechanism set. No mechanism becomes lawful merely because a
# tool accepts it: `sibling` is lawful only under the pinning rule below,
# and no absolute path is lawful under any mechanism.
INTERFACE_MECHANISMS = ("invoke", "consume", "observe", "sibling")
INTERFACE_CONTRACT = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._@/-]*/[0-9]+\.[0-9]+\.[0-9]+$")
INTERFACE_CONTRACT_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._@/-]*$")
INTERFACE_DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
INTERFACE_ENTRY_KEYS = frozenset(
    {
        "id",
        "provides",
        "consumes",
        "provider",
        "mechanism",
        "entry",
        "contract",
        "digest",
        "required",
        "evidence_ref",
    }
)

# The additive decision-register field that records an interface ramp. The
# register tolerates unknown entry-level keys, so a ramp is recorded as a
# decision carrying `{"finding_codes": [...], "effective": "YYYY-MM-DD"}`.
INTERFACE_ESCALATION_FIELD = "escalates"

# The two alternatives a `sibling` edge has when it is not pinned. Named in
# the remediation because "unpinned" is a diagnosis, not a next action.
SIBLING_LAWFUL_ALTERNATIVES = (
    "declare the copy as `mechanism: consume` with its contract and a sha256 "
    "digest, or publish the artifact and consume it by version"
)

# Postures live in `capabilities.json`. A capability is always admitted as
# `expected`; `required` additionally needs a recorded graduation entry.
CAPABILITY_POSTURES = ("exempt", "expected", "required")
CAPABILITY_RAMP_ORDER = {"exempt": 0, "expected": 1, "required": 2}
DEFAULT_GRADUATION_AUDITS = 3

# The closed decline vocabulary. Every member is a boundary argument, not a
# free-text escape hatch; `provider-unavailable` names a provider gap rather
# than a consumer decision.
DECLINE_RATIONALES = (
    "ownership",
    "privacy",
    "lifecycle",
    "dependency-direction",
    "naming-symmetry",
    "not-applicable",
    "provider-unavailable",
)

# Lexical triggers used only to *name* a prose boundary argument in a project's
# own documents. A match is never a decline on its own: it asks for a
# `declines[]` record via CAPABILITY_ADOPTION_DECLINE_UNRECORDED.
_PROSE_DECLINE_TRIGGERS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("ownership", ("owner", "ownership", "owns")),
    ("privacy", ("privacy", "pii", "data boundary", "tenant isolation")),
    ("lifecycle", ("lifecycle", "deprecat", "version cadence", "ship cadence")),
    ("dependency-direction", ("dependency direction", "dependency-direction", "invert", "keep the capability local")),
    ("naming-symmetry", ("naming symmetry", "naming-symmetry", "symmetry")),
    ("not-applicable", ("not applicable", "not-applicable", "does not apply", "no requirement")),
    ("provider-unavailable", ("provider-unavailable", "provider unavailable", "not published", "no published")),
)

# Cross-repo adoption grant finding codes (see
# `openspec/changes/cross-repo-adoption-grant/` and `docs/adoption-grant.md`).
# A separate family from `CAPABILITY_ADOPTION_*` on purpose: the two changes
# are independent and must not be coupled by naming.

# Identity and taxonomy finding codes (see
# `openspec/changes/identity-and-taxonomy-axis-split/` and
# `docs/vocabulary.md`). `role` and `stack` are two independent axes, `id` is
# the immutable join key, and `kind` survives only as a derived, deprecated
# composite so an undocumented consumer of the old string keeps parsing.
TAXONOMY_AXIS_UNKNOWN = "TAXONOMY_AXIS_UNKNOWN"
TAXONOMY_MISMATCH = "TAXONOMY_MISMATCH"
TAXONOMY_LEGACY_VALUE = "TAXONOMY_LEGACY_VALUE"

# Ruling 2 (`design.md` §10), binding: TAXONOMY_MISMATCH is ERROR, and the
# taxonomy audit MUST NOT be enabled until the §5 migration table has been
# applied. The level is therefore fixed at ERROR below and is never softened
# to WARNING — the migration and the enabled check ship in the same commit.
# The flag below is the sequencing hold, not an opt-out: it is a committed,
# repo-relative constant, never an environment variable, and flipping it to
# True before the migration cannot pass quietly, because every unresolved row
# then reports ERROR and turns the gate red. `scripts/axis_migration.py` is
# what applies the migration that releases the flag.
TAXONOMY_MISMATCH_LEVEL = "ERROR"
TAXONOMY_AUDIT_ENABLED = True

# README status-claim finding codes (see
# `openspec/changes/readme-claim-verification/` and
# `docs/product-code-quality.md`). A separate family from `QUALITY_*`'s existing
# members and from the `TAXONOMY_*` codes, and additive: no existing code is
# renamed, narrowed, or reused.
#
# `QUALITY_README_CLAIM_CONTRADICTED` is ERROR, one finding per contradicted
# surface. Ruling 6 (`design.md` §10): a warning nobody must clear is the
# mechanism by which the seven stale claims accumulated in the first place, so
# the level is not negotiable while they stand.
#
# `QUALITY_README_CLAIM_UNSCOPED` is NOT a per-repository finding. It is emitted
# exactly once per run as a fleet-level CHECKER-COVERAGE metric. Ruling 7: it
# fires on `chronicleaf`, whose claim is accurate, and a per-repository finding
# on an honest repository is a false positive that trains people to ignore the
# checker. It carries `severity: REVIEW_REQUIRED` in the checker's own
# vocabulary and is never a pass.
QUALITY_README_CLAIM_CONTRADICTED = "QUALITY_README_CLAIM_CONTRADICTED"
QUALITY_README_CLAIM_UNSCOPED = "QUALITY_README_CLAIM_UNSCOPED"
README_CLAIMS_UNSUPPORTED_FIELD = "readme_claims"

# Archived-change Purpose marker finding code (see
# `openspec/changes/spec-purpose-backfill-gate/` and `docs/vocabulary.md`).
# A separate family from `QUALITY_*` and from `TAXONOMY_*`: this marker is a
# scaffold defect in a specification's own prose, and it is neither a
# classification nor a README claim.
#
# Archiving a change writes `TBD - created by archiving change <name>.` into the
# `## Purpose` of every spec the change produced. The archiver is third-party, so
# the defect is held here instead: `scripts/spec_purpose_backfill.py` resolves
# each one mechanically and records which source it used.
#
# The bare word for this defect is itself one of this repository's own
# `placeholder_markers`, so it never appears as prose in `scripts/`, and the
# change id is named without it. The marker is called the archiver's `TBD` marker
# here; the domain term is kept in `tests/`, `docs/`, and the change, which are
# outside `quality.product_roots`.
SPEC_PURPOSE_PLACEHOLDER = "SPEC_PURPOSE_PLACEHOLDER"

# Both generations the archiver has emitted are the same defect and are matched
# by one pattern. The marker must be the whole *section* body, not merely a line
# that starts with it, so a real sentence appended after the marker is real prose
# and never a finding; that is why the change name is bound to `[\w.-]+` — a
# change name has no spaces, so trailing prose can never be absorbed into it,
# which a bare `[^\n]*` would happily do. `(?:[ \t]*\n)*` after the heading
# tolerates the blank line 7 files in the fleet carry between the heading and the
# marker. Measured: the section-strict form matches the same 1,234 files
# fleet-wide as the looser line form and no others.
#
# This pattern MUST stay identical to `PLACEHOLDER_RE` in
# `scripts/spec_purpose_backfill.py`: a marker the backfill resolves but this rule
# cannot see is a spec that migrates and then still reports ERROR.
SPEC_PURPOSE_PLACEHOLDER_RE = re.compile(
    r"^##[ \t]+Purpose[ \t]*\n(?:[ \t]*\n)*"
    r"(TBD - created by archiving change [\w.-]+?\.(?:[ \t]+Update Purpose after archive\.)?)[ \t]*\n"
    r"(?:[ \t]*\n)*(?=^#{1,6}[ \t]|\Z)",
    re.MULTILINE,
)
# The change name is read out of the marker line for the message only. The
# `Update Purpose after archive.` sentence the current generation appends is
# optional, and the archiver has emitted a date-prefixed change name as well as a
# bare one, so neither spelling is assumed here.
SPEC_PURPOSE_CHANGE_RE = re.compile(
    r"^TBD - created by archiving change (?P<name>[\w.-]+?)\.(?:[ \t]+Update Purpose after archive\.)?$"
)
SPEC_PURPOSE_ARCHIVE_DIR = "openspec/changes/archive"
SPEC_PURPOSE_SPECS_DIR = "openspec/specs"

# Ruling 1 (`design.md` §10), binding: SPEC_PURPOSE_PLACEHOLDER is ERROR, and the
# spec-purpose audit MUST NOT be enabled until the backfill has been applied.
# Measured on 2026-10-03: 1,234 spec files in 71 of the 79 repositories carrying
# an `openspec/specs` tree still hold the marker, so enabling the rule first
# would turn the audit red for everyone with no recorded owner action. The level
# is therefore fixed at ERROR below and is never softened to WARNING to achieve
# the hold.
#
# This is the taxonomy rule's fence, exactly, and the only one for this rule: a
# committed, repo-relative constant, never an environment variable, no skip flag
# and no exemption registry. Flipping it to True before the backfill cannot pass
# quietly, because every unresolved spec then reports ERROR and turns the gate
# red.
#
# Unlike the taxonomy fence there is a second, external precondition, which is
# not a second mechanism but a missing wire: the `governance-quality` concern in
# `driftwatchdog` filters its verdict to `GOVERNANCE_QUALITY_DECLARED_CODES`
# (`src/gate/governance/mod.rs:73-78`, filter at `:453`) and treats the checker's
# exit 1 as an expected exit (`:84`), so with this code outside that set the
# concern would report PASS while the checker fails. `design.md` §6 records the
# exact change that releases it. The flag is the sequencing hold; it is not
# sufficient on its own to make this rule gate anything.
SPEC_PURPOSE_PLACEHOLDER_LEVEL = "ERROR"
SPEC_PURPOSE_AUDIT_ENABLED = True

# A declared verification command that no Gate runs (see
# `driftwatchdog/openspec/changes/bind-declared-verification-command/`).
#
# Ruling D18: a verifier that no gate runs does not gate. Measured on
# 2026-10-04, all 81 sibling repositories declare a `verification.command` in
# their `.project.json` and 6 bind a command into `.ai-gate/gate.yaml`, leaving
# 70 whose gate executed nothing. The existing `VERIFICATION_COMMAND` rule
# checks the opposite direction — that a command is *present* — so it is
# satisfied by all 81 and catches none of this. This rule closes the gap: the
# declaration must reach a blocking gate.
#
# This is the taxonomy rule's fence pattern exactly, and the only one for this
# rule: a committed, repo-relative constant, never an environment variable, no
# skip flag, no exemption registry. The level is fixed at ERROR below and is
# never softened to WARNING to achieve the hold — softening the level would
# hide the rule in a second, dishonest way.
#
# Shipped `False`. The alternative, `True`, reports ERROR for every repository
# that declares a command and binds none: 70 today, of which 4 are excluded
# because their manifest is dirty and 11 cannot parse at all because they carry
# an unknown top-level field, so `True` right now is red for reasons this rule
# does not own. Flipping the constant is the owner's single decision; the fence
# test proves that flipping it changes the result.
#
# Like the taxonomy fence, this flag alone does not make the rule gate anything.
# The `governance-quality` concern in `driftwatchdog` filters its verdict to
# `GOVERNANCE_QUALITY_DECLARED_CODES`, so the code below must be in that
# allow-list or the concern reports PASS while this checker exits 1. That
# declaration ships in the same change.
QUALITY_VERIFICATION_COMMAND_UNBOUND = "QUALITY_VERIFICATION_COMMAND_UNBOUND"
QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL = "ERROR"
VERIFICATION_BINDING_AUDIT_ENABLED = False

# The concern id `scripts/bind_verification_command.py` writes into
# `.ai-gate/gate.yaml`, and the concern id that must also be selected in
# `checks:` for the binding to run at all. The rule and the generator must agree
# on these names or the generator binds something the rule calls unbound.
GATE_MANIFEST_RELATIVE = ".ai-gate/gate.yaml"
DECLARED_VERIFICATION_CONCERN = "declared-verification"

# A repository carrying unresolved gate debt may not open a new OpenSpec
# change, so the debt is paid as work starts instead of in a cleanup push that
# never finishes. The rule ships with **no fence**, unlike the three above: it is
# supposed to be red while debt is open, and a constant that turns it off is the
# switch a blocking rule must not have. Its exit is a recorded decision in
# `gate-debt-exceptions.json`, not a flag.
#
# Like every code here, this one gates nothing unless `driftwatchdog`'s
# `governance-quality` concern declares it in `GOVERNANCE_QUALITY_DECLARED_CODES`.
# An undeclared code is filtered out of the verdict, so the concern would report
# PASS while this checker exits 1.
QUALITY_NEW_CHANGE_WITH_GATE_DEBT = "QUALITY_NEW_CHANGE_WITH_GATE_DEBT"
QUALITY_NEW_CHANGE_WITH_GATE_DEBT_LEVEL = "ERROR"
# A satisfied precondition is still reported. `WARNING` is the only other level
# the concern defines, and only `ERROR` moves the exit code, so a recorded
# exception is visible in the run without blocking it.
QUALITY_NEW_CHANGE_WITH_GATE_DEBT_SATISFIED_LEVEL = "WARNING"

# The changes that close the debt, and are therefore never themselves a
# trigger. A repository that may not start the work which fixes its debt can
# never fix it, and the rule would be a dead end.
GATE_DEBT_CLOSING_CHANGES = frozenset({"close-unbound-gate-checks"})


def taxonomy_audit_enabled(explicit: bool | None = None) -> bool:
    """Resolve the ruling-2 sequencing gate.

    `None` (every production call path) reads the committed constant; tests
    pass an explicit value to exercise both states of the gate.
    """
    return TAXONOMY_AUDIT_ENABLED if explicit is None else explicit


def spec_purpose_audit_enabled(explicit: bool | None = None) -> bool:
    """Resolve the ruling-1 sequencing gate for the archived Purpose marker.

    `None` (every production call path) reads the committed constant; tests pass
    an explicit value to exercise both states of the gate.
    """
    return SPEC_PURPOSE_AUDIT_ENABLED if explicit is None else explicit


def verification_binding_audit_enabled(explicit: bool | None = None) -> bool:
    """Resolve the sequencing gate for an unbound declared verification command.

    `None` (every production call path) reads the committed constant; tests pass
    an explicit value to exercise both states of the gate. Same shape and same
    rationale as the other two fences, so one mechanism governs all three.
    """
    return VERIFICATION_BINDING_AUDIT_ENABLED if explicit is None else explicit


@dataclass
class Finding:
    level: str
    code: str
    message: str
    project: str | None = None


def finding(findings: list[Finding], level: str, code: str, message: str, project: str | None = None) -> None:
    findings.append(Finding(level, code, message, project))


PROJECT_MARKERS = (
    ".project.json",
    ".git",
    "README.md",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "pubspec.yaml",
    "go.mod",
    "pom.xml",
    "Dockerfile",
    "docker-compose.yml",
    "compose.yml",
    "Jenkinsfile",
)


def discover_projects(workspace_root: Path, registry: dict[str, Any]) -> list[dict[str, str]]:
    """Discover immediate child projects without persisting a machine path."""
    discovery = registry.get("discovery") or {}
    mode = discovery.get("mode", "explicit")
    if mode not in {"immediate-directories", "explicit-plus-immediate-directories"}:
        return []
    excluded = {str(item) for item in discovery.get("exclude", []) if isinstance(item, str)}
    projects: list[dict[str, str]] = []
    try:
        children = sorted(workspace_root.iterdir(), key=lambda item: item.name)
    except OSError:
        return projects
    for child in children:
        if child.name.startswith(".") or child.name in excluded or not child.is_dir() or child.is_symlink():
            continue
        if not any((child / marker).exists() for marker in PROJECT_MARKERS):
            continue
        projects.append({"id": child.name, "path": child.relative_to(workspace_root).as_posix()})
    return projects


def load_json(path: Path, findings: list[Finding], code: str) -> dict[str, Any] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        finding(findings, "ERROR", code, f"missing JSON file: {path}")
        return None
    except (OSError, json.JSONDecodeError) as exc:
        finding(findings, "ERROR", code, f"invalid JSON {path}: {exc}")
        return None
    if not isinstance(value, dict):
        finding(findings, "ERROR", code, f"JSON object required: {path}")
        return None
    return value


def load_decisions_register(
    governance_root: Path,
    registry: dict[str, Any],
    findings: list[Finding],
    *,
    self_check: bool,
) -> "list[object] | None":
    """Load `decisions.json` from the governance root and surface findings.

    The register is governance-owned and is required for the governance
    self-check (ERROR `DECISIONS_REGISTER_MISSING`). A portfolio audit on
    sibling repos tolerates the absence of the register (single WARNING
    `DECISIONS_REGISTER_ABSENT`); broken shapes and unknown registry
    references are always errors. Returns the parsed decisions on success,
    `None` on any failure path (the caller does not need the value).

    The `WORKSPACE_DECISIONS_PATH` env var overrides the default location
    so tests can stage a per-workspace register without touching the
    governance root file. Production callers never set it.
    """
    import decisions as decisions_module

    override = os.environ.get("WORKSPACE_DECISIONS_PATH")
    if override:
        register_path = Path(override)
    else:
        register_path = decisions_module.decisions_path(governance_root)
    if not register_path.is_file():
        if self_check:
            finding(findings, "ERROR", DECISIONS_REGISTER_MISSING,
                    f"decisions register missing at {register_path}")
        else:
            finding(findings, "WARNING", DECISIONS_REGISTER_ABSENT,
                    f"decisions register absent at {register_path}; "
                    "self-check mode requires it")
        return None
    try:
        loaded = decisions_module.load_decisions(register_path)
    except decisions_module.DecisionError as exc:
        finding(findings, "ERROR", DECISIONS_REGISTER_INVALID, str(exc))
        return None
    registry_ids = {
        entry.get("id")
        for entry in (registry.get("projects") or [])
        if isinstance(entry, dict) and isinstance(entry.get("id"), str)
    }
    offenders = decisions_module.validate_against_registry(loaded, registry_ids)
    if offenders:
        finding(
            findings,
            "ERROR",
            DECISIONS_REGISTRY_REFERENCE,
            f"decisions register references unknown registry ids: {offenders}",
        )
        return None
    return loaded


def validate_registry(registry: dict[str, Any], workspace_root: Path, findings: list[Finding], check_paths: bool = True, *, canonical_profiles: set[str] | frozenset[str] | None = None, governance_root: Path | None = None, taxonomy_enabled: bool | None = None) -> list[dict[str, Any]]:
    if registry.get("schema_version") != 1:
        finding(findings, "ERROR", "REGISTRY_SCHEMA", "registry schema_version must be 1")
    projects = registry.get("projects")
    if not isinstance(projects, list):
        finding(findings, "ERROR", "REGISTRY_PROJECTS", "registry projects must be an array")
        return []
    seen: set[str] = set()
    seen_paths: set[str] = set()
    for entry in projects:
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "REGISTRY_ENTRY", "each registry project must be an object")
            continue
        project_id = entry.get("id")
        path_value = entry.get("path")
        if not isinstance(project_id, str) or not PROJECT_ID.fullmatch(project_id):
            finding(findings, "ERROR", "PROJECT_ID", f"invalid project id: {project_id!r}")
            continue
        if project_id in seen:
            finding(findings, "ERROR", "PROJECT_DUPLICATE", f"duplicate project id: {project_id}", project_id)
        seen.add(project_id)
        if not isinstance(path_value, str) or Path(path_value).is_absolute() or ".." in Path(path_value).parts:
            finding(findings, "ERROR", "PROJECT_PATH", f"project path must be repository-relative: {path_value!r}", project_id)
        else:
            if path_value in seen_paths:
                finding(findings, "ERROR", "PROJECT_PATH_DUPLICATE", f"duplicate project path: {path_value}", project_id)
            seen_paths.add(path_value)
            if check_paths:
                project_path = workspace_root / path_value
                if not project_path.is_dir():
                    finding(findings, "ERROR", "PROJECT_MISSING", f"registered project directory missing: {project_path}", project_id)
        profile = entry.get("profile")
        if not isinstance(profile, str) or not profile:
            finding(findings, "ERROR", "PROFILE_MISSING", "project profile is required", project_id)
        else:
            profiles = canonical_profiles if canonical_profiles is not None else canonical_profiles_for(governance_root)
            if profile not in profiles:
                finding(findings, "ERROR", "PROFILE_UNKNOWN", f"unknown profile: {profile}; canonical profiles: {sorted(profiles)}", project_id)
        lifecycle = entry.get("lifecycle")
        if lifecycle not in LIFECYCLES:
            finding(findings, "ERROR", "LIFECYCLE", f"invalid lifecycle: {lifecycle!r}", project_id)
        if taxonomy_audit_enabled(taxonomy_enabled):
            # Loaded here rather than up front: while the gate is closed the
            # audit must not depend on the axis sets at all.
            validate_registry_taxonomy(entry, findings, project_id, governance_root=governance_root)
    return projects


def profile_requirements(profile: str) -> tuple[list[str], bool, bool, bool]:
    if profile == "inventory":
        return [".project.json"], False, False, False
    if profile == "tooling":
        return ["README.md", ".project.json"], False, False, True
    if profile == "deployment":
        return ["README.md", ".project.json"], False, False, True
    if profile == "dotnet-template":
        return ["README.md", "AGENTS.md", ".project.json"], True, False, True
    if profile == "governance":
        return ["README.md", "ROADMAP.md", "HANDOFF.md", "AGENTS.md", ".ai-rules/workflow.md", ".ai-rules/completion.md", ".project.json"], True, False, True
    return ["README.md", "ROADMAP.md", "HANDOFF.md", "AGENTS.md", ".ai-rules/workflow.md", ".ai-rules/completion.md", ".project.json"], True, True, True


def load_profile_spec(governance_root: Path, profile: str) -> dict[str, Any] | None:
    """Load the profile JSON next to the governance checker, if present.

    Profiles are documentation by default. The checker only reads them to
    surface a soft warning when a project declares a capability its profile
    does not list as supported.
    """
    path = governance_root / "profiles" / f"{profile}.json"
    if not path.is_file():
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(value, dict):
        return None
    return value


def validate_profile_docs(governance_root: Path, findings: list[Finding], *, canonical_profiles: set[str] | frozenset[str] | None = None) -> None:
    """A `profiles/<name>.json` document naming a non-canonical profile is ERROR `PROFILE_DOC_UNKNOWN`."""
    profiles = canonical_profiles if canonical_profiles is not None else canonical_profiles_for(governance_root)
    profiles_dir = Path(governance_root) / "profiles"
    if not profiles_dir.is_dir():
        return
    for child in sorted(profiles_dir.glob("*.json")):
        try:
            value = json.loads(child.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            finding(findings, "ERROR", "PROFILE_DOC_UNKNOWN", f"cannot parse profile document {child.name}: {exc}")
            continue
        if not isinstance(value, dict):
            finding(findings, "ERROR", "PROFILE_DOC_UNKNOWN", f"profile document must contain a JSON object: {child.name}")
            continue
        documented = value.get("id", child.stem)
        if child.stem not in profiles or documented not in profiles:
            finding(findings, "ERROR", "PROFILE_DOC_UNKNOWN", f"profile document {child.name} names a non-canonical profile: {documented!r}; canonical profiles: {sorted(profiles)}")


def _is_self_consistent_derived_kind(
    declaration: dict[str, Any],
    kind: str,
    *,
    canonical_roles: set[str] | frozenset[str],
    canonical_stacks: set[str] | frozenset[str],
) -> bool:
    """Is `kind` the canonical form derived from this declaration's own axes?

    True only when both axes are present, both are canonical, and `kind`
    equals `derive_kind(role, stack)`. `kind` is a *derived, deprecated*
    composite, so its canonical form is whatever the two axes derive — and the
    derivation is a product of two enumerated sets, so it is not enumerable as
    a hand-maintained list. Absent or non-canonical axes return False: nothing
    has been derived yet, so the legacy form is not measurable.
    """
    role = declaration.get("role")
    stack = declaration.get("stack")
    if not isinstance(role, str) or not isinstance(stack, str):
        return False
    if role not in canonical_roles or stack not in canonical_stacks:
        return False
    return kind == derive_kind(role, stack)


def validate_kind(declaration: dict[str, Any], findings: list[Finding], project_id: str, *, canonical_kinds: set[str] | frozenset[str] | None = None, governance_root: Path | None = None, canonical_roles: set[str] | frozenset[str] | None = None, canonical_stacks: set[str] | frozenset[str] | None = None) -> None:
    """A declaration whose `kind` is neither canonical nor self-consistent is WARNING `KIND_UNKNOWN`.

    A `kind` outside the legacy `kinds` vocabulary is known when it is the
    value its own two axes derive, because the field is derived and deprecated
    and the axes are its authority. `kinds` is the frozen legacy vocabulary, not
    the authority on derived values, so a canonical derivation is not held to
    it; everything else — a hand-authored legacy value that disagrees with its
    own derivation, a one-off value in neither form, and any value whose axes
    are absent or non-canonical — still reports.

    Kind is descriptive and never drives profile requirements.
    """
    kinds = canonical_kinds if canonical_kinds is not None else canonical_kinds_for(governance_root)
    kind = declaration.get("kind")
    if isinstance(kind, str) and kind in kinds:
        return
    if isinstance(kind, str):
        # Only the value that would otherwise be reported consults the axes, so
        # a canonical `kind` keeps resolving with no vocabulary read at all.
        roles = canonical_roles if canonical_roles is not None else canonical_roles_for(governance_root)
        stacks = canonical_stacks if canonical_stacks is not None else canonical_stacks_for(governance_root)
        if _is_self_consistent_derived_kind(declaration, kind, canonical_roles=roles, canonical_stacks=stacks):
            return
    finding(findings, "WARNING", "KIND_UNKNOWN", f"unknown kind: {kind!r}; canonical kinds: {sorted(kinds)}", project_id)


def _resolve_axes(
    surface: dict[str, Any],
    findings: list[Finding],
    project_id: str,
    *,
    canonical_roles: set[str] | frozenset[str],
    canonical_stacks: set[str] | frozenset[str],
    label: str,
) -> tuple[str | None, str | None, bool]:
    """Read one authority surface's two axes.

    Returns `(role, stack, both_canonical)`. An absent or non-canonical value
    is WARNING `TAXONOMY_AXIS_UNKNOWN` naming the offending value and both
    canonical sets; the two axes are independent, so one bad value does not
    suppress the report on the other.
    """
    axes: dict[str, str | None] = {}
    canonical = True
    for axis, values in (("role", canonical_roles), ("stack", canonical_stacks)):
        value = surface.get(axis)
        if value is None:
            finding(findings, "WARNING", TAXONOMY_AXIS_UNKNOWN,
                    f"{label} does not declare a canonical {axis}; canonical {axis} values: {sorted(values)}", project_id)
            axes[axis] = None
            canonical = False
            continue
        if not isinstance(value, str) or value not in values:
            finding(findings, "WARNING", TAXONOMY_AXIS_UNKNOWN,
                    f"{label} {axis} is not canonical: {value!r}; canonical roles: {sorted(canonical_roles)}; "
                    f"canonical stacks: {sorted(canonical_stacks)}", project_id)
            axes[axis] = None
            canonical = False
            continue
        axes[axis] = value
    return axes["role"], axes["stack"], canonical


def validate_registry_taxonomy(
    entry: dict[str, Any],
    findings: list[Finding],
    project_id: str,
    *,
    canonical_roles: set[str] | frozenset[str] | None = None,
    canonical_stacks: set[str] | frozenset[str] | None = None,
    governance_root: Path | None = None,
) -> None:
    """A registry entry's `role`/`stack` must be canonical when present.

    A registry entry may legitimately omit the axes while the fleet migration
    is in flight, so an absent value is still reported as WARNING
    `TAXONOMY_AXIS_UNKNOWN` on the surface that omits it. Registry
    `id`/`path` handling is untouched: an `id` that differs in spelling from
    its `path` is resolved through `path` and is never a finding.
    """
    roles = canonical_roles if canonical_roles is not None else canonical_roles_for(governance_root)
    stacks = canonical_stacks if canonical_stacks is not None else canonical_stacks_for(governance_root)
    _resolve_axes(entry, findings, project_id, canonical_roles=roles, canonical_stacks=stacks, label="registry entry")


def validate_taxonomy(
    entry: dict[str, Any],
    declaration: dict[str, Any],
    findings: list[Finding],
    project_id: str,
    *,
    canonical_roles: set[str] | frozenset[str] | None = None,
    canonical_stacks: set[str] | frozenset[str] | None = None,
    governance_root: Path | None = None,
) -> None:
    """Audit the two classification axes across both authority surfaces.

    - an absent or non-canonical value on either surface is WARNING
      `TAXONOMY_AXIS_UNKNOWN` naming the value and both canonical sets;
    - a disagreement between the two surfaces is ERROR `TAXONOMY_MISMATCH`
      (level fixed by ruling 2) naming both values and both keys;
    - an axis present on only one surface is an absent value, not a
      contradiction, so it reports `TAXONOMY_AXIS_UNKNOWN` and no mismatch;
    - a `kind` that disagrees with the value derived from its own axes, and a
      stack-qualified composite still present while both axes are declared, are
      WARNING `TAXONOMY_LEGACY_VALUE`.

    The axes are descriptive: nothing here reads a required file, a CI or
    verification requirement, or an evidence, deployment, or promotion state.
    """
    roles = canonical_roles if canonical_roles is not None else canonical_roles_for(governance_root)
    stacks = canonical_stacks if canonical_stacks is not None else canonical_stacks_for(governance_root)
    declaration_role, declaration_stack, declaration_canonical = _resolve_axes(
        declaration, findings, project_id, canonical_roles=roles, canonical_stacks=stacks, label="declaration")
    registry_role, registry_stack, _ = _resolve_axes(
        entry, findings, project_id, canonical_roles=roles, canonical_stacks=stacks, label="registry entry")

    if declaration_canonical and registry_role is not None and declaration_role != registry_role:
        finding(findings, TAXONOMY_MISMATCH_LEVEL, TAXONOMY_MISMATCH,
                f"role disagreement: registry entry {registry_role!r} against declaration {declaration_role!r}", project_id)
    if declaration_canonical and registry_stack is not None and declaration_stack != registry_stack:
        finding(findings, TAXONOMY_MISMATCH_LEVEL, TAXONOMY_MISMATCH,
                f"stack disagreement: registry entry {registry_stack!r} against declaration {declaration_stack!r}", project_id)

    if not declaration_canonical:
        # Nothing has been derived yet, so the legacy composite is not yet
        # measurable: an absent axis must not also report TAXONOMY_LEGACY_VALUE.
        return
    derived = derive_kind(declaration_role or "", declaration_stack or "")
    kind = declaration.get("kind")
    if isinstance(kind, str) and kind and kind != derived:
        finding(findings, "WARNING", TAXONOMY_LEGACY_VALUE,
                f"kind {kind!r} is a hand-authored composite that disagrees with the derived value {derived!r} "
                f"(derived as stack-role, or the bare role when stack is 'none')", project_id)


def validate_capabilities(capabilities: Any, project_path: Path, findings: list[Finding], project_id: str, supported: set[str] | None = None) -> None:
    """Audit a project's optional capability declarations.

    Declarations describe intent, not implementation. Governance records the
    declared state and the presence/absence of native evidence; it never
    promotes a capability to verified on its own.

    When `supported` is provided, capabilities not listed there are surfaced
    as a WARNING so profile owners can keep the supported set intentional.
    """
    if not isinstance(capabilities, dict):
        finding(findings, "ERROR", "CAPABILITIES_SHAPE", "capabilities must be an object", project_id)
        return
    for name, entry in capabilities.items():
        if not isinstance(name, str) or not PROJECT_ID.fullmatch(name):
            finding(findings, "ERROR", "CAPABILITY_NAME", f"invalid capability name: {name!r}", project_id)
            continue
        if supported is not None and name not in supported:
            finding(findings, "WARNING", "CAPABILITY_UNSUPPORTED", f"capability {name!r} is not declared as supported by the project profile", project_id)
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "CAPABILITY_SHAPE", f"capability {name!r} must be an object", project_id)
            continue
        owner = entry.get("owner")
        if not isinstance(owner, str) or not owner:
            finding(findings, "ERROR", "CAPABILITY_OWNER", f"capability {name!r} requires a non-empty owner", project_id)
        elif owner not in CAPABILITY_OWNERS and not PROJECT_ID.fullmatch(owner):
            finding(findings, "WARNING", "CAPABILITY_OWNER", f"capability {name!r} owner is not a known platform, product, or shared id: {owner!r}", project_id)
        state = entry.get("evidence_state")
        if state not in CAPABILITY_EVIDENCE_STATES:
            finding(findings, "ERROR", "CAPABILITY_STATE", f"capability {name!r} has invalid evidence_state: {state!r}", project_id)
            continue
        ref = entry.get("evidence_ref")
        if ref is not None and not isinstance(ref, str):
            finding(findings, "ERROR", "CAPABILITY_REF", f"capability {name!r} evidence_ref must be a string when present", project_id)
            ref = None
        if state in {"configured", "verified"}:
            if not isinstance(ref, str) or not ref:
                finding(findings, "WARNING", "CAPABILITY_PENDING", f"capability {name!r} is {state} but has no evidence_ref", project_id)
            elif (project_path / ref).is_file():
                continue
            else:
                level = "ERROR" if state == "verified" else "WARNING"
                code = "CAPABILITY_STALE" if state == "verified" else "CAPABILITY_PENDING"
                finding(findings, level, code, f"capability {name!r} evidence_ref missing: {ref}", project_id)
        elif state == "unverified":
            finding(findings, "WARNING", "CAPABILITY_PENDING", f"capability {name!r} is unverified; native evidence required", project_id)
        elif state == "blocked":
            finding(findings, "WARNING", "CAPABILITY_BLOCKED", f"capability {name!r} is declared blocked; verify the non-goal is recorded", project_id)


# ---------------------------------------------------------------------------
# Capability adoption gate
#
# Measurement is a three-way join and all three sides must agree:
#   1. the registry (`capabilities.json`) names exactly one provider,
#   2. the consumer's own manifests carry a pinned coordinate for the package,
#   3. the consumer's `.project.json` records the pairing in `consumes[]`.
# Everything below is read-only. No project command is executed, no package
# manager is invoked, no lockfile or transitive graph is resolved, and no
# network request is made. Nothing here promotes a capability to `verified`.
# ---------------------------------------------------------------------------


@dataclass
class CapabilityRegistry:
    """The loaded capability registry. Every capability id binds one provider."""

    root: Path
    capabilities: dict[str, Any]
    graduations: list[dict[str, Any]]
    graduation_audits: int


def capability_registry_path(governance_root: Path | None = None) -> Path:
    """Return the capability registry path.

    `WORKSPACE_CAPABILITIES_PATH` mirrors the `WORKSPACE_DECISIONS_PATH` hook
    so tests can isolate the audit from the governance root's real registry.
    """
    override = os.environ.get("WORKSPACE_CAPABILITIES_PATH")
    if override:
        return Path(override)
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    return root / "capabilities.json"


def load_capability_registry(governance_root: Path | None, findings: list[Finding]) -> CapabilityRegistry | None:
    """Load `capabilities.json`, failing closed on a missing or empty registry.

    There is deliberately no empty-registry fallback: a check that degrades to
    "nothing to enforce" would silently restore the invisibility this change
    exists to remove. A missing, unparseable, or empty registry is one
    governance ERROR against the registry itself, not one error per project.
    """
    path = capability_registry_path(governance_root)
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability registry cannot be read at {path}: {exc}")
        return None
    except json.JSONDecodeError as exc:
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability registry is not valid JSON: {exc}")
        return None
    if not isinstance(payload, dict):
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, "capability registry must be a JSON object")
        return None
    capabilities = payload.get("capabilities")
    if not isinstance(capabilities, dict) or not capabilities:
        finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, "capability registry declares no capabilities; refusing to enforce nothing")
        return None
    graduations = payload.get("graduations")
    if not isinstance(graduations, list):
        graduations = []
    audits = payload.get("graduation_audits", DEFAULT_GRADUATION_AUDITS)
    if not isinstance(audits, int) or isinstance(audits, bool) or audits < 1:
        audits = DEFAULT_GRADUATION_AUDITS
    for name, entry in capabilities.items():
        if not isinstance(name, str) or not PROJECT_ID.fullmatch(name):
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"invalid capability id in registry: {name!r}")
            return None
        if not isinstance(entry, dict):
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"registry entry for {name!r} must be an object")
            return None
        posture = entry.get("posture")
        if posture not in CAPABILITY_POSTURES:
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability {name!r} has invalid posture: {posture!r}")
            return None
        if posture == "exempt" and not str(entry.get("reason") or "").strip():
            finding(findings, "ERROR", CAPABILITY_REGISTRY_INVALID, f"capability {name!r} is exempt and requires a recorded reason")
            return None
    return CapabilityRegistry(path.parent, dict(capabilities), graduations, audits)


def _resolved_provider(entry: dict[str, Any]) -> str | None:
    """Return the single provider a capability binds to, or None when ambiguous.

    A capability id that is bound to more than one provider, or to none, is not
    a consumer's fault to fix: it is reported as
    `CAPABILITY_ADOPTION_UNRESOLVED` and never escalates.
    """
    provider = entry.get("provider")
    if isinstance(provider, str) and provider.strip():
        return provider.strip()
    if isinstance(provider, list):
        unique = {item.strip() for item in provider if isinstance(item, str) and item.strip()}
        if len(unique) == 1:
            return unique.pop()
    return None


def _graduation_for(capability_id: str, graduations: list[dict[str, Any]]) -> dict[str, Any] | None:
    """Return the latest appended graduation entry for a capability, if any."""
    matches = [
        item
        for item in graduations
        if isinstance(item, dict) and item.get("capability") == capability_id and item.get("to") == "required"
    ]
    return matches[-1] if matches else None


def effective_capability_posture(
    capability_id: str,
    entry: dict[str, Any],
    registry: CapabilityRegistry,
    provider_verified: bool,
) -> tuple[str, str]:
    """Resolve the posture an adoption finding is reported at.

    Returns `(posture, reason)`. `posture` is one of `exempt`, `expected`, or
    `required`.

    A capability may only be *reported* as `required` when every one of the
    three graduation conditions holds:

    1. the provider's capability is `verified` with a resolving evidence ref;
    2. a recorded consecutive-audit count has reached `graduation_audits`;
    3. a graduation entry was appended to the registry ledger.

    Wall-clock time is never a substitute. A capability whose posture says
    `required` without a valid graduation entry is *demoted back* to
    `expected` and no error is emitted for the entry itself, so lowering a
    counter or editing a date cannot manufacture a fleet error.
    """
    declared = entry.get("posture")
    if declared == "exempt":
        return "exempt", str(entry.get("reason") or "exempt")
    if declared == "expected":
        return "expected", "admitted as a warning"
    if not provider_verified:
        return "expected", "posture required but the provider capability is not verified"
    graduation = _graduation_for(capability_id, registry.graduations)
    if graduation is None:
        return "expected", "posture required but no graduation entry is recorded"
    if graduation.get("from") != "expected" or graduation.get("to") != "required":
        return "expected", "graduation entry does not record an expected -> required transition"
    recorded = graduation.get("audit_count")
    if not isinstance(recorded, int) or isinstance(recorded, bool) or recorded < registry.graduation_audits:
        return "expected", f"graduation entry records {recorded!r} consecutive audits, below graduation_audits={registry.graduation_audits}"
    evidence = graduation.get("provider_evidence")
    if not isinstance(evidence, dict) or not str(evidence.get("evidence_ref") or "").strip():
        return "expected", "graduation entry records no provider evidence"
    return "required", "graduated on a recorded transition"


# --- bounded, read-only manifest reading -----------------------------------
#
# Only the dependency stanza of a bounded list of manifest names is parsed.
# Lockfiles, transitive graphs, and registry queries are never read: resolving
# them needs a network or a package manager, both of which the audit is
# forbidden to use.

_MANIFEST_SCAN_DEPTH = 3
_MANIFEST_SCAN_FILE_BUDGET = 200
_MANIFEST_SKIP_DIRS = frozenset(
    {".git", "bin", "obj", "node_modules", "target", "vendor", "dist", "build", ".venv", "venv", "packages"}
)
_MANIFEST_EXACT_NAMES = ("Directory.Packages.props", "package.json", "Cargo.toml", "pyproject.toml")
_MANIFEST_PREFIXES = ("requirements",)

_MSBuild_PACKAGE_ELEMENT = re.compile(r"<Package(?:Version|Reference)\b[^>]*>", re.IGNORECASE)
_MSBuild_INCLUDE = re.compile(r'\bInclude\s*=\s*"([^"]+)"', re.IGNORECASE)
_MSBuild_VERSION = re.compile(r'\bVersion\s*=\s*"([^"]+)"', re.IGNORECASE)
_TOML_SECTION = re.compile(r"^\s*\[([^\]]+)\]\s*$")
_TOML_DEP_LINE = re.compile(r'^([A-Za-z0-9_.\-]+)\s*=\s*(.+?)\s*$')
_CARGO_DEP_STANZAS = frozenset({"dependencies", "dev-dependencies", "build-dependencies"})
_PYPROJECT_DEP_STANZAS = frozenset({"project.dependencies", "tool.poetry.dependencies", "tool.poetry.dev-dependencies", "project.optional-dependencies"})
_PYPROJECT_ARRAY = re.compile(r"^\s*dependencies\s*=\s*\[(.*)\]\s*$", re.IGNORECASE)
_PYPROJECT_STRING = re.compile(r"^([A-Za-z0-9_.\-]+)\s*(.*)$")


def _split_requirement(item: str) -> tuple[str, str | None]:
    """Split a PEP 508 requirement into a name and an exact version.

    A name with no version, or with a range operator, is not a pinned edge.
    """
    match = _PYPROJECT_STRING.match(item.strip())
    if not match:
        return "", None
    name, specifier = match.group(1), (match.group(2) or "").strip()
    exact = re.match(r"^==\s*([^\s,;]+)$", specifier)
    return name, (exact.group(1) if exact else None)
_REQUIREMENT_LINE = re.compile(r"^([A-Za-z0-9_.\-]+)\s*(\[[^\]]*\])?\s*(==|>=|<=|~=|!=|>|<)?\s*([^\s;#]+)?")


@dataclass
class ManifestRead:
    """The bounded result of reading a project's own dependency manifests."""

    pinned: dict[str, list[str]]
    sources: list[str]
    unreadable: list[str]


def _normalise_package_name(name: str) -> str:
    return name.strip().lower()


def _is_pinned_version(version: str | None) -> bool:
    """A coordinate counts as pinned when it names an exact version.

    A floating range (`*`, `latest`, a `^`/`~`/`>=` constraint, or a missing
    version) is not proof of a restored artifact, so it is not an edge.
    """
    if not version:
        return False
    value = version.strip().strip('"').strip("'")
    if not value or value in {"*", "latest"}:
        return False
    if value[0] in {"^", "~", ">", "<", "="}:
        return False
    return True


def _parse_msbuild_packages(text: str) -> list[tuple[str, str | None]]:
    found: list[tuple[str, str | None]] = []
    for element in _MSBuild_PACKAGE_ELEMENT.findall(text):
        include = _MSBuild_INCLUDE.search(element)
        if not include:
            continue
        version = _MSBuild_VERSION.search(element)
        found.append((include.group(1), version.group(1) if version else None))
    return found


def _parse_package_json_packages(text: str) -> list[tuple[str, str | None]]:
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ValueError(f"package.json is not valid JSON: {exc}") from exc
    found: list[tuple[str, str | None]] = []
    if not isinstance(payload, dict):
        return found
    for stanza in ("dependencies", "devDependencies", "peerDependencies", "optionalDependencies"):
        entries = payload.get(stanza)
        if not isinstance(entries, dict):
            continue
        for name, version in entries.items():
            if isinstance(name, str) and isinstance(version, str):
                found.append((name, version))
    return found


def _strip_toml_comment(line: str) -> str:
    quoted = False
    for index, char in enumerate(line):
        if char == '"':
            quoted = not quoted
        elif char == "#" and not quoted:
            return line[:index]
    return line


def _parse_toml_packages(text: str, stanzas: frozenset[str]) -> list[tuple[str, str | None]]:
    """Parse `[dependencies]`-style stanzas only.

    Nested tables (`tool.poetry.dependencies`) are matched by their dotted
    path so the reader never has to understand TOML beyond table headers and
    simple key/value pairs. A multi-line PEP 621 array is consumed one line at
    a time and yields the same `(name, version)` pairs.
    """
    found: list[tuple[str, str | None]] = []
    current: str | None = None
    in_array = False
    for raw in text.splitlines():
        line = _strip_toml_comment(raw).strip()
        if not line:
            continue
        section = _TOML_SECTION.match(line)
        if section:
            current = section.group(1).strip().strip('"')
            in_array = False
            continue
        if current is None or current not in stanzas:
            continue
        if in_array:
            in_array = "]" not in line
            for item in re.findall(r"[\"']([^\"']+)[\"']", line):
                name, version = _split_requirement(item)
                if name:
                    found.append((name, version))
            continue
        if "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip().strip('"')
        value = value.strip()
        if value.startswith("["):
            in_array = "]" not in value
            for item in re.findall(r"[\"']([^\"']+)[\"']", value):
                name, version = _split_requirement(item)
                if name:
                    found.append((name, version))
            continue
        found.append((key, _toml_version(value)))
    return found


def _toml_version(value: str) -> str | None:
    """Return the exact version a TOML value pins, or None when it floats."""
    match = re.match(r'^\{\s*version\s*=\s*"([^"]+)"', value)
    if match:
        return match.group(1)
    plain = value.strip().strip('"').strip("'")
    if not plain or plain.startswith("{") or plain.startswith("["):
        return None
    return plain


def _parse_pyproject_packages(text: str) -> list[tuple[str, str | None]]:
    found = _parse_toml_packages(text, _PYPROJECT_DEP_STANZAS)
    project = _toml_tables(text).get("project")
    if isinstance(project, dict):
        entries = project.get("dependencies")
        if isinstance(entries, list):
            for item in entries:
                if not isinstance(item, str):
                    continue
                name, version = _split_requirement(item)
                if name:
                    found.append((name, version))
    return found


def _toml_tables(text: str) -> dict[str, Any]:
    """Read tables and simple values out of a TOML manifest.

    This is a deliberately small literal reader, not a TOML implementation: it
    understands table headers, string/number/bool/array literals, and inline
    comments, which is the whole `pyproject.toml` surface the audit reads. A
    construct it does not understand is skipped rather than guessed at.
    """
    root: dict[str, Any] = {}
    current = root
    pending_array: list[str] | None = None
    for raw in text.splitlines():
        line = _strip_toml_comment(raw).strip()
        if not line:
            continue
        if pending_array is not None:
            pending_array.append(line)
            if "]" not in line:
                continue
            joined = " ".join(pending_array)
            pending_array = None
            for key, value in list(current.items())[::-1]:
                if value is None:
                    current[key] = _toml_array(joined)
                    break
            continue
        section = _TOML_SECTION.match(line)
        if section:
            parts = [part.strip().strip('"') for part in section.group(1).split(".")]
            current = root
            for part in parts:
                nxt = current.get(part)
                if not isinstance(nxt, dict):
                    nxt = {}
                    current[part] = nxt
                current = nxt
            continue
        if current is root and "=" not in line:
            continue
        if "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip().strip('"')
        value = value.strip()
        if value.startswith("[") and "]" not in value:
            # A multi-line array: park the key and fill it once the
            # closing bracket arrives on a later line.
            current[key] = None
            pending_array = [value]
            continue
        if value.startswith("["):
            current[key] = _toml_array(value)
            continue
        current[key] = _toml_scalar(value)
    return root


def _toml_array(value: str) -> list[str]:
    inner = value.strip()[1:]
    if inner.endswith("]"):
        inner = inner[:-1]
    return [item.strip().strip('"').strip("'") for item in re.findall(r"[\"']([^\"']*)[\"']", inner)]


def _toml_scalar(value: str) -> Any:
    if value.startswith("{") and value.endswith("}"):
        return {}
    stripped = value.strip().strip('"').strip("'")
    if stripped in {"true", "false"}:
        return stripped == "true"
    try:
        return int(stripped)
    except ValueError:
        return stripped


def _parse_requirements_packages(text: str) -> list[tuple[str, str | None]]:
    found: list[tuple[str, str | None]] = []
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line or line.startswith("-"):
            continue
        match = _REQUIREMENT_LINE.match(line)
        if not match:
            continue
        name, _, operator, version = match.group(1), match.group(2), match.group(3), match.group(4)
        found.append((name, version if operator == "==" else None))
    return found


def _read_manifest_dependencies(relative: str, text: str) -> list[tuple[str, str | None]]:
    if relative == "Directory.Packages.props" or relative.endswith(".csproj"):
        return _parse_msbuild_packages(text)
    if relative == "package.json":
        return _parse_package_json_packages(text)
    if relative == "Cargo.toml":
        return _parse_toml_packages(text, _CARGO_DEP_STANZAS)
    if relative == "pyproject.toml":
        return _parse_pyproject_packages(text)
    return _parse_requirements_packages(text)


def read_manifest_dependencies(project_path: Path) -> ManifestRead:
    """Read the consumer's own dependency manifests, bounded and read-only.

    The walk is capped at `_MANIFEST_SCAN_DEPTH` directories below the project
    root and `_MANIFEST_SCAN_FILE_BUDGET` files, skips build and vendor
    directories, and only parses the dependency stanza of the six manifest
    kinds named in the design. A manifest that exists but cannot be parsed is
    reported in `unreadable` rather than treated as "no dependencies", so an
    unreadable file can never be read as proof of either adoption or absence.
    """
    pinned: dict[str, list[str]] = {}
    sources: list[str] = []
    unreadable: list[str] = []
    budget = _MANIFEST_SCAN_FILE_BUDGET
    for directory, subdirs, files in os.walk(project_path):
        current = Path(directory)
        depth = len(current.relative_to(project_path).parts)
        if depth >= _MANIFEST_SCAN_DEPTH:
            subdirs[:] = []
            continue
        subdirs[:] = sorted(name for name in subdirs if name not in _MANIFEST_SKIP_DIRS and not name.startswith("."))
        for name in sorted(files):
            if not _is_manifest_name(name):
                continue
            if budget <= 0:
                unreadable.append("... scan budget exhausted")
                return ManifestRead(pinned, sources, unreadable)
            budget -= 1
            relative = current.relative_to(project_path).as_posix()
            relative = name if relative == "." else f"{relative}/{name}"
            try:
                text = (current / name).read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                unreadable.append(relative)
                continue
            try:
                entries = _read_manifest_dependencies(relative, text)
            except ValueError:
                unreadable.append(relative)
                continue
            sources.append(relative)
            for package, version in entries:
                if _is_pinned_version(version):
                    pinned.setdefault(_normalise_package_name(package), []).append(f"{version} ({relative})")
    return ManifestRead(pinned, sorted(sources), sorted(unreadable))


def _is_manifest_name(name: str) -> bool:
    """Match only the six manifest kinds the design names, nothing else."""
    if name in _MANIFEST_EXACT_NAMES or name.endswith(".csproj"):
        return True
    return name.startswith(_MANIFEST_PREFIXES) and name.endswith(".txt")


def _validate_consumes_entry(
    entry: Any,
    capability_id: str,
    provider: str,
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> bool:
    """Validate one `consumes[]` record. Returns True when the record holds.

    A record that names the wrong provider is not a consumption edge for this
    capability; the caller reports the capability as not adopted.
    """
    if not isinstance(entry, dict):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_UNEVIDENCED, f"capability {capability_id!r} consumes[] entry must be an object", project_id)
        return False
    declared_provider = entry.get("provider")
    if isinstance(declared_provider, str) and declared_provider.strip() and declared_provider.strip() != provider:
        finding(
            findings,
            "WARNING",
            CAPABILITY_ADOPTION_UNEVIDENCED,
            f"capability {capability_id!r} consumes[] names provider {declared_provider!r} but the registry binds it to {provider!r}",
            project_id,
        )
        return False
    ref = entry.get("evidence_ref")
    if not isinstance(ref, str) or not ref.strip():
        finding(findings, "WARNING", CAPABILITY_ADOPTION_UNEVIDENCED, f"capability {capability_id!r} consumes[] entry for {provider!r} has no evidence_ref", project_id)
        return False
    if _is_absolute_path(ref) or _has_path_escape(ref):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_EVIDENCE_STALE, f"capability {capability_id!r} evidence_ref is absolute or escapes the project root and was not read: {_redact_secret_value(ref, secret_words)}", project_id)
        return False
    if not _standalone_evidence_resolves(project_path, ref):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_EVIDENCE_STALE, f"capability {capability_id!r} evidence_ref no longer resolves in the project root: {_redact_secret_value(ref, secret_words)}", project_id)
        return False
    return True


def _local_implementation_note(entry: dict[str, Any] | None, project_path: Path, secret_words: "re.Pattern[str]") -> str:
    """Describe a declared local implementation without ever guessing one.

    Detection is bounded to declaration-supplied paths. An unregistered
    hand-rolled implementation is never inferred, because an inferred
    duplicate cannot be distinguished from a legitimate boundary decision.
    """
    if not isinstance(entry, dict):
        return ""
    raw = entry.get("local_implementation")
    paths = raw if isinstance(raw, list) else ([raw] if isinstance(raw, str) else [])
    notes: list[str] = []
    for item in paths:
        if not isinstance(item, str) or not item:
            continue
        if _is_absolute_path(item) or _has_path_escape(item):
            notes.append(f"declared local implementation {_redact_secret_value(item, secret_words)} escapes the project root and was not read")
            continue
        state = "present" if (project_path / item).exists() else "absent"
        notes.append(f"declared local implementation {_redact_secret_value(item, secret_words)} ({state})")
    return "; ".join(notes)


def _scan_prose_declines(project_path: Path, capability_id: str) -> list[tuple[str, str]]:
    """Find a project's own prose boundary argument for a capability.

    This names evidence the audit already has; it never *is* a decline. A hit
    asks the repository for a `declines[]` record via
    `CAPABILITY_ADOPTION_DECLINE_UNRECORDED`. The scan is bounded to four
    document names plus `.ai-rules/*.md` and reads them in full only when the
    capability id is mentioned at all.
    """
    documents = ["AGENTS.md", "README.md", "HANDOFF.md", "ROADMAP.md"]
    rules_dir = project_path / ".ai-rules"
    if rules_dir.is_dir():
        documents.extend(f".ai-rules/{item.name}" for item in sorted(rules_dir.glob("*.md")))
    mentions = {name.lower() for name in capability_id.replace("-", " ").split()}
    found: list[tuple[str, str]] = []
    for relative in documents:
        try:
            text = (project_path / relative).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        lowered = text.lower()
        if not all(mention in lowered for mention in mentions):
            continue
        for rationale, triggers in _PROSE_DECLINE_TRIGGERS:
            for trigger in triggers:
                position = lowered.find(trigger)
                while position != -1:
                    window = lowered[max(0, position - 240) : position + 240]
                    if all(mention in window for mention in mentions):
                        found.append((relative, rationale))
                        break
                    position = lowered.find(trigger, position + 1)
                if found and found[-1][0] == relative and found[-1][1] == rationale:
                    break
    return found


def validate_declines(
    declines: Any,
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    registry: CapabilityRegistry,
) -> set[str]:
    """Validate a project's `declines[]` records and return the exempt ids.

    A decline is repo-local and capability-scoped: it exempts this project for
    this capability only, and never suppresses a finding in any other project
    or for any other capability. A decline with no resolvable evidence is
    itself reported, because an assertion is not a boundary argument.
    """
    exempt: set[str] = set()
    if declines is None:
        return exempt
    if not isinstance(declines, list):
        finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, "declines must be an array", project_id)
        return exempt
    for index, entry in enumerate(declines):
        if not isinstance(entry, dict):
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"declines[{index}] must be an object", project_id)
            continue
        capability_id = entry.get("capability")
        if not isinstance(capability_id, str) or not capability_id:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"declines[{index}] requires a capability id", project_id)
            continue
        if capability_id not in registry.capabilities:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"declines[{index}] names capability {capability_id!r}, which binds to no provider in the capability registry", project_id)
            continue
        rationale = entry.get("rationale")
        if rationale not in DECLINE_RATIONALES:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"decline for capability {capability_id!r} cites an unknown rationale: {rationale!r}; accepted values: {', '.join(DECLINE_RATIONALES)}", project_id)
            continue
        ref = entry.get("evidence_ref")
        if not isinstance(ref, str) or not ref.strip():
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"decline for capability {capability_id!r} requires an evidence_ref that resolves in the project root", project_id)
            continue
        if not _standalone_evidence_resolves(project_path, ref):
            finding(findings, "WARNING", CAPABILITY_ADOPTION_DECLINE_UNRECORDED, f"decline for capability {capability_id!r} cites an evidence_ref that does not resolve in the project root: {ref}", project_id)
            continue
        exempt.add(capability_id)
    return exempt


def provider_capability_verified(
    workspace_root: Path,
    provider_paths: dict[str, Path],
    provider_id: str,
    capability_id: str,
) -> bool:
    """Return True when the provider declares this capability `verified`.

    The provider's own declaration is the only accepted evidence. A grant or a
    consumer record never substitutes for it, and an unreadable or absent
    provider declaration is *not* verified — which is what keeps an
    unverifiable provider from graduating a capability to `required`.
    """
    provider_path = provider_paths.get(provider_id)
    if provider_path is None:
        return False
    try:
        declaration = json.loads((provider_path / ".project.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError):
        return False
    if not isinstance(declaration, dict):
        return False
    capabilities = declaration.get("capabilities")
    if not isinstance(capabilities, dict):
        return False
    entry = capabilities.get(capability_id)
    if not isinstance(entry, dict) or entry.get("evidence_state") != "verified":
        return False
    ref = entry.get("evidence_ref")
    if not isinstance(ref, str) or not ref:
        return False
    del workspace_root  # the provider path already carries the root
    return _standalone_evidence_resolves(provider_path, ref)


def measure_capability_adoption(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    registry: CapabilityRegistry | None,
    provider_verified: Any = None,
) -> None:
    """Measure whether this project consumes each registered capability.

    Read-only and additive: a project that declares no `capabilities` block and
    no `declines[]` record produces no adoption finding, and a capability in
    `exempt` posture is never charged to anyone. The measurement never promotes
    a capability to `verified`; it only reports what the three sources agree
    on.
    """
    if registry is None:
        return
    secret_words = secret_words_for()
    capabilities = declaration.get("capabilities")
    consumes_index: dict[str, list[Any]] = {}
    if isinstance(capabilities, dict):
        for name, entry in capabilities.items():
            if isinstance(entry, dict) and isinstance(entry.get("consumes"), list):
                consumes_index[name] = entry["consumes"]
    declined = validate_declines(declaration.get("declines"), project_path, findings, project_id, registry)
    for name, entries in sorted(consumes_index.items()):
        if name not in registry.capabilities and name not in declined:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"capabilities[{name!r}].consumes[] names a capability that binds to no provider in the capability registry", project_id)
    manifests: ManifestRead | None = None
    prose_scanned: set[str] = set()
    for capability_id in sorted(registry.capabilities):
        entry = registry.capabilities[capability_id]
        provider = _resolved_provider(entry)
        if provider is None:
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"capability {capability_id!r} does not bind to exactly one provider in the capability registry; no repository is charged", project_id)
            continue
        if provider == project_id:
            # The provider is exempt for its own capability: `consumes[]` is
            # empty and a self-adoption failure would be meaningless.
            continue
        if capability_id in declined:
            continue
        verified = bool(provider_verified(provider, capability_id)) if callable(provider_verified) else False
        posture, posture_reason = effective_capability_posture(capability_id, entry, registry, verified)
        if posture == "exempt":
            continue
        package = entry.get("package")
        if not isinstance(package, str) or not package.strip():
            finding(findings, "WARNING", CAPABILITY_ADOPTION_UNRESOLVED, f"capability {capability_id!r} binds to provider {provider!r} without naming a package; no repository is charged", project_id)
            continue
        if manifests is None:
            manifests = read_manifest_dependencies(project_path)
        edges = manifests.pinned.get(_normalise_package_name(package), [])
        consumes = consumes_index.get(capability_id, [])
        # The declaration record is validated on its own terms first: an
        # `evidence_ref` that escapes the root or no longer resolves is refused
        # whether or not the manifest edge happens to exist, and a capability
        # with no valid record is undocumented, never adopted.
        record_ok = bool(consumes) and any(
            _validate_consumes_entry(item, capability_id, provider, project_path, findings, project_id, secret_words)
            for item in consumes
        )
        if not edges:
            if manifests.unreadable:
                finding(
                    findings,
                    "WARNING",
                    CAPABILITY_ADOPTION_UNEVIDENCED,
                    f"capability {capability_id!r} consumption is unproven: {provider}/{package} is not pinned and a manifest could not be read ({', '.join(manifests.unreadable)}); adoption is never assumed",
                    project_id,
                )
                continue
            if not consumes and capability_id not in prose_scanned:
                prose_scanned.add(capability_id)
                for document, rationale in _scan_prose_declines(project_path, capability_id):
                    finding(
                        findings,
                        "WARNING",
                        CAPABILITY_ADOPTION_DECLINE_UNRECORDED,
                        f"{document} documents a non-adoption boundary for capability {capability_id!r} ({rationale}) but the project records no declines[] entry; prose is neither adoption nor a decline",
                        project_id,
                    )
                    break
            level = "ERROR" if posture == "required" else "WARNING"
            detail = _local_implementation_note(consumes[0] if consumes else None, project_path, secret_words)
            message = (
                f"capability {capability_id!r} is {posture} ({posture_reason}) but no manifest in the project root pins {package} from provider {provider!r}"
            )
            if consumes:
                message += f"; declared in capabilities[{capability_id!r}].consumes[] with no matching manifest edge"
            else:
                message += f"; capabilities[{capability_id!r}].consumes[] is absent"
            if detail:
                message += f"; {detail}"
            finding(findings, level, CAPABILITY_ADOPTION_MISSING, message, project_id)
            continue
        if not consumes:
            finding(
                findings,
                "WARNING",
                CAPABILITY_ADOPTION_UNEVIDENCED,
                f"manifest pins {package} from {', '.join(sorted(set(edges)))} but capabilities[{capability_id!r}].consumes[] records no evidence_ref for provider {provider!r}; the edge is real and the record is not",
                project_id,
            )
            continue
        del record_ok  # a valid record plus a proven edge is adoption; no finding


ADOPTION_GRANT_SHAPE = "ADOPTION_GRANT_SHAPE"
ADOPTION_GRANT_SCOPE_VIOLATION = "ADOPTION_GRANT_SCOPE_VIOLATION"
ADOPTION_GRANT_UNSATISFIED = "ADOPTION_GRANT_UNSATISFIED"
ADOPTION_GRANT_STALE = "ADOPTION_GRANT_STALE"
ADOPTION_GRANT_PROVIDER_UNVERIFIED = "ADOPTION_GRANT_PROVIDER_UNVERIFIED"
ADOPTION_GRANT_FIELDS = ("capability", "provider", "package", "actor", "issued_at", "expires_at", "write_set")
ADOPTION_RECORD_FIELDS = ("consumer", "capability", "provider", "package", "version", "replaced", "actor", "grant_expires_at", "rollback_ref", "recorded_at")
ADOPTION_LEDGER_FILE = "adoptions.json"
# Sources a source-level coupling would arrive through. The grant permits
# consuming a published artifact; it never permits a checkout dependency.
_COUPLING_SOURCES = ("Cargo.toml", "package.json", "pyproject.toml", "go.mod", "NuGet.config", "Directory.Build.props")
_PROMOTION_LEDGER_PATTERN = re.compile(r"(?:^|/)deployment/promotions(?:\.sha256|\.json)?$")
_MSBUILD_PROJECT_REFERENCE = re.compile(r"<ProjectReference\b[^>]*>", re.IGNORECASE)
_MSBUILD_PATH_ATTRIBUTE = re.compile(r'\b(?:Include|Path)\s*=\s*"([^"]+)"', re.IGNORECASE)


def adoption_ledger_path(governance_root: Path | None = None) -> Path:
    """Return the append-only adoption ledger path at the governance root."""
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    return root / "adoption" / ADOPTION_LEDGER_FILE


def _canonical_record_hash(record: dict[str, Any]) -> str:
    """Hash a record body, excluding the `content_hash` field itself."""
    body = {key: value for key, value in record.items() if key != "content_hash"}
    return hashlib.sha256(json.dumps(body, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()


def load_adoption_ledger(governance_root: Path | None, findings: list[Finding]) -> list[dict[str, Any]]:
    """Load `adoption/adoptions.json` with tamper detection.

    A missing ledger returns an empty list and no finding, matching
    `load_promotion_ledger`: a repository with no grant has nothing to record.
    A record whose `content_hash` does not match its body, or whose
    `prev_hash` does not match the row before it, is refused with
    `ADOPTION_GRANT_UNSATISFIED` — the same posture as a hand-edited
    `deployable: true` (`DEPLOYMENT_PROMOTION`). The rows are still returned so
    the audit can keep reporting against them.
    """
    path = adoption_ledger_path(governance_root)
    if not path.is_file():
        return []
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeDecodeError) as exc:
        finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger cannot be parsed at {path}: {exc}")
        return []
    if not isinstance(payload, dict) or not isinstance(payload.get("records"), list):
        finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger must be an object with a records array: {path}")
        return []
    records = payload["records"]
    previous = ""
    for index, record in enumerate(records):
        if not isinstance(record, dict):
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger row {index} must be an object")
            continue
        recorded = record.get("content_hash")
        if not isinstance(recorded, str) or recorded != _canonical_record_hash(record):
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger row {index} does not match its content_hash; a hand-edited record is refused")
        if record.get("prev_hash", "") != previous:
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption ledger row {index} breaks the append chain; a hand-inserted row is refused")
        previous = recorded if isinstance(recorded, str) else _canonical_record_hash(record)
    return records


def _parse_grant_time(value: Any) -> datetime | None:
    """Parse an ISO 8601 timestamp, tolerating a trailing `Z`."""
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return parsed.replace(tzinfo=timezone.utc)
    return parsed


def _write_set_escapes(project_path: Path, pattern: str) -> bool:
    """Return True when a write_set pattern resolves outside the consumer root.

    `..` and absolute patterns are already rejected as shape errors. This
    catches the remaining escape route: a literal path segment that is a
    symlink pointing outside the repository. The path is never read.
    """
    literal = re.split(r"[*?\[]", pattern, maxsplit=1)[0]
    literal = literal.rsplit("/", 1)[0] if "/" in literal else ""
    if not literal:
        return False
    try:
        root = project_path.resolve(strict=True)
        resolved = (project_path / literal).resolve(strict=False)
    except (OSError, RuntimeError):
        return False
    return not resolved.is_relative_to(root)


def _scan_source_coupling(project_path: Path, provider_path: Path | None = None) -> list[tuple[str, str]]:
    """Return source-level couplings found in the consumer's own manifests.

    A `ProjectReference`, relative path, or `../` dependency on a sibling
    checkout is forbidden under any grant, so this reuses the existing bounded
    sibling scan and adds the MSBuild `ProjectReference` case, which is how a
    .NET consumer would couple to a provider source tree. Nothing outside the
    consumer root is ever read.
    """
    found: list[tuple[str, str]] = list(_bounded_static_scan_for_siblings(project_path))
    candidates = list(_COUPLING_SOURCES)
    candidates.extend(sorted(item.name for item in project_path.glob("*.csproj")))
    for name in candidates:
        path = project_path / name
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for element in _MSBUILD_PROJECT_REFERENCE.findall(text):
            for attribute in _MSBUILD_PATH_ATTRIBUTE.findall(element):
                if _is_absolute_path(attribute) or _has_path_escape(attribute):
                    found.append((name, attribute))
    if provider_path is not None:
        provider_key = provider_path.name.lower()
        for relative, target in found:
            if provider_key in target.lower() or provider_key in relative.lower():
                return [(relative, target)]
    return found


def _declared_consumes(declaration: dict[str, Any], capability_id: str) -> list[Any]:
    capabilities = declaration.get("capabilities")
    if not isinstance(capabilities, dict):
        return []
    entry = capabilities.get(capability_id)
    if not isinstance(entry, dict) or not isinstance(entry.get("consumes"), list):
        return []
    return entry["consumes"]


def validate_adoption_grant(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    capability_registry: CapabilityRegistry | None = None,
    provider_paths: dict[str, Path] | None = None,
    ledger: list[dict[str, Any]] | None = None,
    provider_verified: Any = None,
    now: datetime | None = None,
) -> None:
    """Audit a project's optional `adoption_grant` declaration.

    The grant is the only permission in this repository that lets an agent
    touch a capability owned by a sibling. It is deliberately narrow: one
    consumer, one capability, one provider, one package, one actor, one named
    write set, one mandatory expiry, single use. The provider repository is
    read-only at all times, and the check is read-only like every other audit
    check — it proves an authorization existed, was in scope, and produced a
    recorded, unmodified outcome. It never runs a build or a test.
    """
    grant = declaration.get("adoption_grant")
    if grant is None:
        return
    if not isinstance(grant, dict):
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, "adoption_grant must be an object", project_id)
        return
    for field in ADOPTION_GRANT_FIELDS:
        if field not in grant:
            finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.{field} is required", project_id)
    if any(field not in grant for field in ADOPTION_GRANT_FIELDS):
        return
    write_set = grant["write_set"]
    if not isinstance(write_set, list) or not write_set:
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, "adoption_grant.write_set must be a non-empty array", project_id)
        return
    for index, pattern in enumerate(write_set):
        if not isinstance(pattern, str) or not pattern:
            finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.write_set[{index}] must be a non-empty string", project_id)
            return
        if _is_absolute_path(pattern) or _has_path_escape(pattern):
            finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.write_set[{index}] is absolute or escapes the project root: {pattern}", project_id)
            return
    status = grant.get("status", "active")
    if status not in {"active", "revoked"}:
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, f"adoption_grant.status is invalid: {status!r}", project_id)
        return

    capability_id = str(grant["capability"])
    provider_id = str(grant["provider"])
    package = str(grant["package"])
    paths = provider_paths or {}

    # 1. Identity. A grant may not invent a provider, and it may not be the
    #    thing that creates the capability binding it consumes.
    if provider_id not in paths:
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"adoption_grant names provider {provider_id!r}, which is not a registered project id; the grant is refused and no write under it is validated", project_id)
        return
    if capability_registry is None or capability_id not in capability_registry.capabilities:
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"adoption_grant names capability {capability_id!r}, which binds to no provider in the capability registry; the grant is refused and no write under it is validated", project_id)
        return
    bound_provider = _resolved_provider(capability_registry.capabilities[capability_id])
    if bound_provider != provider_id:
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"adoption_grant names capability {capability_id!r} with provider {provider_id!r}, but the registry binds it to {bound_provider!r}; the grant is refused and no write under it is validated", project_id)
        return
    provider_path = paths.get(provider_id)

    # 2. Window. Expiry is mandatory precisely so a grant cannot become
    #    standing authorization, and a repoint is not retroactively legal.
    moment = now or datetime.now(timezone.utc)
    expires_at = _parse_grant_time(grant["expires_at"])
    issued_at = _parse_grant_time(grant["issued_at"])
    if expires_at is None or issued_at is None:
        finding(findings, "ERROR", ADOPTION_GRANT_SHAPE, "adoption_grant.issued_at and adoption_grant.expires_at must be ISO 8601 timestamps", project_id)
        return
    if expires_at <= moment:
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, f"adoption_grant expired at {grant['expires_at']}; the repoint it authorized is not retroactively lawful", project_id)
    if issued_at > moment:
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, f"adoption_grant is not active before {grant['issued_at']}", project_id)
    if status == "revoked":
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, "adoption_grant is revoked; it authorizes no repoint", project_id)

    # 3. Provider. There is nothing lawful to adopt from an unverified
    #    provider, and a grant never substitutes for the provider's evidence.
    if not (callable(provider_verified) and provider_verified(provider_id, capability_id)):
        finding(findings, "WARNING", ADOPTION_GRANT_PROVIDER_UNVERIFIED, f"provider {provider_id!r} does not declare capability {capability_id!r} as verified with a resolving evidence_ref; the grant has nothing lawful to adopt from", project_id)

    # 4. Write set. Every permitted write resolves inside the consumer root.
    for index, pattern in enumerate(write_set):
        normalised = pattern.replace("\\", "/")
        if _PROMOTION_LEDGER_PATTERN.search(normalised):
            finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"write_set[{index}] names the promotion ledger ({pattern}); adoption is not promotion and is never a path to deployable", project_id)
        if provider_path is not None and provider_path.name.lower() in normalised.lower():
            finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"write_set[{index}] names the provider repository ({pattern}); the provider root is read-only under every grant", project_id)
        if _write_set_escapes(project_path, pattern):
            finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"write_set[{index}] resolves outside the consumer root and was not read: {pattern}", project_id)

    # 5. Forbidden content in the consumer's own files. These are refused
    #    regardless of the resulting code state, because the violation is the
    #    finding.
    for source, target in _scan_source_coupling(project_path, provider_path):
        finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"{source} carries a source-level coupling to a sibling checkout ({target}); a ProjectReference, relative path, or ../ dependency remains forbidden regardless of any grant", project_id)
    deployment = declaration.get("deployment")
    if isinstance(deployment, dict) and deployment.get("deployable"):
        finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, "adoption_grant is active but deployment.deployable is true; adoption is not promotion and is never a path to deployable", project_id)
    granted_version = grant.get("version")
    if isinstance(granted_version, str) and granted_version:
        pinned = read_manifest_dependencies(project_path).pinned.get(_normalise_package_name(package), [])
        for edge in pinned:
            actual = edge.split(" ", 1)[0]
            if actual != granted_version:
                finding(findings, "ERROR", ADOPTION_GRANT_SCOPE_VIOLATION, f"consumer pins {package} {actual} but the grant authorizes {granted_version}; a version change to provider {provider_id!r} is a provider change and needs that change's own authorization", project_id)

    # 6. The record. A granted repoint must leave an immutable adoption row,
    #    and one permission is sized for one change.
    records = ledger or []
    mine = [
        item
        for item in records
        if isinstance(item, dict)
        and item.get("consumer") == project_id
        and item.get("capability") == capability_id
        and item.get("provider") == provider_id
    ]
    adoptions = [item for item in mine if item.get("kind", "adoption") == "adoption"]
    for item in mine:
        missing = [field for field in ADOPTION_RECORD_FIELDS if field not in item]
        if missing:
            finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"adoption record for {project_id}/{capability_id} is missing {', '.join(missing)}; the adoption is not treated as authorized", project_id)
    if len(adoptions) > 1:
        finding(findings, "WARNING", ADOPTION_GRANT_STALE, f"{len(adoptions)} adoption records exist for {project_id}/{capability_id} under one grant; the permission was sized for one change", project_id)
    consumed = _declared_consumes(declaration, capability_id)
    if consumed and not mine:
        finding(findings, "ERROR", ADOPTION_GRANT_UNSATISFIED, f"{project_id} declares capabilities[{capability_id!r}].consumes[] but no adoption record exists in adoption/{ADOPTION_LEDGER_FILE}; the adoption is not treated as authorized", project_id)


# ---------------------------------------------------------------------------
# Declared interfaces and boundary findings
#
# A project MAY declare the boundaries it provides and consumes in an
# optional `interfaces[]` array. Everything below is read-only: the audit
# never writes a declaration, never executes a project command, and never
# reads a byte outside the workspace root it was given. Three properties are
# deliberate and each one is a rule, not an implementation detail:
#
# 1. A declaration is validated as a whole before anything is asserted about
#    it, so a malformed declaration never yields a partial read.
# 2. A project that declares no interfaces reports nothing. Governance
#    charges a boundary a project claimed; it never invents one, so a
#    duplicate that nobody declared stays invisible rather than guessed at.
# 3. An edge counts as declared only when both sides declare it, so a
#    consumption the provider does not provide is a finding rather than
#    silent adoption.
#
# Two of the declaration rules are asserted as findings rather than as
# malformation, because the canonical specification names a distinct code for
# each and a malformed declaration yields no findings at all: a contract
# without a version (`INTERFACE_CONTRACT_UNVERSIONED`) and a `sibling` edge
# that is not pinned (`INTERFACE_SIBLING_PATH_UNPINNED`). Every other rule in
# `schemas/project.schema.json` is in the malformed set below.
# ---------------------------------------------------------------------------


def interface_escalation_dates(governance_root: Path | None = None) -> dict[str, str]:
    """`{finding code: ISO effective date}` recorded in the decisions register.

    A ramp is an owner decision, not a property of the checker. The register
    entry lives in `decisions.json` and carries an `escalates` object naming
    the finding codes and the date the WARNING becomes blocking; the register
    loader tolerates additive entry-level keys, which is what lets the ramp be
    machine-readable without a second list to keep in sync.

    Unreadable or malformed input yields an empty mapping, so the finding
    stays a WARNING: nothing may become blocking on the strength of a record
    that could not be read. Read-only and never raises.
    """
    override = os.environ.get("WORKSPACE_DECISIONS_PATH")
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = Path(override) if override else root / "decisions.json"
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return {}
    if not isinstance(document, dict):
        return {}
    entries = document.get("decisions")
    if not isinstance(entries, list):
        return {}
    dates: dict[str, str] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        ramp = entry.get(INTERFACE_ESCALATION_FIELD)
        if not isinstance(ramp, dict):
            continue
        codes = ramp.get("finding_codes")
        effective = ramp.get("effective")
        if not isinstance(codes, list) or not isinstance(effective, str):
            continue
        try:
            date.fromisoformat(effective)
        except ValueError:
            continue
        for code in codes:
            if isinstance(code, str) and code:
                dates[code] = effective
    return dates


def interface_finding_level(
    code: str,
    escalation: dict[str, str] | None = None,
    today: date | None = None,
) -> str:
    """The level an interface finding is reported at: WARNING, or blocking after the ramp.

    `INTERFACE_SIBLING_PATH_UNPINNED` and `INTERFACE_VENDORED_WITHOUT_DIGEST`
    begin as WARNINGs and become blocking once the date recorded in
    `decisions.json` has passed. The escalation is never triggered by an
    environment variable, by a project-controlled switch, or by elapsed time
    alone: it is an owner decision, it is dated, and it stays visible and
    revocable in the register. `FAIL` in the finding vocabulary is the
    blocking level `ERROR`, which is what the gate turns into a failed run.
    """
    effective = (escalation or {}).get(code)
    if not effective:
        return "WARNING"
    try:
        boundary = date.fromisoformat(effective)
    except ValueError:
        return "WARNING"
    return "ERROR" if (today or date.today()) >= boundary else "WARNING"


def build_interface_index(
    workspace_root: Path, projects: list[dict[str, Any]]
) -> dict[tuple[str, str], dict[str, Any]]:
    """Index every declared `provides` interface by `(project id, interface id)`.

    One bounded read per declaration, read-only, and no directory walk: the
    provider side of an edge is a declaration, so the index is the set of
    declarations the fleet actually makes. A declaration that cannot be read,
    or an `interfaces` value that is not an array, contributes nothing — and
    nothing is exactly what a consumer's missing provision has to look like
    for the one-sided edge to be reported rather than assumed.
    """
    index: dict[tuple[str, str], dict[str, Any]] = {}
    for entry in projects:
        if not isinstance(entry, dict):
            continue
        project_id = entry.get("id")
        path = entry.get("path")
        if not isinstance(project_id, str) or not isinstance(path, str):
            continue
        try:
            declaration = json.loads(
                (workspace_root / path / ".project.json").read_text(encoding="utf-8")
            )
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            continue
        if not isinstance(declaration, dict):
            continue
        interfaces = declaration.get("interfaces")
        if not isinstance(interfaces, list):
            continue
        for item in interfaces:
            if not isinstance(item, dict):
                continue
            interface_id = item.get("id")
            if item.get("provides") is not True or not isinstance(interface_id, str):
                continue
            if item.get("mechanism") not in INTERFACE_MECHANISMS:
                continue
            index.setdefault((project_id, interface_id), item)
    return index


def _interface_shape_problems(interfaces: Any) -> list[str]:
    """Every declaration-shape problem in an `interfaces` value, in order.

    This is the malformed set: the required keys, the id grammar and
    uniqueness, the exclusive side flags, the provider a consumption names,
    the entry and digest and evidence types, the `observe` path rules, the
    digest a `consume`/`observe` edge must carry, and the contract every
    mechanism except `sibling` must name. An unknown mechanism is
    deliberately absent from this set: it has its own finding and the entry is
    otherwise ignored, so a mechanism this checker does not know is not
    reported as a malformed document.
    """
    problems: list[str] = []
    if not isinstance(interfaces, list):
        return ["`interfaces` must be an array of interface declarations"]
    seen: dict[str, int] = {}
    for index, item in enumerate(interfaces):
        where = f"interfaces[{index}]"
        if not isinstance(item, dict):
            problems.append(f"{where} must be an object")
            continue
        interface_id = item.get("id")
        if isinstance(interface_id, str) and PROJECT_ID.fullmatch(interface_id):
            where = f"interface {interface_id!r}"
            if interface_id in seen:
                problems.append(
                    f"{where} repeats the id already declared at interfaces[{seen[interface_id]}]; "
                    "interface ids are unique within a project"
                )
            else:
                seen[interface_id] = index
        else:
            problems.append(f"{where}.id must be a kebab-case identifier")
        unknown = sorted(key for key in item if key not in INTERFACE_ENTRY_KEYS)
        if unknown:
            problems.append(f"{where} has unknown key(s): {', '.join(unknown)}")
        provides = item.get("provides")
        consumes = item.get("consumes")
        for key, value in (("provides", provides), ("consumes", consumes)):
            if value is not None and not isinstance(value, bool):
                problems.append(f"{where}.{key} must be a boolean")
        if (provides is True) == (consumes is True):
            problems.append(f"{where} must declare exactly one of provides/consumes as true")
        mechanism = item.get("mechanism")
        if not isinstance(mechanism, str) or not mechanism:
            problems.append(f"{where}.mechanism is required")
        entry = item.get("entry")
        if not isinstance(entry, str) or not entry:
            problems.append(f"{where}.entry is required")
        elif _is_absolute_path(entry):
            problems.append(f"{where}.entry must not be an absolute path")
        if consumes is True:
            provider = item.get("provider")
            if not isinstance(provider, str) or not PROJECT_ID.fullmatch(provider):
                problems.append(
                    f"{where}.provider must be the kebab-case id of the project providing it"
                )
        contract = item.get("contract")
        if contract is not None and (
            not isinstance(contract, str) or not INTERFACE_CONTRACT_NAME.fullmatch(contract)
        ):
            problems.append(f"{where}.contract must be a \"<name>/<version>\" identifier")
        digest = item.get("digest")
        if digest is not None and (
            not isinstance(digest, str) or not INTERFACE_DIGEST.fullmatch(digest)
        ):
            problems.append(f"{where}.digest must be a sha256 digest of the form sha256:<64 hex>")
        evidence_ref = item.get("evidence_ref")
        if evidence_ref is not None and (
            not isinstance(evidence_ref, str)
            or not evidence_ref
            or _is_absolute_path(evidence_ref)
            or _has_path_escape(evidence_ref)
        ):
            problems.append(f"{where}.evidence_ref must be a repository-relative path")
        required = item.get("required")
        if required is not None and not isinstance(required, bool):
            problems.append(f"{where}.required must be a boolean")
        if isinstance(mechanism, str) and mechanism in INTERFACE_MECHANISMS:
            if mechanism != "sibling" and contract is None:
                problems.append(f"{where}.contract is required for mechanism {mechanism!r}")
            if mechanism in ("consume", "observe") and digest is None:
                problems.append(f"{where}.digest is required for mechanism {mechanism!r}")
            if mechanism == "observe" and isinstance(entry, str) and entry and _has_path_escape(entry):
                problems.append(
                    f"{where}.entry must be a repository-relative document path; "
                    "'..' is refused and never normalized"
                )
    return problems


def _retained_copy_digest(project_path: Path, entry: str) -> str | None:
    """The sha256 of the bytes an `entry` names inside the project, or None.

    `entry` is a retained copy only when it names a file the project actually
    holds. A published coordinate (`@platform/design-tokens`) names no file
    here, and the audit fetches nothing: no network, no package manager, no
    lockfile. `None` therefore means "not a retained copy the audit can
    digest", never "a copy that matched".
    """
    if _is_absolute_path(entry) or _has_path_escape(entry):
        return None
    try:
        resolved = (project_path / entry).resolve(strict=True)
        resolved.relative_to(project_path.resolve(strict=True))
    except (OSError, RuntimeError, ValueError):
        return None
    if not resolved.is_file():
        return None
    try:
        return "sha256:" + hashlib.sha256(resolved.read_bytes()).hexdigest()
    except OSError:
        return None


def _sibling_edge_pinning_problem(item: dict[str, Any]) -> str:
    """Why a `sibling` edge is not pinned, or `""` when it is.

    Both halves are required: the consuming project declares the provider
    `required: true`, and the edge carries a digest of the invocation it
    wrote. The digest is over the `entry` string, which is the invocation —
    the thing a consumer can actually pin, and the same bytes the generator
    hashes when it writes the entry.
    """
    missing: list[str] = []
    if item.get("required") is not True:
        missing.append("`required: true` naming the provider as part of the baseline")
    digest = item.get("digest")
    entry = item.get("entry")
    if not isinstance(digest, str):
        missing.append("a `digest`")
    elif not isinstance(entry, str) or digest != "sha256:" + hashlib.sha256(
        entry.encode("utf-8")
    ).hexdigest():
        missing.append("a `digest` that matches the declared invocation")
    if not missing:
        return ""
    return "is missing " + " and ".join(missing)


def validate_interfaces(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    index: dict[tuple[str, str], dict[str, Any]] | None = None,
    escalation: dict[str, str] | None = None,
) -> None:
    """Validate this project's `interfaces[]` and charge its unpinned edges.

    Read-only. An absent `interfaces` key is valid and silent, and a
    declaration that fails the shape rules above yields
    `INTERFACE_DECLARATION_INVALID` and no interface assertions at all — never
    a partial read. The declared rules are then asserted per entry: the
    closed mechanism set, the versioned contract, the pinning rule for
    `sibling`, the cross-project pairing of a `consumes` edge against the
    provider's own `provides` entry, the consumer's own evidence for a
    consumption, and the digest of a retained byte copy.
    """
    if "interfaces" not in declaration:
        return
    interfaces = declaration.get("interfaces")
    problems = _interface_shape_problems(interfaces)
    if problems:
        for problem in problems:
            finding(
                findings,
                "ERROR",
                INTERFACE_DECLARATION_INVALID,
                f"invalid interface declaration: {problem}",
                project_id,
            )
        return

    for item in interfaces:
        interface_id = item.get("id")
        mechanism = item.get("mechanism")
        if mechanism not in INTERFACE_MECHANISMS:
            finding(
                findings,
                "ERROR",
                INTERFACE_MECHANISM_UNKNOWN,
                f"interface {interface_id!r} declares mechanism {mechanism!r}; "
                f"accepted mechanisms: {', '.join(INTERFACE_MECHANISMS)}",
                project_id,
            )
            continue
        if mechanism == "sibling":
            unpinned = _sibling_edge_pinning_problem(item)
            if unpinned:
                finding(
                    findings,
                    interface_finding_level(INTERFACE_SIBLING_PATH_UNPINNED, escalation),
                    INTERFACE_SIBLING_PATH_UNPINNED,
                    f"interface {interface_id!r} reaches provider {item.get('provider')!r} by a "
                    f"repository-relative path and {unpinned}; a sibling edge is lawful only when "
                    f"the provider is declared `required: true` and the edge is digest-pinned. "
                    f"The two lawful alternatives: {SIBLING_LAWFUL_ALTERNATIVES}",
                    project_id,
                )
        else:
            contract = item.get("contract")
            if not isinstance(contract, str) or not INTERFACE_CONTRACT.fullmatch(contract):
                finding(
                    findings,
                    "ERROR",
                    INTERFACE_CONTRACT_UNVERSIONED,
                    f"interface {interface_id!r} names contract {contract!r} without a version; "
                    "write \"<name>/<major>.<minor>.<patch>\"",
                    project_id,
                )
                continue
        if item.get("consumes") is not True:
            continue
        provider = item.get("provider")
        if (index or {}).get((provider, interface_id)) is None:
            finding(
                findings,
                "WARNING",
                INTERFACE_PROVIDER_UNDECLARED,
                f"interface {interface_id!r} consumes from provider {provider!r}, which declares "
                "no matching `provides` entry for it; an edge counts as declared only when both "
                "sides declare it",
                project_id,
            )
        evidence_ref = item.get("evidence_ref")
        if not isinstance(evidence_ref, str) or not _standalone_evidence_resolves(
            project_path, evidence_ref
        ):
            finding(
                findings,
                "WARNING",
                INTERFACE_CONSUMED_UNUSED,
                f"interface {interface_id!r} declares a consumption with no `evidence_ref` "
                "resolving to this project's own use of the edge; record the repository-relative "
                "path where the project uses it",
                project_id,
            )
        entry = item.get("entry")
        if mechanism != "consume" or not isinstance(entry, str):
            continue
        retained = _retained_copy_digest(project_path, entry)
        if retained is not None and retained != item.get("digest"):
            finding(
                findings,
                interface_finding_level(INTERFACE_VENDORED_WITHOUT_DIGEST, escalation),
                INTERFACE_VENDORED_WITHOUT_DIGEST,
                f"interface {interface_id!r} keeps a byte copy at {entry!r} of provider "
                f"{provider!r} whose declared digest does not pin those bytes; declare the "
                f"sha256 of the retained copy, which is {retained}, or consume the published "
                "artifact by version",
                project_id,
            )


def report_unverified_exempt_capabilities(
    project_id: str,
    findings: list[Finding],
    registry: "CapabilityRegistry | None",
    provider_verified: Any = None,
) -> None:
    """WARNING `CAPABILITY_PROVIDER_UNVERIFIED` on the provider that owns an exemption.

    A registry entry is exempt because its provider declares no verified
    capability record, so without a reporter the exemption silently absorbs
    every consumer and nobody owns the next action. The finding is attributed
    to the provider, which is the one repository that can end it by recording
    verified evidence, and the resolved provider is named in the message.

    Read-only in all three directions the specification names: no posture is
    changed, no consumer is charged for an unfrozen capability, and the
    existing exemption semantics are untouched. A capability whose provider
    declares verified evidence stops being reported here, and a capability
    that binds to no provider is left to `CAPABILITY_ADOPTION_UNRESOLVED`.
    """
    if registry is None:
        return
    for capability_id in sorted(registry.capabilities):
        entry = registry.capabilities[capability_id]
        if not isinstance(entry, dict) or entry.get("posture") != "exempt":
            continue
        provider = _resolved_provider(entry)
        if provider is None or provider != project_id:
            continue
        if callable(provider_verified) and provider_verified(provider, capability_id):
            continue
        reason = str(entry.get("reason") or "no reason recorded")
        finding(
            findings,
            "WARNING",
            CAPABILITY_PROVIDER_UNVERIFIED,
            f"capability {capability_id!r} is exempt ({reason}) and provider {provider!r} "
            "declares no `verified` capability record for it; the exemption lifts when the "
            "provider records verified evidence with a resolving evidence_ref",
            project_id,
        )


def validate_release_evidence(release_evidence: Any, project_path: Path, findings: list[Finding], project_id: str) -> None:
    """Audit a project's optional release evidence record.

    Release evidence is a producer-neutral record of source revision, version,
    toolchain, artifacts, digests, SBOM, provenance, checks, and publication.
    """
    if not isinstance(release_evidence, dict):
        finding(findings, "ERROR", "RELEASE_EVIDENCE_SHAPE", "release_evidence must be an object", project_id)
        return
    seen: set[str] = set()
    for field, entry in release_evidence.items():
        if not isinstance(field, str):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_FIELD", f"release evidence field must be a string, got {type(field).__name__}", project_id)
            continue
        if field in seen:
            finding(findings, "WARNING", "RELEASE_EVIDENCE_DUPLICATE", f"release evidence field repeated: {field}", project_id)
        seen.add(field)
        if field not in RELEASE_EVIDENCE_FIELDS:
            finding(findings, "WARNING", "RELEASE_EVIDENCE_FIELD", f"unknown release evidence field: {field}", project_id)
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_SHAPE", f"release evidence {field!r} must be an object", project_id)
            continue
        state = entry.get("state")
        if state not in RELEASE_EVIDENCE_STATES:
            finding(findings, "ERROR", "RELEASE_EVIDENCE_STATE", f"release evidence {field!r} has invalid state: {state!r}", project_id)
            continue
        ref = entry.get("evidence_ref")
        if ref is not None and not isinstance(ref, str):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_REF", f"release evidence {field!r} evidence_ref must be a string when present", project_id)
            ref = None
        if state in {"configured", "verified"} and isinstance(ref, str) and ref and not (project_path / ref).is_file():
            level = "ERROR" if state == "verified" else "WARNING"
            code = "RELEASE_EVIDENCE_STALE" if state == "verified" else "RELEASE_EVIDENCE_PENDING"
            finding(findings, level, code, f"release evidence {field!r} evidence_ref missing: {ref}", project_id)


def validate_deployment_evidence(deployment: Any, declaration: dict[str, Any], project_path: Path, findings: list[Finding], project_id: str) -> None:
    """For projects that opt in to deployment, require complete release evidence.

    A deployable project must carry a release_evidence block whose required
    fields (revision, artifacts, digests, provenance, checks, publication) are
    either verified or configured with a present evidence_ref. Anything else
    is an ERROR that blocks the deployable gate; metadata alone is never
    sufficient to flip deployment.deployable to true.
    """
    if not isinstance(deployment, dict) or not deployment.get("deployable"):
        return
    declaration_release = declaration.get("release_evidence")
    if not isinstance(declaration_release, dict):
        finding(findings, "ERROR", "RELEASE_EVIDENCE_REQUIRED", "deployable project must declare release_evidence", project_id)
        return
    for field in DEPLOYABLE_RELEASE_REQUIRED_FIELDS:
        entry = declaration_release.get(field)
        if not isinstance(entry, dict):
            finding(findings, "ERROR", "RELEASE_EVIDENCE_INSUFFICIENT", f"deployable project must declare release_evidence.{field}", project_id)
            continue
        state = entry.get("state")
        if state == "verified":
            continue
        if state == "configured":
            ref = entry.get("evidence_ref")
            if isinstance(ref, str) and ref and (project_path / ref).is_file():
                continue
            finding(findings, "ERROR", "RELEASE_EVIDENCE_INSUFFICIENT", f"deployable project release_evidence.{field} configured but evidence_ref missing: {ref!r}", project_id)
            continue
        finding(findings, "ERROR", "RELEASE_EVIDENCE_INSUFFICIENT", f"deployable project release_evidence.{field} state is {state!r}; verified or configured+ref is required", project_id)


def load_promotion_ledger(governance_root: Path, findings: list[Finding]) -> dict[str, dict[str, Any]]:
    """Load the deployment-promo ledger if present, with tamper detection.

    Returns a map of project_id -> latest promote entry. Missing ledger
    returns an empty map (no error). Tampered entries produce a single
    `PROMOTIONS_LEDGER_TAMPERED` ERROR on the findings list; the function
    still returns the on-disk entries so the audit can keep reporting.
    """
    try:
        import promote_project  # local import to keep this module standalone
    except ImportError:
        return {}
    path = promote_project.ledger_path(governance_root)
    sidecar = promote_project.ledger_sidecar_path(governance_root)
    if not path.is_file():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
        entries = payload.get("entries", [])
    except (OSError, json.JSONDecodeError) as exc:
        finding(findings, "ERROR", "PROMOTIONS_LEDGER_TAMPERED", f"cannot parse ledger: {exc}")
        return {}
    if sidecar.is_file():
        try:
            expected = sidecar.read_text(encoding="utf-8").strip()
            actual = promote_project.compute_ledger_hash(entries)
        except OSError:
            expected = ""
            actual = ""
        if expected and expected != actual:
            finding(findings, "ERROR", "PROMOTIONS_LEDGER_TAMPERED", f"ledger entries do not match recorded sidecar at {sidecar}")
    latest: dict[str, dict[str, Any]] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        project_id = entry.get("project_id")
        if not isinstance(project_id, str):
            continue
        if entry.get("kind") == "promote":
            latest[project_id] = entry
    return latest


def validate_deployment_promotion(deployment: Any, project_path: Path, project_id: str, governance_root: Path | None, findings: list[Finding]) -> None:
    """For projects with `deployable: true`, the audit must find a matching
    promotion ledger entry; a stale entry is a WARNING so the project still
    ships while the operator is asked to re-run promotion.
    """
    if not isinstance(deployment, dict) or not deployment.get("deployable"):
        return
    if governance_root is None:
        return
    ledger = load_promotion_ledger(governance_root, findings)
    entry = ledger.get(project_id)
    if entry is None:
        finding(findings, "ERROR", "DEPLOYMENT_PROMOTION", "deployable project has no matching entry in deployment/promotions.json", project_id)
        return
    recorded_revision = entry.get("revision")
    head_file = project_path / ".git" / "HEAD"
    if not head_file.is_file():
        return
    head = head_file.read_text(encoding="utf-8").strip()
    if head.startswith("ref:"):
        ref_name = head.split(":", 1)[1].strip()
        ref_path = project_path / ".git" / ref_name
        head = ref_path.read_text(encoding="utf-8").strip() if ref_path.is_file() else head
    if isinstance(recorded_revision, str) and head and recorded_revision != head and recorded_revision != "unknown":
        finding(findings, "WARNING", "DEPLOYMENT_PROMOTION_STALE", f"promoted revision {recorded_revision!r} differs from project HEAD {head!r}; re-promote at HEAD with scripts/promote_project.py {project_id} --evidence-ref <ref>", project_id)


def validate_product_quality(declaration: dict[str, Any], project_path: Path, findings: list[Finding], project_id: str) -> None:
    """Run the product-code quality checker on a project that opts in.

    The checker is imported in-process. The audit translates its findings into
    the existing Finding vocabulary: FAIL becomes ERROR, REVIEW_REQUIRED
    becomes WARNING, PASS produces no finding.
    """
    import product_code_quality  # local import to keep workspace_check importable without the checker
    import vocabulary as vocabulary_module  # local import for the same reason
    try:
        policy = product_code_quality.load_policy(declaration)
    except (product_code_quality.ConfigError, vocabulary_module.VocabularyError) as exc:
        finding(findings, "WARNING", "QUALITY_POLICY", f"product quality policy invalid: {exc}", project_id)
        return
    try:
        report = product_code_quality.evaluate(policy.with_root(project_path))
    except product_code_quality.ConfigError as exc:
        finding(findings, "WARNING", "QUALITY_POLICY", f"product quality check cannot run: {exc}", project_id)
        return
    for note in report.notes:
        if report.status == "REVIEW_REQUIRED":
            finding(findings, "WARNING", "QUALITY_REVIEW_REQUIRED", note, project_id)
        elif report.status == "FAIL":
            finding(findings, "ERROR", "QUALITY_FAILURE", note, project_id)
    for item in report.findings:
        level = "ERROR" if item.level == "ERROR" or report.status == "FAIL" else "WARNING"
        code = "QUALITY_TEST_CODE" if item.rule_id == "product-test-code" else "QUALITY_PLACEHOLDER"
        message = f"{item.file}:{item.line} {item.marker}".strip()
        finding(findings, level, code, message, project_id)


def promoted_project_ids(governance_root: Path | None = None) -> set[str]:
    """Project ids that hold a `promote` entry in the promotion ledger.

    A finding-free read on purpose: `load_promotion_ledger` owns the ledger's
    integrity findings and `validate_deployment_promotion` owns the staleness
    warning, so this reader exists only to answer the `production-evidence`
    surface question without adding a second stream of the same codes. An
    unreadable ledger yields an empty set, which reads as "no promotion
    evidence" rather than as a verified claim.
    """
    root = Path(governance_root) if governance_root is not None else _default_governance_root()
    path = root / "deployment" / "promotions.json"
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return set()
    entries = document.get("entries") if isinstance(document, dict) else None
    if not isinstance(entries, list):
        return set()
    return {
        str(entry.get("project_id"))
        for entry in entries
        if isinstance(entry, dict)
        and entry.get("kind") == "promote"
        and isinstance(entry.get("project_id"), str)
    }


def validate_readme_claims(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    governance_root: Path | None = None,
    coverage_gaps: list[tuple[str, str, int, str, str]] | None = None,
    promoted_projects: set[str] | None = None,
) -> None:
    """Verify this repository's README status claims against its file tree.

    Invoked for every registered project, triggered by claim presence and by
    nothing else. There is no `readme_claims` block, no `mode`, no opt-out and
    no per-repository exemption: a check that can be switched off is not a
    check (`design.md` §10, ruling 10). Only 2 of 81 declarations carry a
    `quality` block, so gating on that block would leave 79 repositories
    unexamined — which is why this is a sibling of `validate_product_quality`
    and not a branch inside it.

    Two output streams, by design:

    * a contradicted surface is a per-repository ERROR, one finding per
      surface, naming the README line, the surface, and the evidence path;
    * an unscoped statement is NOT a per-repository finding. It is appended to
      `coverage_gaps`, which the caller emits once for the whole run as the
      aggregate `QUALITY_README_CLAIM_UNSCOPED` CHECKER-COVERAGE metric.
    """
    import product_code_quality  # local import: keeps workspace_check importable without the checker

    if README_CLAIMS_UNSUPPORTED_FIELD in declaration:
        # Ruling 10: canonical behaviour applies anyway, and the unsupported
        # field is reported as a WARNING naming it. This is the whole refusal:
        # there is no branch here that reads the value.
        finding(
            findings,
            "WARNING",
            "QUALITY_REVIEW_REQUIRED",
            f"declaration carries an unsupported {README_CLAIMS_UNSUPPORTED_FIELD!r} field; "
            "README claims are verified for every project, honour no mode and cannot be switched off",
            project_id,
        )
    try:
        report = product_code_quality.evaluate_claims(
            project_path,
            declaration,
            governance_root=governance_root,
            manifest_read=read_manifest_dependencies,
            promoted_projects=promoted_projects,
        )
    except (product_code_quality.ConfigError, product_code_quality.VocabularyError) as exc:
        finding(
            findings,
            "WARNING",
            "QUALITY_REVIEW_REQUIRED",
            f"README claim check cannot run for {project_id!r}: {exc}",
            project_id,
        )
        return
    for item in report.contradicted:
        finding(
            findings,
            "ERROR",
            QUALITY_README_CLAIM_CONTRADICTED,
            item.remediation,
            project_id,
        )
    if coverage_gaps is not None:
        for gap in report.gaps:
            coverage_gaps.append((gap.project, gap.file, gap.line, gap.reason, gap.statement))


# Patterns used by the bounded static scan in `validate_standalone`.
# The scan only inspects the listed files, never globs, and never
# executes the project. Detected siblings are recorded as a
# STANDALONE_DETECTED_SIBLING warning but never override the declared
# status — the declaration is the source of truth.
_SIBLING_PATH_PATTERN = re.compile(r"((?:\.\./)+)([A-Za-z0-9_.-]+)")


def _scan_cargo_toml_for_siblings(path: Path) -> list[str]:
    text: str
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if "path" not in stripped:
            continue
        match = _SIBLING_PATH_PATTERN.search(stripped)
        if match:
            found.append(match.group(2))
    return found


def _scan_package_json_for_siblings(path: Path) -> list[str]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return []
    found: list[str] = []
    workspaces = payload.get("workspaces") if isinstance(payload, dict) else None
    candidates: list[Any]
    if isinstance(workspaces, list):
        candidates = workspaces
    elif isinstance(workspaces, dict):
        packages = workspaces.get("packages")
        candidates = packages if isinstance(packages, list) else []
    else:
        candidates = []
    for entry in candidates:
        if not isinstance(entry, str):
            continue
        match = _SIBLING_PATH_PATTERN.search(entry)
        if match:
            found.append(match.group(2))
    return found


def _scan_pyproject_toml_for_siblings(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    in_target = False
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            in_target = line in ("[tool.uv.sources]", "[tool.poetry.sources]")
            continue
        if not in_target:
            continue
        match = _SIBLING_PATH_PATTERN.search(line)
        if match:
            found.append(match.group(2))
    return found


def _scan_go_mod_for_siblings(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("replace "):
            continue
        match = _SIBLING_PATH_PATTERN.search(stripped)
        if match:
            found.append(match.group(2))
    return found


_BOUNDED_STATIC_SCAN_TARGETS: tuple[tuple[str, Any], ...] = (
    ("Cargo.toml", _scan_cargo_toml_for_siblings),
    ("package.json", _scan_package_json_for_siblings),
    ("pyproject.toml", _scan_pyproject_toml_for_siblings),
    ("go.mod", _scan_go_mod_for_siblings),
)


def _bounded_static_scan_for_siblings(project_path: Path) -> list[tuple[str, str]]:
    """Bounded static scan for sibling source references.

    Returns a list of `(file, sibling_id)` pairs. The scan is bounded to
    a fixed set of manifest file names, never executes the project, and
    never follows paths. Detected siblings are advisory only.
    """
    found: list[tuple[str, str]] = []
    for relative, scanner in _BOUNDED_STATIC_SCAN_TARGETS:
        path = project_path / relative
        if not path.is_file():
            continue
        try:
            siblings = scanner(path)
        except Exception:  # noqa: BLE001 - scanner failures are advisory
            continue
        for sibling in siblings:
            found.append((relative, sibling))
    return found


def _is_absolute_path(value: str) -> bool:
    """Return True when `value` is an absolute machine-specific path.

    Absolute Unix paths, Windows paths with a drive letter, and
    Windows UNC paths are all rejected. Backslash is the only
    indicator we use for Windows because the schema pattern
    forbids forward slashes at the start.
    """
    if not isinstance(value, str) or not value:
        return False
    if value.startswith("/"):
        return True
    if re.match(r"^[A-Za-z]:[\\/]", value):
        return True
    if value.startswith("\\\\"):
        return True
    return False


def _has_path_escape(value: str) -> bool:
    """Return True when `value` contains a `..` segment that escapes the root."""
    if not isinstance(value, str) or not value:
        return False
    parts = value.replace("\\", "/").split("/")
    return any(part == ".." for part in parts)


def _redact_secret_value(value: str, secret_words: "re.Pattern[str]") -> str:
    """Redact a value when it contains a secret keyword followed by a value."""
    if not isinstance(value, str) or not secret_words.search(value):
        return value
    return secret_words.sub("[redacted]", value)


def _standalone_evidence_resolves(project_path: Path, evidence_ref: str) -> bool:
    """Return True when the repository-relative evidence ref resolves to a file under the project root."""
    if _is_absolute_path(evidence_ref) or _has_path_escape(evidence_ref):
        return False
    try:
        resolved = (project_path / evidence_ref).resolve(strict=True)
    except (OSError, RuntimeError):
        return False
    try:
        resolved.relative_to(project_path.resolve(strict=True))
    except ValueError:
        return False
    return True


def validate_standalone(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    governance_root: Path | None = None,
) -> None:
    """Audit a project's optional standalone and integrations declarations.

    The validator is the single source of truth for the
    standalone-project-boundary capability. It is invoked only when
    `declaration.standalone` or `declaration.integrations` is present,
    so existing declarations stay unchanged. The audit never executes
    project commands and never rewrites declared values.
    """
    import vocabulary as vocabulary_module

    try:
        canonical_statuses = frozenset(
            vocabulary_module.load_vocabulary_for_governance_root(governance_root or _default_governance_root()).get(
                "standalone_statuses",
                frozenset(),
            )
        )
    except vocabulary_module.VocabularyError as exc:
        finding(findings, "ERROR", STANDALONE_SHAPE, f"vocabulary unavailable for standalone audit: {exc}", project_id)
        return
    secret_words = secret_words_for(governance_root)

    standalone = declaration.get("standalone")
    if standalone is None:
        finding(findings, "WARNING", STANDALONE_BLOCKED, "standalone block absent; cannot classify", project_id)
    else:
        if not isinstance(standalone, dict):
            finding(findings, "ERROR", STANDALONE_SHAPE, "standalone must be an object", project_id)
        else:
            _validate_standalone_shape(standalone, canonical_statuses, findings, project_id, secret_words)
            _validate_standalone_paths(standalone, project_path, findings, project_id, secret_words)
            _validate_standalone_evidence(standalone, project_path, findings, project_id)

    integrations = declaration.get("integrations")
    if integrations is not None:
        _validate_integrations(integrations, project_path, findings, project_id, secret_words)

    detected = _bounded_static_scan_for_siblings(project_path)
    for file_name, sibling in detected:
        finding(
            findings,
            "WARNING",
            STANDALONE_DETECTED_SIBLING,
            f"static scan found '../{sibling}' in {file_name}; declaration is the source of truth",
            project_id,
        )


def _validate_standalone_shape(
    standalone: dict[str, Any],
    canonical_statuses: frozenset[str],
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> None:
    required = ("status", "local_defaults", "required_external_services", "required_sibling_projects")
    for key in required:
        if key not in standalone:
            finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.{key} is required", project_id)
    status = standalone.get("status")
    if status is not None and (not isinstance(status, str) or status not in canonical_statuses):
        finding(
            findings,
            "ERROR",
            STANDALONE_STATUS_INVALID,
            f"standalone.status {status!r} is not one of the canonical values: {sorted(canonical_statuses)}",
            project_id,
        )
    elif status == "blocked":
        # The operator explicitly declared the project cannot be classified
        # from available evidence; surface a warning so the audit does not
        # silently pass.
        finding(findings, "WARNING", STANDALONE_BLOCKED, "standalone.status is 'blocked'; cannot classify", project_id)
    for key in ("local_defaults", "required_external_services", "required_sibling_projects"):
        value = standalone.get(key)
        if value is None:
            continue
        if not isinstance(value, list) or any(not isinstance(item, str) or not item for item in value):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.{key} must be an array of non-empty strings", project_id)
            continue
        for entry in value:
            redacted = _redact_secret_value(entry, secret_words)
            if redacted != entry:
                finding(findings, "ERROR", STANDALONE_SECRET_VALUE, f"standalone.{key} contains a secret-shaped value: [redacted]", project_id)


def _validate_standalone_paths(
    standalone: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> None:
    """Reject absolute paths and `..` escapes; record required siblings and services."""
    for key in ("local_defaults", "required_external_services", "required_sibling_projects"):
        value = standalone.get(key)
        if not isinstance(value, list):
            continue
        for entry in value:
            if not isinstance(entry, str) or not entry:
                continue
            if _is_absolute_path(entry):
                finding(findings, "ERROR", STANDALONE_ABSOLUTE_PATH, f"standalone.{key} contains an absolute path: [redacted]", project_id)
            elif _has_path_escape(entry):
                finding(findings, "ERROR", STANDALONE_PATH_ESCAPE, f"standalone.{key} contains a '..' segment: {entry!r}", project_id)
    siblings = standalone.get("required_sibling_projects")
    if isinstance(siblings, list):
        for entry in siblings:
            if isinstance(entry, str) and entry and not _is_absolute_path(entry) and not _has_path_escape(entry):
                finding(findings, "ERROR", STANDALONE_REQUIRED_SIBLING, f"required sibling project declared: {entry}", project_id)
    services = standalone.get("required_external_services")
    if isinstance(services, list):
        for entry in services:
            if isinstance(entry, str) and entry and not _is_absolute_path(entry) and not _has_path_escape(entry):
                finding(findings, "WARNING", STANDALONE_REQUIRED_SERVICE, f"required external service declared: {entry}", project_id)


def _validate_standalone_evidence(
    standalone: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
) -> None:
    evidence_ref = standalone.get("evidence_ref")
    if evidence_ref is not None and not isinstance(evidence_ref, str):
        finding(findings, "ERROR", STANDALONE_SHAPE, "standalone.evidence_ref must be a string when present", project_id)
        return
    if evidence_ref is None:
        state = standalone.get("evidence_state", "declared")
        if state in ("configured", "verified"):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.evidence_state is {state!r} but evidence_ref is missing", project_id)
        return
    if _is_absolute_path(evidence_ref):
        finding(findings, "ERROR", STANDALONE_ABSOLUTE_PATH, f"standalone.evidence_ref is absolute: [redacted]", project_id)
        return
    if _has_path_escape(evidence_ref):
        finding(findings, "ERROR", STANDALONE_PATH_ESCAPE, f"standalone.evidence_ref escapes the project root: {evidence_ref!r}", project_id)
        return
    if not (project_path / evidence_ref).is_file():
        finding(findings, "WARNING", STANDALONE_EVIDENCE_MISSING, f"standalone.evidence_ref is missing on disk: {evidence_ref!r}", project_id)
        return
    state = standalone.get("evidence_state", "declared")
    if state not in {"declared", "configured", "verified", "unverified"}:
        finding(findings, "ERROR", STANDALONE_SHAPE, f"standalone.evidence_state is invalid: {state!r}", project_id)


def _validate_integrations(
    integrations: Any,
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    secret_words: "re.Pattern[str]",
) -> None:
    if not isinstance(integrations, list):
        finding(findings, "ERROR", STANDALONE_SHAPE, "integrations must be an array", project_id)
        return
    seen_ids: set[str] = set()
    for index, entry in enumerate(integrations):
        if not isinstance(entry, dict):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}] must be an object", project_id)
            continue
        entry_id = entry.get("id")
        if not isinstance(entry_id, str) or not PROJECT_ID.fullmatch(entry_id):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].id must be a kebab-case identifier", project_id)
        elif entry_id in seen_ids:
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].id {entry_id!r} is duplicated", project_id)
        else:
            seen_ids.add(entry_id)
        required = entry.get("required")
        if not isinstance(required, bool):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].required must be a boolean", project_id)
            required = False
        evidence_ref = entry.get("evidence_ref")
        if evidence_ref is not None and not isinstance(evidence_ref, str):
            finding(findings, "ERROR", STANDALONE_SHAPE, f"integrations[{index}].evidence_ref must be a string when present", project_id)
            evidence_ref = None
        if isinstance(evidence_ref, str):
            redacted = _redact_secret_value(evidence_ref, secret_words)
            if redacted != evidence_ref:
                finding(findings, "ERROR", STANDALONE_SECRET_VALUE, f"integrations[{index}].evidence_ref contains a secret-shaped value: [redacted]", project_id)
            if _is_absolute_path(evidence_ref):
                finding(findings, "ERROR", STANDALONE_ABSOLUTE_PATH, f"integrations[{index}].evidence_ref is absolute: [redacted]", project_id)
            elif _has_path_escape(evidence_ref):
                finding(findings, "ERROR", STANDALONE_PATH_ESCAPE, f"integrations[{index}].evidence_ref escapes the project root: {evidence_ref!r}", project_id)
        if not required:
            finding(findings, "WARNING", STANDALONE_OPTIONAL, f"optional integration declared: {entry_id!r}", project_id)
            continue
        if not evidence_ref:
            finding(findings, "WARNING", STANDALONE_INTEGRATION_PENDING, f"required integration missing evidence_ref: {entry_id!r}", project_id)
            continue
        if not _standalone_evidence_resolves(project_path, evidence_ref):
            finding(findings, "WARNING", STANDALONE_INTEGRATION_PENDING, f"required integration evidence_ref is missing or escapes the project root: {entry_id!r}", project_id)


def classify_standalone(
    declaration: dict[str, Any],
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    governance_root: Path | None = None,
) -> dict[str, Any]:
    """Return a normalized standalone observation for a project.

    This is the public surface consumed by `forge_governance_adapter`.
    It runs the same shape/path/secret checks as `validate_standalone`
    so the adapter never has to shell out, and records the canonical
    status the audit derived plus the visible findings, integration
    list, and an evidence-ref resolution flag.

    The function never raises; it always returns a dict with at minimum
    `standalone_status` (one of the six canonical values) and a
    `standalone_findings` list. On audit failure the returned status
    is `blocked` and the findings carry the failure reason.
    """
    import vocabulary as vocabulary_module

    observed: dict[str, Any] = {
        "standalone_status": "blocked",
        "standalone_observed_revision": None,
        "standalone_integrations": [],
        "standalone_findings": [],
    }
    try:
        canonical_statuses = frozenset(
            vocabulary_module.load_vocabulary_for_governance_root(governance_root or _default_governance_root()).get(
                "standalone_statuses",
                frozenset(),
            )
        )
    except vocabulary_module.VocabularyError as exc:
        observed["standalone_status"] = "blocked"
        observed["standalone_findings"] = [
            {"level": "ERROR", "code": STANDALONE_SHAPE, "message": f"vocabulary unavailable: {exc}"},
        ]
        return observed

    internal: list[Finding] = []
    try:
        validate_standalone(declaration, project_path, internal, project_id, governance_root=governance_root)
    except Exception as exc:  # noqa: BLE001 - adapter must never raise
        internal.append(Finding("ERROR", STANDALONE_SHAPE, f"standalone audit crashed: {exc}", project_id))

    for item in internal:
        observed["standalone_findings"].append(
            {"level": item.level, "code": item.code, "message": item.message}
        )

    standalone = declaration.get("standalone")
    declared_status: str | None = None
    if isinstance(standalone, dict) and isinstance(standalone.get("status"), str):
        declared_status = standalone["status"]
        if declared_status in canonical_statuses:
            observed["standalone_status"] = declared_status

    siblings = standalone.get("required_sibling_projects") if isinstance(standalone, dict) else None
    services = standalone.get("required_external_services") if isinstance(standalone, dict) else None
    if isinstance(siblings, list) and siblings:
        observed["standalone_status"] = "workspace-dependent"
    elif isinstance(services, list) and services:
        observed["standalone_status"] = "service-dependent"

    integrations = declaration.get("integrations")
    if isinstance(integrations, list):
        for entry in integrations:
            if not isinstance(entry, dict):
                continue
            observed["standalone_integrations"].append(
                {
                    "id": entry.get("id"),
                    "required": entry.get("required", False),
                    "evidence_ref": entry.get("evidence_ref"),
                }
            )
    if observed["standalone_status"] == "standalone" and observed["standalone_integrations"]:
        observed["standalone_status"] = "standalone-with-optional-integrations"
    return observed


def validate_spec_purposes(project_path: Path, findings: list[Finding], project_id: str) -> None:
    """Report a spec whose `## Purpose` still holds the archiver's TBD marker.

    Archiving a change writes `TBD - created by archiving change <name>.` into the
    `## Purpose` of every spec file that change produced, so a spec whose purpose
    is literally the string `TBD` reads as a finished spec. The level is fixed at
    ERROR by ruling 1: the marker is an unfinished statement presented as a
    settled one, and a warning nobody must clear is how the 1,234 of them
    accumulated in the first place.

    The finding names the spec file relative to its repository and the change the
    marker records, so the remediation is "resolve this spec from this change"
    rather than "look at a list of them". The change name is whatever the marker
    says, in either spelling: the archiver has emitted both the bare change name
    and a date-prefixed one, and this rule does not resolve the archive —
    `scripts/spec_purpose_backfill.py` does that, and guessing here would report a
    change the spec did not come from.

    A repository with no `openspec/specs` tree is skipped, and the rule is
    deliberately not gated on `profile_requirements`: a descriptive rule must
    not become a requirements driver.
    """
    specs_root = project_path / SPEC_PURPOSE_SPECS_DIR
    if not specs_root.is_dir():
        return
    for spec in sorted(specs_root.rglob("spec.md")):
        if not spec.is_file() or spec.is_symlink():
            continue
        try:
            text = spec.read_text(encoding="utf-8")
        except OSError as exc:
            finding(findings, "WARNING", "SPEC_UNREADABLE",
                    f"cannot read spec file: {exc}", project_id)
            continue
        match = SPEC_PURPOSE_PLACEHOLDER_RE.search(text)
        if match is None:
            continue
        marker = match.group(1).strip()
        change = SPEC_PURPOSE_CHANGE_RE.match(marker)
        origin = f" archived from change {change.group('name')!r}" if change else ""
        finding(
            findings,
            SPEC_PURPOSE_PLACEHOLDER_LEVEL,
            SPEC_PURPOSE_PLACEHOLDER,
            f"{spec.relative_to(project_path).as_posix()} holds the archived-change Purpose "
            f"marker {marker!r}{origin}; a spec's purpose is real prose, not the archiver's TBD",
            project_id,
        )


def validate_declared_verification_is_bound(
    project_path: Path, findings: list[Finding], project_id: str
) -> None:
    """Report a declared `verification.command` that no Gate actually runs.

    The existing `VERIFICATION_COMMAND` rule asks the opposite question — is a
    command *declared*? — and all 81 repositories satisfy it, so it catches none
    of the 70 whose `.ai-gate/gate.yaml` binds no command and whose gate
    therefore executes nothing. Ruling D18 calls that documentation: a verifier
    that no gate runs does not gate.

    Both halves must be present for a binding to be real, and this rule checks
    both, because the manifest parser refuses one without the other:

    * `checks:` must select `declared-verification` as `true` — a selection of
      `false` binds nothing, and the generator refuses to override it;
    * `commands:` must bind that concern.

    A repository with no `.ai-gate/gate.yaml` is reported too, and distinctly:
    there is no manifest to bind into, which is a different defect from a
    manifest that declines to bind. The message says which, so the remediation
    is not "add a command" for a repository that has nowhere to put one.

    The reading is line-based on purpose. The concern id is a fixed constant
    written by `scripts/bind_verification_command.py`, and the rule reads the
    same constant, so the two cannot disagree about what "bound" means. A full
    YAML parse is not used because a manifest carrying an unknown top-level
    field fails to parse at all — 11 of the 76 in the fleet do — and a rule
    that crashed on those would be blind exactly where the state is worst.
    """
    manifest = project_path / GATE_MANIFEST_RELATIVE
    command = _declared_verification_command(project_path)
    if command is None:
        return

    if not manifest.is_file():
        native = project_path / "gate.toml"
        where = (
            f"uses a native {native.name}, which this rule does not read; its "
            f"declared command must be bound there"
            if native.is_file()
            else f"has no {GATE_MANIFEST_RELATIVE} to bind it into"
        )
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {where}; "
            "the command is documentation, not a gate",
            project_id,
        )
        return

    try:
        lines = manifest.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command but {GATE_MANIFEST_RELATIVE} cannot be read: {exc}",
            project_id,
        )
        return

    checks = _yaml_block(lines, "checks")
    commands = _yaml_block(lines, "commands")
    if checks is None:
        # No selection block at all: nothing could be bound, whatever
        # `commands:` says.
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {GATE_MANIFEST_RELATIVE} "
            "selects no `checks:` block, so a bound command could never run",
            project_id,
        )
        return

    selected = _yaml_entry(checks, DECLARED_VERIFICATION_CONCERN)
    if selected != "true":
        detail = "does not select it" if selected is None else f"selects it as {selected!r}, not `true`"
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {GATE_MANIFEST_RELATIVE} "
            f"{detail} under `checks:`",
            project_id,
        )
        return

    if _yaml_entry(commands or [], DECLARED_VERIFICATION_CONCERN) is None:
        finding(
            findings,
            QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            f"declares verification.command {command!r} but {GATE_MANIFEST_RELATIVE} "
            f"binds no `commands.{DECLARED_VERIFICATION_CONCERN}`; the gate runs nothing",
            project_id,
        )


def _declared_verification_command(project_path: Path) -> str | None:
    """The repository's own declared command, or None when it declares none.

    Reads the same field the generator reads, with the same refusals: an
    unparseable or absent declaration yields None, and this rule then has
    nothing to report. A rule that reported "declares a command it does not
    declare" would be the rule's own bug, not the repository's.
    """
    declaration_path = project_path / ".project.json"
    if not declaration_path.is_file():
        return None
    try:
        data = json.loads(declaration_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(data, dict):
        return None
    verification = data.get("verification")
    if not isinstance(verification, dict):
        return None
    command = verification.get("command")
    if not isinstance(command, str) or not command.strip():
        return None
    return command.strip()


def _yaml_block(lines: list[str], key: str) -> list[str] | None:
    """The indented lines of a top-level `key:` block, or None if absent."""
    try:
        start = next(i for i, line in enumerate(lines) if line.startswith(f"{key}:"))
    except StopIteration:
        return None
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if line.strip() and not line[0].isspace():
            end = index
            break
    return lines[start + 1 : end]


def _yaml_entry(block: list[str], key: str) -> str | None:
    """One `  <key>: <value>` entry's value inside a block, or None."""
    needle = f"  {key}:"
    for line in block:
        if line.startswith(needle):
            return line[len(needle) :].strip()
    return None


def unarchived_changes(project_path: Path) -> list[str]:
    """The repository's active OpenSpec change names, sorted.

    One walk, two rules: `validate_change_names` names the directories here
    that are not valid change ids, and `validate_new_change_with_gate_debt`
    counts the same set as new work in progress. A trigger the naming rule
    ignores would be a trigger nobody can fix by renaming a directory.

    `archive/` is excluded because an archived change is finished work, and a
    dot-prefixed directory because OpenSpec keeps its own state there.
    """
    changes = project_path / "openspec" / "changes"
    if not changes.is_dir():
        return []
    return [
        child.name
        for child in sorted(changes.iterdir())
        if child.is_dir() and child.name != "archive" and not child.name.startswith(".")
    ]


def validate_change_names(project_path: Path, findings: list[Finding], project_id: str) -> None:
    for name in unarchived_changes(project_path):
        if not CHANGE_ID.fullmatch(name):
            finding(findings, "ERROR", "OPENSPEC_NAME", f"invalid active OpenSpec change name: {name}", project_id)


def validate_openspec_seriality(
    project_path: Path, findings: list[Finding], project_id: str
) -> None:
    """Require one active OpenSpec change and a matching local handoff pointer."""
    if project_id != "workspace-governance":
        return

    changes = unarchived_changes(project_path)
    if len(changes) > 1:
        finding(
            findings,
            "ERROR",
            OPENSPEC_CHANGE_SERIALITY,
            "multiple active OpenSpec changes are not allowed; finish and archive "
            f"the current change before starting another: {', '.join(changes)}",
            project_id,
        )
        return

    handoff_path = project_path / "HANDOFF.md"
    if not handoff_path.exists():
        if changes:
            finding(
                findings,
                "ERROR",
                OPENSPEC_CHANGE_SERIALITY,
                f"HANDOFF.md is missing; current_spec must name the active change {changes[0]!r}",
                project_id,
            )
        return
    try:
        handoff = handoff_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        expected = (
            f"active change {changes[0]!r}"
            if changes
            else "stale current_spec state"
        )
        finding(
            findings,
            "ERROR",
            OPENSPEC_CHANGE_SERIALITY,
            f"HANDOFF.md cannot be read to verify {expected}: {exc}",
            project_id,
        )
        return

    pointers = re.findall(
        r"^[ \t]*current_spec[ \t]*:[ \t]*(.*?)[ \t]*$",
        handoff,
        flags=re.MULTILINE,
    )
    normalized = [value.strip().strip("\"'").strip() for value in pointers]
    if len(changes) == 1:
        expected = changes[0]
        if not normalized:
            problem = "current_spec is missing; it must appear exactly once"
        elif len(normalized) > 1:
            problem = "current_spec appears more than once; it must appear exactly once"
        elif normalized[0] != expected:
            problem = f"current_spec is {normalized[0]!r}; it must name the active change {expected!r}"
        else:
            return
    else:
        if len(normalized) > 1:
            problem = "current_spec appears more than once with no active change"
        elif normalized and normalized[0].lower() not in {"", "none"}:
            problem = f"current_spec {normalized[0]!r} is stale because no change is active"
        else:
            return

    finding(
        findings,
        "ERROR",
        OPENSPEC_CHANGE_SERIALITY,
        f"{problem}; active changes: {', '.join(changes) if changes else 'none'}",
        project_id,
    )


def live_unbound_gate_checks(project_path: Path) -> tuple[tuple[str, ...] | None, str | None]:
    """Read current unbound concerns with the binder's required-check rules.

    This reads manifest declarations only: it does not derive or run commands,
    write the manifest, or update the workspace debt snapshot.
    """
    manifest_path = project_path / ".ai-gate" / "gate.yaml"
    try:
        text = manifest_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return None, f"cannot read {manifest_path.relative_to(project_path)}: {exc}"

    try:
        from bind_gate_commands import (
            PROFILE_DEFAULTS,
            block_entries,
            gate_required,
            read_gate_policy,
            read_manifest,
            top_level_block_bounds,
            unquote_scalar,
        )

        selected, commands, problem = read_manifest(text)
        if problem:
            return None, problem
        policy = read_gate_policy(text)
        if policy.profile not in PROFILE_DEFAULTS:
            return None, f"unsupported gate profile {policy.profile!r}"
        lines = text.splitlines()
        for top_level_key in ("profile", "checks", "commands", "not_applicable"):
            if sum(line.startswith(f"{top_level_key}:") for line in lines) > 1:
                return None, f"duplicate top-level `{top_level_key}:` declaration"
        for block_name in ("checks", "commands", "not_applicable"):
            bounds = top_level_block_bounds(lines, block_name)
            if bounds is None:
                continue
            start, end = bounds
            for line in lines[start + 1:end]:
                stripped = line.strip()
                if not stripped or stripped.startswith("#"):
                    continue
                if (
                    not line.startswith("  ")
                    or line.startswith("   ")
                    or stripped.startswith("-")
                    or ":" not in stripped
                ):
                    return None, f"malformed `{block_name}:` mapping entry: {stripped!r}"
                key, value = stripped.split(":", 1)
                if not key or key != key.strip() or not value.strip():
                    return None, f"malformed `{block_name}:` mapping entry: {stripped!r}"
            entries = block_entries(lines, block_name)
            names = [name for name, _value in entries]
            if len(names) != len(set(names)):
                return None, f"duplicate entry in `{block_name}:` mapping"
            if block_name == "checks" and any(
                value not in {"true", "false", "optional"}
                for _name, value in entries
            ):
                return None, "invalid value in `checks:` mapping"
            if block_name == "not_applicable" and any(
                not unquote_scalar(value).strip() or value.lstrip().startswith("#")
                for _name, value in entries
            ):
                return None, "empty reason in `not_applicable:` mapping"
        if top_level_block_bounds(lines, "checks") is None:
            # Inline empty checks are valid; every other absent checks block
            # is rejected by read_manifest above.
            if not any(line.strip() == "checks: {}" for line in lines):
                return None, "missing `checks:` mapping"
        required = gate_required(selected, policy)

        def has_command(concern: str) -> bool:
            raw = commands.get(concern, "").strip()
            value = unquote_scalar(raw).strip()
            if not raw or raw.startswith("#") or not value:
                return False
            if raw[0] not in {"'", '"'} and value.lower() in {"null", "~"}:
                return False
            return True

        unbound = tuple(sorted(concern for concern in required if not has_command(concern)))
        return unbound, None
    except Exception as exc:  # noqa: BLE001 - unresolved policy must fail closed
        return None, f"cannot resolve the gate manifest: {exc}"


def validate_new_change_with_gate_debt(
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    debt: dict[str, tuple[str, ...]] | None,
    baseline: frozenset[tuple[str, str]] | None,
    exceptions: dict[tuple[str, str], str] | None,
) -> None:
    """Block new work on a repository that still carries gate debt.

    22 repositories record at least one required check the gate reports as
    `UNBOUND`. The record says so; nothing acted on it, so the debt sits still
    while the next feature starts on top of it. This rule is the acting.

    The record remains the source for already-known debt. For a repository
    with active work, the current manifest is also read through the binder's
    required-check parser; an unbound concern missing from the snapshot is a
    live increase and blocks the new change immediately.

    Three ways to *not* be new work, and the second is what keeps the rule a
    ratchet rather than a deadlock:

    - a change named in `GATE_DEBT_CLOSING_CHANGES` is not new work at all: it
      *is* the work that clears the debt, and a rule that blocked it would make
      the debt permanent;
    - a `(project, change)` pair in `GATE_DEBT_BASELINE_FILE` was already open
      when the rule landed. Failing it fails the legacy fleet for the state the
      rule exists to stop growing, and cannot be discharged by any action
      available to the repository. The baseline is a committed list, so this is
      a reviewable one-time grandfathering and not a switch;
    - a pair in `GATE_DEBT_EXCEPTIONS_FILE` is a deliberate decision about new
      work, reported at `WARNING` with its reason quoted so the decision stays
      visible in the run.

    `debt=None` means the debt record could not be read, and `baseline=None`
    that the ratchet could not. The caller has already reported each once as a
    run-level `ERROR`, which is what fails the run; `baseline=None` grandfathers
    nothing, because a ratchet that could not be read grandfathers nothing.
    """
    changes = unarchived_changes(project_path)
    if not changes or debt is None:
        return

    recorded = debt.get(project_id, ())
    current, current_error = live_unbound_gate_checks(project_path)
    increased = (
        tuple(sorted(set(current or ()) - set(recorded)))
        if current_error is None
        else ()
    )
    concerns = tuple(dict.fromkeys((*recorded, *increased)))
    if not concerns and current_error is None:
        return

    closing = ", ".join(sorted(GATE_DEBT_CLOSING_CHANGES))
    unbound = ", ".join(concerns)
    for change in changes:
        if change in GATE_DEBT_CLOSING_CHANGES:
            continue
        if baseline is not None and (project_id, change) in baseline:
            continue
        reason = (exceptions or {}).get((project_id, change))
        if reason is not None:
            finding(
                findings,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT_SATISFIED_LEVEL,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
                f"opens change {change!r} while required check(s) {unbound} are unbound; "
                f"satisfied by a recorded exception in {GATE_DEBT_EXCEPTIONS_FILE}: {reason}",
                project_id,
            )
            continue
        if current_error is not None:
            finding(
                findings,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT_LEVEL,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
                f"opens change {change!r} but the checker could not establish live gate state: {current_error}; "
                f"resolve the manifest before starting new work",
                project_id,
            )
            continue
        if increased:
            finding(
                findings,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT_LEVEL,
                QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
                f"opens change {change!r} while live required check(s) {', '.join(increased)} are unbound "
                f"and absent from gate-debt.json; bind them and regenerate the debt snapshot before starting new work",
                project_id,
            )
            continue
        finding(
            findings,
            QUALITY_NEW_CHANGE_WITH_GATE_DEBT_LEVEL,
            QUALITY_NEW_CHANGE_WITH_GATE_DEBT,
            f"opens change {change!r} while required check(s) {unbound} are unbound; "
            f"bind them under {closing} first, or record a deliberate exception "
            f"in {GATE_DEBT_EXCEPTIONS_FILE} before starting new work",
            project_id,
        )


def validate_handoff_quality(
    project_path: Path,
    findings: list[Finding],
    project_id: str,
    *,
    rules_drift_baseline: frozenset[tuple[str, str, str]] | None = None,
    observed_drift_keys: set[tuple[str, str, str]] | None = None,
    scanned_projects: set[str] | None = None,
) -> None:
    """Surface a scaffold `HANDOFF.md` and gate on rules drift that is not baselined.

    Two distinct surfaces meet here, deliberately:

    - `scripts/rules_drift.py --check` is the **detailed report**. It names every
      clause a repository does not visibly carry, and it never rewrites a project.
    - this function is the **blocking delta**. It reports only a finding whose
      `(project, file, label)` key is absent from `rules-drift-baseline.json`, so a
      repository that documents the shared rules in its own words stays silent and a
      newly introduced gap blocks. A key that is present in the baseline is known
      debt, not a new failure.

    A repository's legitimate escape is the per-file
    `<!-- workspace-governance:rules-customized -->` marker, which
    `rules_drift.OPT_OUT` honours before any clause is evaluated. There is no
    environment variable, no skip flag, and no exemption registry: a check that can
    be switched off is not a check.
    """
    import rules_drift  # local import to keep this module standalone

    if rules_drift.scaffold_handoff(project_path):
        finding(findings, "WARNING", "HANDOFF_SCAFFOLD", "HANDOFF.md still carries the initializer scaffold", project_id)

    if scanned_projects is not None:
        scanned_projects.add(project_id)

    if rules_drift_baseline is None:
        return

    for item in rules_drift.drift(project_path):
        if item["code"] != "RULES_DRIFT":
            # RULES_UNREADABLE is a read failure, not a clause key. It is reported
            # by the detailed report and must never be ratcheted into the baseline.
            continue
        key = (project_id, item["file"], item["label"])
        if observed_drift_keys is not None:
            observed_drift_keys.add(key)
        if key not in rules_drift_baseline:
            finding(
                findings,
                "ERROR",
                "RULES_DRIFT_NEW",
                f"{item['file']} does not carry the {item['label']}, and this finding is not in rules-drift-baseline.json",
                project_id,
            )


def validate_project(
    entry: dict[str, Any],
    workspace_root: Path,
    findings: list[Finding],
    strict: bool = True,
    governance_root: Path | None = None,
    *,
    check_publication: bool = True,
    capability_registry: CapabilityRegistry | None = None,
    provider_paths: dict[str, Path] | None = None,
    adoption_ledger: list[dict[str, Any]] | None = None,
    rules_drift_baseline: frozenset[tuple[str, str, str]] | None = None,
    observed_drift_keys: set[tuple[str, str, str]] | None = None,
    scanned_projects: set[str] | None = None,
    taxonomy_enabled: bool | None = None,
    spec_purpose_enabled: bool | None = None,
    verification_binding_enabled: bool | None = None,
    readme_claim_coverage: list[tuple[str, str, int, str, str]] | None = None,
    promoted_projects: set[str] | None = None,
    gate_debt: dict[str, tuple[str, ...]] | None = None,
    gate_debt_baseline: frozenset[tuple[str, str]] | None = None,
    gate_debt_exceptions: dict[tuple[str, str], str] | None = None,
    interface_index: dict[tuple[str, str], dict[str, Any]] | None = None,
    interface_escalation: dict[str, str] | None = None,
) -> None:
    project_id = str(entry.get("id"))
    project_path = workspace_root / str(entry.get("path"))
    declaration_path = project_path / ".project.json"
    if not declaration_path.exists():
        if strict:
            finding(findings, "ERROR", "DECLARATION_MISSING", "adopted project must contain .project.json", project_id)
        return
    declaration = load_json(declaration_path, findings, "PROJECT_DECLARATION")
    if declaration is None:
        return
    if declaration.get("schema_version") != 1:
        finding(findings, "ERROR", "DECLARATION_SCHEMA", "project declaration schema_version must be 1", project_id)
    if declaration.get("id") != project_id:
        finding(findings, "ERROR", "DECLARATION_ID", f"declaration id must be {project_id!r}", project_id)
    validate_kind(declaration, findings, project_id, governance_root=governance_root)
    if taxonomy_audit_enabled(taxonomy_enabled):
        validate_taxonomy(entry, declaration, findings, project_id, governance_root=governance_root)
    if spec_purpose_audit_enabled(spec_purpose_enabled):
        validate_spec_purposes(project_path, findings, project_id)
    profile = str(entry.get("profile"))
    required, requires_ci, requires_openspec, requires_verification = profile_requirements(profile)
    for relative in required:
        if not (project_path / relative).exists():
            finding(findings, "ERROR", "REQUIRED_FILE", f"missing required file: {relative}", project_id)
    verification = declaration.get("verification")
    if requires_verification and (not isinstance(verification, dict) or not str(verification.get("command", "")).strip()):
        finding(findings, "ERROR", "VERIFICATION_COMMAND", "a real verification command is required", project_id)
    if verification_binding_audit_enabled(verification_binding_enabled):
        validate_declared_verification_is_bound(project_path, findings, project_id)
    if profile != "inventory":
        from repository_metadata import validate_discoverability, validate_publication

        if not (project_path / "LICENSE").is_file():
            finding(findings, "ERROR", "LICENSE_MISSING", "standard project must contain a root LICENSE", project_id)
        validate_discoverability(declaration, project_path, findings, project_id)
        if check_publication:
            validate_publication(declaration, findings, project_id)
    if requires_ci and not any((project_path / candidate).exists() for candidate in ("Jenkinsfile", ".gitlab-ci.yml")) and not (project_path / ".github" / "workflows").is_dir():
        finding(findings, "ERROR", "CI_MISSING", "a CI workflow is required by this profile", project_id)
    if requires_openspec and not (project_path / "openspec").is_dir():
        finding(findings, "ERROR", "OPENSPEC_MISSING", "OpenSpec directory is required by this profile", project_id)
    if requires_openspec:
        validate_change_names(project_path, findings, project_id)
    validate_openspec_seriality(project_path, findings, project_id)
    # Unconditional, unlike the rules above: a repository on any profile that
    # carries gate debt may not open a new change, and `inventory` is the
    # profile most of the debted repositories are on. `None` means the caller
    # did not load the records, so read them from `governance_root` here.
    if gate_debt is None or gate_debt_baseline is None or gate_debt_exceptions is None:
        try:
            gate_debt = load_gate_debt_record(governance_root)
        except GateDebtRecordError:
            gate_debt = None
        try:
            gate_debt_baseline = load_gate_debt_baseline(governance_root)
        except GateDebtBaselineError:
            gate_debt_baseline = None
        try:
            gate_debt_exceptions = load_gate_debt_exceptions(governance_root)
        except GateDebtExceptionsError:
            gate_debt_exceptions = None
    validate_new_change_with_gate_debt(
        project_path,
        findings,
        project_id,
        debt=gate_debt,
        baseline=gate_debt_baseline,
        exceptions=gate_debt_exceptions,
    )
    deployment = declaration.get("deployment")
    if not isinstance(deployment, dict) or not isinstance(deployment.get("deployable"), bool):
        finding(findings, "ERROR", "DEPLOYMENT_DECLARATION", "deployment.deployable must be boolean", project_id)
    else:
        compose = deployment.get("compose_file")
        if compose is not None and (not isinstance(compose, str) or not compose or Path(compose).is_absolute() or ".." in Path(compose).parts):
            finding(findings, "ERROR", "DEPLOYMENT_COMPOSE", "deployment.compose_file must be repository-relative when present", project_id)
        template_type = deployment.get("template_type")
        template_version = deployment.get("template_version")
        rendered_files = deployment.get("rendered_files")
        template_state = deployment.get("template_state")
        if template_type is not None and (not isinstance(template_type, str) or not re.fullmatch(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*", template_type)):
            finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.template_type must be a kebab-case identifier", project_id)
        if template_version is not None and (not isinstance(template_version, int) or isinstance(template_version, bool) or template_version < 1):
            finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.template_version must be a positive integer", project_id)
        if rendered_files is not None:
            if not isinstance(rendered_files, list) or len(set(rendered_files)) != len(rendered_files) or any(not isinstance(item, str) or Path(item).is_absolute() or ".." in Path(item).parts for item in rendered_files):
                finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.rendered_files must contain unique repository-relative paths", project_id)
        if template_state is not None and template_state not in {"configured", "verified", "blocked"}:
            finding(findings, "ERROR", "DEPLOYMENT_TEMPLATE", "deployment.template_state is invalid", project_id)
    if isinstance(deployment, dict) and deployment.get("deployable"):
        compose = deployment.get("compose_file")
        job = deployment.get("jenkins_job")
        if not isinstance(compose, str) or not compose:
            finding(findings, "ERROR", "DEPLOYMENT_COMPOSE", "deployable project must declare compose_file", project_id)
        elif not (project_path / compose).is_file():
            finding(findings, "ERROR", "DEPLOYMENT_COMPOSE", f"declared Compose file is missing: {compose}", project_id)
        if not isinstance(job, str) or not job:
            finding(findings, "ERROR", "DEPLOYMENT_JOB", "deployable project must declare jenkins_job", project_id)
    capabilities = declaration.get("capabilities")
    supported: set[str] | None = None
    if capabilities is not None and governance_root is not None:
        spec = load_profile_spec(governance_root, profile)
        if spec is not None:
            raw = spec.get("supported_capabilities")
            if isinstance(raw, list):
                supported = {item for item in raw if isinstance(item, str)}
    if capabilities is not None:
        validate_capabilities(capabilities, project_path, findings, project_id, supported=supported)
    if capability_registry is not None:
        paths = provider_paths or {}
        resolved = lambda provider, capability: provider_capability_verified(workspace_root, paths, provider, capability)  # noqa: E731
        measure_capability_adoption(
            declaration,
            project_path,
            findings,
            project_id,
            capability_registry,
            provider_verified=resolved,
        )
        validate_adoption_grant(
            declaration,
            project_path,
            findings,
            project_id,
            capability_registry=capability_registry,
            provider_paths=paths,
            ledger=adoption_ledger or [],
            provider_verified=resolved,
        )
        # The exemption's owner, and the next action that ends it. Read-only,
        # and emitted once per exempt capability rather than once per project.
        report_unverified_exempt_capabilities(
            project_id,
            findings,
            capability_registry,
            resolved,
        )
    validate_interfaces(
        declaration,
        project_path,
        findings,
        project_id,
        index=interface_index,
        escalation=interface_escalation,
    )
    release_evidence = declaration.get("release_evidence")
    if release_evidence is not None:
        validate_release_evidence(release_evidence, project_path, findings, project_id)
    if isinstance(deployment, dict) and deployment.get("deployable"):
        validate_deployment_evidence(deployment, declaration, project_path, findings, project_id)
        validate_deployment_promotion(deployment, project_path, project_id, governance_root, findings)
    if isinstance(declaration.get("quality"), dict):
        validate_product_quality(declaration, project_path, findings, project_id)
    # Claim-triggered, not gated: runs for every registered project, and only
    # a present-tense absence claim in the README makes it produce anything.
    validate_readme_claims(
        declaration,
        project_path,
        findings,
        project_id,
        governance_root=governance_root,
        coverage_gaps=readme_claim_coverage,
        promoted_projects=promoted_projects,
    )
    if isinstance(declaration.get("standalone"), dict) or isinstance(declaration.get("integrations"), list):
        validate_standalone(declaration, project_path, findings, project_id, governance_root=governance_root)
    if entry.get("adoption") == "adopted":
        validate_handoff_quality(
            project_path,
            findings,
            project_id,
            rules_drift_baseline=rules_drift_baseline,
            observed_drift_keys=observed_drift_keys,
            scanned_projects=scanned_projects,
        )


def scan_for_secrets(value: Any, findings: list[Finding], location: str = "registry", *, secret_words: "re.Pattern[str] | None" = None, governance_root: Path | None = None) -> None:
    pattern = secret_words if secret_words is not None else secret_words_for(governance_root)
    if isinstance(value, dict):
        for key, child in value.items():
            if pattern.search(str(key)) and child not in (None, "", False):
                finding(findings, "ERROR", "SECRET_VALUE", f"secret-like field contains a value: {location}.{key}")
            scan_for_secrets(child, findings, f"{location}.{key}", secret_words=pattern)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            scan_for_secrets(child, findings, f"{location}[{index}]", secret_words=pattern)


def run(args: argparse.Namespace) -> tuple[list[Finding], dict[str, int]]:
    findings: list[Finding] = []
    governance_root = Path(__file__).resolve().parents[1]
    try:
        vocabulary = _vocabulary_for(governance_root)
    except Exception as exc:
        finding(findings, "ERROR", "VOCABULARY", str(exc))
        return findings, {"errors": len(findings), "warnings": 0}
    canonical_profiles = vocabulary["profiles"]
    canonical_kinds = vocabulary["kinds"]
    import vocabulary as vocabulary_module

    secret_words = vocabulary_module.secret_pattern(vocabulary["secret_field_substrings"])
    registry_path = Path(args.registry)
    if not registry_path.is_absolute():
        registry_path = governance_root / registry_path
    registry = load_json(registry_path, findings, "REGISTRY_FILE")
    if registry is None:
        return findings, {"errors": len(findings), "warnings": 0}
    configured_root = getattr(args, "root", None) or os.environ.get("WORKSPACE_ROOT") or registry.get("workspace_root")
    if args.self_check:
        workspace_root = governance_root
    elif configured_root:
        workspace_root = Path(configured_root).expanduser().resolve()
    else:
        workspace_root = Path.cwd().resolve()
    load_decisions_register(governance_root, registry, findings, self_check=bool(args.self_check))
    capability_registry = load_capability_registry(governance_root, findings)
    adoption_ledger = load_adoption_ledger(governance_root, findings)
    # Ratchet baseline for rules drift. Loaded once, fail-closed. A load failure
    # emits one ERROR and disables the per-project delta rather than guessing: an
    # empty baseline would report every existing gap as new and fail the fleet for
    # the wrong reason. The ERROR already blocks, so the per-project pass is not
    # needed to make the run fail.
    try:
        rules_drift_baseline: frozenset[tuple[str, str, str]] | None = load_rules_drift_baseline(governance_root)
    except RulesDriftBaselineError as exc:
        finding(findings, "ERROR", "RULES_DRIFT_BASELINE", str(exc))
        rules_drift_baseline = None
    observed_drift_keys: set[tuple[str, str, str]] = set()
    scanned_projects: set[str] = set()
    # The gate-debt precondition, read fail-closed in the same shape as the
    # ratchet above. Both records are loaded once for the whole run. A load
    # failure emits one ERROR and closes the rule: no repository is reported as
    # debt-free on the strength of a record that was never read, and the ERROR
    # already fails the run.
    try:
        gate_debt: dict[str, tuple[str, ...]] | None = load_gate_debt_record(governance_root)
    except GateDebtRecordError as exc:
        finding(findings, "ERROR", QUALITY_NEW_CHANGE_WITH_GATE_DEBT, str(exc))
        gate_debt = None
    try:
        gate_debt_baseline: frozenset[tuple[str, str]] | None = load_gate_debt_baseline(governance_root)
    except GateDebtBaselineError as exc:
        finding(findings, "ERROR", QUALITY_NEW_CHANGE_WITH_GATE_DEBT, str(exc))
        gate_debt_baseline = None
    try:
        gate_debt_exceptions: dict[tuple[str, str], str] | None = load_gate_debt_exceptions(governance_root)
    except GateDebtExceptionsError as exc:
        finding(findings, "ERROR", QUALITY_NEW_CHANGE_WITH_GATE_DEBT, str(exc))
        gate_debt_exceptions = None
    # CHECKER-COVERAGE accumulator for the README claim rule. The unscoped
    # metric is emitted ONCE for the whole run, not per repository, because the
    # information it carries is about this checker's coverage and not about a
    # repository's defect (ruling 7).
    readme_claim_coverage: list[tuple[str, str, int, str, str]] = []
    # The interface ramp, read once per run from the decisions register: a
    # WARNING today, blocking after the recorded date. Unreadable input leaves
    # every interface finding a WARNING, because nothing becomes blocking on a
    # record that could not be read.
    interface_escalation = interface_escalation_dates(governance_root)
    # Provider identity is read from the registry's own entries, never from a
    # directory name, so the eight ids that differ from their folder name
    # resolve exactly like the rest.
    provider_paths = {
        str(item.get("id")): workspace_root / str(item.get("path"))
        for item in (registry.get("projects") or [])
        if isinstance(item, dict) and isinstance(item.get("id"), str) and isinstance(item.get("path"), str)
    }
    projects = validate_registry(registry, workspace_root, findings, check_paths=not args.self_check, canonical_profiles=canonical_profiles, governance_root=governance_root)
    validate_profile_docs(governance_root, findings, canonical_profiles=canonical_profiles)
    scan_for_secrets(registry, findings, secret_words=secret_words)
    effective_projects = list(projects)
    explicit_paths = {str(entry.get("path")) for entry in projects}
    discovery = registry.get("discovery") or {}
    if discovery.get("mode") in {"immediate-directories", "explicit-plus-immediate-directories"} and not args.self_check:
        policy = discovery.get("unregistered_policy", "report")
        for discovered in discover_projects(workspace_root, registry):
            if discovered["path"] in explicit_paths:
                continue
            level = "ERROR" if policy == "fail" else "WARNING"
            finding(findings, level, "DISCOVERED_UNREGISTERED", "project folder is discovered but not registered; local metadata is the source of truth", discovered["id"])
            declaration_path = workspace_root / discovered["path"] / ".project.json"
            if declaration_path.is_file():
                try:
                    declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
                except (OSError, json.JSONDecodeError):
                    declaration = {}
                if isinstance(declaration, dict):
                    effective_projects.append({
                        "id": discovered["id"],
                        "path": discovered["path"],
                        "profile": declaration.get("profile", "inventory"),
                        "lifecycle": declaration.get("lifecycle", "inventory"),
                        "adoption": "pending",
                    })
    if args.self_check:
        selected = [{**entry, "path": "."} for entry in projects if entry.get("id") == "workspace-governance"]
        # The interface index is built from the declarations this run audits,
        # so a self-check resolves a `consumes` edge against the governance
        # repository's own declaration and a fleet audit against all of them.
        interface_index = build_interface_index(workspace_root, selected)
        for entry in selected:
            validate_project(
                entry,
                workspace_root,
                findings,
                governance_root=governance_root,
                check_publication=False,
                capability_registry=capability_registry,
                provider_paths=provider_paths,
                adoption_ledger=adoption_ledger,
                rules_drift_baseline=rules_drift_baseline,
                observed_drift_keys=observed_drift_keys,
                scanned_projects=scanned_projects,
                readme_claim_coverage=readme_claim_coverage,
                promoted_projects=promoted_project_ids(governance_root),
                gate_debt=gate_debt,
                gate_debt_baseline=gate_debt_baseline,
                gate_debt_exceptions=gate_debt_exceptions,
                interface_index=interface_index,
                interface_escalation=interface_escalation,
            )
    else:
        selected = effective_projects
        if args.project:
            selected = [entry for entry in effective_projects if entry.get("id") == args.project or entry.get("path") == args.project]
            if not selected:
                finding(findings, "ERROR", "PROJECT_UNKNOWN", f"project not found in registry: {args.project}")
        interface_index = build_interface_index(workspace_root, effective_projects)
        for entry in selected:
            adopted = entry.get("adoption", "pending") == "adopted"
            discovered_only = entry.get("path") not in explicit_paths
            if args.audit or args.project or adopted:
                validate_project(
                    entry,
                    workspace_root,
                    findings,
                    strict=not discovered_only,
                    governance_root=governance_root,
                    capability_registry=capability_registry,
                    provider_paths=provider_paths,
                    adoption_ledger=adoption_ledger,
                    rules_drift_baseline=rules_drift_baseline,
                    observed_drift_keys=observed_drift_keys,
                    scanned_projects=scanned_projects,
                    readme_claim_coverage=readme_claim_coverage,
                    promoted_projects=promoted_project_ids(governance_root),
                    gate_debt=gate_debt,
                    gate_debt_baseline=gate_debt_baseline,
                    gate_debt_exceptions=gate_debt_exceptions,
                    interface_index=interface_index,
                    interface_escalation=interface_escalation,
                )
            else:
                finding(findings, "WARNING", "ADOPTION_PENDING", "project is registered but not yet adopted", str(entry.get("id")))
    # One aggregate CHECKER-COVERAGE metric for the whole run, naming every
    # unscoped statement and the repository it came from. Level WARNING and
    # `REVIEW_REQUIRED` in the checker's own vocabulary: never a pass, and
    # never attributed to a repository.
    if readme_claim_coverage:
        names = "; ".join(
            f"{project} {file}:{line} ({reason}) {statement}"
            for project, file, line, reason, statement in readme_claim_coverage
        )
        finding(
            findings,
            "WARNING",
            QUALITY_README_CLAIM_UNSCOPED,
            f"checker coverage: {len(readme_claim_coverage)} README status statement(s) name no surface this "
            f"check can resolve, so they are untested rather than verified: {names}",
        )
    # Decay signal, not a failure: repositories that have fixed their drift make
    # baseline entries redundant, and the baseline should be tightened to match.
    # This is a WARNING and never touches the exit code.
    #
    # It is evaluated ONLY on a full audit. `--self-check` scans one repository
    # and `--project` scans one entry, so comparing either against a fleet-wide
    # snapshot would report the unscanned remainder as "redundant" and cry wolf.
    scanned_whole_fleet = (
        not args.self_check
        and not args.project
        and {key[0] for key in rules_drift_baseline}.issubset(scanned_projects)
    ) if rules_drift_baseline is not None else False
    if scanned_whole_fleet:
        redundant = rules_drift_baseline - observed_drift_keys
        if redundant:
            finding(
                findings,
                "WARNING",
                "RULES_DRIFT_BASELINE_STALE",
                f"{RULES_DRIFT_BASELINE_FILE} has {len(redundant)} entry(ies) no longer reported; tighten the baseline",
            )
    summary = {"errors": sum(item.level == "ERROR" for item in findings), "warnings": sum(item.level == "WARNING" for item in findings)}
    return findings, summary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", help="workspace root; defaults to registry workspace_root")
    parser.add_argument("--registry", default="projects.json")
    parser.add_argument("--project", help="validate one registered project")
    parser.add_argument("--audit", action="store_true", help="validate every registered project and fail on adoption gaps")
    parser.add_argument("--self-check", action="store_true", help="validate the governance repository only")
    parser.add_argument("--json", action="store_true", dest="as_json")
    parser.add_argument(
        "--decisions",
        action="store_true",
        help="print a summary of the toolchain decision register and exit (loader errors are non-zero)",
    )
    args = parser.parse_args(argv)
    if args.decisions:
        return _print_decisions_summary(args)
    findings, summary = run(args)
    if args.as_json:
        print(json.dumps({"summary": summary, "findings": [asdict(item) for item in findings]}, indent=2, sort_keys=True))
    else:
        for item in findings:
            prefix = f"[{item.project}] " if item.project else ""
            print(f"{item.level} {item.code}: {prefix}{item.message}")
        print(f"Summary: errors={summary['errors']} warnings={summary['warnings']}")
    if any(item.code == "VOCABULARY" for item in findings):
        return 2
    return 1 if summary["errors"] else 0


def _print_decisions_summary(args: argparse.Namespace) -> int:
    """Print the toolchain decision register summary and exit.

    Loader failures (missing file, malformed JSON, bad shape, unknown registry
    references) produce a non-zero exit and a short message on stderr. The
    register is owned by the governance root; this summary reads from there
    regardless of `--root` so the surface is stable for downstream tooling.
    """
    import decisions as decisions_module

    governance_root = _default_governance_root()
    register_path = decisions_module.decisions_path(governance_root)
    try:
        loaded = decisions_module.load_decisions(register_path)
    except decisions_module.DecisionError as exc:
        print(f"ERROR decisions register: {exc}", file=sys.stderr)
        return 1
    registry_path = governance_root / args.registry
    if not registry_path.is_file():
        print(
            f"ERROR decisions register: registry file not found at {registry_path}",
            file=sys.stderr,
        )
        return 1
    try:
        registry = json.loads(registry_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"ERROR decisions register: cannot read registry {registry_path}: {exc}", file=sys.stderr)
        return 1
    if not isinstance(registry, dict):
        print(f"ERROR decisions register: registry must be a JSON object: {registry_path}", file=sys.stderr)
        return 1
    registry_ids = {
        entry.get("id")
        for entry in (registry.get("projects") or [])
        if isinstance(entry, dict) and isinstance(entry.get("id"), str)
    }
    offenders = decisions_module.validate_against_registry(loaded, registry_ids)
    if offenders:
        joined = ", ".join(sorted(offenders))
        print(
            f"ERROR decisions register: unknown registry references: {joined}",
            file=sys.stderr,
        )
        return 1
    summary = [
        {"id": entry.id, "title": entry.title, "status": entry.status, "date": entry.date}
        for entry in loaded
    ]
    if args.as_json:
        print(json.dumps({"decisions": summary, "total": len(summary)}, indent=2, sort_keys=True))
        return 0
    for entry in summary:
        print(f"{entry['id']}\t{entry['status']}\t{entry['date']}\t{entry['title']}")
    print(f"Total decisions: {len(summary)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
