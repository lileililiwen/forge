"""Tests for the workspace toolchain decision register (`decisions.json`)."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))

import decisions
from decisions import (
    DECISION_FORMAT_VERSION,
    Decision,
    DecisionError,
    load_decisions,
    validate_against_registry,
)
from workspace_check import (
    DECISIONS_REGISTER_ABSENT,
    DECISIONS_REGISTER_INVALID,
    DECISIONS_REGISTER_MISSING,
    DECISIONS_REGISTRY_REFERENCE,
    load_decisions_register,
)


def write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def base_entry(**overrides) -> dict:
    entry = {
        "id": "D1",
        "title": "Sample decision",
        "status": "accepted",
        "date": "2026-09-30",
        "owner_project": "workspace-governance",
        "decision": "Sample decision text.",
        "downstream": [],
    }
    entry.update(overrides)
    return entry


def write_register(path: Path, decisions_list) -> None:
    payload = {"version": DECISION_FORMAT_VERSION, "decisions": decisions_list}
    write_json(path, payload)


class LoadDecisionsTests(unittest.TestCase):
    def test_valid_register_loads(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(), base_entry(id="D2", title="Another")])
            loaded = load_decisions(path)
            self.assertEqual([d.id for d in loaded], ["D1", "D2"])
            self.assertEqual(loaded[0].title, "Sample decision")
            self.assertEqual(loaded[0].extra_fields, frozenset())

    def test_empty_array_loads(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [])
            self.assertEqual(load_decisions(path), [])

    def test_missing_file_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn(str(path), str(ctx.exception))
            self.assertIn("missing", str(ctx.exception))

    def test_unparseable_json_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            path.write_text("{not json", encoding="utf-8")
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("not valid JSON", str(ctx.exception))

    def test_non_object_register_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            path.write_text(json.dumps([{"id": "D1"}]), encoding="utf-8")
            with self.assertRaises(DecisionError):
                load_decisions(path)

    def test_wrong_version_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_json(path, {"version": 99, "decisions": []})
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("version must be 1", str(ctx.exception))

    def test_unknown_top_level_key_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_json(path, {"version": 1, "decisions": [], "future_field": []})
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("unknown top-level keys", str(ctx.exception))
            self.assertIn("future_field", str(ctx.exception))

    def test_decisions_not_array_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_json(path, {"version": 1, "decisions": {}})
            with self.assertRaises(DecisionError):
                load_decisions(path)


class EntryValidationTests(unittest.TestCase):
    def test_duplicate_id_raises_naming_both(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(), base_entry()])
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("duplicate id", str(ctx.exception))
            self.assertIn("D1", str(ctx.exception))

    def test_bad_status_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(status="ongoing")])
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("status must be one of", str(ctx.exception))

    def test_bad_date_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            for bad in ["2026/09/30", "30-09-2026", "2026-9-30", "tomorrow"]:
                write_register(path, [base_entry(date=bad)])
                with self.assertRaises(DecisionError) as ctx:
                    load_decisions(path)
                self.assertIn("ISO-8601", str(ctx.exception), f"date {bad!r} did not mention ISO-8601")

    def test_bad_id_pattern_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            for bad in ["d1", "D", "D1000", "DX", "D-1", "1"]:
                write_register(path, [base_entry(id=bad)])
                with self.assertRaises(DecisionError) as ctx:
                    load_decisions(path)
                self.assertIn("must match ^D", str(ctx.exception), f"id {bad!r} did not name the pattern")

    def test_missing_required_field_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            payload = {"id": "D1", "title": "x", "status": "accepted", "date": "2026-09-30",
                       "owner_project": "workspace-governance", "decision": "x", "downstream": []}
            for missing in list(payload.keys()):
                partial = {k: v for k, v in payload.items() if k != missing}
                write_register(path, [partial])
                with self.assertRaises(DecisionError) as ctx:
                    load_decisions(path)
                self.assertIn(missing, str(ctx.exception), f"missing {missing!r} not named")

    def test_non_object_entry_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, ["not a dict"])
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("must be a JSON object", str(ctx.exception))

    def test_non_array_downstream_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(downstream={"repo": "x"})])
            with self.assertRaises(DecisionError):
                load_decisions(path)

    def test_downstream_change_kebab_case_enforced(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(downstream=[{"repo": "workspace-governance", "change": "NotKebab"}])])
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("kebab-case", str(ctx.exception))

    def test_downstream_kebab_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(downstream=[{"repo": "workspace-governance", "change": "kebab-case-1"}])])
            loaded = load_decisions(path)
            self.assertEqual(loaded[0].downstream[0]["change"], "kebab-case-1")

    def test_unknown_entry_key_tolerated(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            entry = base_entry()
            entry["future_field"] = "ignored by v1 loader"
            write_register(path, [entry])
            loaded = load_decisions(path)
            self.assertIn("future_field", loaded[0].extra_fields)

    def test_policy_boundary_string_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(policy_boundary="excludes public catalog")])
            self.assertEqual(load_decisions(path)[0].policy_boundary, "excludes public catalog")

    def test_policy_boundary_non_string_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, [base_entry(policy_boundary=42)])
            with self.assertRaises(DecisionError) as ctx:
                load_decisions(path)
            self.assertIn("policy_boundary", str(ctx.exception))


class RegistryValidationTests(unittest.TestCase):
    def _loaded(self, *entries):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.json"
            write_register(path, list(entries))
            return load_decisions(path)

    def test_known_references_pass(self):
        loaded = self._loaded(base_entry(
            owner_project="workspace-governance",
            downstream=[{"repo": "forge", "change": "kebab-case"}],
        ))
        offenders = validate_against_registry(loaded, {"workspace-governance", "forge"})
        self.assertEqual(offenders, [])

    def test_unknown_owner_listed(self):
        loaded = self._loaded(base_entry(owner_project="nonexistent"))
        offenders = validate_against_registry(loaded, {"workspace-governance"})
        self.assertEqual(offenders, ["nonexistent"])

    def test_unknown_downstream_listed(self):
        loaded = self._loaded(base_entry(downstream=[{"repo": "missing-repo", "change": "kebab-case"}]))
        offenders = validate_against_registry(loaded, {"workspace-governance"})
        self.assertEqual(offenders, ["missing-repo"])

    def test_offenders_sorted_unique(self):
        loaded = self._loaded(
            base_entry(id="D1", owner_project="zeta"),
            base_entry(id="D2", owner_project="alpha"),
            base_entry(id="D3", downstream=[{"repo": "alpha", "change": "kebab-case"},
                                              {"repo": "zeta", "change": "kebab-case"}]),
        )
        offenders = validate_against_registry(loaded, {"workspace-governance"})
        self.assertEqual(offenders, ["alpha", "zeta"])


class RealRegisterTests(unittest.TestCase):
    """The seeded entries (D1-D4) must parse, cross-reference the registry, and restate the public-catalog exclusion."""

    @classmethod
    def setUpClass(cls):
        cls.governance_root = Path(__file__).resolve().parents[1]
        cls.path = cls.governance_root / "decisions.json"

    def test_real_register_loads(self):
        loaded = load_decisions(self.path)
        ids = [d.id for d in loaded]
        # D1-D4 are the seeded register and must always be present. Later
        # decisions are legitimate additions, so assert the seeded set is a
        # subset and that ids stay unique and contiguous rather than freezing
        # the register to exactly four entries.
        self.assertTrue({"D1", "D2", "D3", "D4"}.issubset(set(ids)), ids)
        self.assertEqual(len(ids), len(set(ids)), "duplicate decision id")
        self.assertEqual(ids, [f"D{n}" for n in range(1, len(ids) + 1)], ids)

    def test_every_entry_is_accepted(self):
        loaded = load_decisions(self.path)
        for entry in loaded:
            self.assertEqual(entry.status, "accepted", entry.id)
            self.assertEqual(entry.owner_project, "workspace-governance", entry.id)
            self.assertRegex(entry.date, r"^\d{4}-\d{2}-\d{2}$", entry.id)

    def test_seeded_entries_keep_their_original_date(self):
        loaded = load_decisions(self.path)
        for entry in loaded:
            if entry.id in {"D1", "D2", "D3", "D4"}:
                self.assertEqual(entry.date, "2026-09-30", entry.id)

    def test_d2_restates_public_catalog_exclusion(self):
        loaded = load_decisions(self.path)
        d2 = next(d for d in loaded if d.id == "D2")
        self.assertIsNotNone(d2.policy_boundary)
        self.assertIn("public GitHub Pages catalog", d2.policy_boundary)
        self.assertIn("excluded", d2.policy_boundary)

    def test_d3_restates_ownership_split(self):
        loaded = load_decisions(self.path)
        d3 = next(d for d in loaded if d.id == "D3")
        self.assertIn("platform-contracts", d3.policy_boundary)
        self.assertIn("workspace-governance", d3.policy_boundary)

    def test_downstream_references_resolve_in_registry(self):
        loaded = load_decisions(self.path)
        registry_ids = {entry["id"] for entry in json.loads((self.governance_root / "projects.json").read_text())["projects"]}
        offenders = validate_against_registry(loaded, registry_ids)
        self.assertEqual(offenders, [], f"unknown registry references: {offenders}")


class CliSummaryTests(unittest.TestCase):
    """`workspace_check.py --decisions` must print the summary and exit cleanly."""

    repo_root = Path(__file__).resolve().parents[1]
    script_path = repo_root / "scripts" / "workspace_check.py"

    def run_check(self, *args, cwd: Path) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(self.script_path), *args],
            cwd=str(cwd), capture_output=True, text=True, check=False,
        )

    def test_decisions_summary_lists_each_entry(self):
        result = self.run_check("--decisions", cwd=self.repo_root)
        self.assertEqual(result.returncode, 0, result.stderr)
        for token in ("D1", "D2", "D3", "D4", "accepted", "2026-09-30", "Total decisions"):
            self.assertIn(token, result.stdout, f"missing token {token!r} in stdout:\n{result.stdout}")

    def test_decisions_json_shape(self):
        result = self.run_check("--decisions", "--json", cwd=self.repo_root)
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertIn("decisions", payload)
        ids = [item["id"] for item in payload["decisions"]]
        self.assertTrue({"D1", "D2", "D3", "D4"}.issubset(set(ids)), ids)
        self.assertEqual(len(ids), len(set(ids)), "duplicate decision id")
        for item in payload["decisions"]:
            self.assertEqual(set(item.keys()), {"id", "title", "status", "date"})


class LoadRegisterHelperTests(unittest.TestCase):
    """Unit tests for the `load_decisions_register` audit hook."""

    def test_missing_in_self_check_is_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            findings: list = []
            registry = {"schema_version": 1, "projects": []}
            result = load_decisions_register(Path(tmp), registry, findings, self_check=True)
            self.assertIsNone(result)
            self.assertEqual(len(findings), 1)
            self.assertEqual(findings[0].code, DECISIONS_REGISTER_MISSING)
            self.assertEqual(findings[0].level, "ERROR")

    def test_missing_in_portfolio_audit_is_single_warning(self):
        with tempfile.TemporaryDirectory() as tmp:
            findings: list = []
            registry = {"schema_version": 1, "projects": []}
            result = load_decisions_register(Path(tmp), registry, findings, self_check=False)
            self.assertIsNone(result)
            self.assertEqual(len(findings), 1)
            self.assertEqual(findings[0].code, DECISIONS_REGISTER_ABSENT)
            self.assertEqual(findings[0].level, "WARNING")

    def test_broken_shape_is_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_json(root / "decisions.json", {"version": 1, "decisions": [base_entry(), base_entry()]})
            findings: list = []
            registry = {"schema_version": 1, "projects": []}
            result = load_decisions_register(root, registry, findings, self_check=True)
            self.assertIsNone(result)
            self.assertTrue(any(item.code == DECISIONS_REGISTER_INVALID for item in findings))
            self.assertTrue(all(item.level == "ERROR" for item in findings))

    def test_unknown_registry_reference_is_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_json(root / "decisions.json", {
                "version": 1,
                "decisions": [base_entry(owner_project="not-in-registry")],
            })
            findings: list = []
            registry = {"schema_version": 1, "projects": [{"id": "workspace-governance"}]}
            result = load_decisions_register(root, registry, findings, self_check=True)
            self.assertIsNone(result)
            self.assertTrue(any(item.code == DECISIONS_REGISTRY_REFERENCE for item in findings))

    def test_valid_empty_register_is_silent(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_json(root / "decisions.json", {"version": 1, "decisions": []})
            findings: list = []
            registry = {"schema_version": 1, "projects": []}
            result = load_decisions_register(root, registry, findings, self_check=True)
            self.assertEqual(result, [])
            self.assertEqual(findings, [])

    def test_valid_register_returns_decisions(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_json(root / "decisions.json", {
                "version": 1,
                "decisions": [base_entry(), base_entry(id="D2", title="second")],
            })
            findings: list = []
            registry = {"schema_version": 1, "projects": [{"id": "workspace-governance"}]}
            result = load_decisions_register(root, registry, findings, self_check=True)
            self.assertIsNotNone(result)
            self.assertEqual([entry.id for entry in result], ["D1", "D2"])
            self.assertEqual(findings, [])


if __name__ == "__main__":
    unittest.main()
