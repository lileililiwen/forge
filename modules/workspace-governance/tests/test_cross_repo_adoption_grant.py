"""Tests for the cross-repo adoption grant.

Covers the scenarios in
`openspec/changes/cross-repo-adoption-grant/specs/cross-repo-adoption-grant/spec.md`:

- GR-001 a bounded, single-use, expiring permission
- GR-002 writes confined to the consumer root
- GR-003 expiry, revocation, and single use
- GR-004 a verified provider is required
- GR-005 every granted adoption is recorded, append-only
- GR-006 the agent rule splits source edits from artifact consumption
- GR-007 additive finding codes
"""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))

import workspace_check  # noqa: E402
from workspace_check import (  # noqa: E402
    ADOPTION_GRANT_PROVIDER_UNVERIFIED,
    ADOPTION_GRANT_SCOPE_VIOLATION,
    ADOPTION_GRANT_SHAPE,
    ADOPTION_GRANT_STALE,
    ADOPTION_GRANT_UNSATISFIED,
    CapabilityRegistry,
    _canonical_record_hash,
    adoption_ledger_path,
    load_adoption_ledger,
    validate_adoption_grant,
)

from _decisions_env import restore_decisions_register, use_empty_decisions_register  # noqa: E402


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


CONSUMER = "some-product"
PROVIDER = "dotnet-platform-libs"
CAPABILITY = "idempotency"
PACKAGE = "Platform.Idempotency"
NOW = datetime(2026, 10, 5, 12, 0, 0, tzinfo=timezone.utc)

MSBUILD = (
    '<Project>\n  <ItemGroup>\n    <PackageVersion Include="Platform.Idempotency" Version="0.1.0" />\n  </ItemGroup>\n</Project>\n'
)


def stamp(offset_days: int) -> str:
    return (NOW + timedelta(days=offset_days)).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def base_grant(**overrides) -> dict:
    grant = {
        "capability": CAPABILITY,
        "provider": PROVIDER,
        "package": PACKAGE,
        "version": "0.1.0",
        "actor": "agent:session-1",
        "issued_at": stamp(-1),
        "expires_at": stamp(4),
        "write_set": [".project.json", "Directory.Packages.props", "src/**/CompositionRoot.cs", "tests/**/*.cs"],
        "rollback_ref": "openspec/changes/2026-10-05-adopt-idempotency",
    }
    grant.update(overrides)
    return grant


def record(**overrides) -> dict:
    body = {
        "kind": "adoption",
        "consumer": CONSUMER,
        "capability": CAPABILITY,
        "provider": PROVIDER,
        "package": PACKAGE,
        "version": "0.1.0",
        "replaced": ["src/Infrastructure/IdempotencyGuard.cs"],
        "actor": "agent:session-1",
        "grant_expires_at": stamp(4),
        "rollback_ref": "openspec/changes/2026-10-05-adopt-idempotency",
        "recorded_at": stamp(0),
    }
    body.update(overrides)
    body["content_hash"] = _canonical_record_hash(body)
    return body


def signed_record(previous: str = "", **overrides) -> dict:
    body = dict(overrides)
    body.setdefault("prev_hash", previous)
    return record(**body)


class GrantFixture:
    """A consumer root, a provider root, and a staged capability registry."""

    def __init__(self, *, grant: dict | None = None, consumes: bool = True, manifest: str | None = MSBUILD) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.consumer = self.root / CONSUMER
        self.provider = self.root / PROVIDER
        self.consumer.mkdir(parents=True)
        self.provider.mkdir(parents=True)
        (self.provider / "src").mkdir()
        (self.provider / "idempotency.rs").write_text("pub fn guard() {}\n", encoding="utf-8")
        if manifest is not None:
            (self.consumer / "Directory.Packages.props").write_text(manifest, encoding="utf-8")
        (self.consumer / "docs").mkdir()
        (self.consumer / "docs" / "adoption.md").write_text("# Adoption\n", encoding="utf-8")
        self.declaration: dict = {
            "schema_version": 1,
            "id": CONSUMER,
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
        }
        if consumes:
            self.declaration["capabilities"] = {
                CAPABILITY: {
                    "owner": "platform",
                    "evidence_state": "configured",
                    "consumes": [{"provider": PROVIDER, "package": PACKAGE, "version": "0.1.0", "evidence_ref": "Directory.Packages.props"}],
                }
            }
        if grant is not None:
            self.declaration["adoption_grant"] = grant

    def provider_paths(self) -> dict[str, Path]:
        return {PROVIDER: self.provider, CONSUMER: self.consumer}

    def registry(self) -> CapabilityRegistry:
        return CapabilityRegistry(Path("."), {CAPABILITY: {"provider": PROVIDER, "package": PACKAGE, "posture": "expected"}}, [], 3)

    def audit(self, *, ledger=None, verified: bool = True, now=NOW) -> list:
        findings: list = []
        validate_adoption_grant(
            self.declaration,
            self.consumer,
            findings,
            CONSUMER,
            capability_registry=self.registry(),
            provider_paths=self.provider_paths(),
            ledger=ledger or [],
            provider_verified=lambda provider, capability: verified,
            now=now,
        )
        return findings

    def codes(self, **kwargs) -> list[str]:
        return [item.code for item in self.audit(**kwargs)]

    def add(self, relative: str, text: str) -> None:
        target = self.consumer / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def close(self) -> None:
        self.temp.cleanup()


