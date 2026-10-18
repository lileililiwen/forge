import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))
from auto_gate import run_gate

from _decisions_env import restore_decisions_register, use_empty_decisions_register


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


class AutoGateTests(unittest.TestCase):
    def test_clean_workspace_returns_machine_readable_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "demo-project"
            project.mkdir()
            (project / ".project.json").write_text(json.dumps({
                "schema_version": 1,
                "id": "demo-project",
                "kind": "product",
                "profile": "inventory",
                "lifecycle": "inventory",
                "verification": {"command": "true"},
                "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
            }))
            registry = root / "projects.json"
            registry.write_text(json.dumps({
                "schema_version": 1,
                "workspace_root": None,
                "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
                "projects": [{"id": "demo-project", "path": "demo-project", "profile": "inventory", "lifecycle": "inventory", "adoption": "adopted"}],
            }))

            report, exit_code = run_gate(root, registry, check_deployment=False)

            self.assertEqual(exit_code, 0)
            self.assertEqual(report["summary"]["errors"], 0)
            self.assertEqual(report["status"], "PASS")


if __name__ == "__main__":
    unittest.main()
