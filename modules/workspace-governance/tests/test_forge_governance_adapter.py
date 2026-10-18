"""Tests for `scripts/forge_governance_adapter.py`.

Covers the scenarios in
`openspec/changes/forge-governance-adapter/specs/forge-governance-adapter/spec.md`:
- clean adopted project returns pass and lists WARN findings only as evidence,
- a project with an audit ERROR returns fail with that code first in evidence,
- a registered-but-not-adopted project returns blocked with the adoption state,
- a project that is discovered on disk but not in the registry returns blocked,
- a project that is neither registered nor discovered returns unknown (exit 0),
- malformed stdin exits non-zero with no observation,
- an audit crash exits non-zero and produces no observation,
- evidence lines are capped at 16, truncated at 200 chars, secret-scrubbed,
- the adapter does not write to the governance tree (hash check).
"""

from __future__ import annotations

import copy
import hashlib
import io
import json
import os
import shutil
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))

import workspace_check  # noqa: E402

# Import the adapter from the real scripts/ directory. Tests use synthetic
# registries and workspaces but the adapter module itself is the production
# code.
import forge_governance_adapter as adapter  # noqa: E402

from _decisions_env import restore_decisions_register, use_empty_decisions_register  # noqa: E402


def setUpModule():
    use_empty_decisions_register()


def tearDownModule():
    restore_decisions_register()

from workspace_check import Finding, SECRET_WORDS  # noqa: E402


