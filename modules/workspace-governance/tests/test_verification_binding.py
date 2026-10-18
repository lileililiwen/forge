"""Tests for the `QUALITY_VERIFICATION_COMMAND_UNBOUND` drift rule and its fence.

The rule asks the question the existing `VERIFICATION_COMMAND` rule cannot: not
*is a verification command declared* — all 81 sibling repositories satisfy that —
but *does a blocking gate actually run it*.

The fence test is the load-bearing one. A committed constant that nothing
demonstrably controls is decoration, so `flipping_the_fence_changes_the_result`
asserts both directions against the real rule, at the level the rule runs.
"""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
import workspace_check  # noqa: E402
from bind_verification_command import (  # noqa: E402
    CONCERN_ID,
    bind_repo,
    render_manifest,
)
from workspace_check import (  # noqa: E402
    DECLARED_VERIFICATION_CONCERN,
    GATE_MANIFEST_RELATIVE,
    QUALITY_VERIFICATION_COMMAND_UNBOUND,
    QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL,
    VERIFICATION_BINDING_AUDIT_ENABLED,
    validate_declared_verification_is_bound,
    verification_binding_audit_enabled,
)

DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "product",
    "profile": "inventory",
    "lifecycle": "inventory",
    "verification": {
        "command": "cargo test --workspace",
        "gate_runtime": "driftwatchdog",
        "evidence_status": "planned",
    },
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}

UNBOUND_MANIFEST = """version: 1
runtime: driftwatchdog
profile: product
checks:
  build: true
  tests: true
blocking:
  - FAIL
"""


def bound_manifest(command: str) -> str:
    """The manifest the generator would produce, built by the generator itself."""
    out, status = render_manifest(UNBOUND_MANIFEST, command)
    assert status == "BOUND", status
    return out


class RuleCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.project = self.tmp / "demo-project"
        self.project.mkdir()
        self.write_declaration(DECLARATION)

    def write_declaration(self, value):
        (self.project / ".project.json").write_text(json.dumps(value), encoding="utf-8")

    def write_manifest(self, text: str):
        target = self.project / GATE_MANIFEST_RELATIVE
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
        return target

    def findings_for(self):
        findings = []
        validate_declared_verification_is_bound(self.project, findings, "demo-project")
        return findings


