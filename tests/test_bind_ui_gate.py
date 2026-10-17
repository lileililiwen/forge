"""Tests for `scripts/bind_ui_gate.py`.

The properties that matter, and the ones this file pins:

* the two entries are written into ONE repository's manifest, and the command
  is the expected repository-relative one;
* a second run is a byte-for-byte no-op that reports `no change`;
* `--check` exits non-zero on an unbound repository and writes nothing;
* `--check` exits zero once bound;
* a manifest that is not the governance `rule_pack` shape is refused by name;
* a differing existing entry is reported, not clobbered;
* nothing is written without `--apply`;
* no repository other than the one passed is read for writing or modified.

No test uses `rm`: this host's `rm` is a recoverable-delete shim that prints to
stdout and can fail. Cleanup uses `shutil.rmtree(..., ignore_errors=True)`.
"""

import hashlib
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from bind_ui_gate import (  # noqa: E402
    ALREADY_BOUND,
    BOUND,
    CHECK_ID,
    CONFLICT,
    DEFAULT_COMMAND,
    NO_MANIFEST,
    REFUSED,
    bind_repo,
    command_problem,
    is_rule_pack_shape,
    main,
    render_manifest,
    yaml_scalar,
)

RULE_PACK_MANIFEST = """version: 1
runtime: driftwatchdog
profile: product
rule_pack: workspace-governance@1
checks:
  build: true
  tests: true
blocking:
  - FAIL
  - REVIEW_REQUIRED
"""

# A manifest that already has a `commands:` block, so the insertion path runs
# against a document that is not gaining its first one.
MANIFEST_WITH_COMMANDS = """version: 1
runtime: driftwatchdog
profile: product
rule_pack: workspace-governance@1
checks:
  tests: true
commands:
  tests: 'npm test'
blocking:
  - FAIL
"""

# A document that is called `gate.yaml` but is not a governance rule_pack
# manifest. Writing a governance key into it is how a manifest stops parsing.
FOREIGN_MANIFEST = """name: some-other-tool
inputs:
  - path: src
"""


class RepoCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.repo = self.tmp / "demo"
        (self.repo / ".ai-gate").mkdir(parents=True)
        # The bound command reaches a sibling checkout, and that edge has to
        # be declared in the repository's own `.project.json`. The fixture
        # reproduces the real layout — every repository in this fleet carries
        # a declaration — so a repository that has nowhere to declare the
        # edge is a case a test states for itself rather than the default.
        self.write_declaration()

    def write_declaration(self, body: dict | None = None) -> Path:
        target = self.repo / ".project.json"
        target.write_text(
            json.dumps(body or {"schema_version": 1, "id": "demo"}, indent=2) + "\n",
            encoding="utf-8",
        )
        return target

    def declaration(self) -> dict:
        return json.loads((self.repo / ".project.json").read_text(encoding="utf-8"))

    def write_manifest(self, text: str = RULE_PACK_MANIFEST) -> Path:
        target = self.repo / ".ai-gate" / "gate.yaml"
        target.write_text(text, encoding="utf-8")
        return target


class ShapeTests(unittest.TestCase):
    def test_a_rule_pack_manifest_is_recognised(self):
        lines = RULE_PACK_MANIFEST.splitlines(keepends=True)
        ok, reason = is_rule_pack_shape(lines)
        self.assertTrue(ok, reason)

    def test_a_foreign_manifest_is_refused_by_name(self):
        lines = FOREIGN_MANIFEST.splitlines(keepends=True)
        ok, reason = is_rule_pack_shape(lines)
        self.assertFalse(ok)
        self.assertIn("version:", reason)

    def test_a_foreign_rule_pack_is_refused_by_name(self):
        text = RULE_PACK_MANIFEST.replace(
            "rule_pack: workspace-governance@1", "rule_pack: some-other-pack@3"
        )
        ok, reason = is_rule_pack_shape(text.splitlines(keepends=True))
        self.assertFalse(ok)
        self.assertIn("some-other-pack@3", reason)


