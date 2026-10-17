"""Tests for `scripts/bind_gate_commands.py`.

The properties that matter, and the ones this file pins:

* the shape comes from the files on disk; `.project.json` `stack` is never an
  input, because it disagrees with the disk for a large minority of the fleet;
* a `*.slnx` is a solution on the same footing as a `*.sln`, which a
  `*.sln`-only search cannot see at all;
* **a command is never invented.** An unknown shape, a `package.json` without
  the script the concern needs, a pure-Python `build`, and two equally
  plausible entry points are all *unresolved*, with the reason. A wrong
  command is worse than an honest gap, so the refusals are pinned harder than
  the successes;
* a concern the manifest did not select is never bound;
* an operator's own command is reported as a conflict and never overwritten;
* a dry run writes nothing, and `--apply` is the only thing that writes;
* the debt record names every unresolved concern, and carries no timestamp.

No test uses `rm`: this host's `rm` is a recoverable-delete shim that prints to
stdout and can fail. Cleanup uses `shutil.rmtree(..., ignore_errors=True)`.
"""

import io
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "bind_gate_commands.py"
import workspace_check  # noqa: E402
from bind_gate_commands import (  # noqa: E402
    ALREADY_BOUND,
    BOUND,
    CONFLICT,
    DEBT_FILE,
    DIRTY,
    GENERATED_MARKER,
    NO_MANIFEST,
    PARTIAL,
    PROFILE_DEFAULTS,
    REFUSED,
    RESOLVED,
    SOLUTION_SUFFIXES,
    UNRESOLVED,
    Report,
    bind_repo,
    build_debt,
    derive,
    derive_quality_block,
    debt_concerns,
    detect_shape,
    format_detail,
    gate_required,
    main,
    read_gate_policy,
    read_manifest,
    render_manifest,
    roll_up,
    run,
    unquote_scalar,
    write_debt_record,
)

# The manifest a fixture starts from. `checks:` is the only thing a test
# varies about policy; everything else is fixed so a binding diff is readable.
#
# `profile: minimal` is deliberate. A built-in profile schedules checks the
# manifest never lists, and `product` — 41 of this fleet's 76 manifests —
# schedules three that bind no command, so every fixture using it would
# carry three unresolvable concerns that have nothing to do with what each
# test is about. `minimal` is the built-in profile with an empty default
# set, which leaves `checks:` as the only selection in play here.
# `ProfileSelectionTests` covers the profile path against `product` itself.
MANIFEST = """version: 1
runtime: driftwatchdog
profile: minimal
checks:
{checks}blocking:
  - FAIL
"""

BUILD_AND_TESTS = "  build: true\n  tests: true\n"
TESTS_ONLY = "  tests: true\n"

# A manifest with an existing `commands:` block, so the insertion path is
# exercised against a document that already has one.
WITH_COMMANDS = """version: 1
runtime: driftwatchdog
profile: minimal
checks:
  build: true
  tests: true
commands:
  build: cargo build --all-targets
blocking:
  - FAIL
"""

# The exact reason the tool gives for a pure-Python `build`. Quoted rather
# than re-derived, so a change in the wording is a visible test diff.
PURE_PYTHON_BUILD = (
    "pyproject.toml does not declare a [build-system] table"
)

UNITTEST_RUNNER = "python3 -m unittest discover -s tests -v"

A_SOLUTION = "Microsoft Visual Studio Solution File, Format Version 12.00\n"
A_SLNX = '<Solution>\n</Solution>\n'
A_CSPROJ = '<Project Sdk="Microsoft.NET.Sdk">\n</Project>\n'


