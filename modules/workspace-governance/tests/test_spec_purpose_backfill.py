"""Tests for the spec Purpose capability: the archived-change marker is a
finding, the backfill resolves it mechanically, and the rule is fenced off the
fleet by one committed constant.

Every fleet-dependent claim is pinned against a staged fixture rather than the
sibling repositories, so the suite is hermetic. The one exception is a single
test asserting the marker pattern in the checker and in the backfill tool are the
same pattern, which is a property of this repository's two files rather than of
the fleet. The real fleet is measured by `scripts/spec_purpose_backfill.py`, and
its state is read by `--check`.
"""

import contextlib
import io
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

REPO_ROOT = Path(__file__).parents[1]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import spec_purpose_backfill
import workspace_check
from spec_purpose_backfill import (
    MIN_WHY_CHARS,
    PLACEHOLDER_RE,
    BackfillError,
    apply_plan,
    build_plan,
    main,
    write_plan,
)
from workspace_check import (
    SPEC_PURPOSE_AUDIT_ENABLED,
    SPEC_PURPOSE_CHANGE_RE,
    SPEC_PURPOSE_PLACEHOLDER,
    SPEC_PURPOSE_PLACEHOLDER_LEVEL,
    SPEC_PURPOSE_PLACEHOLDER_RE,
    run,
    spec_purpose_audit_enabled,
    validate_spec_purposes,
)

CURRENT_MARKER = "TBD - created by archiving change add-retention-policy. Update Purpose after archive."
LEGACY_MARKER = "TBD - created by archiving change add-admin-ip-allowlist."
# Long enough to clear MIN_WHY_CHARS, so the `## Why` is usable rather than a
# fragment. The threshold is a judgment call, and this is its control.
USABLE_WHY = (
    "Records must not outlive the purpose they were written for, so retention is "
    "stated as a policy a reviewer can check rather than left as an accident of "
    "whichever storage tier happened to hold them."
)

DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "dotnet-product",
    "profile": "product",
    "lifecycle": "active",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "declared"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}
ENTRY = {
    "id": "demo-project",
    "path": "demo-project",
    "profile": "product",
    "lifecycle": "active",
    "adoption": "adopted",
    "role": "product",
    "stack": "dotnet",
}


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def spec_text(purpose_body: str, *, requirement: str | None = "purge expired records",
              title: str = "# Retention Specification") -> str:
    """A spec file shaped like the ones the archiver emits."""
    body = f"{title}\n\n## Purpose\n{purpose_body}\n"
    if requirement is not None:
        body += f"\n### Requirement: {requirement}\n"
    return body + "\n## Notes\n\nA second section, so `## Purpose` is not the last one.\n"


def place_spec(project: Path, capability: str, purpose_body: str, **kwargs) -> Path:
    path = project / "openspec" / "specs" / capability / "spec.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(spec_text(purpose_body, **kwargs), encoding="utf-8")
    return path


def place_archive(project: Path, directory: str, why: str | None = None) -> Path:
    archive = project / "openspec" / "changes" / "archive" / directory
    archive.mkdir(parents=True, exist_ok=True)
    if why is not None:
        (archive / "proposal.md").write_text(
            f"# Proposal\n\n## Why\n{why}\n\n## What Changes\n\n- something\n", encoding="utf-8")
    return archive


def make_project(workspace: Path, name: str = "alpha") -> Path:
    project = workspace / name
    project.mkdir(parents=True, exist_ok=True)
    return project


def marker_findings(findings, project_id: str = "demo-project"):
    return [item for item in findings
            if item.code == SPEC_PURPOSE_PLACEHOLDER and item.project == project_id]


# ---------------------------------------------------------------------------
# The pattern
# ---------------------------------------------------------------------------


