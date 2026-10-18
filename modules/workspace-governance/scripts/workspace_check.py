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


# Historical script/import path; implementations are grouped by domain.
if __name__ == "__main__":
    sys.modules.setdefault("workspace_check", sys.modules[__name__])
_parts = []
for _name in (
    "workspace_check_projects",
    "workspace_check_capabilities",
    "workspace_check_interfaces",
    "workspace_check_quality",
    "workspace_check_standalone",
    "workspace_check_openspec",
    "workspace_check_project",
    "workspace_check_cli",
):
    _part = __import__(_name)
    _parts.append(_part)
    globals().update({key: value for key, value in vars(_part).items() if not key.startswith("__")})
for _part in _parts:
    _part.__dict__.update({key: value for key, value in globals().items() if not key.startswith("__")})


if __name__ == "__main__":
    sys.exit(main())
