//! `forge use <product>` facade command.
//!
//! The outer Forge workspace is a *thin* control plane. Its job here is to
//! locate the right binary for a product and delegate to it, running in the
//! caller's current directory ("relative use"). Forge adds no AI and no
//! rewriting: it only routes the user to the product's own, accurate tooling.
//!
//! Resolved targets, in priority order:
//!   * in-repo Rust crates under `crates/*` (built into
//!     `<workspace>/target/{debug,release}/<binary>`);
//!   * external products found on `PATH` (e.g. an npm package such as
//!     `gitrunbykaru`, or a Cargo-installed binary);
//!   * npm packages resolved through `npx -y <product>` when not on PATH.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::Format;

/// One product routable through `forge use`.
struct Product {
    /// The id a user types: `forge use <id>`.
    id: &'static str,
    /// Aliases accepted in place of `id`.
    aliases: &'static [&'static str],
    /// The compiled binary name (also the npm/bin name for external ones).
    binary: &'static str,
    /// Cargo package name, used for the `cargo build -p` hint.
    crate_name: &'static str,
    /// `rust` = vendored in-repo crate; `external` = resolved via PATH/npx.
    kind: &'static str,
    /// One-line description for `forge help` / `forge use --list`.
    summary: &'static str,
}

const PRODUCTS: &[Product] = &[
    Product {
        id: "driftwatchdog",
        aliases: &[],
        binary: "driftwatchdog",
        crate_name: "driftwatchdog",
        kind: "rust",
        summary: "Gate policy/execution engine consumed by `forge gate`.",
    },
    Product {
        id: "cargo-audit-boost",
        aliases: &[],
        binary: "cargo-audit-boost",
        crate_name: "cargo-audit-boost",
        kind: "rust",
        summary: "Dependency-audit booster wrapping `cargo audit`.",
    },
    Product {
        id: "cargo-matrix-boost",
        aliases: &[],
        binary: "cargo-matrix-boost",
        crate_name: "cargo-matrix-boost",
        kind: "rust",
        summary: "Feature/version matrix booster for workspace builds.",
    },
    Product {
        id: "cargo-sonar-boost",
        aliases: &["cargo-sonar"],
        binary: "cargo-sonar-boost",
        crate_name: "cargo-sonar-boost",
        kind: "rust",
        summary: "Static-analysis sonar booster for code quality.",
    },
    // External sibling product (npm). Resolved from PATH, then `npx -y`.
    Product {
        id: "gitrunbykaru",
        aliases: &["grbk"],
        binary: "gitrunbykaru",
        crate_name: "gitrunbykaru",
        kind: "external",
        summary: "Git run book: runs tracked git tasks and records outcomes (npm package).",
    },
];

fn resolve_product(id: &str) -> Option<&'static Product> {
    let id = id.trim();
    PRODUCTS
        .iter()
        .find(|p| p.id == id || p.aliases.contains(&id))
}

/// Walk up from the current executable to the Cargo workspace root so the
/// resolver keeps working regardless of the caller's working directory.
pub(crate) fn workspace_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let profile = exe.parent()?; // .../target/<profile>
    let target = profile.parent()?; // .../target
    let root = target.parent()?; // workspace root
    if root.join("Cargo.toml").is_file() {
        Some(root.to_path_buf())
    } else {
        None
    }
}

fn find_in_repo_binary(product: &Product) -> Option<PathBuf> {
    let root = workspace_root()?;
    for profile in ["debug", "release"] {
        let bin = root.join("target").join(profile).join(product.binary);
        if bin.is_file() {
            return Some(bin);
        }
    }
    None
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        for cand in [name, &format!("{name}.exe")] {
            let p = dir.join(cand);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Spawn the resolved binary with inherited stdio and forward the args.
/// Returns the child's exit code (or 127 if the spawn itself failed).
fn delegate(bin: &Path, args: &[String]) -> ExitCode {
    match std::process::Command::new(bin).args(args).status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("forge use: failed to run {}: {e}", bin.display());
            ExitCode::from(127)
        }
    }
}

fn delegate_npx(npx: &Path, product: &str, args: &[String]) -> ExitCode {
    match std::process::Command::new(npx)
        .arg("-y")
        .arg(product)
        .args(args)
        .status()
    {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("forge use: failed to run npx -y {product}: {e}");
            ExitCode::from(127)
        }
    }
}

