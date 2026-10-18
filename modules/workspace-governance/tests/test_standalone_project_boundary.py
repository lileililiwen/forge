"""Tests for the standalone-project-boundary capability.

Covers the scenarios in
`openspec/specs/standalone-project-boundary/spec.md`:

- STB-001 classify project independence (six canonical statuses)
- STB-002 preserve portable path boundaries
- STB-003 optional integrations are bounded and non-fatal
- STB-004 forbidden declared statuses are errors
- STB-005 export normalized adapter evidence
- STB-006 redact credential-shaped values in evidence
- STB-007 no command execution (read-only)
"""

from __future__ import annotations

import copy
import hashlib
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))

import workspace_check  # noqa: E402
from workspace_check import (  # noqa: E402
    Finding,
    STANDALONE_ABSOLUTE_PATH,
    STANDALONE_BLOCKED,
    STANDALONE_DETECTED_SIBLING,
    STANDALONE_EVIDENCE_MISSING,
    STANDALONE_INTEGRATION_PENDING,
    STANDALONE_OPTIONAL,
    STANDALONE_PATH_ESCAPE,
    STANDALONE_REQUIRED_SERVICE,
    STANDALONE_REQUIRED_SIBLING,
    STANDALONE_SECRET_VALUE,
    STANDALONE_SHAPE,
    STANDALONE_STATUS_INVALID,
    classify_standalone,
    validate_standalone,
)

import forge_governance_adapter as adapter  # noqa: E402

from _decisions_env import restore_decisions_register, use_empty_decisions_register  # noqa: E402


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


BASE_DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "product",
    "profile": "inventory",
    "lifecycle": "inventory",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}


def _write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def _make_workspace(
    project_id: str = "demo-project",
    *,
    declaration: dict | None = None,
    adopted: bool = True,
) -> tuple[object, Path]:
    """Build a temp governance + workspace root with a single project.

    The temp root is treated as both the governance root and the
    workspace root by the adapter. The vocabulary file is copied from
    the real governance root so the standalone enums can be loaded.
    Returns `(temp_dir, root)`. The caller MUST keep `temp_dir` alive
    for the lifetime of the test.
    """
    temp = tempfile.TemporaryDirectory()
    root = Path(temp.name)
    project = root / project_id
    project.mkdir()
    payload = copy.deepcopy(declaration) if declaration is not None else copy.deepcopy(BASE_DECLARATION)
    payload.setdefault("id", project_id)
    _write_json(project / ".project.json", payload)
    registry = {
        "schema_version": 1,
        "workspace_root": None,
        "discovery": {"mode": "explicit", "unregistered_policy": "report"},
        "projects": [
            {
                "id": project_id,
                "path": project_id,
                "profile": "inventory",
                "lifecycle": "inventory",
                **({"adoption": "adopted"} if adopted else {"adoption": "pending"}),
            },
        ],
    }
    _write_json(root / "projects.json", registry)
    # Copy the real vocabulary so the standalone enums are loadable
    # in tests that need them. The vocabulary is owned by the
    # governance repo at the canonical path; the test root is
    # treated as a synthetic governance root that also happens to
    # host the project under audit.
    real_vocab = Path(__file__).parents[1] / "vocabulary.json"
    if real_vocab.is_file():
        (root / "vocabulary.json").write_text(real_vocab.read_text(encoding="utf-8"), encoding="utf-8")
    return temp, root


def _request(project_id: str = "demo-project") -> dict:
    return {
        "contract": "0.1.0",
        "action": "check",
        "project_id": project_id,
        "project_path": project_id,
    }


def _evaluate(request: dict, *, governance_root: Path, workspace_root: Path | None = None):
    if workspace_root is None:
        workspace_root = governance_root
    return adapter.evaluate(request, governance_root=governance_root, workspace_root=workspace_root)


# ---------------------------------------------------------------------------
# STB-001 classification
# ---------------------------------------------------------------------------