class CommandTests(unittest.TestCase):
    def test_the_default_command_is_accepted(self):
        self.assertEqual(command_problem(DEFAULT_COMMAND), "")

    def test_the_default_command_names_no_environment_variable(self):
        self.assertNotIn("$", DEFAULT_COMMAND)

    def test_the_default_command_is_not_absolute(self):
        self.assertFalse(DEFAULT_COMMAND.startswith("/"))

    def test_an_absolute_command_is_refused(self):
        self.assertIn("absolute", command_problem("/opt/check.py ."))

    def test_an_environment_dependent_command_is_refused(self):
        self.assertIn(
            "GOVERNANCE_ROOT",
            command_problem("python3 $GOVERNANCE_ROOT/scripts/ui_conformance.py ."),
        )

    def test_a_blank_command_is_refused(self):
        self.assertIn("blank", command_problem("   "))


class RenderTests(unittest.TestCase):
    def test_binds_both_entries(self):
        after, status, added = render_manifest(RULE_PACK_MANIFEST, DEFAULT_COMMAND)
        self.assertEqual(status, BOUND)
        self.assertIn(f"  {CHECK_ID}: true\n", after)
        self.assertIn(f"  {CHECK_ID}: {yaml_scalar(DEFAULT_COMMAND)}\n", after)
        self.assertEqual(set(added), {f"checks.{CHECK_ID}", f"commands.{CHECK_ID}"})

    def test_binds_into_an_existing_commands_block(self):
        after, status, _ = render_manifest(MANIFEST_WITH_COMMANDS, DEFAULT_COMMAND)
        self.assertEqual(status, BOUND)
        self.assertIn("  tests: 'npm test'\n", after)
        self.assertIn(f"  {CHECK_ID}: {yaml_scalar(DEFAULT_COMMAND)}\n", after)

    def test_the_second_render_is_a_byte_for_byte_no_op(self):
        first, status, _ = render_manifest(RULE_PACK_MANIFEST, DEFAULT_COMMAND)
        self.assertEqual(status, BOUND)
        second, status, added = render_manifest(first, DEFAULT_COMMAND)
        self.assertEqual(status, ALREADY_BOUND)
        self.assertEqual(first, second, "the second render must not change a byte")
        self.assertEqual(added, ())

    def test_a_foreign_manifest_is_refused(self):
        after, status, _ = render_manifest(FOREIGN_MANIFEST, DEFAULT_COMMAND)
        self.assertEqual(status, REFUSED)
        self.assertEqual(after, FOREIGN_MANIFEST)

    def test_a_selection_of_false_is_a_conflict_and_is_not_flipped(self):
        text = RULE_PACK_MANIFEST.replace("  tests: true", "  tests: true\n  ui-conformance: false")
        after, status, _ = render_manifest(text, DEFAULT_COMMAND)
        self.assertEqual(status, CONFLICT)
        self.assertEqual(after, text)

    def test_a_differing_existing_command_is_a_conflict_and_is_not_clobbered(self):
        text = MANIFEST_WITH_COMMANDS.replace(
            "  tests: 'npm test'", "  tests: 'npm test'\n  ui-conformance: 'npm run ui'"
        )
        after, status, _ = render_manifest(text, DEFAULT_COMMAND)
        self.assertEqual(status, CONFLICT)
        self.assertIn("ui-conformance: 'npm run ui'", after)
        self.assertNotIn(DEFAULT_COMMAND, after)

    def test_comments_and_ordering_elsewhere_are_preserved(self):
        text = RULE_PACK_MANIFEST.replace(
            "blocking:", "# a hand-written comment\nblocking:"
        )
        after, _status, _ = render_manifest(text, DEFAULT_COMMAND)
        self.assertIn("# a hand-written comment", after)
        self.assertLess(after.index("blocking:"), after.index("commands:"))


