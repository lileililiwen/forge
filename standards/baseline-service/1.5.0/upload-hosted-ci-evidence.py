#!/usr/bin/env python3
"""Upload one real Forge Gate record from a GitHub Actions run."""
import datetime
import glob
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

AUDIENCE = "forge-ci-evidence"
MAX_BYTES = 256 * 1024


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, new_url):
        return None


def required(name):
    value = os.environ.get(name, "").strip()
    if not value:
        raise ValueError(f"required GitHub Actions value {name} is missing")
    return value


def main():
    endpoint = required("FORGE_CI_EVIDENCE_URL")
    parsed = urllib.parse.urlsplit(endpoint)
    if parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password or parsed.fragment:
        raise ValueError("FORGE_CI_EVIDENCE_URL must be an HTTPS URL without credentials or fragment")
    evidence_paths = glob.glob(".forge/gate/*/evidence.json")
    if len(evidence_paths) != 1:
        raise ValueError("expected exactly one Forge Gate evidence record under .forge/gate")
    with open(evidence_paths[0], "rb") as source:
        raw_gate = source.read(MAX_BYTES + 1)
    if len(raw_gate) > MAX_BYTES:
        raise ValueError("Forge Gate evidence exceeds the upload size limit")
    gate = json.loads(raw_gate)
    if gate.get("dry_run") or gate.get("aggregate") in (None, "unknown"):
        raise ValueError("Forge Gate evidence is a rehearsal or has no terminal verdict")
    revision = required("GITHUB_SHA")
    if gate.get("revision") != revision:
        raise ValueError("Forge Gate evidence does not match GITHUB_SHA")
    repository = required("GITHUB_REPOSITORY")
    run_id = required("GITHUB_RUN_ID")
    attempt = int(required("GITHUB_RUN_ATTEMPT"))
    if attempt < 1:
        raise ValueError("GITHUB_RUN_ATTEMPT must be positive")
    server = os.environ.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/")
    if server != "https://github.com":
        raise ValueError("hosted evidence upload supports github.com workflows only")
    document = {
        "contract": "forge-hosted-ci-evidence/1.0",
        "project_id": required("FORGE_PROJECT_ID") if os.environ.get("FORGE_PROJECT_ID") else gate.get("project_id"),
        "repository": repository,
        "run": {
            "provider": "github-actions",
            "workflow": required("GITHUB_WORKFLOW_REF"),
            "run_id": run_id,
            "run_attempt": attempt,
            "job": required("GITHUB_JOB"),
            "url": f"{server}/{repository}/actions/runs/{run_id}",
        },
        "source_revision": revision,
        "gate": gate,
        "observed_at": datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"),
    }
    if not document["project_id"]:
        raise ValueError("Gate evidence has no project_id")
    gate_revision = document["gate"].get("revision")
    if gate_revision != revision or document["gate"].get("project_id") != document["project_id"]:
        raise ValueError("Gate evidence project or revision does not match the upload")

    token_url = required("ACTIONS_ID_TOKEN_REQUEST_URL")
    token = required("ACTIONS_ID_TOKEN_REQUEST_TOKEN")
    parts = urllib.parse.urlsplit(token_url)
    if (parts.scheme != "https" or parts.hostname != "pipelines.actions.githubusercontent.com"
            or parts.port not in (None, 443) or parts.username or parts.password or parts.fragment):
        raise ValueError("GitHub Actions token endpoint is not trusted")
    query = [(key, value) for key, value in urllib.parse.parse_qsl(parts.query, keep_blank_values=True) if key != "audience"]
    query.append(("audience", AUDIENCE))
    token_url = urllib.parse.urlunsplit((parts.scheme, parts.netloc, parts.path, urllib.parse.urlencode(query), ""))
    opener = urllib.request.build_opener(NoRedirect())
    request = urllib.request.Request(token_url, headers={"Authorization": f"Bearer {token}", "Accept": "application/json"})
    try:
        with opener.open(request, timeout=10) as response:
            token_body = response.read(64 * 1024 + 1)
    except (urllib.error.URLError, TimeoutError):
        raise RuntimeError("GitHub Actions token request failed") from None
    except urllib.error.HTTPError as error:
        raise RuntimeError(f"GitHub Actions token request failed (HTTP {error.code})") from None
    if len(token_body) > 64 * 1024:
        raise ValueError("GitHub Actions token response exceeds the size limit")
    id_token = json.loads(token_body).get("value")
    if not isinstance(id_token, str) or not id_token:
        raise ValueError("GitHub Actions did not return an ID token")

    body = json.dumps(document, separators=(",", ":")).encode()
    if len(body) > MAX_BYTES:
        raise ValueError("hosted CI evidence exceeds the upload size limit")
    opener = urllib.request.build_opener(NoRedirect())
    for attempt_number in range(4):
        request = urllib.request.Request(endpoint, data=body, method="POST", headers={
            "Authorization": f"Bearer {id_token}",
            "Content-Type": "application/json",
            "Accept": "application/json",
        })
        try:
            with opener.open(request, timeout=10) as response:
                if response.status not in (200, 201):
                    raise RuntimeError(f"Forge ingest returned unexpected HTTP {response.status}")
                result_body = response.read(64 * 1024 + 1)
            if len(result_body) > 64 * 1024:
                raise RuntimeError("Forge ingest response exceeds the size limit")
            result = json.loads(result_body)
            print(f"Forge hosted evidence recorded: {result.get('id', 'unknown')} (duplicate={str(result.get('duplicate', False)).lower()})")
            return
        except urllib.error.HTTPError as error:
            if error.code == 403:
                raise RuntimeError("Forge rejected the repository binding; register the project and bind its GitHub repository ID in Quality checks") from None
            if error.code != 429 and error.code < 500:
                raise RuntimeError(f"Forge ingest rejected the evidence (HTTP {error.code})") from None
            if attempt_number == 3:
                raise RuntimeError(f"Forge ingest remained unavailable (HTTP {error.code})") from None
        except (urllib.error.URLError, TimeoutError):
            if attempt_number == 3:
                raise RuntimeError("Forge ingest remained unavailable after bounded retries") from None
        time.sleep(0.5 * (2 ** attempt_number))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"hosted evidence upload failed: {error}", file=sys.stderr)
        sys.exit(1)