class StandaloneClassificationTests(unittest.TestCase):
    def test_absent_standalone_block_classifies_blocked(self):
        temp, root = _make_workspace()
        self.addCleanup(temp.cleanup)
        findings: list[Finding] = []
        validate_standalone(
            {"id": "demo-project", "schema_version": 1},
            root / "demo-project",
            findings,
            "demo-project",
        )
        codes = [item.code for item in findings]
        self.assertIn(STANDALONE_BLOCKED, codes)
        self.assertTrue(all(item.level == "WARNING" for item in findings if item.code == STANDALONE_BLOCKED))

    def test_standalone_status_passes_silently(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        findings: list[Finding] = []
        validate_standalone(
            json.loads((root / "demo-project" / ".project.json").read_text()),
            root / "demo-project",
            findings,
            "demo-project",
        )
        self.assertEqual(findings, [], [item.__dict__ for item in findings])

    def test_standalone_with_optional_integrations_passes(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone-with-optional-integrations",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
                "integrations": [
                    {"id": "workspace-governance", "required": False, "evidence_ref": "HANDOFF.md"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        findings: list[Finding] = []
        validate_standalone(
            json.loads((root / "demo-project" / ".project.json").read_text()),
            root / "demo-project",
            findings,
            "demo-project",
        )
        # optional integration produces a STANDALONE_OPTIONAL warning
        self.assertTrue(
            any(item.code == STANDALONE_OPTIONAL for item in findings),
            [item.__dict__ for item in findings],
        )
        self.assertFalse(any(item.code == STANDALONE_REQUIRED_SIBLING for item in findings))
        self.assertFalse(any(item.code == STANDALONE_REQUIRED_SERVICE for item in findings))

    def test_library_dependent_passes(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "library-dependent",
                    "local_defaults": ["serde@1", "tokio@1"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        findings: list[Finding] = []
        validate_standalone(
            json.loads((root / "demo-project" / ".project.json").read_text()),
            root / "demo-project",
            findings,
            "demo-project",
        )
        self.assertEqual(findings, [])

    def test_required_sibling_projects_record_workspace_dependent(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "workspace-dependent",
                    "local_defaults": [],
                    "required_external_services": [],
                    "required_sibling_projects": ["platform-contracts", "openpanel"],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        sibling_findings = [item for item in findings if item.code == STANDALONE_REQUIRED_SIBLING]
        self.assertEqual(len(sibling_findings), 2)
        for item in sibling_findings:
            self.assertEqual(item.level, "ERROR")
        observation = classify_standalone(declaration, root / "demo-project", [], "demo-project")
        self.assertEqual(observation["standalone_status"], "workspace-dependent")

    def test_required_external_services_record_service_dependent(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "service-dependent",
                    "local_defaults": [],
                    "required_external_services": ["postgres", "redis"],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        service_findings = [item for item in findings if item.code == STANDALONE_REQUIRED_SERVICE]
        self.assertEqual(len(service_findings), 2)
        for item in service_findings:
            self.assertEqual(item.level, "WARNING")
        observation = classify_standalone(declaration, root / "demo-project", [], "demo-project")
        self.assertEqual(observation["standalone_status"], "service-dependent")

    def test_declared_blocked_status_records_warning(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "blocked",
                    "local_defaults": [],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_BLOCKED for item in findings))


# ---------------------------------------------------------------------------
# STB-002 portable paths
# ---------------------------------------------------------------------------


class StandalonePathBoundaryTests(unittest.TestCase):
    def test_absolute_path_in_local_defaults_is_error(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["/etc/passwd"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_ABSOLUTE_PATH for item in findings))

    def test_windows_drive_letter_path_is_error(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["C:\\\\Users\\\\fixture"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_ABSOLUTE_PATH for item in findings))

    def test_relative_evidence_ref_resolves(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                    "evidence_ref": "docs/standalone.md",
                    "evidence_state": "configured",
                },
            }
        )
        self.addCleanup(temp.cleanup)
        # write the evidence file
        (root / "demo-project" / "docs").mkdir()
        (root / "demo-project" / "docs" / "standalone.md").write_text("# standalone\n", encoding="utf-8")
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertFalse(any(item.code == STANDALONE_ABSOLUTE_PATH for item in findings))
        self.assertFalse(any(item.code == STANDALONE_PATH_ESCAPE for item in findings))
        self.assertFalse(any(item.code == STANDALONE_EVIDENCE_MISSING for item in findings))

    def test_evidence_ref_with_dotdot_is_error(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                    "evidence_ref": "../outside/secret.md",
                    "evidence_state": "configured",
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_PATH_ESCAPE for item in findings))

    def test_evidence_ref_missing_on_disk_is_warning(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                    "evidence_ref": "docs/does-not-exist.md",
                    "evidence_state": "configured",
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_EVIDENCE_MISSING for item in findings))


# ---------------------------------------------------------------------------
# STB-003 optional integrations
# ---------------------------------------------------------------------------


class StandaloneIntegrationTests(unittest.TestCase):
    def test_optional_integration_with_evidence_no_blocking(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "integrations": [
                    {"id": "workspace-governance", "required": False, "evidence_ref": "HANDOFF.md"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_OPTIONAL for item in findings))
        self.assertFalse(any(item.code == STANDALONE_INTEGRATION_PENDING for item in findings))

    def test_required_integration_with_missing_evidence_is_warning(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "integrations": [
                    {"id": "ci-runner", "required": True, "evidence_ref": "ci/nope.md"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_INTEGRATION_PENDING for item in findings))

    def test_required_integration_with_evidence_passes(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "integrations": [
                    {"id": "ci-runner", "required": True, "evidence_ref": "ci/runner.md"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        (root / "demo-project" / "ci").mkdir()
        (root / "demo-project" / "ci" / "runner.md").write_text("# ci\n", encoding="utf-8")
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertFalse(any(item.code == STANDALONE_INTEGRATION_PENDING for item in findings))
        # required integrations are silent when valid
        integration_codes = [
            item.code for item in findings
            if item.code in (STANDALONE_OPTIONAL, STANDALONE_INTEGRATION_PENDING)
        ]
        self.assertEqual(integration_codes, [])

    def test_integration_evidence_ref_escape_is_error(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "integrations": [
                    {"id": "ci-runner", "required": True, "evidence_ref": "../escape.md"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_PATH_ESCAPE for item in findings))

    def test_integration_absolute_evidence_ref_is_error(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "integrations": [
                    {"id": "ci-runner", "required": True, "evidence_ref": "/etc/secret"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_ABSOLUTE_PATH for item in findings))


# ---------------------------------------------------------------------------
# STB-004 forbidden statuses
# ---------------------------------------------------------------------------


class StandaloneStatusValidationTests(unittest.TestCase):
    def test_unknown_status_is_error(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "all-standalone",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_STATUS_INVALID for item in findings))
        observation = classify_standalone(declaration, root / "demo-project", [], "demo-project")
        self.assertEqual(observation["standalone_status"], "blocked")

    def test_standalone_not_an_object_is_error(self):
        temp, root = _make_workspace(declaration={**BASE_DECLARATION, "standalone": "not-an-object"})
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_SHAPE for item in findings))

    def test_local_defaults_not_array_is_error(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": "sqlite",
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_SHAPE for item in findings))

    def test_integrations_not_array_is_error(self):
        temp, root = _make_workspace(declaration={**BASE_DECLARATION, "integrations": "nope"})
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        self.assertTrue(any(item.code == STANDALONE_SHAPE for item in findings))


# ---------------------------------------------------------------------------
# STB-006 secret redaction
# ---------------------------------------------------------------------------


class StandaloneSecretRedactionTests(unittest.TestCase):
    def test_secret_value_in_local_defaults_is_redacted(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["password=hunter2"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        secret_findings = [item for item in findings if item.code == STANDALONE_SECRET_VALUE]
        self.assertEqual(len(secret_findings), 1)
        self.assertIn("[redacted]", secret_findings[0].message)
        self.assertNotIn("hunter2", secret_findings[0].message)

    def test_secret_value_in_integration_evidence_ref_is_redacted(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "integrations": [
                    {"id": "ci-runner", "required": False, "evidence_ref": "token=ABCDEFG"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        secret_findings = [item for item in findings if item.code == STANDALONE_SECRET_VALUE]
        self.assertEqual(len(secret_findings), 1)
        self.assertIn("[redacted]", secret_findings[0].message)


# ---------------------------------------------------------------------------
# STB-007 bounded static scan
# ---------------------------------------------------------------------------


class StandaloneBoundedStaticScanTests(unittest.TestCase):
    def test_cargo_path_sibling_detected(self):
        temp, root = _make_workspace()
        self.addCleanup(temp.cleanup)
        (root / "demo-project" / "Cargo.toml").write_text(
            "[package]\nname = \"demo\"\n[dependencies]\nfoo = { path = \"../sibling-rs\" }\n",
            encoding="utf-8",
        )
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        detected = [item for item in findings if item.code == STANDALONE_DETECTED_SIBLING]
        self.assertEqual(len(detected), 1)
        self.assertIn("Cargo.toml", detected[0].message)
        self.assertIn("sibling-rs", detected[0].message)

    def test_package_json_workspace_sibling_detected(self):
        temp, root = _make_workspace()
        self.addCleanup(temp.cleanup)
        (root / "demo-project" / "package.json").write_text(
            json.dumps({"name": "demo", "workspaces": ["../sibling-ts"]}),
            encoding="utf-8",
        )
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        detected = [item for item in findings if item.code == STANDALONE_DETECTED_SIBLING]
        self.assertEqual(len(detected), 1)
        self.assertIn("sibling-ts", detected[0].message)

    def test_static_scan_does_not_override_declared_status(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        (root / "demo-project" / "Cargo.toml").write_text(
            "[dependencies]\nfoo = { path = \"../sibling-rs\" }\n",
            encoding="utf-8",
        )
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        observation = classify_standalone(declaration, root / "demo-project", [], "demo-project")
        self.assertEqual(observation["standalone_status"], "standalone")
        self.assertTrue(
            any(item["code"] == STANDALONE_DETECTED_SIBLING for item in observation["standalone_findings"])
        )

    def test_static_scan_never_reads_outside_project_root(self):
        temp, root = _make_workspace()
        self.addCleanup(temp.cleanup)
        # A path that escapes two levels up — the scanner only reports the
        # textual reference, it never resolves the path on disk.
        (root / "demo-project" / "go.mod").write_text(
            "module example.com/demo\nrequire foo v1.0.0\nreplace foo => ../../escape\n",
            encoding="utf-8",
        )
        declaration = json.loads((root / "demo-project" / ".project.json").read_text())
        findings: list[Finding] = []
        validate_standalone(declaration, root / "demo-project", findings, "demo-project")
        detected = [item for item in findings if item.code == STANDALONE_DETECTED_SIBLING]
        self.assertEqual(len(detected), 1)
        self.assertIn("escape", detected[0].message)


# ---------------------------------------------------------------------------
# STB-005 adapter surface
# ---------------------------------------------------------------------------


class AdapterStandaloneObservationTests(unittest.TestCase):
    def test_observation_includes_standalone_fields(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone-with-optional-integrations",
                    "local_defaults": ["sqlite"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
                "integrations": [
                    {"id": "workspace-governance", "required": False, "evidence_ref": "HANDOFF.md"},
                ],
            }
        )
        self.addCleanup(temp.cleanup)
        # Stage a .git/HEAD so the observation has a revision to report.
        (root / "demo-project" / ".git").mkdir()
        (root / "demo-project" / ".git" / "HEAD").write_text("deadbeef1234\n", encoding="utf-8")
        observation = _evaluate(_request(), governance_root=root)
        self.assertIn("standalone_status", observation)
        self.assertEqual(observation["standalone_status"], "standalone-with-optional-integrations")
        self.assertIn("standalone_integrations", observation)
        self.assertEqual(
            observation["standalone_integrations"],
            [
                {"id": "workspace-governance", "required": False, "evidence_ref": "HANDOFF.md"},
            ],
        )
        self.assertIn("standalone_findings", observation)
        self.assertEqual(observation["standalone_observed_revision"], "deadbeef1234")
        # Existing keys still present
        self.assertIn("status", observation)
        self.assertIn("evidence", observation)
        self.assertIn("detail", observation)
        self.assertIn("metadata", observation)

    def test_unknown_project_observation_still_has_standalone_blocked(self):
        temp, root = _make_workspace()
        self.addCleanup(temp.cleanup)
        observation = _evaluate(_request("never-registered"), governance_root=root)
        self.assertEqual(observation["status"], "unknown")
        self.assertEqual(observation["standalone_status"], "blocked")
        self.assertTrue(
            any("STANDALONE_BLOCKED" in line for line in observation["standalone_findings"]),
            observation,
        )

    def test_audit_crash_omits_standalone_fields(self):
        temp, root = _make_workspace()
        self.addCleanup(temp.cleanup)
        # `build_standalone_observation` is the public adapter surface for
        # standalone evidence; it must NEVER raise. An underlying audit
        # crash is translated to a `blocked` status with a single
        # STANDALONE_SHAPE finding, and the observation is still emitted.
        with mock.patch.object(
            workspace_check, "validate_standalone", side_effect=RuntimeError("boom")
        ):
            observation = _evaluate(_request(), governance_root=root)
        self.assertIn("standalone_status", observation)
        self.assertEqual(observation["standalone_status"], "blocked")
        self.assertTrue(
            any("STANDALONE_SHAPE" in line for line in observation["standalone_findings"]),
            observation,
        )

    def test_secret_in_standalone_finding_is_scrubbed(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "standalone",
                    "local_defaults": ["password=hunter2"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        observation = _evaluate(_request(), governance_root=root)
        joined = "\n".join(observation["standalone_findings"])
        self.assertIn("[redacted]", joined)
        self.assertNotIn("hunter2", joined)

    def test_observation_remains_read_only(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "workspace-dependent",
                    "local_defaults": [],
                    "required_external_services": [],
                    "required_sibling_projects": ["openpanel"],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        before = _hash_tree(root)
        _evaluate(_request(), governance_root=root)
        _evaluate(_request("never-registered"), governance_root=root)
        after = _hash_tree(root)
        self.assertEqual(before, after, "adapter mutated the workspace tree")

    def test_main_writes_standalone_block(self):
        temp, root = _make_workspace(
            declaration={
                **BASE_DECLARATION,
                "standalone": {
                    "status": "library-dependent",
                    "local_defaults": ["serde@1"],
                    "required_external_services": [],
                    "required_sibling_projects": [],
                },
            }
        )
        self.addCleanup(temp.cleanup)
        stdout = io.StringIO()
        stderr = io.StringIO()
        with mock.patch.object(adapter, "_resolve_roots", return_value=(root, root)), \
                mock.patch.object(sys, "stdin", io.StringIO(json.dumps(_request()))), \
                redirect_stdout(stdout), \
                mock.patch.object(sys, "stderr", stderr):
            rc = adapter.main()
        self.assertEqual(rc, 0, stderr.getvalue())
        observation = json.loads(stdout.getvalue().strip())
        self.assertEqual(observation["standalone_status"], "library-dependent")
        self.assertIn("standalone_findings", observation)


# ---------------------------------------------------------------------------
# Vocabulary loader integration
# ---------------------------------------------------------------------------


class VocabularyStandaloneTests(unittest.TestCase):
    def test_loader_exposes_standalone_enums(self):
        import vocabulary

        governance_root = Path(__file__).parents[1]
        vocab = vocabulary.load_vocabulary_for_governance_root(governance_root)
        self.assertIn("standalone_statuses", vocab)
        self.assertIn("integration_required_states", vocab)
        self.assertEqual(
            set(vocab["standalone_statuses"]),
            {
                "standalone",
                "standalone-with-optional-integrations",
                "library-dependent",
                "workspace-dependent",
                "service-dependent",
                "blocked",
            },
        )
        self.assertEqual(
            set(vocab["integration_required_states"]),
            {"required", "optional"},
        )

    def test_duplicate_standalone_status_fails(self):
        import vocabulary
        from vocabulary import VocabularyError

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            payload = {
                "schema_version": 1,
                "profiles": ["product"],
                "kinds": ["product"],
                "placeholder_markers": ["todo"],
                "secret_field_substrings": ["token"],
                "standalone_statuses": ["standalone", "standalone"],
            }
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaises(VocabularyError):
                vocabulary.load_vocabulary(path)

    def test_empty_standalone_statuses_fails(self):
        import vocabulary
        from vocabulary import VocabularyError

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            payload = {
                "schema_version": 1,
                "profiles": ["product"],
                "kinds": ["product"],
                "placeholder_markers": ["todo"],
                "secret_field_substrings": ["token"],
                "standalone_statuses": [],
            }
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaises(VocabularyError):
                vocabulary.load_vocabulary(path)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _hash_tree(root: Path) -> str:
    h = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if path.is_file():
            rel = path.relative_to(root).as_posix()
            h.update(rel.encode("utf-8"))
            h.update(b"\0")
            h.update(path.read_bytes())
            h.update(b"\0")
    return h.hexdigest()


if __name__ == "__main__":
    unittest.main()