def _write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def _make_workspace(project_id: str = "demo-project", *, adopted: bool = True) -> tuple[Path, Path]:
    """Build a temp workspace + governance root with a single project.

    The temp dir is BOTH the workspace root and the governance root for the
    adapter's purposes. The adapter reads `projects.json` from
    `governance_root` and uses the same root as the workspace root unless
    a different `workspace_root` is passed in.

    Returns (temp_dir, governance_root). The caller MUST keep `temp_dir`
    alive for the lifetime of the test.
    """
    temp = tempfile.TemporaryDirectory()
    root = Path(temp.name)
    # Governance repo file: projects.json
    project = root / project_id
    project.mkdir()
    declaration = {
        "schema_version": 1,
        "id": project_id,
        "kind": "product",
        "profile": "inventory",
        # Both authority surfaces carry the axes now that the audit is enabled.
        # The declared composite is the bare role `product`, so this pair is
        # the one that derives it (derive_kind("product", "none") ==
        # "product") and leaves the fixture free of taxonomy findings.
        "role": "product",
        "stack": "none",
        "lifecycle": "inventory",
        "verification": {"command": "true", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
        "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
    }
    _write_json(project / ".project.json", declaration)
    registry = {
        "schema_version": 1,
        "workspace_root": None,
        "discovery": {"mode": "explicit", "unregistered_policy": "report"},
        "projects": [
            {
                "id": project_id,
                "path": project_id,
                "profile": "inventory",
                "role": "product",
                "stack": "none",
                "lifecycle": "inventory",
                **({"adoption": "adopted"} if adopted else {"adoption": "pending"}),
            },
        ],
    }
    _write_json(root / "projects.json", registry)
    return temp, root


def _request(project_id: str = "demo-project") -> dict:
    return {
        "contract": "0.1.0",
        "action": "check",
        "project_id": project_id,
        "project_path": project_id,
    }


def _evaluate(request: dict, *, governance_root: Path, workspace_root: Path | None = None):
    if workspace_root is None:
        workspace_root = governance_root
    return adapter.evaluate(request, governance_root=governance_root, workspace_root=workspace_root)


# ---- Request validation ----


class AdapterRequestValidationTests(unittest.TestCase):
    def test_empty_stream_rejected(self):
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request("")

    def test_whitespace_only_stream_rejected(self):
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request("   \n  \n")

    def test_invalid_json_rejected(self):
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request("{not valid")

    def test_non_object_json_rejected(self):
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request("[1,2,3]")

    def test_wrong_contract_rejected(self):
        request = _request()
        request["contract"] = "0.0.9"
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request(json.dumps(request))

    def test_wrong_action_rejected(self):
        request = _request()
        request["action"] = "deploy"
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request(json.dumps(request))

    def test_missing_project_id_rejected(self):
        request = _request()
        del request["project_id"]
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request(json.dumps(request))

    def test_empty_project_id_rejected(self):
        request = _request()
        request["project_id"] = "  "
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request(json.dumps(request))

    def test_missing_project_path_rejected(self):
        request = _request()
        del request["project_path"]
        with self.assertRaises(adapter.AdapterRequestError):
            adapter.load_request(json.dumps(request))

    def test_valid_request_returns_canonicalized_dict(self):
        request = _request()
        canonical = adapter.load_request(json.dumps(request))
        self.assertEqual(canonical["contract"], "0.1.0")
        self.assertEqual(canonical["action"], "check")
        self.assertEqual(canonical["project_id"], "demo-project")
        self.assertEqual(canonical["project_path"], "demo-project")


# ---- Status mapping ----


class AdapterStatusMappingTests(unittest.TestCase):
    def setUp(self) -> None:
        self._temp, self.root = _make_workspace()

    def tearDown(self) -> None:
        self._temp.cleanup()

    def test_clean_adopted_project_returns_pass(self):
        observation = _evaluate(_request(), governance_root=self.root)
        self.assertEqual(observation["status"], "pass")
        self.assertEqual(observation["provider"], "workspace-governance")
        self.assertEqual(observation["protocol_version"], "0.1.0")
        self.assertEqual(observation["project_id"], "demo-project")
        # No findings on a clean inventory project → no evidence.
        self.assertEqual(observation["evidence"], [])

    def test_failing_project_returns_fail_with_error_first(self):
        # Flip the project to deployable without a compose file or Jenkins
        # job — the audit will raise DEPLOYMENT_COMPOSE / DEPLOYMENT_JOB
        # ERRORs that the adapter must surface as `fail`.
        declaration = json.loads((self.root / "demo-project" / ".project.json").read_text())
        declaration["deployment"] = {"deployable": True, "jenkins_job": None, "compose_file": None}
        _write_json(self.root / "demo-project" / ".project.json", declaration)
        observation = _evaluate(_request(), governance_root=self.root)
        self.assertEqual(observation["status"], "fail")
        self.assertTrue(observation["evidence"], "fail observation should carry evidence")
        # First evidence line is an ERROR finding for this project.
        self.assertTrue(observation["evidence"][0].startswith("DEPLOYMENT_"))

    def test_registered_but_not_adopted_returns_blocked(self):
        # Replace the registry with a non-adopted entry; the audit will
        # surface an ADOPTION_PENDING WARNING for this project.
        registry = json.loads((self.root / "projects.json").read_text())
        registry["projects"][0].pop("adoption", None)
        _write_json(self.root / "projects.json", registry)
        observation = _evaluate(_request(), governance_root=self.root)
        self.assertEqual(observation["status"], "blocked")
        self.assertIn("adoption=", observation["detail"])

    def test_discovered_but_unregistered_returns_blocked(self):
        # The governance root is also the workspace root. Replace its
        # registry with an empty one and add a "stray" folder so discovery
        # produces a DISCOVERED_UNREGISTERED finding.
        self.root.joinpath("projects.json").unlink()
        _write_json(self.root / "projects.json", {
            "schema_version": 1,
            "workspace_root": None,
            "discovery": {"mode": "explicit-plus-immediate-directories", "unregistered_policy": "report"},
            "projects": [],
        })
        stray = self.root / "stray"
        stray.mkdir()
        (stray / "README.md").write_text("# stray\n", encoding="utf-8")
        observation = _evaluate(_request("stray"), governance_root=self.root)
        self.assertEqual(observation["status"], "blocked")

    def test_unregistered_project_returns_unknown(self):
        observation = _evaluate(_request("never-registered"), governance_root=self.root)
        self.assertEqual(observation["status"], "unknown")
        self.assertEqual(observation["detail"], "not registered in workspace registry")
        # The audit attributes a PROJECT_UNKNOWN ERROR to the registry; the
        # observation surfaces it so the consumer can see the request was
        # recognized but the project was absent.
        self.assertTrue(
            any("PROJECT_UNKNOWN" in line for line in observation["evidence"]),
            observation,
        )


# ---- Observation shape ----


class AdapterObservationShapeTests(unittest.TestCase):
    def setUp(self) -> None:
        self._temp, self.root = _make_workspace()

    def tearDown(self) -> None:
        self._temp.cleanup()

    def test_observation_metadata_is_well_formed(self):
        observation = _evaluate(_request(), governance_root=self.root)
        self.assertEqual(
            observation["metadata"],
            {"registry_schema_version": 1, "observed_via": "workspace_check"},
        )

    def test_source_revision_omitted_when_project_not_git_repo(self):
        observation = _evaluate(_request(), governance_root=self.root)
        self.assertNotIn("source_revision", observation)

    def test_source_revision_uses_git_head_when_present(self):
        project_path = self.root / "demo-project"
        (project_path / ".git").mkdir()
        (project_path / ".git" / "HEAD").write_text("abc1234567890\n", encoding="utf-8")
        observation = _evaluate(_request(), governance_root=self.root)
        self.assertEqual(observation["source_revision"], "abc1234567890")

    def test_source_revision_follows_symbolic_ref(self):
        project_path = self.root / "demo-project"
        (project_path / ".git").mkdir()
        (project_path / ".git" / "HEAD").write_text("ref: refs/heads/main\n", encoding="utf-8")
        (project_path / ".git" / "refs").mkdir()
        (project_path / ".git" / "refs" / "heads").mkdir()
        (project_path / ".git" / "refs" / "heads" / "main").write_text("feedface\n", encoding="utf-8")
        observation = _evaluate(_request(), governance_root=self.root)
        self.assertEqual(observation["source_revision"], "feedface")

    def test_secret_in_evidence_message_is_scrubbed(self):
        # Build a synthetic finding with a secret-shaped message and verify
        # the rendered evidence scrubs it.
        secret_msg = "deploy token=ABCDEFGHIJKLMNOP api_key=hunter2 used"
        findings = [Finding(level="ERROR", code="SYNTHETIC", message=secret_msg, project="demo-project")]
        evidence = adapter.render_evidence(findings)
        self.assertEqual(len(evidence), 1)
        self.assertIn("[redacted]", evidence[0])
        self.assertNotIn("hunter2", evidence[0])
        self.assertNotIn("ABCDEFGHIJKLMNOP", evidence[0])

    def test_evidence_capped_at_16_lines(self):
        findings = [
            Finding(level="ERROR", code=f"E{i}", message=f"error {i}", project="demo-project")
            for i in range(30)
        ]
        evidence = adapter.render_evidence(findings)
        self.assertEqual(len(evidence), 16)
        # First 16 ERRORs in order
        self.assertEqual(evidence[0], "E0: error 0")
        self.assertEqual(evidence[15], "E15: error 15")

    def test_evidence_orders_errors_before_warnings(self):
        findings = [
            Finding(level="WARNING", code="W_FIRST", message="warning 1", project="demo-project"),
            Finding(level="ERROR", code="E_FIRST", message="error 1", project="demo-project"),
            Finding(level="WARNING", code="W_SECOND", message="warning 2", project="demo-project"),
            Finding(level="ERROR", code="E_SECOND", message="error 2", project="demo-project"),
        ]
        evidence = adapter.render_evidence(findings)
        self.assertEqual(evidence, ["E_FIRST: error 1", "E_SECOND: error 2", "W_FIRST: warning 1", "W_SECOND: warning 2"])

    def test_evidence_line_truncated_to_200_chars(self):
        findings = [Finding(level="ERROR", code="HUGE", message="x" * 500, project="demo-project")]
        evidence = adapter.render_evidence(findings)
        # The original prefix is "HUGE: " (6 chars), then 194 x's, then the
        # ellipsis suffix, so the line is exactly 200 chars.
        self.assertEqual(len(evidence[0]), 200)
        self.assertTrue(evidence[0].endswith("…"))


# ---- main() stdin/stdout behavior ----


class AdapterMainTests(unittest.TestCase):
    def setUp(self) -> None:
        self._temp, self.root = _make_workspace()

    def tearDown(self) -> None:
        self._temp.cleanup()

    def _run_main(self, stdin_text: str) -> tuple[int, str, str]:
        """Run main() with the given stdin and capture stdout + stderr."""
        stdout = io.StringIO()
        stderr = io.StringIO()
        with mock.patch.object(adapter, "_resolve_roots", return_value=(self.root, self.root)), \
                mock.patch.object(sys, "stdin", io.StringIO(stdin_text)), \
                redirect_stdout(stdout), \
                mock.patch.object(sys, "stderr", stderr):
            rc = adapter.main()
        return rc, stdout.getvalue(), stderr.getvalue()

    def test_main_writes_single_observation_document(self):
        rc, stdout, stderr = self._run_main(json.dumps(_request()))
        self.assertEqual(rc, 0)
        self.assertEqual(stderr, "")
        # Exactly one JSON document ending with a newline.
        self.assertTrue(stdout.endswith("\n"))
        self.assertEqual(stdout.count("\n"), 1)
        observation = json.loads(stdout.strip())
        self.assertEqual(observation["status"], "pass")
        self.assertEqual(observation["project_id"], "demo-project")

    def test_main_rejects_malformed_request_with_exit_2(self):
        rc, stdout, stderr = self._run_main("{not json")
        self.assertEqual(rc, 2)
        self.assertEqual(stdout, "")
        self.assertIn("rejected", stderr.lower())

    def test_main_exits_3_when_audit_crashes(self):
        with mock.patch.object(adapter, "run_audit", side_effect=adapter.AuditCrash("boom")):
            stdout = io.StringIO()
            stderr = io.StringIO()
            with mock.patch.object(adapter, "_resolve_roots", return_value=(self.root, self.root)), \
                    mock.patch.object(sys, "stdin", io.StringIO(json.dumps(_request()))), \
                    redirect_stdout(stdout), \
                    mock.patch.object(sys, "stderr", stderr):
                rc = adapter.main()
        self.assertEqual(rc, 3)
        self.assertEqual(stdout.getvalue(), "")
        self.assertIn("audit failure", stderr.getvalue().lower())

    def test_main_exits_0_for_unknown_project(self):
        rc, stdout, stderr = self._run_main(json.dumps(_request("never-here")))
        self.assertEqual(rc, 0)
        self.assertEqual(stderr, "")
        observation = json.loads(stdout.strip())
        self.assertEqual(observation["status"], "unknown")


# ---- Read-only guarantee ----


class AdapterReadOnlyTests(unittest.TestCase):
    def test_adapter_does_not_modify_filesystem(self):
        """Run the adapter against a fixture workspace and verify a
        recursive hash of the tree is identical before and after.
        """
        # Build a temp workspace + governance root via a fresh setup.
        _temp, root = _make_workspace()
        self.addCleanup(_temp.cleanup)
        before = _hash_tree(root)
        # Invoke the full evaluate path twice with two requests; the tree
        # must be byte-identical.
        _evaluate(_request(), governance_root=root)
        _evaluate(_request("never-registered"), governance_root=root)
        after = _hash_tree(root)
        self.assertEqual(before, after, "adapter mutated the workspace tree")


def _hash_tree(root: Path) -> str:
    """Return a SHA-256 of every regular file under `root`, sorted by path."""
    h = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if path.is_file():
            rel = path.relative_to(root).as_posix()
            h.update(rel.encode("utf-8"))
            h.update(b"\0")
            h.update(path.read_bytes())
            h.update(b"\0")
    return h.hexdigest()


if __name__ == "__main__":
    unittest.main()