class RepoCase(unittest.TestCase):
    """A throwaway repository with a manifest and whatever files a test needs."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.repo = self.tmp / "demo"
        (self.repo / ".ai-gate").mkdir(parents=True)
        # A repository starts with a manifest declaring `build` and `tests`. A
        # test about a different selection calls `write_manifest` over the top.
        self.write_manifest()

    # --- fixture construction ------------------------------------

    def write_manifest(self, checks: str = BUILD_AND_TESTS, text: str | None = None):
        target = self.repo / ".ai-gate" / "gate.yaml"
        target.write_text(
            text if text is not None else MANIFEST.format(checks=checks),
            encoding="utf-8",
        )
        return target

    def write(self, relative: str, body: str = ""):
        target = self.repo / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(body, encoding="utf-8")
        return target

    def unlink_manifest(self):
        (self.repo / ".ai-gate" / "gate.yaml").unlink(missing_ok=True)

    def manifest_text(self) -> str:
        return (self.repo / ".ai-gate" / "gate.yaml").read_text(encoding="utf-8")

    def write_sibling(self) -> Path:
        """A `workspace-governance` checkout beside the fixture repository.

        Every repository in this fleet has the workspace checkers at
        `../workspace-governance/scripts/…`, and that reachability is what
        makes a concern bindable, so the fixture reproduces the real layout
        rather than assuming it. Empty files: the binder checks that the path
        exists, and no test here runs the checker.
        """
        sibling = self.repo.parent / "workspace-governance"
        for relative in ("workspace_check.py", "product_code_quality.py"):
            target = sibling / "scripts" / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("# fixture\n", encoding="utf-8")
        return sibling

    def write_declaration(self, extra: dict | None = None) -> Path:
        """A minimal `.project.json`, which all 81 real repositories have.

        The binder refuses to create one — the declaration is the registry's
        document — so a fixture that needs the `quality` write path has to
        supply it, as every repository the fleet write will touch does.
        """
        body = {"schema_version": 1, "id": "demo"}
        if extra:
            body.update(extra)
        return self.write(".project.json", json.dumps(body, indent=2) + "\n")

    def forget_sibling(self) -> None:
        """Remove the sibling checkout, so no workspace checker is reachable."""
        shutil.rmtree(self.repo.parent / "workspace-governance", ignore_errors=True)

    # --- running the tool -----------------------------------------

    def bind(self, apply: bool = False):
        return bind_repo(self.repo, apply=apply, check_clean=False)

    @staticmethod
    def concern(outcome, concern_id: str):
        """The one outcome for `concern_id`, or a hard failure if it is absent."""
        found = [c for c in outcome.concerns if c.concern == concern_id]
        if len(found) != 1:
            raise AssertionError(
                f"expected exactly one {concern_id!r} outcome, got {len(found)}: "
                f"{[c.concern for c in outcome.concerns]}"
            )
        return found[0]

    def bound_command(self, concern_id: str, apply: bool = False) -> str:
        got = self.concern(self.bind(apply=apply), concern_id)
        return got.command

    def derived(self, concern_id: str):
        return derive(concern_id, detect_shape(self.repo), self.repo)


class GitRepoCase(RepoCase):
    """A committed repository, for the paths that consult `git status`."""

    def commit(self):
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)
        subprocess.run(["git", "add", "-A"], cwd=self.repo, check=True)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base"],
            cwd=self.repo,
            check=True,
        )


# --- detection: the disk decides, the label does not ----------


class DetectionTests(RepoCase):
    def test_a_sln_is_a_solution(self):
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        shape = detect_shape(self.repo)
        self.assertEqual(shape.solutions, ["App.sln"])
        self.assertEqual(shape.label(), "dotnet")

    def test_a_slnx_is_a_solution(self):
        # The regression this exists for: 19 repositories in this fleet carry
        # no `*.sln` at all, and a `*.sln`-only search finds nothing in them.
        self.write("Foo.slnx", A_SLNX)
        shape = detect_shape(self.repo)
        self.assertEqual(shape.solutions, ["Foo.slnx"])
        self.assertEqual(shape.label(), "dotnet")

    def test_both_solution_suffixes_are_recognised(self):
        self.assertEqual(SOLUTION_SUFFIXES, (".sln", ".slnx"))

    def test_a_nested_slnx_is_found_at_depth(self):
        self.write("backend/Linglitch.slnx", A_SLNX)
        self.assertEqual(detect_shape(self.repo).solutions, ["backend/Linglitch.slnx"])

    def test_the_declared_stack_is_never_read(self):
        # `loreMix`: the label says typescript and the disk is a .NET solution.
        # A label-driven binder would emit `npm run build` for this repository.
        self.write(
            ".project.json", json.dumps({"schema_version": 1, "stack": "typescript"})
        )
        self.write("LoreMix.slnx", A_SLNX)
        self.write("web/package.json", json.dumps({"scripts": {"build": "vite"}}))
        shape = detect_shape(self.repo)
        self.assertEqual(shape.label(), "dotnet")
        self.assertEqual(
            derive("build", shape, self.repo)[0], "dotnet build LoreMix.slnx"
        )

    def test_a_wrong_label_does_not_produce_a_node_command(self):
        self.write(
            ".project.json", json.dumps({"schema_version": 1, "stack": "dotnet"})
        )
        self.write("package.json", json.dumps({"scripts": {"build": "tsc"}}))
        self.assertEqual(detect_shape(self.repo).label(), "node")

    def test_pyproject_alone_labels_the_repository_python(self):
        self.write("pyproject.toml", "[project]\nname = 'sample'\n")
        self.assertEqual(detect_shape(self.repo).label(), "python")

    def test_the_workspace_flag_follows_the_cargo_manifest(self):
        self.write("Cargo.toml", "[workspace]\nmembers = [\"a\"]\n")
        shape = detect_shape(self.repo)
        self.assertEqual(shape.crates, ["Cargo.toml"])
        self.assertEqual(shape.workspaces, ["Cargo.toml"])

    def test_a_cargo_manifest_without_workspace_is_not_a_workspace(self):
        self.write("Cargo.toml", '[package]\nname = "a"\n')
        shape = detect_shape(self.repo)
        self.assertEqual(shape.crates, ["Cargo.toml"])
        self.assertEqual(shape.workspaces, [])

    def test_a_vendored_conftest_is_not_evidence(self):
        # `alethefy`'s only `conftest.py` is inside a site-packages tree; reading
        # it would make a dependency's file look like this repository's pytest
        # configuration.
        self.write(".venv/lib/python3/site-packages/numpy/conftest.py", "def test_x():\n    pass\n")
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        self.assertEqual(detect_shape(self.repo).pytest_configs, [])

    def test_a_vendored_node_module_is_not_a_candidate(self):
        self.write("node_modules/dep/package.json", json.dumps({"scripts": {"test": "t"}}))
        self.assertEqual(detect_shape(self.repo).node_manifests, [])

    def test_an_indented_testcase_method_is_not_a_bare_test_function(self):
        # The false positive this file's sibling hit: an indented
        # `def test_x(self)` is a `unittest.TestCase` method, not a bare
        # function, so a repository written with unittest is not read as a
        # pytest repository.
        self.write(
            "tests/test_x.py",
            "import unittest\n\n\nclass T(unittest.TestCase):\n    def test_a(self):\n        pass\n",
        )
        shape = detect_shape(self.repo)
        self.assertEqual(shape.python_dirs, ["tests"])
        self.assertEqual(derive("tests", shape, self.repo)[0], UNITTEST_RUNNER)

    def test_an_empty_test_file_says_nothing_about_the_runner(self):
        self.write("tests/test_cli.py", "\n")
        shape = detect_shape(self.repo)
        command, _evidence, reason = derive("tests", shape, self.repo)
        self.assertEqual(command, "")
        self.assertIn("defines a test", reason)


# --- derivation: the exact command string ----------------------


class DerivationTests(RepoCase):
    def test_a_python_build_requires_backend_and_declared_build_frontend(self):
        self.write(
            "pyproject.toml",
            "[build-system]\nrequires = ['setuptools']\nbuild-backend = 'setuptools.build_meta'\n"
            "\n[project]\nname = 'p'\n",
        )
        self.write("requirements-dev.txt", "build>=1\n")
        self.assertEqual(self.bound_command("build"), "python -m build")

    def test_a_python_build_backend_without_frontend_stays_unresolved(self):
        self.write(
            "pyproject.toml",
            "[build-system]\nrequires = ['setuptools']\nbuild-backend = 'setuptools.build_meta'\n"
            "\n[project]\nname = 'p'\n",
        )
        got = self.concern(self.bind(), "build")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("frontend is not declared", got.reason)

    def test_python_build_requires_distribution_metadata(self):
        self.write(
            "pyproject.toml",
            "[build-system]\nrequires = ['setuptools']\nbuild-backend = 'setuptools.build_meta'\n"
            "\n[tool.ruff]\nexclude = ['build']\n",
        )
        self.write("requirements-dev.txt", "build>=1\n")
        got = self.concern(self.bind(), "build")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("distribution metadata", got.reason)

    def test_a_build_word_in_tool_configuration_is_not_a_frontend_dependency(self):
        self.write(
            "pyproject.toml",
            "[build-system]\nrequires = ['hatchling']\nbuild-backend = 'hatchling.build'\n"
            "\n[project]\nname = 'p'\n\n[tool.ruff]\nexclude = ['build']\n",
        )
        got = self.concern(self.bind(), "build")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("frontend is not declared", got.reason)

    def test_a_sln_yields_a_dotnet_build_naming_the_solution(self):
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        self.assertEqual(self.bound_command("build"), "dotnet build App.sln")

    def test_a_slnx_yields_a_dotnet_command(self):
        # The regression lock for the `.slnx` error: with no `*.sln` present,
        # the binder must still produce a .NET command rather than nothing.
        self.write("Foo.slnx", A_SLNX)
        self.assertEqual(self.bound_command("build"), "dotnet build Foo.slnx")
        self.assertEqual(self.bound_command("tests"), "dotnet test Foo.slnx")

    def test_a_workspace_cargo_manifest_yields_the_workspace_flag(self):
        self.write("Cargo.toml", "[workspace]\nmembers = [\"a\"]\n")
        self.assertEqual(self.bound_command("build"), "cargo build --workspace")
        self.assertEqual(self.bound_command("tests"), "cargo test --workspace")

    def test_a_bare_cargo_manifest_yields_a_plain_build(self):
        self.write("Cargo.toml", '[package]\nname = "a"\n')
        self.assertEqual(self.bound_command("build"), "cargo build")
        self.assertEqual(self.bound_command("tests"), "cargo test")

    def test_rust_security_is_the_dependency_audit(self):
        self.write_manifest("  security: true\n")
        self.write("Cargo.toml", '[package]\nname = "a"\n')
        got = self.concern(self.bind(), "security")
        self.assertEqual(got.command, "cargo audit")
        self.assertIn("dependency audit", got.evidence)

    def test_a_nested_node_manifest_carries_its_working_directory(self):
        # The Gate runs every command from the repository root, so a nested
        # manifest is unreachable without the `cd`.
        self.write("web/package.json", json.dumps({"scripts": {"test": "vitest"}}))
        self.assertEqual(self.bound_command("tests"), "cd web && npm test")

    def test_a_root_node_manifest_needs_no_working_directory(self):
        self.write("package.json", json.dumps({"scripts": {"test": "jest"}}))
        self.assertEqual(self.bound_command("tests"), "npm test")

    def test_a_package_json_without_the_script_yields_no_command(self):
        self.write("package.json", json.dumps({"scripts": {"build": "tsc"}}))
        got = self.concern(self.bind(), "tests")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertEqual(got.command, "")
        self.assertIn("no package.json in this repository declares a 'test' script", got.reason)

    def test_a_bare_test_function_repository_binds_pytest(self):
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        self.write("tests/test_x.py", "def test_one():\n    assert True\n")
        self.assertEqual(self.bound_command("tests"), "pytest")

    def test_a_mixed_runner_repository_binds_pytest(self):
        # `unittest discovery` finds no TestCase in a bare-function file and
        # reports success having run nothing, so the repository that needs
        # pytest anywhere gets pytest.
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        self.write(
            "tests/test_a.py",
            "import unittest\nclass T(unittest.TestCase):\n    def test_a(self):\n        pass\n",
        )
        self.write("tests/test_b.py", "def test_b():\n    assert True\n")
        self.assertEqual(self.bound_command("tests"), "pytest")

    def test_a_unittest_only_repository_binds_unittest_discovery(self):
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        self.write(
            "tests/test_a.py",
            "import unittest\nclass T(unittest.TestCase):\n    def test_a(self):\n        pass\n",
        )
        self.assertEqual(self.bound_command("tests"), UNITTEST_RUNNER)

    def test_python_security_needs_a_declared_tool(self):
        # There is no stdlib Python dependency audit, so a command is bound
        # only when the repository names the tool itself.
        self.write_manifest("  security: true\n")
        self.write("pyproject.toml", "[project]\ndependencies = ['pip-audit']\n")
        self.assertEqual(self.bound_command("security"), "pip-audit")

    def test_python_security_without_a_declared_tool_is_unresolved(self):
        self.write_manifest("  security: true\n")
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        got = self.concern(self.bind(), "security")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("not in the standard library", got.reason)

    def test_a_single_csproj_may_stand_in_for_a_solution(self):
        self.write("A.csproj", A_CSPROJ)
        self.assertEqual(self.bound_command("build"), "dotnet build A.csproj")

    def test_a_flutter_package_binds_its_test_runner(self):
        self.write("pubspec.yaml", "name: p\n")
        self.assertEqual(self.bound_command("tests"), "flutter test")

    def test_a_repository_concern_is_a_repository_wide_property(self):
        self.write_manifest("  repository: true\n")
        self.write("A.sln", A_SOLUTION)
        self.assertEqual(self.bound_command("repository"), "git diff --check")

    def test_a_named_script_is_run_when_the_repository_wrote_it(self):
        self.write("scripts/check-openspec-change-names.mjs", "console.log(1)\n")
        self.write_manifest("  openspec_names: true\n")
        self.assertEqual(
            self.bound_command("openspec_names"),
            "node scripts/check-openspec-change-names.mjs",
        )

    def test_a_named_script_is_not_invented_when_it_is_absent(self):
        self.write("A.sln", A_SOLUTION)
        self.write_manifest("  openspec_names: true\n")
        got = self.concern(self.bind(), "openspec_names")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("no scripts/check-openspec-change-names.mjs", got.reason)

    def test_an_unknown_concern_looks_for_its_own_check_script(self):
        self.write("scripts/check-foo.sh", "#!/bin/sh\n")
        self.write_manifest("  foo: true\n")
        self.assertEqual(self.bound_command("foo"), "scripts/check-foo.sh")


# --- refusal: the negative controls ---------------------------


class RefusalTests(RepoCase):
    """A binder that fabricates a command is worse than no binder.

    These are the most important tests in the file: each one must stay red if
    the tool ever starts guessing.
    """

    def pure_python_repo(self):
        self.write_manifest(BUILD_AND_TESTS)
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        self.write(
            "tests/test_x.py",
            "import unittest\nclass T(unittest.TestCase):\n    def test_a(self):\n        pass\n",
        )
        return self.bind()

    def test_a_pure_python_build_is_unresolved(self):
        got = self.concern(self.pure_python_repo(), "build")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertEqual(got.command, "")
        self.assertEqual(got.reason, PURE_PYTHON_BUILD)

    def test_a_pure_python_repository_never_acquires_a_build_command(self):
        # The negative control proper: whatever the tool does, no build command
        # may appear for a pure-Python package.
        outcome = self.pure_python_repo()
        for got in outcome.concerns:
            self.assertNotIn(" -m build", got.command, got.concern)
            self.assertNotIn("setup.py", got.command, got.concern)
            self.assertNotIn("pyproject", got.command, got.concern)
        build = self.concern(outcome, "build")
        self.assertEqual(build.command, "", "a build command was invented")
        # The only command in the whole repository is the one the files show.
        bound = [c.command for c in outcome.concerns if c.command]
        self.assertEqual(bound, [UNITTEST_RUNNER])

    def test_a_pure_python_dry_run_writes_nothing(self):
        self.pure_python_repo()
        self.assertNotIn("commands:", self.manifest_text())

    def test_a_flutter_build_is_unresolved_without_a_target(self):
        self.write("pubspec.yaml", "name: p\n")
        got = self.concern(self.bind(), "build")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("needs a target platform", got.reason)

    def test_an_unrecognised_shape_yields_no_command(self):
        self.write("README.md", "# hi\n")
        for concern_id in ("build", "tests"):
            got = self.concern(self.bind(), concern_id)
            self.assertEqual(got.status, UNRESOLVED, concern_id)
            self.assertEqual(got.command, "", concern_id)

    def test_several_csproj_and_no_solution_is_not_resolved_by_guessing(self):
        self.write("A/A.csproj", A_CSPROJ)
        self.write("B/B.csproj", A_CSPROJ)
        outcome = self.bind()
        for concern_id in ("build", "tests"):
            got = self.concern(outcome, concern_id)
            self.assertEqual(got.status, UNRESOLVED, concern_id)
            self.assertIn(".csproj files and no solution", got.reason)

    def test_two_root_solutions_are_not_resolved_by_picking_one(self):
        self.write("A.sln", A_SOLUTION)
        self.write("B.sln", A_SOLUTION)
        got = self.concern(self.bind(), "build")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("no single entry point", got.reason)

    def test_a_dotnet_repository_has_no_single_lint_command(self):
        self.write_manifest("  lint: true\n")
        self.write("A.sln", A_SOLUTION)
        got = self.concern(self.bind(), "lint")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertIn("no single .NET linter", got.reason)

    def test_a_manifest_with_no_checks_block_is_refused(self):
        self.write("A.sln", A_SOLUTION)
        self.write_manifest(
            text="version: 1\nruntime: driftwatchdog\nblocking:\n  - FAIL\n"
        )
        outcome = self.bind()
        self.assertEqual(outcome.status, REFUSED)
        self.assertIn("no `checks:` block", outcome.detail)
        self.assertEqual(outcome.concerns, [])

    def test_a_missing_manifest_is_reported_not_invented(self):
        self.unlink_manifest()
        outcome = self.bind()
        self.assertEqual(outcome.status, NO_MANIFEST)
        self.assertFalse((self.repo / ".ai-gate" / "gate.yaml").exists())

    def test_a_derived_command_is_never_derived_from_prose(self):
        # A command written in a manifest's `notes:` is not recovered from it.
        self.write_manifest(
            text=MANIFEST.format(checks="  build: true\n")
            + "notes: |\n  use `cargo build --release` for this one\n"
        )
        self.write("Cargo.toml", '[package]\nname = "a"\n')
        self.assertEqual(self.bound_command("build"), "cargo build")


# --- ambiguity: reported, not resolved ------------------------


class DebtRecordLocationTests(unittest.TestCase):
    """The record follows `--root`; only `--debt-out` moves it.

    An earlier attempt made the default the script's own repository, on the
    reasoning that the record belongs to the governance repository rather than
    beside the siblings it describes. That reasoning was true and the change
    was still wrong: a default that ignores the `--root` the caller passed is
    not a default, and the first test run overwrote this repository's real
    `gate-debt.json` with one test repository's debt. The record follows
    `--root`, and `--debt-out` is how a caller redirects it.
    """

    def test_the_record_follows_the_root_argument(self):
        workspace = Path(tempfile.mkdtemp())
        try:
            (workspace / "solo").mkdir()
            (workspace / "solo" / ".ai-gate").mkdir()
            (workspace / "solo" / ".ai-gate" / "gate.yaml").write_text(
                "version: 1\nprofile: inventory\nchecks:\n  tests: true\n", encoding="utf-8"
            )
            (workspace / "solo" / "test_x.py").write_text(
                "import unittest\n\n\nclass T(unittest.TestCase):\n"
                "    def test_it(self):\n        self.assertTrue(True)\n",
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--root", str(workspace), "--debt-only"],
                capture_output=True, text=True,
            )
            self.assertTrue(
                (workspace / DEBT_FILE).is_file(),
                "the record must be written under the --root the caller named, "
                f"not somewhere else: {result.stderr.strip()}",
            )
            self.assertFalse(
                (Path(__file__).resolve().parents[1] / DEBT_FILE).read_text(
                    encoding="utf-8"
                ).count('"solo"') > 0,
                "the test wrote this repository's real debt record; the "
                "generator's default path must follow --root",
            )
        finally:
            shutil.rmtree(workspace, ignore_errors=True)

    def test_debt_out_redirects_the_record(self):
        workspace = Path(tempfile.mkdtemp())
        try:
            (workspace / "solo").mkdir()
            (workspace / "solo" / ".ai-gate").mkdir()
            (workspace / "solo" / ".ai-gate" / "gate.yaml").write_text(
                "version: 1\nprofile: inventory\nchecks:\n  tests: true\n", encoding="utf-8"
            )
            (workspace / "solo" / "test_x.py").write_text(
                "import unittest\n\n\nclass T(unittest.TestCase):\n"
                "    def test_it(self):\n        self.assertTrue(True)\n",
                encoding="utf-8",
            )
            target = workspace / "elsewhere.json"
            subprocess.run(
                [sys.executable, str(SCRIPT), "--root", str(workspace),
                 "--debt-only", "--debt-out", str(target)],
                capture_output=True, text=True,
            )
            self.assertTrue(target.is_file())
            self.assertFalse((workspace / DEBT_FILE).exists())
        finally:
            shutil.rmtree(workspace, ignore_errors=True)


class AmbiguityTests(RepoCase):
    def test_two_disjoint_surfaces_compose_into_one_command(self):
        # `retakeKit`'s shape: a root solution and a root `package.json` with a
        # `test` script. Both must run for the repository to be sound, so the
        # command is both — joined, and neither half left out. Binding only the
        # .NET half would report a green `tests` on a repository whose Node
        # tests never ran, which is the false green this tool exists to avoid.
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        self.write("package.json", json.dumps({"scripts": {"test": "jest"}}))
        got = self.concern(self.bind(), "tests")
        self.assertEqual(got.status, BOUND)
        self.assertEqual(got.command, "(dotnet test App.sln) && (npm test)")
        self.assertIn("dotnet test App.sln", got.evidence)
        self.assertIn("npm test", got.evidence)

    def test_a_composite_names_every_part_it_joins(self):
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        self.write("package.json", json.dumps({"scripts": {"test": "jest"}}))
        got = self.concern(self.bind(), "tests")
        self.assertIn("root solution App.sln", got.evidence)
        self.assertIn("package.json declares scripts.test", got.evidence)

    def test_a_repository_spanning_two_stacks_writes_its_commands(self):
        self.write_manifest(TESTS_ONLY)
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        self.write("package.json", json.dumps({"scripts": {"test": "jest"}}))
        outcome = self.bind()
        self.assertEqual(
            [c.command for c in outcome.concerns if c.status == BOUND],
            ["(dotnet test App.sln) && (npm test)"],
        )
        self.assertIn("commands:", outcome.after)

    def test_two_candidates_of_one_stack_are_still_refused(self):
        # Two Flutter packages are one product described twice, not two halves
        # of a repository. Composing them would run the same suite twice, and
        # picking one would silently skip the other — so the refusal stands.
        self.write("app/pubspec.yaml", "name: app\n")
        self.write("lib/pubspec.yaml", "name: lib\n")
        got = self.concern(self.bind(), "tests")
        self.assertEqual(got.status, UNRESOLVED)
        self.assertEqual(got.command, "")
        self.assertIn("2 equally plausible entry points", got.reason)
        self.assertIn("flutter test", got.reason)

    def test_a_same_stack_refusal_names_both_candidates(self):
        self.write("app/pubspec.yaml", "name: app\n")
        self.write("lib/pubspec.yaml", "name: lib\n")
        got = self.concern(self.bind(), "tests")
        self.assertIn("app/pubspec.yaml", got.reason)
        self.assertIn("lib/pubspec.yaml", got.reason)

    def test_dominance_is_applied_before_composability(self):
        # `linglitch`'s shape: the solution is at the root and the Node manifest
        # is under `frontend/`. The root entry point wins on its own, so the two
        # stacks are NOT composed — dominance runs first, and a nested
        # `npm test` is a layout detail rather than a second product.
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        self.write("web/package.json", json.dumps({"scripts": {"test": "vitest"}}))
        self.assertEqual(self.bound_command("tests"), "dotnet test App.sln")

    def test_a_composite_is_byte_identical_across_runs(self):
        # Order comes from the generators, which iterate sorted manifests. If it
        # varied, every polyglot repository would read DIRTY on every re-run.
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        self.write("package.json", json.dumps({"scripts": {"test": "jest"}}))
        first = self.concern(self.bind(), "tests").command
        self.assertEqual(self.concern(self.bind(), "tests").command, first)
        self.assertEqual(first, "(dotnet test App.sln) && (npm test)")

    def test_each_part_of_a_composite_runs_in_its_own_directory(self):
        # `alltools-platform`'s shape, and the reason every part is wrapped in a
        # subshell. A candidate may `cd` into its own directory, and `cd` is
        # not scoped to one command in an `&&` chain: run flat, the second
        # `cd django_archived` executes from inside `rust/`, where that
        # directory does not exist, and the gate writes a command that can
        # only fail. This test runs the derived command in a real shell with
        # real directories, because the defect is in the shell's semantics and
        # no assertion on the string would see it.
        self.write("rust/Cargo.toml", "[workspace]\nmembers = []\n")
        self.write("frontend/package.json", json.dumps({"scripts": {"build": "tsc"}}))
        self.write_manifest("  build: true\n")
        command = self.concern(self.bind(), "build").command
        self.assertIn("(cd rust", command)
        self.assertIn("(cd frontend", command)
        # Prove the wrapping is what makes it work: the same chain without the
        # subshells must fail, which is the regression this guards.
        flat = command.replace("(", "").replace(")", "")
        self.assertNotEqual(flat, command)
        for marker, directory in (("rust", "rust"), ("frontend", "frontend")):
            (self.repo / directory).mkdir(parents=True, exist_ok=True)
        failed = subprocess.run(
            ["sh", "-c", flat.replace("cargo build --workspace", "true").replace("npm run build", "true")],
            cwd=self.repo, capture_output=True, text=True,
        )
        self.assertNotEqual(
            failed.returncode, 0,
            "the unwrapped chain was expected to fail its second `cd`; if it "
            "succeeded, the directory this test relies on is not being exercised",
        )
        wrapped = subprocess.run(
            ["sh", "-c", command.replace("cargo build --workspace", "true").replace("npm run build", "true")],
            cwd=self.repo, capture_output=True, text=True,
        )
        self.assertEqual(
            wrapped.returncode, 0,
            f"the wrapped composite failed: {wrapped.stderr.strip()}",
        )

    def test_a_root_candidate_beats_a_nested_one(self):
        # `linglitch`'s shape: the solution is at the root and the Node manifest
        # is under `frontend/`, so the root entry point wins on its own.
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        self.write("web/package.json", json.dumps({"scripts": {"test": "vitest"}}))
        self.assertEqual(self.bound_command("tests"), "dotnet test App.sln")

    def test_a_manifest_that_cannot_answer_is_dropped_before_ambiguity(self):
        # `alltools-platform`'s shape: the root `package.json` has no `test`
        # script, so it is not a candidate at all and the Rust workspace is not
        # reported as ambiguous.
        self.write("package.json", json.dumps({"scripts": {"build": "tsc"}}))
        self.write("Cargo.toml", "[workspace]\nmembers = [\"a\"]\n")
        self.assertEqual(self.bound_command("tests"), "cargo test --workspace")

    def test_a_nested_solution_still_names_its_own_path(self):
        self.write("backend/Linglitch.slnx", A_SLNX)
        got = self.concern(self.bind(), "build")
        self.assertEqual(got.command, "dotnet build backend/Linglitch.slnx")
        self.assertEqual(got.evidence, "the only solution, backend/Linglitch.slnx")


# --- the write contract ---------------------------------------


class WriteContractTests(RepoCase):
    def test_a_dry_run_writes_nothing(self):
        self.write_manifest()
        self.write("A.sln", A_SOLUTION)
        before = (self.repo / ".ai-gate" / "gate.yaml").read_bytes()
        outcome = self.bind(apply=False)
        self.assertEqual(self.concern(outcome, "build").status, BOUND)
        self.assertFalse(outcome.wrote)
        self.assertEqual((self.repo / ".ai-gate" / "gate.yaml").read_bytes(), before)

    def test_apply_writes_the_derived_command(self):
        self.write_manifest()
        self.write("A.sln", A_SOLUTION)
        outcome = self.bind(apply=True)
        self.assertTrue(outcome.wrote)
        self.assertIn("  build: 'dotnet build A.sln'\n", self.manifest_text())

    def test_apply_preserves_the_everything_else_in_the_manifest(self):
        self.write_manifest(text=WITH_COMMANDS)
        self.write("A.sln", A_SOLUTION)
        self.bind(apply=True)
        text = self.manifest_text()
        self.assertIn("  build: cargo build --all-targets\n", text)
        self.assertIn("  tests: 'dotnet test A.sln'\n", text)

    def test_a_second_run_derives_nothing_to_render(self):
        # Idempotence is a property of the run, not of `render_manifest`: the
        # second run classifies the already-bound concern as ALREADY_BOUND, so
        # there is no binding left to insert and the file is not rewritten.
        self.write_manifest()
        self.write("A.sln", A_SOLUTION)
        first = self.bind(apply=True)
        self.assertEqual(self.concern(first, "build").status, BOUND)
        before = self.manifest_text()
        second = self.bind(apply=False)
        self.assertEqual(self.concern(second, "build").status, ALREADY_BOUND)
        self.assertFalse(second.wrote)
        self.assertEqual(second.after, second.before)
        self.assertEqual(self.manifest_text(), before)

    def test_the_generated_comment_is_written_once(self):
        self.write_manifest()
        self.write("A.sln", A_SOLUTION)
        self.bind(apply=True)
        self.assertEqual(self.manifest_text().count(GENERATED_MARKER), 1)
        # And a further apply does not accumulate a second copy.
        self.bind(apply=True)
        self.assertEqual(self.manifest_text().count(GENERATED_MARKER), 1)

    def test_bindings_land_in_the_manifests_own_checks_order(self):
        # The concerns are handed to the renderer in the order `checks:` lists
        # them, so the result reads in the repository's own order.
        self.write_manifest("  tests: true\n  build: true\n")
        self.write("A.sln", A_SOLUTION)
        self.bind(apply=True)
        text = self.manifest_text()
        self.assertLess(
            text.index("  tests: 'dotnet test A.sln'"),
            text.index("  build: 'dotnet build A.sln'"),
        )

    def test_a_persons_own_command_is_left_in_place(self):
        # A person wrote a different command: it is reported, never clobbered.
        # The sibling concern that was *not* declared is still bound, so the
        # check is on the hand-written line itself, not on the whole file.
        self.write_manifest(
            text=MANIFEST.format(checks=BUILD_AND_TESTS).replace(
                "blocking:",
                "commands:\n  build: 'cargo build --release --all-features'\nblocking:",
            )
        )
        self.write("Cargo.toml", '[package]\nname = "a"\n')
        before = self.manifest_text()
        outcome = self.bind(apply=True)
        got = self.concern(outcome, "build")
        self.assertEqual(got.status, CONFLICT)
        self.assertIn("left untouched", got.reason)
        after = self.manifest_text()
        # The hand-written line is byte-identical, and the derived command for
        # the same concern appears nowhere.
        self.assertEqual(
            [line for line in after.splitlines() if line.startswith("  build:")],
            [line for line in before.splitlines() if line.startswith("  build:")],
        )
        self.assertNotIn("  build: 'cargo build'\n", after)
        # The sibling concern that was not hand-written is still bound.
        self.assertIn("  tests: 'cargo test'\n", after)

    def test_a_conflict_is_counted_as_left_in_place_and_not_as_debt(self):
        # A conflicting concern has a command the gate will run, so it is not an
        # unbound check. Counting it as debt would report wired checks as gaps.
        # `discover_repos` only finds a repository that has a `.git`, so the
        # fixture is `git init`ed before the fleet-level roll-up is asserted.
        self.write_manifest(
            text=MANIFEST.format(checks=BUILD_AND_TESTS).replace(
                "blocking:",
                "commands:\n  build: 'cargo build --release --all-features'\nblocking:",
            )
        )
        self.write("Cargo.toml", '[package]\nname = "a"\n')
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)
        report = run(self.tmp, apply=False, check_clean=False)
        self.assertEqual(len(report.conflicts()), 1)
        self.assertEqual(report.debt_total(), 0)
        self.assertEqual(report.count(RESOLVED), 1)

    def test_an_identical_existing_command_is_already_bound_not_a_conflict(self):
        for style in ("'dotnet build A.sln'", '"dotnet build A.sln"', "dotnet build A.sln"):
            with self.subTest(style=style):
                self.write_manifest(
                    text=MANIFEST.format(checks="  build: true\n").replace(
                        "blocking:", f"commands:\n  build: {style}\nblocking:"
                    )
                )
                self.write("A.sln", A_SOLUTION)
                got = self.concern(self.bind(), "build")
                self.assertEqual(got.status, ALREADY_BOUND)
                self.assertEqual(got.command, "dotnet build A.sln")

    def test_the_comparison_against_an_existing_command_is_semantic(self):
        self.assertEqual(unquote_scalar("'cargo build'"), "cargo build")
        self.assertEqual(unquote_scalar('"git diff --check"'), "git diff --check")
        self.assertEqual(unquote_scalar("cargo build  # the fast one"), "cargo build")
        self.assertEqual(unquote_scalar("'rg -e ''password'''"), "rg -e 'password'")

    def test_only_the_concerns_the_manifest_declared_are_bound(self):
        # The repository selects `tests` and nothing else, so it must not gain a
        # `security` command it never asked for.
        self.write_manifest(TESTS_ONLY)
        self.write("package.json", json.dumps({"scripts": {"test": "jest"}}))
        outcome = self.bind(apply=True)
        self.assertEqual([c.concern for c in outcome.concerns], ["tests"])
        text = self.manifest_text()
        self.assertIn("  tests: 'npm test'\n", text)
        self.assertNotIn("security", text)

    def test_a_false_check_stays_false_and_is_not_bound(self):
        self.write_manifest("  build: false\n  tests: true\n")
        self.write("package.json", json.dumps({"scripts": {"test": "jest"}}))
        outcome = self.bind(apply=True)
        self.assertEqual([c.concern for c in outcome.concerns], ["tests"])
        self.assertIn("  build: false\n", self.manifest_text())
        self.assertNotIn("build:", self.manifest_text().split("commands:")[1])

    def test_no_checks_value_is_changed(self):
        self.write_manifest("  build: true\n  tests: true\n  security: false\n")
        self.write("A.sln", A_SOLUTION)
        self.bind(apply=True)
        self.assertIn("  security: false\n", self.manifest_text())
        self.assertIn("  build: true\n", self.manifest_text())

        self.assertIn("  build: true\n", self.manifest_text())


class GitWriteContractTests(GitRepoCase):
    """The write paths that stop at `git status` before touching a file."""

    def test_a_dirty_manifest_is_skipped_not_written(self):
        self.write("A.sln", A_SOLUTION)
        self.commit()
        # The owner's uncommitted edit: modify the manifest without staging it.
        with (self.repo / ".ai-gate" / "gate.yaml").open("a", encoding="utf-8") as handle:
            handle.write("# owner WIP\n")
        before = self.manifest_text()
        outcome = bind_repo(self.repo, apply=True, check_clean=True)
        self.assertEqual(outcome.status, DIRTY)
        self.assertFalse(outcome.wrote)
        self.assertEqual(self.manifest_text(), before)

    def test_a_clean_repository_is_written(self):
        self.write("A.sln", A_SOLUTION)
        self.commit()
        outcome = bind_repo(self.repo, apply=True, check_clean=True)
        self.assertTrue(outcome.wrote)
        self.assertIn("dotnet build A.sln", self.manifest_text())

    def test_an_untracked_manifest_is_treated_as_dirty(self):
        self.write("A.sln", A_SOLUTION)
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)
        outcome = bind_repo(self.repo, apply=True, check_clean=True)
        self.assertEqual(outcome.status, DIRTY)

    def test_a_dirty_manifest_is_still_classified_and_recorded(self):
        # The write is refused; the reading is not. The gate resolves a
        # dirty repository exactly as it resolves a clean one and fails on
        # the same checks, so a repository skipped for uncommitted work
        # would take its real debt out of the record — and an `--apply` run
        # straight after a previous one writes a record missing every
        # repository it had just written to.
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        self.commit()
        with (self.repo / ".ai-gate" / "gate.yaml").open("a", encoding="utf-8") as handle:
            handle.write("# owner WIP\n")
        outcome = bind_repo(self.repo, apply=True, check_clean=True)
        self.assertEqual(outcome.status, DIRTY)
        report = Report()
        report.add(outcome)
        debt = build_debt(report, self.repo)
        self.assertEqual(
            [(r["repo"], [u["concern"] for u in r["unresolved"]])
             for r in debt["repositories"]],
            [("demo", ["build", "tests"])],
        )
        self.assertEqual(
            self.concern(outcome, "build").reason, PURE_PYTHON_BUILD
        )


class WorkspaceWriteTests(GitRepoCase):
    """The write contract across a whole fixture workspace, via `run()`."""

    def workspace(self) -> Path:
        root = self.tmp / "ws"
        specs = {
            "argoset": (BUILD_AND_TESTS, {"Cargo.toml": "[workspace]\nmembers = []\n"}),
            "cvunify": ("  build: true\n", {"CvUnify.slnx": A_SLNX}),
            "alethefy": (
                BUILD_AND_TESTS,
                {
                    "pyproject.toml": "[project]\nname = 'p'\n",
                    "tests/test_x.py": "import unittest\nclass T(unittest.TestCase):\n"
                    "    def test_a(self):\n        pass\n",
                },
            ),
        }
        for name, (checks, files) in specs.items():
            repo = root / name
            (repo / ".ai-gate").mkdir(parents=True)
            (repo / ".ai-gate" / "gate.yaml").write_text(
                MANIFEST.format(checks=checks), encoding="utf-8"
            )
            for relative, body in files.items():
                target = repo / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(body, encoding="utf-8")
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
            subprocess.run(
                [
                    "git", "-c", "user.email=t@t", "-c", "user.name=t",
                    "commit", "-qm", "base",
                ],
                cwd=repo,
                check=True,
            )
        return root

    def manifests(self, root: Path) -> dict:
        return {
            path.name: path.read_bytes()
            for path in sorted(root.glob("*/.ai-gate/gate.yaml"))
        }

    def test_a_dry_run_over_a_workspace_writes_nothing(self):
        root = self.workspace()
        before = self.manifests(root)
        report = run(root, apply=False)
        self.assertEqual(len(report.outcomes), 3)
        self.assertEqual(self.manifests(root), before, "a dry run changed a manifest")
        self.assertFalse(any(o.wrote for o in report.outcomes))

    def test_apply_then_a_second_apply_is_byte_for_byte_stable(self):
        root = self.workspace()
        run(root, apply=True)
        first = self.manifests(root)
        run(root, apply=True)
        self.assertEqual(self.manifests(root), first, "the second apply changed a manifest")

    def test_the_run_counts_roll_up_statuses(self):
        root = self.workspace()
        report = run(root, apply=False)
        self.assertEqual(report.names(RESOLVED), ["argoset", "cvunify"])
        self.assertEqual(report.names(PARTIAL), ["alethefy"])
        self.assertEqual(report.count(DIRTY), 0)
        self.assertEqual(report.count(REFUSED), 0)
        # `alethefy` is a pure-Python package: one concern bound, one refused.
        self.assertEqual(report.debt_total(), 1)

    def test_the_run_can_be_restricted_to_named_repositories(self):
        root = self.workspace()
        report = run(root, apply=False, only=["cvunify"])
        self.assertEqual([o.repo for o in report.outcomes], ["cvunify"])

    def test_an_explicitly_targeted_new_scaffold_needs_no_git_directory(self):
        root = self.tmp / "ws"
        repo = root / "fresh-product"
        (repo / ".ai-gate").mkdir(parents=True)
        (repo / ".ai-gate" / "gate.yaml").write_text(
            MANIFEST.format(checks=BUILD_AND_TESTS), encoding="utf-8"
        )
        report = run(root, apply=True, only=["fresh-product"], check_clean=False)
        self.assertEqual([outcome.repo for outcome in report.outcomes], ["fresh-product"])
        self.assertTrue((repo / ".ai-gate" / "gate.yaml").read_text().startswith("version: 1"))


# --- the debt record ------------------------------------------


class DebtOnlyTests(GitRepoCase):
    """`--debt-only` regenerates the record from a run that cannot write.

    Regenerating the record has twice been the thing that wrote the fleet,
    because `--apply` is the only mode that persists the record and it also
    writes every manifest it can bind. These tests pin that the record can be
    regenerated on its own.
    """

    def workspace(self) -> Path:
        root = self.tmp / "ws"
        repo = root / "argoset"
        (repo / ".ai-gate").mkdir(parents=True)
        (repo / ".ai-gate" / "gate.yaml").write_text(
            MANIFEST.format(checks=BUILD_AND_TESTS), encoding="utf-8"
        )
        (repo / "Cargo.toml").write_text("[workspace]\nmembers = []\n", encoding="utf-8")
        subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
        subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base"],
            cwd=repo, check=True,
        )
        return root

    def run_main(self, argv: list[str]) -> tuple[int, str]:
        buffer = io.StringIO()
        with redirect_stdout(buffer):
            code = main(argv)
        return code, buffer.getvalue()

    def test_debt_only_writes_the_record_and_no_manifest(self):
        root = self.workspace()
        manifest = root / "argoset" / ".ai-gate" / "gate.yaml"
        before = manifest.read_bytes()
        record = root / DEBT_FILE
        with redirect_stdout(io.StringIO()):
            code = main(["--root", str(root), "--debt-only"])
        self.assertIn(code, (0, 3), "the debt exit code is about debt, not about the mode")
        self.assertTrue(record.is_file(), "--debt-only did not write the record")
        self.assertEqual(manifest.read_bytes(), before, "--debt-only wrote a manifest")

    def test_debt_only_says_the_manifest_run_is_dry(self):
        root = self.workspace()
        _, output = self.run_main(["--root", str(root), "--debt-only"])
        self.assertIn("DEBT RECORD ONLY (manifest run is dry)", output)

    def test_debt_only_is_refused_alongside_apply(self):
        root = self.workspace()
        buffer = io.StringIO()
        stderr = io.StringIO()
        original = sys.stderr
        sys.stderr = stderr
        try:
            with redirect_stdout(buffer):
                code = main(["--root", str(root), "--debt-only", "--apply"])
        finally:
            sys.stderr = original
        self.assertEqual(code, 2)
        self.assertIn("cannot be combined with --apply", stderr.getvalue())
        self.assertFalse((root / "argoset" / ".ai-gate" / "gate.yaml").read_text()
                         .count(GENERATED_MARKER), "the refused run still wrote a manifest")

    def test_the_record_from_debt_only_matches_a_plain_dry_run(self):
        root = self.workspace()
        with redirect_stdout(io.StringIO()):
            main(["--root", str(root), "--debt-only"])
            written = json.loads((root / DEBT_FILE).read_text(encoding="utf-8"))
            dry_report = run(root, apply=False)
        self.assertEqual(written, build_debt(dry_report, root))


class DebtRecordTests(GitRepoCase):
    def workspace(self) -> Path:
        root = self.tmp / "ws"
        for name, checks, files in [
            (
                "alethefy",
                BUILD_AND_TESTS,
                {
                    "pyproject.toml": "[project]\nname = 'p'\n",
                    "tests/test_x.py": "import unittest\nclass T(unittest.TestCase):\n"
                    "    def test_a(self):\n        pass\n",
                },
            ),
            ("argoset", BUILD_AND_TESTS, {"Cargo.toml": "[workspace]\nmembers = []\n"}),
        ]:
            repo = root / name
            (repo / ".ai-gate").mkdir(parents=True)
            (repo / ".ai-gate" / "gate.yaml").write_text(
                MANIFEST.format(checks=checks), encoding="utf-8"
            )
            for relative, body in files.items():
                target = repo / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(body, encoding="utf-8")
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
            subprocess.run(
                ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base"],
                cwd=repo,
                check=True,
            )
        return root

    def debt_file(self, root: Path) -> Path:
        return root / DEBT_FILE

    def call_main(self, root: Path, *args: str) -> tuple[int, str]:
        buffer = io.StringIO()
        with redirect_stdout(buffer):
            code = main(["--root", str(root), *args])
        return code, buffer.getvalue()

    def test_the_debt_record_is_not_written_on_a_dry_run(self):
        root = self.workspace()
        code, out = self.call_main(root)
        self.assertEqual(code, 3, "debt remaining must exit 3")
        self.assertFalse(self.debt_file(root).exists())
        self.assertIn("(not written; dry run)", out)

    def test_the_debt_record_is_written_on_apply(self):
        root = self.workspace()
        code, out = self.call_main(root, "--apply")
        self.assertEqual(code, 3)
        self.assertTrue(self.debt_file(root).exists())
        self.assertIn("(written)", out)

    def test_the_debt_record_names_the_unresolved_concerns(self):
        root = self.workspace()
        self.call_main(root, "--apply")
        debt = json.loads(self.debt_file(root).read_text(encoding="utf-8"))
        self.assertEqual(
            debt["totals"], {"repositories_with_debt": 1, "unresolved_checks": 1}
        )
        self.assertEqual([r["repo"] for r in debt["repositories"]], ["alethefy"])
        entry = debt["repositories"][0]["unresolved"][0]
        self.assertEqual(entry["concern"], "build")
        self.assertEqual(entry["status"], UNRESOLVED)
        self.assertEqual(entry["reason"], PURE_PYTHON_BUILD)

    def test_the_debt_record_does_not_credit_a_conflict_as_debt(self):
        root = self.tmp / "ws"
        repo = root / "argoset"
        (repo / ".ai-gate").mkdir(parents=True)
        (repo / ".ai-gate" / "gate.yaml").write_text(
            MANIFEST.format(checks=BUILD_AND_TESTS).replace(
                "blocking:",
                "commands:\n  build: 'cargo build --release --all-features'\nblocking:",
            ),
            encoding="utf-8",
        )
        (repo / "Cargo.toml").write_text('[package]\nname = "a"\n', encoding="utf-8")
        subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
        subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base"],
            cwd=repo, check=True,
        )
        code, out = self.call_main(root, "--apply")
        debt = json.loads(self.debt_file(root).read_text(encoding="utf-8"))
        self.assertEqual(code, 0, "a conflict is not debt")
        self.assertEqual(debt["totals"]["unresolved_checks"], 0)
        self.assertEqual(debt["repositories"], [])
        # The summary line is column-aligned, so match the number, not the spacing.
        self.assertRegex(out, r"left in place:\s+1\b")

    def test_by_reason_totals_match_the_check_count(self):
        root = self.workspace()
        report = run(root, apply=False)
        debt = build_debt(report, root)
        self.assertEqual(
            sum(group["checks"] for group in debt["by_reason"]),
            len(report.unresolved()),
        )
        self.assertEqual(debt["totals"]["unresolved_checks"], len(report.unresolved()))

    def test_dirty_manifest_debt_includes_commands_that_cannot_be_written(self):
        root = self.workspace()
        manifest = root / "argoset" / ".ai-gate" / "gate.yaml"
        manifest.write_text(manifest.read_text(encoding="utf-8") + "# owner edit in flight\n", encoding="utf-8")
        report = run(root, apply=False)
        debt = build_debt(report, root)
        recorded = {
            (item["repo"], gap["concern"])
            for item in debt["repositories"]
            for gap in item["unresolved"]
        }
        self.assertTrue({("argoset", "build"), ("argoset", "tests")} <= recorded)

    def test_the_debt_record_is_sorted_everywhere(self):
        root = self.workspace()
        debt = build_debt(run(root, apply=False), root)
        names = [r["repo"] for r in debt["repositories"]]
        self.assertEqual(names, sorted(names))
        for group in debt["by_reason"]:
            self.assertEqual(group["repositories"], sorted(group["repositories"]))
            self.assertEqual(group["entries"], sorted(group["entries"]))
            self.assertEqual(group["checks"], len(set(group["entries"])))

    def test_two_dry_runs_produce_the_same_debt_record(self):
        root = self.workspace()
        first = json.dumps(build_debt(run(root, apply=False), root), indent=2) + "\n"
        second = json.dumps(build_debt(run(root, apply=False), root), indent=2) + "\n"
        self.assertEqual(first, second, "the debt record is not reproducible")

    def test_the_debt_record_carries_no_timestamp(self):
        # A `generated_at` field makes every run a diff and trains reviewers to
        # ignore the file.
        root = self.workspace()
        debt = build_debt(run(root, apply=False), root)
        self.assertNotIn("generated_at", debt)
        self.assertEqual(
            [key for key in debt if "time" in key or "date" in key], []
        )

    def test_the_run_exits_zero_when_nothing_is_unresolved(self):
        root = self.tmp / "ws"
        repo = root / "argoset"
        (repo / ".ai-gate").mkdir(parents=True)
        (repo / ".ai-gate" / "gate.yaml").write_text(
            MANIFEST.format(checks=BUILD_AND_TESTS), encoding="utf-8"
        )
        (repo / "Cargo.toml").write_text("[workspace]\nmembers = []\n", encoding="utf-8")
        subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
        subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base"],
            cwd=repo, check=True,
        )
        code, _out = self.call_main(root, "--apply")
        self.assertEqual(code, 0)

    def test_the_verbose_line_is_empty_for_a_repository_with_no_concerns(self):
        self.unlink_manifest()
        outcome = self.bind()
        self.assertEqual(outcome.status, NO_MANIFEST)
        self.assertEqual(format_detail(outcome, True), "")


# --- the per-repository report line ---------------------------


class OutputTests(RepoCase):
    def test_the_verbose_line_shows_the_command_and_its_evidence(self):
        self.write("Cargo.toml", "[workspace]\nmembers = []\n")
        line = format_detail(self.bind(), verbose=True)
        self.assertIn(
            "build=cargo build --workspace <- Cargo.toml (declares [workspace])", line
        )
        self.assertIn("tests=cargo test --workspace", line)

    def test_the_verbose_line_names_an_unresolved_reason(self):
        self.write("pyproject.toml", "[project]\nname = 'p'\n")
        self.assertIn(
            f"build=UNRESOLVED: {PURE_PYTHON_BUILD}",
            format_detail(self.bind(), verbose=True),
        )

    def test_the_verbose_line_marks_a_conflict_without_printing_its_command(self):
        self.write_manifest(
            text=MANIFEST.format(checks="  build: true\n").replace(
                "blocking:", "commands:\n  build: 'cargo build --release'\nblocking:"
            )
        )
        self.write("Cargo.toml", '[package]\nname = "a"\n')
        self.assertEqual(format_detail(self.bind(), verbose=True), "build=CONFLICT")

    def test_a_quiet_line_is_empty(self):
        self.write("A.sln", A_SOLUTION)
        self.assertEqual(format_detail(self.bind(), verbose=False), "")


# --- the manifest reader --------------------------------------


class ReadManifestTests(RepoCase):
    def test_reads_the_selection_in_the_manifests_own_order(self):
        selected, bound, problem = read_manifest(
            MANIFEST.format(checks="  tests: true\n  build: true\n")
        )
        self.assertEqual(problem, "")
        self.assertEqual(selected, ["tests", "build"])
        self.assertEqual(bound, {})

    def test_a_false_check_is_not_a_selection(self):
        selected, _bound, _problem = read_manifest(
            MANIFEST.format(checks="  build: false\n  tests: true\n")
        )
        self.assertEqual(selected, ["tests"])

    def test_reads_the_existing_commands(self):
        _selected, bound, _problem = read_manifest(WITH_COMMANDS)
        self.assertEqual(bound, {"build": "cargo build --all-targets"})

    def test_roll_up_reports_partial_and_unresolved(self):
        self.assertEqual(roll_up([], 0), RESOLVED)
        self.assertEqual(roll_up([self.outcome("build", UNRESOLVED)], 1), UNRESOLVED)
        self.assertEqual(
            roll_up([self.outcome("build", UNRESOLVED), self.outcome("tests", BOUND)], 2),
            PARTIAL,
        )

    @staticmethod
    def outcome(concern_id, status):
        from bind_gate_commands import ConcernOutcome

        return ConcernOutcome("demo", concern_id, status)


# --- the real fleet, tolerantly --------------------------------


class FleetTest(unittest.TestCase):
    """One end-to-end walk of the real workspace.

    Every other test here uses a fixture. This one exists because the two
    errors found in the field were errors of *scale* — 19 repositories on
    `*.slnx` and a repository labelled `typescript` that is a .NET solution —
    and no fixture proves a count. Every assertion is an invariant that must
    hold however the fleet changes; no test names a repository or a number.

    These assertions read the *commanded* concerns, which is `BOUND` for a
    repository this run would have wired and `ALREADY_BOUND` for one a
    previous run already wired. Filtering on `BOUND` alone made every
    assertion vacuous the moment the fleet was applied: the walk then found
    nothing to check and passed only because `checked > 0` happened to hold
    for the untouched repositories. Reading both is the same invariant
    against more of the fleet, and it stays true whether or not anyone has
    run `--apply` yet.
    """

    #: A concern the gate will actually execute a command for.
    COMMANDED = frozenset({BOUND, ALREADY_BOUND})

    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).parents[2]
        if not (cls.root / "workspace-governance").is_dir():
            raise unittest.SkipTest(f"workspace root not present: {cls.root}")
        cls.report = run(cls.root, apply=False)

    def test_the_fleet_walk_never_refuses_a_repository(self):
        refused = [o for o in self.report.outcomes if o.status == REFUSED]
        self.assertEqual(
            [(o.repo, o.detail) for o in refused],
            [],
            "a repository the tool cannot read is a finding, not an expected state",
        )


    def test_every_path_a_bound_command_names_really_exists(self):
        # The strongest invariant available without running anything: if a
        # command names a file, that file must be in the repository. A binder
        # that points the gate at a path that is not there would fail every
        # check it wires up.
        checked = 0
        for outcome in self.report.outcomes:
            for concern in outcome.concerns:
                if concern.status not in self.COMMANDED:
                    continue
                for path in paths_in(concern.command) | paths_in(concern.evidence):
                    self.assertTrue(
                        (self.root / outcome.repo / path).exists(),
                        f"{outcome.repo}/{concern.concern}: names a path that is not "
                        f"in the repository: {path}",
                    )
                    checked += 1
        self.assertGreater(checked, 0, "no path was checked at all")

    def test_the_fleet_exercises_the_slnx_path_at_scale(self):
        # The `.slnx` regression, at the scale it actually appeared: many
        # repositories in this fleet carry no `*.sln` at all, and a `*.sln`-only
        # search silently finds nothing in them. A `.slnx` repository must still
        # get a .NET command. Asserted as "at least one exists" and never as a
        # count, so ordinary fleet churn cannot break it.
        from_slnx = [
            (outcome.repo, concern.concern, concern.command)
            for outcome in self.report.outcomes
            for concern in outcome.concerns
            if concern.status in self.COMMANDED and ".slnx" in concern.command
        ]
        self.assertTrue(
            from_slnx,
            "no .slnx repository received a .NET command; the .sln-only search bug "
            "is back, or the fleet no longer uses .slnx",
        )
        for repo, concern_id, command in from_slnx:
            for path in paths_in(command):
                self.assertTrue(
                    (self.root / repo / path).exists(),
                    f"{repo}/{concern_id}: {command} names a file that is not there",
                )

    def test_no_bound_dotnet_command_is_missing_its_entry_file(self):
        # A `dotnet` command must name a solution or a project. A repository
        # with neither is expected to be refused, not handed a bare `dotnet
        # build` that cannot run.
        checked = 0
        for outcome in self.report.outcomes:
            for concern in outcome.concerns:
                if concern.status not in self.COMMANDED:
                    continue
                if not concern.command.startswith("dotnet "):
                    continue
                checked += 1
                # A `dotnet` command may name its target (`dotnet build Foo.slnx`,
                # `dotnet test tests/Foo.UnitTests`) or omit it, which is legal
                # when the repository root holds exactly one solution or project
                # for the bare invocation to resolve. What must never happen is a
                # bare `dotnet build` in a repository with no build entry point at
                # all -- that command cannot run.
                named_dirs = any(
                    (self.root / outcome.repo / token).is_dir()
                    for token in concern.command.split()
                    if not token.startswith("-")
                )
                root = self.root / outcome.repo
                root_entries = [
                    p for p in root.iterdir()
                    if p.suffix in (".sln", ".slnx") or p.suffix.endswith("proj")
                ] if root.is_dir() else []
                self.assertTrue(
                    paths_in(concern.command) or named_dirs or len(root_entries) == 1,
                    f"{outcome.repo}/{concern.concern}: {concern.command} names no "
                    f"build file, no project directory, and the root holds "
                    f"{len(root_entries)} build entry points rather than exactly one",
                )
        self.assertGreater(checked, 0, "no .NET command was bound across the fleet")

    def test_every_unresolved_concern_carries_a_reason(self):
        for concern in self.report.unresolved():
            self.assertTrue(
                concern.reason,
                f"{concern.repo}/{concern.concern} is unresolved with no reason",
            )

    def test_every_bound_concern_carries_a_command_and_its_evidence(self):
        for outcome in self.report.outcomes:
            for concern in outcome.concerns:
                if concern.status != BOUND:
                    continue
                self.assertTrue(concern.command, f"{outcome.repo}/{concern.concern}")
                self.assertTrue(
                    concern.evidence, f"{outcome.repo}/{concern.concern} has no evidence"
                )


# --- what a profile selects ------------------------------------


# `aesthra`'s real shape, reduced: a product repository that binds every
# check it declares, so the only thing the gate can still fail on is what
# its profile scheduled. Before this pass that repository was absent from
# `gate-debt.json` while the gate reported three UNBOUND checks for it.
PRODUCT_MANIFEST = """version: 1
runtime: driftwatchdog
profile: product
checks:
{checks}blocking:
  - FAIL