class RuleTests(RuleCase):
    def test_an_unbound_manifest_is_an_error_naming_the_command(self):
        self.write_manifest(UNBOUND_MANIFEST)
        findings = self.findings_for()
        self.assertEqual(len(findings), 1)
        item = findings[0]
        self.assertEqual(item.code, QUALITY_VERIFICATION_COMMAND_UNBOUND)
        self.assertEqual(item.level, "ERROR")
        self.assertEqual(item.level, QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL)
        self.assertIn("cargo test --workspace", item.message)
        self.assertEqual(item.project, "demo-project")

    def test_a_manifest_with_no_manifest_at_all_is_reported_distinctly(self):
        findings = self.findings_for()
        self.assertEqual(len(findings), 1)
        self.assertIn("no .ai-gate/gate.yaml to bind it into", findings[0].message)

    def test_a_native_gate_toml_is_named_rather_than_a_missing_manifest(self):
        # `devloom` binds its commands in the native `gate.toml` form, which
        # this rule does not read. Saying "has no manifest" there would be
        # wrong in a way that sends the owner looking for a file that exists.
        (self.project / "gate.toml").write_text('version = 1\nprofile = "product"\n', encoding="utf-8")
        findings = self.findings_for()
        self.assertEqual(len(findings), 1)
        self.assertIn("uses a native gate.toml", findings[0].message)
        self.assertIn("does not read", findings[0].message)

    def test_a_fully_bound_manifest_is_clean(self):
        self.write_manifest(bound_manifest("cargo test --workspace"))
        self.assertEqual(self.findings_for(), [])

    def test_a_command_bound_without_a_selection_is_reported(self):
        # The manifest parser refuses `commands.<id>` with no selection, so
        # this state is a binding that could never run. The rule must still see
        # it rather than reading the `commands` block and stopping there.
        text = UNBOUND_MANIFEST.replace(
            "blocking:", f"commands:\n  {CONCERN_ID}: 'cargo test --workspace'\nblocking:"
        )
        self.write_manifest(text)
        findings = self.findings_for()
        self.assertEqual(len(findings), 1)
        self.assertIn("does not select it", findings[0].message)

    def test_a_disabled_selection_is_reported_and_not_treated_as_bound(self):
        text = UNBOUND_MANIFEST.replace(
            "  tests: true\n", f"  tests: true\n  {CONCERN_ID}: false\n"
        ).replace(
            "blocking:", f"commands:\n  {CONCERN_ID}: 'cargo test --workspace'\nblocking:"
        )
        self.write_manifest(text)
        findings = self.findings_for()
        self.assertEqual(len(findings), 1)
        self.assertIn("not `true`", findings[0].message)

    def test_a_manifest_with_no_checks_block_is_reported(self):
        text = "version: 1\nruntime: driftwatchdog\nprofile: product\nblocking:\n  - FAIL\n"
        self.write_manifest(text)
        findings = self.findings_for()
        self.assertEqual(len(findings), 1)
        self.assertIn("selects no `checks:` block", findings[0].message)

    def test_an_unparseable_manifest_does_not_crash_the_rule(self):
        # 11 of the 76 fleet manifests carry an unknown top-level field and are
        # refused by the manifest parser. A rule that raised on them would be
        # blind exactly where the state is worst, so it reads line-wise and
        # reports what it can still see.
        text = UNBOUND_MANIFEST + "notes:\n  - a comment the owner wrote\n"
        self.write_manifest(text)
        findings = self.findings_for()
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0].code, QUALITY_VERIFICATION_COMMAND_UNBOUND)

    def test_a_project_declaring_no_command_is_not_reported(self):
        self.write_declaration({**DECLARATION, "verification": {"gate_runtime": "driftwatchdog"}})
        self.assertEqual(self.findings_for(), [])

    def test_a_project_with_no_declaration_is_not_reported(self):
        (self.project / ".project.json").unlink(missing_ok=True)
        self.assertEqual(self.findings_for(), [])

    def test_an_unparseable_declaration_is_not_reported(self):
        (self.project / ".project.json").write_text("{oops", encoding="utf-8")
        self.assertEqual(self.findings_for(), [])

    def test_the_rule_and_the_generator_agree_on_the_concern_id(self):
        # If these two constants drift, the generator binds something the rule
        # calls unbound, and the rule reports a binding that never runs.
        self.assertEqual(DECLARED_VERIFICATION_CONCERN, CONCERN_ID)

    def test_the_generator_output_satisfies_the_rule(self):
        # The end-to-end contract: what the generator writes is exactly what
        # the rule accepts. Without this, the two could each be correct alone
        # and the fleet permanently red.
        manifest = self.write_manifest(UNBOUND_MANIFEST)
        outcome = bind_repo(self.project, apply=True, check_clean=False)
        self.assertTrue(outcome.wrote, outcome.detail)
        self.assertEqual(self.findings_for(), [])
        self.assertIn("declared-verification", manifest.read_text(encoding="utf-8"))


