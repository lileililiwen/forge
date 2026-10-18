"""Regression coverage for repository-owned verification inputs."""

import json
import sys
import unittest
from pathlib import Path

from workspace_root import resolve_workspace_root

REPO_ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = resolve_workspace_root(REPO_ROOT)
sys.path.insert(0, str(REPO_ROOT / "tests"))
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from test_promote_project import (  # noqa: E402
    GOVERNANCE_ROOT_DATA_FILES,
    setup_sandboxed_governance_root,
)


class WorkspaceVerificationBaselineTests(unittest.TestCase):
    def test_mnemora_registry_matches_its_declared_identity(self):
        declaration = json.loads(
            (WORKSPACE_ROOT / "mnemora" / ".project.json").read_text(encoding="utf-8")
        )
        registry = json.loads((REPO_ROOT / "projects.json").read_text(encoding="utf-8"))
        matches = [entry for entry in registry["projects"] if entry.get("id") == "mnemora"]
        self.assertEqual(len(matches), 1)
        self.assertEqual(
            {key: matches[0].get(key) for key in ("id", "path", "profile", "role", "stack")},
            {key: declaration.get(key) for key in ("id", "profile", "role", "stack")}
            | {"path": "mnemora"},
        )

    def test_tracked_taxonomy_and_ui_records_are_copied_to_the_sandbox(self):
        expected = {
            "registry-remote-drift.json",
            "tag-taxonomy.json",
            "ui-adoption-migration.json",
        }
        self.assertTrue(expected.issubset(set(GOVERNANCE_ROOT_DATA_FILES)))
        temp, sandbox = setup_sandboxed_governance_root(REPO_ROOT)
        try:
            for name in expected:
                self.assertTrue((sandbox / name).is_file(), name)
        finally:
            temp.cleanup()


if __name__ == "__main__":
    unittest.main()
