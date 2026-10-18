"""Tests for the capability adoption gate.

Covers the scenarios in
`openspec/changes/capability-adoption-gate/specs/capability-adoption-gate/spec.md`:

- CA-001 three-way join (registry x manifest x `consumes[]`)
- CA-002 derived duplicate-implementation finding
- CA-003 recorded WARN -> FAIL ramp
- CA-004 evidence-backed decline as the lawful exit
- CA-005 unresolvable capability identities stay non-blocking
- CA-006 additive finding codes
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any
from unittest import mock

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))

import workspace_check  # noqa: E402
from workspace_check import (  # noqa: E402
    CAPABILITY_ADOPTION_DECLINE_UNRECORDED,
    CAPABILITY_ADOPTION_EVIDENCE_STALE,
    CAPABILITY_ADOPTION_MISSING,
    CAPABILITY_ADOPTION_UNEVIDENCED,
    CAPABILITY_ADOPTION_UNRESOLVED,
    CAPABILITY_REGISTRY_INVALID,
    DECLINE_RATIONALES,
    CapabilityRegistry,
    effective_capability_posture,
    load_capability_registry,
    measure_capability_adoption,
    read_manifest_dependencies,
    validate_declines,
)

from _decisions_env import restore_decisions_register, use_empty_decisions_register  # noqa: E402


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


PROVIDER = "dotnet-platform-libs"
PACKAGE = "Platform.Idempotency"
CAPABILITY = "idempotency"

BASE_CAPABILITY_ENTRY = {"owner": "platform", "evidence_state": "configured", "evidence_ref": "src/local.cs"}


def registry(
    *,
    posture: str = "expected",
    provider: Any = PROVIDER,
    package: str = PACKAGE,
    reason: str | None = None,
    graduations: list[dict] | None = None,
    audit_threshold: int = 3,
) -> CapabilityRegistry:
    entry: dict[str, Any] = {"provider": provider, "package": package, "posture": posture}
    if posture == "exempt":
        entry["reason"] = reason or "provider-unavailable"
    return CapabilityRegistry(
        Path("."),
        {CAPABILITY: entry},
        list(graduations or []),
        audit_threshold,
    )


def _tree_hash(root: Path) -> str:
    """Recursive content hash of a fixture tree, used for the read-only proof."""
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        digest.update(path.relative_to(root).as_posix().encode("utf-8"))
        if path.is_file():
            digest.update(path.read_bytes())
    return digest.hexdigest()


class AdoptionFixture:
    """A consumer project root plus a staged capability registry."""

    def __init__(self, *, manifest: str | None = None, capability_entry: dict | None = None) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "consumer"
        self.root.mkdir(parents=True)
        if manifest is not None:
            (self.root / "Directory.Packages.props").write_text(manifest, encoding="utf-8")
        self.declaration: dict[str, Any] = {"schema_version": 1, "id": "consumer"}
        if capability_entry is not None:
            self.declaration["capabilities"] = {CAPABILITY: capability_entry}

    def add_file(self, relative: str, text: str) -> None:
        target = self.root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def audit(self, reg: CapabilityRegistry | None = None, *, verified: bool = True, project_id: str = "consumer") -> list:
        findings: list = []
        measure_capability_adoption(
            self.declaration,
            self.root,
            findings,
            project_id,
            reg if reg is not None else registry(),
            provider_verified=lambda provider, capability: verified,
        )
        return findings

    def codes(self, reg: CapabilityRegistry | None = None, **kwargs) -> list[str]:
        return [item.code for item in self.audit(reg, **kwargs)]

    def close(self) -> None:
        self.temp.cleanup()


MSBUILD = '<Project>\n  <ItemGroup>\n    <PackageVersion Include="Platform.Idempotency" Version="0.1.0" />\n  </ItemGroup>\n</Project>\n'


class ThreeWayJoinTests(unittest.TestCase):
    """CA-001: adoption is derived from registry + manifest + consumes[]."""

    def setUp(self):
        self.fixture = AdoptionFixture()
        self.addCleanup(self.fixture.close)

    def test_all_three_sources_agree_emits_nothing(self):
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {
                **BASE_CAPABILITY_ENTRY,
                "consumes": [{"provider": PROVIDER, "package": PACKAGE, "version": "0.1.0", "evidence_ref": "Directory.Packages.props"}],
            }
        }
        self.assertEqual(self.fixture.codes(), [])

    def test_manifest_proves_consumption_but_record_is_missing(self):
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.assertEqual(self.fixture.codes(), [CAPABILITY_ADOPTION_UNEVIDENCED])
        findings = self.fixture.audit()
        self.assertEqual(findings[0].level, "WARNING")
        self.assertIn(PROVIDER, findings[0].message)
        self.assertIn(PACKAGE, findings[0].message)
        self.assertNotIn(CAPABILITY_ADOPTION_MISSING, [item.code for item in findings])

    def test_consumes_entry_without_evidence_ref_is_unevidenced(self):
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": PROVIDER, "package": PACKAGE}]}
        }
        self.assertEqual(self.fixture.codes(), [CAPABILITY_ADOPTION_UNEVIDENCED])

    def test_declaration_without_manifest_edge_is_missing(self):
        self.fixture.add_file("docs/adoption.md", "# Adoption\n\nPinned in the local feed.\n")
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {
                **BASE_CAPABILITY_ENTRY,
                "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "docs/adoption.md"}],
            }
        }
        codes = self.fixture.codes()
        self.assertEqual(codes, [CAPABILITY_ADOPTION_MISSING])
        message = self.fixture.audit()[0].message
        self.assertIn(PROVIDER, message)
        self.assertIn(PACKAGE, message)
        self.assertIn("consumes[]", message)

    def test_registry_only_is_missing(self):
        self.assertEqual(self.fixture.codes(), [CAPABILITY_ADOPTION_MISSING])

    def test_floating_version_is_not_a_pinned_edge(self):
        self.fixture.add_file(
            "Directory.Packages.props",
            '<Project><ItemGroup><PackageVersion Include="Platform.Idempotency" Version="*" /></ItemGroup></Project>',
        )
        self.assertEqual(self.fixture.codes(), [CAPABILITY_ADOPTION_MISSING])

    def test_unreadable_manifest_is_never_a_fail(self):
        self.fixture.add_file("package.json", "{ this is not json")
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {
                **BASE_CAPABILITY_ENTRY,
                "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "Directory.Packages.props"}],
            }
        }
        # Force the unreadable branch by removing the only proving manifest.
        (self.fixture.root / "Directory.Packages.props").unlink()
        self.fixture.add_file("Directory.Packages.props", "<Project>\n")
        findings = self.fixture.audit(registry(posture="required"), verified=True)
        unproven = [item for item in findings if item.code == CAPABILITY_ADOPTION_UNEVIDENCED]
        self.assertEqual(len(unproven), 1)
        self.assertEqual(unproven[0].level, "WARNING")
        self.assertNotIn(CAPABILITY_ADOPTION_MISSING, [item.code for item in findings])

    def test_escaping_evidence_ref_is_refused_and_never_normalised(self):
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "../sibling-lib/shared.props"}]}
        }
        findings = self.fixture.audit()
        codes = [item.code for item in findings]
        self.assertIn(CAPABILITY_ADOPTION_EVIDENCE_STALE, codes)
        self.assertNotIn(CAPABILITY_ADOPTION_MISSING, codes)  # the edge itself is proven
        stale = next(item for item in findings if item.code == CAPABILITY_ADOPTION_EVIDENCE_STALE)
        self.assertIn("../sibling-lib/shared.props", stale.message)
        self.assertIn("was not read", stale.message)

    def test_absolute_evidence_ref_is_refused(self):
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "/etc/hosts"}]}
        }
        self.assertEqual(self.fixture.codes(), [CAPABILITY_ADOPTION_EVIDENCE_STALE])

    def test_evidence_ref_that_stopped_resolving_is_stale(self):
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "gone.props"}]}
        }
        findings = self.fixture.audit()
        codes = [item.code for item in findings]
        self.assertEqual(codes, [CAPABILITY_ADOPTION_EVIDENCE_STALE])
        self.assertIn("no longer resolves", findings[0].message)

    def test_consumes_naming_another_provider_is_not_this_capability(self):
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": "some-other-lib", "package": PACKAGE, "evidence_ref": "Directory.Packages.props"}]}
        }
        findings = self.fixture.audit()
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_UNEVIDENCED])
        self.assertIn("some-other-lib", findings[0].message)


class DuplicateImplementationTests(unittest.TestCase):
    """CA-002: a declared local implementation with no edge is a finding."""

    def setUp(self):
        self.fixture = AdoptionFixture()
        self.addCleanup(self.fixture.close)
        self.fixture.add_file("src/Infrastructure/IdempotencyGuard.cs", "// local retry loop\n")
        self.fixture.add_file("docs/adoption.md", "# Adoption\n")

    def _declaration(self, local: str) -> dict:
        return {
            CAPABILITY: {
                **BASE_CAPABILITY_ENTRY,
                "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "docs/adoption.md", "local_implementation": local}],
            }
        }

    def test_declared_local_implementation_and_no_consumption_is_warning_when_expected(self):
        self.fixture.declaration["capabilities"] = self._declaration("src/Infrastructure/IdempotencyGuard.cs")
        findings = self.fixture.audit()
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_MISSING])
        self.assertEqual(findings[0].level, "WARNING")
        self.assertIn("src/Infrastructure/IdempotencyGuard.cs", findings[0].message)
        self.assertIn("present", findings[0].message)

    def test_declared_local_implementation_is_an_error_when_required(self):
        self.fixture.declaration["capabilities"] = self._declaration("src/Infrastructure/IdempotencyGuard.cs")
        findings = self.fixture.audit(graduated_registry(), verified=True)
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_MISSING])
        self.assertEqual(findings[0].level, "ERROR")

    def test_unregistered_local_implementation_is_never_inferred(self):
        self.fixture.declaration["capabilities"] = {CAPABILITY: dict(BASE_CAPABILITY_ENTRY)}
        findings = self.fixture.audit(registry(posture="exempt"))
        self.assertEqual(findings, [])

    def test_escaping_local_implementation_is_not_read(self):
        self.fixture.declaration["capabilities"] = self._declaration("../../escape/Idempotency.cs")
        findings = self.fixture.audit()
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_MISSING])
        self.assertIn("escapes the project root", findings[0].message)


def graduated_registry(**kwargs) -> CapabilityRegistry:
    entry = {"from": "expected", "to": "required", "audit_count": 3, "provider_evidence": {"state": "verified", "evidence_ref": "src/idempotency/mod.rs"}}
    return registry(posture="required", graduations=[{"capability": CAPABILITY, **entry}], **kwargs)


class RampTests(unittest.TestCase):
    """CA-003: WARN -> FAIL only on a recorded transition."""

    def test_new_capability_is_admitted_as_a_warning(self):
        self.assertEqual(effective_capability_posture(CAPABILITY, registry().capabilities[CAPABILITY], registry(), True)[0], "expected")

    def test_exempt_posture_produces_no_finding(self):
        fixture = AdoptionFixture()
        self.addCleanup(fixture.close)
        self.assertEqual(fixture.codes(registry(posture="exempt")), [])

    def test_graduation_requires_a_verified_provider(self):
        posture, reason = effective_capability_posture(CAPABILITY, graduated_registry().capabilities[CAPABILITY], graduated_registry(), False)
        self.assertEqual(posture, "expected")
        self.assertIn("not verified", reason)

    def test_graduation_requires_a_recorded_entry(self):
        posture, reason = effective_capability_posture(CAPABILITY, registry(posture="required").capabilities[CAPABILITY], registry(posture="required"), True)
        self.assertEqual(posture, "expected")
        self.assertIn("no graduation entry", reason)

    def test_recorded_graduation_reaches_required(self):
        posture, _ = effective_capability_posture(CAPABILITY, graduated_registry().capabilities[CAPABILITY], graduated_registry(), True)
        self.assertEqual(posture, "required")

    def test_audit_count_cannot_be_forged_by_lowering_the_counter(self):
        reg = graduated_registry(audit_threshold=5)
        posture, reason = effective_capability_posture(CAPABILITY, reg.capabilities[CAPABILITY], reg, True)
        self.assertEqual(posture, "expected")
        self.assertIn("below graduation_audits", reason)

    def test_wall_clock_time_is_not_a_substitute(self):
        reg = registry(posture="required", graduations=[{"capability": CAPABILITY, "from": "expected", "to": "required", "audit_count": 99}])
        posture, reason = effective_capability_posture(CAPABILITY, reg.capabilities[CAPABILITY], reg, True)
        self.assertEqual(posture, "expected")
        self.assertIn("provider evidence", reason)

    def test_backwards_transition_is_refused(self):
        reg = registry(
            posture="required",
            graduations=[{"capability": CAPABILITY, "from": "exempt", "to": "required", "audit_count": 3, "provider_evidence": {"evidence_ref": "src/x.rs"}}],
        )
        posture, reason = effective_capability_posture(CAPABILITY, reg.capabilities[CAPABILITY], reg, True)
        self.assertEqual(posture, "expected")
        self.assertIn("expected -> required", reason)

    def test_forged_entry_emits_no_error_for_the_entry_itself(self):
        fixture = AdoptionFixture()
        self.addCleanup(fixture.close)
        findings = fixture.audit(registry(posture="required", graduations=[{"capability": CAPABILITY, "audit_count": 3}]), verified=True)
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_MISSING])
        self.assertEqual(findings[0].level, "WARNING")


class DeclineTests(unittest.TestCase):
    """CA-004: a decline is the lawful exit, and it is contained."""

    def setUp(self):
        self.fixture = AdoptionFixture()
        self.addCleanup(self.fixture.close)
        self.fixture.add_file("docs/shared-ownership.md", "# Shared ownership\n")

    def _audit(self, declines: Any, **kwargs) -> list:
        self.fixture.declaration["declines"] = declines
        return self.fixture.audit(**kwargs)

    def test_documented_decline_emits_no_adoption_finding(self):
        findings = self._audit([{"capability": CAPABILITY, "rationale": "dependency-direction", "evidence_ref": "docs/shared-ownership.md"}])
        self.assertEqual(findings, [])

    def test_every_closed_rationale_is_accepted(self):
        for rationale in DECLINE_RATIONALES:
            with self.subTest(rationale=rationale):
                findings = self._audit([{"capability": CAPABILITY, "rationale": rationale, "evidence_ref": "docs/shared-ownership.md"}])
                self.assertEqual(findings, [])

    def test_rationale_outside_the_closed_vocabulary_is_reported(self):
        findings = self._audit([{"capability": CAPABILITY, "rationale": "we-are-busy", "evidence_ref": "docs/shared-ownership.md"}])
        codes = [item.code for item in findings]
        self.assertIn(CAPABILITY_ADOPTION_DECLINE_UNRECORDED, codes)
        self.assertIn(CAPABILITY_ADOPTION_MISSING, codes)  # not exempt
        self.assertIn("we-are-busy", next(item.message for item in findings if item.code == CAPABILITY_ADOPTION_DECLINE_UNRECORDED))

    def test_decline_without_evidence_is_itself_a_finding(self):
        findings = self._audit([{"capability": CAPABILITY, "rationale": "privacy"}])
        self.assertIn(CAPABILITY_ADOPTION_DECLINE_UNRECORDED, [item.code for item in findings])
        self.assertIn(CAPABILITY_ADOPTION_MISSING, [item.code for item in findings])

    def test_decline_with_unresolvable_evidence_is_reported(self):
        findings = self._audit([{"capability": CAPABILITY, "rationale": "privacy", "evidence_ref": "docs/absent.md"}])
        self.assertIn(CAPABILITY_ADOPTION_DECLINE_UNRECORDED, [item.code for item in findings])

    def test_decline_is_capability_scoped(self):
        other = CapabilityRegistry(Path("."), {CAPABILITY: {"provider": PROVIDER, "package": PACKAGE, "posture": "expected"}, "rate-limiting": {"provider": PROVIDER, "package": "Platform.RateLimiting", "posture": "expected"}}, [], 3)
        findings = self._audit([{"capability": CAPABILITY, "rationale": "lifecycle", "evidence_ref": "docs/shared-ownership.md"}], reg=other)
        codes = [item.code for item in findings]
        self.assertEqual(codes.count(CAPABILITY_ADOPTION_MISSING), 1)
        self.assertIn("rate-limiting", findings[0].message)

    def test_decline_is_repo_local(self):
        other = AdoptionFixture()
        self.addCleanup(other.close)
        self.fixture.declaration["declines"] = [{"capability": CAPABILITY, "rationale": "lifecycle", "evidence_ref": "docs/shared-ownership.md"}]
        self.assertEqual(self.fixture.audit(), [])
        self.assertEqual(other.codes(), [CAPABILITY_ADOPTION_MISSING])

    def test_prose_decline_without_a_record_is_named(self):
        self.fixture.add_file(
            "AGENTS.md",
            "# Agents\n\nAssess the idempotency capability before duplication. Keep the capability local when\ndependency direction makes that the correct boundary for this repository.\n",
        )
        findings = self.fixture.audit()
        codes = [item.code for item in findings]
        self.assertIn(CAPABILITY_ADOPTION_DECLINE_UNRECORDED, codes)
        message = next(item.message for item in findings if item.code == CAPABILITY_ADOPTION_DECLINE_UNRECORDED)
        self.assertIn("AGENTS.md", message)
        self.assertIn("dependency-direction", message)
        self.assertIn(CAPABILITY_ADOPTION_MISSING, codes)

    def test_prose_is_never_treated_as_a_decline(self):
        self.fixture.add_file("README.md", "# Demo\n\ndo not extract for naming symmetry\n")
        findings = self.fixture.audit()
        self.assertIn(CAPABILITY_ADOPTION_MISSING, [item.code for item in findings])

    def test_malformed_declines_block_is_reported(self):
        findings = self._audit({"capability": CAPABILITY})
        self.assertIn(CAPABILITY_ADOPTION_DECLINE_UNRECORDED, [item.code for item in findings])

    def test_provider_is_exempt_from_its_own_capability(self):
        self.fixture.declaration["capabilities"] = {CAPABILITY: dict(BASE_CAPABILITY_ENTRY)}
        self.assertEqual(self.fixture.audit(project_id=PROVIDER), [])

    def test_validate_declines_returns_exempt_ids(self):
        self.fixture.declaration["declines"] = [{"capability": CAPABILITY, "rationale": "ownership", "evidence_ref": "docs/shared-ownership.md"}]
        findings: list = []
        exempt = validate_declines(self.fixture.declaration["declines"], self.fixture.root, findings, "consumer", registry())
        self.assertEqual(exempt, {CAPABILITY})
        self.assertEqual(findings, [])


class UnresolvedIdentityTests(unittest.TestCase):
    """CA-005: an unresolvable id is review-required, never a failure."""

    def setUp(self):
        self.fixture = AdoptionFixture()
        self.addCleanup(self.fixture.close)

    def test_ambiguous_provider_is_unresolved_and_never_escalates(self):
        for posture in ("expected", "required"):
            with self.subTest(posture=posture):
                reg = registry(posture=posture, provider=[PROVIDER, "other-lib"])
                if posture == "required":
                    reg = CapabilityRegistry(Path("."), reg.capabilities, [{"capability": CAPABILITY, "from": "expected", "to": "required", "audit_count": 3, "provider_evidence": {"evidence_ref": "x"}}], 3)
                findings = self.fixture.audit(reg, verified=True)
                self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_UNRESOLVED])
                self.assertEqual(findings[0].level, "WARNING")
                self.assertNotIn(CAPABILITY_ADOPTION_MISSING, [item.code for item in findings])

    def test_capability_bound_to_no_provider_is_unresolved(self):
        reg = CapabilityRegistry(Path("."), {CAPABILITY: {"package": PACKAGE, "posture": "expected"}}, [], 3)
        findings = self.fixture.audit(reg)
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_UNRESOLVED])

    def test_capability_bound_to_no_provider_is_never_a_fail(self):
        reg = CapabilityRegistry(
            Path("."),
            {CAPABILITY: {"package": PACKAGE, "posture": "required"}},
            [{"capability": CAPABILITY, "from": "expected", "to": "required", "audit_count": 9, "provider_evidence": {"evidence_ref": "x"}}],
            3,
        )
        findings = self.fixture.audit(reg, verified=True)
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_UNRESOLVED])
        self.assertEqual(findings[0].level, "WARNING")

    def test_registry_entry_without_a_package_is_unresolved(self):
        reg = CapabilityRegistry(Path("."), {CAPABILITY: {"provider": PROVIDER, "posture": "expected"}}, [], 3)
        findings = self.fixture.audit(reg)
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_UNRESOLVED])

    def test_consumes_for_an_unregistered_capability_is_unresolved(self):
        self.fixture.declaration["capabilities"] = {
            "some-unknown-capability": {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "README.md"}]}
        }
        findings = self.fixture.audit()
        self.assertIn(CAPABILITY_ADOPTION_UNRESOLVED, [item.code for item in findings])


class RegistryLoadTests(unittest.TestCase):
    """A missing or empty registry is a governance error, never a no-op."""

    def _stage(self, payload: str | None) -> Path:
        fd, path = tempfile.mkstemp(prefix="capabilities-", suffix=".json")
        os.close(fd)
        target = Path(path)
        if payload is not None:
            target.write_text(payload, encoding="utf-8")
        self.addCleanup(target.unlink, True)
        return target

    def _load(self, payload: str | None) -> tuple:
        target = self._stage(payload)
        os.environ["WORKSPACE_CAPABILITIES_PATH"] = str(target)
        self.addCleanup(os.environ.pop, "WORKSPACE_CAPABILITIES_PATH", None)
        findings: list = []
        return load_capability_registry(Path("."), findings), findings

    def test_valid_registry_loads(self):
        loaded, findings = self._load(json.dumps({"schema_version": 1, "capabilities": {"idempotency": {"provider": "p", "package": "P", "posture": "expected"}}}))
        self.assertIsNotNone(loaded)
        self.assertEqual(findings, [])

    def test_missing_registry_is_an_error(self):
        loaded, findings = self._load(None)
        self.assertIsNone(loaded)
        self.assertEqual([item.code for item in findings], [CAPABILITY_REGISTRY_INVALID])
        self.assertEqual(findings[0].level, "ERROR")

    def test_unparseable_registry_is_an_error(self):
        loaded, findings = self._load("{ not json")
        self.assertIsNone(loaded)
        self.assertEqual([item.code for item in findings], [CAPABILITY_REGISTRY_INVALID])

    def test_empty_registry_never_degrades_to_enforcing_nothing(self):
        loaded, findings = self._load(json.dumps({"schema_version": 1, "capabilities": {}}))
        self.assertIsNone(loaded)
        self.assertEqual([item.code for item in findings], [CAPABILITY_REGISTRY_INVALID])

    def test_exempt_without_a_reason_is_an_error(self):
        loaded, findings = self._load(json.dumps({"capabilities": {"idempotency": {"provider": "p", "package": "P", "posture": "exempt"}}}))
        self.assertIsNone(loaded)
        self.assertIn("recorded reason", findings[0].message)

    def test_invalid_posture_is_an_error(self):
        loaded, findings = self._load(json.dumps({"capabilities": {"idempotency": {"provider": "p", "package": "P", "posture": "mandatory"}}}))
        self.assertIsNone(loaded)
        self.assertEqual([item.code for item in findings], [CAPABILITY_REGISTRY_INVALID])

    def test_shipped_registry_is_valid(self):
        findings: list = []
        loaded = load_capability_registry(Path(__file__).parents[1], findings)
        self.assertIsNotNone(loaded, findings)
        self.assertEqual(findings, [])
        self.assertTrue(loaded.capabilities)
        for name, entry in loaded.capabilities.items():
            self.assertIn(entry["posture"], ("exempt", "expected", "required"))


class SchemaFixtureTests(unittest.TestCase):
    """Declaration-shape fixtures for `consumes[]` and `declines[]`.

    The audit itself never loads the JSON Schema — it validates in Python, as
    every other check in this repository does — so these fixtures pin the
    *declared* contract. They skip cleanly where `jsonschema` is not
    installed, which keeps the repository's stdlib-only promise intact.
    """

    @classmethod
    def setUpClass(cls):
        try:
            import jsonschema
        except ImportError:  # pragma: no cover - CI installs no packages
            raise unittest.SkipTest("jsonschema is not installed; schema fixtures are skipped")
        cls.jsonschema = jsonschema
        cls.schema = json.loads((Path(__file__).parents[1] / "schemas" / "project.schema.json").read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(cls.schema)
        cls.validator = jsonschema.Draft202012Validator(cls.schema)

    def _declaration(self, **extra) -> dict:
        payload = {
            "schema_version": 1,
            "id": "demo-project",
            "kind": "product",
            "profile": "inventory",
            "lifecycle": "inventory",
            "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
        }
        payload.update(extra)
        return payload

    def _assert_valid(self, payload: dict) -> None:
        self.assertEqual(list(self.validator.iter_errors(payload)), [])

    def _assert_invalid(self, payload: dict) -> None:
        self.assertTrue(list(self.validator.iter_errors(payload)))

    def test_capability_required_keys_are_unchanged(self):
        self.assertEqual(self.schema["$defs"]["capability"]["required"], ["owner", "evidence_state"])

    def test_a_legacy_declaration_stays_valid(self):
        self._assert_valid(self._declaration(capabilities={"idempotency": {"owner": "platform", "evidence_state": "declared"}}))

    def test_consumes_valid(self):
        self._assert_valid(
            self._declaration(
                capabilities={
                    "idempotency": {
                        "owner": "platform",
                        "evidence_state": "declared",
                        "consumes": [{"provider": PROVIDER, "package": PACKAGE, "version": "0.1.0", "evidence_ref": "Directory.Packages.props"}],
                    }
                }
            )
        )

    def test_consumes_missing_evidence_ref(self):
        self._assert_invalid(self._declaration(capabilities={"idempotency": {"owner": "platform", "evidence_state": "declared", "consumes": [{"provider": PROVIDER, "package": PACKAGE}]}}))

    def test_consumes_evidence_ref_escaping_the_root(self):
        self._assert_invalid(self._declaration(capabilities={"idempotency": {"owner": "platform", "evidence_state": "declared", "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "../sibling/x.props"}]}}))

    def test_consumes_non_object_entry(self):
        self._assert_invalid(self._declaration(capabilities={"idempotency": {"owner": "platform", "evidence_state": "declared", "consumes": ["not-an-object"]}}))

    def test_consumes_local_implementation_accepts_a_path_or_a_list(self):
        for value in ("src/Guard.cs", ["src/Guard.cs", "src/Other.cs"]):
            with self.subTest(value=value):
                self._assert_valid(
                    self._declaration(
                        capabilities={
                            "idempotency": {
                                "owner": "platform",
                                "evidence_state": "declared",
                                "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "README.md", "local_implementation": value}],
                            }
                        }
                    )
                )

    def test_decline_valid(self):
        self._assert_valid(self._declaration(declines=[{"capability": CAPABILITY, "rationale": "dependency-direction", "evidence_ref": "docs/shared-ownership.md", "review_by": "2026-12-01"}]))

    def test_decline_unknown_rationale(self):
        self._assert_invalid(self._declaration(declines=[{"capability": CAPABILITY, "rationale": "too-busy", "evidence_ref": "docs/x.md"}]))

    def test_decline_unresolvable_evidence_ref(self):
        self._assert_invalid(self._declaration(declines=[{"capability": CAPABILITY, "rationale": "privacy", "evidence_ref": "../outside.md"}]))

    def test_every_closed_rationale_is_allowed_by_the_schema(self):
        for rationale in DECLINE_RATIONALES:
            with self.subTest(rationale=rationale):
                self._assert_valid(self._declaration(declines=[{"capability": CAPABILITY, "rationale": rationale, "evidence_ref": "docs/x.md"}]))


class ReadOnlyProofTests(unittest.TestCase):
    """The audit reads manifests and writes nothing."""

    def setUp(self):
        self.fixture = AdoptionFixture(manifest=MSBUILD)
        self.addCleanup(self.fixture.close)
        self.fixture.add_file("Directory.Packages.props", MSBUILD)
        self.fixture.add_file("src/Program.cs", "// consumer\n")
        self.fixture.declaration["capabilities"] = {
            CAPABILITY: {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "Directory.Packages.props"}]}
        }

    def test_tree_is_unchanged_after_the_audit(self):
        before = _tree_hash(self.fixture.root)
        with mock.patch("subprocess.run", side_effect=AssertionError("the audit must not spawn a child process")), mock.patch(
            "subprocess.Popen", side_effect=AssertionError("the audit must not spawn a child process")
        ), mock.patch("subprocess.check_output", side_effect=AssertionError("the audit must not spawn a child process")):
            findings = self.fixture.audit()
        self.assertEqual(findings, [])
        self.assertEqual(before, _tree_hash(self.fixture.root))

    def test_no_package_manager_is_invoked(self):
        with mock.patch("subprocess.Popen", side_effect=AssertionError("no package manager")) as popen:
            read_manifest_dependencies(self.fixture.root)
        popen.assert_not_called()

    def test_manifest_scan_is_bounded_and_skips_vendor_trees(self):
        self.fixture.add_file("node_modules/left-pad/package.json", '{"dependencies": {"vendored": "1.0.0"}}')
        self.fixture.add_file("obj/Debug/generated.csproj", '<Project><ItemGroup><PackageReference Include="Generated" Version="1.0.0" /></ItemGroup></Project>')
        manifests = read_manifest_dependencies(self.fixture.root)
        self.assertIn("platform.idempotency", manifests.pinned)
        self.assertNotIn("vendored", manifests.pinned)
        self.assertNotIn("generated", manifests.pinned)
        self.assertIn("Directory.Packages.props", manifests.sources)

    def test_lockfiles_are_never_read(self):
        self.fixture.add_file("package-lock.json", '{"packages": {"node_modules/left-pad": {"version": "1.0.0"}}}')
        manifests = read_manifest_dependencies(self.fixture.root)
        self.assertNotIn("package-lock.json", manifests.sources)
        self.assertNotIn("left-pad", manifests.pinned)


class AdditiveCompatibilityTests(unittest.TestCase):
    """CA-006: a project that declares nothing new is measured to no finding."""

    def test_plain_declaration_is_unchanged_under_the_shipped_registry(self):
        """The no-regression guarantee: the registry this change ships.

        Every capability the governance root actually registers is `exempt`
        with reason `provider-unavailable`, because no provider in the fleet
        declares a `verified` capability yet. Charging 81 consumers for a
        provider gap would be noise, so the shipped registry produces no
        adoption finding for any project.
        """
        fixture = AdoptionFixture(manifest=MSBUILD)
        self.addCleanup(fixture.close)
        fixture.declaration = {
            "schema_version": 1,
            "id": "consumer",
            "capabilities": {"identity": {"owner": "platform", "evidence_state": "configured", "evidence_ref": "src/x.rs"}},
        }
        findings: list = []
        shipped = load_capability_registry(Path(__file__).parents[1], findings)
        self.assertIsNotNone(shipped, findings)
        measure_capability_adoption(fixture.declaration, fixture.root, findings, "consumer", shipped)
        self.assertEqual([item for item in findings if item.code.startswith("CAPABILITY_")], [])

    def test_adopting_a_provider_is_always_a_provider_side_decision(self):
        """Every shipped entry is `exempt`, so no repo is charged on day one."""
        shipped = load_capability_registry(Path(__file__).parents[1], [])
        assert shipped is not None
        for name, entry in shipped.capabilities.items():
            with self.subTest(capability=name):
                self.assertEqual(entry["posture"], "exempt")
                self.assertEqual(entry["reason"], "provider-unavailable")
        self.assertEqual(shipped.graduations, [])

    def test_a_repo_that_declares_nothing_is_charged_once_a_capability_is_expected(self):
        """This is the gate doing its job, not a regression."""
        fixture = AdoptionFixture()
        self.addCleanup(fixture.close)
        fixture.declaration = {"schema_version": 1, "id": "consumer"}
        findings = fixture.audit(registry())
        self.assertEqual([item.code for item in findings], [CAPABILITY_ADOPTION_MISSING])
        self.assertEqual(findings[0].level, "WARNING")

    def test_every_declared_code_is_in_the_capability_namespace(self):
        for code in (
            CAPABILITY_ADOPTION_MISSING,
            CAPABILITY_ADOPTION_UNEVIDENCED,
            CAPABILITY_ADOPTION_EVIDENCE_STALE,
            CAPABILITY_ADOPTION_DECLINE_UNRECORDED,
            CAPABILITY_ADOPTION_UNRESOLVED,
        ):
            self.assertTrue(code.startswith("CAPABILITY_"), code)
            self.assertEqual(code, code.upper())
            self.assertRegex(code, r"^[A-Z][A-Z0-9_]*$")

    def test_audit_still_does_not_promote_a_capability_to_verified(self):
        """The checker records intent; it never writes a state back."""
        self.assertIn("never promotes a capability to verified on its own", " ".join(workspace_check.validate_capabilities.__doc__.split()))
        fixture = AdoptionFixture(manifest=MSBUILD)
        self.addCleanup(fixture.close)
        before = json.dumps(fixture.declaration, sort_keys=True)
        fixture.declaration["capabilities"] = {
            CAPABILITY: {**BASE_CAPABILITY_ENTRY, "consumes": [{"provider": PROVIDER, "package": PACKAGE, "evidence_ref": "Directory.Packages.props"}]}
        }
        fixture.audit()
        self.assertEqual(fixture.declaration["capabilities"][CAPABILITY]["evidence_state"], "configured")
        del before

    def test_existing_capability_codes_are_unchanged(self):
        for code in ("CAPABILITIES_SHAPE", "CAPABILITY_NAME", "CAPABILITY_OWNER", "CAPABILITY_STATE", "CAPABILITY_REF", "CAPABILITY_PENDING", "CAPABILITY_STALE", "CAPABILITY_UNSUPPORTED", "CAPABILITY_BLOCKED"):
            self.assertIn(code, Path(workspace_check.__file__).read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
