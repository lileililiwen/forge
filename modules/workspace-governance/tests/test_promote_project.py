"""Tests for `scripts/promote_project.py` and the deployment-promo ledger.

Covers the scenarios in
`openspec/changes/deployable-batch-onboarding/specs/deployable-batch-onboarding/spec.md`:
- clean promotion mutates the declaration and appends a ledger entry,
- missing evidence reference is rejected without side effects,
- conflicting manual declarations are protected without --force,
- --force overrides the conflict guard,
- --demote reverses deployable and records a demoted entry,
- the append-only validator refuses a tampered ledger,
- the audit (workspace_check) surfaces DEPLOYMENT_PROMOTION for hand-edited
  deployable=true and DEPLOYMENT_PROMOTION_STALE for a stale revision,
- the existing deployable+release-evidence gate remains unchanged.
"""

import copy
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import argparse
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))

from workspace_check import run  # noqa: E402
import promote_project  # noqa: E402


# Every top-level governance data file the audit reads is fail-closed: a missing
# one makes `workspace_check` refuse the run, which would mask whatever the
# promote tests are actually asserting. They are copied as ONE declared set, and
# `SandboxCoverageTests` fails if a new one reaches the governance root without
# being placed here or in the absence table below. Hand-maintaining the copy
# list in place broke this fixture twice — `rules-drift-baseline.json` and the
# three `gate-debt*` records each arrived as a new fail-closed file and turned
# seven tests red with an exit code unrelated to what they test.
GOVERNANCE_ROOT_DATA_FILES = (
    "vocabulary.json",
    "capabilities.json",
    "rules-drift-baseline.json",
    "gate-debt.json",
    "gate-debt-baseline.json",
    "gate-debt-exceptions.json",
    "ui-conformance-baseline.json",
    "registry-remote-drift.json",
    "tag-taxonomy.json",
    "ui-adoption-migration.json",
)

