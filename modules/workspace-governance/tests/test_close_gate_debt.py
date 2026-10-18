import sys
import tempfile
import unittest
import json
import io
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
import close_gate_debt as driver
from check_concern import (
    check_accessibility,
    check_fixtures,
    check_privacy,
    check_provenance,
    check_schemas,
)


class ClassificationTests(unittest.TestCase):
    def test_current_debt_reason_families_map_to_a_through_g(self):
        cases = {
            "no rule derives an accessibility command and this repository has no scripts/check-accessibility.py to run": "A",
            "2 .csproj files and no solution": "B",
            "a pure-Python package has no build step": "C",
            "no readable pyproject.toml declares a Python build backend": "C",
            "the build-system exists but the `build` frontend is not declared": "C",
            "no Python security tool is declared": "D",
            "2 equally plausible entry points: a vs b": "E",
            "no package.json declares a 'build' script": "F",
            "gate manifest is dirty; binding remains unbound: python3 check.py": "G",
        }
        for reason, expected in cases.items():
            with self.subTest(reason=reason):
                self.assertEqual(driver.classify(reason), expected)

    def test_unknown_reason_escalates_without_classifying_as_a_remedy(self):
        self.assertEqual(driver.classify("new binder refusal wording"), "?")

    def test_dirty_manifest_class_is_owner_action_without_mutation_in_apply_mode(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / ".ai-gate" / "gate.yaml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text("profile: tooling\n", encoding="utf-8")
            before = manifest.read_bytes()
            report = driver.Report(root=root, apply=True)
            report._one(
                "demo", "governance-quality",
                "gate manifest is dirty; binding remains unbound: python3 check.py",
            )

            self.assertEqual(len(report.remedies), 1)
            remedy = report.remedies[0]
            self.assertEqual(remedy.kind, "G")
            self.assertFalse(remedy.applied)
            self.assertIn("review and commit", remedy.question)
            self.assertEqual(manifest.read_bytes(), before)

    def test_dirty_manifest_classifier_requires_the_binder_reason_marker(self):
        self.assertEqual(driver.classify("gate manifest dirty; command not bound"), "?")
        self.assertEqual(
            driver.classify("prefix: gate manifest is dirty; binding remains unbound: python3 check.py"),
            "?",
        )
        self.assertEqual(
            driver.classify("gate manifest is dirty; binding remains unbound:   "),
            "?",
        )

    def test_malformed_debt_record_fails_closed_without_raising(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / driver.DEBT_FILE).write_text(
                json.dumps({"repositories": [{"repo": "demo", "unresolved": [{"concern": "build"}]}]}),
                encoding="utf-8",
            )
            output = io.StringIO()
            errors = io.StringIO()
            with patch.object(driver, "GOVERNANCE_ROOT", root), redirect_stdout(output), redirect_stderr(errors):
                code = driver.main(["--root", str(root), "--dry"])
            self.assertEqual(code, 2)
            self.assertIn("invalid debt record", errors.getvalue())


class SolutionRemedyTests(unittest.TestCase):
    def test_existing_slnx_adds_only_missing_projects_and_verifies_all(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / "src" / "One.csproj"
            second = root / "tests" / "Two.Tests.csproj"
            first.parent.mkdir(parents=True)
            second.parent.mkdir(parents=True)
            first.write_text("<Project />", encoding="utf-8")
            second.write_text("<Project />", encoding="utf-8")
            solution = root / "App.slnx"
            solution.write_text(
                '<Solution><Project Path="src/One.csproj" /></Solution>\n',
                encoding="utf-8",
            )

            def add_project(command, cwd):
                path = command[-1]
                body = solution.read_text(encoding="utf-8")
                solution.write_text(
                    body.replace("</Solution>", f'<Project Path="{path}" /></Solution>'),
                    encoding="utf-8",
                )
                return 0, "added"

            with patch.object(driver, "_run", side_effect=add_project) as run:
                ok, detail = driver.remedy_b(root, "build")

            self.assertTrue(ok, detail)
            run.assert_called_once()
            self.assertIn("all 2 discovered project(s)", detail)
            self.assertIn('Path="src/One.csproj"', solution.read_text(encoding="utf-8"))
            self.assertIn('Path="tests/Two.Tests.csproj"', solution.read_text(encoding="utf-8"))
            with patch.object(driver, "_run") as rerun:
                ok, detail = driver.remedy_b(root, "build")
            self.assertTrue(ok, detail)
            rerun.assert_not_called()
            self.assertIn("added 0", detail)

    def test_exit_zero_with_dotnet_add_error_is_not_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "App.csproj"
            project.write_text("<Project />", encoding="utf-8")
            (root / "App.slnx").write_text("<Solution />\n", encoding="utf-8")
            with patch.object(driver, "_run", return_value=(0, "Project or directory not found")):
                ok, detail = driver.remedy_b(root, "build")
            self.assertFalse(ok)
            self.assertIn("not found", detail)

    def test_empty_solution_is_rejected_when_cli_did_not_add_projects(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "App.csproj").write_text("<Project />", encoding="utf-8")
            (root / "App.slnx").write_text("<Solution />\n", encoding="utf-8")
            with patch.object(driver, "_run", return_value=(0, "")):
                ok, detail = driver.remedy_b(root, "build")
            self.assertFalse(ok)
            self.assertIn("missing project(s)", detail)

    def test_legacy_sln_project_entries_are_verified_by_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "App.csproj"
            project.write_text("<Project />", encoding="utf-8")
            solution = root / "App.sln"
            solution.write_text("Microsoft Visual Studio Solution File\n", encoding="utf-8")

            def add_project(_command, cwd):
                self.assertEqual(cwd, root)
                solution.write_text(
                    solution.read_text(encoding="utf-8")
                    + 'Project("{A}") = "App", "App.csproj", "{B}"\nEndProject\n',
                    encoding="utf-8",
                )
                return 0, "added"

            with patch.object(driver, "_run", side_effect=add_project):
                ok, detail = driver.remedy_b(root, "build")
            self.assertTrue(ok, detail)
            self.assertIn("all 1 discovered project(s)", detail)


class PythonBuildRemedyTests(unittest.TestCase):
    def test_existing_python_package_gets_a_derivable_build_frontend(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "pyproject.toml"
            project.write_text("[project]\nname = 'demo'\n", encoding="utf-8")
            ok, detail = driver.remedy_c(root, "build")
            self.assertTrue(ok, detail)
            self.assertIn("[build-system]", project.read_text(encoding="utf-8"))
            self.assertIn("build>=1", (root / "requirements-dev.txt").read_text(encoding="utf-8"))

    def test_repository_without_package_metadata_is_not_made_into_a_fake_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "manage.py").write_text("print('app')\n", encoding="utf-8")
            ok, detail = driver.remedy_c(root, "build")
            self.assertFalse(ok)
            self.assertIn("owner must identify", detail)
            self.assertFalse((root / "pyproject.toml").exists())

    def test_build_configuration_without_project_metadata_is_not_packaged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "pyproject.toml"
            project.write_text("[tool.ruff]\nline-length = 100\n", encoding="utf-8")
            ok, detail = driver.remedy_c(root, "build")
            self.assertFalse(ok)
            self.assertIn("distribution metadata", detail)
            self.assertEqual(project.read_text(encoding="utf-8"), "[tool.ruff]\nline-length = 100\n")


class RemedyIdempotencyTests(unittest.TestCase):
    def test_existing_checker_launcher_is_kept_without_duplicate_write(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "scripts" / "check-privacy.py"
            target.parent.mkdir()
            target.write_text("# existing reviewed launcher\n", encoding="utf-8")
            before = target.read_bytes()
            ok, detail = driver.remedy_a(root, "privacy")
            self.assertTrue(ok)
            self.assertIn("already exists", detail)
            self.assertEqual(target.read_bytes(), before)

    def test_existing_pip_audit_declaration_is_run_without_duplicate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            requirements = root / "requirements-dev.txt"
            requirements.write_text("pip-audit>=2\n", encoding="utf-8")
            before = requirements.read_bytes()
            with patch.object(driver, "_run", return_value=(1, "network unavailable")) as run:
                ok, detail = driver.remedy_d(root, "security")
            self.assertTrue(ok)
            self.assertIn("FAILED", detail)
            self.assertIn("network unavailable", detail)
            self.assertEqual(requirements.read_bytes(), before)
            run.assert_called_once()

    def test_dry_report_does_not_write_a_checker(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "demo").mkdir()
            report = driver.Report(root=root, apply=False)
            report._one("demo", "privacy", "no rule derives a privacy command and no scripts/check-privacy.py")
            self.assertFalse((root / "demo" / "scripts" / "check-privacy.py").exists())
            self.assertEqual(report.remedies[0].kind, "A")


class ConcernCheckerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def write(self, relative, text):
        target = self.root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
        return target

    def test_accessibility_finds_missing_document_language(self):
        self.write("index.html", "<html><body><h1>Title</h1></body></html>\n")
        findings = check_accessibility(self.root)
        self.assertEqual(len(findings), 1)
        self.assertEqual((findings[0].file, findings[0].line), ("index.html", 1))

    def test_privacy_finds_a_literal_personal_email(self):
        self.write("src/app.py", 'contact = "person@company.invalid"\n')
        findings = check_privacy(self.root)
        self.assertTrue(any("literal email address" in finding.message for finding in findings))

    def test_provenance_requires_a_third_party_notice_when_dependencies_exist(self):
        self.write("package.json", '{"dependencies":{"demo":"^1.0"}}\n')
        findings = check_provenance(self.root)
        self.assertEqual(len(findings), 1)
        self.assertIn("no third-party notice", findings[0].message)
        self.write("THIRD-PARTY-NOTICES", "demo: MIT\n")
        self.assertEqual(check_provenance(self.root), [])

    def test_fixtures_match_schema_names_through_directory_ancestors(self):
        self.write("schemas/record.schema.json", '{"type":"object"}\n')
        self.write("fixtures/record/cache/valid.json", '{"id":1}\n')
        self.assertEqual(check_fixtures(self.root), [])

    def test_schemas_report_malformed_json(self):
        self.write("schemas/record.schema.json", '{not json}\n')
        findings = check_schemas(self.root)
        self.assertEqual(len(findings), 1)
        self.assertIn("schema is not valid JSON", findings[0].message)


if __name__ == "__main__":
    unittest.main()
