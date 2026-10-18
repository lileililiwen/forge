"""Declared interfaces and boundary findings (`interfaces[]`).

Covers the scenarios in
`openspec/changes/interface-declaration-and-boundary-findings/`:

- the declaration shape: each mechanism, the closed set, the contract
  version, the digest form, the exclusive side flags, and the silent absent
  block
- the boundary findings: `INTERFACE_DECLARATION_INVALID`,
  `INTERFACE_MECHANISM_UNKNOWN`, `INTERFACE_CONTRACT_UNVERSIONED`,
  `INTERFACE_PROVIDER_UNDECLARED`, `INTERFACE_CONSUMED_UNUSED`,
  `INTERFACE_SIBLING_PATH_UNPINNED`, `INTERFACE_VENDORED_WITHOUT_DIGEST`
- the registry reporter: `CAPABILITY_PROVIDER_UNVERIFIED` for every exempt
  entry whose provider declares no verified record
- the recorded WARN -> FAIL ramp, and the read-only promise of the audit
"""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))

from workspace_check import (  # noqa: E402
    CAPABILITY_PROVIDER_UNVERIFIED,
    INTERFACE_CONSUMED_UNUSED,
    INTERFACE_CONTRACT_UNVERSIONED,
    INTERFACE_DECLARATION_INVALID,
    INTERFACE_MECHANISM_UNKNOWN,
    INTERFACE_MECHANISMS,
    INTERFACE_PROVIDER_UNDECLARED,
    INTERFACE_SIBLING_PATH_UNPINNED,
    INTERFACE_VENDORED_WITHOUT_DIGEST,
    build_interface_index,
    interface_escalation_dates,
    interface_finding_level,
    run,
)

from _decisions_env import (  # noqa: E402
    restore_decisions_register,
    use_decisions_register,
    use_empty_decisions_register,
)


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()


PROVIDER = "provider-project"
CONSUMER = "consumer-project"

DECLARATION = {
    "schema_version": 1,
    "id": "project",
    "kind": "python-product",
    "role": "product",
    "stack": "python",
    "profile": "inventory",
    "lifecycle": "inventory",
    "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
    "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
}

DIGEST_OF_TOKENS = "sha256:" + "a" * 64


def digest_of(text: str) -> str:
    return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()


def digest_of_bytes(text: bytes) -> str:
    return "sha256:" + hashlib.sha256(text).hexdigest()


