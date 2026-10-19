#!/usr/bin/env python3
"""Offline JSON Schema and fixture checks for Forge's maintained subset."""

import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
CONTRACTS = ROOT / "contracts"
EXPECTED_UNSUPPORTED = {
    "platform.identity-subject",
    "platform.permission-decision",
    "platform.tenant-context",
    "platform.job-outcome",
}


class ContractSchemaTests(unittest.TestCase):
    def test_family_catalog_has_supported_and_unsupported_entries(self):
        catalog = json.loads((CONTRACTS / "family-support.json").read_text())
        rows = catalog["families"]
        names = [row["family"] for row in rows]
        self.assertEqual(len(names), len(set(names)))
        unsupported = {row["family"] for row in rows if row["status"] == "unsupported"}
        self.assertEqual(unsupported, EXPECTED_UNSUPPORTED)
        for row in rows:
            if row["status"] == "unsupported":
                self.assertTrue(row.get("reason", "").strip())
                self.assertNotIn("schema", row)
            else:
                self.assertTrue((CONTRACTS / row["schema"]).is_file())
                self.assertTrue(row.get("version"))

    def test_every_supported_family_schema_and_fixture_pair(self):
        catalog = json.loads((CONTRACTS / "family-support.json").read_text())
        supported = [row for row in catalog["families"] if row["status"] == "supported"]
        self.assertEqual(len(supported), 6)
        for row in supported:
            with self.subTest(family=row["family"]):
                schema_path = CONTRACTS / row["schema"]
                schema = json.loads(schema_path.read_text())
                Draft202012Validator.check_schema(schema)
                fixtures = CONTRACTS / "fixtures" / row["family"].removeprefix("platform.")
                valid = json.loads((fixtures / "valid.json").read_text())
                invalid = json.loads((fixtures / "invalid.json").read_text())
                validator = Draft202012Validator(schema)
                self.assertEqual(list(validator.iter_errors(valid)), [])
                self.assertTrue(list(validator.iter_errors(invalid)))

    def test_platform_registry_contains_only_owned_families(self):
        support = json.loads((CONTRACTS / "family-support.json").read_text())
        registry = json.loads((CONTRACTS / "registry.json").read_text())
        expected = {
            row["family"]
            for row in support["families"]
            if row["status"] == "supported" and row["family"] != "platform.public-portfolio-manifest"
        }
        self.assertEqual(set(registry["contracts"]), expected)

    def test_schema_directory_contains_no_unsupported_family_schemas(self):
        catalog = json.loads((CONTRACTS / "family-support.json").read_text())
        expected = {
            row["schema"]
            for row in catalog["families"]
            if row["status"] == "supported"
        } | {"schemas/envelope.schema.json", "schemas/registry.schema.json"}
        actual = {
            path.relative_to(CONTRACTS).as_posix()
            for path in (CONTRACTS / "schemas").glob("*.schema.json")
        }
        self.assertEqual(actual, expected)


if __name__ == "__main__":
    unittest.main(verbosity=2)
