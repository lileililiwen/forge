from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-dotnet-platform-libs-import.py"
ROOT = SCRIPT.parents[1]
SPEC = importlib.util.spec_from_file_location("dotnet_platform_libraries_import", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load import checker: {SCRIPT}")
IMPORT_CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORT_CHECK)


class ImportedTreeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.prefix = b"modules/dotnet-platform-libs/"
        self.moved = {
            b".github/workflows/quality.yml": b".github/workflows/platform-dotnet-quality.yml",
            b".github/workflows/release.yml": b".github/workflows/platform-dotnet-release.yml",
        }
        self.source = {
            b"README.md": (b"100644", b"readme-sha"),
            b"Directory.Build.props": (b"100644", b"props-sha"),
            b".github/workflows/quality.yml": (b"100644", b"quality-sha"),
            b".github/workflows/release.yml": (b"100644", b"release-sha"),
        }
        self.destination = {
            self.prefix + b"README.md": (b"100644", b"readme-sha"),
            self.prefix + b"Directory.Build.props": (b"100644", b"props-sha"),
            b".github/workflows/platform-dotnet-quality.yml": (b"100644", b"quality-new-sha"),
            b".github/workflows/platform-dotnet-release.yml": (b"100644", b"release-new-sha"),
        }

    def test_exact_tree_and_declared_workflow_moves_pass(self) -> None:
        problems = IMPORT_CHECK.compare_trees(
            self.source,
            self.destination,
            prefix=self.prefix,
            moved_paths=self.moved,
            adapted_source_paths=set(self.moved),
            added_destination_paths=set(),
        )

        self.assertEqual(problems, [])

    def test_unlisted_source_content_change_fails(self) -> None:
        changed = dict(self.destination)
        changed[self.prefix + b"README.md"] = (b"100644", b"edited-readme")

        problems = IMPORT_CHECK.compare_trees(
            self.source,
            changed,
            prefix=self.prefix,
            moved_paths=self.moved,
            adapted_source_paths=set(self.moved),
            added_destination_paths=set(),
        )

        self.assertTrue(any("README.md" in problem and "blob" in problem for problem in problems))

    def test_allowed_adaptation_still_requires_same_mode_and_path(self) -> None:
        changed = dict(self.destination)
        changed[self.prefix + b"Directory.Build.props"] = (b"100644", b"forge-repo-url")

        problems = IMPORT_CHECK.compare_trees(
            self.source,
            changed,
            prefix=self.prefix,
            moved_paths=self.moved,
            adapted_source_paths={b"Directory.Build.props", *self.moved},
            added_destination_paths=set(),
        )

        self.assertEqual(problems, [])

        changed[self.prefix + b"Directory.Build.props"] = (b"100755", b"forge-repo-url")
        problems = IMPORT_CHECK.compare_trees(
            self.source,
            changed,
            prefix=self.prefix,
            moved_paths=self.moved,
            adapted_source_paths={b"Directory.Build.props", *self.moved},
            added_destination_paths=set(),
        )
        self.assertTrue(any("mode" in problem for problem in problems))

    def test_missing_and_extra_paths_fail(self) -> None:
        destination = dict(self.destination)
        destination.pop(self.prefix + b"README.md")
        destination[b"modules/dotnet-platform-libs/EXTRA.txt"] = (b"100644", b"extra")

        problems = IMPORT_CHECK.compare_trees(
            self.source,
            destination,
            prefix=self.prefix,
            moved_paths=self.moved,
            adapted_source_paths=set(self.moved),
            added_destination_paths=set(),
        )

        self.assertTrue(any("missing" in problem and "README.md" in problem for problem in problems))
        self.assertTrue(any("unexpected" in problem and "EXTRA.txt" in problem for problem in problems))

    def test_whitespace_normalization_only_removes_extra_final_blank_lines(self) -> None:
        normalize = IMPORT_CHECK.normalized_final_newlines

        self.assertEqual(normalize(b"content\n"), normalize(b"content\n\n\n"))
        self.assertNotEqual(normalize(b"content\n"), normalize(b"changed\n\n"))
        self.assertNotEqual(normalize(b"content\n"), normalize(b"content \n"))