class MarkerPatternTests(unittest.TestCase):
    """One pattern, two generations, one blank-line tolerance, two files."""

    def test_both_generations_match(self):
        for body in (CURRENT_MARKER, LEGACY_MARKER):
            with self.subTest(marker=body):
                self.assertIsNotNone(PLACEHOLDER_RE.search(spec_text(body)))

    def test_a_blank_line_between_heading_and_marker_still_matches(self):
        # 7 files in the fleet carry this shape. A pattern that missed them would
        # leave a spec that the backfill resolves and the rule still reports.
        self.assertIsNotNone(PLACEHOLDER_RE.search(spec_text(f"\n{CURRENT_MARKER}")))

    def test_real_prose_is_not_a_marker(self):
        prose = f"{CURRENT_MARKER} Yes, this sentence is real prose that happens to start the same way."
        self.assertIsNone(PLACEHOLDER_RE.search(spec_text(prose)))

    def test_an_absent_purpose_is_not_a_marker(self):
        self.assertIsNone(PLACEHOLDER_RE.search("# Retention Specification\n\n## Notes\n\nx\n"))

    def test_the_change_name_is_read_in_both_spellings(self):
        for marker, expected in (
            (CURRENT_MARKER, "add-retention-policy"),
            (LEGACY_MARKER, "add-admin-ip-allowlist"),
            ("TBD - created by archiving change 2026-09-18-work-on-main-baseline-rule.", "2026-09-18-work-on-main-baseline-rule"),
        ):
            with self.subTest(marker=marker):
                match = SPEC_PURPOSE_CHANGE_RE.match(marker)
                self.assertIsNotNone(match)
                self.assertEqual(match.group("name"), expected)

    def test_the_checker_and_the_tool_share_one_pattern(self):
        # workspace_check.py's own comment makes this a MUST: a marker the
        # backfill resolves but the rule cannot see is a spec that migrates and
        # then still reports ERROR.
        self.assertEqual(SPEC_PURPOSE_PLACEHOLDER_RE.pattern, PLACEHOLDER_RE.pattern)
        self.assertEqual(SPEC_PURPOSE_PLACEHOLDER_RE.flags, PLACEHOLDER_RE.flags)


# ---------------------------------------------------------------------------
# The rule
# ---------------------------------------------------------------------------


class SpecPurposeRuleTests(unittest.TestCase):
    """`validate_spec_purposes` against staged spec files."""

    def findings_for(self, project: Path):
        found: list = []
        validate_spec_purposes(project, found, "demo-project")
        return found

    def test_a_surviving_marker_is_an_error_naming_the_file_and_the_change(self):
        workspace = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, workspace, ignore_errors=True)
        project = make_project(workspace)
        place_spec(project, "retention", CURRENT_MARKER)
        found = self.findings_for(project)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0].level, "ERROR")
        self.assertEqual(found[0].code, SPEC_PURPOSE_PLACEHOLDER)
        self.assertIn("openspec/specs/retention/spec.md", found[0].message)
        self.assertIn("add-retention-policy", found[0].message)

    def test_the_legacy_generation_is_the_same_defect(self):
        workspace = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, workspace, ignore_errors=True)
        project = make_project(workspace)
        place_spec(project, "admin", LEGACY_MARKER)
        found = self.findings_for(project)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0].level, "ERROR")

    def test_a_real_purpose_is_never_a_finding(self):
        workspace = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, workspace, ignore_errors=True)
        project = make_project(workspace)
        place_spec(project, "retention",
                   f"{CURRENT_MARKER} The retention specification covers purge expired records, "
                   "which is real prose that happens to begin with the marker.")
        self.assertEqual(self.findings_for(project), [])

    def test_a_repository_with_no_specs_tree_is_skipped(self):
        workspace = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, workspace, ignore_errors=True)
        project = make_project(workspace)
        (project / "openspec" / "changes").mkdir(parents=True)
        self.assertEqual(self.findings_for(project), [])

    def test_every_marker_in_the_tree_is_reported(self):
        workspace = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, workspace, ignore_errors=True)
        project = make_project(workspace)
        place_spec(project, "retention", CURRENT_MARKER)
        place_spec(project, "admin", LEGACY_MARKER)
        place_spec(project, "healthy", "The healthy specification states a real purpose.")
        self.assertEqual(len(self.findings_for(project)), 2)


# ---------------------------------------------------------------------------
# The fence
# ---------------------------------------------------------------------------