class GrantShapeTests(unittest.TestCase):
    """GR-001: a grant is bounded, and it may not create its own binding."""

    def setUp(self):
        self.fixture = GrantFixture(grant=base_grant())
        self.addCleanup(self.fixture.close)

    def test_a_bounded_grant_emits_nothing(self):
        self.assertEqual(self.fixture.codes(ledger=[signed_record()]), [])

    def test_no_grant_means_no_finding(self):
        fixture = GrantFixture(grant=None)
        self.addCleanup(fixture.close)
        self.assertEqual(fixture.codes(), [])

    def test_non_object_grant_is_a_shape_error(self):
        self.fixture.declaration["adoption_grant"] = "please-adopt"
        findings = self.fixture.audit()
        self.assertEqual([item.code for item in findings], [ADOPTION_GRANT_SHAPE])
        self.assertEqual(findings[0].level, "ERROR")

    def test_every_required_field_is_required(self):
        for field in ("capability", "provider", "package", "actor", "issued_at", "expires_at", "write_set"):
            with self.subTest(field=field):
                grant = base_grant()
                grant.pop(field)
                self.fixture.declaration["adoption_grant"] = grant
                findings = self.fixture.audit()
                self.assertIn(ADOPTION_GRANT_SHAPE, [item.code for item in findings])
                self.assertIn(field, findings[0].message)

    def test_write_set_with_a_parent_segment_is_refused_as_shape(self):
        self.fixture.declaration["adoption_grant"] = base_grant(write_set=[".project.json", "../sibling/AGENTS.md"])
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SHAPE, [item.code for item in findings])
        self.assertIn("escapes the project root", " ".join(item.message for item in findings))

    def test_write_set_with_an_absolute_path_is_refused_as_shape(self):
        self.fixture.declaration["adoption_grant"] = base_grant(write_set=["/etc/hosts"])
        self.assertIn(ADOPTION_GRANT_SHAPE, self.fixture.codes())

    def test_empty_write_set_is_refused_as_shape(self):
        self.fixture.declaration["adoption_grant"] = base_grant(write_set=[])
        self.assertIn(ADOPTION_GRANT_SHAPE, self.fixture.codes())

    def test_unregistered_provider_is_refused(self):
        self.fixture.declaration["adoption_grant"] = base_grant(provider="not-a-project")
        findings = self.fixture.audit()
        self.assertEqual([item.code for item in findings], [ADOPTION_GRANT_PROVIDER_UNVERIFIED])
        self.assertIn("not-a-project", findings[0].message)

    def test_unbound_capability_is_refused(self):
        self.fixture.declaration["adoption_grant"] = base_grant(capability="rate-limiting")
        findings = self.fixture.audit()
        self.assertEqual([item.code for item in findings], [ADOPTION_GRANT_PROVIDER_UNVERIFIED])
        self.assertIn("rate-limiting", findings[0].message)

    def test_capability_bound_to_another_provider_is_refused(self):
        self.fixture.declaration["adoption_grant"] = base_grant(capability="telemetry")
        registry = CapabilityRegistry(Path("."), {"telemetry": {"provider": "some-other-lib", "package": "P", "posture": "expected"}}, [], 3)
        findings: list = []
        validate_adoption_grant(
            self.fixture.declaration,
            self.fixture.consumer,
            findings,
            CONSUMER,
            capability_registry=registry,
            provider_paths=self.fixture.provider_paths(),
            ledger=[],
            provider_verified=lambda provider, capability: True,
            now=NOW,
        )
        self.assertEqual([item.code for item in findings], [ADOPTION_GRANT_PROVIDER_UNVERIFIED])
        self.assertIn("some-other-lib", findings[0].message)

    def test_unverified_provider_is_refused(self):
        findings = self.fixture.audit(ledger=[signed_record()], verified=False)
        self.assertIn(ADOPTION_GRANT_PROVIDER_UNVERIFIED, [item.code for item in findings])
        self.assertEqual(next(item for item in findings if item.code == ADOPTION_GRANT_PROVIDER_UNVERIFIED).level, "WARNING")

    def test_non_iso_timestamps_are_refused_as_shape(self):
        self.fixture.declaration["adoption_grant"] = base_grant(expires_at="next tuesday")
        self.assertIn(ADOPTION_GRANT_SHAPE, self.fixture.codes())


