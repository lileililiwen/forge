import tomllib
import unittest
from pathlib import Path


MODULE_ROOT = Path(__file__).resolve().parents[1]


class GateManifestContractTests(unittest.TestCase):
    def test_script_modules_are_syntactically_valid(self):
        for script in sorted((MODULE_ROOT / "scripts").glob("*.py")):
            with self.subTest(script=script.name):
                compile(script.read_text(), str(script), "exec")

    def test_native_manifest_preserves_governance_checks_and_scans_python_scripts(self):
        manifest = tomllib.loads((MODULE_ROOT / "gate.toml").read_text())

        self.assertEqual(manifest["version"], 1)
        self.assertEqual(manifest["profile"], "governance")
        self.assertEqual(manifest["rule_pack_version"], "workspace-governance@1")
        self.assertNotIn("profiles", manifest)
        self.assertEqual(
            {check["id"]: check["command"] for check in manifest["checks"]},
            {
                "declared-verification": "python3 -m unittest discover -s tests -v",
                # The module checks itself. `--root ..` rooted the audit at the
                # module's parent directory inside the Forge tree, where the
                # registry's project paths do not resolve, and reported 87
                # PROJECT_MISSING errors. `--self-check` roots the audit at the
                # module that owns the registry.
                "governance-quality": "python3 ../workspace-governance/scripts/workspace_check.py --self-check --json",
                "product_quality": "python3 scripts/product_code_quality.py .",
                "repository": "git diff --check",
                "security": "pip-audit -r requirements-dev.txt",
                "tests": "python3 -m unittest discover -s tests -v",
            },
        )
        self.assertTrue(manifest["blocking"]["review_required_blocks"])
        self.assertEqual(manifest["source_size"], {"include": ["scripts/**/*.py"]})


if __name__ == "__main__":
    unittest.main()