class FenceTests(unittest.TestCase):
    """One committed constant, and both of its states, on one corpus."""

    def make_workspace(self, *, with_specs_tree: bool = True, profile: str = "product") -> Path:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        project = root / "demo-project"
        project.mkdir()
        declaration = dict(DECLARATION, profile=profile)
        write_json(project / ".project.json", declaration)
        write_json(root / "projects.json", {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": [dict(ENTRY, profile=profile)],
        })
        if with_specs_tree:
            place_spec(project, "retention", CURRENT_MARKER)
            place_spec(project, "admin", LEGACY_MARKER)
        return root

    def audit(self, root: Path):
        (root / "vocabulary.json").write_text(
            (REPO_ROOT / "vocabulary.json").read_text(encoding="utf-8"), encoding="utf-8")
        for name in ("capabilities.json", "rules-drift-baseline.json"):
            source = REPO_ROOT / name
            if source.is_file():
                shutil.copy2(source, root / name)
        if (REPO_ROOT / "profiles").is_dir():
            shutil.copytree(REPO_ROOT / "profiles", root / "profiles")
        original = workspace_check.SPEC_PURPOSE_AUDIT_ENABLED
        self.addCleanup(setattr, workspace_check, "SPEC_PURPOSE_AUDIT_ENABLED", original)
        return run(SimpleNamespace(registry=str(root / "projects.json"), root=str(root), project=None,
                                   audit=True, self_check=False, as_json=False))

    def set_fence(self, value: bool) -> None:
        original = workspace_check.SPEC_PURPOSE_AUDIT_ENABLED
        workspace_check.SPEC_PURPOSE_AUDIT_ENABLED = value
        self.addCleanup(setattr, workspace_check, "SPEC_PURPOSE_AUDIT_ENABLED", original)

    def test_the_committed_constant_is_true(self):
        # The hold is open. The fleet write has landed (0 markers remain) and
        # the `governance-quality` concern now declares this code
        # (driftwatchdog `6390ab9`, measured PASS/exit 0 -> FAIL/exit 1), so
        # the rule is both wired and satisfied. Severity never softened.
        self.assertTrue(SPEC_PURPOSE_AUDIT_ENABLED)
        self.assertEqual(SPEC_PURPOSE_PLACEHOLDER_LEVEL, "ERROR")

    def test_the_fence_closed_reports_nothing_on_the_unmigrated_corpus(self):
        root = self.make_workspace()
        self.set_fence(False)
        findings, _ = self.audit(root)
        self.assertEqual(marker_findings(findings), [])

    def test_the_fence_open_reports_error_for_every_marker_and_turns_the_gate_red(self):
        # The same corpus, the same check, the only change being the committed
        # constant. This is the property that makes the hold a sequencing gate
        # rather than an opt-out: it cannot pass quietly.
        root = self.make_workspace()
        self.set_fence(True)
        findings, totals = self.audit(root)
        found = marker_findings(findings)
        self.assertEqual(len(found), 2, "every un-backfilled spec must report")
        self.assertTrue(all(item.level == "ERROR" for item in found))
        self.assertGreater(totals["errors"], 0, "the gate must go red")

    def test_the_fence_open_on_a_backfilled_corpus_is_green(self):
        root = self.make_workspace()
        for capability, body in (("retention", "The retention specification states a real purpose."),
                                 ("admin", "The admin specification states a real purpose.")):
            place_spec(root / "demo-project", capability, body)
        self.set_fence(True)
        findings, _ = self.audit(root)
        self.assertEqual(marker_findings(findings), [])

    def test_a_repository_with_no_specs_tree_reports_nothing_even_with_the_fence_open(self):
        root = self.make_workspace(with_specs_tree=False)
        self.set_fence(True)
        findings, _ = self.audit(root)
        self.assertEqual(marker_findings(findings), [])

    def test_the_rule_is_not_gated_on_profile_requirements(self):
        # A profile that does not require OpenSpec still gets the check: a
        # descriptive rule must not become a requirements driver.
        root = self.make_workspace(profile="standalone")
        self.set_fence(True)
        findings, _ = self.audit(root)
        self.assertEqual(len(marker_findings(findings)), 2)

    def test_the_fence_is_a_committed_constant_not_an_environment_variable(self):
        # The point is not the value but the provenance: the resolved value
        # tracks the committed constant whatever the environment says. Setting
        # the variable to the *opposite* of the constant must not flip it.
        opposite = "0" if SPEC_PURPOSE_AUDIT_ENABLED else "1"
        previous = os.environ.get("SPEC_PURPOSE_AUDIT_ENABLED")
        os.environ["SPEC_PURPOSE_AUDIT_ENABLED"] = opposite
        try:
            resolved = spec_purpose_audit_enabled()
            self.assertEqual(resolved, SPEC_PURPOSE_AUDIT_ENABLED)
            self.assertNotEqual(resolved, opposite == "1",
                                "the environment must not be able to set the fence")
        finally:
            if previous is None:
                del os.environ["SPEC_PURPOSE_AUDIT_ENABLED"]
            else:
                os.environ["SPEC_PURPOSE_AUDIT_ENABLED"] = previous

    def test_an_explicit_argument_overrides_the_constant_for_tests_only(self):
        self.assertTrue(spec_purpose_audit_enabled(True))
        self.assertFalse(spec_purpose_audit_enabled(False))


# ---------------------------------------------------------------------------
# The tool
# ---------------------------------------------------------------------------


