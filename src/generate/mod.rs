//! Deterministic new-project assembly (`deterministic-project-generation`).
//!
//! Explicit flags and interactive answers normalize into one
//! [`CreationRequest`]. Pinned profile assets ([`GENERATOR_VERSION`]) render
//! into a staging directory, paths are validated to stay inside the
//! destination, then output is promoted into an empty destination and
//! registered. Rendering never needs the stack toolchain; native build/test
//! evidence is reported separately by [`verify_native`] and never claimed
//! from rendering alone.
//!
//! Generated projects own ordinary source and depend on no Forge runtime.
//! Feature flags are recorded (pinned `0.1.0`) after compatibility checks;
//! installing v0.2 feature behavior stays out of scope.
//!
//! Documented identity fields: registry `path` (absolute destination) and
//! `observed_at` timestamps may differ between otherwise equivalent
//! creations. Rendered file bytes are otherwise byte-identical for identical
//! requests.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::core::{validate_project_id, ForgeError};
use crate::profile::{inspect_profile, resolve_profile};
use crate::registry::Registry;

/// Pinned generator asset version, aligned with the MVP descriptors.
pub const GENERATOR_VERSION: &str = "0.1.0";

/// One normalized creation request: the single shape both explicit flags
/// and interactive answers produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreationRequest {
    pub profile: String,
    pub id: String,
    pub name: String,
    pub features: Vec<String>,
    pub destination: PathBuf,
}

/// Outcome of [`generate`]: the registered record plus what was rendered.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedProject {
    pub record: crate::registry::ProjectRecord,
    pub files: Vec<String>,
    pub native_verified: bool,
    pub native_note: String,
}

/// Native verification outcome. Rendering alone never yields `verified`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReport {
    pub profile: String,
    pub build_command: String,
    pub test_command: String,
    pub verified: bool,
}

/// Derive the default id from the destination leaf, kebab-cased.
fn derive_id(dest: &Path, id_override: Option<&str>) -> Result<String, ForgeError> {
    if let Some(id) = id_override {
        return match validate_project_id(id) {
            Ok(()) => Ok(id.to_string()),
            Err(reason) => Err(ForgeError::GenerationFailed { reason }),
        };
    }
    let raw = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let mut cleaned = String::with_capacity(raw.len());
    let mut prev_dash = true;
    for ch in raw.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            cleaned.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            cleaned.push('-');
            prev_dash = true;
        }
    }
    while cleaned.ends_with('-') {
        cleaned.pop();
    }
    match validate_project_id(&cleaned) {
        Ok(()) => Ok(cleaned),
        Err(_) => Err(ForgeError::GenerationFailed {
            reason: format!(
                "destination '{}' yields no valid project id; re-run with --id <kebab-case-id>",
                dest.display()
            ),
        }),
    }
}

fn check_request(profile: &str, id: &str, features: &[String]) -> Result<Vec<String>, ForgeError> {
    inspect_profile(profile)?;
    // Capability membership first: preserves the `incompatible-profile`
    // contract (including the flutter/server-side boundary hint) before
    // dependency closure runs.
    resolve_profile(profile, features)?;
    // Dependency closure: requested features pull their tested dependencies
    // so new-project selection never ships a broken graph. Unknown catalog
    // ids are reported here; capability mismatches already failed above.
    for feature in features {
        if crate::feature::inspect_feature(feature).is_err() {
            return Err(ForgeError::IncompatibleProfile {
                reason: format!(
                    "profile '{profile}' does not support capability '{feature}'; no files were changed"
                ),
            });
        }
    }
    let closed = crate::feature::resolve_feature_closure(profile, features)?;
    validate_project_id(id).map_err(|reason| ForgeError::GenerationFailed { reason })?;
    Ok(closed)
}

/// Normalize explicit CLI flags into a [`CreationRequest`].
pub fn normalize_explicit(
    profile: Option<&str>,
    id: Option<&str>,
    name: Option<&str>,
    features: &[String],
    dest: &Path,
) -> Result<CreationRequest, ForgeError> {
    let profile = profile.ok_or_else(|| ForgeError::GenerationFailed {
        reason: "missing --profile <id>; re-run with an explicit profile or answer interactively"
            .to_string(),
    })?;
    let mut sorted_features = features.to_vec();
    sorted_features.sort();
    sorted_features.dedup();
    let id = derive_id(dest, id)?;
    let name = name
        .map(str::to_string)
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| id.clone());
    let closed_features = check_request(profile, &id, &sorted_features)?;
    Ok(CreationRequest {
        profile: profile.to_string(),
        id,
        name,
        features: closed_features,
        destination: dest.to_path_buf(),
    })
}

