import unittest
from pathlib import Path

import sys

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from github_metadata import GitHubCommandError, build_edit_command, check_repository, require_authentication


class FakeRunner:
    def __init__(self, returncode=0, stdout="", stderr=""):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr
        self.calls = []

    def __call__(self, command, cwd):
        self.calls.append((command, cwd))
        return self.returncode, self.stdout, self.stderr


class GitHubMetadataTests(unittest.TestCase):
    def test_auth_failure_is_blocking_and_manual(self):
        runner = FakeRunner(returncode=1, stderr="not logged in")
        with self.assertRaisesRegex(GitHubCommandError, "gh auth login"):
            require_authentication(Path("/tmp/project"), runner)
        self.assertEqual(runner.calls[0][0], ["gh", "auth", "status"])

    def test_edit_command_is_additive_and_contains_no_token(self):
        command = build_edit_command(
            "owner/repo",
            "Repository governance tool",
            "https://example.com",
            ["repository-governance", "github-metadata"],
        )
        self.assertEqual(command[:4], ["gh", "repo", "edit", "owner/repo"])
        self.assertIn("--add-topic", command)
        self.assertNotIn("token", " ".join(command).lower())
        self.assertNotIn("ghp_", " ".join(command))

    def test_existing_topics_are_preserved_when_local_topics_are_present(self):
        import json
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            project = Path(directory)
            (project / ".project.json").write_text(json.dumps({"discoverability": {
                "description": "Repository governance tool",
                "keywords": ["repository"],
                "topics": ["repository-governance"],
                "audience": ["developers"],
                "use_cases": ["audit projects"],
                "search_phrases": ["repository governance"],
                "homepage": "https://example.com",
                "license_spdx": "MIT",
                "visibility": "public",
            }}), encoding="utf-8")
            runner = FakeRunner(stdout="owner/repo\n")
            runner.results = {
                ("gh", "auth", "status"): (0, "", ""),
                ("git", "remote", "get-url", "origin"): (0, "https://github.com/owner/repo.git\n", ""),
                ("gh", "repo", "view", "owner/repo", "--json", "nameWithOwner,description,homepageUrl,visibility,repositoryTopics,defaultBranchRef"): (0, json.dumps({
                    "nameWithOwner": "owner/repo",
                    "description": "Repository governance tool",
                    "homepageUrl": "https://example.com",
                    "visibility": "PUBLIC",
                    "repositoryTopics": ["repository-governance", "existing-topic"],
                    "defaultBranchRef": {"name": "main"},
                }), ""),
            }
            def mapped(command, cwd):
                runner.calls.append((command, cwd))
                return runner.results.get(tuple(command), (runner.returncode, runner.stdout, runner.stderr))
            result = check_repository(project, mapped)
            self.assertEqual(result["state"], "metadata_verified")
            self.assertEqual(result["differences"], {})


if __name__ == "__main__":
    unittest.main()