class BackfillToolTests(unittest.TestCase):
    """`build_plan` / `apply_plan` against staged repositories."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / "workspace"
        self.workspace.mkdir()
        self.governance = Path(self.temp.name) / "governance"
        self.governance.mkdir()

    def build(self, **kwargs):
        return build_plan(self.workspace, self.governance, **kwargs)

    def test_the_archived_why_is_preferred_and_recorded_with_its_evidence(self):
        project = make_project(self.workspace)
        place_spec(project, "retention", CURRENT_MARKER)
        place_archive(project, "2026-09-18-add-retention-policy", USABLE_WHY)
        plan = self.build()
        self.assertEqual(plan["totals"]["specs"], 1)
        row = plan["specs"][0]
        self.assertEqual(row["source"], "archived-change-why")
        self.assertEqual(row["purpose"], " ".join(USABLE_WHY.split()))
        self.assertEqual(row["evidence"],
                         "openspec/changes/archive/2026-09-18-add-retention-policy/proposal.md#why")

    def test_the_marker_holds_the_bare_name_and_the_archive_is_date_prefixed(self):
        # Joining the two directly resolves nothing. Resolution strips the date
        # prefix from the directory and indexes the remainder.
        project = make_project(self.workspace)
        place_spec(project, "retention", CURRENT_MARKER)
        place_archive(project, "2026-09-18-add-retention-policy", USABLE_WHY)
        self.assertEqual(self.build()["specs"][0]["source"], "archived-change-why")

    def test_a_date_prefixed_marker_resolves_to_an_unprefixed_archive(self):
        # The archiver has emitted both spellings, so both sides accept both.
        project = make_project(self.workspace)
        place_spec(project, "baseline",
                   "TBD - created by archiving change 2026-10-03-work-on-main-baseline-rule.")
        place_archive(project, "work-on-main-baseline-rule", USABLE_WHY)
        self.assertEqual(self.build()["specs"][0]["source"], "archived-change-why")

    def test_a_why_below_the_threshold_is_treated_as_absent(self):
        project = make_project(self.workspace)
        place_spec(project, "retention", CURRENT_MARKER)
        place_archive(project, "2026-09-18-add-retention-policy", "too short")
        row = self.build()["specs"][0]
        self.assertEqual(row["source"], "first-requirement")

    def test_the_threshold_is_the_recorded_boundary(self):
        # Exactly at the boundary is usable; one character under is not. The two
        # specs name different changes so each resolves its own archive.
        project = make_project(self.workspace)
        place_spec(project, "at", CURRENT_MARKER)
        place_archive(project, "2026-09-18-add-retention-policy", "y" * MIN_WHY_CHARS)
        place_spec(project, "under", LEGACY_MARKER)
        place_archive(project, "2026-09-18-add-admin-ip-allowlist", "y" * (MIN_WHY_CHARS - 1))
        sources = {row["spec"].split("/")[-2]: row["source"] for row in self.build()["specs"]}
        self.assertEqual(sources["at"], "archived-change-why")
        self.assertEqual(sources["under"], "first-requirement")

    def test_the_requirement_fallback_reproduces_the_prior_arts_exact_string(self):
        project = make_project(self.workspace)
        place_spec(project, "retention", CURRENT_MARKER, requirement="Purge expired records.")
        row = self.build()["specs"][0]
        self.assertEqual(row["source"], "first-requirement")
        # Preserved byte-for-byte from openlearning/scripts/fill_spec_purposes.py:
        # the capability name verbatim from the `# ... Specification` title, the
        # requirement topic with its trailing `.` stripped and first letter
        # lowercased.
        self.assertEqual(
            row["purpose"],
            "The Retention specification covers purge expired records and the related "
            "scenarios that govern this capability across the platform.",
        )

    def test_a_spec_with_no_requirement_falls_back_to_the_capability_name(self):
        project = make_project(self.workspace)
        place_spec(project, "retention", CURRENT_MARKER, requirement=None)
        row = self.build()["specs"][0]
        self.assertEqual(row["source"], "capability-name")
        self.assertEqual(
            row["purpose"],
            "The Retention specification defines the behaviors, contracts, and "
            "scenarios that govern this capability across the platform.",
        )

    def test_a_repository_with_no_specs_tree_contributes_nothing(self):
        (self.workspace / "beta").mkdir()
        self.assertEqual(self.build()["totals"]["projects_scanned"], 0)

    def test_the_plan_totals_name_the_policy(self):
        project = make_project(self.workspace)
        place_spec(project, "retention", CURRENT_MARKER)
        place_archive(project, "2026-09-18-add-retention-policy", USABLE_WHY)
        plan = self.build()
        self.assertEqual(plan["policy"]["min_why_chars"], MIN_WHY_CHARS)
        self.assertEqual(plan["totals"]["by_source"], {"archived-change-why": 1})
        self.assertEqual(plan["change"], "spec-purpose-backfill-gate")

    def test_an_empty_plan_is_a_valid_state_not_an_error(self):
        # This is what a dry run sees on a backfilled fleet. Raising here would
        # make "is the backfill applied?" the one question the tool cannot answer.
        self.assertEqual(self.build()["totals"]["specs"], 0)


# ---------------------------------------------------------------------------
# Refusal instead of a guess
# ---------------------------------------------------------------------------


class AmbiguousArchiveTests(unittest.TestCase):
    """Two changes under one name is a refusal, and it is row-level."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / "workspace"
        self.workspace.mkdir()
        self.governance = Path(self.temp.name) / "governance"
        self.governance.mkdir()

    def test_the_ambiguous_row_is_planned_with_no_sentence_and_both_directories(self):
        project = make_project(self.workspace)
        place_spec(project, "toolkit", "TBD - created by archiving change platform-testing-toolkit.")
        place_archive(project, "2026-09-08-platform-testing-toolkit", USABLE_WHY)
        place_archive(project, "2026-09-10-platform-testing-toolkit", USABLE_WHY + " Second version.")
        row = self.build()["specs"][0]
        self.assertEqual(row["source"], "ambiguous-archive")
        self.assertIsNone(row["purpose"])
        self.assertTrue(row["requires_owner_decision"])
        self.assertIn("2026-09-08-platform-testing-toolkit", row["evidence"])
        self.assertIn("2026-09-10-platform-testing-toolkit", row["evidence"])

    def test_the_refusal_is_row_level_so_the_rest_stays_reviewable(self):
        # Refusing the whole plan would make the tool unable to produce a
        # reviewable plan at all: 1 other row here, 1,233 on the fleet.
        ambiguous = make_project(self.workspace, "alpha")
        place_spec(ambiguous, "toolkit", "TBD - created by archiving change platform-testing-toolkit.")
        place_archive(ambiguous, "2026-09-08-platform-testing-toolkit", USABLE_WHY)
        place_archive(ambiguous, "2026-09-10-platform-testing-toolkit", USABLE_WHY)
        clean = make_project(self.workspace, "beta")
        place_spec(clean, "retention", CURRENT_MARKER)
        place_archive(clean, "2026-09-18-add-retention-policy", USABLE_WHY)
        plan = self.build()
        self.assertEqual(plan["totals"]["specs"], 2)
        self.assertEqual(plan["totals"]["requires_owner_decision"], 1)
        self.assertEqual(plan["totals"]["resolvable"], 1)
        resolved = [row for row in plan["specs"] if not row.get("requires_owner_decision")]
        self.assertEqual(len(resolved), 1)
        self.assertEqual(resolved[0]["source"], "archived-change-why")

    def test_apply_is_all_or_nothing(self):
        # A half-applied fleet is the one state the closed fence cannot see.
        ambiguous = make_project(self.workspace, "alpha")
        place_spec(ambiguous, "toolkit", "TBD - created by archiving change platform-testing-toolkit.")
        place_archive(ambiguous, "2026-09-08-platform-testing-toolkit", USABLE_WHY)
        place_archive(ambiguous, "2026-09-10-platform-testing-toolkit", USABLE_WHY)
        clean = make_project(self.workspace, "beta")
        clean_spec = place_spec(clean, "retention", CURRENT_MARKER)
        place_archive(clean, "2026-09-18-add-retention-policy", USABLE_WHY)
        before = clean_spec.read_text(encoding="utf-8")
        with self.assertRaises(BackfillError) as caught:
            apply_plan(self.build(), self.workspace)
        self.assertIn("owner decision", str(caught.exception))
        self.assertIn("1 row(s) need an owner decision", str(caught.exception))
        self.assertEqual(clean_spec.read_text(encoding="utf-8"), before,
                         "no file may be written while a row is undecided")

    def build(self, **kwargs):
        return build_plan(self.workspace, self.governance, **kwargs)