"""

# The three checks `profile: product` schedules. `source-file-size` is the
# fourth: the gate runs it in process, so it is never unbound and must never
# appear in the record.
#
# `governance-quality` binds wherever the workspace checker is reachable, so
# it leaves this tuple on a repository that has one. The two product-quality
# concerns do not: their checker is fail-closed on a `quality` block the
# repository has not written, and this tool invents no policy.
PRODUCT_PROFILE_DEBT = (
    "governance-quality",
    "placeholder-threshold",
    "product-code-boundary",
)

# The concerns a bare fixture leaves in debt, and the one it closes. A test
# that wants all three unresolved removes the sibling checkout first.
UNREACHABLE_PROFILE_DEBT = PRODUCT_PROFILE_DEBT


class ProfileSelectionTests(RepoCase):
    """A `profile:` schedules checks the manifest never lists.

    The binder used to read only `checks:`, so every check the gate
    scheduled through a profile was invisible to it: a repository whose
    only unbound checks came from its profile had no unresolved concerns,
    and therefore no entry in the debt record at all. Across this fleet
    that was 30 repositories and 122 checks the gate was failing on.

    The fixture is a **sibling** of a `workspace-governance` checkout, which
    is the shape every repository in this fleet has. Reachability is
    therefore true by default here, and the tests that need it false remove
    the sibling with [`forget_sibling`]. Modelling the real layout is the
    point: a fixture that cannot reach a checker could not tell "refused
    because the checker is missing" apart from "refused for another reason",
    which is the whole distinction this pass turns on.
    """

    def setUp(self):
        super().setUp()
        self.write_sibling()
        # Every repository in this fleet carries a `.project.json`, and the
        # command that reaches the workspace checkers is declared in it as a
        # `mechanism: sibling` edge. The fixture reproduces that layout for
        # the same reason it reproduces the sibling checkout: a repository
        # with nowhere to declare the edge cannot bind it lawfully, so a
        # fixture without one could not tell "refused because the declaration
        # is missing" apart from "bound".
        self.write_declaration()
        self.write_manifest(checks=BUILD_AND_TESTS, text=PRODUCT_MANIFEST.format(
            checks=BUILD_AND_TESTS
        ))

    def debt(self) -> dict:
        report = Report()
        report.add(self.bind())
        return build_debt(report, self.repo)

    def entry(self, debt: dict) -> dict:
        """This repository's debt entry, or a failure naming what is there."""
        found = [r for r in debt["repositories"] if r["repo"] == "demo"]
        if len(found) != 1:
            raise AssertionError(
                "demo is missing from the debt record; the record names "
                f"{[r['repo'] for r in debt['repositories']]}"
            )
        return found[0]

    def unresolved(self, debt: dict) -> dict[str, str]:
        return {u["concern"]: u["reason"] for u in self.entry(debt)["unresolved"]}

    def test_a_profile_scheduled_check_is_recorded_as_debt(self):
        self.write("App.sln", A_SOLUTION)
        self.write("App/App.csproj", A_CSPROJ)
        # `build` and `tests` bind. Of the profile's three, `governance-quality`
        # binds too — the workspace checker is reachable — so only the two
        # product-quality concerns are left, and they are the two the gate
        # still reports for this manifest.
        self.assertEqual(
            set(self.unresolved(self.debt())),
            {"placeholder-threshold", "product-code-boundary"},
        )

    def test_a_repository_whose_only_debt_is_profile_scheduled_is_still_listed(self):
        # The shape that lost 30 repositories: every declared check is
        # bound, so nothing in `checks:` is unresolved and the entry used to
        # be dropped for having no gaps at all.
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        debt = self.debt()
        self.assertEqual(
            set(self.unresolved(debt)), {"placeholder-threshold", "product-code-boundary"}
        )
        self.assertEqual(debt["totals"]["repositories_with_debt"], 1)
        self.assertEqual(debt["totals"]["unresolved_checks"], 2)

    def test_a_profile_scheduled_check_names_the_way_out(self):
        # The reason has to name the file and the key, or the next reader
        # has nothing to act on. It names both, it names the block that would
        # be written, and it offers the two real ways out: write the policy,
        # or exempt the check with a reason. This fixture carries a Rust
        # source root, so the block is derivable and the reason says so.
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\n")
        reason = self.unresolved(self.debt())["placeholder-threshold"]
        self.assertIn(".project.json", reason)
        self.assertIn("`quality`", reason)
        self.assertIn("product_roots", reason)
        self.assertIn("not_applicable", reason)

    def test_the_in_process_check_is_never_recorded(self):
        # The gate executes `source-file-size` itself; it is scheduled by
        # the same profile as the other three and is never unbound.
        self.assertNotIn("source-file-size", self.unresolved(self.debt()))

    def test_a_disabled_profile_check_is_not_recorded(self):
        self.write_manifest(text=PRODUCT_MANIFEST.format(
            checks=BUILD_AND_TESTS + "  governance-quality: false\n"
        ))
        self.assertNotIn("governance-quality", self.unresolved(self.debt()))

    def test_an_exempted_check_is_not_recorded(self):
        self.write_manifest(text=PRODUCT_MANIFEST.format(checks=BUILD_AND_TESTS).replace(
            "blocking:",
            'not_applicable:\n  placeholder-threshold: "no placeholder text here"\nblocking:',
        ))
        self.assertNotIn("placeholder-threshold", self.unresolved(self.debt()))

    def test_an_optional_check_is_neither_bound_nor_recorded(self):
        # `"optional"` schedules the check but does not make it required, so
        # the gate does not fail on it having no command.
        self.write_manifest(text=PRODUCT_MANIFEST.format(
            checks=BUILD_AND_TESTS + "  accessibility: optional\n"
        ))
        self.assertNotIn("accessibility", self.unresolved(self.debt()))

    def test_a_profile_scheduled_check_is_bound_in_both_blocks_when_reachable(self):
        # The change this file exists to pin. The gate refuses a `commands:`
        # entry for a check the manifest does not select, so the old answer
        # was to record and never write. But `profile: product` is the
        # repository's own declaration that it runs these three checks, and
        # the gate is already failing it for not binding them. The correct
        # write is both halves, in one operation.
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        outcome = self.bind(apply=True)
        text = self.manifest_text()
        self.assertEqual(self.concern(outcome, "governance-quality").status, BOUND)
        # Both halves, so the manifest stays parseable AND the check runs.
        self.assertIn("  governance-quality: true\n", text)
        # The command is scoped to THIS repository, because the concern
        # filters by code and not by project: an unscoped workspace audit
        # charges every repository for every other repository's findings.
        self.assertIn(
            "  governance-quality: 'python3 ../workspace-governance/scripts/"
            "workspace_check.py --root .. --project demo --json'",
            text,
        )
        # And it leaves the debt record.
        self.assertNotIn("governance-quality", self.unresolved(self.debt()))

    def test_the_governance_command_is_scoped_to_this_repository(self):
        # The scoping is a correctness property, not a style choice. The
        # gate's concern filters findings by `code` alone, so a
        # workspace-wide audit makes every repository's gate fail on every
        # other repository's findings. Each repository's own name goes in.
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        self.bind(apply=True)
        self.assertIn("--project demo", self.manifest_text())

    def test_the_checks_entry_precedes_the_commands_entry_it_licenses(self):
        # A `commands:` entry above a `checks:` entry is fine for the parser
        # (both are top-level blocks), but the ordering that reads correctly
        # is selection first, then the command that satisfies it. It also
        # means a reader diffing the file sees the selection change beside
        # the profile that asked for it rather than at the bottom.
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        self.bind(apply=True)
        text = self.manifest_text()
        self.assertLess(
            text.index("  governance-quality: true"),
            text.index("commands:"),
            "the selection was written below the commands block",
        )

    def test_a_profile_scheduled_check_with_no_reachable_checker_stays_in_debt(self):
        # The negative half, and the case that protects the invariant. With
        # no sibling workspace-governance checkout there is no checker to
        # reach, so NEITHER half is written: a `checks:` entry with no
        # command behind it is strictly worse than the gap, because it makes
        # a declared check one the gate now fails on while running nothing.
        self.forget_sibling()
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        before = self.manifest_text()
        outcome = self.bind(apply=True)
        self.assertEqual(self.manifest_text(), before, "a selection was written with no command")
        self.assertEqual(
            self.concern(outcome, "governance-quality").status, UNRESOLVED
        )
        self.assertEqual(
            set(self.unresolved(self.debt())), set(UNREACHABLE_PROFILE_DEBT)
        )
        # And the reason says why, naming the path that is not there.
        self.assertIn(
            "workspace_check.py", self.unresolved(self.debt())["governance-quality"]
        )

    def test_a_missing_quality_block_names_the_file_and_the_key(self):
        # The debt record has to be actionable. "needs configuration" tells
        # the next reader nothing; ".project.json has no `quality` block,
        # and the checker exits 2 without one" tells them what to write.
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\n")
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        reason = self.unresolved(self.debt())["placeholder-threshold"]
        self.assertIn(".project.json", reason)
        self.assertIn("`quality`", reason)
        self.assertIn("exits 2", reason)
        # A dry run writes nothing, so the concern is not bound even though
        # the block is derivable and the checker is perfectly reachable.
        self.assertNotIn("placeholder-threshold", self.manifest_text())

    def test_a_quality_block_is_derived_from_disk_and_binds_both_concerns(self):
        # The change this class now covers. The block's two derivable fields
        # come from the files on disk, with the manifest each root came from
        # attached as evidence, and the third field is the stated strict
        # default. Both concerns are answered by one scan, and both are bound
        # in the same operation that writes the block.
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\n")
        self.write_declaration()
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        outcome = self.bind(apply=True)
        for concern in ("placeholder-threshold", "product-code-boundary"):
            self.assertEqual(self.concern(outcome, concern).status, BOUND, concern)
        block = json.loads((self.repo / ".project.json").read_text(encoding="utf-8"))["quality"]
        self.assertEqual(block["product_roots"], ["src"])
        self.assertEqual(block["languages"], ["rust"])
        self.assertEqual(block["placeholder_threshold"], 0)
        # The judgement is written as a deliberate default, and the file
        # carries no invented `exclude` and no hand-copied marker list: the
        # marker vocabulary is the checker's own, and a guessed exclusion
        # hides a real finding rather than reporting one.
        self.assertNotIn("exclude", block)
        self.assertNotIn("placeholder_markers", block)
        text = self.manifest_text()
        for concern in ("placeholder-threshold", "product-code-boundary"):
            self.assertIn(f"  {concern}: true", text)
            self.assertIn("product_code_quality.py .", text)
        # And the concern left the debt record: a fresh run over the applied
        # repository finds nothing unresolved, which is the point of writing
        # the block and the command together.
        self.assertEqual(self.debt()["totals"]["unresolved_checks"], 0)

    def test_the_derived_root_names_the_manifest_it_came_from(self):
        # A root is a fact about where the code is, and the evidence is what
        # makes it checkable rather than asserted.
        self.write("rust/Cargo.toml", '[package]\nname = "demo"\n')
        self.write("rust/src/lib.rs", "pub fn f() {}\n")
        block, evidence, reason = derive_quality_block(self.repo, detect_shape(self.repo))
        self.assertEqual(reason, "")
        self.assertEqual(block["product_roots"], ["rust/src"])
        self.assertIn("rust/Cargo.toml", evidence)
        self.assertIn("rust/src", evidence)

    def test_a_repository_with_no_derivable_product_root_is_refused(self):
        # The refusal. A manifest with no `src` directory beside it gives
        # nothing to scan, and this tool does not fall back to `.` — a root
        # that scans the whole repository is not a derivation, and the
        # checker would report on files that are not product.
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("README.md", "# demo\n")
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        block, _evidence, reason = derive_quality_block(self.repo, detect_shape(self.repo))
        self.assertIsNone(block)
        self.assertIn("no product root is derivable", reason)
        self.assertIn("Cargo.toml", reason, "the refusal must name what it examined")
        self.assertIn(".project.json", reason, "and name the way out")
        # Nothing was written for the quality concerns: not the block, not the
        # selection, not the command. A selection with no command behind it is
        # worse than the gap it replaced. (`governance-quality` is reachable
        # in this fixture and does bind, declaring its sibling edge in the
        # repository's own `.project.json`; it is not what this test is about.)
        before = self.manifest_text()
        outcome = self.bind(apply=True)
        text = self.manifest_text()
        self.assertNotIn("placeholder-threshold", text)
        self.assertNotIn("product-code-boundary", text)
        self.assertIn("governance-quality", text, "the fixture's own concern still binds")
        self.assertNotEqual(text, before)
        declaration = json.loads((self.repo / ".project.json").read_text(encoding="utf-8"))
        self.assertNotIn(
            "quality",
            declaration,
            "a refused quality concern wrote no policy block",
        )
        self.assertEqual(
            [item["id"] for item in declaration["interfaces"]],
            ["workspace-check"],
            "only the reachable sibling edge was declared",
        )
        self.assertEqual(
            self.concern(outcome, "placeholder-threshold").status, UNRESOLVED
        )
        self.assertIn(
            "placeholder-threshold", self.unresolved(self.debt())
        )

    def test_a_test_project_is_never_a_product_root(self):
        # The checker's own `DEFAULT_EXCLUDED_DIRS` skips `tests`, so a root
        # under one would scan nothing and report PASS. A false pass is worse
        # than a refusal, so the only `.csproj` files here yield no root.
        self.write("tests/Api.Tests/Api.Tests.csproj", A_CSPROJ)
        self.write("tests/Api.Tests/Thing.cs", "class Thing {}\n")
        block, _evidence, reason = derive_quality_block(self.repo, detect_shape(self.repo))
        self.assertIsNone(block)
        self.assertIn("no product root is derivable", reason)

    def test_a_hand_written_quality_block_is_left_exactly_as_it_is(self):
        # The repository's own policy is its own decision, including the parts
        # this tool would derive differently. A project that wrote
        # `product_roots: ["app"]` gets `app` scanned, not `src`. The one key
        # this run adds is the declaration of the sibling edge its own bound
        # command walks; the hand-written block is untouched, byte for byte.
        original = {
            "schema_version": 1,
            "id": "demo",
            "quality": {
                "product_roots": ["app"],
                "languages": ["python"],
                "placeholder_threshold": 3,
            },
        }
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\n")
        self.write(".project.json", json.dumps(original, indent=2) + "\n")
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        before = (self.repo / ".project.json").read_text(encoding="utf-8")
        outcome = self.bind(apply=True)
        after = (self.repo / ".project.json").read_text(encoding="utf-8")
        self.assertEqual(
            json.loads(after)["quality"],
            original["quality"],
            "the block's own values were changed",
        )
        self.assertEqual(
            {key: value for key, value in json.loads(after).items() if key != "interfaces"},
            json.loads(before),
            "the run changed something other than the declared sibling edge",
        )
        self.assertEqual(
            [item["id"] for item in json.loads(after)["interfaces"]],
            ["product-code-quality", "workspace-check"],
        )
        self.assertIn(
            "declared interfaces -> product-code-quality, workspace-check", outcome.detail
        )

    def test_a_declared_concern_is_not_given_a_duplicate_selection(self):
        # A repository that already lists the concern made the selection
        # itself. The second run of an applied repository must add no second
        # `checks:` entry and rewrite nothing.
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\n")
        self.write_declaration()
        self.write_manifest(text=PRODUCT_MANIFEST.format(
            checks=BUILD_AND_TESTS + "  placeholder-threshold: true\n"
        ))
        self.bind(apply=True)
        text = self.manifest_text()
        self.assertEqual(
            text.count("  placeholder-threshold: true"), 1, "the selection was duplicated"
        )
        self.assertIn(
            "  placeholder-threshold: 'python3 ../workspace-governance/scripts/"
            "product_code_quality.py .'",
            text,
        )
        # Idempotence: a second apply changes nothing.
        manifest_before = self.manifest_text()
        declaration_before = (self.repo / ".project.json").read_text(encoding="utf-8")
        self.bind(apply=True)
        self.assertEqual(self.manifest_text(), manifest_before)
        self.assertEqual(
            (self.repo / ".project.json").read_text(encoding="utf-8"),
            declaration_before,
            "a second run rewrote the block",
        )

    def test_a_dry_run_writes_no_quality_block(self):
        # The whole tool is dry by default, and this is the file it did not
        # write before. It stays unwritten.
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\n")
        self.write_declaration()
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        before = self.manifest_text()
        declaration = (self.repo / ".project.json").read_text(encoding="utf-8")
        outcome = self.bind(apply=False)
        self.assertEqual(self.manifest_text(), before)
        self.assertEqual(
            (self.repo / ".project.json").read_text(encoding="utf-8"),
            declaration,
            "a dry run wrote the block",
        )
        self.assertIn("would bind", outcome.detail)
        self.assertIn("placeholder-threshold", self.unresolved(self.debt()))

    def test_a_repository_with_no_declaration_is_refused_not_given_one(self):
        # `.project.json` is the registry's document. Writing a one-key file
        # to stand in for it would be inventing the repository's identity, so
        # the run refuses by name and writes no manifest entry either. The
        # sibling edge makes this sharper: a command that reaches another
        # checkout is lawful only when the declaration accounts for it, so a
        # repository with nowhere to declare the edge cannot bind the command.
        (self.repo / ".project.json").unlink()
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\\n")
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        outcome = self.bind(apply=True)
        self.assertEqual(outcome.status, REFUSED)
        self.assertIn(".project.json", outcome.detail)
        self.assertNotIn("placeholder-threshold", self.manifest_text())
        self.assertNotIn(
            "governance-quality: 'python3",
            self.manifest_text(),
            "a command that reaches another checkout is not bound without a declaration",
        )
        self.assertFalse((self.repo / ".project.json").exists())

    def test_a_quality_policy_is_honoured_when_the_repository_wrote_one(self):
        # The other half of the previous two: a repository that HAS declared
        # the policy gets both halves, because the checker can then actually
        # run. This is the 1 of 81 case, and it is the one that proves the
        # refusal above is about the missing policy and not about the checker.
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        self.write(".project.json", json.dumps({"quality": {"product_roots": ["src"]}}))
        outcome = self.bind(apply=True)
        self.assertEqual(
            self.concern(outcome, "placeholder-threshold").status, BOUND
        )
        text = self.manifest_text()
        self.assertIn("  placeholder-threshold: true", text)
        self.assertIn("product_code_quality.py .", text)

    def test_an_already_declared_concern_is_not_given_a_duplicate(self):
        # A repository that already lists the concern has made the selection
        # itself; adding a second entry would be writing over its own
        # decision. Only the `commands:` half is missing.
        self.write_manifest(text=PRODUCT_MANIFEST.format(
            checks=BUILD_AND_TESTS + "  governance-quality: true\n"
        ))
        self.bind(apply=True)
        text = self.manifest_text()
        self.assertEqual(
            text.count("  governance-quality: true"), 1, "the selection was duplicated"
        )
        self.assertIn("workspace_check.py", text)

    def test_a_disabled_concern_is_never_selected(self):
        # `checks: <id>: false` is the repository taking the concern out of
        # the gate's selection. The binder must not write it back as `true`,
        # whatever the profile schedules and whatever a checker could answer.
        self.write_manifest(text=PRODUCT_MANIFEST.format(
            checks=BUILD_AND_TESTS + "  governance-quality: false\n"
        ))
        self.bind(apply=True)
        text = self.manifest_text()
        self.assertIn("  governance-quality: false", text)
        self.assertNotIn("  governance-quality: true", text)
        self.assertNotIn("governance-quality: 'python3", text)

    def test_a_repositorys_own_script_wins_over_the_workspace_checker(self):
        # A repository that wrote `scripts/check-governance-quality.*` meant
        # that script. The workspace checker is a fallback for repositories
        # that wrote none, not an override.
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        self.write("scripts/check-governance-quality.mjs", "export default [];\n")
        outcome = self.bind(apply=True)
        self.assertEqual(
            self.concern(outcome, "governance-quality").command,
            "scripts/check-governance-quality.mjs",
        )
        self.assertNotIn("workspace_check.py", self.manifest_text())

    def test_a_manifest_declared_profile_check_with_a_script_is_bound(self):
        # `driftwatchdog`'s shape: the concern is named under `checks:` as
        # well as by the profile, so a command for it is legal and binds.
        self.write_manifest(text=PRODUCT_MANIFEST.format(
            checks=BUILD_AND_TESTS + "  governance-quality: true\n"
        ))
        self.write("scripts/check-governance-quality.mjs", "export default [];\n")
        outcome = self.bind(apply=True)
        self.assertEqual(self.concern(outcome, "governance-quality").status, BOUND)
        self.assertIn(
            "  governance-quality: 'scripts/check-governance-quality.mjs'",
            self.manifest_text(),
        )

    def test_an_unknown_profile_schedules_nothing(self):
        # A profile that is not built in selects exactly the checks the
        # manifest declares, so 17 of this fleet's 21 profile names add
        # nothing on their own. The declared `build` and `tests` are
        # unresolvable in this bare fixture; the point is that no
        # profile-scheduled check joins them.
        self.write_manifest(text=PRODUCT_MANIFEST.format(checks=BUILD_AND_TESTS).replace(
            "profile: product", "profile: web-saas"
        ))
        self.assertEqual(set(self.unresolved(self.debt())), {"build", "tests"})

    def test_the_minimal_profile_schedules_nothing(self):
        self.assertEqual(gate_required([], read_gate_policy("profile: minimal\n")), [])

    def test_the_profile_table_matches_the_gate(self):
        # Every built-in profile name the gate accepts, with the same
        # defaults. A name added on the gate's side without a row here would
        # silently stop contributing debt, which is this defect again.
        self.assertEqual(
            set(PROFILE_DEFAULTS),
            {
                "backend", "frontend", "full", "minimal", "product",
                "rust-product", "release", "deployable", "typescript-product",
                "python-product", "dotnet-product", "dotnet-library",
                "typescript-monorepo", "rust-platform", "tooling", "deployment",
                "inventory", "governance",
            },
        )
        self.assertEqual(
            PROFILE_DEFAULTS["product"],
            (
                "governance-quality",
                "placeholder-threshold",
                "product-code-boundary",
                "source-file-size",
            ),
        )