class BindRepoTests(RepoCase):
    def test_reports_no_manifest_rather_than_creating_one(self):
        outcome = bind_repo(self.repo, apply=True)
        self.assertEqual(outcome.status, NO_MANIFEST)
        self.assertFalse((self.repo / ".ai-gate" / "gate.yaml").exists())

    def test_a_dry_run_writes_nothing(self):
        manifest = self.write_manifest()
        before = manifest.read_text(encoding="utf-8")
        outcome = bind_repo(self.repo, apply=False)
        self.assertEqual(outcome.status, BOUND)
        self.assertFalse(outcome.wrote)
        self.assertEqual(manifest.read_text(encoding="utf-8"), before)

    def test_apply_writes_and_the_second_run_reports_already_bound(self):
        manifest = self.write_manifest()
        first = bind_repo(self.repo, apply=True)
        self.assertEqual(first.status, BOUND)
        self.assertTrue(first.wrote)
        after_first = manifest.read_bytes()

        second = bind_repo(self.repo, apply=True)
        self.assertEqual(second.status, ALREADY_BOUND)
        self.assertIn("no change", second.detail)
        self.assertFalse(second.wrote)
        self.assertEqual(manifest.read_bytes(), after_first)

    def test_a_foreign_manifest_is_refused_and_untouched(self):
        manifest = self.write_manifest(FOREIGN_MANIFEST)
        before = manifest.read_text(encoding="utf-8")
        outcome = bind_repo(self.repo, apply=True)
        self.assertEqual(outcome.status, REFUSED)
        self.assertFalse(outcome.wrote)
        self.assertEqual(manifest.read_text(encoding="utf-8"), before)

    def test_a_manifest_with_no_checks_block_is_refused(self):
        text = "version: 1\nruntime: driftwatchdog\nrule_pack: workspace-governance@1\n"
        manifest = self.write_manifest(text)
        outcome = bind_repo(self.repo, apply=True)
        self.assertEqual(outcome.status, REFUSED)
        self.assertIn("checks:", outcome.detail)
        self.assertEqual(manifest.read_text(encoding="utf-8"), text)

    def test_only_the_named_repository_is_touched(self):
        self.write_manifest()
        other = self.tmp / "other"
        (other / ".ai-gate").mkdir(parents=True)
        other_manifest = other / ".ai-gate" / "gate.yaml"
        other_manifest.write_text(RULE_PACK_MANIFEST, encoding="utf-8")
        before = other_manifest.read_bytes()

        bind_repo(self.repo, apply=True)

        self.assertEqual(other_manifest.read_bytes(), before)
        self.assertNotIn(CHECK_ID, other_manifest.read_text(encoding="utf-8"))


class SiblingDeclarationTests(RepoCase):
    """The command this tool binds reaches another checkout, so it declares the edge.

    `workspace_check.py` recomputes the digest from the `entry` it reads, so
    the digest asserted here is the audit's own recomputation, and the round
    trip end to end is asserted next to the fleet writer in
    `tests/test_bind_gate_commands.py`.
    """

    def interfaces(self) -> list:
        return self.declaration().get("interfaces", [])

    def test_binding_declares_the_sibling_edge(self):
        self.write_manifest()
        outcome = bind_repo(self.repo, apply=True)
        self.assertEqual(outcome.status, BOUND)
        self.assertIn(f"interfaces.{CHECK_ID}", outcome.added)
        self.assertEqual(
            self.interfaces(),
            [
                {
                    "id": CHECK_ID,
                    "consumes": True,
                    "provider": "workspace-governance",
                    "mechanism": "sibling",
                    "entry": DEFAULT_COMMAND,
                    "digest": "sha256:"
                    + hashlib.sha256(DEFAULT_COMMAND.encode("utf-8")).hexdigest(),
                    "required": True,
                    "evidence_ref": ".ai-gate/gate.yaml",
                }
            ],
        )
        self.assertIn("declared interfaces.ui-conformance", outcome.detail)

    def test_a_second_run_declares_nothing_new(self):
        self.write_manifest()
        bind_repo(self.repo, apply=True)
        before = (self.repo / ".project.json").read_bytes()
        outcome = bind_repo(self.repo, apply=True)
        self.assertEqual(outcome.status, ALREADY_BOUND)
        self.assertEqual(outcome.added, ())
        self.assertFalse(outcome.wrote)
        self.assertEqual((self.repo / ".project.json").read_bytes(), before)

    def test_a_repository_with_no_declaration_is_refused(self):
        manifest = self.write_manifest()
        before = manifest.read_bytes()
        (self.repo / ".project.json").unlink()
        outcome = bind_repo(self.repo, apply=True)
        self.assertEqual(outcome.status, REFUSED)
        self.assertIn(".project.json", outcome.detail)
        self.assertEqual(manifest.read_bytes(), before)
        self.assertFalse((self.repo / ".project.json").exists())

    def test_an_undescribable_sibling_command_is_refused(self):
        manifest = self.write_manifest()
        before = manifest.read_bytes()
        outcome = bind_repo(self.repo, apply=True, command="python3 ../Bad_Provider/x.py")
        self.assertEqual(outcome.status, REFUSED)
        self.assertIn("cannot describe", outcome.detail)
        self.assertEqual(manifest.read_bytes(), before)

    def test_a_local_command_declares_no_edge(self):
        self.write_manifest()
        outcome = bind_repo(self.repo, apply=True, command="python3 scripts/check-ui.py .")
        self.assertEqual(outcome.status, BOUND)
        self.assertNotIn("interfaces", self.declaration())
        self.assertNotIn("interfaces", outcome.added)

    def test_a_hand_written_entry_is_never_overwritten(self):
        self.write_manifest()
        self.write_declaration(
            {
                "interfaces": [
                    {
                        "id": CHECK_ID,
                        "consumes": True,
                        "provider": "workspace-governance",
                        "mechanism": "sibling",
                        "entry": DEFAULT_COMMAND,
                        "digest": "sha256:" + "a" * 64,
                        "required": True,
                        "evidence_ref": ".ai-gate/gate.yaml",
                    }
                ]
            }
        )
        bind_repo(self.repo, apply=True)
        self.assertEqual(self.interfaces()[0]["digest"], "sha256:" + "a" * 64)