class AmbiguousAsDerivedTests(unittest.TestCase):
    """The owner opt-in: decline the archives, derive from the spec itself.

    Two archive directories under one change name are two different changes, so
    the tool refuses rather than guess a `## Why`. `--resolve-ambiguous-as-derived`
    is the recorded way out: the Purpose comes from the spec's own first
    requirement, which asserts nothing about which change produced it, and the
    declined candidates stay in `evidence` so the reader can still see them.
    """

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / "workspace"
        self.workspace.mkdir()
        self.governance = Path(self.temp.name) / "governance"
        self.governance.mkdir()
        self.project = make_project(self.workspace, "alpha")
        self.spec = place_spec(
            self.project, "toolkit", "TBD - created by archiving change platform-testing-toolkit."
        )
        place_archive(self.project, "2026-09-08-platform-testing-toolkit", USABLE_WHY)
        place_archive(self.project, "2026-09-10-platform-testing-toolkit", USABLE_WHY + " Second.")

    def build(self, **kwargs):
        return build_plan(self.workspace, self.governance, **kwargs)

    def test_the_opt_in_resolves_the_row_and_records_the_declined_archives(self):
        plan = self.build(resolve_ambiguous_as_derived=True)
        self.assertEqual(plan["totals"]["requires_owner_decision"], 0)
        row = plan["specs"][0]
        self.assertNotIn("requires_owner_decision", row)
        self.assertTrue(row["purpose"])
        self.assertIn("declined-ambiguous-archive", row["evidence"])
        self.assertIn("2026-09-08-platform-testing-toolkit", row["evidence"])
        self.assertIn("2026-09-10-platform-testing-toolkit", row["evidence"])
        self.assertNotIn(USABLE_WHY[:40], row["purpose"],
                         "the sentence must not come from either declined archive")

    def test_the_opt_in_writes_a_purpose(self):
        apply_plan(self.build(resolve_ambiguous_as_derived=True), self.workspace)
        text = self.spec.read_text(encoding="utf-8")
        self.assertNotIn("TBD - created by archiving change", text)
        self.assertIn("## Purpose", text)