class InterfaceWorkspace:
    """A staged workspace: a provider, a consumer, and the registry that joins them."""

    def __init__(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.registry_path = self.root / "projects.json"
        self.projects: list[dict] = []

    def close(self) -> None:
        self.temp.cleanup()

    def add(self, project_id: str, **declaration) -> Path:
        path = self.root / project_id
        path.mkdir(parents=True, exist_ok=True)
        body = {**DECLARATION, "id": project_id, **declaration}
        self.write_path(path / ".project.json", json.dumps(body, indent=2))
        self.projects.append(
            {
                "id": project_id,
                "path": project_id,
                "profile": body["profile"],
                "role": body["role"],
                "stack": body["stack"],
                "lifecycle": body["lifecycle"],
                "adoption": "adopted",
            }
        )
        return path

    def write(self, project_id: str, relative: str, text: str) -> Path:
        return self.write_path(self.root / project_id / relative, text)

    @staticmethod
    def write_path(path: Path, text: str) -> Path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def write_registry(self) -> Path:
        return self.write_path(
            self.registry_path,
            json.dumps(
                {
                    "schema_version": 1,
                    "workspace_root": None,
                    "discovery": {"mode": "explicit", "unregistered_policy": "fail"},
                    "projects": self.projects,
                }
            ),
        )

    def audit(self) -> list:
        self.write_registry()
        findings, _summary = run(
            SimpleNamespace(
                registry=str(self.registry_path),
                root=str(self.root),
                project=None,
                audit=True,
                self_check=False,
                as_json=False,
            )
        )
        return findings


def codes(findings) -> list[str]:
    return [item.code for item in findings]


def interface_codes(findings) -> list[str]:
    return [item.code for item in findings if item.code.startswith("INTERFACE_")]


# The provider side of every paired mechanism, so a consumer's edge resolves.
PROVIDER_INTERFACES = [
    {
        "id": "gate-result",
        "provides": True,
        "mechanism": "invoke",
        "entry": "driftwatchdog gate --format json",
        "contract": "platform.gate-result/1.0.0",
        "required": False,
    },
    {
        "id": "ui-tokens",
        "provides": True,
        "mechanism": "consume",
        "entry": "@platform/design-tokens",
        "contract": "platform.design-tokens/0.1.0",
        "digest": DIGEST_OF_TOKENS,
        "required": False,
    },
    {
        "id": "ledger",
        "provides": True,
        "mechanism": "observe",
        "entry": "docs/ledger.json",
        "contract": "platform.ledger/1.0.0",
        "digest": digest_of("ledger"),
        "required": False,
    },
    {
        "id": "workspace-check",
        "provides": True,
        "mechanism": "invoke",
        "entry": "scripts/workspace_check.py",
        "contract": "platform.workspace-check/1.0.0",
        "required": False,
    },
    {
        "id": "ui-gate",
        "provides": True,
        "mechanism": "invoke",
        "entry": "scripts/ui_conformance.py",
        "contract": "platform.ui-conformance/1.0.0",
        "required": False,
    },
]

SIBLING_COMMAND = (
    "python3 ../workspace-governance/scripts/workspace_check.py "
    "--root .. --project consumer-project --json"
)

UI_SIBLING_COMMAND = "python3 ../workspace-governance/scripts/ui_conformance.py ."


class DeclaredMechanismsTests(unittest.TestCase):
    """One project declaring a pair of every mechanism, and the one unlawful edge."""

    def setUp(self):
        self.workspace = InterfaceWorkspace()
        self.addCleanup(self.workspace.close)
        self.workspace.add(PROVIDER, interfaces=PROVIDER_INTERFACES)
        self.consumer = self.workspace.add(
            CONSUMER,
            interfaces=[
                {
                    "id": "gate-result",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                    "required": True,
                    "evidence_ref": "docs/use.md",
                },
                {
                    "id": "ui-tokens",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "consume",
                    "entry": "@platform/design-tokens",
                    "contract": "platform.design-tokens/0.1.0",
                    "digest": DIGEST_OF_TOKENS,
                    "required": False,
                    "evidence_ref": "docs/use.md",
                },
                {
                    "id": "ledger",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "observe",
                    "entry": "docs/ledger.json",
                    "contract": "platform.ledger/1.0.0",
                    "digest": digest_of("ledger"),
                    "required": False,
                    "evidence_ref": "docs/use.md",
                },
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": SIBLING_COMMAND,
                    "digest": digest_of(SIBLING_COMMAND),
                    "required": True,
                    "evidence_ref": "docs/use.md",
                },
                {
                    "id": "ui-gate",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": UI_SIBLING_COMMAND,
                    "digest": digest_of(UI_SIBLING_COMMAND),
                    "required": True,
                    "evidence_ref": "docs/use.md",
                },
            ],
        )
        self.workspace.write(CONSUMER, "docs/use.md", "this project uses these edges\n")

    def test_every_lawful_mechanism_is_accepted(self):
        findings = self.workspace.audit()
        self.assertEqual(interface_codes(findings), [], findings)
        self.assertEqual(findings, [], findings)

    def test_the_unlawful_sibling_edge_alone_is_reported(self):
        declaration = json.loads((self.consumer / ".project.json").read_text(encoding="utf-8"))
        for item in declaration["interfaces"]:
            if item["id"] == "ui-gate":
                item.pop("digest")
                item["required"] = False
        self.workspace.write_path(
            self.consumer / ".project.json", json.dumps(declaration, indent=2)
        )
        findings = self.workspace.audit()
        self.assertEqual(interface_codes(findings), [INTERFACE_SIBLING_PATH_UNPINNED], findings)
        message = findings[0].message
        self.assertIn("ui-gate", message)
        self.assertEqual(findings[0].level, "WARNING")
        self.assertIn("`required: true`", message)
        self.assertIn("digest", message)
        # The remediation names both lawful alternatives, not just the defect.
        self.assertIn("mechanism: consume", message)
        self.assertIn("publish the artifact", message)

    def test_nothing_declared_is_silent(self):
        self.workspace.add("quiet-project")
        findings = self.workspace.audit()
        self.assertEqual(interface_codes(findings), [], findings)

    def test_a_one_sided_edge_is_not_treated_as_adopted(self):
        provider_declaration = json.loads(
            (self.workspace.root / PROVIDER / ".project.json").read_text(encoding="utf-8")
        )
        provider_declaration["interfaces"] = [
            item for item in provider_declaration["interfaces"] if item["id"] != "ledger"
        ]
        self.workspace.write_path(
            self.workspace.root / PROVIDER / ".project.json",
            json.dumps(provider_declaration, indent=2),
        )
        findings = self.workspace.audit()
        self.assertEqual(interface_codes(findings), [INTERFACE_PROVIDER_UNDECLARED], findings)
        self.assertIn(PROVIDER, findings[0].message)
        self.assertIn("ledger", findings[0].message)

    def test_a_provider_outside_the_registry_is_reported_as_undeclared(self):
        declaration = json.loads((self.consumer / ".project.json").read_text(encoding="utf-8"))
        for item in declaration["interfaces"]:
            item["provider"] = "unregistered-provider"
        self.workspace.write_path(
            self.consumer / ".project.json", json.dumps(declaration, indent=2)
        )
        findings = self.workspace.audit()
        self.assertEqual(
            sorted(interface_codes(findings)),
            sorted([INTERFACE_PROVIDER_UNDECLARED] * 5),
            findings,
        )

    def test_the_declared_interface_ids_are_indexed_for_the_whole_registry(self):
        self.workspace.write_registry()
        index = build_interface_index(self.workspace.root, self.workspace.projects)
        self.assertEqual(
            sorted(key for key in index if key[0] == PROVIDER),
            [(PROVIDER, item["id"]) for item in sorted(PROVIDER_INTERFACES, key=lambda i: i["id"])],
        )
        self.assertNotIn((CONSUMER, "gate-result"), index)