# --- the sibling edge a bound command makes --------------------


INVENTORY_MANIFEST = """version: 1
runtime: driftwatchdog
profile: inventory
checks:
blocking:
  - FAIL
"""

GOVERNANCE_COMMAND = (
    "python3 ../workspace-governance/scripts/workspace_check.py "
    "--root .. --project demo --json"
)
PRODUCT_QUALITY_COMMAND = (
    "python3 ../workspace-governance/scripts/product_code_quality.py ."
)


class SiblingDeclarationTests(RepoCase):
    """A bound command that reaches another checkout declares the edge it makes.

    The declaration is `mechanism: sibling`, the provider, `required: true`,
    and a digest of the invocation the tool wrote. `workspace_check.py`
    recomputes that digest from the `entry` it reads, so a declaration this
    tool writes is asserted twice here: against the command string, and
    against the audit that would charge the edge if it were unpinned.
    """

    def setUp(self):
        super().setUp()
        self.write_sibling()
        self.write_declaration(
            {
                "role": "product",
                "stack": "python",
                "kind": "python-product",
                "profile": "inventory",
                "lifecycle": "inventory",
                "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
                "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
            }
        )
        self.write_manifest(checks="", text=INVENTORY_MANIFEST)

    def declaration(self) -> dict:
        return json.loads((self.repo / ".project.json").read_text(encoding="utf-8"))

    def interfaces(self) -> list:
        return self.declaration().get("interfaces", [])

    def audit(self) -> list:
        # The other side of the edge: the provider checkout declares the
        # provision, so a passing audit here also proves the pairing and not
        # merely the pin.
        provider = self.repo.parent / "workspace-governance"
        (provider / ".project.json").write_text(
            json.dumps(
                {
                    **{key: value for key, value in self.declaration().items() if key not in ("interfaces",)},
                    "id": "workspace-governance",
                    "interfaces": [
                        {
                            "id": interface_id,
                            "provides": True,
                            "mechanism": "invoke",
                            "entry": f"scripts/{relative}",
                            "contract": f"platform.{interface_id}/1.0.0",
                            "required": False,
                        }
                        for interface_id, relative in (
                            ("workspace-check", "workspace_check.py"),
                            ("product-code-quality", "product_code_quality.py"),
                        )
                    ],
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        registry = self.repo.parent / "projects.json"
        registry.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "workspace_root": None,
                    "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
                    "projects": [
                        {
                            "id": "demo",
                            "path": "demo",
                            "profile": "inventory",
                            "role": "product",
                            "stack": "python",
                            "lifecycle": "inventory",
                            "adoption": "adopted",
                        },
                        {
                            "id": "workspace-governance",
                            "path": "workspace-governance",
                            "profile": "inventory",
                            "role": "product",
                            "stack": "python",
                            "lifecycle": "inventory",
                            "adoption": "adopted",
                        },
                    ],
                }
            ),
            encoding="utf-8",
        )
        findings, _summary = workspace_check.run(
            SimpleNamespace(
                registry=str(registry),
                root=str(self.repo.parent),
                project=None,
                audit=True,
                self_check=False,
                as_json=False,
            )
        )
        return findings

    def test_a_fresh_bind_declares_exactly_one_sibling_edge(self):
        outcome = self.bind(apply=True)
        self.assertEqual(self.concern(outcome, "governance-quality").status, BOUND)
        self.assertEqual(
            self.interfaces(),
            [
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": "workspace-governance",
                    "mechanism": "sibling",
                    "entry": GOVERNANCE_COMMAND,
                    "digest": "sha256:" + hashlib.sha256(GOVERNANCE_COMMAND.encode("utf-8")).hexdigest(),
                    "required": True,
                    "evidence_ref": ".ai-gate/gate.yaml",
                }
            ],
        )
        self.assertIn(GOVERNANCE_COMMAND, self.manifest_text())
        self.assertIn("declared interfaces -> workspace-check", outcome.detail)

    def test_the_audit_recomputes_the_digest_and_reports_nothing(self):
        self.bind(apply=True)
        entry = self.interfaces()[0]
        self.assertEqual(
            entry["digest"],
            "sha256:" + hashlib.sha256(entry["entry"].encode("utf-8")).hexdigest(),
            "the digest must be the one the audit recomputes from the entry",
        )
        findings = self.audit()
        self.assertEqual(
            [item.code for item in findings if item.code.startswith("INTERFACE_")],
            [],
            findings,
        )

    def test_a_second_apply_declares_nothing_new(self):
        self.bind(apply=True)
        first = (self.repo / ".project.json").read_bytes()
        manifest = self.manifest_text()
        outcome = self.bind(apply=True)
        self.assertEqual(self.concern(outcome, "governance-quality").status, ALREADY_BOUND)
        self.assertEqual((self.repo / ".project.json").read_bytes(), first)
        self.assertEqual(self.manifest_text(), manifest)

    def test_a_dry_run_writes_no_declaration(self):
        before = (self.repo / ".project.json").read_bytes()
        outcome = self.bind(apply=False)
        self.assertIn("would write interfaces -> workspace-check", outcome.detail)
        self.assertEqual((self.repo / ".project.json").read_bytes(), before)

    def test_two_concerns_over_one_command_declare_one_edge(self):
        # `placeholder-threshold` and `product-code-boundary` are two gates
        # over one scan, so the same command answers both and the edge is
        # declared once.
        self.write("Cargo.toml", '[package]\nname = "demo"\n')
        self.write("src/main.rs", "fn main() {}\n")
        self.write_manifest(checks="", text=PRODUCT_MANIFEST.format(checks=""))
        outcome = self.bind(apply=True)
        for concern in ("placeholder-threshold", "product-code-boundary"):
            self.assertEqual(self.concern(outcome, concern).status, BOUND)
        ids = [item["id"] for item in self.interfaces()]
        self.assertEqual(sorted(ids), ["product-code-quality", "workspace-check"])
        self.assertEqual(len(ids), len(set(ids)), "an edge was declared twice")
        quality = next(item for item in self.interfaces() if item["id"] == "product-code-quality")
        self.assertEqual(quality["entry"], PRODUCT_QUALITY_COMMAND)
        self.assertEqual(quality["required"], True)
        self.assertEqual(quality["provider"], "workspace-governance")

    def test_an_unreachable_checker_declares_nothing(self):
        self.forget_sibling()
        outcome = self.bind(apply=True)
        self.assertEqual(self.concern(outcome, "governance-quality").status, UNRESOLVED)
        self.assertNotIn("interfaces", self.declaration())

    def test_a_repository_with_no_declaration_cannot_bind_a_sibling_command(self):
        (self.repo / ".project.json").unlink()
        outcome = self.bind(apply=True)
        self.assertEqual(self.concern(outcome, "governance-quality").status, UNRESOLVED)
        self.assertIn(".project.json", self.concern(outcome, "governance-quality").reason)
        self.assertNotIn("governance-quality", self.manifest_text())
        self.assertFalse((self.repo / ".project.json").exists())

    def test_a_command_that_cannot_be_described_leaves_the_concern_unbound(self):
        # A command that walks into another checkout in a shape this tool
        # cannot resolve to a provider and an entry is not declared and not
        # bound: a fabricated declaration would be worse than the gap.
        with mock.patch(
            "bind_gate_commands.derive",
            return_value=("python3 ../Bad_Provider/scripts/x.py", "elsewhere", ""),
        ):
            outcome = self.bind(apply=True)
        concern = self.concern(outcome, "governance-quality")
        self.assertEqual(concern.status, UNRESOLVED)
        self.assertIn("cannot describe that edge", concern.reason)
        self.assertIn("mechanism: consume", concern.reason)
        self.assertNotIn("interfaces", self.declaration())

    def test_a_hand_written_entry_is_never_overwritten(self):
        declaration = self.declaration()
        declaration["interfaces"] = [
            {
                "id": "workspace-check",
                "consumes": True,
                "provider": "workspace-governance",
                "mechanism": "sibling",
                "entry": "python3 ../workspace-governance/scripts/workspace_check.py --root .. --project demo --json",
                "digest": "sha256:" + "a" * 64,
                "required": True,
                "evidence_ref": ".ai-gate/gate.yaml",
            }
        ]
        (self.repo / ".project.json").write_text(json.dumps(declaration, indent=2) + "\n")
        self.bind(apply=True)
        self.assertEqual(self.interfaces()[0]["digest"], "sha256:" + "a" * 64)


