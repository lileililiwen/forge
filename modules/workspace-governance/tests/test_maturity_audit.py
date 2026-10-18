import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import sys

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
import maturity_audit
import workspace_check


DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "product",
    "profile": "inventory",
    "lifecycle": "inventory",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "implemented"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}


class MaturityAuditDelegationTests(unittest.TestCase):
    def make_workspace(self, declaration=None):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        project = root / "demo-project"
        project.mkdir()
        (project / ".project.json").write_text(json.dumps(declaration or DECLARATION), encoding="utf-8")
        registry = {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": [{"id": "demo-project", "path": "demo-project", "profile": "inventory", "lifecycle": "inventory", "adoption": "adopted"}],
        }
        registry_path = root / "projects.json"
        registry_path.write_text(json.dumps(registry), encoding="utf-8")
        return root, registry_path

    def test_counts_match_authoritative_audit(self):
        root, registry = self.make_workspace()
        expected = workspace_check.run(SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=True, self_check=False, as_json=False))[1]
        actual = maturity_audit.authoritative_summary(root, str(registry))
        self.assertEqual(actual, expected)

    def test_counts_match_when_audit_fails(self):
        declaration = {k: v for k, v in DECLARATION.items() if k != "schema_version"}
        root, registry = self.make_workspace(declaration=declaration)
        actual = maturity_audit.authoritative_summary(root, str(registry))
        self.assertGreater(actual["errors"], 0)


if __name__ == "__main__":
    unittest.main()
