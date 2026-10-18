import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import sys

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from workspace_check import run


DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "product",
    "profile": "inventory",
    "lifecycle": "inventory",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "implemented"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}
ENTRY = {"id": "demo-project", "path": "demo-project", "profile": "inventory", "lifecycle": "inventory", "adoption": "adopted"}


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


class RegistryIntegrityTests(unittest.TestCase):
    def make_workspace(self, declaration=None, entries=None, discovery=None):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        project = root / "demo-project"
        project.mkdir()
        write_json(project / ".project.json", declaration if declaration is not None else dict(DECLARATION))
        registry = {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": discovery or {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": [dict(ENTRY)] if entries is None else entries,
        }
        registry_path = root / "projects.json"
        write_json(registry_path, registry)
        return root, registry_path

    def audit(self, root, registry):
        return run(SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=True, self_check=False, as_json=False))

    def test_unregistered_discovery_with_fail_policy_is_error(self):
        root, registry = self.make_workspace(entries=[], discovery={"mode": "immediate-directories", "unregistered_policy": "fail"})
        findings, summary = self.audit(root, registry)
        self.assertGreaterEqual(summary["errors"], 1, findings)
        self.assertTrue(any(item.code == "DISCOVERED_UNREGISTERED" and item.level == "ERROR" for item in findings))

    def test_duplicate_registry_path_is_error(self):
        other = dict(ENTRY, id="demo-project-2")
        root, registry = self.make_workspace(entries=[dict(ENTRY), other])
        findings, summary = self.audit(root, registry)
        self.assertTrue(any(item.code == "PROJECT_PATH_DUPLICATE" for item in findings), findings)

    def test_duplicate_registry_id_is_error(self):
        root, registry = self.make_workspace(entries=[dict(ENTRY), dict(ENTRY)])
        findings, summary = self.audit(root, registry)
        self.assertTrue(any(item.code == "PROJECT_DUPLICATE" for item in findings), findings)

    def test_declaration_id_mismatch_is_error(self):
        root, registry = self.make_workspace(declaration=dict(DECLARATION, id="different-id"))
        findings, summary = self.audit(root, registry)
        self.assertTrue(any(item.code == "DECLARATION_ID" for item in findings), findings)

    def test_missing_schema_version_is_error(self):
        declaration = {k: v for k, v in DECLARATION.items() if k != "schema_version"}
        root, registry = self.make_workspace(declaration=declaration)
        findings, summary = self.audit(root, registry)
        self.assertTrue(any(item.code == "DECLARATION_SCHEMA" for item in findings), findings)

    def test_adopted_without_declaration_is_error(self):
        root, registry = self.make_workspace()
        (root / "demo-project" / ".project.json").unlink()
        findings, summary = self.audit(root, registry)
        self.assertTrue(any(item.code == "DECLARATION_MISSING" for item in findings), findings)


if __name__ == "__main__":
    unittest.main()
