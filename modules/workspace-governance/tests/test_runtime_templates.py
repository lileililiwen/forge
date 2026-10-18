import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))

from runtime_templates import (  # noqa: E402
    TemplateError,
    check_target,
    inspect_target,
    load_registry,
    render_target,
)


class RuntimeTemplateRegistryTests(unittest.TestCase):
    def test_registry_has_ten_supported_types(self):
        registry = load_registry()
        self.assertEqual(
            set(registry),
            {
                "dotnet-web-postgres",
                "rust-web-postgres",
                "python-web-postgres",
                "node-web",
                "static-site",
                "worker-postgres",
                "dotnet-web-postgres-external",
                "rust-web-postgres-external",
                "python-web-postgres-external",
                "worker-postgres-external",
            },
        )
        for entry in registry.values():
            self.assertEqual(entry["version"], 1)
            self.assertTrue(entry["files"])
            self.assertTrue(entry["required_variables"])

    def test_external_templates_list_env_example(self):
        registry = load_registry()
        for template_id in {
            "dotnet-web-postgres-external",
            "rust-web-postgres-external",
            "python-web-postgres-external",
            "worker-postgres-external",
        }:
            self.assertIn(".env.example", registry[template_id]["files"])
            self.assertIn("db_host", registry[template_id]["required_variables"])
            self.assertIn("db_user", registry[template_id]["required_variables"])
            self.assertIn("db_port", registry[template_id].get("optional_variables", []))

    def test_internal_templates_have_no_db_host_variable(self):
        registry = load_registry()
        for template_id in {
            "dotnet-web-postgres",
            "rust-web-postgres",
            "python-web-postgres",
            "worker-postgres",
        }:
            self.assertNotIn("db_host", registry[template_id]["required_variables"])
            self.assertNotIn(".env.example", registry[template_id]["files"])


