"""Tests for the platform capability and release evidence audit rules.

Covers the scenarios in `openspec/changes/platform-capability-and-release-governance/specs/platform-capability-manifest/spec.md`:
- declared capabilities with valid evidence,
- missing/unverified evidence surfaced as review-required,
- deployable projects blocked without a complete release_evidence record,
- backward compatibility for projects that omit the new optional fields.
"""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))
from workspace_check import run

from _decisions_env import restore_decisions_register, use_empty_decisions_register


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


BASE_DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "product",
    "profile": "inventory",
    # The two taxonomy axes are carried on both authority surfaces now that the
    # audit is enabled. The declared composite is the bare role `product`, so
    # `role`/`stack` are the pair that derives it: derive_kind("product",
    # "none") == "product", which is why no TAXONOMY_LEGACY_VALUE follows.
    "role": "product",
    "stack": "none",
    "lifecycle": "inventory",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def make_args(root: Path, registry: Path) -> SimpleNamespace:
    return SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=False, self_check=False, as_json=False)


class CapabilityAndReleaseEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.project = self.root / "demo-project"
        self.project.mkdir()
        self.registry_path = self.root / "projects.json"
        self.declaration_path = self.project / ".project.json"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _write_project(self, declaration: dict, registry_entry: dict | None = None) -> None:
        write_json(self.declaration_path, declaration)
        registry = {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": [registry_entry or {
                "id": declaration["id"],
                "path": "demo-project",
                "profile": declaration["profile"],
                "role": declaration["role"],
                "stack": declaration["stack"],
                "lifecycle": declaration["lifecycle"],
                "adoption": "adopted",
            }],
        }
        write_json(self.registry_path, registry)

    def test_capabilities_and_release_evidence_absent_is_backward_compatible(self):
        """A project without the new optional fields must continue to pass cleanly."""
        self._write_project(BASE_DECLARATION)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertEqual(summary["errors"], 0, findings)
        self.assertEqual(summary["warnings"], 0, findings)

    def test_valid_capability_with_present_evidence_passes(self):
        """WHEN a project declares a capability with a present evidence_ref, no findings."""
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["capabilities"] = {
            "identity": {"owner": "platform", "evidence_state": "configured", "evidence_ref": "docs/identity.md"},
        }
        self._write_project(declaration)
        (self.project / "docs").mkdir()
        (self.project / "docs" / "identity.md").write_text("# Identity\n")
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertEqual(summary["errors"], 0, findings)
        self.assertEqual(summary["warnings"], 0, findings)

    def test_unverified_capability_is_review_required(self):
        """WHEN a declared capability has no native evidence, governance flags it as review-required."""
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["capabilities"] = {
            "billing": {"owner": "platform", "evidence_state": "unverified"},
        }
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertEqual(summary["errors"], 0, findings)
        codes = [item.code for item in findings]
        self.assertIn("CAPABILITY_PENDING", codes)

    def test_verified_capability_with_missing_evidence_ref_is_stale(self):
        """WHEN a capability claims verified but the evidence_ref is missing, governance surfaces an ERROR."""
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["capabilities"] = {
            "release": {"owner": "product", "evidence_state": "verified", "evidence_ref": "docs/release.md"},
        }
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertGreater(summary["errors"], 0)
        self.assertTrue(any(item.code == "CAPABILITY_STALE" for item in findings))

    def test_blocked_capability_is_surfaced_as_warning(self):
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["capabilities"] = {
            "jobs": {"owner": "platform", "evidence_state": "blocked"},
        }
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertEqual(summary["errors"], 0, findings)
        self.assertTrue(any(item.code == "CAPABILITY_BLOCKED" for item in findings))

    def test_invalid_capability_state_is_rejected(self):
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["capabilities"] = {
            "admin": {"owner": "product", "evidence_state": "guaranteed"},
        }
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertGreater(summary["errors"], 0)
        self.assertTrue(any(item.code == "CAPABILITY_STATE" for item in findings))

    def test_deployable_without_release_evidence_fails_closed(self):
        """WHEN a project is deployable but omits release_evidence, the audit fails closed."""
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["deployment"] = {"deployable": True, "jenkins_job": "demo-project-job", "compose_file": "docker-compose.yml"}
        (self.project / "docker-compose.yml").write_text("version: '3'\nservices: {}\n")
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertGreater(summary["errors"], 0)
        self.assertTrue(any(item.code == "RELEASE_EVIDENCE_REQUIRED" for item in findings))

    def test_deployable_with_complete_release_evidence_passes(self):
        """WHEN a deployable project supplies a complete release_evidence record, the audit passes the new rules."""
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["deployment"] = {"deployable": True, "jenkins_job": "demo-project-job", "compose_file": "docker-compose.yml"}
        (self.project / "docker-compose.yml").write_text("version: '3'\nservices: {}\n")
        (self.project / "release-artifacts").mkdir()
        (self.project / "release-artifacts" / "digest.txt").write_text("sha256:example\n")
        (self.project / "release-artifacts" / "sbom.json").write_text("{\"sbom\": true}\n")
        (self.project / "release-artifacts" / "provenance.json").write_text("{\"provenance\": true}\n")
        (self.project / "release-artifacts" / "checks.txt").write_text("cargo test --workspace\n")
        (self.project / "release-artifacts" / "publication.txt").write_text("internal-mirror:tag\n")
        declaration["release_evidence"] = {
            "revision": {"state": "verified", "value": "abc123"},
            "version": {"state": "declared", "value": "0.1.0"},
            "toolchain": {"state": "declared", "value": "rust-1.75"},
            "artifacts": {"state": "configured", "evidence_ref": "release-artifacts/digest.txt"},
            "digests": {"state": "configured", "evidence_ref": "release-artifacts/digest.txt"},
            "sbom": {"state": "configured", "evidence_ref": "release-artifacts/sbom.json"},
            "provenance": {"state": "configured", "evidence_ref": "release-artifacts/provenance.json"},
            "checks": {"state": "configured", "evidence_ref": "release-artifacts/checks.txt", "value": ["cargo test --workspace"]},
            "publication": {"state": "configured", "evidence_ref": "release-artifacts/publication.txt"},
        }
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        insufficient = [item for item in findings if item.code == "RELEASE_EVIDENCE_INSUFFICIENT"]
        self.assertEqual(insufficient, [], insufficient)

    def test_deployable_with_stale_release_evidence_ref_is_rejected(self):
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["deployment"] = {"deployable": True, "jenkins_job": "demo-project-job", "compose_file": "docker-compose.yml"}
        (self.project / "docker-compose.yml").write_text("version: '3'\nservices: {}\n")
        declaration["release_evidence"] = {
            "revision": {"state": "verified", "value": "abc123"},
            "artifacts": {"state": "verified", "evidence_ref": "release-artifacts/missing.txt"},
            "digests": {"state": "verified"},
            "provenance": {"state": "verified"},
            "checks": {"state": "verified"},
            "publication": {"state": "verified"},
        }
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertGreater(summary["errors"], 0)
        self.assertTrue(any(item.code == "RELEASE_EVIDENCE_STALE" for item in findings))

    def test_unknown_release_evidence_field_is_warning(self):
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["release_evidence"] = {"notes": {"state": "declared", "value": "extra"}}
        self._write_project(declaration)
        findings, summary = run(make_args(self.root, self.registry_path))
        self.assertEqual(summary["errors"], 0, findings)
        self.assertTrue(any(item.code == "RELEASE_EVIDENCE_FIELD" for item in findings))

    def test_capability_outside_profile_support_set_warns(self):
        """When a project declares a capability its profile does not list, governance warns."""
        governance_root = Path(__file__).resolve().parents[1]
        declaration = json.loads(json.dumps(BASE_DECLARATION))
        declaration["profile"] = "dotnet-library"  # this profile lists a narrow capability set
        declaration["capabilities"] = {
            "billing": {"owner": "platform", "evidence_state": "declared"},  # not in dotnet-library's supported set
        }
        self._write_project(declaration)
        from workspace_check import Finding, validate_project
        entry = {"id": "demo-project", "path": "demo-project", "profile": "dotnet-library", "lifecycle": "inventory", "adoption": "adopted"}
        local_findings: list[Finding] = []
        validate_project(entry, self.root, local_findings, governance_root=governance_root)
        self.assertTrue(any(item.code == "CAPABILITY_UNSUPPORTED" for item in local_findings))


if __name__ == "__main__":
    unittest.main()