class CliTests(RepoCase):
    def test_check_exits_non_zero_and_writes_nothing_when_unbound(self):
        manifest = self.write_manifest()
        before = manifest.read_bytes()
        code = main([str(self.repo), "--check"])
        self.assertEqual(code, 1, "a dry run that would change something must fail")
        self.assertEqual(manifest.read_bytes(), before)

    def test_check_exits_zero_once_bound(self):
        self.write_manifest()
        self.assertEqual(main([str(self.repo), "--apply"]), 0)
        self.assertEqual(main([str(self.repo), "--check"]), 0)

    def test_check_after_apply_leaves_the_file_unchanged(self):
        manifest = self.write_manifest()
        main([str(self.repo), "--apply"])
        after_apply = manifest.read_bytes()
        self.assertEqual(main([str(self.repo), "--check"]), 0)
        self.assertEqual(manifest.read_bytes(), after_apply)

    def test_default_run_without_apply_does_not_write(self):
        manifest = self.write_manifest()
        before = manifest.read_bytes()
        self.assertEqual(main([str(self.repo)]), 0)
        self.assertEqual(manifest.read_bytes(), before)

    def test_check_surfaces_a_missing_declaration(self):
        # The command half is bound and the edge is not declared: the check
        # mode exists to surface that rather than pass on the strength of a
        # bound command that no declaration accounts for.
        self.write_manifest()
        main([str(self.repo), "--apply"])
        declaration = self.declaration()
        declaration.pop("interfaces")
        (self.repo / ".project.json").write_text(json.dumps(declaration, indent=2) + "\n")
        self.assertEqual(main([str(self.repo), "--check"]), 1)

    def test_check_cannot_be_combined_with_apply(self):
        self.write_manifest()
        self.assertEqual(main([str(self.repo), "--check", "--apply"]), 2)

    def test_json_output_carries_the_outcome(self):
        self.write_manifest()
        import io
        from contextlib import redirect_stdout

        buffer = io.StringIO()
        with redirect_stdout(buffer):
            code = main([str(self.repo), "--check", "--json"])
        self.assertEqual(code, 1)
        payload = json.loads(buffer.getvalue())
        self.assertEqual(payload["status"], BOUND)
        self.assertEqual(payload["repo"], self.repo.name)
        self.assertFalse(payload["wrote"])
        self.assertEqual(
            payload["added"],
            [
                f"checks.{CHECK_ID}",
                f"commands.{CHECK_ID}",
                f"interfaces.{CHECK_ID}",
            ],
        )

    def test_an_absolute_command_is_refused_before_any_read(self):
        self.assertEqual(
            main([str(self.repo), "--apply", "--command", "/opt/check.py ."]), 2
        )
        self.assertFalse((self.repo / ".ai-gate" / "gate.yaml").exists())

    def test_a_missing_repository_exits_two(self):
        self.assertEqual(main([str(self.tmp / "nope"), "--check"]), 2)


if __name__ == "__main__":
    unittest.main()