class ScopeTests(unittest.TestCase):
    """GR-002: the write set is the consumer root, and only the consumer root."""

    def setUp(self):
        self.fixture = GrantFixture(grant=base_grant())
        self.addCleanup(self.fixture.close)

    def test_permitted_consumer_side_repoint_is_accepted(self):
        self.fixture.add("src/App/CompositionRoot.cs", "builder.Services.AddIdempotency();\n")
        self.fixture.add("tests/App.Tests/ConformanceTests.cs", "public class ConformanceTests { }\n")
        self.assertEqual(self.fixture.codes(ledger=[signed_record()]), [])

    def test_write_set_naming_the_provider_is_a_violation(self):
        self.fixture.declaration["adoption_grant"] = base_grant(write_set=[".project.json", f"{PROVIDER}/src/Guard.cs"])
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, [item.code for item in findings])
        self.assertEqual(next(item for item in findings if item.code == ADOPTION_GRANT_SCOPE_VIOLATION).level, "ERROR")

    def test_write_set_naming_the_promotion_ledger_is_a_violation(self):
        self.fixture.declaration["adoption_grant"] = base_grant(write_set=[".project.json", "deployment/promotions.json"])
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, [item.code for item in findings])
        self.assertIn("promotion ledger", " ".join(item.message for item in findings))

    def test_symlink_escaping_the_consumer_root_is_a_violation(self):
        outside = self.fixture.root / "outside"
        outside.mkdir()
        (outside / "steal.cs").write_text("// outside\n", encoding="utf-8")
        try:
            (self.fixture.consumer / "linked").symlink_to(outside, target_is_directory=True)
        except OSError:  # pragma: no cover - platform without symlink permission
            self.skipTest("symlinks are unavailable in this environment")
        self.fixture.declaration["adoption_grant"] = base_grant(write_set=[".project.json", "linked/steal.cs"])
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, [item.code for item in findings])
        self.assertIn("outside the consumer root", " ".join(item.message for item in findings))

    def test_project_reference_to_a_sibling_is_forbidden_under_any_grant(self):
        self.fixture.add("App.csproj", '<Project><ItemGroup><ProjectReference Include="../dotnet-platform-libs/src/Platform.Idempotency" /></ItemGroup></Project>')
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, [item.code for item in findings])
        self.assertIn("source-level coupling", " ".join(item.message for item in findings))

    def test_absolute_project_reference_is_forbidden(self):
        self.fixture.add("App.csproj", '<Project><ItemGroup><ProjectReference Include="/home/paul/code/dotnet-platform-libs/x.csproj" /></ItemGroup></Project>')
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, self.fixture.codes())

    def test_cargo_path_dependency_on_a_sibling_is_forbidden(self):
        self.fixture.add("Cargo.toml", '[dependencies]\nplatform = { path = "../dotnet-platform-libs/src/Platform.Core" }\n')
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, self.fixture.codes())

    def test_provider_version_bump_is_a_violation(self):
        self.fixture.add("Directory.Packages.props", '<Project><ItemGroup><PackageVersion Include="Platform.Idempotency" Version="0.2.0" /></ItemGroup></Project>')
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, [item.code for item in findings])
        message = next(item.message for item in findings if item.code == ADOPTION_GRANT_SCOPE_VIOLATION)
        self.assertIn("0.2.0", message)
        self.assertIn(PROVIDER, message)

    def test_granted_version_is_accepted(self):
        self.assertEqual(self.fixture.codes(ledger=[signed_record()]), [])

    def test_promotion_is_never_in_scope(self):
        self.fixture.declaration["deployment"] = {"deployable": True, "jenkins_job": "x", "compose_file": "docker-compose.yml"}
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, [item.code for item in findings])
        self.assertIn("never a path to deployable", " ".join(item.message for item in findings))

    def test_a_violation_is_reported_whatever_the_code_state(self):
        """The violation is the finding, independent of the resulting state."""
        self.fixture.add("App.csproj", '<Project><ItemGroup><ProjectReference Include="../dotnet-platform-libs/x.csproj" /></ItemGroup></Project>')
        self.fixture.declaration["capabilities"] = {}
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_SCOPE_VIOLATION, [item.code for item in findings])


