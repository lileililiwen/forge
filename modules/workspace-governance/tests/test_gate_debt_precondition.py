"""Tests for the `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` precondition.

The rule asks the question no document can ask: may this repository open a
*new* OpenSpec change while it still carries a required check the gate reports
as `UNBOUND`?

The properties that matter, and the ones this file pins:

* a repository with **no** debt is silent, however many changes it has;
* a debted repository opening a **new** change is an `ERROR` naming the
  unbound concerns, the change, and the change that records the fix;
* a change that was already open when the rule landed is **grandfathered** by
  the committed ratchet, and a debted repository can therefore never be a
  permanent red — which is the difference between a ratchet and a deadlock;
* the debt-closing change is never a trigger, because a repository that may
  not start the work which fixes its debt can never fix it;
* a deliberate exception satisfies the rule at `WARNING` and stays visible
  with its reason quoted, so a decision is reviewable rather than invisible;
* every one of the three records is read **fail-closed**: a missing, corrupt or
  mistyped record fails the run rather than reading as "no debt";
* the checker **reads** the ratchet and never writes it.

The gate test at the end is the one that cannot be faked in Python: it runs the
real `driftwatchdog` binary over a fixture, because an undeclared code is
filtered out of the concern's verdict and reports PASS while the checker exits
1. `PATH` is never consulted — a `cargo install`ed build routinely predates the
repository it is asked about.

No test uses `rm`: this host's `rm` is a recoverable-delete shim that prints to
stdout and can fail. Cleanup uses `tempfile.TemporaryDirectory()`.
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "scripts"))
import gate_debt_baseline  # noqa: E402
import workspace_check  # noqa: E402
from workspace_check import (  # noqa: E402
    GATE_DEBT_BASELINE_FILE,
    GATE_DEBT_BASELINE_KEYS,
    GATE_DEBT_CLOSING_CHANGES,
    GATE_DEBT_EXCEPTIONS_FILE,
    GATE_DEBT_EXCEPTIONS_KEYS,
    GateDebtBaselineError,
    GateDebtExceptionsError,
    GateDebtRecordError,
    load_gate_debt_baseline,
    load_gate_debt_exceptions,
    load_gate_debt_record,
    validate_new_change_with_gate_debt,
)


def write_record(path: Path, list_key: str, entries: list[dict[str, str]]) -> None:
    path.write_text(json.dumps({"schema_version": 1, list_key: entries}, indent=2), encoding="utf-8")


def debt_record(entries: dict[str, list[str]]) -> dict:
    return {
        "schema_version": 1,
        "totals": {"repositories_with_debt": len(entries), "unresolved_checks": sum(len(v) for v in entries.values())},
        "repositories": [
            {"repo": repo, "shape": "node", "unresolved": [{"concern": c, "status": "UNRESOLVED", "reason": "r"} for c in concerns]}
            for repo, concerns in entries.items()
        ],
    }


class RecordsCase(unittest.TestCase):
    """A temporary governance root holding the three records the rule reads."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.gov = Path(self._tmp.name)
        self.write_debt({"actoria": ["accessibility", "privacy"]})
        self.write_baseline([("actoria", "post-mvp-readiness")])
        self.write_exceptions([])

    def write_debt(self, entries: dict[str, list[str]]) -> None:
        (self.gov / "gate-debt.json").write_text(json.dumps(debt_record(entries), indent=2), encoding="utf-8")

    def write_baseline(self, pairs: list[tuple[str, str]]) -> None:
        write_record(
            self.gov / GATE_DEBT_BASELINE_FILE,
            "findings",
            [{"project": project, "change": change} for project, change in pairs],
        )

    def write_exceptions(self, entries: list[tuple[str, str, str]]) -> None:
        write_record(
            self.gov / GATE_DEBT_EXCEPTIONS_FILE,
            "exceptions",
            [{"project": p, "change": c, "reason": r} for p, c, r in entries],
        )