# Top-level data files deliberately kept OUT of the sandbox, each with the
# evidence for why its absence is harmless here. Nothing may be added to this
# table on the strength of "the promote path probably does not read it": the
# check is the audit run, and the audit is what reads the file.
GOVERNANCE_ROOT_DATA_FILES_ABSENT = {
    "projects.json": (
        "the fixture writes its own one-project registry into the sandbox "
        "workspace root; the real registry names 81 sibling paths that do not "
        "exist under the temp root"
    ),
    "decisions.json": (
        "a portfolio audit tolerates its absence with a single WARNING "
        "DECISIONS_REGISTER_ABSENT (scripts/workspace_check.py "
        "load_decisions_register); only --self-check requires it, and this "
        "fixture is not a self-check"
    ),
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


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def setup_workspace(governance_root: Path, project_id: str = "demo-project"):
    """Create a temp workspace root and a governance repo that points at it.

    Returns (temp, root). The caller MUST keep `temp` alive for the lifetime
    of the test; the function only stores a Path on the returned object.
    """
    temp = tempfile.TemporaryDirectory()
    root = Path(temp.name)
    # The temp dir IS the workspace root and the governance root for tests
    # (so deployment/promotions.json is rooted here). The promote script picks
    # up the workspace via WORKSPACE_ROOT or via the registry.
    write_json(root / "projects.json", {
        "schema_version": 1,
        "workspace_root": None,
        "discovery": {"mode": "explicit", "unregistered_policy": "report"},
        "projects": [
            {
                "id": project_id,
                "path": project_id,
                "profile": "inventory",
                "lifecycle": "inventory",
                "adoption": "adopted",
            },
        ],
    })
    project = root / project_id
    project.mkdir()
    write_json(project / ".project.json", copy.deepcopy(BASE_DECLARATION))
    # Provide a compose file so promote detects one by default
    (project / "docker-compose.yml").write_text("services: {}\n", encoding="utf-8")
    return temp, root


def setup_sandboxed_governance_root(governance_root: Path):
    """Return (temp, sandbox_root) where sandbox_root is a copy of the
    governance repository rooted at a temp directory. The real governance
    repo's `deployment/promotions.json` is never touched by tests.
    """
    temp = tempfile.TemporaryDirectory()
    sandbox = Path(temp.name)
    # Copy the scripts/ dir; tests only need the modules + the deployment/
    # subdir which will be created on demand.
    (sandbox / "scripts").mkdir()
    for child in (governance_root / "scripts").iterdir():
        if child.name in ("__pycache__",):
            continue
        target = sandbox / "scripts" / child.name
        if child.is_dir():
            shutil.copytree(child, target)
        else:
            shutil.copy2(child, target)
    (sandbox / "deployment").mkdir()
    # The declared data-file set, copied as one unit. A declared file that is
    # absent is a hard error here rather than a silent skip: under `is_file()`
    # guards a file that failed to copy is indistinguishable from a file that
    # was never needed, and the first symptom is an exit code in a test that
    # asserts something else entirely.
    for record in GOVERNANCE_ROOT_DATA_FILES:
        source = governance_root / record
        if not source.is_file():
            raise FileNotFoundError(
                f"{record} is declared load-bearing for the sandbox but is "
                f"absent from {governance_root}"
            )
        shutil.copy2(source, sandbox / record)
    # profiles/ is the one load-bearing directory, and it is optional in shape:
    # a governance root without one is not malformed, so it stays guarded.
    if (governance_root / "profiles").is_dir():
        shutil.copytree(governance_root / "profiles", sandbox / "profiles")
    return temp, sandbox


class SandboxCoverageTests(unittest.TestCase):
    """The sandbox must not fall silently behind the governance root.

    `setup_sandboxed_governance_root` builds a partial replica of the governance
    root, and the audit fails closed on any data file the replica is missing. A
    new fail-closed record therefore breaks the promote tests in a way that
    names neither the new file nor the fixture. These tests make the coupling
    explicit and put the failure where the new file is added.
    """

    _REAL_GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]

    def test_every_governance_root_data_file_is_accounted_for(self):
        # `.project.json` is this repository's own declaration, not a fleet
        # data record, and pathlib's glob matches dotfiles.
        on_disk = {
            p.name
            for p in self._REAL_GOVERNANCE_ROOT.glob("*.json")
            if p.is_file() and not p.name.startswith(".")
        }
        unaccounted = (
            on_disk
            - set(GOVERNANCE_ROOT_DATA_FILES)
            - set(GOVERNANCE_ROOT_DATA_FILES_ABSENT)
        )
        self.assertEqual(
            unaccounted,
            set(),
            "top-level governance data file(s) that are neither copied into "
            "the sandbox nor listed in GOVERNANCE_ROOT_DATA_FILES_ABSENT: "
            f"{sorted(unaccounted)}. The audit is fail-closed on a data file it "
            "cannot read, so leaving one out here turns every promote test red "
            "with an exit code that has nothing to do with what it asserts.",
        )

    def test_the_absence_table_explains_itself(self):
        for name, reason in GOVERNANCE_ROOT_DATA_FILES_ABSENT.items():
            self.assertNotIn(
                name, GOVERNANCE_ROOT_DATA_FILES,
                f"{name} is both copied and declared absent",
            )
            self.assertTrue(
                reason.strip(),
                f"{name} is declared absent with no reason; an unexplained "
                f"absence is indistinguishable from an oversight",
            )

    def test_the_sandbox_actually_carries_the_declared_set(self):
        temp, sandbox = setup_sandboxed_governance_root(self._REAL_GOVERNANCE_ROOT)
        try:
            for record in GOVERNANCE_ROOT_DATA_FILES:
                self.assertTrue(
                    (sandbox / record).is_file(),
                    f"{record} is declared load-bearing but is not in the sandbox",
                )
        finally:
            temp.cleanup()


class _SandboxedGovernanceRootMixin:
    """Mixin that swaps the governance_root used by promote_project for a
    sandbox copy so the real `deployment/promotions.json` is never written.
    """

    _REAL_GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]

    def setUp(self) -> None:
        self._gov_temp, self.governance_root = setup_sandboxed_governance_root(self._REAL_GOVERNANCE_ROOT)
        # Save the original sys.path and sys.modules so tearDown can restore.
        self._original_path = list(sys.path)
        self._original_modules = {name: module for name, module in sys.modules.items() if name in {
            "promote_project", "workspace_check", "product_code_quality",
        }}
        # Remove the real modules so the next import resolves from the sandbox.
        for name in list(self._original_modules):
            sys.modules.pop(name, None)
        # Prepend the sandbox scripts dir so its copies are imported first.
        sys.path.insert(0, str(self.governance_root / "scripts"))
        # Import the sandboxed modules; the import resolves from sys.path.
        import promote_project as _pp  # noqa: PLC0415
        import workspace_check as _wc  # noqa: PLC0415
        # _promote_project and _workspace_check hold the SANDBOXED modules.
        # The same modules live in sys.modules under their canonical names,
        # so internal `import promote_project` calls (e.g. from
        # workspace_check.validate_deployment_promotion) resolve to the
        # sandboxed copy.
        self._promote_project = _pp
        self._workspace_check = _wc

    def tearDown(self) -> None:
        # Restore sys.path and sys.modules to the pre-test state.
        for name in list(self._original_modules):
            sys.modules.pop(name, None)
        for name, module in self._original_modules.items():
            sys.modules[name] = module
        sys.path[:] = self._original_path
        self._gov_temp.cleanup()


