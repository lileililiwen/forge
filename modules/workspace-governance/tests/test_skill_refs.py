"""Tests for the governance front-door skill reference integrity."""

import re
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))

import check_skill_refs
from check_skill_refs import check, cited_paths


REPO_ROOT = Path(__file__).parents[1]
SKILL = REPO_ROOT / "skills" / "workspace-governance" / "SKILL.md"
REFERENCES = sorted((REPO_ROOT / "skills" / "workspace-governance" / "references").glob("*.md"))


def write_skill_tree(root: Path, skill_text: str, references: dict[str, str]) -> None:
    skill_dir = root / "skills" / "workspace-governance" / "references"
    skill_dir.mkdir(parents=True, exist_ok=True)
    (skill_dir.parent / "SKILL.md").write_text(skill_text, encoding="utf-8")
    for name, text in references.items():
        (skill_dir / name).write_text(text, encoding="utf-8")


class SkillRefsPositiveTests(unittest.TestCase):
    def test_repository_references_are_valid(self):
        self.assertEqual(check(REPO_ROOT), [])

    def test_routing_rows_resolve_to_references(self):
        text = SKILL.read_text(encoding="utf-8")
        rows = [line for line in text.splitlines() if line.startswith("|") and "references/" in line]
        self.assertGreater(len(rows), 0)
        for row in rows:
            for mention in re.findall(r"references/[A-Za-z0-9_.-]+\.md", row):
                self.assertTrue((SKILL.parent / mention).is_file(), f"route names a missing reference: {row}")

    def test_safe_default_is_stated(self):
        text = SKILL.read_text(encoding="utf-8")
        self.assertIn("workspace_check.py --root .. --audit", text)
        self.assertIn("openspec list", text)

    def test_every_reference_links_an_owner(self):
        self.assertGreater(len(REFERENCES), 0)
        for reference in REFERENCES:
            paths = cited_paths(reference.read_text(encoding="utf-8"))
            owned = [item for item in paths
                     if item.startswith(("scripts/", "openspec/specs/", "schemas/", "docs/"))
                     or item in ("vocabulary.json", "projects.json")]
            self.assertGreater(len(owned), 0, f"{reference.name} links no owning script or spec")

    def test_no_destructive_instructions(self):
        corpus = SKILL.read_text(encoding="utf-8") + "\n".join(path.read_text(encoding="utf-8") for path in REFERENCES)
        self.assertNotIn("git push", corpus)
        self.assertIsNone(re.search(r"promote_project\.py\s+\S+\s+--(evidence-ref|force|demote)", corpus))
        self.assertIn("explicit-approval boundary", corpus)


class SkillRefsNegativeTests(unittest.TestCase):
    def test_missing_script_fails_naming_reference_and_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_skill_tree(root, "# Skill\n", {"broken.md": "Run `python3 scripts/does-not-exist.py --audit`.\n"})
            errors = check(root)
            self.assertEqual(len(errors), 1)
            self.assertIn("broken.md", errors[0])
            self.assertIn("scripts/does-not-exist.py", errors[0])

    def test_missing_skill_entry_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertTrue(check(Path(tmp)))

    def test_prose_names_and_install_paths_are_not_checked(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_skill_tree(root, "# Skill\n", {"ok.md": (
                "See driftwatchdog and `openspec list`.\n"
                "Install with `~/.agents/skills/workspace-governance`.\n"
            )})
            self.assertEqual(check(root), [])


if __name__ == "__main__":
    unittest.main()
