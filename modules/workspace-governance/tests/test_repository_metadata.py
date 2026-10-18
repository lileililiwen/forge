import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import sys

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from repository_metadata import REQUIRED_DISCOVERABILITY_FIELDS, validate_discoverability
from workspace_check import run


class RepositoryMetadataTests(unittest.TestCase):
    def test_missing_metadata_has_stable_finding(self):
        findings = []
        validate_discoverability({}, Path("."), findings, "demo")
        self.assertEqual([item.code for item in findings], ["DISCOVERABILITY_MISSING"])

    def test_invalid_metadata_reports_fields(self):
        findings = []
        metadata = {
            "description": "",
            "keywords": ["repository"],
            "topics": ["Bad Topic"],
            "audience": ["developers"],
            "use_cases": ["audit repositories"],
            "search_phrases": ["repository governance tool"],
            "homepage": "https://example.com/tool",
            "license_spdx": "MIT",
            "visibility": "unknown",
        }
        validate_discoverability(
            {"discoverability": metadata},
            Path("."), findings, "demo",
        )
        self.assertTrue(any(item.code == "DISCOVERABILITY_INVALID" for item in findings))
        self.assertTrue(any("description" in item.message for item in findings))

    def test_complete_metadata_passes(self):
        declaration = {
            "discoverability": {
                "description": "A useful repository tool.",
                "keywords": ["repository", "tool"],
                "topics": ["repository-tool"],
                "audience": ["developers"],
                "use_cases": ["audit repositories"],
                "search_phrases": ["repository governance tool"],
                "homepage": "https://example.com/tool",
                "license_spdx": "MIT",
                "visibility": "public",
            }
        }
        findings = []
        validate_discoverability(declaration, Path("."), findings, "demo")
        self.assertEqual(findings, [])
        self.assertEqual(set(REQUIRED_DISCOVERABILITY_FIELDS), set(declaration["discoverability"]))

    def test_product_audit_exposes_missing_license_and_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "demo"
            project.mkdir()
            declaration = {
                "schema_version": 1,
                "id": "demo",
                "kind": "product",
                "profile": "product",
                "lifecycle": "active",
                "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
                "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
            }
            (project / ".project.json").write_text(json.dumps(declaration), encoding="utf-8")
            (project / "README.md").write_text("# Demo\n", encoding="utf-8")
            registry = {
                "schema_version": 1,
                "workspace_root": None,
                "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
                "projects": [{"id": "demo", "path": "demo", "profile": "product", "lifecycle": "active", "adoption": "adopted"}],
            }
            registry_path = root / "projects.json"
            registry_path.write_text(json.dumps(registry), encoding="utf-8")
            findings, summary = run(SimpleNamespace(registry=str(registry_path), root=str(root), project=None, audit=True, self_check=False, as_json=False))
            self.assertGreaterEqual(summary["errors"], 2)
            self.assertTrue(any(item.code == "LICENSE_MISSING" for item in findings))
            self.assertTrue(any(item.code == "DISCOVERABILITY_MISSING" for item in findings))
            self.assertEqual((project / ".project.json").read_text(encoding="utf-8"), json.dumps(declaration))

    def test_self_check_does_not_require_live_github(self):
        findings = []
        declaration = {"discoverability": {}}
        validate_discoverability(declaration, Path("."), findings, "workspace-governance")
        self.assertTrue(any(item.code == "DISCOVERABILITY_INCOMPLETE" for item in findings))


if __name__ == "__main__":
    unittest.main()