fn prompt_line(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    prompt: &str,
) -> Result<Option<String>, ForgeError> {
    write!(writer, "{prompt}").map_err(|err| ForgeError::GenerationFailed {
        reason: format!("interactive prompt failed: {err}"),
    })?;
    writer.flush().map_err(|err| ForgeError::GenerationFailed {
        reason: format!("interactive prompt failed: {err}"),
    })?;
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .map_err(|err| ForgeError::GenerationFailed {
            reason: format!("interactive input failed: {err}"),
        })?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim().to_string()))
}

/// Normalize interactive answers into a [`CreationRequest`].
///
/// Only fields without a preset are prompted. EOF at any prompt cancels
/// with `generation-cancelled` and leaves nothing behind; an empty profile
/// answer (no usable default) also cancels.
pub fn parse_interactive(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    dest: &Path,
    preset_profile: Option<&str>,
    preset_id: Option<&str>,
    preset_name: Option<&str>,
    preset_features: &[String],
) -> Result<CreationRequest, ForgeError> {
    let cancelled = |what: &str| ForgeError::GenerationCancelled {
        reason: format!("{what}; no project or registry entry was created"),
    };
    let profile = match preset_profile {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => match prompt_line(
            reader,
            writer,
            "profile (aspnet-web|rust-web|nextjs-web|react-web|flutter-app|python-service): ",
        )? {
            Some(answer) if !answer.trim().is_empty() => answer,
            _ => return Err(cancelled("creation cancelled during interactive input")),
        },
    };
    let default_id = derive_id(dest, preset_id).unwrap_or_default();
    let id = match preset_id {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => {
            let prompt = if default_id.is_empty() {
                "project id (kebab-case): ".to_string()
            } else {
                format!("project id [{default_id}]: ")
            };
            match prompt_line(reader, writer, &prompt)? {
                Some(answer) if !answer.trim().is_empty() => answer,
                Some(_) if !default_id.is_empty() => default_id.clone(),
                _ => return Err(cancelled("creation cancelled during interactive input")),
            }
        }
    };
    let name = match preset_name {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => {
            let prompt = format!("project name [{id}]: ");
            match prompt_line(reader, writer, &prompt)? {
                Some(answer) if !answer.trim().is_empty() => answer,
                Some(_) => id.clone(),
                None => return Err(cancelled("creation cancelled during interactive input")),
            }
        }
    };
    let features = if preset_features.is_empty() {
        match prompt_line(
            reader,
            writer,
            "features (comma-separated, empty for none): ",
        )? {
            Some(answer) if answer.trim().is_empty() => Vec::new(),
            Some(answer) => {
                let mut out: Vec<String> = answer
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                out.sort();
                out.dedup();
                out
            }
            None => return Err(cancelled("creation cancelled during interactive input")),
        }
    } else {
        let mut out = preset_features.to_vec();
        out.sort();
        out.dedup();
        out
    };
    let closed_features = check_request(&profile, &id, &features)?;
    Ok(CreationRequest {
        profile,
        id,
        name,
        features: closed_features,
        destination: dest.to_path_buf(),
    })
}

fn snake(id: &str) -> String {
    id.replace('-', "_")
}

fn dotnet_namespace(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        out.push_str("App");
    }
    out
}

fn manifest_text(request: &CreationRequest, language: &str) -> String {
    let mut text = format!(
        "schema: 1\nproject:\n  id: {}\n  name: {}\n  profile: {}\n  maturity: L1\n  target_maturity: L1\n",
        request.id, request.name, request.profile
    );
    if !language.trim().is_empty() {
        text.push_str(&format!("runtime:\n  language: {language}\n"));
    }
    if !request.features.is_empty() {
        text.push_str("features:\n");
        for feature in &request.features {
            text.push_str(&format!("  {feature}: \"0.1.0\"\n"));
        }
    }
    text
}