class RuleCase(RecordsCase):
    """A project directory with named unarchived changes, plus the rule's call."""

    def setUp(self):
        super().setUp()
        self.project = self.gov / "actoria"
        self.write_gate_manifest({"accessibility": "python3 scripts/check-accessibility.py", "privacy": "python3 scripts/check-privacy.py"})
        self.findings: list[workspace_check.Finding] = []

    def write_gate_manifest(self, checks: dict[str, str | None]) -> None:
        gate = self.project / ".ai-gate" / "gate.yaml"
        gate.parent.mkdir(parents=True, exist_ok=True)
        lines = ["profile: minimal", "checks:"]
        lines.extend(f"  {concern}: true" for concern in checks)
        commands = {concern: command for concern, command in checks.items() if command}
        if commands:
            lines.append("commands:")
            lines.extend(f"  {concern}: {command}" for concern, command in commands.items())
        gate.write_text("\n".join(lines) + "\n", encoding="utf-8")

    def open_changes(self, *names: str) -> None:
        for name in names:
            (self.project / "openspec" / "changes" / name).mkdir(parents=True, exist_ok=True)

    def archive(self, name: str) -> None:
        (self.project / "openspec" / "changes" / "archive" / name).mkdir(parents=True, exist_ok=True)

    def audit(self) -> list[workspace_check.Finding]:
        self.findings = []
        validate_new_change_with_gate_debt(
            self.project,
            self.findings,
            "actoria",
            debt=load_gate_debt_record(self.gov),
            baseline=load_gate_debt_baseline(self.gov),
            exceptions=load_gate_debt_exceptions(self.gov),
        )
        return self.findings

    def codes(self) -> list[str]:
        return [item.code for item in self.audit()]


