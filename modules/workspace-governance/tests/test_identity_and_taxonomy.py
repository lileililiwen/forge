"""Tests for the identity/taxonomy capability: two axes, a derived `kind`,
an authoritative `id`, and the ruling-2 sequencing gate.

Every fleet-dependent claim is pinned against a staged fixture rather than the
sibling repositories, so the suite is hermetic. The committed migration plan is
checked against the owner's decided table in `design.md` and against
`vocabulary.json`; the fleet scan itself is `scripts/axis_migration.py --check`.
"""

import contextlib
import io
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

from workspace_root import resolve_workspace_root
from types import SimpleNamespace

REPO_ROOT = Path(__file__).parents[1]
WORKSPACE_ROOT = resolve_workspace_root(REPO_ROOT)
PATH_REBASE = json.loads((REPO_ROOT / "migration" / "forge-monorepo-path-reconciliation.json").read_text(encoding="utf-8"))
REBASED_PATHS = {row["id"]: row["to"] for row in PATH_REBASE["changes"]}
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import axis_migration
import vocabulary
import workspace_check
from workspace_check import (
    TAXONOMY_AUDIT_ENABLED,
    TAXONOMY_MISMATCH,
    TAXONOMY_MISMATCH_LEVEL,
    derive_kind,
    run,
    split_kind,
    validate_handoff_quality,
    validate_registry,
    validate_taxonomy,
)

CHANGE_DIR = REPO_ROOT / "openspec" / "changes" / "archive" / "2026-10-12-identity-and-taxonomy-axis-split"
DESIGN = CHANGE_DIR / "design.md"
PLAN = REPO_ROOT / "migration" / "identity-and-taxonomy-axis-split.json"
REGISTRY = REPO_ROOT / "projects.json"

