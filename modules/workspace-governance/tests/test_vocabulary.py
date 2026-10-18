"""Tests for the single-sourced portfolio vocabulary (vocabulary.json)."""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))

import product_code_quality as pcq
import vocabulary
from vocabulary import VocabularyError, load_vocabulary
from workspace_check import derive_kind, run, validate_kind, validate_profile_docs

from _decisions_env import restore_decisions_register, use_empty_decisions_register

REPO_ROOT = Path(__file__).parents[1]


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


CANONICAL_PROFILES = {
    "governance", "product", "typescript-product", "python-product",
    "rust-product", "dotnet-product", "typescript-monorepo", "dotnet-library",
    "dotnet-template", "rust-platform", "python-runtime", "platform",
    "tooling", "deployment", "inventory",
}
CANONICAL_KINDS = {
    "product", "typescript-product", "python-product", "rust-product",
    "dotnet-product", "typescript-monorepo", "shared-library", "tooling",
    "platform", "governance", "deployment", "inventory",
}
# Every non-canonical declaration value observed at authoring time, with its
# canonical remap target from the design.
REMAPPED_KINDS = {
    "application": "product",
    "open-source-knowledge-platform": "product",
    "open-source-assessment-platform": "product",
    "web-site": "product",
    "application-platform": "platform",
    "runtime-platform": "platform",
    "control-plane": "platform",
    "rust-tooling": "tooling",
    "runtime-operator": "tooling",
    "quality-gate": "tooling",
    "agent-runtime": "tooling",
    "shared": "shared-library",
    "infrastructure": "deployment",
}
REMAPPED_PROFILES = {
    "web-platform": "product",
    "web-saas": "product",
    "mobile-platform": "product",
    "cross-border-care-platform": "product",
    "privacy-sensitive-web-saas": "product",
}
# Terms the previous ad-hoc lists and platform-contracts already matched.
# The vocabulary secret list must stay a superset of all of these.
LEGACY_SECRET_TERMS = {
    "password", "secret", "token", "apikey", "api_key", "credential",
    "private_key", "privatekey", "passwd", "bearer", "session_ke",
}