class RuleTests(RuleCase):
    def test_a_repository_with_no_debt_is_silent_however_many_changes_it_has(self):
        self.write_debt({"aegify": ["build"]})
        self.write_gate_manifest({"accessibility": "python3 scripts/check-accessibility.py"})
        self.open_changes("post-mvp-readiness", "close-unbound-gate-checks", "another-feature")
        self.assertEqual(self.codes(), [])

    def test_a_new_live_unbound_check_blocks_when_the_snapshot_says_clean(self):
        self.write_debt({})
        self.write_gate_manifest({
            "accessibility": "python3 scripts/check-accessibility.py",
            "privacy": "python3 scripts/check-privacy.py",
            "performance": None,
        })
        self.open_changes("billing-seam")
        findings = self.audit()
        self.assertEqual([item.code for item in findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        self.assertEqual(findings[0].level, "ERROR")
        self.assertIn("performance", findings[0].message)
        self.assertIn("billing-seam", findings[0].message)

    def test_a_new_live_bound_check_does_not_increase_debt(self):
        self.write_debt({})
        self.write_gate_manifest({
            "accessibility": "python3 scripts/check-accessibility.py",
            "privacy": "python3 scripts/check-privacy.py",
            "performance": "python3 scripts/check-performance.py",
        })
        self.open_changes("billing-seam")
        self.assertEqual(self.codes(), [])

    def test_unreadable_live_gate_state_fails_closed_for_new_work(self):
        (self.project / ".ai-gate" / "gate.yaml").unlink()
        self.write_debt({})
        self.open_changes("billing-seam")
        findings = self.audit()
        self.assertEqual([item.code for item in findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        self.assertIn("could not establish", findings[0].message)

    def test_unknown_live_gate_profile_fails_closed_for_new_work(self):
        (self.project / ".ai-gate" / "gate.yaml").write_text(
            "profile: typo-profile\nchecks:\n  build: true\n", encoding="utf-8"
        )
        self.write_debt({})
        self.open_changes("billing-seam")
        findings = self.audit()
        self.assertEqual([item.code for item in findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        self.assertIn("typo-profile", findings[0].message)

    def test_malformed_live_checks_mapping_fails_closed_for_new_work(self):
        (self.project / ".ai-gate" / "gate.yaml").write_text(
            "profile: minimal\nchecks:\n    performance: true\n", encoding="utf-8"
        )
        self.write_debt({})
        self.open_changes("billing-seam")
        findings = self.audit()
        self.assertEqual([item.code for item in findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        self.assertIn("malformed", findings[0].message)

    def test_yaml_comment_in_command_value_is_treated_as_unbound(self):
        (self.project / ".ai-gate" / "gate.yaml").write_text(
            "profile: minimal\nchecks:\n  performance: true\ncommands:\n  performance: # no command\n",
            encoding="utf-8",
        )
        self.write_debt({})
        self.open_changes("billing-seam")
        findings = self.audit()
        self.assertEqual([item.code for item in findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        self.assertIn("performance", findings[0].message)

    def test_empty_not_applicable_reason_cannot_hide_live_debt(self):
        (self.project / ".ai-gate" / "gate.yaml").write_text(
            "profile: minimal\nchecks:\n  performance: true\n"
            "not_applicable:\n  performance:\n",
            encoding="utf-8",
        )
        self.write_debt({})
        self.open_changes("billing-seam")
        findings = self.audit()
        self.assertEqual([item.code for item in findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        self.assertIn("malformed `not_applicable:`", findings[0].message)

    def test_a_new_change_on_a_debted_repository_is_an_error_naming_what_to_do(self):
        self.open_changes("post-mvp-readiness", "billing-seam")
        findings = self.audit()
        self.assertEqual([item.code for item in findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        item = findings[0]
        self.assertEqual(item.level, "ERROR")
        self.assertEqual(item.project, "actoria")
        # Actionable: the unbound concerns, the new change, the change that
        # records the fix, and where a deliberate decision goes.
        self.assertIn("accessibility, privacy", item.message)
        self.assertIn("'billing-seam'", item.message)
        self.assertIn("close-unbound-gate-checks", item.message)
        self.assertIn(GATE_DEBT_EXCEPTIONS_FILE, item.message)

    def test_a_grandfathered_change_alone_is_silent(self):
        self.open_changes("post-mvp-readiness")
        self.assertEqual(self.codes(), [])

    def test_the_ratchet_grandfathers_the_pair_it_names_and_no_other(self):
        self.open_changes("post-mvp-readiness", "billing-seam")
        findings = self.audit()
        self.assertEqual(len(findings), 1)
        self.assertIn("'billing-seam'", findings[0].message)
        self.assertNotIn("post-mvp-readiness", findings[0].message)

    def test_the_debt_closing_change_is_never_a_trigger(self):
        self.write_baseline([])
        self.open_changes("close-unbound-gate-checks")
        self.assertEqual(self.codes(), [])

    def test_an_archived_change_is_not_new_work(self):
        self.archive("billing-seam")
        self.write_baseline([])
        self.assertEqual(self.codes(), [])

    def test_a_repository_with_no_openspec_directory_is_silent(self):
        (self.project / "openspec" / "changes").mkdir(parents=True)
        self.write_baseline([])
        self.assertEqual(self.codes(), [])

    def test_a_recorded_exception_satisfies_the_rule_and_stays_visible(self):
        self.write_exceptions([("actoria", "billing-seam", "owner asked for it on 2026-10-05")])
        self.open_changes("post-mvp-readiness", "billing-seam")
        findings = self.audit()
        self.assertEqual(len(findings), 1)
        item = findings[0]
        # Only ERROR moves the exit code, so WARNING satisfies the rule
        # without hiding it: the reason is quoted and the record is named.
        self.assertEqual(item.level, "WARNING")
        self.assertIn("owner asked for it on 2026-10-05", item.message)
        self.assertIn(GATE_DEBT_EXCEPTIONS_FILE, item.message)

    def test_an_exception_does_not_cover_a_change_nobody_recorded(self):
        self.write_exceptions([("actoria", "billing-seam", "recorded")])
        self.open_changes("post-mvp-readiness", "billing-seam", "search-index")
        findings = self.audit()
        # The exception is per pair, not per repository: the recorded change is
        # satisfied and visible, the unrecorded one still blocks.
        self.assertEqual(
            [(item.level, item.message.split("'")[1]) for item in findings],
            [("WARNING", "billing-seam"), ("ERROR", "search-index")],
        )

    def test_an_unreadable_ratchet_grandfathers_nothing(self):
        # Fail-closed in the direction that keeps the rule honest: a ratchet
        # that could not be read is not a ratchet, so already-open work is
        # reported again rather than silently allowed. The run-level ERROR is
        # asserted in `RunTests`; this pins the per-project half.
        (self.gov / GATE_DEBT_BASELINE_FILE).write_text("{not json", encoding="utf-8")
        self.open_changes("post-mvp-readiness")
        self.findings = []
        validate_new_change_with_gate_debt(
            self.project,
            self.findings,
            "actoria",
            debt=load_gate_debt_record(self.gov),
            baseline=None,
            exceptions={},
        )
        self.assertEqual([item.code for item in self.findings], ["QUALITY_NEW_CHANGE_WITH_GATE_DEBT"])
        self.assertEqual(self.findings[0].level, "ERROR")

    def test_an_unreadable_debt_record_reports_no_repository_as_debt_free(self):
        # `debt=None` is what the caller passes after it has already failed the
        # run: the record was never read, so the rule asserts nothing about any
        # repository. The run-level ERROR is what blocks.
        self.open_changes("billing-seam")
        self.findings = []
        validate_new_change_with_gate_debt(
            self.project, self.findings, "actoria", debt=None, baseline=frozenset(), exceptions={}
        )
        self.assertEqual(self.findings, [])

    def test_the_rule_and_the_generator_agree_on_the_debt_closing_change(self):
        self.assertIn("close-unbound-gate-checks", GATE_DEBT_CLOSING_CHANGES)


class RecordLoaderTests(RecordsCase):
    """Every record is read fail-closed, with the drift baseline's refusals."""

    def test_the_records_load(self):
        self.assertEqual(load_gate_debt_record(self.gov), {"actoria": ("accessibility", "privacy")})
        self.assertEqual(load_gate_debt_baseline(self.gov), frozenset({("actoria", "post-mvp-readiness")}))
        self.assertEqual(load_gate_debt_exceptions(self.gov), {})

    def test_a_missing_record_raises_naming_the_file(self):
        for name, error in (
            ("gate-debt.json", GateDebtRecordError),
            (GATE_DEBT_BASELINE_FILE, GateDebtBaselineError),
            (GATE_DEBT_EXCEPTIONS_FILE, GateDebtExceptionsError),
        ):
            with self.subTest(record=name):
                (self.gov / name).unlink(missing_ok=True)
                with self.assertRaises(error) as caught:
                    if name == "gate-debt.json":
                        load_gate_debt_record(self.gov)
                    elif name == GATE_DEBT_BASELINE_FILE:
                        load_gate_debt_baseline(self.gov)
                    else:
                        load_gate_debt_exceptions(self.gov)
                self.assertIn(name, str(caught.exception))

    def test_a_corrupt_record_raises_rather_than_reading_as_empty(self):
        for loader, error in (
            (load_gate_debt_record, GateDebtRecordError),
            (load_gate_debt_baseline, GateDebtBaselineError),
            (load_gate_debt_exceptions, GateDebtExceptionsError),
        ):
            with self.subTest(loader=loader.__name__):
                for name in ("gate-debt.json", GATE_DEBT_BASELINE_FILE, GATE_DEBT_EXCEPTIONS_FILE):
                    (self.gov / name).write_text("{not json", encoding="utf-8")
                    with self.assertRaises(error):
                        loader(self.gov)

    def test_a_record_that_is_not_an_object_or_has_no_array_raises(self):
        for text in ("[]", '"x"', "{}", '{"findings": {}}'):
            with self.subTest(text=text):
                (self.gov / GATE_DEBT_BASELINE_FILE).write_text(text, encoding="utf-8")
                with self.assertRaises(GateDebtBaselineError):
                    load_gate_debt_baseline(self.gov)

    def test_an_entry_with_an_unknown_key_is_refused_not_ignored(self):
        # A typo that were ignored would silently drop a recorded entry, and the
        # recorded entry is the whole exit.
        for name, keys, list_key, error, extra in (
            (GATE_DEBT_BASELINE_FILE, GATE_DEBT_BASELINE_KEYS, "findings", GateDebtBaselineError, {"reasn": "typo"}),
            (
                GATE_DEBT_EXCEPTIONS_FILE,
                GATE_DEBT_EXCEPTIONS_KEYS,
                "exceptions",
                GateDebtExceptionsError,
                {"project": "actoria", "change": "x", "reasn": "typo"},
            ),
        ):
            with self.subTest(record=name):
                (self.gov / name).write_text(json.dumps({list_key: [extra]}), encoding="utf-8")
                with self.assertRaises(error) as caught:
                    (load_gate_debt_baseline if error is GateDebtBaselineError else load_gate_debt_exceptions)(self.gov)
                self.assertIn("unknown key", str(caught.exception))
                self.assertIn("reasn", str(caught.exception))
                self.assertEqual(set(keys), {"project", "change"} | ({"reason"} if error is GateDebtExceptionsError else set()))

    def test_an_entry_missing_a_key_or_carrying_an_empty_value_raises(self):
        for entry in ({"project": "actoria"}, {"project": "", "change": "x"}, {"project": 1, "change": "x"}):
            with self.subTest(entry=entry):
                (self.gov / GATE_DEBT_BASELINE_FILE).write_text(json.dumps({"findings": [entry]}), encoding="utf-8")
                with self.assertRaises(GateDebtBaselineError):
                    load_gate_debt_baseline(self.gov)

    def test_an_empty_snapshot_is_valid_and_grandfathers_nothing(self):
        # Distinct from the drift baseline, which requires an entry: the honest
        # state of this ratchet before the first legacy repository opens a
        # change, and the state a reviewer drives it back to as debt is paid.
        for name in (GATE_DEBT_BASELINE_FILE, GATE_DEBT_EXCEPTIONS_FILE):
            with self.subTest(record=name):
                (self.gov / name).write_text(json.dumps({"findings": [], "exceptions": []}), encoding="utf-8")
                self.assertEqual(load_gate_debt_baseline(self.gov), frozenset())
                self.assertEqual(load_gate_debt_exceptions(self.gov), {})

    def test_a_debt_record_entry_without_a_concern_raises(self):
        (self.gov / "gate-debt.json").write_text(
            json.dumps({"repositories": [{"repo": "actoria", "unresolved": [{"status": "UNRESOLVED"}]}]}),
            encoding="utf-8",
        )
        with self.assertRaises(GateDebtRecordError):
            load_gate_debt_record(self.gov)

    def test_a_repository_with_no_unresolved_concerns_is_not_debt(self):
        self.write_debt({})
        self.assertEqual(load_gate_debt_record(self.gov), {})


class GeneratorTests(RecordsCase):
    """The ratchet is generated from disk, and `--check` writes nothing."""

    def build_workspace(self) -> Path:
        for repo, changes in (
            ("actoria", ["post-mvp-readiness", "close-unbound-gate-checks", "billing-seam"]),
            ("aegify", ["post-mvp-readiness"]),
        ):
            for change in changes:
                (self.gov / repo / "openspec" / "changes" / change).mkdir(parents=True, exist_ok=True)
        self.write_debt({"actoria": ["accessibility"], "aegify": ["build"]})
        return self.gov

    def run_generator(self, *args: str) -> int:
        return gate_debt_baseline.main(
            ["--governance-root", str(self.gov), "--root", str(self.gov), *args]
        )

    def test_the_generator_writes_the_open_non_closing_pairs(self):
        self.build_workspace()
        out = self.gov / "written-baseline.json"
        self.assertEqual(self.run_generator("--out", str(out)), 0)
        self.assertEqual(
            json.loads(out.read_text(encoding="utf-8"))["findings"],
            [
                {"project": "actoria", "change": "billing-seam"},
                {"project": "actoria", "change": "post-mvp-readiness"},
                {"project": "aegify", "change": "post-mvp-readiness"},
            ],
        )

    def test_check_mode_reports_drift_and_writes_nothing(self):
        self.build_workspace()
        target = self.gov / GATE_DEBT_BASELINE_FILE
        stale = target.read_text(encoding="utf-8")
        self.assertEqual(self.run_generator("--check"), 1)
        self.assertEqual(target.read_text(encoding="utf-8"), stale)
        self.run_generator()
        self.assertEqual(self.run_generator("--check"), 0)

    def test_the_generator_refuses_a_root_holding_none_of_the_debted_repositories(self):
        # Otherwise a mistyped root writes an empty ratchet, and an empty
        # ratchet grandfathers nothing: the whole legacy fleet goes red.
        self.write_debt({"actoria": ["accessibility"]})
        empty = self.gov / "elsewhere"
        empty.mkdir()
        self.assertEqual(
            gate_debt_baseline.main(
                ["--governance-root", str(self.gov), "--root", str(empty), "--out", str(self.gov / "x.json")]
            ),
            2,
        )
        self.assertFalse((self.gov / "x.json").exists())

    def test_the_generator_refuses_an_unreadable_debt_record(self):
        (self.gov / "gate-debt.json").unlink(missing_ok=True)
        self.assertEqual(self.run_generator(), 2)


class RunCase(unittest.TestCase):
    """A sandboxed governance root, so `run()` reads these records and not ours.

    `workspace_check.run` resolves its governance root from the script's own
    location, so a sandbox has to be a *copy* of `scripts/` — the same trick
    `test_promote_project.py` uses. The checker is then run as a subprocess,
    which is also the honest way to assert on its exit code.
    """

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.gov = Path(self._tmp.name) / "governance"
        (self.gov / "scripts").mkdir(parents=True)
        for child in (REPO_ROOT / "scripts").iterdir():
            if child.name == "__pycache__":
                continue
            target = self.gov / "scripts" / child.name
            shutil.copytree(child, target) if child.is_dir() else shutil.copy2(child, target)
        for name in ("vocabulary.json", "capabilities.json", "rules-drift-baseline.json", "gate-debt.json"):
            shutil.copy2(REPO_ROOT / name, self.gov / name)
        # This unit suite owns its debt fixture. Do not let a refreshed live
        # fleet snapshot silently change the behavior under test.
        (self.gov / "gate-debt.json").write_text(
            json.dumps(debt_record({"actoria": ["accessibility", "privacy"]}), indent=2),
            encoding="utf-8",
        )
        shutil.copytree(REPO_ROOT / "profiles", self.gov / "profiles")
        shutil.copy2(REPO_ROOT / GATE_DEBT_EXCEPTIONS_FILE, self.gov / GATE_DEBT_EXCEPTIONS_FILE)
        (self.gov / GATE_DEBT_BASELINE_FILE).write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "generator": "tests",
                    "findings": [{"project": "actoria", "change": "post-mvp-readiness"}],
                }
            ),
            encoding="utf-8",
        )
        self.workspace = self.gov / "workspace"
        self.project = self.workspace / "actoria"
        (self.project / "openspec" / "changes" / "post-mvp-readiness").mkdir(parents=True)
        (self.project / ".project.json").write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "id": "actoria",
                    "kind": "product",
                    "role": "product",
                    "stack": "none",
                    "profile": "inventory",
                    "lifecycle": "inventory",
                    "verification": {"command": "cargo test"},
                    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
                }
            ),
            encoding="utf-8",
        )
        gate = self.project / ".ai-gate" / "gate.yaml"
        gate.parent.mkdir(parents=True)
        gate.write_text(
            "profile: minimal\nchecks:\n  accessibility: true\n  privacy: true\n",
            encoding="utf-8",
        )
        registry = json.loads((REPO_ROOT / "projects.json").read_text(encoding="utf-8"))
        registry["projects"] = [
            {"id": "actoria", "path": "actoria", "profile": "inventory", "lifecycle": "inventory", "adoption": "adopted"}
        ]
        (self.gov / "projects.json").write_text(json.dumps(registry, indent=2), encoding="utf-8")

    def run_checker(self) -> tuple[int, dict]:
        completed = subprocess.run(
            [
                sys.executable,
                str(self.gov / "scripts" / "workspace_check.py"),
                "--root",
                str(self.workspace),
                "--registry",
                str(self.gov / "projects.json"),
                "--audit",
                "--json",
            ],
            capture_output=True,
            text=True,
            timeout=300,
            check=False,
        )
        return completed.returncode, json.loads(completed.stdout)

    def new_code(self, document: dict) -> list[dict]:
        return [item for item in document["findings"] if item["code"] == "QUALITY_NEW_CHANGE_WITH_GATE_DEBT"]


class RunTests(RunCase):
    def test_a_grandfathered_change_leaves_the_run_clean(self):
        code, document = self.run_checker()
        self.assertEqual(self.new_code(document), [])
        self.assertEqual(code, 0, document)

    def test_a_new_change_on_a_debted_repository_fails_the_run(self):
        (self.project / "openspec" / "changes" / "billing-seam").mkdir(parents=True)
        code, document = self.run_checker()
        found = self.new_code(document)
        self.assertEqual(code, 1)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0]["level"], "ERROR")
        self.assertEqual(found[0]["project"], "actoria")
        self.assertIn("accessibility", found[0]["message"])

    def test_live_unbound_check_added_after_clean_snapshot_fails_next_new_change(self):
        (self.gov / "gate-debt.json").write_text(
            json.dumps(debt_record({}), indent=2), encoding="utf-8"
        )
        (self.project / ".ai-gate" / "gate.yaml").write_text(
            "profile: minimal\nchecks:\n"
            "  accessibility: true\n  privacy: true\n  late-check: true\n"
            "commands:\n  accessibility: python3 check-a11y.py\n  privacy: python3 check-privacy.py\n",
            encoding="utf-8",
        )
        (self.project / "openspec" / "changes" / "billing-seam").mkdir()
        code, document = self.run_checker()
        found = self.new_code(document)
        self.assertEqual(code, 1)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0]["level"], "ERROR")
        self.assertIn("late-check", found[0]["message"])
        self.assertIn("billing-seam", found[0]["message"])

    def test_a_missing_debt_record_fails_the_run_and_claims_nothing(self):
        # Fail-closed: the record was never read, so no repository may be
        # reported as debt-free on its strength. The run fails, loudly, by name.
        (self.gov / "gate-debt.json").unlink(missing_ok=True)
        code, document = self.run_checker()
        self.assertEqual(code, 1)
        messages = " ".join(item["message"] for item in self.new_code(document))
        self.assertIn("missing gate debt record", messages)
        self.assertNotIn("billing", messages)

    def test_a_corrupt_ratchet_fails_the_run(self):
        (self.gov / GATE_DEBT_BASELINE_FILE).write_text("{not json", encoding="utf-8")
        code, document = self.run_checker()
        self.assertEqual(code, 1)
        self.assertIn("unreadable gate debt baseline", " ".join(i["message"] for i in self.new_code(document)))

    def test_a_typo_in_the_exception_record_honours_no_exception(self):
        (self.gov / GATE_DEBT_EXCEPTIONS_FILE).write_text(
            json.dumps({"exceptions": [{"project": "actoria", "change": "billing-seam", "reasn": "typo"}]}),
            encoding="utf-8",
        )
        (self.project / "openspec" / "changes" / "billing-seam").mkdir(parents=True)
        code, document = self.run_checker()
        self.assertEqual(code, 1)
        messages = " ".join(item["message"] for item in self.new_code(document))
        self.assertIn("unknown key(s): reasn", messages)

    def test_a_recorded_exception_keeps_the_run_green_and_stays_visible(self):
        (self.project / "openspec" / "changes" / "billing-seam").mkdir(parents=True)
        (self.gov / GATE_DEBT_EXCEPTIONS_FILE).write_text(
            json.dumps(
                {
                    "exceptions": [
                        {"project": "actoria", "change": "billing-seam", "reason": "owner asked for it"}
                    ]
                }
            ),
            encoding="utf-8",
        )
        code, document = self.run_checker()
        found = self.new_code(document)
        self.assertEqual([item["level"] for item in found], ["WARNING"])
        self.assertIn("owner asked for it", found[0]["message"])
        self.assertEqual(code, 0, document)

    def test_the_checker_never_writes_the_ratchet(self):
        # It reads, it does not write. A checker that regenerated the ratchet on
        # every run would make the exit unreviewable and unreviewed.
        digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()  # noqa: E731
        before = {
            name: digest(self.gov / name)
            for name in ("gate-debt.json", GATE_DEBT_BASELINE_FILE, GATE_DEBT_EXCEPTIONS_FILE)
        }
        before[".ai-gate/gate.yaml"] = digest(self.project / ".ai-gate" / "gate.yaml")
        (self.project / "openspec" / "changes" / "billing-seam").mkdir(parents=True)
        self.run_checker()
        after = {
            name: digest(self.gov / name)
            for name in ("gate-debt.json", GATE_DEBT_BASELINE_FILE, GATE_DEBT_EXCEPTIONS_FILE)
        }
        after[".ai-gate/gate.yaml"] = digest(self.project / ".ai-gate" / "gate.yaml")
        self.assertEqual(after, before)


def gate_binary() -> Path | None:
    """The repository build of the gate, or None when it is not built.

    `PATH` is deliberately not consulted. The `driftwatchdog` on `PATH` is a
    stale 2026-10-01 build that reports 0 for the whole fleet, and an older
    build cannot answer a question about a code added today.
    """
    for candidate in (
        REPO_ROOT.parent / "driftwatchdog" / "target" / "debug" / "driftwatchdog",
        REPO_ROOT.parent / "driftwatchdog" / "target" / "release" / "driftwatchdog",
    ):
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    return None


class GateAcceptanceTests(unittest.TestCase):
    """The real gate must accept the code; an undeclared one reports PASS.

    This is the whole point of adding the code to
    `GOVERNANCE_QUALITY_DECLARED_CODES`, measured rather than argued: the
    checker emits the finding and exits 1, and the concern either reports it or
    silently drops it. Observed on this workspace before the declaration landed:
    `0 of 93 governance findings matched this concern's scope`, status PASS.
    """

    @classmethod
    def setUpClass(cls):
        cls.gate = gate_binary()
        if cls.gate is None:
            raise unittest.SkipTest(
                "the driftwatchdog gate binary is not built; run `cargo build` in driftwatchdog/ "
                "to prove the concern accepts the new code"
            )
        cls._tmp = tempfile.TemporaryDirectory()
        root = Path(cls._tmp.name)
        cls.gov = root / "governance"
        (cls.gov / "scripts").mkdir(parents=True)
        for child in (REPO_ROOT / "scripts").iterdir():
            if child.name == "__pycache__":
                continue
            target = cls.gov / "scripts" / child.name
            shutil.copytree(child, target) if child.is_dir() else shutil.copy2(child, target)
        for name in (
            "vocabulary.json", "capabilities.json", "rules-drift-baseline.json",
            "gate-debt.json", GATE_DEBT_BASELINE_FILE, GATE_DEBT_EXCEPTIONS_FILE,
        ):
            shutil.copy2(REPO_ROOT / name, cls.gov / name)
        (cls.gov / "gate-debt.json").write_text(
            json.dumps(debt_record({"actoria": ["accessibility", "privacy"]}), indent=2),
            encoding="utf-8",
        )
        (cls.gov / GATE_DEBT_BASELINE_FILE).write_text(
            json.dumps({"schema_version": 1, "generator": "tests", "findings": [
                {"project": "actoria", "change": "post-mvp-readiness"}
            ]}),
            encoding="utf-8",
        )
        shutil.copytree(REPO_ROOT / "profiles", cls.gov / "profiles")
        registry = json.loads((REPO_ROOT / "projects.json").read_text(encoding="utf-8"))
        registry["projects"] = [{
            "id": "actoria", "path": "actoria", "profile": "inventory",
            "lifecycle": "inventory", "adoption": "adopted",
        }]
        (cls.gov / "projects.json").write_text(json.dumps(registry, indent=2), encoding="utf-8")
        cls.workspace = root / "workspace"
        cls.repo = cls.workspace / "actoria"
        # `post-mvp-readiness` is the grandfathered pair, `close-unbound-gate-checks`
        # is exempt, and `billing-seam` is new work: exactly one trigger.
        for change in ("post-mvp-readiness", "close-unbound-gate-checks", "billing-seam"):
            (cls.repo / "openspec" / "changes" / change).mkdir(parents=True)
        (cls.repo / ".project.json").write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "id": "actoria",
                    "kind": "product",
                    "role": "product",
                    "stack": "none",
                    "profile": "inventory",
                    "lifecycle": "inventory",
                    "verification": {"command": "cargo test"},
                    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
                }
            ),
            encoding="utf-8",
        )
        (cls.repo / ".ai-gate").mkdir()
        (cls.repo / ".ai-gate" / "gate.yaml").write_text(
            "version: 1\n"
            "runtime: driftwatchdog\n"
            "profile: minimal\n"
            "checks:\n"
            "  governance-quality: true\n"
            "blocking:\n"
            "  - FAIL\n"
            "commands:\n"
            f"  governance-quality: 'python3 {cls.gov / 'scripts' / 'workspace_check.py'} "
            f"--root {cls.workspace} --project actoria --json'\n",
            encoding="utf-8",
        )
        subprocess.run(["git", "init", "-q"], cwd=cls.repo, check=True, capture_output=True)
        subprocess.run([str(cls.gate), "init"], cwd=cls.repo, check=True, capture_output=True)
        cls.completed = subprocess.run(
            [str(cls.gate), "gate", "--format", "json"], cwd=cls.repo, capture_output=True, text=True, timeout=300
        )
        cls.document = json.loads(cls.completed.stdout)

    @classmethod
    def tearDownClass(cls):
        cls._tmp.cleanup()

    def concern(self) -> dict:
        results = [item for item in self.document["results"] if item["gate_id"] == "governance-quality"]
        self.assertEqual(len(results), 1, self.document)
        return results[0]

    def test_the_checker_itself_emits_the_code_and_exits_non_zero(self):
        # Without this, the gate test below could pass for the wrong reason: a
        # clean fixture would also produce a PASS.
        completed = subprocess.run(
            [sys.executable, str(self.gov / "scripts" / "workspace_check.py"),
             "--root", str(self.workspace), "--project", "actoria", "--json"],
            capture_output=True, text=True, timeout=300, check=False,
        )
        document = json.loads(completed.stdout)
        self.assertEqual(completed.returncode, 1, completed.stdout)
        codes = [item["code"] for item in document["findings"]]
        self.assertIn("QUALITY_NEW_CHANGE_WITH_GATE_DEBT", codes)

    def test_the_concern_reports_the_finding_as_a_failure(self):
        concern = self.concern()
        self.assertEqual(concern["status"], "FAIL", concern)
        self.assertEqual(concern["findings"][0]["rule"], "QUALITY_NEW_CHANGE_WITH_GATE_DEBT")
        self.assertEqual(concern["findings"][0]["severity"], "error")

    def test_the_gate_run_is_blocked(self):
        self.assertEqual(self.document["status"], "FAIL")
        self.assertTrue(self.document["blocked"])
        self.assertEqual(self.completed.returncode, 1)

    def test_the_code_is_declared(self):
        # The declaration is a list in another repository, so it is read from
        # that source rather than restated here: a copy would drift.
        mod_rs = REPO_ROOT.parent / "driftwatchdog" / "src" / "gate" / "governance" / "mod.rs"
        if not mod_rs.is_file():
            self.skipTest(f"driftwatchdog source not present: {mod_rs}")
        self.assertIn('"QUALITY_NEW_CHANGE_WITH_GATE_DEBT"', mod_rs.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