class PromoteProjectTests(_SandboxedGovernanceRootMixin, unittest.TestCase):
    def setUp(self) -> None:
        _SandboxedGovernanceRootMixin.setUp(self)
        # Build a synthetic workspace under a temp directory but reuse the
        # governance scripts directory for module imports.
        self._ws_temp, self.workspace = setup_workspace(self.governance_root)
        # Convenience alias to the reloaded module
        self.pp = self._promote_project

    def tearDown(self) -> None:
        self._ws_temp.cleanup()
        _SandboxedGovernanceRootMixin.tearDown(self)

    def _write_release_evidence(self, declaration: dict) -> dict:
        declaration["release_evidence"] = {
            "revision": {"state": "configured", "value": "abc", "evidence_ref": "release-artifacts/revision.txt"},
            "artifacts": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/artifacts.txt"},
            "digests": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/digests.txt"},
            "provenance": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/provenance.json"},
            "checks": {"state": "configured", "value": ["true"], "evidence_ref": "release-artifacts/checks.txt"},
            "publication": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/publication.txt"},
        }
        project_path = self.workspace / "demo-project"
        (project_path / "release-artifacts").mkdir(parents=True, exist_ok=True)
        for filename in ("revision.txt", "artifacts.txt", "digests.txt", "provenance.json", "checks.txt", "publication.txt"):
            (project_path / "release-artifacts" / filename).write_text("placeholder\n", encoding="utf-8")
        return declaration

    # ---- Ledger helpers ----

    def _load_ledger(self) -> dict:
        return self.pp.load_ledger(self.governance_root, allow_create=True)

    def _write_ledger(self, payload: dict) -> None:
        path = self.pp.ledger_path(self.governance_root)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        sidecar = self.pp.ledger_sidecar_path(self.governance_root)
        sidecar.write_text(self.pp.compute_ledger_hash(payload["entries"]), encoding="utf-8")

    def _default_args(self, **overrides):
        defaults = {
            "project_id": "demo-project",
            "evidence_ref": "local:HEAD:abc1234",
            "compose_file": None,
            "jenkins_job": None,
            "actor": "tester",
            "force": False,
            "demote": False,
            "registry": str(self.workspace / "projects.json"),
            "governance_root": self.governance_root,
            "workspace_root": self.workspace,
            "quiet": False,
        }
        defaults.update(overrides)
        return argparse.Namespace(**defaults)

    # ---- Pure helpers ----

    def test_detect_compose_file_finds_docker_compose(self):
        project_path = self.workspace / "demo-project"
        self.assertEqual(
            self.pp.detect_compose_file(project_path),
            "docker-compose.yml",
        )

    def test_detect_compose_file_finds_compose_yml(self):
        project_path = self.workspace / "demo-project"
        (project_path / "docker-compose.yml").unlink()
        (project_path / "compose.yml").write_text("services: {}\n", encoding="utf-8")
        self.assertEqual(self.pp.detect_compose_file(project_path), "compose.yml")

    def test_detect_compose_file_returns_none_when_missing(self):
        project_path = self.workspace / "demo-project"
        (project_path / "docker-compose.yml").unlink()
        self.assertIsNone(self.pp.detect_compose_file(project_path))

    def test_compute_ledger_hash_is_stable(self):
        entries = [{"project_id": "demo", "decided_at": "2026-01-01T00:00:00Z"}]
        self.assertEqual(
            self.pp.compute_ledger_hash(entries),
            self.pp.compute_ledger_hash(entries),
        )
        self.assertNotEqual(
            self.pp.compute_ledger_hash(entries),
            self.pp.compute_ledger_hash(entries + [{"project_id": "x"}]),
        )

    def test_validate_ledger_accepts_missing_file(self):
        # The sidecar should NOT be created just by loading an empty ledger;
        # only writes through append_promotion create it.
        self.pp.load_ledger(self.governance_root, allow_create=True)

    def test_validate_ledger_rejects_tampered_entries(self):
        # First write a clean ledger with one entry
        self._write_ledger({"schema": "promotions/1", "entries": [
            {"project_id": "demo-project", "decided_at": "2026-01-01T00:00:00Z", "actor": "x", "revision": "abc", "evidence_ref": "r", "manifest_generated": True, "kind": "promote"},
        ]})
        # Now overwrite the entries array removing the first entry but keeping
        # the sidecar hash. The validator must raise.
        tampered = {"schema": "promotions/1", "entries": []}
        path = self.pp.ledger_path(self.governance_root)
        path.write_text(json.dumps(tampered, indent=2) + "\n", encoding="utf-8")
        with self.assertRaises(self.pp.LedgerTamperError):
            self.pp.load_ledger(self.governance_root, allow_create=False)

    # ---- main() flow ----

    def test_promote_writes_deployment_block_and_ledger_entry(self):
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        (self.workspace / "demo-project" / ".git").mkdir()
        (self.workspace / "demo-project" / ".git" / "HEAD").write_text("abc1234\n", encoding="utf-8")
        args = self._default_args()
        rc = self.pp.main(args)
        self.assertEqual(rc, 0, "promote should succeed")
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        deployment = declaration["deployment"]
        self.assertTrue(deployment["deployable"])
        self.assertEqual(deployment["compose_file"], "docker-compose.yml")
        self.assertEqual(deployment["jenkins_job"], "demo-project")
        ledger = read_json(self.pp.ledger_path(self.governance_root))
        self.assertEqual(len(ledger["entries"]), 1)
        entry = ledger["entries"][0]
        self.assertEqual(entry["project_id"], "demo-project")
        self.assertEqual(entry["evidence_ref"], "local:HEAD:abc1234")
        self.assertEqual(entry["actor"], "tester")
        self.assertTrue(entry["manifest_generated"])
        self.assertEqual(entry["kind"], "promote")

    def test_promote_refuses_missing_evidence_ref(self):
        args = self._default_args(evidence_ref="")
        rc = self.pp.main(args)
        self.assertEqual(rc, 2)
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self.assertFalse(declaration["deployment"]["deployable"])
        # No ledger entry should be written
        path = self.pp.ledger_path(self.governance_root)
        self.assertFalse(path.exists())

    def test_promote_refuses_conflicting_existing_compose_file(self):
        # Pre-set a different compose file in the declaration
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        declaration["deployment"]["compose_file"] = "infra/prod.yml"
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        args = self._default_args()
        rc = self.pp.main(args)
        self.assertEqual(rc, 2)
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self.assertFalse(declaration["deployment"]["deployable"])
        self.assertEqual(declaration["deployment"]["compose_file"], "infra/prod.yml")

    def test_promote_force_overrides_conflict(self):
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        declaration["deployment"]["compose_file"] = "infra/prod.yml"
        self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        (self.workspace / "demo-project" / ".git").mkdir()
        (self.workspace / "demo-project" / ".git" / "HEAD").write_text("abc1234\n", encoding="utf-8")
        args = self._default_args(force=True)
        rc = self.pp.main(args)
        self.assertEqual(rc, 0, f"promote should succeed with --force; stdout was suppressed")
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self.assertEqual(declaration["deployment"]["compose_file"], "docker-compose.yml")

    def test_promote_refuses_unknown_project_id(self):
        args = self._default_args(project_id="not-registered")
        rc = self.pp.main(args)
        self.assertEqual(rc, 2)

    def test_promote_rolls_back_when_release_evidence_missing(self):
        # Set deployable=true to a project that has no release_evidence block;
        # the post-audit will raise RELEASE_EVIDENCE_REQUIRED and the script
        # must roll the declaration back to the original state.
        args = self._default_args()
        rc = self.pp.main(args)
        # We expect the promotion to roll back because the synthetic project
        # has no release_evidence. rc is non-zero and the deployment block is
        # untouched.
        self.assertNotEqual(rc, 0)
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self.assertFalse(declaration["deployment"]["deployable"])
        # The ledger must not have been appended either: either the file is
        # absent, or the rolled-back content has no entries for this project.
        path = self.pp.ledger_path(self.governance_root)
        if path.exists():
            payload = read_json(path)
            entries = payload.get("entries", [])
            relevant = [entry for entry in entries if entry.get("project_id") == "demo-project"]
            self.assertEqual(relevant, [], "ledger must not contain a promote entry for the rolled-back project")

    def test_demote_records_demoted_entry_and_keeps_history(self):
        # First, give the project a release_evidence block so a promote can
        # actually succeed, then demote.
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        # Also create the .git dir so revision can be detected
        (self.workspace / "demo-project" / ".git").mkdir()
        (self.workspace / "demo-project" / ".git" / "HEAD").write_text("deadbeef\n", encoding="utf-8")

        # Promote
        rc = self.pp.main(self._default_args())
        self.assertEqual(rc, 0)
        # Demote
        rc = self.pp.main(self._default_args(demote=True))
        self.assertEqual(rc, 0)
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self.assertFalse(declaration["deployment"]["deployable"])
        ledger = read_json(self.pp.ledger_path(self.governance_root))
        self.assertEqual(len(ledger["entries"]), 2)
        self.assertEqual(ledger["entries"][1]["kind"], "demote")
        self.assertEqual(ledger["entries"][1]["project_id"], "demo-project")

    def test_promote_is_idempotent_for_same_revision(self):
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        (self.workspace / "demo-project" / ".git").mkdir()
        (self.workspace / "demo-project" / ".git" / "HEAD").write_text("deadbeef\n", encoding="utf-8")

        rc = self.pp.main(self._default_args())
        self.assertEqual(rc, 0)
        first_ledger = read_json(self.pp.ledger_path(self.governance_root))
        # Second invocation with the same revision and evidence_ref must NOT
        # append a duplicate entry.
        rc = self.pp.main(self._default_args())
        self.assertEqual(rc, 0)
        second_ledger = read_json(self.pp.ledger_path(self.governance_root))
        self.assertEqual(len(first_ledger["entries"]), len(second_ledger["entries"]))