BASE_DECLARATION = {
    "schema_version": 1,
    "id": "demo-project",
    "kind": "product",
    "profile": "inventory",
    "lifecycle": "inventory",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def make_workspace(declaration=None, profile="inventory"):
    temp = tempfile.TemporaryDirectory()
    root = Path(temp.name)
    project = root / "demo-project"
    project.mkdir()
    decl = dict(BASE_DECLARATION if declaration is None else declaration)
    write_json(project / ".project.json", decl)
    registry = {
        "schema_version": 1,
        "workspace_root": None,
        "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
        "projects": [{"id": "demo-project", "path": "demo-project", "profile": profile,
                      "lifecycle": "inventory", "adoption": "adopted"}],
    }
    registry_path = root / "projects.json"
    write_json(registry_path, registry)
    return temp, root, registry_path


def audit(root, registry_path):
    return run(SimpleNamespace(registry=str(registry_path), root=str(root),
                               project=None, audit=True, self_check=False, as_json=False))


class VocabularyLoadTests(unittest.TestCase):
    def test_canonical_sets_match_design(self):
        vocab = vocabulary.load_vocabulary_for_governance_root(Path(__file__).parents[1])
        self.assertEqual(set(vocab["profiles"]), CANONICAL_PROFILES)
        self.assertEqual(set(vocab["kinds"]), CANONICAL_KINDS)
        self.assertEqual(vocab["schema_version"], 1)

    def test_placeholder_markers_are_unambiguous_debt_tokens(self):
        """The marker vocabulary carries code debt, not English.

        `placeholder` and `stub` were dropped: measured over
        `driftwatchdog/src/`, `placeholder` matched 26 times and none of
        the 26 was real debt -- every one was prose describing the word
        or the gate's own `placeholder-threshold` concern identifier. A
        check that cries wolf is ignored. See the spec delta in
        `openspec/changes/bind-declared-gate-checks`.
        """
        vocab = vocabulary.load_vocabulary_for_governance_root(Path(__file__).parents[1])
        self.assertEqual(tuple(vocab["placeholder_markers"]),
                         ("todo", "fixme", "xxx", "hack", "unimplemented!", "todo!"))
        for ordinary_english_word in ("placeholder", "stub"):
            self.assertNotIn(ordinary_english_word, vocab["placeholder_markers"])

    def test_secret_list_is_superset(self):
        vocab = vocabulary.load_vocabulary_for_governance_root(Path(__file__).parents[1])
        loaded = {term.lower() for term in vocab["secret_field_substrings"]}
        for term in LEGACY_SECRET_TERMS:
            matches = [candidate for candidate in loaded
                       if term in candidate or candidate in term.replace("-", "_")]
            self.assertTrue(matches, f"vocabulary no longer covers secret term: {term}")
        pattern = vocabulary.secret_pattern(vocab["secret_field_substrings"])
        for term in LEGACY_SECRET_TERMS:
            self.assertIsNotNone(pattern.search(term), f"secret pattern misses: {term}")

    def test_missing_vocabulary_names_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            missing = Path(tmp) / "vocabulary.json"
            with self.assertRaises(VocabularyError) as ctx:
                load_vocabulary(missing)
            self.assertIn(str(missing), str(ctx.exception))

    def test_unparseable_vocabulary_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            path.write_text("{not json", encoding="utf-8")
            with self.assertRaises(VocabularyError):
                load_vocabulary(path)

    def test_empty_list_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            vocab = {"schema_version": 1, "profiles": ["product"], "kinds": ["product"],
                     "placeholder_markers": ["todo"], "secret_field_substrings": []}
            path.write_text(json.dumps(vocab), encoding="utf-8")
            with self.assertRaises(VocabularyError):
                load_vocabulary(path)

    def test_duplicate_entry_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "vocabulary.json"
            vocab = {"schema_version": 1, "profiles": ["product", "product"], "kinds": ["product"],
                     "placeholder_markers": ["todo"], "secret_field_substrings": ["token"]}
            path.write_text(json.dumps(vocab), encoding="utf-8")
            with self.assertRaises(VocabularyError):
                load_vocabulary(path)


class ProfileEnforcementTests(unittest.TestCase):
    def test_canonical_registry_profile_passes(self):
        temp, root, registry = make_workspace(profile="inventory")
        self.addCleanup(temp.cleanup)
        findings, summary = audit(root, registry)
        self.assertFalse([item for item in findings if item.code == "PROFILE_UNKNOWN"], findings)
        self.assertEqual(summary["errors"], 0, findings)

    def test_non_canonical_registry_profile_is_error(self):
        temp, root, registry = make_workspace(profile="web-saas")
        self.addCleanup(temp.cleanup)
        findings, summary = audit(root, registry)
        unknown = [item for item in findings if item.code == "PROFILE_UNKNOWN"]
        self.assertEqual(len(unknown), 1)
        self.assertEqual(unknown[0].level, "ERROR")
        self.assertIn("web-saas", unknown[0].message)
        self.assertGreater(summary["errors"], 0)

    def test_every_remapped_profile_target_is_canonical(self):
        for old, new in REMAPPED_PROFILES.items():
            self.assertNotIn(old, CANONICAL_PROFILES)
            self.assertIn(new, CANONICAL_PROFILES)

    def test_profile_docs_agree_with_vocabulary(self):
        from workspace_check import Finding
        findings: list[Finding] = []
        governance_root = Path(__file__).parents[1]
        validate_profile_docs(governance_root, findings, canonical_profiles=CANONICAL_PROFILES)
        self.assertEqual(findings, [])

    def test_profile_doc_outside_set_is_error(self):
        from workspace_check import Finding
        with tempfile.TemporaryDirectory() as tmp:
            profiles = Path(tmp) / "profiles"
            profiles.mkdir()
            (profiles / "web-saas.json").write_text(json.dumps({"id": "web-saas"}), encoding="utf-8")
            findings: list[Finding] = []
            validate_profile_docs(Path(tmp), findings, canonical_profiles=CANONICAL_PROFILES)
            self.assertEqual(len(findings), 1)
            self.assertEqual(findings[0].code, "PROFILE_DOC_UNKNOWN")
            self.assertEqual(findings[0].level, "ERROR")


class KindEnforcementTests(unittest.TestCase):
    def test_canonical_kind_passes(self):
        from workspace_check import Finding
        findings: list[Finding] = []
        validate_kind({"kind": "product"}, findings, "demo", canonical_kinds=CANONICAL_KINDS)
        self.assertEqual(findings, [])

    def test_non_canonical_kind_warns_without_driving_requirements(self):
        temp, root, registry = make_workspace()
        self.addCleanup(temp.cleanup)
        project = root / "demo-project"
        for name in ("README.md", "ROADMAP.md", "HANDOFF.md", "AGENTS.md"):
            (project / name).write_text("# x\n")
        (project / ".ai-rules").mkdir()
        (project / ".ai-rules" / "workflow.md").write_text("x\n")
        (project / ".ai-rules" / "completion.md").write_text("x\n")
        decl = dict(BASE_DECLARATION, kind="product", profile="product")
        write_json(project / ".project.json", decl)
        data = json.loads(registry.read_text())
        data["projects"][0]["profile"] = "product"
        write_json(registry, data)
        canonical_findings, canonical_summary = audit(root, registry)
        decl = dict(BASE_DECLARATION, kind="quality-gate", profile="product")
        write_json(project / ".project.json", decl)
        findings, summary = audit(root, registry)
        unknown = [item for item in findings if item.code == "KIND_UNKNOWN"]
        self.assertEqual(len(unknown), 1)
        self.assertEqual(unknown[0].level, "WARNING")
        self.assertIn("quality-gate", unknown[0].message)
        rest = [item for item in findings if item.code != "KIND_UNKNOWN"]
        canonical_rest = [item for item in canonical_findings if item.code != "KIND_UNKNOWN"]
        self.assertEqual([(item.level, item.code) for item in rest],
                         [(item.level, item.code) for item in canonical_rest])
        self.assertEqual(summary["errors"], canonical_summary["errors"])

    def test_every_remapped_kind_target_is_canonical(self):
        for old, new in REMAPPED_KINDS.items():
            self.assertNotIn(old, CANONICAL_KINDS)
            self.assertIn(new, CANONICAL_KINDS)


class DerivedKindKnownTests(unittest.TestCase):
    """`kind` is derived and deprecated, so its canonical form is whatever the
    two axes derive. `kinds` is the frozen legacy vocabulary and is not the
    authority on a derived value: the derivation is a product of two enumerated
    axis sets, so it is deliberately not a hand-maintained list. Every side of
    the rule is pinned — a self-consistent derivation is known, and the three
    ways of *not* being one still report, at WARNING."""

    ROLES = {"product", "platform", "tooling", "governance", "deployment", "inventory"}
    STACKS = {"dotnet", "rust", "typescript", "python", "flutter", "none"}

    def collected(self, declaration):
        findings: list = []
        validate_kind(declaration, findings, "demo-project",
                      canonical_kinds=CANONICAL_KINDS,
                      canonical_roles=self.ROLES, canonical_stacks=self.STACKS)
        return findings

    def test_a_self_consistent_derived_value_produces_no_finding(self):
        # `rust-platform` is deliberately NOT in the frozen 12, and is known
        # anyway because it is what `platform`/`rust` derive.
        self.assertNotIn("rust-platform", CANONICAL_KINDS)
        declaration = {"kind": "rust-platform", "role": "platform", "stack": "rust"}
        self.assertEqual(self.collected(declaration), [])

    def test_every_derived_value_outside_the_legacy_vocabulary_is_known(self):
        # The whole derivation space, measured against the real axis sets and
        # the frozen 12. This is the sweep that the hand-maintained list could
        # not express: no combination is enumerated here.
        vocab = vocabulary.load_vocabulary_for_governance_root(REPO_ROOT)
        unlisted = 0
        for role in vocab["roles"]:
            for stack in vocab["stacks"]:
                kind = derive_kind(role, stack)
                if kind not in CANONICAL_KINDS:
                    unlisted += 1
                declaration = {"kind": kind, "role": role, "stack": stack}
                self.assertEqual(self.collected(declaration), [], f"{role}/{stack}")
        # The space is genuinely larger than the legacy list, so the sweep
        # cannot pass by accident on a vocabulary that already covers it.
        self.assertGreater(unlisted, 0)

    def test_a_legacy_value_that_disagrees_with_its_own_derivation_still_reports(self):
        declaration = {"kind": "rust-governance", "role": "governance", "stack": "python"}
        self.assertNotEqual(declaration["kind"], derive_kind("governance", "python"))
        found = self.collected(declaration)
        self.assertEqual([item.code for item in found], ["KIND_UNKNOWN"])
        self.assertEqual(found[0].level, "WARNING")

    def test_a_value_in_neither_form_still_reports(self):
        declaration = {"kind": "quality-gate", "role": "tooling", "stack": "rust"}
        found = self.collected(declaration)
        self.assertEqual([item.code for item in found], ["KIND_UNKNOWN"])
        self.assertEqual(found[0].level, "WARNING")
        self.assertIn("quality-gate", found[0].message)

    def test_absent_axes_behave_as_before(self):
        # Nothing has been derived, so an unlisted value is still unmeasured.
        found = self.collected({"kind": "rust-platform"})
        self.assertEqual([item.code for item in found], ["KIND_UNKNOWN"])
        self.assertEqual(found[0].level, "WARNING")

    def test_a_half_declared_axis_pair_behaves_as_before(self):
        found = self.collected({"kind": "rust-platform", "role": "platform"})
        self.assertEqual([item.code for item in found], ["KIND_UNKNOWN"])

    def test_non_canonical_axes_behave_as_before(self):
        # Both axes are present but `role` is outside its set, so the derived
        # form is not measurable and the rule does not excuse the value.
        for axes in ({"role": "library", "stack": "rust"},
                     {"role": "platform", "stack": "haskell"}):
            found = self.collected({"kind": "rust-platform", **axes})
            self.assertEqual([item.code for item in found], ["KIND_UNKNOWN"], axes)

    def test_an_absent_or_non_string_kind_still_reports(self):
        for declaration in ({}, {"kind": None}, {"kind": 7}):
            found = self.collected(declaration)
            self.assertEqual([item.code for item in found], ["KIND_UNKNOWN"], declaration)

    def test_a_canonical_kind_stays_known_with_unrelated_axes(self):
        self.assertEqual(self.collected({"kind": "shared-library", "role": "platform", "stack": "rust"}), [])


class SharedWordListTests(unittest.TestCase):
    def test_secret_scan_fires_with_vocabulary_term(self):
        temp, root, registry = make_workspace()
        self.addCleanup(temp.cleanup)
        data = json.loads(registry.read_text())
        data["bearer"] = "do-not-store"
        write_json(registry, data)
        findings, summary = audit(root, registry)
        self.assertTrue(any(item.code == "SECRET_VALUE" for item in findings), findings)
        self.assertGreater(summary["errors"], 0)

    def test_placeholder_default_comes_from_vocabulary(self):
        vocab = vocabulary.load_vocabulary_for_governance_root(Path(__file__).parents[1])
        self.assertEqual(pcq.default_placeholder_markers(), tuple(vocab["placeholder_markers"]))
        self.assertEqual(pcq.Policy(product_roots=("src",), languages=("python",)).placeholder_markers,
                         tuple(vocab["placeholder_markers"]))

    def test_placeholder_override_still_honoured(self):
        policy = pcq.parse_policy({"product_roots": ["src"], "languages": ["python"],
                                   "placeholder_markers": ["custommarker"]})
        self.assertEqual(policy.placeholder_markers, ("custommarker",))


if __name__ == "__main__":
    unittest.main()