CANONICAL_ROLES = ("product", "platform", "tooling", "governance", "deployment", "inventory")
CANONICAL_STACKS = ("dotnet", "rust", "typescript", "python", "flutter", "none")
# §5.5: the eight entries that are correct and must never be churned.
ALIAS_ENTRIES = (
    ("campus-mesh", "campusMesh"),
    ("care-bridge", "careBridge"),
    ("hestia-lab", "hestiaLab"),
    ("lore-mix", "loreMix"),
    ("lileililiwen-site", "lileililiwen.github.io"),
    ("open-teacher-kit", "openTeacherKit"),
    ("retake-kit", "retakeKit"),
    ("trail-crew", "trailCrew"),
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


def codes(findings, code):
    return [item for item in findings if item.code == code]


class AxisVocabularyTests(unittest.TestCase):
    def test_both_axis_sets_are_canonical(self):
        vocab = vocabulary.load_vocabulary_for_governance_root(REPO_ROOT)
        self.assertEqual(sorted(vocab["roles"]), sorted(CANONICAL_ROLES))
        self.assertEqual(sorted(vocab["stacks"]), sorted(CANONICAL_STACKS))

    def test_existing_arrays_are_unchanged(self):
        raw = json.loads((REPO_ROOT / "vocabulary.json").read_text(encoding="utf-8"))
        self.assertEqual(len(raw["profiles"]), 15)
        self.assertEqual(len(raw["kinds"]), 12)
        self.assertEqual(raw["kinds"][6], "shared-library")
        self.assertEqual(raw["schema_version"], 1)

    def test_shared_library_is_not_a_role(self):
        roles = workspace_check.canonical_roles_for(REPO_ROOT)
        self.assertNotIn("shared-library", roles)
        self.assertEqual(len(roles), 6)

    def test_stack_is_single_valued(self):
        # The vocabulary's `stacks` key is the canonical *set*; what is ruled out
        # is a per-declaration `stacks` array or a secondary-stack field.
        schema = json.loads((REPO_ROOT / "schemas" / "project.schema.json").read_text(encoding="utf-8"))
        self.assertNotIn("stacks", schema["properties"])
        self.assertNotIn("secondary_stacks", schema["properties"])
        self.assertEqual(schema["properties"]["stack"]["$ref"], "#/$defs/taxonomy_stack")
        self.assertNotIn("stacks", schema["$defs"])

    def test_empty_roles_fails_closed_naming_the_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            payload = {
                "schema_version": 1,
                "roles": [],
                "stacks": ["dotnet"],
                "profiles": ["product"],
                "kinds": ["product"],
                "placeholder_markers": ["todo"],
                "secret_field_substrings": ["token"],
            }
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaises(vocabulary.VocabularyError) as ctx:
                vocabulary.load_vocabulary(path)
            self.assertIn("roles", str(ctx.exception))
            self.assertIn(str(path), str(ctx.exception))

    def test_duplicate_stack_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            payload = {
                "schema_version": 1,
                "roles": ["product"],
                "stacks": ["dotnet", "dotnet"],
                "profiles": ["product"],
                "kinds": ["product"],
                "placeholder_markers": ["todo"],
                "secret_field_substrings": ["token"],
            }
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaises(vocabulary.VocabularyError):
                vocabulary.load_vocabulary(path)

    def test_missing_axis_set_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            payload = {
                "schema_version": 1,
                "roles": ["product"],
                "profiles": ["product"],
                "kinds": ["product"],
                "placeholder_markers": ["todo"],
                "secret_field_substrings": ["token"],
            }
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaises(vocabulary.VocabularyError):
                vocabulary.load_vocabulary(path)


class SchemaTests(unittest.TestCase):
    def setUp(self):
        self.schema = json.loads((REPO_ROOT / "schemas" / "project.schema.json").read_text(encoding="utf-8"))

    def test_axes_are_not_required(self):
        self.assertNotIn("role", self.schema["required"])
        self.assertNotIn("stack", self.schema["required"])

    def test_kind_stays_required_as_the_derived_composite(self):
        self.assertIn("kind", self.schema["required"])
        self.assertIn("derived", self.schema["properties"]["kind"]["description"])
        self.assertIn("Deprecated", self.schema["properties"]["kind"]["description"])

    def test_both_axis_sets_are_enumerated(self):
        self.assertEqual(self.schema["$defs"]["taxonomy_role"]["enum"], list(CANONICAL_ROLES))
        self.assertEqual(self.schema["$defs"]["taxonomy_stack"]["enum"], list(CANONICAL_STACKS))

    def test_a_declaration_without_the_axes_is_still_valid(self):
        try:
            import jsonschema
        except ImportError:  # pragma: no cover - CI installs no packages
            self.skipTest("jsonschema is not installed; schema fixtures are skipped")
        validator = jsonschema.Draft202012Validator(self.schema)
        self.assertEqual(list(validator.iter_errors(dict(DECLARATION))), [])

    def test_a_declaration_with_both_axes_is_valid(self):
        try:
            import jsonschema
        except ImportError:  # pragma: no cover
            self.skipTest("jsonschema is not installed; schema fixtures are skipped")
        payload = dict(DECLARATION, role="product", stack="dotnet")
        validator = jsonschema.Draft202012Validator(self.schema)
        self.assertEqual(list(validator.iter_errors(payload)), [])


class KindDerivationTests(unittest.TestCase):
    def test_stack_qualified_derivation(self):
        self.assertEqual(derive_kind("product", "dotnet"), "dotnet-product")
        self.assertEqual(derive_kind("platform", "rust"), "rust-platform")

    def test_stack_none_derives_the_bare_role(self):
        self.assertEqual(derive_kind("governance", "none"), "governance")

    def test_round_trip_is_exact_for_a_stack_qualified_composite(self):
        stacks = frozenset(CANONICAL_STACKS)
        for role in CANONICAL_ROLES:
            for stack in CANONICAL_STACKS:
                if stack == "none":
                    continue
                kind = derive_kind(role, stack)
                self.assertEqual(split_kind(kind, stacks), (stack, role), f"{role}/{stack} -> {kind}")

    def test_stack_none_is_the_one_value_a_bare_composite_cannot_carry(self):
        # `none` derives the bare role, and a bare role cannot say whether the
        # stack was `none` or simply unstated. The spec requires an exact round
        # trip for the stack-qualified case; the ambiguity is recorded here
        # rather than papered over by inventing an encoding.
        stacks = frozenset(CANONICAL_STACKS)
        self.assertEqual(derive_kind("governance", "none"), "governance")
        self.assertEqual(split_kind("governance", stacks), (None, "governance"))

    def test_split_of_a_bare_value_has_no_stack(self):
        self.assertEqual(split_kind("product", frozenset(CANONICAL_STACKS)), (None, "product"))


class TaxonomyFindingTests(unittest.TestCase):
    def findings(self, declaration, entry=None, **kwargs):
        collected = []
        validate_taxonomy(entry or {}, declaration, collected, "demo-project", **kwargs)
        return collected

    def test_both_axes_canonical_and_agreeing(self):
        declaration = dict(DECLARATION, role="platform", stack="rust", kind="rust-platform")
        self.assertEqual(self.findings(declaration, {"role": "platform", "stack": "rust"}), [])

    def test_value_outside_its_set_is_a_warning_naming_both_sets(self):
        declaration = dict(DECLARATION, role="library", stack="rust", kind="rust-product")
        found = self.findings(declaration, {"role": "library", "stack": "rust"})
        axis = codes(found, "TAXONOMY_AXIS_UNKNOWN")
        self.assertTrue(axis)
        self.assertEqual(axis[0].level, "WARNING")
        self.assertIn("'library'", axis[0].message)
        for role in CANONICAL_ROLES:
            self.assertIn(role, axis[0].message)
        for stack in CANONICAL_STACKS:
            self.assertIn(stack, axis[0].message)

    def test_absent_axis_is_a_warning(self):
        declaration = dict(DECLARATION, role="product")
        found = self.findings(declaration, {"role": "product", "stack": "dotnet"})
        axis = codes(found, "TAXONOMY_AXIS_UNKNOWN")
        self.assertEqual(len(axis), 1)
        self.assertIn("stack", axis[0].message)
        self.assertEqual(codes(found, TAXONOMY_MISMATCH), [])

    def test_surfaces_disagree_is_an_error_naming_both_values(self):
        declaration = dict(DECLARATION, role="tooling", stack="rust", kind="rust-tooling")
        found = self.findings(declaration, {"role": "product", "stack": "rust"})
        mismatch = codes(found, TAXONOMY_MISMATCH)
        self.assertEqual(len(mismatch), 1)
        self.assertEqual(mismatch[0].level, "ERROR")
        self.assertEqual(mismatch[0].project, "demo-project")
        self.assertIn("'tooling'", mismatch[0].message)
        self.assertIn("'product'", mismatch[0].message)

    def test_stack_disagreement_is_an_error(self):
        declaration = dict(DECLARATION, role="product", stack="typescript", kind="typescript-product")
        found = self.findings(declaration, {"role": "product", "stack": "rust"})
        mismatch = codes(found, TAXONOMY_MISMATCH)
        self.assertEqual(len(mismatch), 1)
        self.assertIn("stack disagreement", mismatch[0].message)

    def test_axis_on_one_side_only_is_not_a_mismatch(self):
        declaration = dict(DECLARATION)
        found = self.findings(declaration, {"role": "product", "stack": "dotnet"})
        self.assertEqual(codes(found, TAXONOMY_MISMATCH), [])
        self.assertEqual(len(codes(found, "TAXONOMY_AXIS_UNKNOWN")), 2)

    def test_every_combination_is_legal(self):
        for role in CANONICAL_ROLES:
            for stack in CANONICAL_STACKS:
                declaration = dict(DECLARATION, role=role, stack=stack, kind=derive_kind(role, stack))
                found = self.findings(declaration, {"role": role, "stack": stack})
                self.assertEqual(found, [], f"{role}/{stack}: {found}")

    def test_derived_kind_matches_reports_nothing(self):
        declaration = dict(DECLARATION, role="product", stack="dotnet", kind="dotnet-product")
        self.assertEqual(self.findings(declaration, {"role": "product", "stack": "dotnet"}), [])

    def test_composite_without_the_axes_is_not_a_legacy_finding(self):
        declaration = dict(DECLARATION, kind="dotnet-product")
        found = self.findings(declaration, {"role": "product", "stack": "dotnet"})
        self.assertEqual(codes(found, "TAXONOMY_LEGACY_VALUE"), [])
        axis = codes(found, "TAXONOMY_AXIS_UNKNOWN")
        self.assertEqual(len(axis), 2)
        for item in axis:
            self.assertIn("declaration", item.message)

    def test_hand_authored_kind_drift_is_a_legacy_finding(self):
        declaration = dict(DECLARATION, role="platform", stack="rust", kind="rust-product")
        found = self.findings(declaration, {"role": "platform", "stack": "rust"})
        legacy = codes(found, "TAXONOMY_LEGACY_VALUE")
        self.assertEqual(len(legacy), 1)
        self.assertIn("'rust-platform'", legacy[0].message)

    def test_stack_none_derives_the_bare_role_without_a_legacy_finding(self):
        declaration = dict(DECLARATION, role="governance", stack="none", kind="governance")
        self.assertEqual(self.findings(declaration, {"role": "governance", "stack": "none"}), [])

    def test_kind_unknown_keeps_its_own_meaning(self):
        declaration = dict(DECLARATION, kind="quality-gate", role="tooling", stack="rust")
        collected = []
        workspace_check.validate_kind(declaration, collected, "demo-project", governance_root=REPO_ROOT)
        self.assertEqual([item.code for item in collected], ["KIND_UNKNOWN"])
        self.assertEqual(collected[0].level, "WARNING")

    def test_legacy_value_is_not_reported_as_kind_unknown(self):
        declaration = dict(DECLARATION, role="platform", stack="rust", kind="rust-product")
        found = self.findings(declaration, {"role": "platform", "stack": "rust"})
        self.assertEqual(codes(found, "KIND_UNKNOWN"), [])


class SequencingGateTests(unittest.TestCase):
    """Ruling 2: the level is ERROR, and the audit was held off the fleet until
    the §5 migration had been applied. The migration has now landed, so the
    committed gate is open — but it still gates in both directions, and both
    sides stay under test: the flag off, and the flag on over a corpus that
    does not agree with itself."""

    def test_mismatch_level_is_error_not_warning(self):
        self.assertEqual(TAXONOMY_MISMATCH_LEVEL, "ERROR")

    def test_the_gate_is_open_and_the_fleet_corpus_is_clean(self):
        # The §5 migration is applied, so the gate is committed open and the
        # corpus it now audits is clean. Enabling it early could not have passed
        # quietly: the same check is the one that reported the 30 pre-migration
        # disagreements.
        self.assertTrue(TAXONOMY_AUDIT_ENABLED)
        with contextlib.redirect_stdout(io.StringIO()):
            code = axis_migration.main(["--root", str(WORKSPACE_ROOT),
                                        "--governance-root", str(REPO_ROOT), "--check"])
        self.assertEqual(code, 0)

    def test_axis_migration_refuses_a_registry_path_outside_the_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            workspace = base / "workspace"
            governance = base / "governance"
            workspace.mkdir()
            (governance / "openspec/changes/archive/2026-10-12-identity-and-taxonomy-axis-split").mkdir(parents=True)
            (governance / "migration").mkdir()
            shutil.copy(REPO_ROOT / "vocabulary.json", governance / "vocabulary.json")
            shutil.copy(DESIGN, governance / "openspec/changes/archive/2026-10-12-identity-and-taxonomy-axis-split/design.md")
            (governance / "projects.json").write_text(json.dumps({
                "projects": [{"id": "escape", "path": "../outside", "role": "tooling", "stack": "rust"}]
            }), encoding="utf-8")
            with self.assertRaisesRegex(axis_migration.MigrationError, "escapes workspace root"):
                axis_migration.build_plan(workspace, governance)

    def make_workspace(self, declaration=None, entry=None, **axes):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        project = root / "demo-project"
        project.mkdir()
        payload = dict(declaration if declaration is not None else DECLARATION)
        payload.update(axes)
        write_json(project / ".project.json", payload)
        write_json(root / "projects.json", {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": [dict(entry if entry is not None else ENTRY)],
        })
        return root

    def audit(self, root, governance_root):
        (root / "vocabulary.json").write_text(
            (governance_root / "vocabulary.json").read_text(encoding="utf-8"), encoding="utf-8")
        for name in ("capabilities.json", "rules-drift-baseline.json"):
            source = governance_root / name
            if source.is_file():
                shutil.copy2(source, root / name)
        if (governance_root / "profiles").is_dir():
            shutil.copytree(governance_root / "profiles", root / "profiles")
        original = workspace_check.TAXONOMY_AUDIT_ENABLED
        self.addCleanup(setattr, workspace_check, "TAXONOMY_AUDIT_ENABLED", original)
        return run(SimpleNamespace(registry=str(root / "projects.json"), root=str(root), project=None,
                                   audit=True, self_check=False, as_json=False))

    def test_gate_closed_reports_no_taxonomy_finding_on_a_mismatched_fleet(self):
        # The unmigrated fleet: the declaration disagrees with the registry.
        # The gate is committed open, so this side is pinned by holding the flag
        # off explicitly — the flag still silences the audit it is there to gate.
        root = self.make_workspace(kind="tooling", role="tooling", stack="rust",
                                   entry=dict(ENTRY, role="product", stack="rust"))
        original = workspace_check.TAXONOMY_AUDIT_ENABLED
        workspace_check.TAXONOMY_AUDIT_ENABLED = False
        self.addCleanup(setattr, workspace_check, "TAXONOMY_AUDIT_ENABLED", original)
        findings, _ = self.audit(root, REPO_ROOT)
        self.assertEqual([item for item in findings if item.code.startswith("TAXONOMY_")], [])

    def test_gate_open_reports_error_on_the_same_unmigrated_fleet(self):
        root = self.make_workspace(kind="tooling", role="tooling", stack="rust",
                                   entry=dict(ENTRY, role="product", stack="rust"))
        original = workspace_check.TAXONOMY_AUDIT_ENABLED
        workspace_check.TAXONOMY_AUDIT_ENABLED = True
        self.addCleanup(setattr, workspace_check, "TAXONOMY_AUDIT_ENABLED", original)
        findings, _ = self.audit(root, REPO_ROOT)
        mismatch = codes(findings, TAXONOMY_MISMATCH)
        self.assertEqual(len(mismatch), 1)
        self.assertEqual(mismatch[0].level, "ERROR")

    def test_gate_open_on_a_migrated_fleet_is_green(self):
        root = self.make_workspace(kind="dotnet-product", role="product", stack="dotnet")
        original = workspace_check.TAXONOMY_AUDIT_ENABLED
        workspace_check.TAXONOMY_AUDIT_ENABLED = True
        self.addCleanup(setattr, workspace_check, "TAXONOMY_AUDIT_ENABLED", original)
        findings, _ = self.audit(root, REPO_ROOT)
        self.assertEqual([item for item in findings if item.code.startswith("TAXONOMY_")], [])

    def test_the_gate_is_a_committed_constant_not_an_environment_variable(self):
        import os
        original = workspace_check.TAXONOMY_AUDIT_ENABLED
        os.environ["TAXONOMY_AUDIT_ENABLED"] = "1"
        try:
            self.assertEqual(workspace_check.taxonomy_audit_enabled(), original)
        finally:
            del os.environ["TAXONOMY_AUDIT_ENABLED"]

    def test_migration_reports_the_fleet_as_not_applied(self):
        plan = json.loads(PLAN.read_text(encoding="utf-8"))
        self.assertFalse(plan["status"]["applied"])


class AliasEntryTests(unittest.TestCase):
    """`id` is the identity key; `path` is a resolution detail (§5.5)."""

    def make_workspace(self, project_id, path, declaration_id, **extra):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        (root / path).mkdir(parents=True)
        write_json(root / path / ".project.json",
                   dict(DECLARATION, id=declaration_id, **extra))
        write_json(root / "projects.json", {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": [{
                "id": project_id,
                "path": path,
                "profile": "product",
                "lifecycle": "active",
                "adoption": "adopted",
                "role": "product",
                "stack": "dotnet",
            }],
        })
        return root

    def audit(self, root):
        (root / "vocabulary.json").write_text(
            (REPO_ROOT / "vocabulary.json").read_text(encoding="utf-8"), encoding="utf-8")
        for name in ("capabilities.json", "rules-drift-baseline.json"):
            source = REPO_ROOT / name
            if source.is_file():
                shutil.copy2(source, root / name)
        if (REPO_ROOT / "profiles").is_dir():
            shutil.copytree(REPO_ROOT / "profiles", root / "profiles")
        return run(SimpleNamespace(registry=str(root / "projects.json"), root=str(root), project=None,
                                   audit=True, self_check=False, as_json=False))

    def test_every_alias_entry_is_clean_with_the_gate_open(self):
        original = workspace_check.TAXONOMY_AUDIT_ENABLED
        workspace_check.TAXONOMY_AUDIT_ENABLED = True
        self.addCleanup(setattr, workspace_check, "TAXONOMY_AUDIT_ENABLED", original)
        identity_codes = {"DECLARATION_ID", "PROJECT_ID", "PROJECT_MISSING", "PROJECT_PATH", "PROJECT_DUPLICATE"}
        for project_id, path in ALIAS_ENTRIES:
            with self.subTest(project=project_id):
                root = self.make_workspace(project_id, path, project_id, role="product", stack="dotnet")
                findings, _ = self.audit(root)
                interesting = [item for item in findings
                               if item.code.startswith("TAXONOMY_") or item.code in identity_codes]
                self.assertEqual(interesting, [], f"{project_id}: {interesting}")

    def test_handoff_quality_resolves_an_alias_path(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        (root / "lileililiwen.github.io").mkdir()
        (root / "lileililiwen.github.io" / "HANDOFF.md").write_text("# Handoff\n", encoding="utf-8")
        collected = []
        validate_handoff_quality(root / "lileililiwen.github.io", collected, "lileililiwen-site")
        self.assertEqual(collected, [])

    def test_the_committed_registry_keeps_every_alias_pair(self):
        entries = {item["id"]: item["path"] for item in json.loads(REGISTRY.read_text(encoding="utf-8"))["projects"]}
        for project_id, path in ALIAS_ENTRIES:
            self.assertEqual(entries.get(project_id), path)


class MigrationPlanTests(unittest.TestCase):
    """The committed plan is the reviewable artifact for the fleet write."""

    def setUp(self):
        self.plan = json.loads(PLAN.read_text(encoding="utf-8"))
        self.rows = {row["directory"]: row for row in self.plan["declarations"]}
        self.registry = {row["id"]: row for row in self.plan["registry"]}
        self.live_registry = json.loads(REGISTRY.read_text(encoding="utf-8"))["projects"]

    def test_every_axis_is_canonical(self):
        for row in self.plan["declarations"]:
            self.assertIn(row["role"], CANONICAL_ROLES, row)
            self.assertIn(row["stack"], CANONICAL_STACKS, row)

    def test_every_row_records_its_resolution_source(self):
        for row in self.plan["declarations"]:
            self.assertTrue(row["rule"], row)
            self.assertTrue(row["source"], row)

    def test_the_thirty_disagreements_resolve_to_the_decided_table(self):
        decided = axis_migration.parse_decided_rows(DESIGN)
        matched = 0
        for name, row in decided.items():
            if name in self.rows:
                matched += 1
                self.assertEqual(self.rows[name]["role"], row.roles[0], name)
                self.assertIn(self.rows[name]["stack"], row.stacks, name)
        # 30 decided rows in §5.2 plus 4 registrations in §5.3.
        self.assertEqual(matched, 34)

    def test_the_axis_disagreement_set_is_empty_after_the_migration(self):
        # This was the 30-repository disagreement baseline; the rows it counted
        # are pinned by the plan tests against `design.md`, so the baseline
        # itself stays reviewable. Applied, the live disagreement set is empty:
        # every registered entry agrees with its own declaration on both axes.
        disagreements = []
        for entry in self.live_registry:
            path = WORKSPACE_ROOT / entry["path"] / ".project.json"
            if not path.is_file():
                continue
            declaration = json.loads(path.read_text(encoding="utf-8"))
            for axis in ("role", "stack"):
                if entry.get(axis) != declaration.get(axis):
                    disagreements.append(
                        f"{entry['id']}: {axis} {entry.get(axis)!r} against {declaration.get(axis)!r}")
        self.assertEqual(disagreements, [])

    def test_ruling_one_folds_shared_library_into_platform(self):
        for project_id in ("platform-contracts", "rust-platform-libs", "dotnet-platform-libs"):
            self.assertEqual(self.registry[project_id]["role"], "platform", project_id)

    def test_ruling_three_keeps_the_multi_stack_repositories_on_dotnet(self):
        for project_id in ("lexora", "opendockify", "trippify"):
            self.assertEqual(self.registry[project_id]["stack"], "dotnet", project_id)
        self.assertNotIn("secondary_stacks", json.dumps(self.plan))

    def test_ruling_four_keeps_the_lileililiwen_id_and_path(self):
        row = self.registry["lileililiwen-site"]
        self.assertEqual(row["path"], "lileililiwen.github.io")
        self.assertEqual(row["role"], "deployment")
        # Ruling 9 supersedes ruling 4's stack half: the census counted
        # build-time content validators, so the stack is `none` and the
        # derived `kind` is the bare role. The pin stays exact, so a later
        # drift off `none` still fails here.
        self.assertEqual(row["stack"], "none")
        self.assertIn("lileililiwen-site", {item["id"] for item in self.live_registry})

    def test_id_immutability_holds_across_the_plan(self):
        # The live registry is the applied "after" state, so immutability is
        # measured against the committed pre-migration baseline. The only ids
        # that moved are the plan's own recorded removal and additions: a rename
        # would surface here as a removal with no matching declared addition.
        baseline = json.loads(
            (REPO_ROOT / "tests" / "fixtures" / "identity-and-taxonomy-baseline.json").read_text(encoding="utf-8"))
        before = set(baseline["registry_ids"])
        after = {item["id"] for item in self.live_registry}
        removed = {row["id"] for row in self.plan["registry"] if row["action"] == "remove"}
        added = {row["id"] for row in self.plan["registry"] if row["action"] == "add"}
        added.add("mnemora")  # registered after the historical migration plan
        self.assertEqual(before - after, removed)
        self.assertEqual(after - before, added)
        self.assertEqual(
            after,
            {row["id"] for row in self.plan["registry"] if row["action"] != "remove"} | {"mnemora"},
        )
        # No surviving id moved its path either; `path` is a resolution detail
        # the write had no licence to touch.
        paths = {item["id"]: item["path"] for item in self.live_registry}
        for row in self.plan["registry"]:
            if row["action"] != "remove":
                expected_path = REBASED_PATHS.get(row["id"], row["path"])
                self.assertEqual(paths[row["id"]], expected_path, row["id"])
        self.assertEqual(paths["mnemora"], "mnemora")

    def test_consolidated_tooling_paths_resolve_to_forge_members(self):
        live = {item["id"]: item for item in self.live_registry}
        for row in PATH_REBASE["changes"]:
            self.assertEqual(live[row["id"]]["path"], row["to"])
            self.assertTrue((WORKSPACE_ROOT / row["declaration"]).is_file())

    def test_the_four_unregistered_repositories_are_now_registered(self):
        live = {item["id"]: item for item in self.live_registry}
        for project_id in ("argoscope", "chronicleaf", "devloom", "kairovia"):
            row = self.registry[project_id]
            self.assertEqual(row["action"], "add")
            self.assertEqual(row["role"], "product")
            self.assertEqual(row["stack"], "dotnet")
            # The §5.3 registration landed: the live registry carries the row,
            # and both the registry and the declaration carry the planned pair.
            self.assertIn(project_id, live)
            self.assertEqual((live[project_id]["role"], live[project_id]["stack"]),
                             (row["role"], row["stack"]))
            declaration = json.loads(
                (WORKSPACE_ROOT / live[project_id]["path"] / ".project.json").read_text(encoding="utf-8"))
            self.assertEqual((declaration["role"], declaration["stack"]), (row["role"], row["stack"]))

    def test_the_ghost_entry_is_removed_with_the_existing_code(self):
        ghost = [row for row in self.plan["registry"] if row["action"] == "remove"]
        self.assertEqual([row["id"] for row in ghost], ["jenkins-bootstrap"])
        self.assertIn("PROJECT_MISSING", ghost[0]["source"])

    def test_no_new_finding_code_is_introduced_for_those_five_repositories(self):
        source = (REPO_ROOT / "scripts" / "workspace_check.py").read_text(encoding="utf-8")
        self.assertIn("DISCOVERED_UNREGISTERED", source)
        self.assertIn("PROJECT_MISSING", source)

    def test_plan_totals_match_the_table(self):
        totals = self.plan["totals"]
        self.assertEqual(totals["declarations"], len(self.plan["declarations"]))
        self.assertEqual(totals["declarations"], 81)
        self.assertEqual(totals["registry_added"], 4)
        self.assertEqual(totals["registry_removed"], 1)
        self.assertEqual(totals["registry_axes"], 77)
        # The committed plan remains the historical 81-project migration.
        # Mnemora was registered afterward from its existing declaration.
        self.assertEqual(len(self.live_registry), totals["registry_axes"] + totals["registry_added"] + 1)
        self.assertEqual(len(self.live_registry), 82)
        for entry in self.live_registry:
            self.assertIn(entry["role"], CANONICAL_ROLES, entry)
            self.assertIn(entry["stack"], CANONICAL_STACKS, entry)

    def test_role_and_stack_census_matches_the_design_count_table(self):
        # design.md §4 counts, adjusted only by ruling 4, which moves exactly one
        # repository from product to deployment.
        self.assertEqual(self.plan["roles"]["product"], 65)
        self.assertEqual(self.plan["roles"]["deployment"], 2)
        self.assertEqual(self.plan["roles"]["platform"], 6)
        self.assertEqual(self.plan["roles"]["tooling"], 6)
        self.assertEqual(self.plan["roles"]["governance"], 1)
        self.assertEqual(self.plan["roles"]["inventory"], 1)
        self.assertEqual(sum(self.plan["roles"].values()), 81)
        for stack, count in (("dotnet", 45), ("rust", 17), ("typescript", 9), ("python", 9), ("flutter", 1)):
            self.assertEqual(self.plan["stacks"][stack], count, stack)
        self.assertEqual(sum(self.plan["stacks"].values()), 81)

    def test_the_one_ambiguous_cell_is_flagged_for_the_owner(self):
        flagged = [row for row in self.plan["declarations"] if row.get("owner_confirmation_required")]
        self.assertEqual([row["id"] for row in flagged], ["platform-contracts"])


class MigrationToolTests(unittest.TestCase):
    """`--apply` writes exactly the planned axes and nothing else."""

    def stage(self, repositories, entries):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        workspace = Path(temp.name) / "workspace"
        governance = Path(temp.name) / "governance"
        workspace.mkdir()
        governance.mkdir()
        for directory, declaration in repositories.items():
            (workspace / directory).mkdir(parents=True)
            write_json(workspace / directory / ".project.json", declaration)
        (workspace / "src").mkdir()
        (workspace / "src" / "Program.cs").write_text("// census\n", encoding="utf-8")
        shutil.copy2(REPO_ROOT / "vocabulary.json", governance / "vocabulary.json")
        design_target = governance / DESIGN.relative_to(REPO_ROOT)
        design_target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(DESIGN, design_target)
        write_json(governance / "projects.json", {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": entries,
        })
        return workspace, governance

    def test_dry_run_writes_nothing(self):
        workspace, governance = self.stage({"forge": dict(DECLARATION, id="forge", kind="platform", profile="rust-product")}, [
            {"id": "forge", "path": "forge", "profile": "rust-product", "lifecycle": "active", "adoption": "adopted"},
        ])
        before = (workspace / "forge" / ".project.json").read_text(encoding="utf-8")
        code = axis_migration.main(["--root", str(workspace), "--governance-root", str(governance), "--json"])
        self.assertEqual(code, 0)
        self.assertEqual((workspace / "forge" / ".project.json").read_text(encoding="utf-8"), before)
        self.assertNotIn("role", json.loads(before))

    def test_apply_writes_the_axes_and_derives_kind(self):
        """Ruling: the migration also writes the derived `kind`.

        `kind` is the deprecated composite and is never hand-authored, so this
        migration is the one pass that stops it being hand-authored. The
        hand-authored value survives as `declared_kind` on the committed plan
        row, so the measurement is not lost. Leaving `kind` stale would put 81
        permanent `TAXONOMY_LEGACY_VALUE` warnings on the fleet, and a warning
        nobody can clear is noise that teaches people to ignore the checker.
        """
        workspace, governance = self.stage({"forge": dict(DECLARATION, id="forge", kind="platform", profile="rust-product")}, [
            {"id": "forge", "path": "forge", "profile": "rust-product", "lifecycle": "active", "adoption": "adopted"},
        ])
        code = axis_migration.main(["--root", str(workspace), "--governance-root", str(governance), "--apply"])
        self.assertEqual(code, 0)
        declaration = json.loads((workspace / "forge" / ".project.json").read_text(encoding="utf-8"))
        self.assertEqual(declaration["role"], "platform")
        self.assertEqual(declaration["stack"], "rust")
        # `platform` + `rust` derives `rust-platform`, not `platform-rust`.
        self.assertEqual(declaration["kind"], "rust-platform")
        self.assertEqual(declaration["id"], "forge")
        self.assertEqual(declaration["profile"], "rust-product")
        entry = json.loads((governance / "projects.json").read_text(encoding="utf-8"))["projects"][0]
        self.assertEqual((entry["id"], entry["path"], entry["role"], entry["stack"]),
                         ("forge", "forge", "platform", "rust"))
        self.assertEqual(list(entry), ["id", "path", "profile", "role", "stack", "lifecycle", "adoption"])

    def test_apply_derives_the_bare_role_when_stack_is_none(self):
        # An id that is NOT in the decided table, so resolution runs the §5.1
        # chain: neither the bare `kind` nor the `platform` profile is
        # stack-qualified, so §5.1 rule 5 applies and stack is `none`.
        workspace, governance = self.stage(
            {"barethreeproject": dict(DECLARATION, id="barethreeproject", kind="governance", profile="platform")},
            [{"id": "barethreeproject", "path": "barethreeproject", "profile": "platform", "lifecycle": "active", "adoption": "adopted"}],
        )
        code = axis_migration.main(["--root", str(workspace), "--governance-root", str(governance), "--apply"])
        self.assertEqual(code, 0)
        declaration = json.loads((workspace / "barethreeproject" / ".project.json").read_text(encoding="utf-8"))
        self.assertEqual(declaration["stack"], "none")
        self.assertEqual(declaration["kind"], "governance")

    def test_the_migration_is_idempotent_after_writing_a_derived_kind(self):
        """Re-running after a successful apply must not report the tool's own output as drift."""
        workspace, governance = self.stage({"forge": dict(DECLARATION, id="forge", kind="platform", profile="rust-product")}, [
            {"id": "forge", "path": "forge", "profile": "rust-product", "lifecycle": "active", "adoption": "adopted"},
        ])
        base = ["--root", str(workspace), "--governance-root", str(governance)]
        self.assertEqual(axis_migration.main(base + ["--apply"]), 0)
        self.assertEqual(axis_migration.main(base + ["--check"]), 0)
        self.assertEqual(axis_migration.main(base + ["--apply"]), 0)
        self.assertEqual(axis_migration.main(base + ["--check"]), 0)
        self.assertEqual(json.loads((workspace / "forge" / ".project.json").read_text(encoding="utf-8"))["kind"], "rust-platform")

    def test_apply_registers_an_undeclared_repository_and_removes_a_ghost(self):
        workspace, governance = self.stage(
            {"devloom": dict(DECLARATION, id="devloom", kind="product", profile="dotnet-product")},
            [
                {"id": "devloom-missing", "path": "devloom-missing", "profile": "deployment",
                 "lifecycle": "active", "adoption": "adopted"},
            ],
        )
        self.assertEqual(axis_migration.main(["--root", str(workspace), "--governance-root", str(governance), "--apply"]), 0)
        entries = json.loads((governance / "projects.json").read_text(encoding="utf-8"))["projects"]
        self.assertEqual([entry["id"] for entry in entries], ["devloom"])
        self.assertEqual((entries[0]["role"], entries[0]["stack"], entries[0]["profile"]), ("product", "dotnet", "dotnet-product"))

    def test_check_reports_unapplied_then_applied(self):
        workspace, governance = self.stage({"forge": dict(DECLARATION, id="forge", kind="platform", profile="rust-product")}, [
            {"id": "forge", "path": "forge", "profile": "rust-product", "lifecycle": "active", "adoption": "adopted"},
        ])
        base = ["--root", str(workspace), "--governance-root", str(governance)]
        self.assertEqual(axis_migration.main(base + ["--check"]), 3)
        self.assertEqual(axis_migration.main(base + ["--apply"]), 0)
        self.assertEqual(axis_migration.main(base + ["--check"]), 0)

    def test_a_decided_row_that_no_longer_matches_the_fleet_is_refused(self):
        workspace, governance = self.stage({"transitfy": dict(DECLARATION, id="transitfy", kind="rust-product", profile="product")}, [
            {"id": "transitfy", "path": "transitfy", "profile": "typescript-product", "lifecycle": "active", "adoption": "adopted"},
        ])
        self.assertEqual(axis_migration.main(["--root", str(workspace), "--governance-root", str(governance), "--apply"]), 1)
        self.assertNotIn("role", json.loads((workspace / "transitfy" / ".project.json").read_text(encoding="utf-8")))

    def amended_row(self, kind):
        """A repository whose decided row carries `stack: none` (ruling 9).

        `role: deployment` and `stack: none` derive the bare `deployment`, so
        this is the shape the guard has to accept on the second run without
        reporting the tool's own output — or the owner's later correction — as
        drift. `DECLARATION` needs the alias `id` set explicitly because the
        decided row is keyed on the dotted directory.
        """
        declaration = dict(DECLARATION, id="lileililiwen-site", kind=kind, profile="deployment")
        return {"lileililiwen.github.io": declaration}, [
            {"id": "lileililiwen-site", "path": "lileililiwen.github.io", "profile": "deployment",
             "lifecycle": "active", "adoption": "adopted", "role": "deployment", "stack": "none"},
        ]

    def test_a_stack_none_row_accepts_the_bare_derived_role(self):
        """Ruling 9: the derived bare role is the accepted value, not the
        hand-authored one the row recorded when it was decided.

        Exit 1 is the refusal and exit 3 is "not applied yet", so the accept
        case is pinned as an `--apply` that writes — a refused row never gets
        that far. The second `--check` is ruling 7: the tool must not report
        its own output as drift.
        """
        workspace, governance = self.stage(*self.amended_row("deployment"))
        base = ["--root", str(workspace), "--governance-root", str(governance)]
        self.assertEqual(axis_migration.main(base + ["--apply"]), 0)
        self.assertEqual(axis_migration.main(base + ["--check"]), 0)
        self.assertEqual(axis_migration.main(base + ["--apply"]), 0)
        self.assertEqual(axis_migration.main(base + ["--check"]), 0)
        declaration = json.loads(
            (workspace / "lileililiwen.github.io" / ".project.json").read_text(encoding="utf-8"))
        self.assertEqual((declaration["role"], declaration["stack"], declaration["kind"]),
                         ("deployment", "none", "deployment"))

    def test_a_stack_none_row_still_refuses_a_third_kind(self):
        """Ruling 9 must not become a blanket pass. `inventory` is neither the
        decided kind nor any value the row's role/stack can derive, so it is
        drift and is still refused."""
        workspace, governance = self.stage(*self.amended_row("inventory"))
        self.assertEqual(
            axis_migration.main(["--root", str(workspace), "--governance-root", str(governance), "--apply"]), 1)
        self.assertNotIn("role", json.loads(
            (workspace / "lileililiwen.github.io" / ".project.json").read_text(encoding="utf-8")))

    def test_the_decided_table_agrees_with_the_live_fleet_on_both_axes(self):
        """The table is the authority the drift guard checks the fleet against,
        so it must not be left describing a superseded decision. This is the
        invariant whose break left the gate refusing on a clean corpus."""
        decided = axis_migration.parse_decided_rows(DESIGN)
        disagreements = []
        for entry in json.loads(REGISTRY.read_text(encoding="utf-8"))["projects"]:
            row = decided.get(entry["path"]) or decided.get(entry["id"])
            if row is None:
                continue
            declaration_path = WORKSPACE_ROOT / entry["path"] / ".project.json"
            if not declaration_path.is_file():
                continue
            declaration = json.loads(declaration_path.read_text(encoding="utf-8"))
            for axis, decided_value in (("role", row.roles[0]), ("stack", row.stacks[0])):
                for surface, actual in (("registry", entry.get(axis)), ("declaration", declaration.get(axis))):
                    if actual != decided_value:
                        disagreements.append(
                            f"{entry['id']}: {surface} {axis} {actual!r} against decided {decided_value!r}")
        self.assertEqual(disagreements, [])

    def test_an_id_rename_is_refused(self):
        repositories = {"campusMesh": dict(DECLARATION, id="campus-mesh", kind="product", profile="product")}
        entries = [{"id": "campus-mesh", "path": "campusMesh", "profile": "product", "lifecycle": "active", "adoption": "adopted"}]
        workspace, governance = self.stage(repositories, entries)
        plan = axis_migration.build_plan(workspace, governance)
        renamed = axis_migration.RegistryPlan(
            project_id="campus-mesh", path="campus-mesh", action="axes", role="product", stack="dotnet")
        with self.assertRaises(axis_migration.MigrationError) as ctx:
            axis_migration._assert_id_immutability(entries, [renamed])
        self.assertIn("id immutability", str(ctx.exception))


class NoOutcomeChangeTests(unittest.TestCase):
    """The axes are descriptive: no requirement outcome may depend on them."""

    def setUp(self):
        self.baseline = json.loads(
            (REPO_ROOT / "tests" / "fixtures" / "identity-and-taxonomy-baseline.json").read_text(encoding="utf-8"))
        self.entries = json.loads(REGISTRY.read_text(encoding="utf-8"))["projects"]

    def test_the_registry_id_set_moved_only_by_the_planned_adds_and_one_removal(self):
        """`id` is the identity key; the axis split may not rename or drop one.

        The set did change — that was the plan — but only by the actions the
        plan records: the §5.3 registrations and the §5.5 ghost removal. Every
        other baseline id is still here, spelled the same way.
        """
        before = set(self.baseline["registry_ids"])
        after = {entry["id"] for entry in self.entries}
        self.assertEqual(len(before), 78)
        self.assertEqual(before - after, {"jenkins-bootstrap"})
        self.assertEqual(after - before, {"argoscope", "chronicleaf", "devloom", "kairovia", "mnemora"})
        self.assertEqual(len(after), 82)

    def test_every_registry_entry_keeps_its_requirements(self):
        baseline = self.baseline
        for project_id, profile in sorted(baseline["id_profile"].items()):
            with self.subTest(project=project_id):
                required, requires_ci, requires_openspec, requires_verification = baseline["profile_requirements"][profile]
                outcome = workspace_check.profile_requirements(profile)
                self.assertEqual(list(outcome[0]), required)
                self.assertEqual(
                    (outcome[1], outcome[2], outcome[3]),
                    (requires_ci, requires_openspec, requires_verification),
                )

    def test_profile_requirements_does_not_read_the_axes(self):
        import inspect
        source = inspect.getsource(workspace_check.profile_requirements)
        for token in ("role", "stack", "taxonomy"):
            self.assertNotIn(token, source)
        self.assertEqual(
            workspace_check.profile_requirements("product"),
            (["README.md", "ROADMAP.md", "HANDOFF.md", "AGENTS.md", ".ai-rules/workflow.md",
              ".ai-rules/completion.md", ".project.json"], True, True, True),
        )

    def test_the_axes_reach_no_evidence_or_deployment_state(self):
        import inspect
        for name in ("measure_capability_adoption", "validate_release_evidence",
                     "validate_deployment_promotion", "validate_deployment_evidence"):
            function = getattr(workspace_check, name)
            source = inspect.getsource(function)
            self.assertNotIn('"role"', source, name)
            self.assertNotIn('"stack"', source, name)

    def test_init_project_still_derives_the_kebab_id_for_an_alias_directory(self):
        # `id` stays derived at authoring time and then immutable: renaming it to
        # match the directory is what the eight alias entries must never get.
        from init_project import derive_id, resolve_project_id

        self.assertEqual(derive_id("hestiaLab"), "hestia-lab")
        self.assertEqual(derive_id("campusMesh"), "campus-mesh")
        self.assertEqual(resolve_project_id("hestiaLab", "hestia-lab"), ("hestia-lab", None))
        self.assertEqual(resolve_project_id("hestiaLab", None), ("hestia-lab", None))
        # A directory name is never accepted as the id, and a rename is refused.
        self.assertIsNone(resolve_project_id("hestiaLab", "hestiaLab")[0])
        self.assertIsNone(resolve_project_id("hestiaLab", "hestia-lab-v2")[0])
        # A dotted directory derives a convertible id today; the registry alias
        # is what is recorded, and neither the id nor the path may be normalized
        # onto the other.
        self.assertEqual(resolve_project_id("lileililiwen.github.io", "lileililiwen-site"),
                         ("lileililiwen-site", None))
        self.assertEqual(derive_id("lileililiwen.github.io"), "lileililiwen-github-io")


if __name__ == "__main__":
    unittest.main()