class LifecycleTests(unittest.TestCase):
    """GR-003: a grant is bounded in time and in use."""

    def setUp(self):
        self.fixture = GrantFixture(grant=base_grant())
        self.addCleanup(self.fixture.close)

    def test_expired_grant_is_stale(self):
        self.fixture.declaration["adoption_grant"] = base_grant(expires_at=stamp(-1))
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_STALE, [item.code for item in findings])
        message = next(item.message for item in findings if item.code == ADOPTION_GRANT_STALE)
        self.assertIn("expired", message)
        self.assertEqual(next(item for item in findings if item.code == ADOPTION_GRANT_STALE).level, "WARNING")

    def test_grant_is_not_active_before_issue(self):
        self.fixture.declaration["adoption_grant"] = base_grant(issued_at=stamp(2))
        self.assertIn(ADOPTION_GRANT_STALE, self.fixture.codes())

    def test_revoked_grant_is_stale(self):
        self.fixture.declaration["adoption_grant"] = base_grant(status="revoked")
        findings = self.fixture.audit()
        self.assertIn(ADOPTION_GRANT_STALE, [item.code for item in findings])
        self.assertIn("revoked", " ".join(item.message for item in findings))

    def test_invalid_status_is_a_shape_error(self):
        self.fixture.declaration["adoption_grant"] = base_grant(status="spent")
        self.assertIn(ADOPTION_GRANT_SHAPE, self.fixture.codes())

    def test_a_second_repoint_under_one_grant_is_refused(self):
        first = signed_record()
        second = signed_record(prev_hash=first["content_hash"], version="0.1.1", recorded_at=stamp(1))
        findings = self.fixture.audit(ledger=[first, second])
        self.assertIn(ADOPTION_GRANT_STALE, [item.code for item in findings])
        self.assertIn("sized for one change", " ".join(item.message for item in findings))

    def test_a_rollback_row_does_not_count_as_a_second_repoint(self):
        first = signed_record()
        rollback = signed_record(prev_hash=first["content_hash"], kind="rollback", recorded_at=stamp(1))
        self.assertEqual(self.fixture.codes(ledger=[first, rollback]), [])

    def test_expiry_does_not_retroactively_legalize_a_repoint(self):
        self.fixture.declaration["adoption_grant"] = base_grant(expires_at=stamp(-2), issued_at=stamp(-9))
        findings = self.fixture.audit(ledger=[signed_record()])
        codes = [item.code for item in findings]
        self.assertIn(ADOPTION_GRANT_STALE, codes)
        self.assertNotIn(ADOPTION_GRANT_UNSATISFIED, codes)


