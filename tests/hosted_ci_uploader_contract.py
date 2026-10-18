import contextlib
import io
import json
import os
import runpy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit


ROOT = Path(__file__).resolve().parents[1]
SERVICE = ROOT / "standards/baseline-service/1.6.0/upload-hosted-ci-evidence.py"
CLIENT = ROOT / "standards/baseline-client/1.3.0/upload-hosted-ci-evidence.py"


class Response:
    status = 201

    def __init__(self, body):
        self.body = body

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        return False

    def read(self, _limit):
        return self.body


class Opener:
    def __init__(self):
        self.requests = []

    def open(self, request, timeout):
        self.requests.append((request, timeout))
        if "pipelines.actions.githubusercontent.com" in request.full_url:
            return Response(b'{"value":"secret-id-token"}')
        return Response(b'{"id":"hci-test-123","duplicate":false}')


class HostedCiUploaderContract(unittest.TestCase):
    def test_service_and_client_snapshots_use_identical_uploader(self):
        self.assertEqual(SERVICE.read_bytes(), CLIENT.read_bytes())

    def test_upload_uses_fixed_audience_and_does_not_print_token(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            evidence_dir = root / ".forge/gate/sample-service"
            evidence_dir.mkdir(parents=True)
            (evidence_dir / "evidence.json").write_text(json.dumps({
                "project_id": "sample-service",
                "revision": "a" * 40,
                "aggregate": "passed",
                "profile": "full",
                "dry_run": False,
            }))
            values = {
                "FORGE_CI_EVIDENCE_URL": "https://forge.example.test/v1/ci/github/evidence",
                "GITHUB_SHA": "a" * 40,
                "GITHUB_REPOSITORY": "owner/repository",
                "GITHUB_RUN_ID": "123456789",
                "GITHUB_RUN_ATTEMPT": "1",
                "GITHUB_SERVER_URL": "https://github.com",
                "GITHUB_WORKFLOW_REF": "owner/repository/.github/workflows/forge-gate.yml@refs/heads/main",
                "GITHUB_JOB": "upload-hosted-evidence",
                "ACTIONS_ID_TOKEN_REQUEST_URL": "https://pipelines.actions.githubusercontent.com/oidc?existing=1&audience=wrong",
                "ACTIONS_ID_TOKEN_REQUEST_TOKEN": "runner-request-token",
            }
            opener = Opener()
            stdout = io.StringIO()
            stderr = io.StringIO()
            with patch.dict(os.environ, values, clear=True), patch("urllib.request.build_opener", return_value=opener), patch("time.sleep"), contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                with contextlib.chdir(root):
                    runpy.run_path(str(SERVICE), run_name="hosted_ci_uploader_test") ["main"]()

            self.assertEqual(len(opener.requests), 2)
            token_request, _ = opener.requests[0]
            self.assertEqual(parse_qs(urlsplit(token_request.full_url).query)["audience"], ["forge-ci-evidence"])
            upload_request, timeout = opener.requests[1]
            self.assertEqual(upload_request.get_header("Authorization"), "Bearer secret-id-token")
            self.assertEqual(timeout, 10)
            document = json.loads(upload_request.data)
            self.assertEqual(document["project_id"], "sample-service")
            self.assertEqual(document["source_revision"], "a" * 40)
            self.assertIn("hci-test-123", stdout.getvalue())
            for secret in ("secret-id-token", "runner-request-token"):
                self.assertNotIn(secret, stdout.getvalue())
                self.assertNotIn(secret, stderr.getvalue())

    def test_fast_evidence_is_refused_before_network_access(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            evidence_dir = root / ".forge/gate/sample-service"
            evidence_dir.mkdir(parents=True)
            (evidence_dir / "evidence.json").write_text(json.dumps({
                "aggregate": "passed",
                "profile": "fast",
                "dry_run": False,
            }))
            opener = Opener()
            with patch.dict(os.environ, {"FORGE_CI_EVIDENCE_URL": "https://forge.example.test/evidence"}, clear=True), patch("urllib.request.build_opener", return_value=opener):
                with contextlib.chdir(root), self.assertRaisesRegex(ValueError, "full-profile"):
                    runpy.run_path(str(SERVICE), run_name="hosted_ci_uploader_test") ["main"]()
            self.assertEqual(opener.requests, [])


if __name__ == "__main__":
    unittest.main()
