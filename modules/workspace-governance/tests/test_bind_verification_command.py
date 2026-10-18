"""Tests for `scripts/bind_verification_command.py`.

The properties that matter, and the ones this file pins:

* the bound command is the repository's own declared string, verbatim;
* a missing or unparseable declaration is **refused**, never invented;
* a second run is a byte-for-byte no-op;
* a differing existing binding is reported, not clobbered;
* a dirty manifest is skipped rather than written into.

No test uses `rm`: this host's `rm` is a recoverable-delete shim that prints to
stdout and can fail. Cleanup uses `shutil.rmtree(..., ignore_errors=True)` and
`Path.unlink(missing_ok=True)`.
"""

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from bind_verification_command import (  # noqa: E402
    ALREADY_BOUND,
    BOUND,
    CONCERN_ID,
    CONFLICT,
    DIRTY,
    NO_MANIFEST,
    REFUSED,
    bind_repo,
    discover_repos,
    read_declared_command,
    render_manifest,
    run,
    yaml_scalar,
)

MINIMAL_MANIFEST = """version: 1
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

# A manifest with an existing, unrelated `commands:` block, so the insertion
# path is exercised against a document that already has one.
MANIFEST_WITH_COMMANDS = """version: 1
runtime: driftwatchdog
profile: product
checks:
  build: true
  tests: true
  governance-quality: true
commands:
  build: cargo build --all-targets
  tests: cargo test --all
blocking:
  - FAIL