class SourceHistoryTests(unittest.TestCase):
    def git(self, root: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(root), *args],
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout.strip()

    def commit(self, root: Path, message: str) -> str:
        self.git(root, "add", "--", ".")
        self.git(root, "-c", "user.name=Import Test", "-c", "user.email=import-test@example.invalid", "commit", "-m", message)
        return self.git(root, "rev-parse", "HEAD")

    def test_all_source_only_commits_must_be_ancestors_of_import(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            target = root / "target"
            source.mkdir()
            target.mkdir()
            self.git(source, "init", "-q")
            (source / "base.txt").write_text("base\n")
            baseline = self.commit(source, "source baseline")
            (source / "library.txt").write_text("library\n")
            source_head = self.commit(source, "source-only commit")

            self.git(target, "init", "-q")
            self.git(target, "fetch", str(source), source_head)
            (target / "forge.txt").write_text("forge\n")
            target_head = self.commit(target, "forge baseline")
            tree = self.git(target, "rev-parse", "HEAD^{tree}")
            imported = self.git(
                target,
                "-c",
                "user.name=Import Test",
                "-c",
                "user.email=import-test@example.invalid",
                "commit-tree",
                tree,
                "-p",
                target_head,
                "-p",
                source_head,
                "-m",
                "source import",
            )
            branch = self.git(target, "symbolic-ref", "HEAD")
            self.git(target, "update-ref", branch, imported, target_head)

            problems = IMPORT_CHECK.check_source_ancestry(
                target,
                source_revision=source_head,
                source_baseline=baseline,
                pending_merge_heads=(),
            )

            self.assertEqual(problems, [])

            self.git(target, "update-ref", branch, target_head, imported)
            problems = IMPORT_CHECK.check_source_ancestry(
                target,
                source_revision=source_head,
                source_baseline=baseline,
                pending_merge_heads=(target_head,),
            )
            self.assertTrue(any("source-only" in problem for problem in problems))


class WorkflowBoundaryTests(unittest.TestCase):
    def test_forge_ci_fetches_history_for_import_ancestry_check(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        import_step = workflow.index("Verify the in-tree .NET platform import")
        checkout_step = workflow.rfind("fetch-depth: 0", 0, import_step)

        self.assertGreaterEqual(checkout_step, 0)

    def test_quality_workflow_covers_module_governance_and_root_workflow_changes(self) -> None:
        workflow = (ROOT / ".github/workflows/platform-dotnet-quality.yml").read_text()

        self.assertIn("'modules/dotnet-platform-libs/**'", workflow)
        self.assertIn("'modules/workspace-governance/projects.json'", workflow)
        self.assertIn("'.github/workflows/platform-dotnet-quality.yml'", workflow)
        self.assertIn("working-directory: modules/dotnet-platform-libs", workflow)
        self.assertIn("./scripts/quality-gate.sh", workflow)
        self.assertIn("./scripts/conformance.sh", workflow)
        self.assertIn("./scripts/audit-packages.sh", workflow)

    def test_release_is_manual_and_targets_forge_feed(self) -> None:
        workflow = (ROOT / ".github/workflows/platform-dotnet-release.yml").read_text()

        self.assertIn("workflow_dispatch:", workflow)
        self.assertNotIn("  pull_request:", workflow)
        self.assertNotIn("  push:", workflow)
        self.assertIn("https://lileililiwen.github.io/forge/nuget/", workflow)
        self.assertIn("dotnet-version: 10.0.400", workflow)
        self.assertIn("const semver = new RegExp", workflow)
        self.assertIn("VERSION must be SemVer", workflow)
        self.assertIn("./scripts/quality-gate.sh", workflow)
        self.assertIn("./scripts/check-public-api.sh", workflow)
        self.assertIn("./scripts/audit-packages.sh", workflow)
        self.assertIn("./scripts/conformance.sh", workflow)

    def test_workspace_bootstrap_script_has_no_machine_specific_workspace_guess(self) -> None:
        script = (ROOT / "modules/dotnet-platform-libs/scripts/test-workspace-bootstrap.sh").read_text()

        self.assertIn("FORGE_WORKSPACE_ROOT", script)
        self.assertIn("mktemp -d", script)
        self.assertNotIn("/home/paul/code", script)


if __name__ == "__main__":
    unittest.main()