class RuntimeTemplateInspectionTests(unittest.TestCase):
    def test_dotnet_inspection_suggests_dotnet_template(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "src" / "App.csproj").write_text("<Project />\n", encoding="utf-8")
            result = inspect_target(root)
            self.assertEqual(result["suggestions"], ["dotnet-web-postgres"])
            self.assertIn("src/App.csproj", result["evidence"])

    def test_empty_target_requires_explicit_type(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(TemplateError) as context:
                inspect_target(Path(directory))
            self.assertEqual(context.exception.code, "TYPE_REQUIRED")


class RuntimeTemplateRenderingTests(unittest.TestCase):
    def declaration(self):
        return {
            "schema_version": 1,
            "id": "demo-app",
            "kind": "product",
            "profile": "product",
            "lifecycle": "active",
            "verification": {"command": "./scripts/verify.sh", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
        }

    def variables(self, template_type):
        values = {"project_name": "demo-app"}
        is_external = template_type.endswith("-postgres-external")
        is_worker = template_type in {"worker-postgres", "worker-postgres-external"}
        if not is_worker:
            values.update({"port": "8080", "health_path": "/healthz"})
        if template_type in {"dotnet-web-postgres", "rust-web-postgres"}:
            values.update({"app_project": "src/App/App.csproj" if template_type.startswith("dotnet") else "app", "database_name": "demo"})
        elif template_type == "python-web-postgres":
            values.update({"entrypoint": "app.main:app", "database_name": "demo"})
        elif template_type == "node-web":
            values.update({"build_command": "npm run build", "start_command": "npm run start", "output_dir": "dist"})
        elif template_type == "static-site":
            values.update({"output_dir": "dist"})
        elif template_type == "worker-postgres":
            values.update({"worker_command": "python -m worker", "database_name": "demo"})
        elif template_type == "dotnet-web-postgres-external":
            values.update({"app_project": "src/App/App.csproj", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        elif template_type == "rust-web-postgres-external":
            values.update({"app_project": "app", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        elif template_type == "python-web-postgres-external":
            values.update({"entrypoint": "app.main:app", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        elif template_type == "worker-postgres-external":
            values.update({"worker_command": "python -m worker", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        return values

    def test_render_writes_relative_files_and_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            result = render_target(root, "static-site", self.variables("static-site"))
            self.assertEqual(result["state"], "configured")
            self.assertTrue((root / "Dockerfile").is_file())
            self.assertTrue((root / "compose.yaml").is_file())
            updated = json.loads((root / ".project.json").read_text(encoding="utf-8"))
            self.assertEqual(updated["deployment"]["compose_file"], "compose.yaml")
            self.assertFalse(updated["deployment"]["deployable"])
            self.assertEqual(updated["deployment"]["template_type"], "static-site")

    def test_existing_file_refuses_without_exact_replace(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            (root / "compose.yaml").write_text("existing\n", encoding="utf-8")
            with self.assertRaises(TemplateError) as context:
                render_target(root, "static-site", self.variables("static-site"))
            self.assertEqual(context.exception.code, "TARGET_EXISTS")
            self.assertEqual((root / "compose.yaml").read_text(), "existing\n")

    def test_secret_variable_is_rejected_before_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(TemplateError) as context:
                render_target(Path(directory), "static-site", {"project_name": "demo", "port": "8080", "health_path": "/healthz", "token": "secret"})
            self.assertEqual(context.exception.code, "RENDER_REJECTED")
            self.assertFalse((Path(directory) / "Dockerfile").exists())

    def test_template_output_has_no_unresolved_placeholders(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            render_target(root, "node-web", self.variables("node-web"))
            self.assertEqual(check_target(root)["state"], "configured")
            for path in root.rglob("*"):
                if path.is_file() and path.name != ".project.json":
                    self.assertNotIn("{{", path.read_text(encoding="utf-8"))

    def test_every_template_renders_without_cross_family_files(self):
        for template_type in load_registry():
            with self.subTest(template_type=template_type), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
                result = render_target(root, template_type, self.variables(template_type))
                self.assertEqual(result["template_type"], template_type)
                self.assertTrue((root / "Dockerfile").is_file())
                self.assertTrue((root / "compose.yaml").is_file())
                self.assertIn(check_target(root)["compose_validation"], {"verified", "TOOL_UNAVAILABLE"})


class ExternalPostgresTemplateTests(unittest.TestCase):
    def declaration(self):
        return {
            "schema_version": 1,
            "id": "demo-app",
            "kind": "product",
            "profile": "product",
            "lifecycle": "active",
            "verification": {"command": "./scripts/verify.sh", "gate_runtime": "driftwatchdog", "evidence_status": "planned"},
            "deployment": {"deployable": False, "jenkins_job": None, "compose_file": None},
        }

    def base_variables(self, template_type):
        values = {"project_name": "demo-app"}
        is_worker = template_type == "worker-postgres-external"
        if not is_worker:
            values.update({"port": "8080", "health_path": "/healthz"})
        if template_type == "dotnet-web-postgres-external":
            values.update({"app_project": "src/App/App.csproj", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        elif template_type == "rust-web-postgres-external":
            values.update({"app_project": "app", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        elif template_type == "python-web-postgres-external":
            values.update({"entrypoint": "app.main:app", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        elif template_type == "worker-postgres-external":
            values.update({"worker_command": "python -m worker", "database_name": "demo", "db_user": "demo_role", "db_host": "host.docker.internal"})
        return values

    def test_external_template_has_no_db_service_or_volume(self):
        for template_type in (
            "dotnet-web-postgres-external",
            "rust-web-postgres-external",
            "python-web-postgres-external",
            "worker-postgres-external",
        ):
            with self.subTest(template_type=template_type), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
                render_target(root, template_type, self.base_variables(template_type))
                compose = (root / "compose.yaml").read_text(encoding="utf-8")
                self.assertNotIn("\n  db:", compose)
                self.assertNotIn("postgres:16", compose)
                self.assertNotIn("pg_isready", compose)
                self.assertNotIn("POSTGRES_DB", compose)
                self.assertNotIn("volumes:", compose)
                self.assertNotIn("var/lib/postgresql", compose)

    def test_external_template_records_database_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            render_target(root, "python-web-postgres-external", self.base_variables("python-web-postgres-external"))
            updated = json.loads((root / ".project.json").read_text(encoding="utf-8"))
            self.assertEqual(updated["deployment"]["database_source"], "external")
            self.assertEqual(updated["deployment"]["template_type"], "python-web-postgres-external")
            self.assertIn(".env.example", updated["deployment"]["rendered_files"])

    def test_external_template_writes_blank_password_env_example(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            render_target(root, "python-web-postgres-external", self.base_variables("python-web-postgres-external"))
            env_example = (root / ".env.example").read_text(encoding="utf-8")
            self.assertIn("PGHOST=host.docker.internal", env_example)
            self.assertIn("PGUSER=demo_role", env_example)
            self.assertIn("PGDATABASE=demo", env_example)
            self.assertIn("PGPASSWORD=", env_example)
            self.assertNotIn("PGPASSWORD=demo", env_example)
            self.assertNotIn("PGPASSWORD=secret", env_example)

    def test_external_template_ignores_env_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            render_target(root, "python-web-postgres-external", self.base_variables("python-web-postgres-external"))
            gitignore = (root / ".gitignore").read_text(encoding="utf-8")
            self.assertIn(".env", gitignore.splitlines())
            self.assertNotIn("!.env", gitignore.splitlines())

    def test_external_template_idempotent_env_ignore(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".gitignore").write_text("node_modules/\ntarget/\n", encoding="utf-8")
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            render_target(root, "python-web-postgres-external", self.base_variables("python-web-postgres-external"))
            first_text = (root / ".gitignore").read_text(encoding="utf-8")
            first_lines = [line for line in first_text.splitlines() if line.strip() == ".env"]
            self.assertEqual(len(first_lines), 1)
            self.assertIn("node_modules/", first_text)
            self.assertIn("target/", first_text)
            # Re-render the same family with --replace. The .gitignore is not a
            # generated file, so a second render must not duplicate the ignore
            # entry or re-write unrelated bytes.
            replace = {"Dockerfile", "compose.yaml", ".dockerignore", "README.md", ".env.example", ".project.json"}
            render_target(root, "python-web-postgres-external", self.base_variables("python-web-postgres-external"), replace=replace)
            second_text = (root / ".gitignore").read_text(encoding="utf-8")
            self.assertEqual(first_text, second_text)
            second_lines = [line for line in second_text.splitlines() if line.strip() == ".env"]
            self.assertEqual(len(second_lines), 1)

    def test_external_template_refuses_with_unignore_rule(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".gitignore").write_text("node_modules/\n!.env\n", encoding="utf-8")
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            with self.assertRaises(TemplateError) as context:
                render_target(root, "python-web-postgres-external", self.base_variables("python-web-postgres-external"))
            self.assertEqual(context.exception.code, "SECRET_IGNORE_CONFLICT")
            # No file was written because the .gitignore check runs first.
            self.assertFalse((root / "Dockerfile").exists())
            self.assertFalse((root / "compose.yaml").exists())
            self.assertFalse((root / ".env.example").exists())

    def test_external_template_refuses_with_existing_compose(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            (root / "compose.yaml").write_text("existing\n", encoding="utf-8")
            with self.assertRaises(TemplateError) as context:
                render_target(root, "python-web-postgres-external", self.base_variables("python-web-postgres-external"))
            self.assertEqual(context.exception.code, "TARGET_EXISTS")
            # .env.example was not written and .gitignore was not created.
            self.assertFalse((root / ".env.example").exists())
            self.assertFalse((root / ".gitignore").exists())

    def test_external_template_dockerignore_excludes_env(self):
        for template_type in (
            "dotnet-web-postgres-external",
            "rust-web-postgres-external",
            "python-web-postgres-external",
            "worker-postgres-external",
        ):
            with self.subTest(template_type=template_type), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
                render_target(root, template_type, self.base_variables(template_type))
                dockerignore = (root / ".dockerignore").read_text(encoding="utf-8")
                self.assertIn(".env", dockerignore.splitlines())
                self.assertIn(".env.*", dockerignore.splitlines())
                self.assertIn("!.env.example", dockerignore.splitlines())

    def test_external_template_default_db_port(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            variables = self.base_variables("python-web-postgres-external")
            # db_port is optional; renderer fills in the documented default.
            render_target(root, "python-web-postgres-external", variables)
            env_example = (root / ".env.example").read_text(encoding="utf-8")
            self.assertIn("PGPORT=5432", env_example)
            compose = (root / "compose.yaml").read_text(encoding="utf-8")
            self.assertIn("PGPORT:-5432", compose)

    def test_external_template_rejects_secret_shaped_variable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            variables = self.base_variables("python-web-postgres-external")
            variables["db_password"] = "literal-password-attempt"
            with self.assertRaises(TemplateError) as context:
                render_target(root, "python-web-postgres-external", variables)
            self.assertEqual(context.exception.code, "RENDER_REJECTED")

    def test_internal_template_does_not_create_env_example_or_gitignore(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".project.json").write_text(json.dumps(self.declaration()), encoding="utf-8")
            variables = {"project_name": "demo-app", "port": "8080", "health_path": "/healthz", "entrypoint": "app.main:app", "database_name": "demo"}
            render_target(root, "python-web-postgres", variables)
            self.assertFalse((root / ".env.example").exists())
            self.assertFalse((root / ".gitignore").exists())
            self.assertNotIn("database_source", json.loads((root / ".project.json").read_text(encoding="utf-8"))["deployment"])


if __name__ == "__main__":
    unittest.main()
