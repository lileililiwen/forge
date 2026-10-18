"""Tests for the UI convention conformance checker.

Every scratch repository is built inside a `tempfile.TemporaryDirectory()`, so
cleanup is in-process. This host has an `rm` shim that intercepts and fails a
shell delete, and `AGENTS.md` forbids it; nothing here shells out to delete.
"""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))

import generate_ui_conformance_baseline as baseline_generator  # noqa: E402
import ui_conformance as ui  # noqa: E402


TOKEN_SOURCE = Path("/home/paul/code/dotnet-platform-libs/ui/tokens/tokens.json")
TOKEN_MODULE = Path("/home/paul/code/dotnet-platform-libs/ui/scripts/lib/token-model.mjs")


def write(root: Path, relative: str, text: str) -> Path:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def declare(root: Path, ui_block: dict | None) -> None:
    document: dict = {"schema_version": 1, "id": root.name, "kind": "python-product"}
    if ui_block is not None:
        document["ui"] = ui_block
    write(root, ".project.json", json.dumps(document, indent=2))


def scratch(**_: object) -> "tempfile.TemporaryDirectory[str]":
    return tempfile.TemporaryDirectory(prefix="ui-conformance-")


class CensusTests(unittest.TestCase):
    def test_suffix_set_and_pruned_directories(self):
        with scratch() as tmp:
            root = Path(tmp)
            write(root, "app/pages/Home.tsx", "export const Home = 1;\n")
            write(root, "app/style.css", "body { margin: 0; }\n")
            write(root, "templates/index.html", "<html></html>\n")
            write(root, "web/main.dart", "void main() {}\n")
            write(root, "README.md", "# not ui\n")
            for pruned in ("node_modules", "dist", "target", "coverage", ".next", "obj", "bin"):
                write(root, f"{pruned}/ui/Widget.tsx", "export const W = 1;\n")
            found = ui.census(root)
        self.assertEqual(found, ["app/pages/Home.tsx", "app/style.css", "templates/index.html", "web/main.dart"])

    def test_unsupported_suffix_is_not_ui(self):
        with scratch() as tmp:
            root = Path(tmp)
            write(root, "src/main.ts", "export const x = 1;\n")
            write(root, "src/util.js", "module.exports = 1;\n")
            self.assertEqual(ui.census(root), [])

    def test_symlinked_ui_file_is_not_read(self):
        with scratch() as tmp:
            root = Path(tmp)
            write(root, "real/Page.tsx", "export const P = 1;\n")
            (root / "linked").symlink_to(root / "real", target_is_directory=True)
            found = ui.census(root)
        self.assertEqual(found, ["real/Page.tsx"])


class ScopeParsingTests(unittest.TestCase):
    def test_absent_block_is_fully_undeclared(self):
        policy = ui.parse_ui_policy({"schema_version": 1})
        self.assertFalse(policy.scope_declared)
        self.assertFalse(policy.adopted)
        self.assertEqual(policy.max_files, ui.DEFAULT_SCOPE_CEILING)

    def test_valid_declaration(self):
        policy = ui.parse_ui_policy({"ui": {
            "scope": {"roots": ["apps/web/src"], "exclude": ["apps/web/src/legacy"], "max_files": 250},
            "shared_layer": {"adopted": True, "consume_paths": ["vendor/ui"]},
            "branding_document": "docs/branding.json",
        }})
        self.assertEqual(policy.scope_roots, ("apps/web/src",))
        self.assertEqual(policy.scope_exclude, ("apps/web/src/legacy",))
        self.assertEqual(policy.max_files, 250)
        self.assertTrue(policy.adopted)
        self.assertEqual(policy.consume_paths, ("vendor/ui",))

    def test_absolute_root_is_a_configuration_error(self):
        with self.assertRaises(ui.ConfigError):
            ui.parse_ui_policy({"ui": {"scope": {"roots": ["/etc"]}}})

    def test_escaping_root_is_a_configuration_error(self):
        with self.assertRaises(ui.ConfigError):
            ui.parse_ui_policy({"ui": {"scope": {"roots": ["../sibling/src"]}}})

    def test_bad_ceiling_is_a_configuration_error(self):
        with self.assertRaises(ui.ConfigError):
            ui.parse_ui_policy({"ui": {"scope": {"roots": ["src"], "max_files": 0}}})

    def test_in_scope_excludes_declared_paths(self):
        policy = ui.parse_ui_policy({"ui": {"scope": {
            "roots": ["src"], "exclude": ["src/legacy"],
        }}})
        self.assertTrue(ui._in_scope("src/app.css", policy))
        self.assertFalse(ui._in_scope("src/legacy/old.css", policy))
        self.assertFalse(ui._in_scope("docs/readme.css", policy))