pub(crate) fn cmd_use(
    list: bool,
    product: Option<&str>,
    args: &[String],
    format: Format,
) -> ExitCode {
    if list {
        print!("{}", render_list());
        return ExitCode::SUCCESS;
    }

    let id = match product {
        Some(p) => p.trim(),
        None => {
            return emit_error(format, "`forge use` requires a product id; try `forge help`.");
        }
    };
    if id.is_empty() {
        return emit_error(format, "`forge use` requires a product id; try `forge help`.");
    }

    // Known catalog product: route by kind.
    if let Some(p) = resolve_product(id) {
        match p.kind {
            "rust" => {
                return match find_in_repo_binary(p) {
                    Some(bin) => delegate(&bin, args),
                    None => emit_error(
                        format,
                        &format!(
                            "product '{}' is not built. Run `cargo build -p {}` (or `cargo build --workspace`) then retry.",
                            p.id, p.crate_name
                        ),
                    ),
                };
            }
            "external" => {
                if let Some(bin) = find_in_path(p.binary) {
                    return delegate(&bin, args);
                }
                if let Some(npx) = find_in_path("npx") {
                    return delegate_npx(&npx, p.binary, args);
                }
                return emit_error(
                    format,
                    &format!(
                        "external product '{}' not found on PATH and `npx` is unavailable. Install it (e.g. `npm i -g {}`) or run `npx -y {}` directly.",
                        p.id, p.binary, p.binary
                    ),
                );
            }
            _ => {}
        }
    }

    // Not in the catalog: still allow a literal binary on PATH (a locally
    // installed tool the catalog does not enumerate), but never auto-run npx
    // for an arbitrary name — that would hit the npm registry on a typo.
    if let Some(bin) = find_in_path(id) {
        return delegate(&bin, args);
    }

    emit_error(
        format,
        &format!(
            "unknown product '{id}'. Known products: {}. For an external tool, install it first or run it directly.",
            PRODUCTS.iter().map(|p| p.id).collect::<Vec<_>>().join(", ")
        ),
    )
}

pub(crate) fn render_help() -> String {
    let mut out = String::new();
    out.push_str("forge — facade (use)\n\n");
    out.push_str("The outer Forge workspace routes you to consolidated tooling crates\n");
    out.push_str("and external sibling products without re-implementing them.\n\n");
    out.push_str("USAGE:\n");
    out.push_str("  forge use <product> [--] [ARGS...]   run a product in the current directory\n");
    out.push_str("  forge use --list                    list routable products\n");
    out.push_str("  forge module list                   discover in-tree modules\n");
    out.push_str("  forge module verify <id>            run one module's Forge Gate\n");
    out.push_str("  forge help                          show this overview\n\n");
    out.push_str("Resolution, in priority order:\n");
    out.push_str("  1. in-repo Rust crate binary   (crates/* -> target/{debug,release}/<bin>)\n");
    out.push_str("  2. external binary on PATH       (e.g. an npm package)\n");
    out.push_str("  3. npm package via `npx -y`      (when not on PATH)\n\n");
    out.push_str("PRODUCTS:\n");
    for p in PRODUCTS {
        let alias = if p.aliases.is_empty() {
            String::new()
        } else {
            format!(" (alias: {})", p.aliases.join(", "))
        };
        out.push_str(&format!("  {:<20} {}{}\n", p.id, p.kind, alias));
        out.push_str(&format!("  {:<20}   {}\n", "", p.summary));
    }
    out.push_str("\nEXAMPLES:\n");
    out.push_str("  forge use driftwatchdog -- --help\n");
    out.push_str("  forge use cargo-audit-boost -- .\n");
    out.push_str("  forge use gitrunbykaru -- --help   # external npm product\n");
    out
}

pub(crate) fn render_list() -> String {
    let mut out = String::new();
    for p in PRODUCTS {
        out.push_str(&format!("{}  [{}]\n", p.id, p.kind));
    }
    out
}

fn emit_error(format: Format, msg: &str) -> ExitCode {
    match format {
        Format::Json | Format::Ndjson => {
            eprintln!(
                "{}",
                serde_json::json!({ "error": { "code": "product-not-found", "message": msg } })
            );
        }
        _ => {
            eprintln!("error: {msg}");
        }
    }
    ExitCode::from(2)
}
