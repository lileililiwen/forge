from __future__ import annotations

import json
import unittest
from pathlib import Path


FORGE_ROOT = Path(__file__).resolve().parents[3]
WORKSPACE_ROOT = FORGE_ROOT.parent
MODULES_ROOT = FORGE_ROOT / "modules"


class PlatformModulePathTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.registry = json.loads(
            (MODULES_ROOT / "workspace-governance" / "projects.json").read_text(encoding="utf-8")
        )
        cls.reconciliation = json.loads(
            (MODULES_ROOT / "workspace-governance" / "migration" / "forge-monorepo-path-reconciliation.json").read_text(encoding="utf-8")
        )

    def test_dotnet_project_path_resolves_to_the_forge_module(self) -> None:
        project = next(item for item in self.registry["projects"] if item["id"] == "dotnet-platform-libs")
        self.assertEqual(project["path"], "forge/modules/dotnet-platform-libs")
        self.assertEqual(WORKSPACE_ROOT / project["path"] / ".project.json", MODULES_ROOT / "dotnet-platform-libs" / ".project.json")

    def test_dotnet_import_reconciliation_preserves_project_identity(self) -> None:
        entry = next(item for item in self.reconciliation["changes"] if item["id"] == "dotnet-platform-libs")
        self.assertEqual(entry["from"], "dotnet-platform-libs")
        self.assertEqual(entry["to"], "forge/modules/dotnet-platform-libs")
        self.assertEqual(WORKSPACE_ROOT / entry["declaration"], MODULES_ROOT / "dotnet-platform-libs" / ".project.json")

    def test_ui_token_sources_are_module_relative(self) -> None:
        ui_root = MODULES_ROOT / "dotnet-platform-libs" / "ui"
        self.assertTrue((ui_root / "tokens" / "tokens.json").is_file())
        self.assertTrue((ui_root / "scripts" / "lib" / "token-model.mjs").is_file())
        test_source = (MODULES_ROOT / "workspace-governance" / "tests" / "test_ui_conformance.py").read_text(encoding="utf-8")
        self.assertNotIn("/home/paul/code/dotnet-platform-libs", test_source)


if __name__ == "__main__":
    unittest.main()