class WorkspaceCheckPromotionTests(_SandboxedGovernanceRootMixin, unittest.TestCase):
    def setUp(self) -> None:
        _SandboxedGovernanceRootMixin.setUp(self)
        self._ws_temp, self.workspace = setup_workspace(self.governance_root)

    def tearDown(self) -> None:
        self._ws_temp.cleanup()
        _SandboxedGovernanceRootMixin.tearDown(self)

    def _write_release_evidence(self, declaration: dict) -> dict:
        # Configure the required fields with present evidence_refs so the
        # existing deployable+release-evidence gate passes; the promotion
        # rule is then the only finding we need to assert.
        declaration["release_evidence"] = {
            "revision": {"state": "configured", "value": "abc", "evidence_ref": "release-artifacts/revision.txt"},
            "artifacts": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/artifacts.txt"},
            "digests": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/digests.txt"},
            "provenance": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/provenance.json"},
            "checks": {"state": "configured", "value": ["true"], "evidence_ref": "release-artifacts/checks.txt"},
            "publication": {"state": "configured", "value": "n/a", "evidence_ref": "release-artifacts/publication.txt"},
        }
        # Provide the evidence_ref files so the audit passes the existing rule
        project_path = self.workspace / "demo-project"
        (project_path / "release-artifacts").mkdir(parents=True, exist_ok=True)
        for filename in ("revision.txt", "artifacts.txt", "digests.txt", "provenance.json", "checks.txt", "publication.txt"):
            (project_path / "release-artifacts" / filename).write_text("placeholder\n", encoding="utf-8")
        return declaration

    def _write_ledger_entry(self, project_id: str, revision: str = "abc1234") -> None:
        self._promote_project.append_promotion(
            self.governance_root,
            {
                "project_id": project_id,
                "decided_at": "2026-01-01T00:00:00Z",
                "actor": "tester",
                "revision": revision,
                "evidence_ref": "local:HEAD:abc1234",
                "manifest_generated": True,
                "kind": "promote",
            },
        )

    def test_hand_edited_deployable_without_ledger_entry_fails_audit(self):
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        declaration["deployment"] = {"deployable": True, "jenkins_job": "demo-project", "compose_file": "docker-compose.yml"}
        declaration = self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        findings, summary = self._workspace_check.run(SimpleNamespace(
            registry=str(self.workspace / "projects.json"),
            root=str(self.workspace),
            project="demo-project",
            audit=False,
            self_check=False,
            as_json=False,
        ))
        self.assertGreater(summary["errors"], 0)
        codes = [item.code for item in findings]
        self.assertIn("DEPLOYMENT_PROMOTION", codes)

    def test_deployable_with_ledger_entry_passes_audit(self):
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        declaration["deployment"] = {"deployable": True, "jenkins_job": "demo-project", "compose_file": "docker-compose.yml"}
        declaration = self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        (self.workspace / "demo-project" / ".git").mkdir()
        (self.workspace / "demo-project" / ".git" / "HEAD").write_text("abc1234\n", encoding="utf-8")
        self._write_ledger_entry("demo-project", revision="abc1234")
        findings, summary = self._workspace_check.run(SimpleNamespace(
            registry=str(self.workspace / "projects.json"),
            root=str(self.workspace),
            project="demo-project",
            audit=False,
            self_check=False,
            as_json=False,
        ))
        self.assertEqual(summary["errors"], 0, findings)
        stale = [item for item in findings if item.code == "DEPLOYMENT_PROMOTION_STALE"]
        self.assertEqual(stale, [])

    def test_stale_ledger_entry_warns(self):
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        declaration["deployment"] = {"deployable": True, "jenkins_job": "demo-project", "compose_file": "docker-compose.yml"}
        declaration = self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        (self.workspace / "demo-project" / ".git").mkdir()
        (self.workspace / "demo-project" / ".git" / "HEAD").write_text("newsha\n", encoding="utf-8")
        self._write_ledger_entry("demo-project", revision="oldsha")
        findings, summary = self._workspace_check.run(SimpleNamespace(
            registry=str(self.workspace / "projects.json"),
            root=str(self.workspace),
            project="demo-project",
            audit=False,
            self_check=False,
            as_json=False,
        ))
        self.assertEqual(summary["errors"], 0, findings)
        codes = [item.code for item in findings]
        self.assertIn("DEPLOYMENT_PROMOTION_STALE", codes)

    def test_demote_removes_error_for_subsequent_audit(self):
        # Hand-edit deployable=true; audit reports DEPLOYMENT_PROMOTION.
        declaration = read_json(self.workspace / "demo-project" / ".project.json")
        declaration["deployment"] = {"deployable": True, "jenkins_job": "demo-project", "compose_file": "docker-compose.yml"}
        declaration = self._write_release_evidence(declaration)
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        findings, _ = self._workspace_check.run(SimpleNamespace(
            registry=str(self.workspace / "projects.json"),
            root=str(self.workspace),
            project="demo-project",
            audit=False,
            self_check=False,
            as_json=False,
        ))
        self.assertTrue(any(item.code == "DEPLOYMENT_PROMOTION" for item in findings))

        # Now restore deployable=false. The promotion error should vanish.
        declaration["deployment"]["deployable"] = False
        write_json(self.workspace / "demo-project" / ".project.json", declaration)
        findings, summary = self._workspace_check.run(SimpleNamespace(
            registry=str(self.workspace / "projects.json"),
            root=str(self.workspace),
            project="demo-project",
            audit=False,
            self_check=False,
            as_json=False,
        ))
        self.assertEqual(summary["errors"], 0, findings)
        self.assertFalse(any(item.code == "DEPLOYMENT_PROMOTION" for item in findings))


class PromoteProjectCliTests(_SandboxedGovernanceRootMixin, unittest.TestCase):
    """Smoke-test the CLI form of the script via subprocess."""

    def setUp(self) -> None:
        _SandboxedGovernanceRootMixin.setUp(self)
        self._ws_temp, self.workspace = setup_workspace(self.governance_root)

    def tearDown(self) -> None:
        self._ws_temp.cleanup()
        _SandboxedGovernanceRootMixin.tearDown(self)

    def test_cli_missing_evidence_ref_exits_2(self):
        cmd = [
            sys.executable,
            str(self.governance_root / "scripts" / "promote_project.py"),
            "demo-project",
            "--registry",
            str(self.workspace / "projects.json"),
            "--governance-root",
            str(self.governance_root),
            "--workspace-root",
            str(self.workspace),
            "--actor",
            "tester",
        ]
        env = {**os.environ, "PYTHONPATH": str(self.governance_root / "scripts")}
        result = subprocess.run(cmd, capture_output=True, text=True, env=env)
        self.assertEqual(result.returncode, 2, result.stderr)


if __name__ == "__main__":
    unittest.main()