class FenceTests(RuleCase):
    """The fence must be a real control, not a comment.

    These go through the production `run()` path rather than calling the rule
    directly, because the fence guards the *call site* in `validate_project`,
    not the rule function. A test that called the rule directly would pass no
    matter what the constant said, which is exactly the kind of green that
    makes a fence decorative.
    """

    def setUp(self):
        super().setUp()
        original = workspace_check.VERIFICATION_BINDING_AUDIT_ENABLED
        self.addCleanup(
            setattr, workspace_check, "VERIFICATION_BINDING_AUDIT_ENABLED", original
        )
        self.registry = self.tmp / "projects.json"
        self.registry.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "workspace_root": None,
                    "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
                    "projects": [
                        {
                            "id": "demo-project",
                            "path": "demo-project",
                            "profile": "inventory",
                            "lifecycle": "inventory",
                            "adoption": "adopted",
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )

    def set_fence(self, value: bool):
        workspace_check.VERIFICATION_BINDING_AUDIT_ENABLED = value

    def audit(self):
        from types import SimpleNamespace

        return workspace_check.run(
            SimpleNamespace(
                registry=str(self.registry),
                root=str(self.tmp),
                project=None,
                audit=False,
                self_check=False,
                as_json=False,
            )
        )

    def codes(self):
        findings, _summary = self.audit()
        return [item.code for item in findings]

    def test_the_fence_ships_closed(self):
        # Recorded so that flipping the shipped state is a visible edit rather
        # than a silent change of behaviour.
        self.assertIs(VERIFICATION_BINDING_AUDIT_ENABLED, False)

    def test_the_resolver_reads_the_committed_constant(self):
        self.set_fence(False)
        self.assertFalse(verification_binding_audit_enabled())
        self.set_fence(True)
        self.assertTrue(verification_binding_audit_enabled())

    def test_an_explicit_argument_overrides_the_constant(self):
        self.set_fence(False)
        self.assertTrue(verification_binding_audit_enabled(True))
        self.set_fence(True)
        self.assertFalse(verification_binding_audit_enabled(False))

    def test_flipping_the_fence_changes_the_result(self):
        self.write_manifest(UNBOUND_MANIFEST)
        self.set_fence(False)
        self.assertNotIn(
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            self.codes(),
            "fence closed must report nothing",
        )
        self.set_fence(True)
        self.assertIn(
            QUALITY_VERIFICATION_COMMAND_UNBOUND,
            self.codes(),
            "fence open must report the finding",
        )

    def test_the_finding_is_an_error_that_raises_the_summary_when_open(self):
        self.write_manifest(UNBOUND_MANIFEST)
        self.set_fence(False)
        _findings, closed = self.audit()
        self.set_fence(True)
        findings, opened = self.audit()
        self.assertEqual(
            opened["errors"], closed["errors"] + 1, "the rule must be a real error"
        )
        item = next(
            i for i in findings if i.code == QUALITY_VERIFICATION_COMMAND_UNBOUND
        )
        self.assertEqual(item.level, "ERROR")
        self.assertEqual(item.project, "demo-project")

    def test_a_bound_manifest_is_clean_with_the_fence_open(self):
        # The fence must not be what makes the fleet green: with the rule on,
        # a correctly bound repository reports nothing.
        self.write_manifest(bound_manifest("cargo test --workspace"))
        self.set_fence(True)
        self.assertNotIn(QUALITY_VERIFICATION_COMMAND_UNBOUND, self.codes())

    def test_flipping_the_fence_never_softens_the_level(self):
        # The hold must come from the fence alone. If opening the fence also
        # dropped the level to WARNING, the rule would be hidden in two ways at
        # once and the constant would stop being the whole story.
        self.write_manifest(UNBOUND_MANIFEST)
        for value in (True, False):
            self.set_fence(value)
            findings, _summary = self.audit()
            for item in findings:
                if item.code == QUALITY_VERIFICATION_COMMAND_UNBOUND:
                    self.assertEqual(
                        item.level, "ERROR", f"level changed with fence={value}"
                    )
        self.assertEqual(QUALITY_VERIFICATION_COMMAND_UNBOUND_LEVEL, "ERROR")

    def test_the_fence_is_not_an_environment_variable(self):
        # A committed repo-relative constant, never an environment variable:
        # an env var would let a run opt itself out silently.
        import os

        os.environ["VERIFICATION_BINDING_AUDIT_ENABLED"] = "1"
        self.addCleanup(os.environ.pop, "VERIFICATION_BINDING_AUDIT_ENABLED", None)
        self.assertFalse(verification_binding_audit_enabled())
        self.write_manifest(UNBOUND_MANIFEST)
        self.set_fence(False)
        self.assertNotIn(QUALITY_VERIFICATION_COMMAND_UNBOUND, self.codes())


if __name__ == "__main__":
    unittest.main()