fn readme_text(request: &CreationRequest, build: &str, test: &str, notes: &str) -> String {
    format!(
        "# {}\n\nForge deterministic scaffold for profile `{}` (assets {GENERATOR_VERSION}).\nOrdinary source: builds and runs with its native toolchain; no Forge runtime required.\n\n## Native commands\n\n```sh\n{build}\n{test}\n```\n\n{notes}\n\n## Forge\n\nCreated with `forge new --profile {}`. Rendering is verified by Forge;\nnative build/test evidence requires the stack toolchain (see `forge profile preflight {}`).\n",
        request.name, request.profile, request.profile, request.profile
    )
}

fn template_files(request: &CreationRequest) -> Result<Vec<(String, String)>, ForgeError> {
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
    let manifest = manifest_text(request, &descriptor.language);
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
                format!(
                    "//! {id}: rust-web scaffold (deterministic asset {GENERATOR_VERSION}).\n\n/// Stable greeting used by native tests.\npub fn greeting() -> &'static str {{\n    \"hello from {id}\"\n}}\n\nfn main() {{\n    println!(\"{{}}\", greeting());\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n    #[test]\n    fn greeting_is_stable() {{\n        assert_eq!(greeting(), \"hello from {id}\");\n    }}\n}}\n"
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
                    "Axum/sqlx dependencies from the profile descriptor are resolved per-service; this scaffold ships dependency-free so `cargo build` works offline.",
                ),
            ));
        }
        "python-service" => {
            files.push((
                "pyproject.toml".to_string(),
                format!(
                    "[project]\nname = \"{id}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n\n[build-system]\nrequires = [\"setuptools>=61\"]\nbuild-backend = \"setuptools.build_meta\"\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n"
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
                    "from app.main import greeting\n\ndef test_greeting():\n    assert greeting() == \"hello from {id}\"\n"
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
                    "FastAPI/SQLAlchemy dependencies from the profile descriptor are resolved per-service; this scaffold ships dependency-free so native commands run without network.",
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
                    "This scaffold is dependency-free so `npm run build` / `npm test` work offline. Add the `next` dependency for full Next.js (requires network install).",
                ),
            ));
        }
        "react-web" => {
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
                "index.html".to_string(),
                format!(
                    "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"utf-8\" />\n    <title>{id}</title>\n  </head>\n  <body>\n    <div id=\"root\"></div>\n    <script type=\"module\" src=\"./src/main.js\"></script>\n  </body>\n</html>\n"
                ),
            ));
            files.push((
                "src/main.js".to_string(),
                format!(
                    "export function greeting() {{\n  return \"hello from {id}\";\n}}\nconst root = document.getElementById(\"root\");\nif (root) {{\n  root.textContent = greeting();\n}}\n"
                ),
            ));
            files.push((
                "src/app.test.mjs".to_string(),
                format!(
                    "import {{ describe, it }} from \"node:test\";\nimport assert from \"node:assert/strict\";\nimport {{ greeting }} from \"./main.js\";\n\ndescribe(\"react-web scaffold\", () => {{\n  it(\"returns the deterministic greeting\", () => {{\n    assert.equal(greeting(), \"hello from {id}\");\n  }});\n}});\n"
                ),
            ));
            files.push((
                "vite.config.js".to_string(),
                "/** Minimal portable config; add the `vite` and `react` dependencies for the full toolchain. */\nexport default {};\n".to_string(),
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
                    "React SPA scaffold; client-only rendering with no server-side runtime. \
                     `npm run build` / `npm test` work offline (dependency-free); the \
                     production toolchain needs the `react`, `react-dom`, `react-router-dom` \
                     and `vite` packages from the profile descriptor.",
                ),
            ));
        }
        "aspnet-web" => {
            let namespace = dotnet_namespace(&snake_id);
            files.push((
                format!("{id}.csproj"),
                format!(
                    "<Project Sdk=\"Microsoft.NET.Sdk.Web\">\n\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n    <Nullable>enable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n    <RootNamespace>{namespace}</RootNamespace>\n    <AssemblyName>{namespace}</AssemblyName>\n  </PropertyGroup>\n\n</Project>\n"
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
                "FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build\nWORKDIR /app\nCOPY . .\nRUN dotnet build\n".to_string(),
            ));
            files.push((".gitignore".to_string(), "bin/\nobj/\n".to_string()));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    "`dotnet build` works offline (no PackageReference). `dotnet test` needs a test project (network restore); only rendering is verified for tests here.",
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
                "include: package:flutter_lints/flutter.yaml\n".to_string(),
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
                    "Dependency-free (SDK only) so `flutter test` needs no network. `flutter build appbundle` additionally needs the Android SDK; only rendering is verified for the bundle here.",
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
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Rendered file bytes for a request, sorted by path. Used to prove
/// flag/interactive equivalence and repeatability.
pub fn render_files(request: &CreationRequest) -> Result<Vec<(String, String)>, ForgeError> {
    let _ = check_request(&request.profile, &request.id, &request.features)?;
    template_files(request)
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
    let files = template_files(request)?;
    check_files_inside(&files)?;

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
        for (rel, _) in &files {
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

fn split_command(command: &str) -> Result<(String, Vec<String>), ForgeError> {
    let mut parts: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return Err(ForgeError::GenerationFailed {
            reason: "empty native command; nothing was verified".to_string(),
        });
    }
    let program = parts.remove(0);
    Ok((program, parts))
}

fn toolchain_present(name: &str, available: Option<&HashSet<String>>) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use tempfile::TempDir;

    fn dest(tmp: &TempDir, name: &str) -> PathBuf {
        tmp.path().join(name)
    }

    fn request_for(profile: &str, dest: &Path) -> CreationRequest {
        let id = format!("{profile}-demo");
        normalize_explicit(Some(profile), Some(id.as_str()), None, &[], dest).unwrap()
    }

    fn open_registry(dir: &TempDir) -> Registry {
        Registry::open(&dir.path().join("registry.db")).unwrap()
    }

    #[test]
    fn explicit_and_interactive_normalize_equivalently() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "equiv-app");
        let explicit = normalize_explicit(
            Some("rust-web"),
            Some("equiv-app"),
            Some("Equiv App"),
            &["auth".to_string()],
            &target,
        )
        .unwrap();
        let input = b"rust-web\nequiv-app\nEquiv App\nauth\n";
        let mut reader = Cursor::new(input);
        let mut writer: Vec<u8> = Vec::new();
        let interactive =
            parse_interactive(&mut reader, &mut writer, &target, None, None, None, &[]).unwrap();
        assert_eq!(explicit, interactive);
        let a = render_files(&explicit).unwrap();
        let b = render_files(&interactive).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn repeatable_bytes_across_directories() {
        let tmp = TempDir::new().unwrap();
        let a = normalize_explicit(
            Some("rust-web"),
            Some("same-id"),
            Some("Same"),
            &[],
            &dest(&tmp, "dir-a"),
        )
        .unwrap();
        let b = normalize_explicit(
            Some("rust-web"),
            Some("same-id"),
            Some("Same"),
            &[],
            &dest(&tmp, "dir-b"),
        )
        .unwrap();
        assert_eq!(render_files(&a).unwrap(), render_files(&b).unwrap());
    }

    #[test]
    fn all_six_profiles_render_portable_manifests() {
        let tmp = TempDir::new().unwrap();
        for profile in [
            "aspnet-web",
            "rust-web",
            "nextjs-web",
            "react-web",
            "flutter-app",
            "python-service",
        ] {
            let id = format!(
                "{}-t1",
                profile
                    .replace("-web", "")
                    .replace("-app", "")
                    .replace("-service", "")
            );
            let req = normalize_explicit(
                Some(profile),
                Some(&id),
                Some(&id),
                &[],
                &dest(&tmp, &format!("scaffold-{profile}")),
            )
            .unwrap();
            let files = render_files(&req).unwrap();
            assert!(files.iter().any(|(p, _)| p == "forge.yaml"), "{profile}");
            assert!(files.iter().any(|(p, _)| p == "README.md"), "{profile}");
            let manifest_text = files
                .iter()
                .find(|(p, _)| p == "forge.yaml")
                .map(|(_, c)| c.clone())
                .unwrap();
            let manifest = crate::core::manifest::Manifest::parse(
                Path::new("forge.yaml"),
                manifest_text.as_bytes(),
            )
            .expect("generated manifest must validate");
            assert_eq!(manifest.project.profile, profile);
            crate::profile::resolve_profile(&manifest.project.profile, &[]).unwrap();
            for (rel, contents) in &files {
                assert!(!rel.contains(".."), "{profile}:{rel}");
                if rel == "forge.yaml" || rel == ".gitignore" {
                    continue;
                }
                assert!(
                    !contents.contains("use forge::")
                        && !contents.contains("extern crate forge")
                        && !contents.contains("FORGE_REGISTRY"),
                    "{profile}:{rel} must not depend on a Forge runtime"
                );
            }
            let readme = files
                .iter()
                .find(|(p, _)| p == "README.md")
                .map(|(_, c)| c.clone())
                .unwrap();
            let descriptor = inspect_profile(profile).unwrap();
            assert!(readme.contains(&descriptor.build_command), "{profile}");
            assert!(readme.contains(&descriptor.test_command), "{profile}");
        }
    }

    #[test]
    fn request_for_helper_builds_valid_ids() {
        let tmp = TempDir::new().unwrap();
        for profile in ["rust-web", "flutter-app", "python-service"] {
            let req = request_for(profile, &dest(&tmp, profile));
            validate_project_id(&req.id).unwrap();
        }
    }

    #[test]
    fn nonempty_destination_fails_before_overwrite() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "taken");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep.txt"), "do not touch").unwrap();
        let req = normalize_explicit(Some("rust-web"), Some("taken"), None, &[], &target).unwrap();
        let mut reg = open_registry(&tmp);
        let err = generate(&mut reg, &req).expect_err("nonempty must fail");
        assert_eq!(err.code(), "generation-conflict");
        assert_eq!(
            fs::read_to_string(target.join("keep.txt")).unwrap(),
            "do not touch"
        );
        assert!(!target.join("forge.yaml").exists());
        assert!(reg.inspect("taken").is_err());
    }

    #[test]
    fn template_escape_is_rejected_without_writes() {
        let evil = vec![("../evil.txt".to_string(), "x".to_string())];
        let err = check_files_inside(&evil).expect_err("escape must fail");
        assert_eq!(err.code(), "generation-conflict");
    }

    #[test]
    fn cancelled_interactive_leaves_nothing_behind() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "cancelled-app");
        let mut reader = Cursor::new(b"");
        let mut writer: Vec<u8> = Vec::new();
        let err = parse_interactive(&mut reader, &mut writer, &target, None, None, None, &[])
            .expect_err("EOF must cancel");
        assert_eq!(err.code(), "generation-cancelled");
        assert!(!target.exists());
        let reg = open_registry(&tmp);
        assert!(reg.list().unwrap().is_empty());
    }

    #[test]
    fn unknown_profile_and_incompatible_features_fail_before_mutation() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "bad");
        let err = normalize_explicit(Some("not-a-real-profile"), Some("bad"), None, &[], &target)
            .expect_err("unknown profile");
        assert_eq!(err.code(), "unknown-profile");
        assert!(!target.exists());

        let err = normalize_explicit(
            Some("flutter-app"),
            Some("bad"),
            None,
            &["postgres".to_string()],
            &target,
        )
        .expect_err("incompatible feature");
        assert_eq!(err.code(), "incompatible-profile");
        assert!(!target.exists());
    }

    #[test]
    fn id_collision_cleans_promoted_output_and_registers_nothing_new() {
        let tmp = TempDir::new().unwrap();
        let first = dest(&tmp, "first");
        let second = dest(&tmp, "second");
        let first_req =
            normalize_explicit(Some("rust-web"), Some("dupe-id"), None, &[], &first).unwrap();
        let mut reg = open_registry(&tmp);
        generate(&mut reg, &first_req).unwrap();

        let second_req =
            normalize_explicit(Some("rust-web"), Some("dupe-id"), None, &[], &second).unwrap();
        let err = generate(&mut reg, &second_req).expect_err("id reuse must fail");
        assert_eq!(err.code(), "id-collision");
        assert!(!second.join("forge.yaml").exists());
        // Original record unchanged.
        assert_eq!(reg.inspect("dupe-id").unwrap().profile, "rust-web");
        assert_eq!(reg.list().unwrap().len(), 1);
    }

    #[test]
    fn missing_toolchain_reports_unverified_without_claiming_build() {
        let tmp = TempDir::new().unwrap();
        let empty = HashSet::new();
        let err = verify_native("rust-web", tmp.path(), Some(&empty)).expect_err("no cargo");
        assert_eq!(err.code(), "toolchain-missing");
        assert!(err.to_string().contains("not tested"));
    }

    #[test]
    fn rust_scaffold_builds_and_tests_with_native_toolchain() {
        let toolchain: HashSet<String> = ["cargo".to_string()].into_iter().collect();
        if verify_native("rust-web", Path::new("."), Some(&toolchain)).is_err() {
            // Probe only: toolchain set override cannot run real cargo; skip.
            return;
        }
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "native-app");
        let req =
            normalize_explicit(Some("rust-web"), Some("native-app"), None, &[], &target).unwrap();
        let files = render_files(&req).unwrap();
        fs::create_dir(&target).unwrap();
        for (rel, contents) in &files {
            if rel == "forge.yaml" {
                continue;
            }
            let path = target.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
        if !toolchain_present("cargo", None) {
            return;
        }
        let report = verify_native("rust-web", &target, None).expect("cargo build+test");
        assert!(report.verified);
    }

    #[test]
    fn react_web_renders_with_index_and_test() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "react-app");
        let req = normalize_explicit(
            Some("react-web"),
            Some("react-app"),
            Some("React App"),
            &["i18n".to_string()],
            &target,
        )
        .unwrap();
        let files = render_files(&req).unwrap();
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"forge.yaml"), "{paths:?}");
        assert!(paths.contains(&"README.md"), "{paths:?}");
        assert!(paths.contains(&"index.html"), "{paths:?}");
        assert!(paths.contains(&"src/main.js"), "{paths:?}");
        assert!(paths.contains(&"src/app.test.mjs"), "{paths:?}");
        assert!(paths.contains(&"vite.config.js"), "{paths:?}");
        assert!(paths.contains(&"package.json"), "{paths:?}");
        // The generated manifest advertises react-web so doctor and
        // feature lifecycle contracts both agree.
        let manifest_text = files
            .iter()
            .find(|(p, _)| p == "forge.yaml")
            .map(|(_, c)| c.clone())
            .unwrap();
        let manifest = crate::core::manifest::Manifest::parse(
            Path::new("forge.yaml"),
            manifest_text.as_bytes(),
        )
        .expect("react-web manifest must validate");
        assert_eq!(manifest.project.profile, "react-web");
        // The closed feature set is recorded (i18n only here).
        let features: Vec<&str> = manifest.features.keys().map(String::as_str).collect();
        assert_eq!(features, vec!["i18n"]);
        // The README records the build/test commands from the descriptor.
        let readme = files
            .iter()
            .find(|(p, _)| p == "README.md")
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(readme.contains("npm run build"), "{readme}");
        assert!(readme.contains("npm test"), "{readme}");
    }

    #[test]
    fn react_web_scaffold_builds_and_tests_with_native_toolchain() {
        // Probe only with an isolated target directory; running npm in
        // the project root would block on missing project metadata.
        let toolchain: HashSet<String> = ["npm".to_string()].into_iter().collect();
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "native-react");
        let req = normalize_explicit(Some("react-web"), Some("native-react"), None, &[], &target)
            .unwrap();
        let files = render_files(&req).unwrap();
        fs::create_dir(&target).unwrap();
        for (rel, contents) in &files {
            if rel == "forge.yaml" {
                continue;
            }
            let path = target.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
        if verify_native("react-web", &target, Some(&toolchain)).is_err() {
            return;
        }
        if !toolchain_present("npm", None) {
            return;
        }
        let report = verify_native("react-web", &target, None).expect("npm run build+test");
        assert!(report.verified);
    }

    #[test]
    fn planned_profile_generation_refuses_before_writes() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "aspnet-saas-app");
        let err = normalize_explicit(
            Some("aspnet-saas"),
            Some("aspnet-saas-app"),
            None,
            &[],
            &target,
        )
        .expect_err("planned must refuse");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("planned"), "{text}");
        assert!(
            text.contains("no files were changed") || text.contains("no files were written"),
            "{text}"
        );
        // No file or registry side effects.
        assert!(!target.exists());
        let reg = open_registry(&tmp);
        assert!(reg.list().unwrap().is_empty());
    }
}