# --- the committed record against the gate itself --------------


GATE_PLAN_HEADER = "gate plan (dry-run"
GATE_UNBOUND_HEADER = "unbound required checks ("
GATE_UNBOUND_ENTRY = re.compile(r"^  - ([A-Za-z0-9_.-]+):")


def gate_binary(root: Path) -> Path | None:
    """The gate binary to use as the oracle, or None when it is not built.

    `PATH` is deliberately not consulted. A `cargo install`ed build is
    routinely older than the repository it is asked about, and an older
    build does not report unbound checks at all — the test below proves the
    binary is a usable oracle before it compares anything against it.
    """
    for candidate in (
        root / "driftwatchdog" / "target" / "debug" / "driftwatchdog",
        root / "driftwatchdog" / "target" / "release" / "driftwatchdog",
    ):
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    return None


def gate_unbound(gate: Path, repo: Path) -> set[str]:
    """The check ids `driftwatchdog gate --dry-run` reports unbound for `repo`.

    The gate's own dry-run output is the authority for what it fails on;
    this reads the one section of it that says so, and refuses to guess
    when that section is absent or unrecognised.
    """
    completed = subprocess.run(
        [str(gate), "gate", "--dry-run"],
        cwd=repo,
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
    )
    text = completed.stdout + completed.stderr
    if GATE_PLAN_HEADER not in text:
        raise AssertionError(
            f"{repo.name}: the gate printed no dry-run plan, so it cannot be "
            f"read as an oracle:\n{text.strip()[:400]}"
        )
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if not line.startswith(GATE_UNBOUND_HEADER):
            continue
        found = set()
        for entry in lines[index + 1:]:
            match = GATE_UNBOUND_ENTRY.match(entry)
            if match is None:
                break
            found.add(match.group(1))
        return found
    return set()