class FindingTests(unittest.TestCase):
    """Each of the eight codes, in isolation."""

    def _conformant(self, root: Path) -> None:
        declare(root, {
            "scope": {"roots": ["src"]},
            "shared_layer": {"adopted": True, "consume_paths": ["vendor/ui"]},
            "branding_document": "docs/branding.json",
            "contrast_evidence": "docs/contrast-audit.json",
        })
        write(root, "src/App.css", ":root { color: #0f172a; }\n[data-theme='dark'] { color: #f8fafc; }\n")
        write(root, "docs/branding.json", "{}\n")
        write(root, "docs/contrast-audit.json", "{}\n")

    def _evaluate(self, root: Path, **kwargs: object) -> ui.Report:
        policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
        return ui.evaluate(root, policy, **kwargs)  # type: ignore[arg-type]

    def test_conformant_repository_passes(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._conformant(root)
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertEqual(report.status, "PASS")
        self.assertEqual(report.findings, [])
        self.assertEqual(ui.exit_code(report), 0)

    def test_no_ui_surface_is_out_of_scope(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, None)
            write(root, "README.md", "# nothing here\n")
            write(root, "src/main.py", "print(1)\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE)
        self.assertEqual(report.status, "PASS")
        self.assertEqual(report.summary["ui_files"], 0)

    def test_ui_scope_undeclared_is_review_required(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, None)
            write(root, "src/App.css", "body { color: #ff00aa; }\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE)
        self.assertIn(ui.UI_SCOPE_UNDECLARED, report.codes())
        finding = next(f for f in report.findings if f.rule_id == ui.UI_SCOPE_UNDECLARED)
        self.assertEqual(finding.level, "REVIEW_REQUIRED")

    def test_ui_scope_undeclared_exits_two_when_nothing_else_blocks(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {"shared_layer": {"adopted": True}})
            write(root, "src/App.css", ".x { color: #0f172a; }\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE)
        self.assertEqual(report.codes(), {ui.UI_SCOPE_UNDECLARED})
        self.assertEqual(report.status, "REVIEW_REQUIRED")
        self.assertEqual(ui.exit_code(report), 2)

    def test_ui_auth_handrolled(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._conformant(root)
            write(root, "src/LoginPage.tsx", "export const Login = 1;\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())
        self.assertEqual(ui.exit_code(report), 1)

    def test_ui_token_not_consumed(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {"scope": {"roots": ["src"]}})
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE)
        self.assertIn(ui.UI_TOKEN_NOT_CONSUMED, report.codes())

    def test_ui_color_literal(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._conformant(root)
            write(root, "src/theme.css", ".x { color: #ff00aa; }\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_COLOR_LITERAL, report.codes())
        finding = next(f for f in report.findings if f.rule_id == ui.UI_COLOR_LITERAL)
        self.assertEqual(finding.file, "src/theme.css")
        self.assertEqual(finding.line, 1)

    def test_declared_literal_does_not_fire(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._conformant(root)
            write(root, "src/theme.css", ".x { color: #0f172a; }\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertNotIn(ui.UI_COLOR_LITERAL, report.codes())

    def test_an_rgba_spelling_of_a_token_colour_is_still_a_literal(self):
        """The comparison is literal spelling, not colour-space equivalence.

        `tokens.json` declares `#f8fafc`. A repository writing the same colour
        as `rgb(248, 250, 252)` has hardcoded a colour in the UI source, which
        is exactly what the rule targets, so it is reported. Converting colour
        spaces is deliberately out of scope (design.md §4.4).
        """
        with scratch() as tmp:
            root = Path(tmp)
            self._conformant(root)
            write(root, "src/theme.css", ".x { background: rgb(248, 250, 252); }\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_COLOR_LITERAL, report.codes())

    def test_three_digit_hex_normalises(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._conformant(root)
            write(root, "src/theme.css", ".x { color: #0F172A; }\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertNotIn(ui.UI_COLOR_LITERAL, report.codes())
        self.assertEqual(ui.normalise_literal("#FFF"), "#ffffff")
        self.assertEqual(ui.normalise_literal("#0F172A"), "#0f172a")

    def test_ui_theme_absent(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "branding_document": "docs/branding.json",
                "contrast_evidence": "docs/contrast-audit.json",
            })
            write(root, "src/App.css", ".x { color: #0f172a; }\n")
            write(root, "docs/branding.json", "{}\n")
            write(root, "docs/contrast-audit.json", "{}\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_THEME_ABSENT, report.codes())

    def test_ui_branding_absent(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "contrast_evidence": "docs/contrast-audit.json",
            })
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            write(root, "docs/contrast-audit.json", "{}\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_BRANDING_ABSENT, report.codes())

    def test_ui_contrast_unmeasured(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "branding_document": "docs/branding.json",
            })
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            write(root, "docs/branding.json", "{}\n")
            report = self._evaluate(root, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_CONTRAST_UNMEASURED, report.codes())

    def test_declared_branding_path_that_does_not_resolve_is_a_finding(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._conformant(root)
            # A root-level branding.json exists, but the repository declared a
            # different path. A declared path that does not resolve must not
            # fall back to the default name.
            write(root, "branding.json", "{}\n")
            policy = ui.parse_ui_policy({
                "scope": {"roots": ["src"]},
                "shared_layer": {"adopted": True},
                "branding_document": "docs/missing.json",
            })
            report = ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertNotIn(ui.UI_BRANDING_ABSENT, report.codes())  # adoption suppresses it
        resolved = ui._declared_or_default(root, "docs/missing.json", ui.DEFAULT_BRANDING_FILES)
        self.assertIsNone(resolved)


class TokenBoundaryTests(unittest.TestCase):
    def test_resolver_returns_the_declared_colour_literals(self):
        literals = ui.resolve_token_literals(TOKEN_SOURCE, TOKEN_MODULE)
        self.assertIn("#0f172a", literals)
        self.assertNotIn("#ff00aa", literals)

    def test_missing_module_fails_closed(self):
        with self.assertRaises(ui.TokenSourceError):
            ui.resolve_token_literals(TOKEN_SOURCE, Path("/nonexistent/token-model.mjs"))

    def test_missing_token_source_fails_closed(self):
        with self.assertRaises(ui.TokenSourceError):
            ui.resolve_token_literals(Path("/nonexistent/tokens.json"), TOKEN_MODULE)

    def test_unavailable_token_source_reports_every_literal(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "shared_layer": {"adopted": True},
            })
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
            report = ui.evaluate(root, policy, token_source=Path("/nonexistent/tokens.json"), module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_COLOR_LITERAL, report.codes())
        self.assertTrue(any("token source" in note for note in report.notes))
        # Fail-closed, and never a pass: the comparison did not run, so the
        # literal is reported as unreached and the process is non-zero.
        self.assertNotEqual(ui.exit_code(report), 0)
        self.assertEqual(report.summary["coverage_review"], 1)


class AdoptionCombinationTests(unittest.TestCase):
    """The case this checker exists for: adoption declared, login hand-rolled."""

    def test_adoption_plus_hand_rolled_login_still_fails(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "shared_layer": {"adopted": True, "consume_paths": ["vendor/ui"]},
                "branding_document": "docs/branding.json",
                "contrast_evidence": "docs/contrast-audit.json",
            })
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            write(root, "src/auth/LoginForm.tsx", "export const LoginForm = 1;\n")
            write(root, "docs/branding.json", "{}\n")
            write(root, "docs/contrast-audit.json", "{}\n")
            policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
            report = ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())
        # adoption suppresses 5-7 and the token rule, and nothing else
        self.assertNotIn(ui.UI_THEME_ABSENT, report.codes())
        self.assertNotIn(ui.UI_BRANDING_ABSENT, report.codes())
        self.assertNotIn(ui.UI_CONTRAST_UNMEASURED, report.codes())
        self.assertNotIn(ui.UI_TOKEN_NOT_CONSUMED, report.codes())
        self.assertEqual(ui.exit_code(report), 1)

    def test_consume_path_exempts_the_shared_package_screen(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "shared_layer": {"adopted": True, "consume_paths": ["vendor/ui"]},
            })
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            write(root, "vendor/ui/Login.tsx", "export const Login = 1;\n")
            policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
            report = ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertNotIn(ui.UI_AUTH_HANDROLLED, report.codes())

    def test_directory_named_auth_does_not_match(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "shared_layer": {"adopted": True},
            })
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            write(root, "src/authors/Article.tsx", "export const Article = 1;\n")
            policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
            report = ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertNotIn(ui.UI_AUTH_HANDROLLED, report.codes())


class SharedLayerEvidenceTests(unittest.TestCase):
    """A migrated login page is not a hand-rolled one. The file proves it."""

    def _adopter(self, root: Path) -> None:
        declare(root, {
            "scope": {"roots": ["Pages"]},
            "shared_layer": {"adopted": True, "consume_paths": ["vendor/ui"]},
        })
        write(root, "Pages/Home.cshtml", "<main class=\"platform-shell\"></main>\n")

    def _report(self, root: Path) -> object:
        policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
        return ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)

    def _note(self, report, rel: str) -> str:
        return next(n for n in report.notes if rel in n)

    def test_bare_login_page_still_fires(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {"scope": {"roots": ["src"]}})
            write(root, "src/Home.tsx", "export const Home = 1;\n")
            write(root, "src/LoginPage.tsx",
                  "export const LoginPage = () => (\n"
                  "  <form class=\"login\">\n"
                  "    <label htmlFor=\"u\">User</label>\n"
                  "    <input id=\"u\" />\n"
                  "  </form>\n"
                  ");\n")
            report = self._report(root)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())
        self.assertEqual(
            [f.file for f in report.findings if f.rule_id == ui.UI_AUTH_HANDROLLED],
            ["src/LoginPage.tsx"],
        )

    def test_shared_class_name_does_not_fire_and_names_the_signal(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/Login.cshtml",
                  '<section class="platform-card platform-auth-card">\n'
                  '  <form class="platform-auth-form"><input class="platform-control" /></form>\n'
                  "</section>\n")
            report = self._report(root)
        self.assertNotIn(ui.UI_AUTH_HANDROLLED, report.codes())
        self.assertIn("platform-* class name", self._note(report, "Pages/Login.cshtml"))

    def test_shared_token_stylesheet_does_not_fire(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/Register.cshtml",
                  '<link rel="stylesheet" href="~/css/platform/design-tokens/dist/tokens.css" />\n'
                  '<form class="register"><input name="u" /></form>\n')
            report = self._report(root)
        self.assertNotIn(ui.UI_AUTH_HANDROLLED, report.codes())
        self.assertIn("vendored token stylesheet", self._note(report, "Pages/Register.cshtml"))

    def test_shared_layer_stylesheet_does_not_fire(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/ForgotPassword.cshtml",
                  '@import url("~/css/platform/react-auth/styles.css");\n'
                  '<form class="forgot"><input name="u" /></form>\n')
            report = self._report(root)
        self.assertNotIn(ui.UI_AUTH_HANDROLLED, report.codes())
        self.assertIn("shared-layer stylesheet", self._note(report, "Pages/ForgotPassword.cshtml"))

    def test_declared_adopter_writing_a_fresh_login_page_still_fires(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/Login.cshtml",
                  '<form class="login"><input name="u" /></form>\n')
            report = self._report(root)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())
        self.assertEqual(ui.exit_code(report), 1)

    def test_handler_page_rendered_by_the_shared_surface_does_not_fire(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/_ViewStart.cshtml", '@{\n    Layout = "_Layout";\n}\n')
            write(root, "Pages/Shared/_Layout.cshtml",
                  '<link rel="stylesheet" href="~/css/platform/react-auth/styles.css" />\n'
                  "@RenderBody()\n")
            # A POST handler with no screen: the control that posts to it is the
            # shared one, rendered by the layout.
            write(root, "Pages/Logout.cshtml",
                  "@page\n@model Demo.LogoutModel\n@{\n    ViewData[\"Title\"] = \"Out\";\n}\n")
            report = self._report(root)
        self.assertNotIn(ui.UI_AUTH_HANDROLLED, report.codes())
        note = self._note(report, "Pages/Logout.cshtml")
        self.assertIn("Pages/Shared/_Layout.cshtml", note)

    def test_handler_page_is_reported_when_the_layout_resolves_to_nothing_shared(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/_ViewStart.cshtml", '@{\n    Layout = "_Layout";\n}\n')
            write(root, "Pages/Shared/_Layout.cshtml", '<link rel="stylesheet" href="~/css/site.css" />\n')
            write(root, "Pages/Logout.cshtml", "@page\n@model Demo.LogoutModel\n")
            report = self._report(root)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())

    def test_handler_page_is_reported_when_no_layout_resolves(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/Logout.cshtml", "@page\n@model Demo.LogoutModel\n")
            report = self._report(root)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())

    def test_handler_page_is_reported_without_a_declared_adoption(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {"scope": {"roots": ["Pages"]}})
            write(root, "Pages/Home.cshtml", '<main class="platform-shell"></main>\n')
            write(root, "Pages/_ViewStart.cshtml", '@{\n    Layout = "_Layout";\n}\n')
            write(root, "Pages/Shared/_Layout.cshtml",
                  '<link rel="stylesheet" href="~/css/platform/react-auth/styles.css" />\n')
            write(root, "Pages/Logout.cshtml", "@page\n@model Demo.LogoutModel\n")
            report = self._report(root)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())

    def test_non_razor_file_that_renders_nothing_is_still_reported(self):
        with scratch() as tmp:
            root = Path(tmp)
            self._adopter(root)
            write(root, "Pages/Login.tsx", "export const Login = 1;\n")
            report = self._report(root)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())

    def test_a_layout_outside_the_project_is_never_read(self):
        with scratch() as tmp:
            root = Path(tmp) / "demo"
            root.mkdir()
            (Path(tmp) / "secret.cshtml").write_text(
                '<link rel="stylesheet" href="~/css/platform/react-auth/styles.css" />\n',
                encoding="utf-8",
            )
            self._adopter(root)
            write(root, "Pages/Login.cshtml", '@layout="../../secret.cshtml"\n')
            report = self._report(root)
        self.assertIn(ui.UI_AUTH_HANDROLLED, report.codes())


class ScopeCeilingTests(unittest.TestCase):
    def test_over_ceiling_repository_names_its_count(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {"scope": {"roots": ["src"], "max_files": 2}})
            for index in range(5):
                write(root, f"src/Page{index}.tsx", "export const P = 1;\n")
            policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
            report = ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertEqual(report.summary["ui_files"], 5)
        self.assertTrue(any("over the 2-file ceiling" in note for note in report.notes))

    def test_content_rules_are_confined_to_the_declared_scope(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, {
                "scope": {"roots": ["src"]},
                "shared_layer": {"adopted": True},
            })
            write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")
            # Outside the declared scope: never opened, so never reported.
            write(root, "legacy/old.css", ".x { color: #ff00aa; }\n")
            policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
            report = ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertEqual(report.summary["ui_files"], 2)
        self.assertEqual(report.summary["files_in_scope"], 1)
        self.assertNotIn(ui.UI_COLOR_LITERAL, report.codes())

    def test_no_declared_scope_opens_no_file(self):
        with scratch() as tmp:
            root = Path(tmp)
            declare(root, None)
            write(root, "src/App.css", ".x { color: #ff00aa; }\n")
            policy = ui.parse_ui_policy(ui._load_declaration(root / ".project.json"))
            report = ui.evaluate(root, policy, token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE)
        self.assertEqual(report.summary["files_in_scope"], 0)
        self.assertNotIn(ui.UI_COLOR_LITERAL, report.codes())
        self.assertTrue(any("no file was opened" in note for note in report.notes))


class BaselineTests(unittest.TestCase):
    def _entry(self, codes: list[str]) -> dict[str, object]:
        return {
            "project": "demo",
            "role": "consumer",
            "ui_files": 3,
            "scope_ceiling": 400,
            "scope_oversize": False,
            "scope_declared": False,
            "codes": codes,
            "counts": {code: 1 for code in codes},
        }

    def test_covered_code_is_suppressed_and_counted(self):
        report = ui.Report(schema_version=1, status="PASS", project="demo")
        report.findings = [
            ui._finding(ui.UI_AUTH_HANDROLLED, "ERROR", "src/Login.tsx", 0, "Login.tsx", "x"),
        ]
        ui.apply_baseline(report, [ui.UI_AUTH_HANDROLLED])
        self.assertEqual(report.findings, [])
        self.assertEqual(report.summary["baselined"], 1)
        self.assertEqual(report.status, "PASS")
        self.assertEqual(ui.exit_code(report), 0)

    def test_uncovered_code_still_fails(self):
        report = ui.Report(schema_version=1, status="PASS", project="demo")
        report.findings = [
            ui._finding(ui.UI_COLOR_LITERAL, "ERROR", "src/a.css", 1, "#ff00aa", "x"),
        ]
        ui.apply_baseline(report, [ui.UI_AUTH_HANDROLLED])
        self.assertEqual(report.status, "FAIL")
        self.assertEqual(ui.exit_code(report), 1)

    def test_uncovered_review_required_still_exits_two(self):
        report = ui.Report(schema_version=1, status="PASS", project="demo")
        report.findings = [
            ui._finding(ui.UI_SCOPE_UNDECLARED, "REVIEW_REQUIRED", ".project.json", 0, "x", "x"),
        ]
        ui.apply_baseline(report, [ui.UI_AUTH_HANDROLLED])
        self.assertEqual(report.status, "REVIEW_REQUIRED")
        self.assertEqual(ui.exit_code(report), 2)

    def test_missing_baseline_file_is_an_empty_baseline(self):
        self.assertEqual(ui.load_baseline(Path("/nonexistent/baseline.json")), {})

    def test_malformed_baseline_is_a_configuration_error(self):
        with scratch() as tmp:
            path = Path(tmp) / "baseline.json"
            path.write_text(json.dumps({"entries": [{"project": "demo"}]}), encoding="utf-8")
            with self.assertRaises(ui.ConfigError):
                ui.load_baseline(path)

    def test_generator_renders_the_recorded_shape(self):
        text = baseline_generator.render([self._entry([ui.UI_AUTH_HANDROLLED])])
        document = json.loads(text)
        self.assertEqual(document["schema_version"], 1)
        self.assertEqual(document["generator"], "scripts/generate_ui_conformance_baseline.py")
        self.assertIn("only shrinks", document["note"])
        self.assertEqual(document["entries"][0]["project"], "demo")

    def test_generator_check_reports_drift(self):
        with scratch() as tmp:
            root = Path(tmp)
            out = root / "baseline.json"
            out.write_text(baseline_generator.render([self._entry([ui.UI_THEME_ABSENT])]), encoding="utf-8")
            stale = baseline_generator._stale_projects(out, [self._entry([ui.UI_AUTH_HANDROLLED])])
        self.assertEqual(stale, ["demo"])

    def test_generator_check_is_silent_when_current(self):
        with scratch() as tmp:
            root = Path(tmp)
            out = root / "baseline.json"
            measured = [self._entry([ui.UI_AUTH_HANDROLLED])]
            out.write_text(baseline_generator.render(measured), encoding="utf-8")
            stale = baseline_generator._stale_projects(out, measured)
        self.assertEqual(stale, [])

    def test_generator_refuses_an_empty_ratchet(self):
        with scratch() as tmp:
            with self.assertRaises(baseline_generator.BaselineRootError):
                baseline_generator.collect(
                    Path(tmp), token_source=TOKEN_SOURCE, module_path=TOKEN_MODULE, node_timeout=30,
                )

    def test_recorded_roles(self):
        self.assertEqual(baseline_generator.record_role("alltools-platform"), "tooling")
        self.assertEqual(baseline_generator.record_role("dotnet-platform-libs"), "provider")
        self.assertEqual(baseline_generator.record_role("fotofy"), "consumer")


class CliTests(unittest.TestCase):
    def _conformant(self, root: Path) -> None:
        declare(root, {
            "scope": {"roots": ["src"]},
            "shared_layer": {"adopted": True},
        })
        write(root, "src/App.css", "[data-theme='dark'] { color: #0f172a; }\n")

    def test_exit_zero_on_a_conformant_repository(self):
        with scratch() as tmp:
            root = Path(tmp) / "demo"
            root.mkdir()
            self._conformant(root)
            missing = Path(tmp) / "no-baseline.json"
            code = ui.main([
                str(root), "--json",
                "--baseline", str(missing),
                "--tokens", str(TOKEN_SOURCE), "--token-module", str(TOKEN_MODULE),
            ])
        self.assertEqual(code, 0)

    def test_exit_one_on_a_blocking_finding(self):
        with scratch() as tmp:
            root = Path(tmp) / "demo"
            root.mkdir()
            self._conformant(root)
            write(root, "src/LoginPage.tsx", "export const Login = 1;\n")
            missing = Path(tmp) / "no-baseline.json"
            code = ui.main([
                str(root), "--json",
                "--baseline", str(missing),
                "--tokens", str(TOKEN_SOURCE), "--token-module", str(TOKEN_MODULE),
            ])
        self.assertEqual(code, 1)

    def test_exit_two_on_a_configuration_error(self):
        with scratch() as tmp:
            root = Path(tmp) / "demo"
            root.mkdir()
            write(root, "src/App.css", ".x { color: #0f172a; }\n")
            write(root, ".project.json", json.dumps({"ui": {"scope": {"roots": ["/etc"]}}}))
            missing = Path(tmp) / "no-baseline.json"
            code = ui.main([str(root), "--baseline", str(missing)])
        self.assertEqual(code, 2)

    def test_missing_project_directory_is_a_configuration_error(self):
        self.assertEqual(ui.main(["/nonexistent/project", "--baseline", "/nonexistent/baseline.json"]), 2)

    def test_the_load_bearing_proof(self):
        """A conformant repository that then hand-rolls a login must fail."""
        with scratch() as tmp:
            root = Path(tmp) / "demo"
            root.mkdir()
            self._conformant(root)
            missing = Path(tmp) / "no-baseline.json"
            argv = [str(root), "--baseline", str(missing),
                    "--tokens", str(TOKEN_SOURCE), "--token-module", str(TOKEN_MODULE)]
            self.assertEqual(ui.main(argv), 0, "precondition: conformant before the change")
            write(root, "src/pages/SignIn.tsx", "export const SignIn = 1;\n")
            self.assertEqual(ui.main(argv), 1, "the login page is load-bearing")
            (root / "src/pages/SignIn.tsx").unlink()
            self.assertEqual(ui.main(argv), 0, "reverting restores the pass")


class ShippedBaselineTests(unittest.TestCase):
    def test_the_committed_baseline_matches_its_generator(self):
        path = ui._governance_root() / ui.BASELINE_FILE
        self.assertTrue(path.is_file(), f"{ui.BASELINE_FILE} must be committed")
        document = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(document["schema_version"], 1)
        self.assertEqual(document["generator"], "scripts/generate_ui_conformance_baseline.py")
        for entry in document["entries"]:
            self.assertIn(entry["project"], {p["path"] for p in _registry_projects()})
            for code in entry["codes"]:
                self.assertIn(code, ui.ALL_CODES)


def _registry_projects() -> list[dict[str, str]]:
    root = ui._governance_root()
    document = json.loads((root / "projects.json").read_text(encoding="utf-8"))
    return [p for p in document["projects"] if isinstance(p, dict) and isinstance(p.get("path"), str)]


if __name__ == "__main__":
    unittest.main()