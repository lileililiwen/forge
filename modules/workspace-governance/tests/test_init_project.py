import unittest
from unittest.mock import patch
from pathlib import Path

import sys

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from init_project import apply_local_metadata, derive_id, resolve_project_id, main
from bind_gate_commands import ConcernOutcome, Outcome, PARTIAL, PROFILE_DEFAULTS, REFUSED, Report
from repository_metadata import build_discoverability


class DeriveIdTests(unittest.TestCase):
    def test_camel_case_becomes_kebab(self):
        self.assertEqual(derive_id("hestiaLab"), "hestia-lab")
        self.assertEqual(derive_id("campusMesh"), "campus-mesh")
        self.assertEqual(derive_id("careBridge"), "care-bridge")
        self.assertEqual(derive_id("openTeacherKit"), "open-teacher-kit")

    def test_plain_name_is_unchanged(self):
        self.assertEqual(derive_id("dharmatlas"), "dharmatlas")
        self.assertEqual(derive_id("trailCrew"), "trail-crew")


class ResolveProjectIdTests(unittest.TestCase):
    def test_derives_when_omitted(self):
        self.assertEqual(resolve_project_id("openTeacherKit", None), ("open-teacher-kit", None))

    def test_matching_explicit_id_accepted(self):
        self.assertEqual(resolve_project_id("hestiaLab", "hestia-lab"), ("hestia-lab", None))

    def test_conflicting_explicit_id_refused(self):
        resolved, error = resolve_project_id("hestiaLab", "something-else")
        self.assertIsNone(resolved)
        self.assertIn("conflicts", error)

    def test_dotted_directory_alias_allowed(self):
        self.assertEqual(resolve_project_id("lileililiwen.github.io", "lileililiwen-site"), ("lileililiwen-site", None))

    def test_invalid_explicit_id_refused(self):
        resolved, error = resolve_project_id("demo", "Bad_Id")
        self.assertIsNone(resolved)
        self.assertIn("must match", error)


class MetadataRepairTests(unittest.TestCase):
    def test_missing_semantic_fields_are_rejected(self):
        with self.assertRaises(ValueError):
            build_discoverability(description="", keywords=[], topics=[], audience=[], use_cases=[], search_phrases=[], homepage="", license_spdx="MIT", visibility="public")

    def test_explicit_metadata_is_normalized(self):
        metadata = build_discoverability(
            description="Repository governance tool",
            keywords=["repository", "governance"],
            topics=["Repository Governance"],
            audience=["developers"],
            use_cases=["audit projects"],
            search_phrases=["repository governance"],
            homepage="https://example.com",
            license_spdx="MIT",
            visibility="public",
        )
        self.assertEqual(metadata["topics"], ["repository-governance"])

    def test_apply_repair_adds_metadata_and_license(self):
        import json
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory)
            (target / ".project.json").write_text(json.dumps({"id": "demo"}), encoding="utf-8")
            metadata = build_discoverability(
                description="Repository governance tool",
                keywords=["repository"],
                topics=["repository-governance"],
                audience=["developers"],
                use_cases=["audit projects"],
                search_phrases=["repository governance"],
                homepage="https://example.com",
                license_spdx="MIT",
                visibility="public",
            )
            changed = apply_local_metadata(target, metadata, "MIT")
            self.assertEqual(changed, [".project.json", "LICENSE"])
            self.assertEqual(json.loads((target / ".project.json").read_text())["discoverability"], metadata)
            self.assertIn("MIT License", (target / "LICENSE").read_text())


class ScaffoldGateTests(unittest.TestCase):
    def run_init(self, target: Path, profile: str) -> int:
        with patch("sys.argv", ["init_project.py", str(target), "--profile", profile]):
            return main()

    def test_supported_profile_registry_includes_gate_workspace_profiles(self):
        self.assertTrue({
            "typescript-product", "python-product", "dotnet-product",
            "dotnet-library", "typescript-monorepo", "rust-platform",
            "tooling", "deployment", "inventory", "governance",
        }.issubset(PROFILE_DEFAULTS))

    def test_unknown_profile_is_rejected_before_target_creation(self):
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "new-project"
            with patch("sys.argv", ["init_project.py", str(target), "--profile", "typescript-produc"]):
                self.assertEqual(main(), 2)
            self.assertFalse(target.exists())

    def test_binder_runs_after_manifest_creation_for_only_the_new_repo(self):
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "new-project"

            def binder_call(root, **kwargs):
                self.assertEqual(root, target.parent)
                self.assertEqual(kwargs, {"apply": True, "only": ["new-project"], "check_clean": False})
                self.assertTrue((target / ".ai-gate" / "gate.yaml").is_file())
                self.assertTrue((target / ".project.json").is_file())
                return Report()

            with patch("init_project.run_gate_binder", side_effect=binder_call):
                self.assertEqual(self.run_init(target, "typescript-product"), 0)

    def test_binder_execution_failure_is_reported_and_scaffold_is_preserved(self):
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "new-project"
            report = Report()
            report.add(Outcome("new-project", REFUSED, "invalid manifest"))
            with patch("init_project.run_gate_binder", return_value=report):
                self.assertEqual(self.run_init(target, "typescript-product"), 2)
            self.assertTrue((target / ".ai-gate" / "gate.yaml").is_file())

    def test_completed_bind_with_unresolved_debt_is_reported_without_claiming_pass(self):
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "new-project"
            report = Report()
            report.add(Outcome(
                "new-project", PARTIAL, "four unresolved checks",
                concerns=[ConcernOutcome("new-project", "build", "UNRESOLVED", reason="no build input")],
            ))
            with patch("init_project.run_gate_binder", return_value=report):
                with patch("builtins.print") as output:
                    self.assertEqual(self.run_init(target, "typescript-product"), 0)
            printed = " ".join(str(call.args[0]) for call in output.call_args_list if call.args)
            self.assertIn("1 unresolved check(s)", printed)
            self.assertIn("remains blocked until those checks are bound", printed)


if __name__ == "__main__":
    unittest.main()