class BackslashInArchivedProseTests(unittest.TestCase):
    """Archived `## Why` prose is inserted literally.

    A `re.sub` *string* replacement treats a backslash in the replacement as an
    escape, so a spec whose archived rationale contains one (a regex, a Windows
    path, an escaped quote) raised `re.error: bad escape` part-way through the
    fleet and left the write half applied. The replacement is a function.
    """

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / "workspace"
        self.workspace.mkdir()
        self.governance = Path(self.temp.name) / "governance"
        self.governance.mkdir()
        self.project = make_project(self.workspace, "alpha")
        self.spec = place_spec(self.project, "retention", CURRENT_MARKER)
        self.why = (
            "Audit rows are written with a pattern that matches \\s+ and a literal "
            "backslash in the payload, and the reference implementation uses "
            "C:\\temp\\audit as its scratch path. " + "Filler prose to clear the "
            "minimum length floor for a usable archived rationale. " * 2
        )
        place_archive(self.project, "2026-09-18-add-retention-policy", self.why)

    def test_the_purpose_is_written_literally_and_the_marker_is_gone(self):
        plan = build_plan(self.workspace, self.governance)
        self.assertEqual(plan["totals"]["requires_owner_decision"], 0)
        self.assertIn("\\s+", plan["specs"][0]["purpose"])
        apply_plan(plan, self.workspace)
        text = self.spec.read_text(encoding="utf-8")
        self.assertNotIn("TBD - created by archiving change", text)
        self.assertIn("\\s+", text, "a backslash in archived prose must survive the write verbatim")
        self.assertIn("C:\\temp\\audit", text)


# ---------------------------------------------------------------------------
# The safety contract
# ---------------------------------------------------------------------------