class RecordTests(unittest.TestCase):
    """GR-005: every granted adoption leaves an immutable, chained row."""

    def setUp(self):
        self.fixture = GrantFixture(grant=base_grant())
        self.addCleanup(self.fixture.close)

    def test_repoint_with_no_record_is_unsatisfied(self):
        findings = self.fixture.audit(ledger=[])
        self.assertIn(ADOPTION_GRANT_UNSATISFIED, [item.code for item in findings])
        self.assertIn("no adoption record", " ".join(item.message for item in findings))
        self.assertEqual(next(item for item in findings if item.code == ADOPTION_GRANT_UNSATISFIED).level, "ERROR")

    def test_a_record_keeps_the_original_when_rolled_back(self):
        first = signed_record()
        rollback = signed_record(prev_hash=first["content_hash"], kind="rollback", recorded_at=stamp(1))
        self.fixture.declaration["adoption_grant"] = base_grant(status="revoked")
        findings = self.fixture.audit(ledger=[first, rollback])
        self.assertNotIn(ADOPTION_GRANT_UNSATISFIED, [item.code for item in findings])

    def test_record_missing_a_mandatory_field_is_unsatisfied(self):
        for field in workspace_check.ADOPTION_RECORD_FIELDS:
            with self.subTest(field=field):
                body = record()
                body.pop(field)
                body["content_hash"] = _canonical_record_hash(body)
                findings = self.fixture.audit(ledger=[body])
                self.assertIn(ADOPTION_GRANT_UNSATISFIED, [item.code for item in findings])

    def test_every_mandatory_record_field_is_present_in_a_good_record(self):
        for field in workspace_check.ADOPTION_RECORD_FIELDS:
            self.assertIn(field, record())