"""


class RepoCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.repo = self.tmp / "demo"
        (self.repo / ".ai-gate").mkdir(parents=True)

    def write_project(self, value, raw: str | None = None):
        target = self.repo / ".project.json"
        if raw is not None:
            target.write_text(raw, encoding="utf-8")
        else:
            target.write_text(json.dumps(value), encoding="utf-8")
        return target

    def write_manifest(self, text: str = MINIMAL_MANIFEST):
        target = self.repo / ".ai-gate" / "gate.yaml"
        target.write_text(text, encoding="utf-8")
        return target

    def declaring(self, command: str):
        return {
            "schema_version": 1,
            "id": "demo",
            "verification": {"command": command, "gate_runtime": "driftwatchdog"},
        }


class DeclarationTests(RepoCase):
    def test_reads_the_declared_command_verbatim(self):
        command = "cargo test --workspace && cargo clippy --all-targets -- -D warnings"
        self.write_project(self.declaring(command))
        got, refusal = read_declared_command(self.repo)
        self.assertEqual(refusal, "", "a readable declaration must not be refused")
        self.assertEqual(got, command)

    def test_strips_surrounding_whitespace_but_keeps_the_command(self):
        self.write_project(self.declaring("  npm test  "))
        got, refusal = read_declared_command(self.repo)
        self.assertEqual(refusal, "")
        self.assertEqual(got, "npm test")

    def test_refuses_when_the_project_file_is_absent(self):
        got, refusal = read_declared_command(self.repo)
        self.assertIsNone(got)
        self.assertIn("no .project.json", refusal)

    def test_refuses_when_the_project_file_is_unparseable(self):
        self.write_project(None, raw="{not json at all")
        got, refusal = read_declared_command(self.repo)
        self.assertIsNone(got)
        self.assertIn("not parseable", refusal)

    def test_refuses_when_verification_is_absent(self):
        self.write_project({"schema_version": 1, "id": "demo"})
        got, refusal = read_declared_command(self.repo)
        self.assertIsNone(got)
        self.assertIn("no `verification`", refusal)

    def test_refuses_when_the_command_is_absent(self):
        self.write_project({"verification": {"gate_runtime": "driftwatchdog"}})
        got, refusal = read_declared_command(self.repo)
        self.assertIsNone(got)
        self.assertIn("no `command`", refusal)

    def test_refuses_when_the_command_is_not_a_string(self):
        self.write_project({"verification": {"command": ["npm", "test"]}})
        got, refusal = read_declared_command(self.repo)
        self.assertIsNone(got)
        self.assertIn("is not a string", refusal)

    def test_refuses_when_the_command_is_blank(self):
        self.write_project(self.declaring("   \n  "))
        got, refusal = read_declared_command(self.repo)
        self.assertIsNone(got)
        self.assertIn("is blank", refusal)

    def test_never_invents_a_command_for_anything_it_refuses(self):
        for raw, payload in [
            (None, {"id": "demo"}),
            ("{oops", None),
            (None, {"verification": {"command": 7}}),
            (None, {"verification": {"command": ""}}),
        ]:
            self.write_project(payload, raw=raw)
            got, _ = read_declared_command(self.repo)
            self.assertIsNone(got, f"invented a command for {raw or payload!r}")


class ScalarTests(unittest.TestCase):
    def test_quotes_a_plain_command(self):
        self.assertEqual(yaml_scalar("npm test"), "'npm test'")

    def test_doubles_an_embedded_single_quote(self):
        self.assertEqual(yaml_scalar("rg 'password'"), "'rg ''password'''")

    def test_leaves_backslashes_literal(self):
        # A double-quoted scalar would turn `\s` into an escape; a declared
        # command must be written back in meaning.
        self.assertEqual(yaml_scalar(r"rg -e 'a\\s*b'"), r"'rg -e ''a\\s*b'''")

    def test_survives_a_real_yaml_round_trip(self):
        # The scalar must be readable by the parser that actually reads these
        # manifests. `yaml` may not be installed, so skip rather than fail.
        try:
            import yaml
        except ImportError:  # pragma: no cover - environment dependent
            self.skipTest("PyYAML not installed")
        for command in [
            "npm test",
            "npm run typecheck && npm test",
            r"rg -e 'password\\s*' --glob '!target' .",
            "if a; then echo x; else echo y; fi",
            "python3 -c 'print(1)'",
            "echo \"quoted $HOME\"",
        ]:
            parsed = yaml.safe_load(f"commands:\n  c: {yaml_scalar(command)}\n")
            self.assertEqual(parsed["commands"]["c"], command, command)


class RenderTests(RepoCase):
    def test_creates_the_commands_block_when_absent(self):
        out, status = render_manifest(MINIMAL_MANIFEST, "cargo test --all")
        self.assertEqual(status, BOUND)
        self.assertIn(f"commands:\n", out)
        self.assertIn(f"  {CONCERN_ID}: 'cargo test --all'\n", out)
        self.assertIn(f"  {CONCERN_ID}: true\n", out)

    def test_inserts_into_an_existing_commands_block(self):
        out, status = render_manifest(MANIFEST_WITH_COMMANDS, "npm test")
        self.assertEqual(status, BOUND)
        self.assertIn("  build: cargo build --all-targets\n", out)
        self.assertIn(f"  {CONCERN_ID}: 'npm test'\n", out)
        # The command entry must land inside the commands block, not after it.
        commands_at = out.index("commands:\n")
        entry_at = out.index(f"  {CONCERN_ID}: 'npm test'\n")
        blocking_at = out.index("blocking:\n")
        self.assertLess(commands_at, entry_at)
        self.assertLess(entry_at, blocking_at)

    def test_is_idempotent_byte_for_byte(self):
        first, status = render_manifest(MINIMAL_MANIFEST, "cargo test --all")
        self.assertEqual(status, BOUND)
        second, status2 = render_manifest(first, "cargo test --all")
        self.assertEqual(status2, ALREADY_BOUND)
        self.assertEqual(first, second, "the second render must change nothing")

    def test_third_render_is_also_stable(self):
        once, _ = render_manifest(MINIMAL_MANIFEST, "npm test")
        twice, _ = render_manifest(once, "npm test")
        thrice, _ = render_manifest(twice, "npm test")
        self.assertEqual(once, twice)
        self.assertEqual(twice, thrice)

    def test_idempotent_when_a_commands_block_already_existed(self):
        once, _ = render_manifest(MANIFEST_WITH_COMMANDS, "npm test")
        twice, status = render_manifest(once, "npm test")
        self.assertEqual(status, ALREADY_BOUND)
        self.assertEqual(once, twice)

    def test_does_not_duplicate_the_generated_comment(self):
        once, _ = render_manifest(MINIMAL_MANIFEST, "npm test")
        twice, _ = render_manifest(once, "npm test")
        self.assertEqual(once.count("bind_verification_command.py"), 1)
        self.assertEqual(twice.count("bind_verification_command.py"), 1)

    def test_writes_the_command_verbatim_including_operators(self):
        command = "npm run typecheck && npm test || echo 'fallback'"
        out, _ = render_manifest(MINIMAL_MANIFEST, command)
        self.assertIn(f"  {CONCERN_ID}: {yaml_scalar(command)}\n", out)

    def test_refuses_to_clobber_a_differing_existing_binding(self):
        existing = MINIMAL_MANIFEST.replace(
            "blocking:", f"commands:\n  {CONCERN_ID}: 'npm run check'\nblocking:"
        )
        out, status = render_manifest(existing, "npm test")
        self.assertEqual(status, CONFLICT)
        self.assertEqual(out, existing, "a conflict must leave the file untouched")

    def test_refuses_to_override_a_disabled_selection(self):
        existing = MINIMAL_MANIFEST.replace(
            "  tests: true\n", f"  tests: true\n  {CONCERN_ID}: false\n"
        )
        out, status = render_manifest(existing, "npm test")
        self.assertEqual(status, CONFLICT)
        self.assertIn(f"  {CONCERN_ID}: false\n", out)
        self.assertNotIn("'npm test'", out)

    def test_repairs_a_bound_command_whose_selection_is_missing(self):
        existing = MINIMAL_MANIFEST.replace(
            "blocking:", f"commands:\n  {CONCERN_ID}: 'npm test'\nblocking:"
        )
        out, status = render_manifest(existing, "npm test")
        self.assertEqual(status, BOUND)
        self.assertIn(f"  {CONCERN_ID}: true\n", out)
        self.assertIn(f"  {CONCERN_ID}: 'npm test'\n", out)
        # And that repaired form is then stable.
        again, status2 = render_manifest(out, "npm test")
        self.assertEqual(status2, ALREADY_BOUND)
        self.assertEqual(out, again)

    def test_preserves_comments_and_ordering(self):
        source = MINIMAL_MANIFEST.replace(
            "checks:", "# a comment the owner wrote\nchecks:"
        )
        out, _ = render_manifest(source, "npm test")
        self.assertIn("# a comment the owner wrote\n", out)
        self.assertLess(out.index("# a comment the owner wrote"), out.index("checks:"))

    def test_a_manifest_with_no_checks_block_is_refused_not_synthesised(self):
        # There is no selection to attach the command to, and inventing a whole
        # `checks` section would be synthesising the repository's policy. A
        # `commands.<id>` with no selection is refused by the manifest parser,
        # so writing one would be worse than writing nothing.
        source = "version: 1\nruntime: driftwatchdog\nprofile: product\nblocking:\n  - FAIL\n"
        out, status = render_manifest(source, "npm test")
        self.assertEqual(status, REFUSED)
        self.assertEqual(out, source)
        self.assertNotIn("commands:", out)


class BindRepoTests(RepoCase):
    def test_dry_run_reports_but_writes_nothing(self):
        self.write_project(self.declaring("npm test"))
        manifest = self.write_manifest()
        outcome = bind_repo(self.repo, apply=False, check_clean=False)
        self.assertEqual(outcome.status, BOUND)
        self.assertFalse(outcome.wrote)
        self.assertEqual(manifest.read_text(encoding="utf-8"), MINIMAL_MANIFEST)

    def test_apply_writes_both_halves(self):
        self.write_project(self.declaring("npm test"))
        manifest = self.write_manifest()
        outcome = bind_repo(self.repo, apply=True, check_clean=False)
        self.assertTrue(outcome.wrote)
        text = manifest.read_text(encoding="utf-8")
        self.assertIn(f"  {CONCERN_ID}: true\n", text)
        self.assertIn(f"  {CONCERN_ID}: 'npm test'\n", text)

    def test_second_apply_is_a_no_op_on_disk(self):
        self.write_project(self.declaring("npm test"))
        manifest = self.write_manifest()
        bind_repo(self.repo, apply=True, check_clean=False)
        first = manifest.read_bytes()
        second_outcome = bind_repo(self.repo, apply=True, check_clean=False)
        second = manifest.read_bytes()
        self.assertEqual(second_outcome.status, ALREADY_BOUND)
        self.assertFalse(second_outcome.wrote)
        self.assertEqual(first, second, "the second run must not change a byte")

    def test_refuses_without_writing_when_the_declaration_is_unparseable(self):
        self.write_project(None, raw="{oops")
        manifest = self.write_manifest()
        outcome = bind_repo(self.repo, apply=True, check_clean=False)
        self.assertEqual(outcome.status, REFUSED)
        self.assertEqual(manifest.read_text(encoding="utf-8"), MINIMAL_MANIFEST)

    def test_refuses_without_writing_when_the_command_is_absent(self):
        self.write_project({"verification": {"gate_runtime": "driftwatchdog"}})
        manifest = self.write_manifest()
        outcome = bind_repo(self.repo, apply=True, check_clean=False)
        self.assertEqual(outcome.status, REFUSED)
        self.assertEqual(manifest.read_text(encoding="utf-8"), MINIMAL_MANIFEST)

    def test_reports_no_manifest_rather_than_inventing_one(self):
        self.write_project(self.declaring("npm test"))
        outcome = bind_repo(self.repo, apply=True, check_clean=False)
        self.assertEqual(outcome.status, NO_MANIFEST)
        self.assertFalse((self.repo / ".ai-gate" / "gate.yaml").exists())

    def test_a_dirty_manifest_is_skipped_not_written(self):
        self.write_project(self.declaring("npm test"))
        manifest = self.write_manifest()
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)
        subprocess.run(["git", "add", "-A"], cwd=self.repo, check=True)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base"],
            cwd=self.repo,
            check=True,
        )
        # The owner's uncommitted edit: modify the manifest without staging it.
        manifest.write_text(MINIMAL_MANIFEST + "# owner WIP\n", encoding="utf-8")
        before = manifest.read_text(encoding="utf-8")
        outcome = bind_repo(self.repo, apply=True, check_clean=True)
        self.assertEqual(outcome.status, DIRTY)
        self.assertEqual(manifest.read_text(encoding="utf-8"), before)

    def test_an_untracked_manifest_is_treated_as_dirty(self):
        self.write_project(self.declaring("npm test"))
        self.write_manifest()
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)
        outcome = bind_repo(self.repo, apply=True, check_clean=True)
        self.assertEqual(outcome.status, DIRTY)

    def test_a_clean_repository_is_written(self):
        self.write_project(self.declaring("npm test"))
        self.write_manifest()
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)
        subprocess.run(["git", "add", "-A"], cwd=self.repo, check=True)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base"],
            cwd=self.repo,
            check=True,
        )
        outcome = bind_repo(self.repo, apply=True, check_clean=True)
        self.assertEqual(outcome.status, BOUND)
        self.assertTrue(outcome.wrote)


class RunTests(RepoCase):
    def _workspace(self) -> Path:
        root = self.tmp / "ws"
        for name, command in [("alpha", "npm test"), ("beta", "cargo test --all")]:
            repo = root / name
            (repo / ".ai-gate").mkdir(parents=True)
            (repo / ".project.json").write_text(
                json.dumps({"verification": {"command": command}}), encoding="utf-8"
            )
            (repo / ".ai-gate" / "gate.yaml").write_text(MINIMAL_MANIFEST, encoding="utf-8")
        return root

    def test_discovers_sibling_git_repositories_in_order(self):
        root = self._workspace()
        for name in ("alpha", "beta"):
            subprocess.run(["git", "init", "-q"], cwd=root / name, check=True)
        found = [p.name for p in discover_repos(root)]
        self.assertEqual(found, ["alpha", "beta"])

    def test_run_counts_outcomes_per_repository(self):
        root = self._workspace()
        for name in ("alpha", "beta"):
            subprocess.run(["git", "init", "-q"], cwd=root / name, check=True)
        report = run(root, apply=False, check_clean=False)
        self.assertEqual(len(report.outcomes), 2)
        self.assertEqual(report.count(BOUND), 2)
        self.assertEqual(sorted(report.names(BOUND)), ["alpha", "beta"])

    def test_run_can_be_restricted_to_named_repositories(self):
        root = self._workspace()
        for name in ("alpha", "beta"):
            subprocess.run(["git", "init", "-q"], cwd=root / name, check=True)
        report = run(root, apply=False, only=["alpha"], check_clean=False)
        self.assertEqual([o.repo for o in report.outcomes], ["alpha"])

    def test_a_fleet_run_defaults_to_checking_cleanliness(self):
        # The fixture repositories are `git init`ed but never committed, so
        # their manifests are untracked. A default fleet run must therefore
        # report DIRTY, not BOUND: the safety default is the whole point of
        # walking 81 sibling repositories.
        root = self._workspace()
        for name in ("alpha", "beta"):
            subprocess.run(["git", "init", "-q"], cwd=root / name, check=True)
        report = run(root, apply=False)
        self.assertEqual(report.count(DIRTY), 2)
        self.assertEqual(report.count(BOUND), 0)


if __name__ == "__main__":
    unittest.main()