class SafetyContractTests(unittest.TestCase):
    """Dry run writes nothing; `--apply` is the only write."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / "workspace"
        self.workspace.mkdir()
        self.governance = Path(self.temp.name) / "governance"
        self.governance.mkdir()
        self.project = make_project(self.workspace)
        self.spec = place_spec(self.project, "retention", CURRENT_MARKER)
        place_archive(self.project, "2026-09-18-add-retention-policy", USABLE_WHY)

    def snapshot(self) -> dict[str, str]:
        return {path.relative_to(self.workspace).as_posix(): path.read_text(encoding="utf-8")
                for path in sorted(self.workspace.rglob("*")) if path.is_file()}

    def call(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = main(["--root", str(self.workspace), "--governance-root", str(self.governance), *argv])
        return code, out.getvalue(), err.getvalue()

    def test_a_dry_run_writes_nothing_at_all(self):
        before = self.snapshot()
        code, out, err = self.call()
        self.assertEqual(code, 0)
        self.assertIn("retention/spec.md", out, "the plan goes to stdout")
        self.assertIn("Re-run with --apply under explicit authorization.", err)
        self.assertEqual(self.snapshot(), before)

    def test_the_dry_run_lists_every_placeholder_with_its_source(self):
        _, out, _ = self.call()
        self.assertIn("source=archived-change-why", out)
        self.assertIn(USABLE_WHY.split()[0], out)

    def test_json_output_is_the_same_plan(self):
        _, out, _ = self.call("--json")
        plan = json.loads(out)
        self.assertEqual(plan["totals"]["specs"], 1)
        self.assertEqual(plan["specs"][0]["source"], "archived-change-why")

    def test_apply_writes_and_names_the_flag_and_the_dependency(self):
        code, _, err = self.call("--apply")
        self.assertEqual(code, 0)
        self.assertIn(" ".join(USABLE_WHY.split()), self.spec.read_text(encoding="utf-8"))
        self.assertNotIn("TBD - created by archiving change", self.spec.read_text(encoding="utf-8"))
        self.assertIn("SPEC_PURPOSE_AUDIT_ENABLED = True", err)
        self.assertIn("driftwatchdog", err)
        self.assertIn("applied 1 writes", err)

    def test_apply_is_idempotent_and_never_re_derives_an_edited_purpose(self):
        self.call("--apply")
        self.spec.write_text(self.spec.read_text(encoding="utf-8").replace(
            " ".join(USABLE_WHY.split()), "A purpose a human wrote by hand."), encoding="utf-8")
        code, _, err = self.call("--apply")
        self.assertEqual(code, 0)
        self.assertIn("applied 0 writes", err)
        self.assertIn("A purpose a human wrote by hand.", self.spec.read_text(encoding="utf-8"))

    def test_apply_replaces_the_section_and_leaves_the_rest_of_the_file_alone(self):
        self.call("--apply")
        text = self.spec.read_text(encoding="utf-8")
        self.assertTrue(text.startswith("# Retention Specification\n\n## Purpose\n\n"))
        self.assertIn("### Requirement: purge expired records", text)
        self.assertIn("A second section", text)
        self.assertNotIn("TBD", text)

    def test_write_plan_creates_the_record_without_writing_the_fleet(self):
        before = self.snapshot()
        code, _, _ = self.call("--write-plan")
        self.assertEqual(code, 0)
        self.assertEqual(self.snapshot(), before)
        plan = json.loads((self.governance / "migration" / "spec-purpose-backfill-gate.json")
                          .read_text(encoding="utf-8"))
        self.assertEqual(plan["totals"]["specs"], 1)
        self.assertFalse(plan["status"]["applied"])


# ---------------------------------------------------------------------------
# `--check` in all three states
# ---------------------------------------------------------------------------


class CheckStateTests(unittest.TestCase):
    """Not applied, applied, and re-applied must all be answerable.

    The drift refusal is a plan-time guard. A backfilled fleet is exactly the
    state in which every planned marker is legitimately absent, so `--check`
    turns the refusal off: reporting that as drift would make exit 3 the only
    answer this tool could ever give.
    """

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / "workspace"
        self.workspace.mkdir()
        self.governance = Path(self.temp.name) / "governance"
        self.governance.mkdir()
        self.project = make_project(self.workspace)
        self.spec = place_spec(self.project, "retention", CURRENT_MARKER)
        place_archive(self.project, "2026-09-18-add-retention-policy", USABLE_WHY)

    def call(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = main(["--root", str(self.workspace), "--governance-root", str(self.governance), *argv])
        return code, out.getvalue(), err.getvalue()

    def test_not_applied_exits_three_and_names_the_outstanding_row(self):
        code, _, err = self.call("--check")
        self.assertEqual(code, 3)
        self.assertIn("backfill not applied: 1 row(s) outstanding", err)
        self.assertIn("retention/spec.md", err)
        self.assertIn("SPEC_PURPOSE_AUDIT_ENABLED must stay False", err)

    def test_applied_exits_zero(self):
        self.call("--apply")
        code, out, _ = self.call("--check")
        self.assertEqual(code, 0)
        self.assertIn("backfill applied", out)

    def test_re_applied_is_idempotent_and_still_exits_zero(self):
        self.call("--apply")
        self.call("--apply")
        code, out, _ = self.call("--check")
        self.assertEqual(code, 0)
        self.assertIn("backfill applied", out)

    def test_check_is_not_refused_for_drift_once_a_plan_is_committed(self):
        # The bug this pins: with a committed plan, a fully applied fleet is
        # indistinguishable from a fleet that moved under the plan, so the
        # refusal fired and `--check` could never report "applied".
        self.call("--write-plan")
        self.call("--apply")
        code, out, _ = self.call("--check")
        self.assertEqual(code, 0)
        self.assertIn("backfill applied", out)


# ---------------------------------------------------------------------------
# The drift refusal
# ---------------------------------------------------------------------------


class DriftRefusalTests(unittest.TestCase):
    """`_assert_matches_committed_plan`, exercised against a fixture.

    The generated plan is not committed (design.md §5), so this mechanism is
    inert against the live fleet. An inert mechanism tested only against the
    fleet would be an untested mechanism, so it is pinned here instead.
    """

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / "workspace"
        self.workspace.mkdir()
        self.governance = Path(self.temp.name) / "governance"
        self.governance.mkdir()
        self.project = make_project(self.workspace)
        self.spec = place_spec(self.project, "retention", CURRENT_MARKER)
        place_archive(self.project, "2026-09-18-add-retention-policy", USABLE_WHY)
        self.plan_path = self.governance / "migration" / "spec-purpose-backfill-gate.json"

    def commit_plan(self, plan: dict) -> None:
        write_plan(plan, self.governance)

    def test_an_absent_plan_file_leaves_the_refusal_inert(self):
        plan = build_plan(self.workspace, self.governance)
        self.assertEqual(plan["totals"]["specs"], 1)
        self.assertFalse(self.plan_path.exists())

    def test_a_matching_plan_does_not_refuse(self):
        self.commit_plan(build_plan(self.workspace, self.governance, verify_committed=False))
        self.assertEqual(build_plan(self.workspace, self.governance)["totals"]["specs"], 1)

    def test_a_planned_marker_that_is_gone_is_drift(self):
        self.commit_plan(build_plan(self.workspace, self.governance, verify_committed=False))
        self.spec.write_text(spec_text("A purpose a human wrote by hand."), encoding="utf-8")
        with self.assertRaises(BackfillError) as caught:
            build_plan(self.workspace, self.governance)
        self.assertIn("no longer present", str(caught.exception))

    def test_a_marker_naming_a_different_change_is_drift(self):
        self.commit_plan(build_plan(self.workspace, self.governance, verify_committed=False))
        self.spec.write_text(
            spec_text("TBD - created by archiving change some-other-change. Update Purpose after archive."),
            encoding="utf-8")
        with self.assertRaises(BackfillError) as caught:
            build_plan(self.workspace, self.governance)
        self.assertIn("plan records", str(caught.exception))

    def test_a_marker_the_plan_never_recorded_is_drift(self):
        extra = place_spec(self.project, "admin", LEGACY_MARKER)
        self.commit_plan(build_plan(self.workspace, self.governance, verify_committed=False))
        extra.unlink(missing_ok=True)
        place_spec(self.project, "admin", LEGACY_MARKER)
        other = make_project(self.workspace, "beta")
        place_spec(other, "billing", LEGACY_MARKER)
        with self.assertRaises(BackfillError) as caught:
            build_plan(self.workspace, self.governance)
        self.assertIn("not covered by the committed plan", str(caught.exception))

    def test_apply_refuses_on_drift_rather_than_writing_on_top_of_it(self):
        self.commit_plan(build_plan(self.workspace, self.governance, verify_committed=False))
        self.spec.write_text(spec_text("A purpose a human wrote by hand."), encoding="utf-8")
        code, _, err = self.call()
        self.assertEqual(code, 1)
        self.assertIn("refused:", err)
        self.assertIn("drifted", err)

    def call(self) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = main(["--root", str(self.workspace), "--governance-root", str(self.governance)])
        return code, out.getvalue(), err.getvalue()


# ---------------------------------------------------------------------------
# The fence is the only hold
# ---------------------------------------------------------------------------


class FenceIsTheOnlyMechanismTests(unittest.TestCase):
    """No second mechanism exists to soften, bypass, or exempt the rule."""

    def source(self) -> str:
        return (REPO_ROOT / "scripts" / "workspace_check.py").read_text(encoding="utf-8")

    def test_the_level_is_error_and_never_softened_by_a_constant(self):
        self.assertEqual(SPEC_PURPOSE_PLACEHOLDER_LEVEL, "ERROR")

    def test_no_environment_variable_can_reach_the_fence(self):
        text = self.source()
        # One committed assignment, read only through the resolver. The
        # literal is whatever the fence currently holds, so this test tracks
        # the constant instead of pinning the value it had when written.
        self.assertEqual(text.count("SPEC_PURPOSE_AUDIT_ENABLED = "), 1)
        self.assertIn("return SPEC_PURPOSE_AUDIT_ENABLED if explicit is None else explicit", text)
        anchor = text.index(f"SPEC_PURPOSE_AUDIT_ENABLED = {SPEC_PURPOSE_AUDIT_ENABLED}")
        window = text[anchor:anchor + 400].replace("os.environ.get", "ENV_SENTINEL_ABSENT")
        self.assertNotIn("os.environ", window)

    def test_the_tool_and_the_checker_agree_on_the_fence_name(self):
        text = (REPO_ROOT / "scripts" / "spec_purpose_backfill.py").read_text(encoding="utf-8")
        self.assertIn("SPEC_PURPOSE_AUDIT_ENABLED = True", text)
        self.assertIn("SPEC_PURPOSE_PLACEHOLDER", text)
        self.assertIn("design.md §6", text)


if __name__ == "__main__":
    unittest.main()
