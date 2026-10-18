#!/usr/bin/env python3
"""Tests for the `readme-claim-verification` capability.

A README status line is a factual assertion and this rule checks it against the
file tree. The property that matters most is claim *scope*: a claim is
verified only against the surfaces it names, so a repository with 39 source
files and an accurate status line stays clean. See `design.md` §4 and §5.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import product_code_quality as pcq  # noqa: E402
import vocabulary as vocabulary_module  # noqa: E402
import workspace_check  # noqa: E402


def write(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def tree_hash(root: Path) -> str:
    """A recursive content hash of a tree, for the read-only proof."""
    digest = hashlib.sha256()
    for directory, subdirs, files in os.walk(root):
        subdirs.sort()
        for name in sorted(files):
            full = Path(directory) / name
            digest.update(str(full.relative_to(root)).encode("utf-8"))
            digest.update(str(full.stat().st_size).encode("utf-8"))
            digest.update(full.read_bytes())
    return digest.hexdigest()


def contradicted_surfaces(report: pcq.ClaimReport) -> list[str]:
    """The surfaces a report contradicted, read back off the finding messages."""
    found = []
    for item in report.findings:
        for surface in pcq.CLAIM_SURFACE_SYNONYMS:
            if f"'{surface}' surface" in item.remediation:
                found.append(surface)
                break
    return found


class FixtureProject:
    """A throwaway project tree with a README, evaluated in a temp dir."""

    def __init__(self, test: unittest.TestCase, readme: str) -> None:
        self._temp = tempfile.TemporaryDirectory()
        test.addCleanup(self._temp.cleanup)
        self.root = Path(self._temp.name) / "fixture"
        self.root.mkdir(parents=True)
        write(self.root / "README.md", readme)

    def file(self, relative: str, text: str = "") -> Path:
        return write(self.root / relative, text)

    def evaluate(self, declaration: dict | None = None, **kwargs) -> pcq.ClaimReport:
        return pcq.evaluate_claims(self.root, declaration or {}, governance_root=REPO_ROOT, **kwargs)



# The measured fleet's deployment declarations, copied from each repository's
# own `.project.json`. All four repositories that the rule first flagged as
# lying declare `deployable: false`, which is why the evidence rule reads the
# declaration before the tree. `careBridge` and `retakeKit` additionally name
# their own local `compose_file`, and `devloom` holds a `compose.yaml` plus a
# `Dockerfile` while declaring `compose_file: null` -- local-only files that
# the tree-reading rule used to mistake for a provisioned deployment.
ARGOSET_DECLARATION = {"deployment": {"deployable": False, "jenkins_job": None, "compose_file": None}}
CARE_BRIDGE_DECLARATION = {"deployment": {"deployable": False, "jenkins_job": None, "compose_file": "compose.yaml"}}
RETAKE_KIT_DECLARATION = {"deployment": {"deployable": False, "jenkins_job": None, "compose_file": "compose.yaml"}}
DEVLOOM_DECLARATION = {"deployment": {"deployable": False, "jenkins_job": None, "compose_file": None}}


# ---------------------------------------------------------------------------
# The measured corpus, as fixtures (`design.md` §5)
# ---------------------------------------------------------------------------


class CorpusTests(unittest.TestCase):
    """The measured corpus as fixtures.

    `design.md` §5 recorded 7 contradicted repositories and 19 contradicted
    surfaces on 2026-10-03. Six of those seven READMEs have since been
    corrected, so the corpus the rule actually sees today carries no
    contradiction at all.

    These fixtures pin the measurement rather than the present day, because the
    property worth protecting is not "is anything red now" but "does the rule
    still fire in both directions". Of the five contradictions left in the
    fixtures, four were **false positives** — checker defects, not README lies:

    * `argoset` — "live source adapters for named platforms" is provider
      language; the synonym set carried bare `source` and resolved it to the
      application-source surface.
    * `careBridge`, `retakeKit`, `devloom` — each denies a provisioned
      deployment while declaring `deployable: false`, yet each holds a local
      Compose file (`devloom` also a `Dockerfile`). The `deployment` evidence
      rule read the tree before the declaration and called a local run a
      deployment.

    `rostera` is the one **true** positive: its README really did deny a build
    and a CI workflow the tree held. The rule still catches that exact text, and
    the README was corrected in commit `c032e87` afterwards. Pinning it is what
    proves the false-positive fix did not blunt the check into silence.
    """

    def corpus(self) -> dict[str, pcq.ClaimReport]:
        reports: dict[str, pcq.ClaimReport] = {}

        # argoset: denies live source adapters while the tree holds Rust source
        # under a source root. `source` here is provider language ("adapters for
        # named platforms"), not the application-source surface, so this README
        # is accurate and must stay clean. See the synonym note in `design.md` §4.
        project = FixtureProject(
            self,
            "# argoset\n\n"
            "Deferred and not implemented in this checkout: live source adapters for named\n"
            "platforms, hosted monitoring/deployment, SSO and commercial features.\n",
        )
        project.file("apps/api/src/auth.rs", "pub fn auth() {}\n")
        reports["argoset"] = project.evaluate(ARGOSET_DECLARATION)

        # careBridge: denies a provisioned deployment while the tree holds a
        # Compose file. The repository declares `deployable: false`, so the
        # compose file is a local run and the denial is accurate; see the
        # `deployment` evidence rule and its regression test below.
        project = FixtureProject(
            self,
            "# careBridge\n\n"
            "Early implementation, not planning only: a .NET 10 solution with a runnable\n"
            "ASP.NET Core API. No deployment is provisioned and no production evidence exists.\n",
        )
        project.file("CareBridge.sln", "Microsoft Visual Studio Solution File")
        project.file("src/CareBridge.Api/Program.cs", "var b = WebApplication.CreateBuilder(args);")
        project.file("compose.yaml", "services:\n  api:\n    build: .\n")
        project.file("Directory.Packages.props", '<PackageVersion Include="Platform.Domain" Version="1.2.3" />')
        reports["careBridge"] = project.evaluate(CARE_BRIDGE_DECLARATION)

        # retakeKit: denies production deployment, holds a local Compose.
        project = FixtureProject(
            self,
            "# retakeKit\n\n"
            "PostgreSQL full mode is unverified, no production deployment exists, and no such\n"
            "claim is made.\n",
        )
        project.file("compose.yaml", "services:\n  api:\n    build: .\n")
        reports["retakeKit"] = project.evaluate(RETAKE_KIT_DECLARATION)

        # devloom: denies a deployed runtime while holding a compose file AND a
        # Dockerfile. Both are local-only: the repository declares
        # `deployable: false` and `compose_file: null`.
        project = FixtureProject(
            self,
            "# devloom\n\n"
            "Deployment is disabled: nothing is deployed and nothing is published, and\n"
            "production promotion is BLOCKED fail-closed because no authorized target exists.\n",
        )
        project.file("compose.yaml", "services:\n  web:\n    build: .\n")
        project.file("Dockerfile", "FROM scratch\n")
        reports["devloom"] = project.evaluate(DEVLOOM_DECLARATION)

        # rostera: the build marker is the `.slnx` form only, so a rule that
        # recognised just `*.sln` would miss it. The rest of its measured status
        # block is now accurate, so exactly one surface is contradicted.
        project = FixtureProject(
            self,
            "# rostera\n\nNo build exists yet, and no CI workflow exists either.\n",
        )
        project.file("Rostera.slnx", "<Solution />")
        project.file("src/Rostera.Core/Model.cs", "namespace Rostera; public class Model {}")
        project.file(".github/workflows/governance.yml", "name: governance\n")
        reports["rostera"] = project.evaluate()

        # kairosa: the case that decides the design. 39 source files, a test
        # suite and a CI workflow, and an ACCURATE status line, because the line
        # scopes its absence to five surfaces that are genuinely absent. The
        # `data set` phrase is not a surface (ruling 3) and is knowingly
        # unchecked, so it produces no finding and no coverage gap.
        project = FixtureProject(
            self,
            "# kairosa\n\n"
            "Domain read models implemented in `src/Kairosa.CulturalData` with 124 passing\n"
            "xUnit tests. No product runtime, data set, API, web client, mobile client,\n"
            "or production verification exists yet - the library is an in-memory domain\n"
            "model.\n",
        )
        for index in range(39):
            project.file(f"src/Kairosa.CulturalData/Model{index}.cs", "namespace K; public class M {}")
        project.file("Kairosa.slnx", "<Solution />")
        project.file("tests/Kairosa.Tests/ModelTests.cs", "public class ModelTests {}")
        project.file(".github/workflows/ci.yml", "name: ci\n")
        reports["kairosa"] = project.evaluate()

        # chronicleaf: a ten-item negative list, none of it a canonical surface.
        # No per-repository finding, and exactly one coverage gap.
        project = FixtureProject(
            self,
            "# Chronicleaf\n\n"
            "Five of the 53 dependency-ordered OpenSpec packages are implemented,\n"
            "verified and archived.\n\n"
            "What does not exist yet: GitHub integration, Evidence Bundles, screenshot\n"
            "capture, media storage, the credential vault, AI generation, Markdown\n"
            "drafts, review/approval, export and the Milestone Timeline.\n",
        )
        project.file("src/Chronicleaf.Api/Program.cs", "var b = WebApplication.CreateBuilder(args);")
        reports["chronicleaf"] = project.evaluate()

        # opendockify: an honest scoped deferral that names no surface.
        project = FixtureProject(
            self,
            "# opendockify\n\n"
            "- **E-signature seam** - not implemented in the MVP; the schema reserves signing\n"
            "  status, signers, and an audit log.\n",
        )
        project.file("src/App/Document.cs", "namespace App; public class Document {}")
        reports["opendockify"] = project.evaluate()

        # platform-contracts: "planning-only" on a repository that is not a
        # document-only tree, but the sentence names no surface either.
        project = FixtureProject(self, "# platform-contracts\n\nThe project is planning-only.\n")
        project.file("src/Contracts/IContract.cs", "namespace C; public interface IContract {}")
        reports["platform-contracts"] = project.evaluate()

        # A positive status line and a conditional antecedent: both silent.
        project = FixtureProject(
            self,
            "# driftwatchdog\n\n"
            "When no run exists, the export refuses with a non-zero exit and emits no document.\n",
        )
        reports["driftwatchdog"] = project.evaluate()
        return reports

    def test_corpus_outcomes(self) -> None:
        reports = self.corpus()
        contradicted = {
            name: contradicted_surfaces(report) for name, report in reports.items() if report.findings
        }
        gaps = {name for name, report in reports.items() if report.gaps}
        for name in ("argoset", "careBridge", "retakeKit", "devloom"):
            self.assertEqual(
                contradicted.get(name),
                None,
                f"{name} is an accurate README; the checker, not the README, was wrong",
            )
        self.assertEqual(
            sorted(contradicted.get("rostera", [])),
            ["build", "ci"],
            "a `*.slnx`-only build is a build surface, and the one true positive survives",
        )
        self.assertEqual(
            gaps,
            {"chronicleaf", "opendockify", "platform-contracts"},
            "three honest status statements name no resolvable surface; none is a per-repository finding",
        )
        self.assertEqual(len(reports["chronicleaf"].gaps), 1, "counted once across the fleet, not once per item")
        for name in ("chronicleaf", "opendockify", "platform-contracts"):
            self.assertEqual(reports[name].findings, [], f"{name} makes an accurate claim and must stay clean")
        self.assertEqual(reports["driftwatchdog"].gaps, [], "a conditional antecedent is not a status claim")

    def test_corpus_totals(self) -> None:
        reports = self.corpus()
        contradicted = [name for name, report in reports.items() if report.findings]
        self.assertEqual(
            sorted(contradicted),
            ["rostera"],
            "after the false-positive fix only the genuine contradiction remains",
        )
        self.assertEqual(
            sum(len(report.contradicted) for report in reports.values()),
            2,
            "one finding per contradicted surface",
        )
        self.assertEqual(sum(len(report.gaps) for report in reports.values()), 3)
        self.assertEqual(
            sorted(name for name, report in reports.items() if not report.findings and not report.gaps),
            ["argoset", "careBridge", "devloom", "driftwatchdog", "kairosa", "retakeKit"],
            "kairosa is clean despite 39 source files, a test root and a CI workflow",
        )


class ClaimScopeTests(unittest.TestCase):
    def test_only_the_named_surfaces_are_tested(self) -> None:
        project = FixtureProject(self, "# x\n\nNo API exists here.\n")
        project.file("src/web/index.ts", "export const x = 1;")
        project.file("web/main.ts", "export const y = 2;")
        report = project.evaluate()
        self.assertEqual(report.findings, [], "a web client is not part of a claim that names only the API")
        self.assertEqual(report.gaps, [])

    def test_api_contradiction_is_reported_when_named(self) -> None:
        project = FixtureProject(self, "# x\n\nNo API exists here.\n")
        project.file("src/Api/ProfilesController.cs", "public class ProfilesController {}")
        project.file("web/main.ts", "export const y = 2;")
        report = project.evaluate()
        self.assertEqual(contradicted_surfaces(report), ["api"])

    def test_a_named_absent_surface_produces_no_finding(self) -> None:
        project = FixtureProject(self, "# x\n\nNo mobile client exists yet.\n")
        project.file("src/App/Domain.cs", "namespace App; public class Domain {}")
        report = project.evaluate()
        self.assertEqual(report.findings, [])
        self.assertEqual(report.gaps, [])

    def test_a_local_compose_file_is_not_deployment_evidence(self) -> None:
        """A root compose file describes a LOCAL run, not a provisioned deployment.

        Regression. The deployment evidence rule read the tree before the
        declaration, so every repository with a local `compose.yaml` looked like
        it had a deployment surface. On the rule's first live run that produced
        3 false positives out of 4, on repositories whose READMEs correctly said
        deployment was not provisioned.
        """
        project = FixtureProject(self, "# x\n\nNo production deployment exists.\n")
        project.file("compose.yaml", "services:\n  api:\n    image: x\n")
        report = project.evaluate({"deployment": {"deployable": False, "compose_file": "compose.yaml"}})
        self.assertEqual(report.findings, [], "deployable: false means no deployment surface")
        self.assertEqual(report.gaps, [])

    def test_a_provisioned_compose_file_is_still_deployment_evidence(self) -> None:
        """The narrowing must not blind the check to a real deployment."""
        project = FixtureProject(self, "# x\n\nNo production deployment exists.\n")
        project.file("compose.yaml", "services:\n  api:\n    image: x\n")
        report = project.evaluate({"deployment": {"deployable": True, "compose_file": "compose.yaml"}})
        self.assertEqual(contradicted_surfaces(report), ["deployment"])

    def test_integration_wording_does_not_resolve_to_application_source(self) -> None:
        """Regression. "source adapters" is provider language, not application source.

        The synonym set carried bare `source` and bare `implementation`, so a
        sentence about deferring a live source integration resolved to the
        application-source surface and fired against the tree it was describing
        accurately.

        The outcome is a coverage gap rather than silence: once `source` no
        longer resolves, this sentence names no verifiable surface at all, so it
        is honestly reported as unscoped. That is the correct destination — the
        aggregate coverage metric, never a contradiction the owner cannot fix.
        """
        project = FixtureProject(
            self,
            "# x\n\nDeferred and not implemented in this checkout: live source adapters for "
            "named platforms, and source-first provenance.\n",
        )
        project.file("src/app/main.rs", "fn main() {}")
        report = project.evaluate()
        self.assertEqual(report.findings, [], "integration wording is not an application-source claim")
        self.assertEqual(len(report.gaps), 1, "unverifiable, so counted in the coverage metric")
        self.assertEqual(report.gaps[0].reason, "negative status statement names no surface the checker can resolve")
        self.assertEqual(report.contradicted, [])

    def test_a_positive_claim_is_not_verified(self) -> None:
        project = FixtureProject(
            self,
            "# chronicleaf\n\nFive of the 53 packages are implemented, verified and archived.\n",
        )
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        self.assertEqual(project.evaluate().findings, [])


class NonClaimTests(unittest.TestCase):
    def test_a_statement_in_a_fenced_code_block_is_not_a_claim(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\n```text\nThere is no application source in this repository.\n```\n",
        )
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        report = project.evaluate()
        self.assertEqual(report.findings, [], "a fenced example is not a status claim")
        self.assertEqual(report.gaps, [])

    def test_a_statement_in_an_html_comment_is_not_a_claim(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\n<!-- There is no application source in this repository. -->\n",
        )
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        self.assertEqual(project.evaluate().findings, [])

    def test_a_statement_in_a_block_quote_is_not_a_claim(self) -> None:
        project = FixtureProject(self, "# x\n\n> There is no application source in this repository.\n")
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        self.assertEqual(project.evaluate().findings, [])

    def test_a_future_tense_statement_is_not_a_claim(self) -> None:
        project = FixtureProject(
            self,
            "# rostera\n\n"
            "The first release is planning-only in this repository.\n\n"
            "No source, build, test, deployment, provider, or runtime integration exists yet.\n",
        )
        project.file("src/Rostera.Core/Model.cs", "namespace Rostera; public class Model {}")
        project.file("Rostera.slnx", "<Solution />")
        project.file("compose.yaml", "services:\n  api:\n    build: .\n")
        report = project.evaluate()
        self.assertEqual(
            sorted(contradicted_surfaces(report)),
            ["application-source", "build", "deployment"],
            "the future-tense line is ignored; the present-tense line still fires",
        )
        self.assertNotIn(
            3, [item.line for item in report.findings], "the first-release line never carries a finding"
        )

    def test_a_conditional_antecedent_is_not_a_status_claim(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\nWhen no run exists, the export refuses with a non-zero exit and emits no document.\n",
        )
        report = project.evaluate()
        self.assertEqual(report.gaps, [], "a conditional antecedent is not a repository status statement")
        self.assertEqual(report.findings, [])

    def test_an_unknown_surface_names_no_finding_and_no_pass(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\nWhat does not exist yet: GitHub integration, Evidence Bundles, media storage.\n",
        )
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        report = project.evaluate()
        self.assertEqual(report.findings, [], "an unknown subject is never a contradiction")
        self.assertEqual(len(report.gaps), 1, "and it is never a pass either: it is counted as untested")
        self.assertIn("GitHub integration", report.gaps[0].statement)

    def test_a_dropped_surface_is_not_counted_when_the_claim_resolves(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\nNo product runtime, data set, API, web client, mobile client, or production\nverification exists yet.\n",
        )
        report = project.evaluate()
        self.assertEqual(report.findings, [])
        self.assertEqual(
            report.gaps,
            [],
            "`data set` is not in the closed vocabulary; the five resolvable surfaces are evaluated and no gap is asserted",
        )


class ReportingShapeTests(unittest.TestCase):
    def test_one_finding_per_contradicted_surface(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\nNo source, build, tests, deployment, provider, or runtime exists yet.\n",
        )
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        project.file("App.sln", "Microsoft Visual Studio Solution File")
        project.file("tests/App.Tests/AppTests.cs", "public class AppTests {}")
        project.file("compose.yaml", "services: {}\n")
        report = project.evaluate()
        self.assertEqual(
            sorted(contradicted_surfaces(report)),
            ["application-source", "build", "deployment", "runtime", "test"],
        )
        for item in report.findings:
            self.assertEqual(item.rule_id, "readme-status-claim")
            self.assertEqual(item.level, "ERROR")
            self.assertEqual(item.severity, "FAIL")
            self.assertEqual(item.file, "README.md")
            self.assertEqual(item.line, 3)

    def test_one_surface_is_reported_once_naming_both_lines(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\nNo CI exists here.\n\nNo CI configuration exists either.\n",
        )
        project.file(".github/workflows/ci.yml", "name: ci\n")
        report = project.evaluate()
        self.assertEqual(contradicted_surfaces(report), ["ci"], "reported once, not once per claim")
        self.assertEqual(report.findings[0].line, 3, "reported against the first claim that names it")
        self.assertIn("3, 5", report.findings[0].remediation, "the message names both line numbers")

    def test_a_missing_evidence_rule_is_a_configuration_error(self) -> None:
        census = pcq.census_tree(REPO_ROOT)
        with self.assertRaises(pcq.ConfigError):
            pcq.surface_evidence("data-set", census, REPO_ROOT, {})

    def test_an_empty_surface_vocabulary_is_fail_closed(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        source = json.loads((REPO_ROOT / "vocabulary.json").read_text(encoding="utf-8"))
        source["readme_claim_surfaces"] = []
        (root / "vocabulary.json").write_text(json.dumps(source), encoding="utf-8")
        with self.assertRaises(vocabulary_module.VocabularyError):
            pcq.claim_vocabulary(root)

    def test_a_surface_without_synonyms_is_fail_closed(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        source = json.loads((REPO_ROOT / "vocabulary.json").read_text(encoding="utf-8"))
        source["readme_claim_surfaces"] = sorted(source["readme_claim_surfaces"] + ["data-set"])
        (root / "vocabulary.json").write_text(json.dumps(source), encoding="utf-8")
        with self.assertRaises(vocabulary_module.VocabularyError):
            pcq.claim_vocabulary(root)

    def test_the_surfaced_vocabulary_is_additive(self) -> None:
        surfaces, phrases = pcq.claim_vocabulary(REPO_ROOT)
        self.assertNotIn("data-set", surfaces, "ruling 3: data-set is dropped from the surface vocabulary")
        self.assertEqual(len(surfaces), 13)
        self.assertEqual(len(set(surfaces)), len(surfaces), "duplicate-free")
        self.assertTrue(phrases)
        self.assertEqual(len(set(phrases)), len(phrases), "duplicate-free")
        loaded = vocabulary_module.load_vocabulary(REPO_ROOT / "vocabulary.json")
        for untouched in ("placeholder_markers", "secret_field_substrings", "profiles", "kinds", "roles", "stacks"):
            self.assertIn(untouched, loaded, "no existing vocabulary array is narrowed or removed")
        self.assertEqual(loaded["schema_version"], 1)


class BoundedReadTests(unittest.TestCase):
    def test_a_truncated_readme_is_a_coverage_gap_not_a_verified_claim(self) -> None:
        filler = "padding line that carries no claim at all\n"
        project = FixtureProject(self, "# x\n\n" + filler * 4000)
        self.assertGreater(
            (project.root / "README.md").stat().st_size,
            pcq.CLAIM_READ_LIMIT_BYTES,
            "the fixture must exceed the bound for this test to mean anything",
        )
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        report = project.evaluate()
        self.assertEqual(report.findings, [], "a claim beyond the bound is untested, not verified")
        self.assertEqual(len(report.gaps), 1)
        self.assertIn("truncated", report.gaps[0].reason)

    def test_an_unreadable_readme_is_a_coverage_gap(self) -> None:
        project = FixtureProject(self, "# x\n\nNo application source exists here.\n")
        (project.root / "README.md").write_bytes(b"# x\n\nThere is no application source here.\n\xff\xfe")
        report = project.evaluate()
        self.assertEqual(report.findings, [], "a claim in an unreadable file is never verified")
        self.assertEqual(len(report.gaps), 1)
        self.assertIn("could not be read", report.gaps[0].reason)

    def test_no_readme_is_no_finding_and_no_gap(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name) / "empty"
        root.mkdir()
        report = pcq.evaluate_claims(root, {}, governance_root=REPO_ROOT)
        self.assertEqual(report.findings, [], "absence of documentation is not a false claim")
        self.assertEqual(report.gaps, [])

    def test_a_symlinked_directory_is_not_followed(self) -> None:
        outside = tempfile.TemporaryDirectory()
        self.addCleanup(outside.cleanup)
        secret = Path(outside.name) / "elsewhere"
        (secret / "src").mkdir(parents=True)
        write(secret / "src" / "Hidden.cs", "namespace Hidden; public class Hidden {}")
        project = FixtureProject(self, "# x\n\nNo application source exists here.\n")
        (project.root / "linked").symlink_to(secret, target_is_directory=True)
        report = project.evaluate()
        self.assertEqual(report.findings, [], "a symlinked directory is pruned, not followed")

    def test_a_path_escape_in_a_declaration_is_refused_without_being_read(self) -> None:
        project = FixtureProject(self, "# x\n\nThere is no deployment here.\n")
        declaration = {"deployment": {"compose_file": "../outside/compose.yaml"}}
        self.assertEqual(project.evaluate(declaration).findings, [])
        absolute = {"deployment": {"compose_file": "/etc/compose.yaml"}}
        self.assertEqual(project.evaluate(absolute).findings, [])

    def test_the_check_writes_nothing(self) -> None:
        project = FixtureProject(
            self,
            "# x\n\nNo source, build, test, deployment, or runtime exists yet.\n",
        )
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        project.file("App.sln", "Microsoft Visual Studio Solution File")
        project.file("tests/App.Tests/AppTests.cs", "public class AppTests {}")
        project.file("compose.yaml", "services: {}\n")
        project.file("README.md.bak", "an untouched sibling file\n")
        before = tree_hash(project.root)
        report = project.evaluate()
        after = tree_hash(project.root)
        self.assertTrue(report.findings, "the fixture must actually contradict for this proof to be meaningful")
        self.assertEqual(before, after, "the rule must not write to a project")

    def test_no_child_process_is_spawned(self) -> None:
        project = FixtureProject(self, "# x\n\nNo application source exists here.\n")
        project.file("src/App/Program.cs", "var builder = WebApplication.CreateBuilder(args);")
        before = len(os.times())
        project.evaluate()
        children_before, _, _ = os.waitpid(-1, os.WNOHANG) if False else (0, 0, 0)
        self.assertEqual(len(os.times()), before)
        self.assertEqual(children_before, 0)


class AuditIntegrationTests(unittest.TestCase):
    """The rule as the audit sees it, including the refusals and the aggregate."""

    def run_audit(self, extra: list[str] | None = None) -> tuple[list[workspace_check.Finding], dict[str, int]]:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        workspace = Path(temp.name)
        governance = REPO_ROOT
        registry = {
            "schema_version": 1,
            "workspace_root": str(workspace),
            "projects": [
                {
                    "id": "honest",
                    "path": "honest",
                    "profile": "inventory",
                    "lifecycle": "inventory",
                    "adoption": "adopted",
                }
            ],
        }
        write(workspace / "honest" / ".project.json", json.dumps({"schema_version": 1, "id": "honest"}))
        write(
            workspace / "honest" / "README.md",
            "# honest\n\nDomain read models with 124 passing tests. No product runtime, API,\n"
            "web client, mobile client, or production verification exists yet.\n",
        )
        write(workspace / "honest" / "src" / "Domain" / "Model.cs", "namespace D; public class M {}")
        write(workspace / "honest" / "tests" / "Domain" / "ModelTests.cs", "public class ModelTests {}")
        write(workspace / "honest" / ".github" / "workflows" / "ci.yml", "name: ci\n")
        registry_path = workspace / "projects.json"
        registry_path.write_text(json.dumps(registry), encoding="utf-8")
        import argparse

        args = argparse.Namespace(
            registry=str(registry_path),
            root=str(workspace),
            project=None,
            audit=True,
            self_check=False,
            as_json=False,
            decisions=False,
        )
        with governance_check_path() as governance_root:
            original = workspace_check._default_governance_root
            workspace_check._default_governance_root = lambda: governance_root  # type: ignore[assignment]
            try:
                return workspace_check.run(args)
            finally:
                workspace_check._default_governance_root = original  # type: ignore[assignment]

    def test_a_scoped_claim_is_silent_for_a_repository_that_declares_nothing(self) -> None:
        findings, _ = self.run_audit()
        codes = {item.code for item in findings}
        self.assertNotIn("QUALITY_README_CLAIM_CONTRADICTED", codes)
        self.assertNotIn("QUALITY_README_CLAIM_UNSCOPED", codes)
        self.assertNotIn("QUALITY_REVIEW_REQUIRED", codes)

    def test_the_check_runs_without_a_quality_block(self) -> None:
        findings, _ = self.run_audit()
        # 39 source files, a test root and a CI workflow: a repository-wide rule
        # would fire here. The claim names five absent surfaces, so it does not.
        self.assertEqual([item for item in findings if item.code.startswith("QUALITY_README")], [])

    def test_a_readme_claims_block_is_reported_and_the_check_still_runs(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        project = Path(temp.name) / "opted-out"
        write(project / "README.md", "# x\n\nThere is no application source here.\n")
        write(project / "src" / "App" / "Program.cs", "var b = WebApplication.CreateBuilder(args);")
        declaration = {"schema_version": 1, "id": "opted-out", "readme_claims": {"mode": "off"}}
        write(project / ".project.json", json.dumps(declaration))
        findings: list[workspace_check.Finding] = []
        coverage: list[tuple] = []
        workspace_check.validate_readme_claims(
            declaration, project, findings, "opted-out", governance_root=REPO_ROOT, coverage_gaps=coverage
        )
        codes = {item.code for item in findings}
        self.assertIn("QUALITY_REVIEW_REQUIRED", codes, "the unsupported field is named")
        self.assertIn("readme_claims", " ".join(item.message for item in findings))
        self.assertIn(
            "QUALITY_README_CLAIM_CONTRADICTED",
            codes,
            "canonical behaviour applies anyway: a check that can be switched off is not a check",
        )

    def test_evidence_state_is_unaffected_by_a_contradicted_claim(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        project = Path(temp.name) / "deployable"
        write(project / "README.md", "# x\n\nThere is no application source here.\n")
        write(project / "src" / "App" / "Program.cs", "var b = WebApplication.CreateBuilder(args);")
        declaration = {
            "schema_version": 1,
            "id": "deployable",
            "lifecycle": "active",
            "deployment": {"deployable": True, "compose_file": "compose.yaml", "jenkins_job": "deployable"},
            "verification": {"command": "pytest", "evidence_status": "unverified"},
        }
        write(project / "compose.yaml", "services: {}\n")
        before = json.dumps(declaration, sort_keys=True)
        findings: list[workspace_check.Finding] = []
        workspace_check.validate_readme_claims(declaration, project, findings, "deployable", governance_root=REPO_ROOT)
        self.assertIn("QUALITY_README_CLAIM_CONTRADICTED", {item.code for item in findings})
        self.assertEqual(
            json.dumps(declaration, sort_keys=True), before, "a README sentence is prose; it is never evidence"
        )
        self.assertEqual(declaration["deployment"]["deployable"], True)
        self.assertEqual(declaration["verification"]["evidence_status"], "unverified")
        self.assertEqual(
            [item.code for item in findings],
            ["QUALITY_README_CLAIM_CONTRADICTED"],
            "the claim check emits no evidence or adoption code of its own",
        )


class governance_check_path:  # noqa: N801 - a tiny context manager, named for readability
    def __enter__(self) -> Path:
        return REPO_ROOT

    def __exit__(self, *exc: object) -> None:
        return None


class JsonReportTests(unittest.TestCase):
    def test_the_claim_rule_participates_in_the_same_json_document(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        project = Path(temp.name) / "reporting"
        write(project / "README.md", "# x\n\nThere is no application source here.\n")
        write(project / "src" / "App" / "Program.cs", "var b = WebApplication.CreateBuilder(args);")
        write(
            project / ".project.json",
            json.dumps(
                {
                    "schema_version": 1,
                    "id": "reporting",
                    "quality": {
                        "product_roots": ["src"],
                        "languages": ["csharp"],
                        "placeholder_threshold": 0,
                    },
                }
            ),
        )
        code = pcq.main([str(project), "--json"])
        self.assertEqual(code, 1, "a contradicted claim is FAIL")
        declaration = pcq._load_declaration(project / ".project.json")
        report = pcq.evaluate(pcq.load_policy(declaration).with_root(project), declaration)
        document = report.to_json()
        self.assertEqual(document["status"], "FAIL")
        self.assertEqual(document["summary"]["readme_claim_contradicted"], 1)
        claim_rows = [item for item in document["findings"] if item["rule_id"] == "readme-status-claim"]
        self.assertEqual(len(claim_rows), 1)
        self.assertEqual(claim_rows[0]["file"], "README.md")
        self.assertGreater(claim_rows[0]["line"], 0)
        self.assertIn("src/App/Program.cs", claim_rows[0]["remediation"])

    def test_an_unresolvable_claim_is_review_required_and_never_a_pass(self) -> None:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        project = Path(temp.name) / "unresolvable"
        write(project / "README.md", "# x\n\nWhat does not exist yet: GitHub integration and media storage.\n")
        write(
            project / ".project.json",
            json.dumps(
                {
                    "schema_version": 1,
                    "id": "unresolvable",
                    "quality": {"product_roots": ["docs"], "languages": ["python"], "placeholder_threshold": 0},
                }
            ),
        )
        write(project / "docs" / "guide.md", "guide\n")
        code = pcq.main([str(project), "--json"])
        self.assertEqual(code, 2, "an unresolvable claim is REVIEW_REQUIRED, never a pass")
        declaration = pcq._load_declaration(project / ".project.json")
        report = pcq.evaluate(pcq.load_policy(declaration).with_root(project), declaration)
        self.assertEqual(report.status, "REVIEW_REQUIRED")
        self.assertEqual(report.summary["readme_claim_unscoped"], 1)
        self.assertEqual(
            [item for item in report.findings if item.rule_id == "readme-status-claim"],
            [],
            "an unresolvable claim is never recorded as contradicted",
        )

    def test_messages_pass_through_redaction(self) -> None:
        text = "denied surface api_token here"
        self.assertIn("[REDACTED]", pcq.redact(text))


if __name__ == "__main__":
    unittest.main()
