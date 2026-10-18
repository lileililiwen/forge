import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))
from workspace_check import discover_projects, run, validate_openspec_seriality

from _decisions_env import restore_decisions_register, use_empty_decisions_register


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


BASE_DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "product",
    "profile": "inventory",
    "lifecycle": "inventory",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


class WorkspaceCheckTests(unittest.TestCase):
    def make_workspace(self):
        temp = tempfile.TemporaryDirectory()
        root = Path(temp.name)
        project = root / "demo-project"
        project.mkdir()
        write_json(project / ".project.json", BASE_DECLARATION)
        registry = {"schema_version": 1, "workspace_root": None, "discovery": {"mode": "explicit", "unregistered_policy": "fail"}, "projects": [{"id": "demo-project", "path": "demo-project", "profile": "inventory", "lifecycle": "inventory", "adoption": "adopted"}]}
        registry_path = root / "projects.json"
        write_json(registry_path, registry)
        return temp, root, registry_path

    def make_seriality_workspace(
        self, project_id, changes=(), handoff=None, archived=(), hidden=()
    ):
        temp, root, registry_path = self.make_workspace()
        project = root / "demo-project"
        declaration = json.loads((project / ".project.json").read_text())
        declaration["id"] = project_id
        write_json(project / ".project.json", declaration)
        registry = json.loads(registry_path.read_text())
        registry["projects"][0]["id"] = project_id
        write_json(registry_path, registry)
        changes_dir = project / "openspec" / "changes"
        for name in changes:
            (changes_dir / name).mkdir(parents=True, exist_ok=True)
        for name in archived:
            (changes_dir / "archive" / name).mkdir(parents=True, exist_ok=True)
        for name in hidden:
            (changes_dir / name).mkdir(parents=True, exist_ok=True)
        if handoff is not None:
            (project / "HANDOFF.md").write_text(handoff, encoding="utf-8")
        return temp, root, registry_path, project

    def run_seriality_workspace(
        self, project_id, changes=(), handoff=None, archived=(), hidden=()
    ):
        temp, root, registry_path, project = self.make_seriality_workspace(
            project_id, changes=changes, handoff=handoff,
            archived=archived, hidden=hidden,
        )
        self.addCleanup(temp.cleanup)
        before = {
            path.relative_to(project): path.read_bytes()
            for path in project.rglob("*")
            if path.is_file()
        }
        findings, summary = run(SimpleNamespace(
            registry=str(registry_path), root=str(root), project=None,
            audit=False, self_check=False, as_json=False,
        ))
        after = {
            path.relative_to(project): path.read_bytes()
            for path in project.rglob("*")
            if path.is_file()
        }
        self.assertEqual(after, before, "the audit must be read-only")
        return findings, summary

    def test_governance_seriality_accepts_no_active_change_with_no_pointer(self):
        findings, _ = self.run_seriality_workspace("workspace-governance")
        self.assertNotIn("OPENSPEC_CHANGE_SERIALITY", {item.code for item in findings})

    def test_governance_seriality_accepts_no_active_change_with_none_pointer(self):
        findings, _ = self.run_seriality_workspace(
            "workspace-governance", handoff="current_spec: none\n"
        )
        self.assertNotIn("OPENSPEC_CHANGE_SERIALITY", {item.code for item in findings})

    def test_governance_seriality_accepts_one_matching_pointer(self):
        findings, _ = self.run_seriality_workspace(
            "workspace-governance", changes=("only-change",),
            handoff="current_spec: only-change\n",
        )
        self.assertNotIn("OPENSPEC_CHANGE_SERIALITY", {item.code for item in findings})

    def test_governance_seriality_ignores_archives_and_hidden_state(self):
        findings, _ = self.run_seriality_workspace(
            "workspace-governance", changes=("only-change",),
            handoff="current_spec: only-change\n", archived=("finished",),
            hidden=(".cache",),
        )
        self.assertNotIn("OPENSPEC_CHANGE_SERIALITY", {item.code for item in findings})

    def test_governance_seriality_rejects_multiple_active_changes(self):
        findings, _ = self.run_seriality_workspace(
            "workspace-governance", changes=("zeta-change", "alpha-change"),
            handoff="current_spec: alpha-change\n",
        )
        matched = [item for item in findings if item.code == "OPENSPEC_CHANGE_SERIALITY"]
        self.assertEqual(len(matched), 1)
        self.assertEqual(matched[0].level, "ERROR")
        self.assertLess(matched[0].message.index("alpha-change"), matched[0].message.index("zeta-change"))

    def test_governance_seriality_rejects_missing_duplicate_and_mismatched_pointer(self):
        cases = (
            (None, "missing"),
            ("current_spec: other-change\n", "other-change"),
            ("current_spec: only-change\ncurrent_spec: only-change\n", "exactly once"),
        )
        for handoff, expected in cases:
            with self.subTest(expected=expected):
                findings, _ = self.run_seriality_workspace(
                    "workspace-governance", changes=("only-change",), handoff=handoff
                )
                matched = [item for item in findings if item.code == "OPENSPEC_CHANGE_SERIALITY"]
                self.assertEqual(len(matched), 1)
                self.assertEqual(matched[0].level, "ERROR")
                self.assertIn(expected, matched[0].message)

    def test_governance_seriality_rejects_stale_pointer_when_no_change_is_active(self):
        findings, _ = self.run_seriality_workspace(
            "workspace-governance", handoff="current_spec: archived-change\n"
        )
        self.assertTrue(any(
            item.code == "OPENSPEC_CHANGE_SERIALITY" and item.level == "ERROR"
            for item in findings
        ))

    def test_governance_seriality_fails_closed_on_unreadable_handoff(self):
        temp, root, registry_path, project = self.make_seriality_workspace(
            "workspace-governance", changes=("only-change",)
        )
        self.addCleanup(temp.cleanup)
        (project / "HANDOFF.md").write_bytes(b"\xff")
        findings, _ = run(SimpleNamespace(
            registry=str(registry_path), root=str(root), project=None,
            audit=False, self_check=False, as_json=False,
        ))
        self.assertTrue(any(
            item.code == "OPENSPEC_CHANGE_SERIALITY" and item.level == "ERROR"
            for item in findings
        ))

    def test_seriality_rule_does_not_constrain_other_projects(self):
        findings, _ = self.run_seriality_workspace(
            "demo-project", changes=("one-change", "another-change"),
            handoff="current_spec: one-change\n",
        )
        self.assertNotIn("OPENSPEC_CHANGE_SERIALITY", {item.code for item in findings})

    def test_real_governance_repository_has_one_active_change_and_matching_pointer(self):
        findings = []
        validate_openspec_seriality(
            Path(__file__).parents[1], findings, "workspace-governance"
        )
        self.assertFalse(
            findings,
            "the required tests Gate blocks multiple active changes or an invalid current_spec pointer",
        )

    def test_valid_inventory_project_passes(self):
        temp, root, registry = self.make_workspace()
        self.addCleanup(temp.cleanup)
        findings, summary = run(SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=False, self_check=False, as_json=False))
        self.assertEqual(summary["errors"], 0, findings)

    def test_missing_declaration_fails_audit(self):
        temp, root, registry = self.make_workspace()
        self.addCleanup(temp.cleanup)
        (root / "demo-project" / ".project.json").unlink()
        findings, summary = run(SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=True, self_check=False, as_json=False))
        self.assertGreater(summary["errors"], 0)
        self.assertTrue(any(item.code == "DECLARATION_MISSING" for item in findings))

    def test_secret_value_is_rejected(self):
        temp, root, registry = self.make_workspace()
        self.addCleanup(temp.cleanup)
        data = json.loads(registry.read_text())
        data["api_token"] = "do-not-store"
        write_json(registry, data)
        findings, summary = run(SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=False, self_check=False, as_json=False))
        self.assertGreater(summary["errors"], 0)
        self.assertTrue(any(item.code == "SECRET_VALUE" for item in findings))

    def test_invalid_active_openspec_name_is_rejected(self):
        temp, root, registry = self.make_workspace()
        self.addCleanup(temp.cleanup)
        project = root / "demo-project"
        declaration = json.loads((project / ".project.json").read_text())
        declaration["profile"] = "product"
        write_json(project / ".project.json", declaration)
        registry_data = json.loads(registry.read_text())
        registry_data["projects"][0]["profile"] = "product"
        write_json(registry, registry_data)
        (project / "README.md").write_text("# Demo\n")
        (project / "ROADMAP.md").write_text("# Roadmap\n")
        (project / "HANDOFF.md").write_text("current_spec: valid-change\n")
        (project / "AGENTS.md").write_text("# Agents\n")
        (project / ".ai-rules").mkdir()
        (project / ".ai-rules/workflow.md").write_text("BFS DFS BFS\n")
        (project / ".ai-rules/completion.md").write_text("completion\n")
        (project / "openspec/changes/2026-bad").mkdir(parents=True)
        (project / ".github/workflows").mkdir(parents=True)
        findings, summary = run(SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=False, self_check=False, as_json=False))
        self.assertGreater(summary["errors"], 0)
        self.assertTrue(any(item.code == "OPENSPEC_NAME" for item in findings))

    def test_new_project_folder_is_discovered_without_registry_edit(self):
        temp, root, registry = self.make_workspace()
        self.addCleanup(temp.cleanup)
        candidate = root / "new-project"
        candidate.mkdir()
        (candidate / "README.md").write_text("# New project\n", encoding="utf-8")
        write_json(candidate / ".project.json", {**BASE_DECLARATION, "id": "new-project"})
        data = json.loads(registry.read_text())
        data["discovery"] = {"mode": "immediate-directories", "unregistered_policy": "report"}
        write_json(registry, data)

        findings, summary = run(SimpleNamespace(registry=str(registry), root=str(root), project=None, audit=True, self_check=False, as_json=False))

        self.assertEqual(summary["errors"], 0, findings)
        self.assertTrue(any(item.code == "DISCOVERED_UNREGISTERED" and item.project == "new-project" for item in findings))

    def test_discovery_uses_relative_folder_names(self):
        temp, root, registry = self.make_workspace()
        self.addCleanup(temp.cleanup)
        (root / "new-project").mkdir()
        (root / "new-project" / "README.md").write_text("# New project\n", encoding="utf-8")
        discovered = discover_projects(root, {"discovery": {"mode": "immediate-directories"}})
        self.assertEqual(discovered[0]["path"], "demo-project")
        self.assertEqual(discovered[1]["path"], "new-project")


if __name__ == "__main__":
    unittest.main()
