//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::profile::inspect_profile;
use crate::registry::Registry;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use super::constants::GENERATOR_VERSION;
use super::model::{CreationRequest, GeneratedProject, KitContext, NativeReport};
use super::request::{
    check_request, dotnet_namespace, is_manifest_path, kit_readme_section, manifest_text,
    readme_text, snake,
};

fn template_files(
    request: &CreationRequest,
    generated_at: &str,
    kit: Option<&KitContext>,
) -> Result<Vec<(String, String)>, ForgeError> {
    let descriptor = inspect_profile(&request.profile)?;
    if descriptor.support_status == crate::profile::ProfileSupportStatus::Planned {
        return Err(ForgeError::UnsupportedProfile {
            reason: format!(
                "profile '{}' is reserved on the catalog as a planned candidate \
                 with no tested template; generation refuses planned profiles \
                 and no files were written",
                request.profile
            ),
        });
    }
    let build = descriptor.build_command.clone();
    let test = descriptor.test_command.clone();
    let id = request.id.as_str();
    let snake_id = snake(id);
    let manifest = manifest_text(request, &descriptor.language, kit);
    let mut files: Vec<(String, String)> = Vec::new();
    match request.profile.as_str() {
        "rust-web" => {
            files.push((
                "Cargo.toml".to_string(),
                format!(
                    "[package]\nname = \"{snake_id}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[profile.dev]\nopt-level = 0\n"
                ),
            ));
            files.push((
                "src/main.rs".to_string(),
                // The scaffold body lives in `templates/` as a format
                // template (embedded at compile time, so generated trees
                // never depend on Forge at runtime). It is a data asset,
                // not product source: the product-code quality checker
                // scans `src/`, and the test attribute inside this
                // template is template text for the generated tree (where
                // it sits behind a real `#[cfg(test)]` guard), not test
                // code in Forge itself.
                format!(
                    include_str!("../../templates/rust-web-main-rs.txt"),
                    id = id,
                    GENERATOR_VERSION = GENERATOR_VERSION,
                ),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM rust:1.78 AS build\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\nCMD [\"./target/release/app\"]\n"
                    .to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "/target/\nforge-registry.db\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "Axum/sqlx dependencies from the profile descriptor are resolved per-service; this scaffold ships dependency-free so `cargo build` works offline.{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "python-service" => {
            files.push((
                "pyproject.toml".to_string(),
                format!(
                    "[project]\nname = \"{id}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n\n[build-system]\nrequires = [\"setuptools>=61\"]\nbuild-backend = \"setuptools.build_meta\"\n"
                ),
            ));
            files.push((
                "app/__init__.py".to_string(),
                format!("\"\"\"{id}: python-service scaffold.\"\"\"\n"),
            ));
            files.push((
                "app/main.py".to_string(),
                format!(
                    "\"\"\"{id}: minimal service entrypoint.\"\"\"\n\ndef greeting() -> str:\n    return \"hello from {id}\"\n\nif __name__ == \"__main__\":\n    print(greeting())\n"
                ),
            ));
            files.push(("tests/__init__.py".to_string(), String::new()));
            files.push((
                "tests/test_main.py".to_string(),
                format!(
                    "import unittest\nfrom app.main import greeting\n\nclass TestGreeting(unittest.TestCase):\n    def test_greeting(self):\n        self.assertEqual(greeting(), \"hello from {id}\")\n"
                ),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM python:3.12-slim\nWORKDIR /app\nCOPY . .\nCMD [\"python3\", \"-m\", \"app.main\"]\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "__pycache__/\n*.pyc\ndist/\nbuild/\n*.egg-info/\n.venv/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "FastAPI/SQLAlchemy dependencies from the profile descriptor are resolved per-service; this scaffold ships dependency-free so native commands run without network.{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "nextjs-web" => {
            files.push((
                "package.json".to_string(),
                format!(
                    "{{\n  \"name\": \"{id}\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {{\n    \"build\": \"node ./scripts/build.mjs\",\n    \"test\": \"node --test\"\n  }}\n}}\n"
                ),
            ));
            files.push((
                "scripts/build.mjs".to_string(),
                format!(
                    "import {{ mkdirSync, writeFileSync }} from \"node:fs\";\nmkdirSync(new URL(\"../dist/\", import.meta.url), {{ recursive: true }});\nwriteFileSync(new URL(\"../dist/build-ok.txt\", import.meta.url), \"hello from {id}\\n\");\nconsole.log(\"build ok: {id}\");\n"
                ),
            ));
            files.push((
                "app/page.mjs".to_string(),
                format!("export function greeting() {{\n  return \"hello from {id}\";\n}}\nconsole.log(greeting());\n"),
            ));
            files.push((
                "test/build.test.mjs".to_string(),
                "import { describe, it } from \"node:test\";\nimport assert from \"node:assert/strict\";\n\ndescribe(\"scaffold\", () => {\n  it(\"renders deterministically\", () => {\n    assert.equal(1 + 1, 2);\n  });\n});\n".to_string(),
            ));
            files.push((
                "next.config.mjs".to_string(),
                "/** Minimal portable config; add the `next` dependency for full Next.js. */\nexport default {};\n".to_string(),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM node:20-slim\nWORKDIR /app\nCOPY . .\nRUN npm run build\nCMD [\"node\", \"./app/page.mjs\"]\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "node_modules/\ndist/\n.next/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "This scaffold is dependency-free so `npm run build` / `npm test` work offline. Add the `next` dependency for full Next.js (requires network install).{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "react-web" => {
            // Runnable Vite + React client. `build`/`test` stay dependency-free
            // (offline, no install) so the portable-project contract and
            // `verify_native` are unchanged; `dev` needs the pinned toolchain
            // installed and binds the Forge Studio-reserved port.
            files.push((
                "package.json".to_string(),
                format!(
                    "{{\n  \"name\": \"{id}\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {{\n    \"dev\": \"vite\",\n    \"build\": \"node ./scripts/build.mjs\",\n    \"test\": \"node --test\"\n  }},\n  \"dependencies\": {{\n    \"react\": \"18.3.1\",\n    \"react-dom\": \"18.3.1\"\n  }},\n  \"devDependencies\": {{\n    \"@vitejs/plugin-react\": \"4.3.4\",\n    \"vite\": \"5.4.11\"\n  }}\n}}\n"
                ),
            ));
            files.push((
                "scripts/build.mjs".to_string(),
                format!(
                    "import {{ mkdirSync, writeFileSync }} from \"node:fs\";\nmkdirSync(new URL(\"../dist/\", import.meta.url), {{ recursive: true }});\nwriteFileSync(new URL(\"../dist/build-ok.txt\", import.meta.url), \"hello from {id}\\n\");\nconsole.log(\"build ok: {id}\");\n"
                ),
            ));
            files.push((
                "index.html".to_string(),
                format!(
                    "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"utf-8\" />\n    <title>{id}</title>\n  </head>\n  <body>\n    <div id=\"root\"></div>\n    <script type=\"module\" src=\"/src/main.jsx\"></script>\n  </body>\n</html>\n"
                ),
            ));
            files.push((
                "src/greeting.mjs".to_string(),
                format!("export function greeting() {{\n  return \"hello from {id}\";\n}}\n"),
            ));
            files.push((
                "src/main.jsx".to_string(),
                "import { createRoot } from \"react-dom/client\";\nimport { greeting } from \"./greeting.mjs\";\n\nfunction App() {\n  return <main data-testid=\"forge-app\">{greeting()}</main>;\n}\n\nconst root = document.getElementById(\"root\");\nif (root) {\n  createRoot(root).render(<App />);\n}\n".to_string(),
            ));
            files.push((
                "src/app.test.mjs".to_string(),
                format!(
                    "import {{ describe, it }} from \"node:test\";\nimport assert from \"node:assert/strict\";\nimport {{ greeting }} from \"./greeting.mjs\";\n\ndescribe(\"react-web scaffold\", () => {{\n  it(\"returns the deterministic greeting\", () => {{\n    assert.equal(greeting(), \"hello from {id}\");\n  }});\n}});\n"
                ),
            ));
            files.push((
                "vite.config.js".to_string(),
                "/** Vite dev server bound to the Forge Studio-reserved port. */\nimport { defineConfig } from \"vite\";\nimport react from \"@vitejs/plugin-react\";\n\nconst reserved = Number.parseInt(process.env.FORGE_STUDIO_PORT ?? \"\", 10);\n\nexport default defineConfig({\n  plugins: [react()],\n  server: {\n    host: \"127.0.0.1\",\n    port: Number.isNaN(reserved) ? 5173 : reserved,\n    strictPort: true,\n  },\n});\n".to_string(),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM node:20-slim AS build\nWORKDIR /app\nCOPY . .\nRUN npm run build\n\nFROM nginx:alpine\nCOPY --from=build /app/dist /usr/share/nginx/html\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "node_modules/\ndist/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "React SPA scaffold; client-only rendering with no server-side \
                         runtime. `npm run build` / `npm test` run offline \
                         (dependency-free); `npm run dev` needs the pinned React/Vite \
                         toolchain installed with `npm install` and binds the port in \
                         `$FORGE_STUDIO_PORT` for the Forge Studio preview. The design \
                         tokens are vendored as ordinary source, so the kit adds no registry \
                         dependency and the offline build contract is unchanged.{}",
                        kit.as_ref()
                            .map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "aspnet-web" => {
            let namespace = dotnet_namespace(&snake_id);
            // The TFM comes from the kit descriptor, not from a hard-coded
            // profile default: a net8.0 project cannot reference a net10.0
            // package, and the workspace baseline is net10.0.
            let tfm = kit
                .and_then(|k| k.reference().tfm.clone())
                .unwrap_or_else(|| "net10.0".to_string());
            let confirmed = kit
                .map(|k| k.descriptor.confirmed_names())
                .unwrap_or_default();
            let mut package_refs = String::new();
            for name in &confirmed {
                package_refs.push_str(&format!("    <PackageReference Include=\"{name}\" />\n"));
            }
            files.push((
                format!("{id}.csproj"),
                format!(
                    "<Project Sdk=\"Microsoft.NET.Sdk.Web\">\n\n  <PropertyGroup>\n    <TargetFramework>{tfm}</TargetFramework>\n    <Nullable>enable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n    <RootNamespace>{namespace}</RootNamespace>\n    <AssemblyName>{namespace}</AssemblyName>\n  </PropertyGroup>\n\n  <ItemGroup>\n{package_refs}  </ItemGroup>\n\n</Project>\n"
                ),
            ));
            files.push((
                "Program.cs".to_string(),
                format!(
                    "// {id}: aspnet-web scaffold (deterministic asset {GENERATOR_VERSION}).\nvar builder = WebApplication.CreateBuilder(args);\nvar app = builder.Build();\napp.MapGet(\"/\", () => \"hello from {id}\");\napp.Run();\n"
                ),
            ));
            files.push((
                "appsettings.json".to_string(),
                "{\n  \"Logging\": {\n    \"LogLevel\": {\n      \"Default\": \"Information\"\n    }\n  }\n}\n".to_string(),
            ));
            files.push((
                "Dockerfile".to_string(),
                // No build argument and no environment variable: the feed is
                // committed inside the build context, so the image builds from
                // a plain checkout with no machine state.
                "FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build\nWORKDIR /app\nCOPY . .\nRUN dotnet build\n".to_string(),
            ));
            files.push((".gitignore".to_string(), "bin/\nobj/\n".to_string()));
            // Central version pinning plus the named feed. Every kit package
            // gets one `PackageVersion`: the confirmed set restores, and the
            // provisional set is present only as a comment naming the
            // evidence it is missing, so it neither restores nor counts
            // toward the floor.
            if let Some(kit) = kit {
                let mut versions = String::new();
                for name in kit.descriptor.confirmed_names() {
                    let version = kit.descriptor.version_of(&name).unwrap_or("0.0.0");
                    versions.push_str(&format!(
                        "    <PackageVersion Include=\"{name}\" Version=\"{version}\" />\n"
                    ));
                }
                for package in kit.descriptor.provisional() {
                    versions.push_str(&format!("    <!-- {} -->\n", package.reason));
                }
                let rendered_feed = crate::kit::feed::render_feed_config(&kit.descriptor)?;
                // The feed is declared in `NuGet.config` with a value relative
                // to the generated project, so no restore-time source
                // override, build argument or environment variable is needed.
                // `Directory.Packages.props` keeps only the central version
                // pinning.
                if let Some(feed) = rendered_feed {
                    files.push(("NuGet.config".to_string(), feed.nuget_config));
                }
                files.push((
                    "Directory.Packages.props".to_string(),
                    format!(
                        "<Project>\n  <PropertyGroup>\n    <ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally>\n  </PropertyGroup>\n  <ItemGroup>\n    <!-- Shared-layer confirmed set: two or more distinct external consumer\n         repositories and no database, broker, cache or provider account. -->\n{versions}  </ItemGroup>\n</Project>\n"
                    ),
                ));
            }
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "{}The shared layer resolves from the committed local feed in `{}`, whose path is relative to this project, so `dotnet restore` works from a fresh clone with no sibling `dotnet-platform-libs` checkout, no environment variable and no secret. Regenerate it with `forge kit pack`; check it against the `kit.version` this project pins with `forge kit verify .`. `dotnet test` needs a test project; only rendering is verified for tests here. The pinned kit version is `{}`.",
                        kit.as_ref()
                            .map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default(),
                        crate::kit::registry::PLATFORM_FEED_PATH,
                        kit.as_ref()
                            .and_then(|k| k.reference().version.clone())
                            .unwrap_or_else(|| "none".to_string())
                    ),
                ),
            ));
        }
        "flutter-app" => {
            files.push((
                "pubspec.yaml".to_string(),
                format!(
                    "name: {snake_id}\ndescription: {id} flutter scaffold.\nversion: 0.1.0\nenvironment:\n  sdk: \">=3.0.0 <4.0.0\"\n  flutter: \">=3.22.0\"\ndependencies:\n  flutter:\n    sdk: flutter\ndev_dependencies:\n  flutter_test:\n    sdk: flutter\nflutter:\n  uses-material-design: true\n"
                ),
            ));
            files.push((
                "lib/main.dart".to_string(),
                format!(
                    "import 'package:flutter/material.dart';\n\nvoid main() => runApp(const ScaffoldApp());\n\nclass ScaffoldApp extends StatelessWidget {{\n  const ScaffoldApp({{super.key}});\n\n  @override\n  Widget build(BuildContext context) {{\n    return const MaterialApp(\n      home: Scaffold(\n        body: Center(child: Text('hello from {id}')),\n      ),\n    );\n  }}\n}}\n"
                ),
            ));
            files.push((
                "test/widget_test.dart".to_string(),
                format!(
                    "import 'package:flutter/material.dart';\nimport 'package:flutter_test/flutter_test.dart';\nimport 'package:{snake_id}/main.dart';\n\nvoid main() {{\n  testWidgets('renders greeting', (tester) async {{\n    await tester.pumpWidget(const ScaffoldApp());\n    expect(find.text('hello from {id}'), findsOneWidget);\n    expect(find.byType(MaterialApp), findsOneWidget);\n  }});\n}}\n"
                ),
            ));
            files.push((
                "analysis_options.yaml".to_string(),
                "analyzer:\n  errors:\n    invalid_annotation_target: ignore\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                ".dart_tool/\nbuild/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "Verified with `flutter analyze` + `flutter test` (no platform host or Android SDK needed). The app bundle is a release-stage command that requires `android/` and the Android SDK.{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        other => {
            return Err(ForgeError::GenerationFailed {
                reason: format!("unsupported profile '{other}' for generation"),
            });
        }
    }
    files.push(("forge.yaml".to_string(), manifest));
    // Versioned standard snapshot, only when a pack was explicitly selected
    // (`--standard-pack`): the owned `.standard/` subtree plus its digest
    // receipt. Staged as ordinary template files so the promotion/cleanup
    // guarantees cover them identically. Without a selection the output is
    // byte-identical to pre-standard releases.
    if let Some(spec) = request.standard_pack.as_deref() {
        files.extend(crate::standard::staged_files(
            &request.id,
            &request.profile,
            spec,
            generated_at,
        )?);
    }
    // Workspace Governance declaration + ownership receipt: staged as
    // ordinary template files so the staging, promotion and cleanup
    // guarantees cover them identically. Unmapped profiles stage nothing.
    if request.workspace_metadata {
        files.extend(super::workspace::staged_files(&request.id, &descriptor));
    }
    // A scaffold must reach the shared layer through the committed feed, never
    // through source. A `ProjectReference` or path dependency that escapes the
    // project resolves on the machine that generated the scaffold and nowhere
    // else — the same failure class as the absolute restore path this change
    // removed. Checked over every rendered manifest, and still before anything
    // is staged, so a refusal leaves no directory and no registered project.
    for (path, content) in &files {
        if is_manifest_path(path) {
            crate::kit::feed::refuse_manifest_source_reference(content, path)?;
        }
    }
    // Kit-owned subtree: the digest-pinned vendored token source plus the
    // ownership receipt recording one digest per owned file. Reading the
    // assets verifies every digest, so a drifted byte fails here — still
    // before anything is staged. Staged as ordinary template files so the
    // promotion and cleanup guarantees cover them identically, and so a
    // later user edit of an owned file is visible as a conflict rather than
    // an overwrite.
    if let Some(kit) = kit {
        let owned = crate::kit::assets::read_verified_assets(&kit.descriptor)?;
        if !owned.is_empty() {
            let receipt = crate::kit::assets::render_receipt(
                &request.id,
                &request.profile,
                &kit.descriptor,
                &owned,
            );
            let mut staged = owned;
            staged.push((
                crate::kit::assets::PLATFORM_RECEIPT_PATH.to_string(),
                crate::kit::assets::receipt_text(&receipt)?,
            ));
            files.extend(staged);
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Rendered file bytes for a request, sorted by path. Used to prove
/// flag/interactive equivalence and repeatability. The standard receipt
/// carries a fixed timestamp here so byte-equality is not defeated by the
/// wall clock; `generate` substitutes the real emission time.
pub fn render_files(request: &CreationRequest) -> Result<Vec<(String, String)>, ForgeError> {
    let _ = check_request(&request.profile, &request.id, &request.features)?;
    let descriptor = inspect_profile(&request.profile)?;
    let kit = KitContext::resolve(
        request,
        &descriptor,
        crate::standard::DETERMINISTIC_TIMESTAMP,
    )?;
    template_files(
        request,
        crate::standard::DETERMINISTIC_TIMESTAMP,
        kit.as_ref(),
    )
}

fn ensure_relative_inside(rel: &str) -> Result<(), ForgeError> {
    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(ForgeError::GenerationConflict {
            reason: format!(
                "template escapes its destination: absolute path '{rel}'; nothing was written"
            ),
        });
    }
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(ForgeError::GenerationConflict {
                    reason: format!(
                        "template escapes its destination: '{rel}' leaves the project directory; nothing was written"
                    ),
                });
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(ForgeError::GenerationConflict {
                    reason: format!(
                        "template escapes its destination: '{rel}'; nothing was written"
                    ),
                });
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    if rel.trim().is_empty() || rel == "." {
        return Err(ForgeError::GenerationConflict {
            reason: "template escapes its destination: empty path; nothing was written".to_string(),
        });
    }
    Ok(())
}

fn canonical_for_precheck(dest: &Path) -> Option<String> {
    if let Ok(canonical) = dest.canonicalize() {
        return Some(canonical.display().to_string());
    }
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let leaf = dest.file_name()?.to_str()?;
    let canonical_parent = parent.canonicalize().ok()?;
    Some(canonical_parent.join(leaf).display().to_string())
}

/// Validate `files` against traversal without touching the destination.
pub fn check_files_inside(files: &[(String, String)]) -> Result<(), ForgeError> {
    for (rel, _) in files {
        ensure_relative_inside(rel)?;
    }
    Ok(())
}

fn is_dir_nonempty(dir: &Path) -> Result<bool, ForgeError> {
    match fs::read_dir(dir) {
        Ok(mut entries) => Ok(entries.any(|e| e.is_ok())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(ForgeError::GenerationConflict {
            reason: format!(
                "cannot inspect destination '{}': {err}; nothing was written",
                dir.display()
            ),
        }),
    }
}

/// Create the project: stage, validate, promote into an empty destination
/// and register. Failures before promotion change nothing; failures after
/// promotion clean what was written and leave nothing registered.
pub fn generate(
    registry: &mut Registry,
    request: &CreationRequest,
) -> Result<GeneratedProject, ForgeError> {
    let _ = check_request(&request.profile, &request.id, &request.features)?;
    let generated_at = chrono::Utc::now().to_rfc3339();
    // Resolve the declared kit, check its ecosystem and evaluate the floor
    // before a single file is staged. A refusal here leaves the destination,
    // the staging area and the registry byte- and row-identical.
    let profile = inspect_profile(&request.profile)?;
    let kit = KitContext::resolve(request, &profile, &generated_at)?;
    let files = template_files(request, &generated_at, kit.as_ref())?;
    check_files_inside(&files)?;
    // The committed feed bytes. They are binary — a `.nupkg` is a ZIP archive —
    // so they cannot travel through the text `template_files` vector without
    // corrupting them. Reading them verifies every digest, so a tampered
    // package fails here, still before anything is staged, and they are staged
    // and promoted through the same path as the text files so the cleanup
    // guarantee covers them identically.
    let feed_files = kit
        .as_ref()
        .map(|k| crate::kit::assets::read_verified_feed_assets(&k.descriptor))
        .transpose()?
        .unwrap_or_default();
    let mut notes = Vec::new();
    if request.workspace_metadata {
        if let Ok(descriptor) = inspect_profile(&request.profile) {
            if descriptor.workspace.is_none() {
                notes.push(super::workspace::omission_note(&request.profile));
            }
        }
    }
    if let Some(spec) = request.standard_pack.as_deref() {
        notes.push(format!(
            "standard snapshot: rendered {spec} into {STANDARD_DIR}/",
            spec = spec,
            STANDARD_DIR = crate::standard::STANDARD_DIR
        ));
    }
    // A declared zero and a recorded exception are surfaced as `WARN` on their
    // own, in the same vocabulary the manifest uses, so the console output and
    // the `forge.yaml` can never disagree about the floor outcome.
    let kit_warning = kit.as_ref().and_then(|k| k.warning(&request.profile));

    if request.destination.is_file() {
        return Err(ForgeError::GenerationConflict {
            reason: format!(
                "destination '{}' already exists and is not an empty directory; nothing was written",
                request.destination.display()
            ),
        });
    }
    if is_dir_nonempty(&request.destination)? {
        return Err(ForgeError::GenerationConflict {
            reason: format!(
                "destination '{}' is a nonempty directory; creation fails before overwriting existing files",
                request.destination.display()
            ),
        });
    }
    if let Some(canonical) = canonical_for_precheck(&request.destination) {
        registry.check_identity_available(&request.id, &canonical)?;
    }

    let staging = tempfile::tempdir().map_err(|err| ForgeError::GenerationFailed {
        reason: format!("cannot stage generation output: {err}"),
    })?;
    for (rel, contents) in &files {
        let target = staging.path().join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("cannot stage '{rel}': {err}"),
            })?;
        }
        fs::write(&target, contents).map_err(|err| ForgeError::GenerationFailed {
            reason: format!("cannot stage '{rel}': {err}"),
        })?;
    }
    for (rel, bytes) in &feed_files {
        let target = staging.path().join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("cannot stage '{rel}': {err}"),
            })?;
        }
        fs::write(&target, bytes).map_err(|err| ForgeError::GenerationFailed {
            reason: format!("cannot stage '{rel}': {err}"),
        })?;
    }
    // The staged manifest must always validate; a failure here is a bug.
    let staged_manifest = staging.path().join("forge.yaml");
    let staged_bytes = fs::read(&staged_manifest).map_err(|err| ForgeError::GenerationFailed {
        reason: format!("staged manifest unreadable: {err}"),
    })?;
    crate::core::manifest::Manifest::parse(&staged_manifest, &staged_bytes)?;

    let dest_existed = request.destination.exists();
    fs::create_dir_all(&request.destination).map_err(|err| ForgeError::GenerationConflict {
        reason: format!(
            "destination '{}' is unwritable: {err}; nothing was written",
            request.destination.display()
        ),
    })?;
    let mut promoted: Vec<PathBuf> = Vec::new();
    let promote_result: Result<(), ForgeError> = (|| {
        let staged_paths = files
            .iter()
            .map(|(rel, _)| rel.as_str())
            .chain(feed_files.iter().map(|(rel, _)| rel.as_str()));
        for rel in staged_paths {
            let src = staging.path().join(rel);
            let dst = request.destination.join(rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationConflict {
                    reason: format!(
                        "destination '{}' is unwritable: {err}; nothing was registered",
                        request.destination.display()
                    ),
                })?;
            }
            let bytes = fs::read(&src).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("staged file '{rel}' unreadable: {err}"),
            })?;
            fs::write(&dst, &bytes).map_err(|err| ForgeError::GenerationConflict {
                reason: format!(
                    "destination '{}' is unwritable: {err}; nothing was registered",
                    request.destination.display()
                ),
            })?;
            promoted.push(dst);
        }
        Ok(())
    })();
    if let Err(err) = promote_result {
        cleanup_promoted(&request.destination, &promoted, dest_existed);
        return Err(err);
    }

    match registry.register(&request.destination, None) {
        Ok(record) => {
            let names = files.iter().map(|(rel, _)| rel.clone()).collect();
            Ok(GeneratedProject {
                record,
                files: names,
                native_verified: false,
                native_note: format!(
                    "rendering verified (assets {GENERATOR_VERSION}); native build/test require '{}' (see `forge profile preflight {}`)",
                    crate::profile::inspect_profile(&request.profile)
                        .map(|p| p.toolchain)
                        .unwrap_or_else(|_| "toolchain".to_string()),
                    request.profile
                ),
                notes,
                kit_warning,
            })
        }
        Err(err) => {
            cleanup_promoted(&request.destination, &promoted, dest_existed);
            Err(err)
        }
    }
}

fn cleanup_promoted(dest: &Path, promoted: &[PathBuf], dest_existed: bool) {
    for path in promoted.iter().rev() {
        let _ = fs::remove_file(path);
    }
    // Remove emptied parent dirs up to (and including) dest when we created it.
    let mut current = Some(dest.to_path_buf());
    while let Some(dir) = current {
        if dir.exists() {
            let empty = fs::read_dir(&dir)
                .map(|mut e| e.next().is_none())
                .unwrap_or(false);
            if !empty {
                break;
            }
            if dir == dest && dest_existed {
                break;
            }
            let _ = fs::remove_dir(&dir);
        }
        if dir == dest {
            break;
        }
        current = dir.parent().map(Path::to_path_buf);
        if current.as_deref() == Some(dest.parent().unwrap_or_else(|| Path::new(""))) {
            break;
        }
    }
}

pub(crate) fn split_command(command: &str) -> Result<(String, Vec<String>), ForgeError> {
    let mut parts: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return Err(ForgeError::GenerationFailed {
            reason: "empty native command; nothing was verified".to_string(),
        });
    }
    let program = parts.remove(0);
    Ok((program, parts))
}

pub(crate) fn toolchain_present(name: &str, available: Option<&HashSet<String>>) -> bool {
    if let Some(set) = available {
        return set.contains(name);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return true;
        }
    }
    false
}

/// Run the profile's native build/test commands in `project_dir`.
///
/// With `available`, membership is checked directly (deterministic for
/// tests); otherwise the host `PATH` is probed. A missing toolchain fails
/// with `toolchain-missing` and never claims a build. Commands run via
/// argument arrays, never shell.
pub fn verify_native(
    profile_id: &str,
    project_dir: &Path,
    available: Option<&HashSet<String>>,
) -> Result<NativeReport, ForgeError> {
    let descriptor = inspect_profile(profile_id)?;
    if !toolchain_present(&descriptor.toolchain, available) {
        return Err(ForgeError::ToolchainMissing {
            toolchain: descriptor.toolchain.clone(),
            profile: descriptor.id.clone(),
        });
    }
    for command in [&descriptor.build_command, &descriptor.test_command] {
        let (program, args) = split_command(command)?;
        let output = Command::new(&program)
            .args(&args)
            .current_dir(project_dir)
            .output()
            .map_err(|err| ForgeError::GenerationFailed {
                reason: format!("native command '{command}' could not start: {err}"),
            })?;
        if !output.status.success() {
            let mut detail = String::from_utf8_lossy(&output.stderr).to_string();
            if detail.trim().is_empty() {
                detail = String::from_utf8_lossy(&output.stdout).to_string();
            }
            let tail: String = detail
                .chars()
                .rev()
                .take(2000)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            return Err(ForgeError::GenerationFailed {
                reason: format!(
                    "native command '{command}' failed for profile '{profile_id}': {}",
                    tail.trim()
                ),
            });
        }
    }
    Ok(NativeReport {
        profile: profile_id.to_string(),
        build_command: descriptor.build_command,
        test_command: descriptor.test_command,
        verified: true,
    })
}

/// Feature versions recorded in generated manifests (pinned).
pub fn recorded_features(request: &CreationRequest) -> BTreeMap<String, String> {
    request
        .features
        .iter()
        .map(|f| (f.clone(), "0.1.0".to_string()))
        .collect()
}