class LedgerTests(unittest.TestCase):
    """The ledger is append-only and tamper-evident."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "adoption").mkdir()
        self.path = self.root / "adoption" / "adoptions.json"

    def _write(self, records) -> None:
        self.path.write_text(json.dumps({"schema_version": 1, "records": records}, indent=2) + "\n", encoding="utf-8")

    def test_ledger_path_is_canonical(self):
        self.assertEqual(adoption_ledger_path(self.root), self.path)

    def test_shipped_ledger_is_valid_and_empty(self):
        findings: list = []
        records = load_adoption_ledger(Path(__file__).parents[1], findings)
        self.assertEqual(records, [])
        self.assertEqual(findings, [])

    def test_missing_ledger_is_not_an_error(self):
        findings: list = []
        self.assertEqual(load_adoption_ledger(self.root, findings), [])
        self.assertEqual(findings, [])

    def test_hand_edited_record_is_refused(self):
        row = signed_record()
        row["version"] = "9.9.9"  # content_hash no longer covers the body
        self._write([row])
        findings: list = []
        load_adoption_ledger(self.root, findings)
        self.assertEqual([item.code for item in findings], [ADOPTION_GRANT_UNSATISFIED])
        self.assertIn("hand-edited", findings[0].message)

    def test_hand_inserted_record_breaks_the_append_chain(self):
        first = signed_record()
        inserted = signed_record(prev_hash="", recorded_at=stamp(1))
        self._write([first, inserted])
        findings: list = []
        load_adoption_ledger(self.root, findings)
        self.assertTrue(any("append chain" in item.message for item in findings), findings)

    def test_unparseable_ledger_is_refused(self):
        self.path.write_text("{ not json", encoding="utf-8")
        findings: list = []
        self.assertEqual(load_adoption_ledger(self.root, findings), [])
        self.assertEqual([item.code for item in findings], [ADOPTION_GRANT_UNSATISFIED])

    def test_hash_covers_the_body_only(self):
        row = record()
        expected = hashlib.sha256(json.dumps({k: v for k, v in row.items() if k != "content_hash"}, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()
        self.assertEqual(row["content_hash"], expected)


class AgentRuleTests(unittest.TestCase):
    """GR-006: the shared clause splits source edits from consumption."""

    def _clause(self) -> str:
        return (Path(workspace_check.__file__).parents[1] / "templates" / "project" / "AGENTS.md").read_text(encoding="utf-8")

    def test_the_canonical_template_carries_the_amended_clause(self):
        text = self._clause()
        self.assertIn("Assess shared infrastructure before duplication", text)
        self.assertIn("adoption_grant", text)
        self.assertIn("an un-named assessment is not an assessment", text)

    def test_source_edits_to_a_sibling_remain_forbidden(self):
        text = self._clause()
        self.assertIn("Do not edit a sibling repository without scope authorization", text)
        self.assertIn("ProjectReference", text)

    def test_boundary_arguments_are_preserved(self):
        text = self._clause()
        self.assertIn("ownership, privacy, lifecycle, or dependency direction", text)

    def test_the_governance_repository_carries_the_same_clause(self):
        text = (Path(workspace_check.__file__).parents[1] / "AGENTS.md").read_text(encoding="utf-8")
        self.assertIn("adoption_grant", text)

    def test_the_amendment_relaxes_nothing(self):
        for banned in ("ProjectReference", "../", "bumping a provider", "source-level coupling"):
            self.assertIn(banned, self._clause())


class ReadOnlyProofTests(unittest.TestCase):
    """GR-007: the grant check reads the workspace and writes nothing."""

    def setUp(self):
        self.fixture = GrantFixture(grant=base_grant())
        self.addCleanup(self.fixture.close)
        self.fixture.add("src/App/CompositionRoot.cs", "builder.Services.AddIdempotency();\n")

    def _tree_hash(self, root: Path) -> str:
        digest = hashlib.sha256()
        for path in sorted(root.rglob("*")):
            digest.update(path.relative_to(root).as_posix().encode("utf-8"))
            if path.is_file():
                digest.update(path.read_bytes())
        return digest.hexdigest()

    def test_consumer_and_provider_are_unchanged_after_the_check(self):
        before = (self._tree_hash(self.fixture.consumer), self._tree_hash(self.fixture.provider))
        with mock.patch("subprocess.Popen", side_effect=AssertionError("the audit must not spawn a child process")), mock.patch(
            "subprocess.run", side_effect=AssertionError("the audit must not spawn a child process")
        ), mock.patch("subprocess.check_output", side_effect=AssertionError("the audit must not spawn a child process")):
            findings = self.fixture.audit(ledger=[signed_record()])
        self.assertEqual(findings, [])
        self.assertEqual(before, (self._tree_hash(self.fixture.consumer), self._tree_hash(self.fixture.provider)))

    def test_nothing_outside_the_consumer_root_is_read(self):
        outside = self.fixture.root / "outside"
        outside.mkdir()
        (outside / "secret.txt").write_text("do not read me\n", encoding="utf-8")
        self.fixture.declaration["adoption_grant"] = base_grant(write_set=[".project.json", "../outside/secret.txt"])
        reads: list[Path] = []
        original = Path.read_text

        def recording_read(self, *args, **kwargs):
            reads.append(self)
            return original(self, *args, **kwargs)

        with mock.patch.object(Path, "read_text", recording_read):
            findings = self.fixture.audit(ledger=[signed_record()])
        self.assertIn(ADOPTION_GRANT_SHAPE, [item.code for item in findings])
        self.assertEqual([str(item) for item in reads if "outside" in str(item)], [])

    def test_the_grant_check_never_runs_a_build_or_a_test(self):
        source = Path(workspace_check.__file__).read_text(encoding="utf-8")
        body = source.split("def validate_adoption_grant(")[1].split("\ndef ")[0]
        for forbidden in ("check_output", "subprocess", "run(", "Popen"):
            self.assertNotIn(forbidden, body, f"{forbidden!r} must not appear in the grant check")


class AdditiveCompatibilityTests(unittest.TestCase):
    """GR-007: a project with no grant is measured to no grant finding."""

    def test_a_project_without_a_grant_produces_no_finding(self):
        fixture = GrantFixture(grant=None, consumes=False)
        self.addCleanup(fixture.close)
        self.assertEqual(fixture.codes(), [])

    def test_every_declared_code_uses_the_repo_style(self):
        for code in (
            ADOPTION_GRANT_SHAPE,
            ADOPTION_GRANT_SCOPE_VIOLATION,
            ADOPTION_GRANT_UNSATISFIED,
            ADOPTION_GRANT_STALE,
            ADOPTION_GRANT_PROVIDER_UNVERIFIED,
        ):
            self.assertTrue(code.startswith("ADOPTION_GRANT_"), code)
            self.assertEqual(code, code.upper())
            self.assertRegex(code, r"^[A-Z][A-Z0-9_]*$")

    def test_the_families_are_distinct(self):
        self.assertNotIn("ADOPTION_GRANT_", workspace_check.CAPABILITY_ADOPTION_MISSING)
        self.assertNotIn("CAPABILITY_", ADOPTION_GRANT_STALE)

    def test_promotion_ledger_and_registry_are_untouched(self):
        text = Path(workspace_check.__file__).read_text(encoding="utf-8")
        self.assertNotIn("promotions.json", text.split("def load_adoption_ledger")[1].split("def ")[0])
        self.assertEqual(workspace_check.adoption_ledger_path(Path("/x")).name, "adoptions.json")


if __name__ == "__main__":
    unittest.main()