class MalformedDeclarationTests(unittest.TestCase):
    """A malformed declaration yields no interface assertions at all."""

    def audit(self, interfaces, **extra) -> list:
        workspace = InterfaceWorkspace()
        self.addCleanup(workspace.close)
        workspace.add(PROVIDER, interfaces=PROVIDER_INTERFACES)
        workspace.add(CONSUMER, interfaces=interfaces, **extra)
        return workspace.audit()

    def test_both_side_flags_is_one_finding_and_no_partial_read(self):
        findings = self.audit(
            [
                {
                    "id": "both-sides",
                    "provides": True,
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": "python3 ../workspace-provider/scripts/x.py",
                    "required": False,
                },
                {
                    "id": "unlawful",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": "python3 ../provider-project/scripts/ui_conformance.py .",
                    "required": False,
                    "evidence_ref": "docs/use.md",
                },
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)
        self.assertIn("exactly one of provides/consumes", findings[0].message)

    def test_neither_side_flag_is_refused(self):
        findings = self.audit(
            [
                {
                    "id": "no-side",
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)

    def test_a_duplicate_id_is_refused(self):
        entry = {
            "id": "twice",
            "provides": True,
            "mechanism": "invoke",
            "entry": "driftwatchdog gate --format json",
            "contract": "platform.gate-result/1.0.0",
        }
        findings = self.audit([entry, dict(entry)])
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)
        self.assertIn("unique", findings[0].message)

    def test_a_missing_provider_on_a_consumption_is_refused(self):
        findings = self.audit(
            [
                {
                    "id": "no-provider",
                    "consumes": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                    "evidence_ref": "docs/use.md",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)
        self.assertIn("provider", findings[0].message)

    def test_a_missing_digest_on_a_consumption_or_observation_is_refused(self):
        for mechanism, extra in (
            ("consume", {"entry": "@platform/design-tokens", "contract": "platform.design-tokens/0.1.0"}),
            ("observe", {"entry": "docs/ledger.json", "contract": "platform.ledger/1.0.0"}),
        ):
            with self.subTest(mechanism=mechanism):
                findings = self.audit(
                    [
                        {
                            "id": "undigested",
                            "consumes": True,
                            "provider": PROVIDER,
                            "mechanism": mechanism,
                            "required": False,
                            "evidence_ref": "docs/use.md",
                            **extra,
                        }
                    ]
                )
                self.assertEqual(
                    interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings
                )
                self.assertIn("digest", findings[0].message)

    def test_a_bad_digest_form_is_refused(self):
        findings = self.audit(
            [
                {
                    "id": "bad-digest",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "consume",
                    "entry": "@platform/design-tokens",
                    "contract": "platform.design-tokens/0.1.0",
                    "digest": "sha256:not-hex",
                    "required": False,
                    "evidence_ref": "docs/use.md",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)

    def test_an_absolute_entry_and_an_escaping_observation_are_refused(self):
        for entry in ("/opt/check.py .",):
            with self.subTest(entry=entry):
                findings = self.audit(
                    [
                        {
                            "id": "absolute",
                            "provides": True,
                            "mechanism": "invoke",
                            "entry": entry,
                            "contract": "platform.gate-result/1.0.0",
                        }
                    ]
                )
                self.assertEqual(
                    interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings
                )
        findings = self.audit(
            [
                {
                    "id": "escaping",
                    "provides": True,
                    "mechanism": "observe",
                    "entry": "../../outside/ledger.json",
                    "contract": "platform.ledger/1.0.0",
                    "digest": digest_of("ledger"),
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)
        self.assertIn("repository-relative", findings[0].message)

    def test_an_unknown_key_is_refused(self):
        findings = self.audit(
            [
                {
                    "id": "extra-key",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                    "version": "1.0.0",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)
        self.assertIn("unknown key", findings[0].message)

    def test_a_non_array_interfaces_value_is_refused(self):
        findings = self.audit("not-an-array")
        self.assertEqual(interface_codes(findings), [INTERFACE_DECLARATION_INVALID], findings)


class AssertedRuleTests(unittest.TestCase):
    """The two rules the specification names a distinct code for."""

    def audit(self, interfaces, provider_interfaces=None) -> list:
        workspace = InterfaceWorkspace()
        self.addCleanup(workspace.close)
        workspace.add(PROVIDER, interfaces=provider_interfaces or PROVIDER_INTERFACES)
        workspace.add(CONSUMER, interfaces=interfaces)
        workspace.write(CONSUMER, "docs/use.md", "use\n")
        return workspace.audit()

    def test_an_unknown_mechanism_is_named_and_the_entry_is_otherwise_ignored(self):
        findings = self.audit(
            [
                {
                    "id": "webhook",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "webhook",
                    "entry": "https://example.invalid/hook",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_MECHANISM_UNKNOWN], findings)
        self.assertEqual(findings[0].level, "ERROR")
        self.assertIn("webhook", findings[0].message)
        for mechanism in INTERFACE_MECHANISMS:
            self.assertIn(mechanism, findings[0].message)

    def test_a_contract_without_a_version_is_named(self):
        findings = self.audit(
            [
                {
                    "id": "gate-result",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result",
                    "evidence_ref": "docs/use.md",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_CONTRACT_UNVERSIONED], findings)
        self.assertEqual(findings[0].level, "ERROR")
        self.assertIn("<name>/<major>.<minor>.<patch>", findings[0].message)

    def test_a_contract_with_a_version_is_accepted(self):
        findings = self.audit(
            [
                {
                    "id": "gate-result",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [], findings)

    def test_a_sibling_digest_that_does_not_match_the_invocation_is_unpinned(self):
        findings = self.audit(
            [
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": SIBLING_COMMAND,
                    "digest": digest_of(SIBLING_COMMAND + " "),
                    "required": True,
                    "evidence_ref": "docs/use.md",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_SIBLING_PATH_UNPINNED], findings)
        self.assertIn("matches the declared invocation", findings[0].message)

    def test_a_sibling_edge_without_required_is_unpinned(self):
        findings = self.audit(
            [
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": SIBLING_COMMAND,
                    "digest": digest_of(SIBLING_COMMAND),
                    "required": False,
                    "evidence_ref": "docs/use.md",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_SIBLING_PATH_UNPINNED], findings)
        self.assertIn("baseline", findings[0].message)

    def test_a_consumption_without_its_own_evidence_is_unused(self):
        findings = self.audit(
            [
                {
                    "id": "gate-result",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_CONSUMED_UNUSED], findings)
        self.assertIn("evidence_ref", findings[0].message)

    def test_an_evidence_ref_that_does_not_resolve_is_unused(self):
        findings = self.audit(
            [
                {
                    "id": "gate-result",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                    "evidence_ref": "docs/missing.md",
                }
            ]
        )
        self.assertEqual(interface_codes(findings), [INTERFACE_CONSUMED_UNUSED], findings)


class VendoredCopyTests(unittest.TestCase):
    """A retained byte copy is lawful only when its digest is the retained bytes."""

    RETAINED = '{"retained": true}\n'

    def audit(self, digest: str) -> list:
        workspace = InterfaceWorkspace()
        self.addCleanup(workspace.close)
        workspace.add(
            PROVIDER,
            interfaces=[
                *PROVIDER_INTERFACES,
                {
                    "id": "retained-checker",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "scripts/workspace_check.py",
                    "contract": "platform.workspace-check/1.0.0",
                    "required": False,
                },
            ],
        )
        workspace.add(
            CONSUMER,
            interfaces=[
                {
                    "id": "retained-checker",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "consume",
                    "entry": "scripts/governance/retained.py",
                    "contract": "platform.workspace-check/1.0.0",
                    "digest": digest,
                    "required": False,
                    "evidence_ref": "docs/use.md",
                }
            ],
        )
        workspace.write(CONSUMER, "docs/use.md", "use\n")
        workspace.write(CONSUMER, "scripts/governance/retained.py", self.RETAINED)
        return workspace.audit()

    def test_a_copy_pinned_to_its_own_bytes_is_lawful(self):
        findings = self.audit(digest_of_bytes(self.RETAINED.encode("utf-8")))
        self.assertEqual(interface_codes(findings), [], findings)

    def test_a_copy_whose_digest_is_not_its_bytes_is_reported(self):
        findings = self.audit("sha256:" + "b" * 64)
        self.assertEqual(interface_codes(findings), [INTERFACE_VENDORED_WITHOUT_DIGEST], findings)
        self.assertEqual(findings[0].level, "WARNING")
        self.assertIn("scripts/governance/retained.py", findings[0].message)
        self.assertIn(PROVIDER, findings[0].message)
        self.assertIn(digest_of_bytes(self.RETAINED.encode("utf-8")), findings[0].message)

    def test_a_published_coordinate_is_not_a_retained_copy(self):
        # `@platform/design-tokens` names no file here, and the audit fetches
        # nothing: a coordinate the repository does not hold cannot be a copy
        # of it, so the digest is not re-derived against anything and the
        # declaration is taken at its word.
        findings = self.audit_for(
            {
                "id": "ui-tokens",
                "consumes": True,
                "provider": PROVIDER,
                "mechanism": "consume",
                "entry": "@platform/design-tokens",
                "contract": "platform.design-tokens/0.1.0",
                "digest": "sha256:" + "b" * 64,
                "required": False,
                "evidence_ref": "docs/use.md",
            }
        )
        self.assertEqual(interface_codes(findings), [], findings)

    def audit_for(self, interface: dict) -> list:
        workspace = InterfaceWorkspace()
        self.addCleanup(workspace.close)
        workspace.add(PROVIDER, interfaces=PROVIDER_INTERFACES)
        workspace.add(CONSUMER, interfaces=[interface])
        workspace.write(CONSUMER, "docs/use.md", "use\n")
        return workspace.audit()


class RampTests(unittest.TestCase):
    """The escalation is a recorded owner decision, read from `decisions.json`."""

    def setUp(self):
        self.workspace = InterfaceWorkspace()
        self.addCleanup(self.workspace.close)
        self.workspace.add(PROVIDER, interfaces=PROVIDER_INTERFACES)
        self.workspace.add(
            CONSUMER,
            interfaces=[
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": SIBLING_COMMAND,
                    "required": False,
                    "evidence_ref": "docs/use.md",
                }
            ],
        )
        self.workspace.write(CONSUMER, "docs/use.md", "use\n")
        use_empty_decisions_register()

    @staticmethod
    def register(effective: str) -> dict:
        return {
            "version": 1,
            "decisions": [
                {
                    "id": "D1",
                    "title": "Ramp",
                    "status": "accepted",
                    "date": "2026-10-17",
                    "owner_project": "workspace-governance",
                    "decision": "Ramp the finding.",
                    "escalates": {
                        "finding_codes": [INTERFACE_SIBLING_PATH_UNPINNED],
                        "effective": effective,
                    },
                    "downstream": [],
                }
            ],
        }

    def test_before_the_recorded_date_the_finding_is_a_warning(self):
        use_decisions_register(self.register("2999-01-01"))
        findings = self.workspace.audit()
        matched = [item for item in findings if item.code == INTERFACE_SIBLING_PATH_UNPINNED]
        self.assertEqual(len(matched), 1, findings)
        self.assertEqual(matched[0].level, "WARNING")

    def test_after_the_recorded_date_the_finding_blocks(self):
        use_decisions_register(self.register("2000-01-01"))
        findings = self.workspace.audit()
        matched = [item for item in findings if item.code == INTERFACE_SIBLING_PATH_UNPINNED]
        self.assertEqual(len(matched), 1, findings)
        self.assertEqual(matched[0].level, "ERROR")
        self.assertIn("mechanism: consume", matched[0].message)

    def test_without_a_recorded_ramp_the_finding_stays_a_warning(self):
        use_empty_decisions_register()
        findings = self.workspace.audit()
        matched = [item for item in findings if item.code == INTERFACE_SIBLING_PATH_UNPINNED]
        self.assertEqual(matched[0].level, "WARNING")

    def test_a_malformed_ramp_record_never_escalates(self):
        payload = self.register("2000-01-01")
        payload["decisions"][0]["escalates"] = {"finding_codes": "nonsense", "effective": "soon"}
        use_decisions_register(payload)
        self.assertEqual(interface_escalation_dates(Path(".")), {})
        self.assertEqual(
            interface_finding_level(INTERFACE_SIBLING_PATH_UNPINNED, {}, None), "WARNING"
        )

    def test_the_shipped_register_records_the_two_ramped_codes(self):
        restore_decisions_register()
        dates = interface_escalation_dates(Path(__file__).parents[1])
        self.assertEqual(
            sorted(dates),
            sorted([INTERFACE_SIBLING_PATH_UNPINNED, INTERFACE_VENDORED_WITHOUT_DIGEST]),
        )
        self.assertEqual(len(set(dates.values())), 1, dates)
        use_empty_decisions_register()


class ProviderUnverifiedTests(unittest.TestCase):
    """Every exempt registry entry reports its unverified provider."""

    def audit_with_provider(self, provider_declaration: dict | None) -> list:
        workspace = InterfaceWorkspace()
        self.addCleanup(workspace.close)
        workspace.add("dotnet-platform-libs", **(provider_declaration or {}))
        workspace.add(CONSUMER)
        return workspace.audit()

    def test_the_shipped_registry_reports_every_exempt_entry(self):
        import workspace_check

        registry = workspace_check.load_capability_registry(
            Path(__file__).parents[1], []
        )
        self.assertIsNotNone(registry)
        exempt = [
            name
            for name, entry in registry.capabilities.items()
            if entry.get("posture") == "exempt"
        ]
        self.assertEqual(len(exempt), 9, exempt)
        findings = self.audit_with_provider(None)
        matched = [item for item in findings if item.code == CAPABILITY_PROVIDER_UNVERIFIED]
        self.assertEqual(len(matched), 9, codes(findings))
        self.assertEqual({item.project for item in matched}, {"dotnet-platform-libs"})
        self.assertEqual({item.level for item in matched}, {"WARNING"})
        cited = {item.message.split("'")[1] for item in matched}
        self.assertEqual(sorted(cited), sorted(exempt))
        for name in exempt:
            message = next(item.message for item in matched if f"'{name}'" in item.message)
            self.assertIn("dotnet-platform-libs", message)
            self.assertIn("verified", message)

    def test_the_exemption_semantics_are_untouched(self):
        import workspace_check

        findings = self.audit_with_provider(None)
        self.assertEqual(
            [item.code for item in findings if item.code.startswith("CAPABILITY_ADOPTION")],
            [],
        )
        registry = workspace_check.load_capability_registry(Path(__file__).parents[1], [])
        for name, entry in registry.capabilities.items():
            self.assertEqual(entry.get("posture"), "exempt", name)

    def test_a_verified_provider_record_stops_the_reporter(self):
        declaration = {
            "capabilities": {
                name: {
                    "owner": "platform",
                    "evidence_state": "verified",
                    "evidence_ref": "docs/evidence.md",
                }
                for name in self.shipped_exempt_ids()
            }
        }
        workspace = InterfaceWorkspace()
        self.addCleanup(workspace.close)
        provider = workspace.add("dotnet-platform-libs", **declaration)
        workspace.add(CONSUMER)
        workspace.write("dotnet-platform-libs", "docs/evidence.md", "evidence\n")
        findings = workspace.audit()
        self.assertEqual(
            [item.code for item in findings if item.code == CAPABILITY_PROVIDER_UNVERIFIED],
            [],
            findings,
        )
        self.assertTrue(provider.is_dir())

    @staticmethod
    def shipped_exempt_ids() -> list[str]:
        import workspace_check

        registry = workspace_check.load_capability_registry(Path(__file__).parents[1], [])
        return sorted(registry.capabilities)


class ReadOnlyTests(unittest.TestCase):
    """The audit must not write anything, not even the declaration it reads."""

    def test_the_tree_is_unchanged_after_the_audit(self):
        workspace = InterfaceWorkspace()
        self.addCleanup(workspace.close)
        workspace.add(PROVIDER, interfaces=PROVIDER_INTERFACES)
        workspace.add(
            CONSUMER,
            interfaces=[
                {
                    "id": "ui-gate",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": "python3 ../workspace-governance/scripts/ui_conformance.py .",
                    "required": False,
                    "evidence_ref": "docs/use.md",
                }
            ],
        )
        workspace.write(CONSUMER, "docs/use.md", "use\n")
        workspace.write_registry()
        before = {
            path.relative_to(workspace.root): path.read_bytes()
            for path in workspace.root.rglob("*")
            if path.is_file()
        }
        workspace.audit()
        after = {
            path.relative_to(workspace.root): path.read_bytes()
            for path in workspace.root.rglob("*")
            if path.is_file()
        }
        self.assertEqual(after, before, "the audit must be read-only")


class SchemaFixtureTests(unittest.TestCase):
    """The declared contract in `schemas/project.schema.json`.

    The audit validates in Python and never loads the JSON Schema, so these
    fixtures pin the *declared* contract. They skip cleanly where
    `jsonschema` is not installed, which keeps the stdlib-only promise intact.
    """

    @classmethod
    def setUpClass(cls):
        try:
            import jsonschema
        except ImportError:  # pragma: no cover - CI installs no packages
            raise unittest.SkipTest("jsonschema is not installed; schema fixtures are skipped")
        cls.schema = json.loads(
            (Path(__file__).parents[1] / "schemas" / "project.schema.json").read_text(
                encoding="utf-8"
            )
        )
        jsonschema.Draft202012Validator.check_schema(cls.schema)
        cls.validator = jsonschema.Draft202012Validator(cls.schema)

    def declaration(self, **extra) -> dict:
        return {**DECLARATION, "id": "demo-project", **extra}

    def valid(self, interfaces) -> None:
        payload = self.declaration(interfaces=interfaces)
        self.assertEqual(list(self.validator.iter_errors(payload)), [])

    def invalid(self, interfaces) -> None:
        payload = self.declaration(interfaces=interfaces)
        self.assertTrue(list(self.validator.iter_errors(payload)))

    def test_every_mechanism_is_accepted(self):
        self.valid(
            [
                {
                    "id": "gate-result",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                },
                {
                    "id": "ui-tokens",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "consume",
                    "entry": "@platform/design-tokens",
                    "contract": "platform.design-tokens/0.1.0",
                    "digest": DIGEST_OF_TOKENS,
                    "evidence_ref": "docs/use.md",
                    "required": False,
                },
                {
                    "id": "ledger",
                    "provides": True,
                    "mechanism": "observe",
                    "entry": "docs/ledger.json",
                    "contract": "platform.ledger/1.0.0",
                    "digest": digest_of("ledger"),
                },
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": SIBLING_COMMAND,
                    "digest": digest_of(SIBLING_COMMAND),
                    "required": True,
                    "evidence_ref": "docs/use.md",
                },
            ]
        )

    def test_an_absent_block_is_valid(self):
        payload = self.declaration()
        self.assertEqual(list(self.validator.iter_errors(payload)), [])

    def test_an_unknown_mechanism_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "webhook",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "webhook",
                    "entry": "https://example.invalid/hook",
                    "contract": "platform.hook/1.0.0",
                    "digest": DIGEST_OF_TOKENS,
                }
            ]
        )

    def test_a_contract_without_a_version_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "gate-result",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result",
                }
            ]
        )

    def test_a_sibling_without_a_digest_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": SIBLING_COMMAND,
                    "required": True,
                }
            ]
        )

    def test_a_non_exclusive_entry_is_rejected(self):
        for flags in (
            {"provides": True, "consumes": True},
            {"provides": False, "consumes": False},
            {},
        ):
            with self.subTest(flags=flags):
                self.invalid(
                    [
                        {
                            "id": "both-sides",
                            "mechanism": "invoke",
                            "entry": "driftwatchdog gate --format json",
                            "contract": "platform.gate-result/1.0.0",
                            **flags,
                        }
                    ]
                )

    def test_a_consumption_without_a_provider_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "gate-result",
                    "consumes": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                }
            ]
        )

    def test_a_bad_digest_form_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "ui-tokens",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "consume",
                    "entry": "@platform/design-tokens",
                    "contract": "platform.design-tokens/0.1.0",
                    "digest": "sha256:zzz",
                }
            ]
        )

    def test_a_non_kebab_case_id_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "Gate_Result",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                }
            ]
        )

    def test_an_unknown_key_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "gate-result",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "driftwatchdog gate --format json",
                    "contract": "platform.gate-result/1.0.0",
                    "version": "1.0.0",
                }
            ]
        )

    def test_an_absolute_entry_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "gate-result",
                    "provides": True,
                    "mechanism": "invoke",
                    "entry": "/opt/check.py",
                    "contract": "platform.gate-result/1.0.0",
                }
            ]
        )

    def test_an_escaping_observation_is_rejected(self):
        self.invalid(
            [
                {
                    "id": "ledger",
                    "provides": True,
                    "mechanism": "observe",
                    "entry": "../../outside/ledger.json",
                    "contract": "platform.ledger/1.0.0",
                    "digest": digest_of("ledger"),
                }
            ]
        )

    def test_a_sibling_entry_may_escape_to_the_workspace_root(self):
        # The one mechanism whose vocabulary is a relative invocation of
        # another checkout: `../<provider>/…` is the edge, and the pinning
        # rule — digest plus `required: true` — is what makes it lawful.
        self.valid(
            [
                {
                    "id": "workspace-check",
                    "consumes": True,
                    "provider": PROVIDER,
                    "mechanism": "sibling",
                    "entry": SIBLING_COMMAND,
                    "digest": digest_of(SIBLING_COMMAND),
                    "required": True,
                }
            ]
        )


if __name__ == "__main__":
    unittest.main()
