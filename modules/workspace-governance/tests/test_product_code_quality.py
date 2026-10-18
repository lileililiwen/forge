"""Tests for `scripts/product_code_quality.py`.

Covers the scenarios in
`openspec/changes/product-code-quality-gates/specs/product-code-quality-boundary/spec.md`:
- valid product policy scans the declared product scope,
- missing or invalid policy is REVIEW_REQUIRED,
- Rust `#[cfg(test)]` containment is allowed; outside it is rejected,
- Python, TypeScript, and C# test patterns in product scope are FAIL,
- placeholder threshold comparison is enforced,
- excluded fixtures do not count toward the threshold,
- unreadable product files are REVIEW_REQUIRED (fail-closed),
- the JSON output is one parseable document and the process exits 1 on FAIL.
"""

import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))
import product_code_quality as pcq  # noqa: E402

from _decisions_env import restore_decisions_register, use_empty_decisions_register  # noqa: E402


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


SCRIPTS_DIR = Path(__file__).resolve().parents[1] / "scripts"


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _make_project(roots: list[str], languages: list[str], exclude: list[str] | None = None, threshold: int = 0) -> tuple[tempfile.TemporaryDirectory, Path]:
    tmp = tempfile.TemporaryDirectory()
    root = Path(tmp.name)
    declaration = {
        "schema_version": 1,
        "id": "fixture",
        "kind": "product",
        "profile": "product",
        "lifecycle": "inventory",
        "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
        "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
        "quality": {
            "product_roots": roots,
            "languages": languages,
            "exclude": exclude or [],
            "placeholder_threshold": threshold,
        },
    }
    (root / ".project.json").write_text(json.dumps(declaration), encoding="utf-8")
    return tmp, root


class PolicyTests(unittest.TestCase):
    def test_minimal_policy_parses(self):
        policy = pcq.parse_policy({"product_roots": ["src"], "languages": ["rust"]})
        self.assertEqual(policy.product_roots, ("src",))
        self.assertEqual(policy.languages, ("rust",))
        self.assertEqual(policy.placeholder_threshold, 0)
        self.assertEqual(policy.placeholder_markers, pcq.DEFAULT_PLACEHOLDER_MARKERS)

    def test_empty_product_roots_is_rejected(self):
        with self.assertRaises(pcq.ConfigError):
            pcq.parse_policy({"product_roots": [], "languages": ["rust"]})

    def test_unknown_language_is_rejected(self):
        with self.assertRaises(pcq.ConfigError):
            pcq.parse_policy({"product_roots": ["src"], "languages": ["brainfuck"]})

    def test_threshold_must_be_non_negative(self):
        with self.assertRaises(pcq.ConfigError):
            pcq.parse_policy({"product_roots": ["src"], "languages": ["rust"], "placeholder_threshold": -1})

    def test_exclude_must_be_project_relative(self):
        with self.assertRaises(pcq.ConfigError):
            pcq.parse_policy({"product_roots": ["src"], "languages": ["rust"], "exclude": ["../escape"]})

    def test_threshold_must_be_integer(self):
        with self.assertRaises(pcq.ConfigError):
            pcq.parse_policy({"product_roots": ["src"], "languages": ["rust"], "placeholder_threshold": "zero"})


class RustBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["rust"])

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_cfg_test_module_is_allowed(self):
        _write(self.root / "src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn it_works() { assert_eq!(add(1, 2), 3); }\n}\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "PASS", report.notes)
        self.assertEqual(report.summary["test_code"], 0)

    def test_test_attribute_outside_cfg_test_is_rejected(self):
        _write(self.root / "src/lib.rs", "#[test]\nfn it_works() { assert!(true); }\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "product-test-code" for f in report.findings))

    def test_mod_tests_without_cfg_test_is_rejected(self):
        _write(self.root / "src/lib.rs", "mod tests {\n    #[test]\n    fn it_works() {}\n}\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "product-test-code" for f in report.findings))

    def test_tests_directory_outside_product_root_is_excluded(self):
        _write(self.root / "src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n")
        _write(self.root / "tests/integration.rs", "#[test]\nfn external() {}\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "PASS", report.notes)
        self.assertEqual(report.summary["test_code"], 0)


class RustModuleTreeBoundaryTests(unittest.TestCase):
    """The Rust branch decides test scope from the crate's module tree.

    The shape these pin is `src/<dir>/tests.rs` reached by
    `#[cfg(test)] mod tests;` in the sibling `mod.rs`. It is what
    `driftwatchdog/src/gate/governance/tests.rs` is, and a single-file lexical
    scan reports its contents as product test code, which made a healthy
    repository red. Every test here has to keep failing if the declaration goes
    away — that is the difference between reading the module graph and adding a
    file-name exception.
    """

    TESTS_RS = (
        "use super::*;\n"
        "\n"
        "#[test]\n"
        "fn it_works() {\n"
        "    assert_eq!(add(1, 2), 3);\n"
        "}\n"
        "\n"
        "#[test]\n"
        "fn it_also_works() {\n"
        "    assert_eq!(add(2, 2), 4);\n"
        "}\n"
    )

    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["rust"])

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def _evaluate(self) -> Any:
        declaration = json.loads((self.root / ".project.json").read_text(encoding="utf-8"))
        return pcq.evaluate(pcq.load_policy(declaration).with_root(self.root))

    def _test_code_findings(self, report: Any) -> list[Any]:
        return [f for f in report.findings if f.rule_id == "product-test-code"]

    def _findings_in(self, report: Any, rel: str) -> list[Any]:
        return [f for f in self._test_code_findings(report) if f.file == rel]

    def test_cfg_test_declaration_in_sibling_mod_rs_clears_the_tests_file(self):
        """The shape measured on driftwatchdog: declared, so not product code."""
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nmod tests;\n")
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        self.assertEqual(report.summary["test_code"], 0, [f.marker for f in report.findings])
        self.assertEqual(report.status, "PASS", report.notes)

    def test_the_declaration_is_what_clears_it_not_the_file_name(self):
        """Remove the declaration and the same file is product test code again."""
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\nmod tests;\n")
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        findings = self._findings_in(report, "src/governance/tests.rs")
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertEqual(len(findings), 2, [(f.file, f.line) for f in findings])
        # The unguarded declaration is itself a finding: it is the line a reader
        # has to fix, and the two test functions are what it drags into product.
        self.assertEqual([f.line for f in self._findings_in(report, "src/governance/mod.rs")], [3])

    def test_a_tests_file_with_no_declaration_at_all_is_reported(self):
        """No `mod.rs` anywhere: the name authorizes nothing."""
        _write(self.root / "src/governance.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n")
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertEqual(len(self._findings_in(report, "src/governance/tests.rs")), 2)

    def test_a_non_cfg_test_declaration_still_reports_the_tests_file(self):
        """`mod tests;` without the guard compiles the file into the product."""
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\nmod tests;\n")
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        self.assertEqual(len(self._findings_in(report, "src/governance/tests.rs")), 2)

    def test_an_ambiguous_cfg_does_not_authorize_test_scope(self):
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(any(test, feature = \"integration\"))]\nmod tests;\n")
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        self.assertEqual(len(self._findings_in(report, "src/governance/tests.rs")), 2)

    def test_lib_rs_declares_the_test_module_too(self):
        _write(self.root / "src/governance/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nmod tests;\n")
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        self.assertEqual(report.summary["test_code"], 0, [f.marker for f in report.findings])

    def test_an_intervening_path_attribute_does_not_disarm_the_guard(self):
        """`#[cfg(test)]` `#[path = ...]` `mod tests;` is one guarded item.

        This is the shape in `rust-ecommerce/src/shipment/mod.rs` and
        `alltools-platform/.../typed_loop.rs`, where the old scanner dropped the
        guard on the attribute line between the guard and the item.
        """
        _write(
            self.root / "src/governance/mod.rs",
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\n#[path = \"tests.rs\"]\nmod tests;\n",
        )
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        self.assertEqual(report.summary["test_code"], 0, [f.marker for f in report.findings])
        self.assertEqual(report.status, "PASS", report.notes)

    def test_an_intervening_allow_attribute_does_not_disarm_an_inline_guard(self):
        """`#[cfg(test)]` `#[allow(...)]` `mod tests { ... }` is one item too."""
        _write(
            self.root / "src/governance.rs",
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\n#[allow(clippy::await_holding_lock)]\nmod tests {\n    use super::*;\n    #[test]\n    fn it_works() { assert_eq!(add(1, 2), 3); }\n}\n",
        )
        report = self._evaluate()
        self.assertEqual(report.summary["test_code"], 0, [f.marker for f in report.findings])
        self.assertEqual(report.status, "PASS", report.notes)

    def test_real_test_code_in_a_product_file_is_still_reported(self):
        """A product module with a bare `#[test]` and no guard stays a finding."""
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n")
        _write(self.root / "src/governance/add.rs", "#[test]\nfn it_works() { assert_eq!(add(1, 2), 3); }\n")
        report = self._evaluate()
        findings = self._test_code_findings(report)
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertEqual([f.file for f in findings], ["src/governance/add.rs"])

    def test_a_non_test_file_reached_by_a_cfg_test_declaration_is_cleared(self):
        """The rule is the declaration, not the conventional file name."""
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nmod helpers;\n")
        _write(self.root / "src/governance/helpers.rs", "#[test]\nfn helper_works() { assert_eq!(add(1, 2), 3); }\n")
        report = self._evaluate()
        self.assertEqual(report.summary["test_code"], 0, [f.marker for f in report.findings])

    def test_a_declaring_file_elsewhere_does_not_clear_the_file(self):
        """The declaration has to reach *this* module, not some other one."""
        _write(self.root / "src/other/mod.rs", "#[cfg(test)]\nmod tests;\n")
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n")
        _write(self.root / "src/governance/tests.rs", self.TESTS_RS)
        report = self._evaluate()
        self.assertEqual(len(self._test_code_findings(report)), 2)

    def test_a_tests_directory_in_a_product_root_stays_out_of_scope(self):
        """Already excluded by the `tests` path segment, and still excluded."""
        _write(self.root / "src/governance/mod.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n")
        _write(self.root / "src/governance/tests/mod.rs", "#[test]\nfn it_works() {}\n")
        report = self._evaluate()
        self.assertEqual(report.summary["test_code"], 0)
        self.assertEqual(report.summary["excluded_paths"], 1, report.summary)

    def test_helper_predicates_answer_the_module_tree_questions(self):
        declared = self.root / "src/gate/governance"
        _write(declared / "mod.rs", "#[cfg(test)]\nmod tests;\npub mod sibling;\n")
        _write(declared / "tests.rs", self.TESTS_RS)
        _write(declared / "sibling.rs", "pub fn x() {}\n")
        self.assertTrue(pcq._rust_declared_under_cfg_test(declared / "tests.rs"))
        self.assertFalse(pcq._rust_declared_under_cfg_test(declared / "sibling.rs"))
        self.assertEqual(pcq._rust_module_keys(declared / "tests.rs"), ("tests",))


class PythonBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["python"])

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_unittest_import_in_product_is_rejected(self):
        _write(self.root / "src/util.py", "import unittest\nclass UtilTests(unittest.TestCase): pass\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "product-test-code" for f in report.findings))

    def test_def_test_in_non_test_file_is_rejected(self):
        _write(self.root / "src/util.py", "def add(a, b):\n    return a + b\n\ndef test_add():\n    assert add(1, 2) == 3\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "product-test-code" for f in report.findings))

    def test_pure_module_passes(self):
        _write(self.root / "src/util.py", "def add(a, b):\n    return a + b\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "PASS", report.notes)
        self.assertEqual(report.summary["test_code"], 0)


class TypeScriptBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["typescript"])

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_describe_call_in_source_is_rejected(self):
        _write(self.root / "src/util.ts", "export function add(a: number, b: number): number { return a + b; }\n\ndescribe('add', () => { it('works', () => {}); });\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "product-test-code" for f in report.findings))

    def test_pure_source_passes(self):
        _write(self.root / "src/util.ts", "export function add(a: number, b: number): number { return a + b; }\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "PASS", report.notes)
        self.assertEqual(report.summary["test_code"], 0)


class TypeScriptQualifierTests(unittest.TestCase):
    """A method named `test` is a method, not a test.

    Every line here was a measured false positive in the fleet. A two-token
    lexical match cannot tell a Mocha global from a regular expression or a
    string method that shares the name, and the fix anchors on the qualifier,
    which is the only thing that separates them. These are the real lines, not
    invented ones — `argoset`, `crossify`, `fotofy` and `transitfy` were red
    for exactly these shapes.
    """

    CASES = [
        ("regex_literal", 'if (/attestation/i.test(err.message)) { fail(); }'),
        ("regex_literal_return", "return /iphone|android/.test(ua);"),
        ("named_regex", "} else if (QUOTA_PATTERN.test(body)) {"),
        ("string_literal", 'return typeof v === "string" && /^https?:\\/\\//.test(v);'),
        ("optional_chain", "if (re?.test(body)) { x(); }"),
        ("member", "suite.test('x');"),
        ("prose_comment", " * contract drift test (see `src/api/contract.test.ts`) can compare"),
        ("identifier_prefix", "const latest = compute();"),
        ("word_suffix", "contest(1);"),
    ]

    def test_no_method_or_comment_is_reported_as_test_code(self):
        for name, body in self.CASES:
            with self.subTest(case=name):
                tmp = pathlib.Path(tempfile.mkdtemp())
                try:
                    path = tmp / "sample.ts"
                    findings = pcq._scan_typescript_test_code(path, body.splitlines())
                    self.assertEqual(
                        findings, [],
                        f"{name} was reported as test code: {body!r}",
                    )
                finally:
                    shutil.rmtree(tmp, ignore_errors=True)

    def test_a_bare_global_call_is_still_reported(self):
        # The negative control for every case above. If the qualifier anchor
        # were dropped to make the false positives go away, these would pass
        # silently and the rule would be dead.
        for name, body in (
            ("describe", "describe('thing', () => {});"),
            ("it", "  it('works', () => {});"),
            ("test", "test('works', () => expect(1).toBe(1));"),
        ):
            with self.subTest(case=name):
                tmp = pathlib.Path(tempfile.mkdtemp())
                try:
                    path = tmp / "sample.ts"
                    findings = pcq._scan_typescript_test_code(path, body.splitlines())
                    self.assertEqual(len(findings), 1, f"{name} was not reported")
                    self.assertEqual(findings[0].rule_id, "product-test-code")
                finally:
                    shutil.rmtree(tmp, ignore_errors=True)


class CSharpBoundaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["csharp"])

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_test_attribute_in_product_is_rejected(self):
        _write(self.root / "src/Util.cs", "public class Util { public int Add(int a, int b) => a + b; }\n\npublic class UtilTests { [Test] public void AddWorks() {} }\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "product-test-code" for f in report.findings))

    def test_fact_attribute_in_product_is_rejected(self):
        _write(self.root / "src/Util.cs", "public class Util {\n    [Fact]\n    public void Works() {}\n}\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "product-test-code" for f in report.findings))


class PlaceholderTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["rust"], threshold=0)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_todo_marker_fails_at_zero_threshold(self):
        _write(self.root / "src/lib.rs", "pub fn x() { /* TODO: finish this */ }\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertTrue(any(f.rule_id == "placeholder-marker" for f in report.findings))

    def test_no_marker_passes(self):
        _write(self.root / "src/lib.rs", "pub fn x() -> i32 { 42 }\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.status, "PASS", report.notes)
        self.assertEqual(report.summary["placeholder"], 0)

    def test_threshold_above_count_passes(self):
        tmp, root = _make_project(["src"], ["rust"], threshold=3)
        try:
            _write(root / "src/lib.rs", "pub fn x() { /* TODO */ }\n")
            report = pcq.evaluate(pcq.load_policy(json.loads((root / ".project.json").read_text())).with_root(root))
            self.assertEqual(report.status, "PASS", report.notes)
            self.assertEqual(report.summary["placeholder"], 1)
        finally:
            tmp.cleanup()

    def test_the_english_word_placeholder_in_prose_is_not_a_finding(self):
        """The marker vocabulary carries debt tokens, not English.

        `placeholder` and `stub` were removed from `vocabulary.json`
        because on real source they match prose describing the word, not
        unfinished work. `TODO` and `FIXME` still do.
        """
        _write(self.root / "src/lib.rs",
               "//! This module extends this placeholder with a real implementation.\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.summary["placeholder"], 0)
        self.assertEqual(report.status, "PASS", report.notes)

    def test_the_english_word_placeholder_in_an_identifier_is_not_a_finding(self):
        """A concern named `placeholder-threshold` is the gate's own id."""
        _write(self.root / "src/gate.rs",
               "const PLACEHOLDER_THRESHOLD: usize = 0;\n"
               "/// Ordered, comma-separated placeholder names.\n"
               "pub fn stub_names() -> usize { 0 }\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.summary["placeholder"], 0)
        self.assertEqual(report.status, "PASS", report.notes)

    def test_todo_and_fixme_are_still_findings(self):
        _write(self.root / "src/lib.rs",
               "// TODO: finish this\n"
               "pub fn x() { /* FIXME: and this */ }\n")
        report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
        self.assertEqual(report.summary["placeholder"], 2)
        self.assertEqual(report.status, "FAIL", report.notes)
        self.assertEqual({f.marker.lower() for f in report.findings}, {"todo", "fixme"})

    def test_excluded_fixture_does_not_count(self):
        tmp, root = _make_project(["src"], ["rust"], threshold=0, exclude=["src/fixtures"])
        try:
            _write(root / "src/lib.rs", "pub fn x() -> i32 { 42 }\n")
            _write(root / "src/fixtures/sample.rs", "// TODO: not real code\n")
            report = pcq.evaluate(pcq.load_policy(json.loads((root / ".project.json").read_text())).with_root(root))
            self.assertEqual(report.status, "PASS", report.notes)
            self.assertEqual(report.summary["placeholder"], 0)
        finally:
            tmp.cleanup()

    def test_unreadable_product_file_is_review_required(self):
        tmp, root = _make_project(["src"], ["rust"], threshold=0)
        try:
            target = root / "src/lib.rs"
            _write(target, "pub fn x() -> i32 { 42 }\n")
            os.chmod(target, 0o000)
            if os.access(target, os.R_OK):
                # Filesystem ignores permission bits (e.g. running as root).
                self.skipTest("permission bits are not enforced on this filesystem")
            report = pcq.evaluate(pcq.load_policy(json.loads((root / ".project.json").read_text())).with_root(root))
            self.assertEqual(report.status, "REVIEW_REQUIRED", report.notes)
            self.assertTrue(any("unreadable" in note for note in report.notes))
        finally:
            try:
                os.chmod(target, 0o644)
            except OSError:
                pass
            tmp.cleanup()


class ConfigErrorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_missing_declaration_is_review_required(self):
        result = subprocess.run(
            [sys.executable, str(SCRIPTS_DIR / "product_code_quality.py"), str(self.root), "--json"],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 2)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "REVIEW_REQUIRED")
        self.assertEqual(report["schema_version"], pcq.SCHEMA_VERSION)

    def test_missing_quality_block_is_review_required(self):
        declaration = {
            "schema_version": 1,
            "id": "fixture",
            "kind": "product",
            "profile": "product",
            "lifecycle": "inventory",
            "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
        }
        (self.root / ".project.json").write_text(json.dumps(declaration), encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(SCRIPTS_DIR / "product_code_quality.py"), str(self.root), "--json"],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 2)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "REVIEW_REQUIRED")

    def test_product_roots_pointing_at_missing_directory_is_review_required(self):
        _make_project_root = _make_project(["src"], ["rust"])
        tmp, root = _make_project_root
        try:
            # Intentionally do not create src/.
            result = subprocess.run(
                [sys.executable, str(SCRIPTS_DIR / "product_code_quality.py"), str(root), "--json"],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 2)
            report = json.loads(result.stdout)
            self.assertEqual(report["status"], "REVIEW_REQUIRED")
        finally:
            tmp.cleanup()


class JsonContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["rust"], threshold=0)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_json_failure_report_includes_findings_and_exits_1(self):
        _write(self.root / "src/lib.rs", "pub fn x() { /* TODO: finish */ }\n")
        result = subprocess.run(
            [sys.executable, str(SCRIPTS_DIR / "product_code_quality.py"), str(self.root), "--json"],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "FAIL")
        self.assertEqual(report["schema_version"], pcq.SCHEMA_VERSION)
        self.assertGreater(len(report["findings"]), 0)
        for finding in report["findings"]:
            self.assertIn("rule_id", finding)
            self.assertIn("file", finding)
            self.assertIn("line", finding)
            self.assertIn("remediation", finding)
            self.assertIn("severity", finding)

    def test_secret_token_in_marker_is_redacted(self):
        # The default marker vocabulary does not include literal secret tokens,
        # so simulate a project whose marker is the literal word "secret" by
        # extending the vocabulary.
        declaration = json.loads((self.root / ".project.json").read_text())
        declaration["quality"]["placeholder_markers"] = ["secret"]
        (self.root / ".project.json").write_text(json.dumps(declaration), encoding="utf-8")
        _write(self.root / "src/lib.rs", "pub fn x() { /* secret value */ }\n")
        result = subprocess.run(
            [sys.executable, str(SCRIPTS_DIR / "product_code_quality.py"), str(self.root), "--json"],
            check=False,
            capture_output=True,
            text=True,
        )
        # The default policy treats unknown markers as pass (counted but not
        # listed). The literal word "secret" is a placeholder marker, so the
        # scan would find it; however the SECRET_WORDS redaction replaces
        # "secret" with "[REDACTED]" in the output.
        if result.returncode == 1:
            report = json.loads(result.stdout)
            for finding in report["findings"]:
                self.assertNotIn("secret", finding["marker"].lower())


class SymlinkAndTraversalTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp, self.root = _make_project(["src"], ["generic"])

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_symlink_to_outside_path_is_ignored(self):
        # Use an absolute outside directory to avoid `..` resolution pitfalls.
        outside = Path(tempfile.gettempdir()) / f"_pcq_outside_{os.getpid()}"
        try:
            outside.mkdir()
            (outside / "leaked.txt").write_text("TODO: leaked marker\n")
            (self.root / "src").mkdir(parents=True, exist_ok=True)
            link = self.root / "src" / "leak"
            link.symlink_to(outside)
            report = pcq.evaluate(pcq.load_policy(json.loads((self.root / ".project.json").read_text())).with_root(self.root))
            # The symlink points at a directory, not a file, so it is not
            # enumerated as a file by `_candidate_files`. The test is that
            # nothing inside the symlink target is treated as product code.
            self.assertEqual(report.summary["placeholder"], 0)
        finally:
            try:
                (self.root / "src" / "leak").unlink()
            except FileNotFoundError:
                pass
            try:
                (outside / "leaked.txt").unlink()
            except FileNotFoundError:
                pass
            outside.rmdir()

    def test_product_root_must_be_project_relative(self):
        with self.assertRaises(pcq.ConfigError):
            pcq.parse_policy({"product_roots": ["/etc/passwd"], "languages": ["generic"]})


class AuditIntegrationTests(unittest.TestCase):
    """Workspace Governance audit translates checker findings into Findings."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.project = self.root / "demo-project"
        self.project.mkdir()
        self.registry_path = self.root / "projects.json"

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def _run_audit(self, declaration: dict[str, Any]) -> tuple[list, dict[str, int]]:
        import sys
        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
        import workspace_check
        if declaration.get("profile") != "inventory":
            declaration.setdefault("discoverability", {
                "description": "A test project for governance checks.",
                "keywords": ["test", "governance"],
                "topics": ["governance-test"],
                "audience": ["developers"],
                "use_cases": ["run governance tests"],
                "search_phrases": ["governance test project"],
                "homepage": "https://example.com/demo-project",
                "license_spdx": "MIT",
                "visibility": "private",
            })
            declaration.setdefault("publication", {
                "provider": "github",
                "owner": "example",
                "repository": "demo-project",
                "default_branch": "main",
                "state": "metadata_verified",
                "evidence_ref": "HANDOFF.md",
            })
        declaration_path = self.project / ".project.json"
        declaration_path.write_text(json.dumps(declaration), encoding="utf-8")
        (self.project / "README.md").write_text("# demo\n", encoding="utf-8")
        (self.project / "ROADMAP.md").write_text("# r\n", encoding="utf-8")
        (self.project / "HANDOFF.md").write_text("current_spec: x\n", encoding="utf-8")
        (self.project / "AGENTS.md").write_text("# a\n", encoding="utf-8")
        # Use the real shipped rule files rather than stubs. A stub cannot carry
        # the baseline clauses, so every rule-drift clause reads as absent and the
        # fixture drowns in RULES_DRIFT_NEW. Loading the templates also means this
        # fixture cannot rot when the canonical rule text changes.
        import sys as _sys
        _sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
        import init_project as _init_project
        _templates = _init_project.load_templates()
        (self.project / ".ai-rules").mkdir()
        (self.project / ".ai-rules" / "workflow.md").write_text(_templates[".ai-rules/workflow.md"], encoding="utf-8")
        (self.project / ".ai-rules" / "completion.md").write_text(_templates[".ai-rules/completion.md"], encoding="utf-8")
        (self.project / "LICENSE").write_text("MIT License\n", encoding="utf-8")
        (self.project / "openspec").mkdir()
        (self.project / "Jenkinsfile").write_text("// ci\n", encoding="utf-8")
        entry = {
            "id": declaration["id"],
            "path": "demo-project",
            "profile": declaration["profile"],
            "lifecycle": declaration["lifecycle"],
            "adoption": "adopted",
        }
        # The registry entry is the second authority surface for the two axes,
        # so a declaration that declares them gets the same pair mirrored here;
        # a disagreement between the surfaces is an ERROR, not a warning.
        for axis in ("role", "stack"):
            if axis in declaration:
                entry[axis] = declaration[axis]
        registry = {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
            "projects": [entry],
        }
        self.registry_path.write_text(json.dumps(registry), encoding="utf-8")
        args = type("Args", (), {
            "registry": str(self.registry_path),
            "root": str(self.root),
            "project": None,
            "audit": False,
            "self_check": False,
            "as_json": False,
        })()
        return workspace_check.run(args)

    def test_quality_block_drives_audit_findings(self):
        declaration = {
            "schema_version": 1,
            "id": "demo-project",
            "kind": "product",
            "profile": "product",
            "lifecycle": "active",
            "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
            "quality": {
                "product_roots": ["src"],
                "languages": ["rust"],
                "placeholder_threshold": 0,
            },
        }
        (self.project / "src").mkdir()
        (self.project / "src" / "lib.rs").write_text("pub fn x() { /* TODO: finish */ }\n", encoding="utf-8")
        findings, summary = self._run_audit(declaration)
        self.assertGreater(summary["errors"], 0, findings)
        self.assertTrue(any(item.code == "QUALITY_PLACEHOLDER" for item in findings), findings)

    def test_quality_block_without_source_directory_is_review_required(self):
        declaration = {
            "schema_version": 1,
            "id": "demo-project",
            "kind": "product",
            "profile": "product",
            "lifecycle": "active",
            "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
            "quality": {
                "product_roots": ["src"],
                "languages": ["rust"],
                "placeholder_threshold": 0,
            },
        }
        # src/ intentionally not created
        findings, summary = self._run_audit(declaration)
        # REVIEW_REQUIRED becomes WARNING
        self.assertGreater(summary["warnings"], 0, findings)
        self.assertTrue(any(item.code == "QUALITY_REVIEW_REQUIRED" or item.code == "QUALITY_POLICY" for item in findings), findings)

    def test_clean_quality_block_passes(self):
        declaration = {
            "schema_version": 1,
            "id": "demo-project",
            # The project ships `src/lib.rs` and declares `languages: ["rust"]`,
            # so `rust` is the real stack, and `kind` carries the composite that
            # pair derives (derive_kind("product", "rust") == "rust-product")
            # rather than the pre-axes bare role.
            "kind": "rust-product",
            "profile": "product",
            "role": "product",
            "stack": "rust",
            "lifecycle": "active",
            "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
            "quality": {
                "product_roots": ["src"],
                "languages": ["rust"],
                "placeholder_threshold": 0,
            },
        }
        (self.project / "src").mkdir()
        (self.project / "src" / "lib.rs").write_text("pub fn add(a: i32, b: i32) -> i32 { a + b }\n", encoding="utf-8")
        findings, summary = self._run_audit(declaration)
        self.assertEqual(summary["errors"], 0, findings)
        self.assertEqual(summary["warnings"], 0, findings)


if __name__ == "__main__":
    unittest.main()