class GateOracleTest(unittest.TestCase):
    """The committed debt record must equal the gate's unbound set.

    The record exists so that a reader — human or agent — cannot walk past
    an unclosed gate check. A record that names a subset reads as complete,
    which is the failure this pins: 48 of 170 checks across 25 of 55
    repositories is worse than no record, because nothing in the file says
    so.

    Every assertion is set equality, never a count, so the fleet may grow,
    shrink or re-profile without breaking it. What cannot change is that the
    record and the gate agree.
    """

    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).parents[2]
        if not (cls.root / "workspace-governance").is_dir():
            raise unittest.SkipTest(f"workspace root not present: {cls.root}")
        cls.gate = gate_binary(cls.root)
        if cls.gate is None:
            raise unittest.SkipTest(
                "the driftwatchdog gate binary is not built; run `cargo build` "
                "in driftwatchdog/ to check the debt record against the gate"
            )
        if not cls.gate_reports_unbound():
            raise unittest.SkipTest(
                f"{cls.gate} does not report unbound required checks, so it "
                "predates the feature and cannot be the oracle; rebuild it"
            )
        cls.record = json.loads(
            (cls.root / "workspace-governance" / DEBT_FILE).read_text(encoding="utf-8")
        )
        cls.recorded = {
            (r["repo"], u["concern"])
            for r in cls.record["repositories"]
            for u in r["unresolved"]
        }
        cls.unbound = set()
        for repo in sorted(cls.root.iterdir()):
            if not (repo / ".ai-gate" / "gate.yaml").is_file():
                continue
            cls.unbound |= {(repo.name, c) for c in gate_unbound(cls.gate, repo)}

    @classmethod
    def gate_reports_unbound(cls) -> bool:
        """Positive control: does this binary report an unbound check at all?

        One throwaway repository with a required check and no command. A
        build that does not report unbound checks answers an empty set for
        every repository, and the comparison below would then fail on the
        record instead of on the tool.
        """
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp) / "oracle-probe"
            (repo / ".ai-gate").mkdir(parents=True)
            (repo / ".ai-gate" / "gate.yaml").write_text(
                "version: 1\nruntime: driftwatchdog\nprofile: minimal\nchecks:\n"
                "  tests: true\nblocking:\n  - FAIL\n",
                encoding="utf-8",
            )
            try:
                return gate_unbound(cls.gate, repo) == {"tests"}
            except AssertionError:
                return False

    def test_the_record_names_every_check_the_gate_calls_unbound(self):
        missing = sorted(self.unbound - self.recorded)
        self.assertEqual(
            missing,
            [],
            f"{len(missing)} unbound required check(s) the gate reports and the debt "
            f"record does not name, in {len({r for r, _ in missing})} "
            f"repositories; first few: {missing[:6]}",
        )

    def test_the_record_names_nothing_the_gate_calls_bound(self):
        extra = sorted(self.recorded - self.unbound)
        self.assertEqual(
            extra,
            [],
            f"{len(extra)} check(s) the record calls debt that the gate does not "
            f"report as unbound; first few: {extra[:6]}",
        )

    def test_a_fresh_run_reproduces_the_gate_set(self):
        # The committed record can only stay right while the generator is.
        # A binder that stopped seeing profile-scheduled checks would leave
        # today's record looking perfect and the next `--apply` quietly
        # write a worse one, which is the same defect one commit later.
        report = run(self.root, apply=False)
        produced = {
            (outcome.repo, concern.concern)
            for outcome in report.outcomes
            for concern in debt_concerns(outcome)
        }
        self.assertEqual(
            sorted(produced - self.unbound),
            [],
            "a fresh run finds debt the gate does not report as unbound",
        )
        self.assertEqual(
            sorted(self.unbound - produced),
            [],
            "the gate reports unbound checks a fresh run does not find; the "
            "record would be regenerated without them",
        )

    def test_the_record_totals_match_what_it_lists(self):
        self.assertEqual(
            self.record["totals"]["unresolved_checks"], len(self.recorded)
        )
        self.assertEqual(
            self.record["totals"]["repositories_with_debt"],
            len({r for r, _ in self.recorded}),
        )

    def test_every_listed_gap_carries_a_reason(self):
        for entry in self.record["repositories"]:
            for gap in entry["unresolved"]:
                self.assertTrue(
                    gap["reason"],
                    f"{entry['repo']}/{gap['concern']} is listed with no reason",
                )



# Suffixes that make a token a file rather than prose. A concern's evidence is
# a sentence, so only these are read as paths and checked against the disk.
BUILD_FILE_SUFFIXES = (
    ".sln", ".slnx", ".csproj", ".fsproj", ".vbproj",
    ".json", ".toml", ".yaml", ".yml", ".ini", ".cfg",
    ".mjs", ".js", ".ts", ".sh", ".py", ".gradle", ".xml",
)
PATH_TOKEN = re.compile(r"^[A-Za-z0-9_.\-/]+$")


def paths_in(text: str) -> set:
    """Every build-file path named in `text`, and nothing that is only prose."""
    found = set()
    for token in text.replace("(", " ").replace(")", " ").replace("`", " ").split():
        cleaned = token.strip(",;:'\"`")
        if PATH_TOKEN.match(cleaned) and cleaned.endswith(BUILD_FILE_SUFFIXES):
            found.add(cleaned)
    return found


if __name__ == "__main__":
    unittest.main()
