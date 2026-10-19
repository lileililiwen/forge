from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "check-rust-platform-libs-import.py"
ROOT = SCRIPT.parents[1]
SPEC = importlib.util.spec_from_file_location("rust_platform_libraries_import", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load import checker: {SCRIPT}")
IMPORT_CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORT_CHECK)


class ImportedTreeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.prefix = b"modules/rust-platform-libs/"
        self.moved = {
            b".github/workflows/ci.yml": b".github/workflows/platform-rust-quality.yml",
            b".github/workflows/publish.yml": b".github/workflows/platform-rust-publish.yml",
        }
        self.source = {
            b"Cargo.toml": (b"100644", b"cargo-original"),
            b"crates/platform-core/src/lib.rs": (b"100644", b"core-source"),
            b".github/workflows/ci.yml": (b"100644", b"ci-source"),
            b".github/workflows/publish.yml": (b"100644", b"publish-source"),
        }
        self.destination = {
            self.prefix + b"Cargo.toml": (b"100644", b"cargo-forge"),
            self.prefix + b"crates/platform-core/src/lib.rs": (b"100644", b"core-source"),
            b".github/workflows/platform-rust-quality.yml": (b"100644", b"ci-forge"),
            b".github/workflows/platform-rust-publish.yml": (b"100644", b"publish-forge"),
        }

    def compare(self, destination: dict[bytes, tuple[bytes, bytes]]) -> list[str]:
        return IMPORT_CHECK.compare_trees(
            self.source,
            destination,
            prefix=self.prefix,
            moved_paths=self.moved,
            adapted_source_paths={b"Cargo.toml", *self.moved},
            added_destination_paths=set(),
        )

    def test_exact_tree_and_declared_adaptations_pass(self) -> None:
        self.assertEqual(self.compare(self.destination), [])

    def test_undeclared_content_drift_fails(self) -> None:
        changed = dict(self.destination)
        changed[self.prefix + b"crates/platform-core/src/lib.rs"] = (b"100644", b"edited")
        self.assertTrue(any("blob" in item for item in self.compare(changed)))

    def test_adaptation_does_not_allow_mode_drift(self) -> None:
        changed = dict(self.destination)
        changed[self.prefix + b"Cargo.toml"] = (b"100755", b"cargo-forge")
        self.assertTrue(any("mode" in item for item in self.compare(changed)))

    def test_missing_and_unexpected_paths_fail(self) -> None:
        changed = dict(self.destination)
        changed.pop(self.prefix + b"crates/platform-core/src/lib.rs")
        changed[self.prefix + b"EXTRA"] = (b"100644", b"extra")
        errors = self.compare(changed)
        self.assertTrue(any("missing" in item for item in errors))
        self.assertTrue(any("unexpected" in item for item in errors))

    def test_whitespace_normalization_only_removes_extra_final_newlines(self) -> None:
        normalize = IMPORT_CHECK.normalized_final_newlines
        self.assertEqual(normalize(b"content\n"), normalize(b"content\n\n\n"))
        self.assertNotEqual(normalize(b"content\n"), normalize(b"changed\n\n"))
        self.assertNotEqual(normalize(b"content\n"), normalize(b"content \n"))


class SourceHistoryTests(unittest.TestCase):
    def git(self, root: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(root), *args], check=True, capture_output=True, text=True
        )
        return result.stdout.strip()

    def commit(self, root: Path, message: str) -> str:
        self.git(root, "add", "--", ".")
        self.git(
            root,
            "-c", "user.name=Import Test", "-c", "user.email=import-test@example.invalid",
            "commit", "-m", message,
        )
        return self.git(root, "rev-parse", "HEAD")

    def test_every_source_commit_must_be_in_forge_ancestry(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, target = root / "source", root / "target"
            source.mkdir()
            target.mkdir()
            self.git(source, "init", "-q")
            (source / "base.txt").write_text("base\n")
            self.commit(source, "source base")
            (source / "module.txt").write_text("module\n")
            source_head = self.commit(source, "source head")

            self.git(target, "init", "-q")
            self.git(target, "fetch", str(source), source_head)
            (target / "forge.txt").write_text("forge\n")
            target_head = self.commit(target, "forge base")
            tree = self.git(target, "rev-parse", "HEAD^{tree}")
            imported = self.git(
                target, "-c", "user.name=Import Test", "-c",
                "user.email=import-test@example.invalid", "commit-tree", tree,
                "-p", target_head, "-p", source_head, "-m", "source import",
            )
            branch = self.git(target, "symbolic-ref", "HEAD")
            self.git(target, "update-ref", branch, imported, target_head)
            with patch.object(IMPORT_CHECK, "SOURCE_COMMIT_COUNT", 2):
                self.assertEqual(IMPORT_CHECK.check_source_ancestry(target, source_revision=source_head), [])

            self.git(target, "update-ref", branch, target_head, imported)
            with patch.object(IMPORT_CHECK, "SOURCE_COMMIT_COUNT", 2):
                errors = IMPORT_CHECK.check_source_ancestry(target, source_revision=source_head)
            self.assertTrue(any("not preserved" in item for item in errors))

    def test_git_tree_reader_ignores_untracked_source_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            self.git(source, "init", "-q")
            (source / "tracked.txt").write_text("tracked\n")
            source_head = self.commit(source, "tracked")
            (source / "user-secret.txt").write_text("untracked\n")
            files = IMPORT_CHECK.source_tree_from_repository(source, source_head)
            self.assertEqual(set(files), {b"tracked.txt"})


class ImportBoundaryTests(unittest.TestCase):
    def test_module_is_not_a_git_submodule_and_parity_has_no_sibling_default(self) -> None:
        module = ROOT / "modules/rust-platform-libs"
        self.assertTrue((module / "Cargo.toml").is_file())
        self.assertFalse((ROOT / ".gitmodules").exists())
        for name in ("fixture_parity.rs", "typed_payload_parity.rs"):
            source = (module / "crates/platform-contracts/tests" / name).read_text()
            self.assertIn('join("contracts")', source)
            self.assertNotIn('join("platform-contracts")', source)

    def test_quality_workflow_never_publishes_and_checks_the_module_paths(self) -> None:
        workflow = (ROOT / ".github/workflows/platform-rust-quality.yml").read_text()
        self.assertIn("modules/rust-platform-libs/**", workflow)
        self.assertIn("modules/rust-platform-libs", workflow)
        self.assertIn("1.95.0", workflow)
        self.assertIn("./scripts/conformance.sh", workflow)
        self.assertNotIn("cargo publish", workflow)
        self.assertIn("fetch-depth: 0", workflow)

    def test_publish_workflow_uses_namespaced_tag_and_existing_approval_gates(self) -> None:
        workflow = (ROOT / ".github/workflows/platform-rust-publish.yml").read_text()
        for required in (
            "rust-platform-libs-v", "PUBLICATION-APPROVED", "CARGO_REGISTRY_TOKEN",
            "confirm_owner_approval", "cargo publish", "modules/rust-platform-libs",
        ):
            self.assertIn(required, workflow)
        self.assertLess(workflow.index("Require dispatch confirmation"), workflow.index("Publish crates in topological order"))
        self.assertLess(workflow.index("Refuse without PUBLICATION-APPROVED marker"), workflow.index("Verify crates.io names are available"))
        self.assertLess(workflow.index("Resolve and validate release version"), workflow.index("Publish crates in topological order"))
        self.assertNotIn("refs/tags/v}", workflow)


if __name__ == "__main__":
    unittest.main()
