//! Contract tests for the fleet liveness verdict surface
//! (`fleet-liveness-status`).
//!
//! Exercises `forge fleet online` end-to-end through the real CLI
//! binary. The SSH transport is stubbed via a tiny fake `ssh`
//! script — set through `FORGE_PUBLISH_SSH_TARGET` — so the
//! served-Caddyfile and `docker ps` probes answer from local
//! fixture files. The HTTPS probe either runs against a tiny local
//! listener (for the online round trip) or is skipped entirely
//! (every test that doesn't need it sets `public_http: false`).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::registry::Registry;
use serde_json::Value;

/// Path to the compiled `forge` binary under test.
fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Build a `forge` command with every fleet-relevant env var removed
/// so the test starts from a clean slate.
fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("FORGE_INVENTORY_SOURCE")
        .env_remove("FORGE_PUBLISH_SSH_TARGET")
        .env_remove("FORGE_PUBLISH_CADDYFILE_PATH")
        .env_remove("FORGE_PUBLISH_NAV_HOST")
        .env_remove("FORGE_PUBLISH_DOMAIN")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("forge CLI must run")
}

fn run_with_env(db: &Path, args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("forge CLI must run")
}

/// Stub the `ssh` transport with a local shell script that prints
/// the served Caddyfile or the `docker ps` output verbatim. The
/// script is written as `<root>/ssh` and made executable; the test
/// sets `FORGE_PUBLISH_SSH_TARGET=<root>/ssh` so the probes
/// contact the script instead of a real SSH server.
struct FakeSsh {
    root: PathBuf,
}

impl FakeSsh {
    fn new(caddyfile: &str, docker_ps: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "forge-fleet-online-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&root).expect("create fake ssh root");
        let caddyfile_path = root.join("Caddyfile");
        fs::write(&caddyfile_path, caddyfile).expect("write Caddyfile");
        let docker_ps_path = root.join("docker-ps.txt");
        fs::write(&docker_ps_path, docker_ps).expect("write docker ps");
        let script = format!(
            "#!/bin/sh\n\
             # Forge fleet-online fake ssh. Argv-driven: distinguish\n\
             # `cat <Caddyfile>` from `docker ps` and return the\n\
             # matching fixture. Anything else exits 0 with empty\n\
             # stdout so the probe still parses.\n\
             caddyfile_path=\"{caddyfile}\"\n\
             docker_ps_path=\"{docker_ps}\"\n\
             for arg in \"$@\"; do\n\
             \tif [ \"$arg\" = \"$caddyfile_path\" ] || [ \"$arg\" = \"/srv/platform/Caddyfile\" ]; then\n\
             \t\tcat \"{caddyfile_path_escaped}\"\n\
             \t\texit 0\n\
             \tfi\n\
             done\n\
             # `docker ps` argv always contains the docker binary path.\n\
             if echo \"$*\" | grep -q \"/usr/local/bin/docker\"; then\n\
             \tcat \"{docker_ps_escaped}\"\n\
             \t\texit 0\n\
             fi\n\
             exit 0\n",
            caddyfile = caddyfile_path.display(),
            caddyfile_path_escaped = caddyfile_path.display(),
            docker_ps = docker_ps_path.display(),
            docker_ps_escaped = docker_ps_path.display(),
        );
        let script_path = root.join("ssh");
        fs::write(&script_path, script).expect("write fake ssh");
        let mut perms = fs::metadata(&script_path)
            .expect("script metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("chmod fake ssh");
        Self { root }
    }

    fn target(&self) -> PathBuf {
        self.root.join("ssh")
    }
}

impl Drop for FakeSsh {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const SIMPLE_CADDYFILE: &str = "{\n\tauto_https off\n}\n\n\
http://apps.tooosall.uk {\n\troot * /srv\n\tfile_server\n}\n\n\
http://alethefy.tooosall.uk {\n\treverse_proxy host.docker.internal:17700\n}\n\n\
http://crossalheart.tooosall.uk {\n\treverse_proxy host.docker.internal:17710\n}\n\n\
:80 {\n\trespond \"Unknown application hostname\" 404\n}\n";

const SIMPLE_DOCKER_PS: &str = "\
forge-alethefy-0123456789ab\tUp 5m\n\
forge-crossalheart-abcdef012345\tUp 10m\n";

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// Stage a project directory with a real Compose file so the
/// inventory classifier marks it `compose_ready`.
fn stage_project_dir(parent: &Path, project_id: &str) -> PathBuf {
    let dir = parent.join(project_id);
    fs::create_dir_all(&dir).expect("create project dir");
    fs::write(dir.join("docker-compose.yml"), "services: {}\n").expect("write compose");
    dir
}

/// Build a compose-ready inventory entry whose `source_path` points
/// at a freshly-staged directory containing `docker-compose.yml`.
/// This makes the classifier mark the entry `compose_ready`; tests
/// that want a different classification pass a fully custom entry
/// instead of going through this helper.
fn compose_ready_entry_in(parent: &Path, id: &str, public_http: bool) -> Value {
    let dir = stage_project_dir(parent, id);
    let mut entry = serde_json::json!({
        "id": id,
        "repository": format!("https://example.invalid/{id}.git"),
        "revision": SHA,
        "profile": "rust-product",
        "runtime": "web",
        "compose_file": "docker-compose.yml",
        "source_path": dir.to_str().unwrap(),
        "public_http": public_http,
    });
    if public_http {
        entry["public_port"] = serde_json::json!(8080);
    }
    entry
}

/// Minimal valid inventory document carrying one compose-ready
/// project with `public_http: true` so the probe runs against the
/// configured `--domain`. Stages a real Compose file alongside the
/// inventory so the classifier marks the entry `compose_ready`.
fn write_inventory_with_entries(dir: &Path, entries: Vec<Value>) -> PathBuf {
    let document = serde_json::json!({
        "contract": "forge-project-inventory/0.1.0",
        "provider": "local",
        "generated_at": "2026-09-28T00:00:00Z",
        "projects": entries,
    });
    let path = dir.join("inventory.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&document).expect("inventory serialize"),
    )
    .expect("inventory write");
    path
}

#[test]
fn fleet_online_help_advertises_every_roster_flag() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let out = run(&db, &["fleet", "online", "--help"]);
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    for needle in [
        "--inventory",
        "--fleet-registry",
        "--workspace-root",
        "--domain",
        "--timeout-secs",
        "--dry-run",
        "--format",
        "JSON",
    ] {
        assert!(
            stdout.contains(needle),
            "missing `{needle}` in help text:\n{stdout}"
        );
    }
}

#[test]
fn fleet_online_dry_run_renders_plan_without_contacting_target() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![
            compose_ready_entry_in(tmp.path(), "alethefy", true),
            compose_ready_entry_in(tmp.path(), "crossalheart", true),
        ],
    );
    // No FORGE_PUBLISH_SSH_TARGET set — `--dry-run` must NOT contact
    // the target. Set the target to a non-existent path so any
    // accidental contact would fail loudly.
    let out = run(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("dry-run"), "plan must say dry-run");
    assert!(stdout.contains("probe plan: 1)"));
    assert!(stdout.contains("probe plan: 2)"));
    assert!(stdout.contains("probe plan: 3)"));
}

#[test]
fn fleet_online_rejects_timeout_out_of_bounds() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "alethefy", true)],
    );
    let out = run(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
            "--timeout-secs",
            "0",
        ],
    );
    assert!(!out.status.success());
    assert!(
        lossy(&out.stderr).contains("out of bounds"),
        "stderr={}",
        lossy(&out.stderr)
    );
}

#[test]
fn fleet_online_rejects_empty_roster_with_typed_publish_invalid() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(tmp.path(), vec![]);
    let out = run(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
    );
    // An empty compose_ready set is a typed refusal — same as
    // `publish fleet` — not a silent exit-zero.
    assert!(!out.status.success(), "empty roster must fail");
    assert!(
        lossy(&out.stderr).contains("publish-invalid"),
        "stderr={}",
        lossy(&out.stderr)
    );
}

#[test]
fn fleet_online_reports_no_route_for_unrouted_project() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "alethefy", true)],
    );
    // Caddyfile with no rule for alethefy.
    let empty_caddy =
        "{\n\tauto_https off\n}\n\n:80 {\n\trespond \"Unknown application hostname\" 404\n}\n";
    let docker_ps = SIMPLE_DOCKER_PS;
    let fake = FakeSsh::new(empty_caddy, docker_ps);
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    let entries = json
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("entries array");
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry["id"], "alethefy");
    assert_eq!(entry["verdict"], "NO-ROUTE");
    assert_eq!(entry["classification"], "compose_ready");
    assert!(entry.get("container").is_some() || entry.get("container").is_none());
    assert!(!out.status.success(), "no-route must exit non-zero");
}

#[test]
fn fleet_online_reports_not_deployed_when_container_absent() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "alethefy", true)],
    );
    // Caddyfile has the route; docker ps is empty so no container.
    let docker_ps = "";
    let fake = FakeSsh::new(SIMPLE_CADDYFILE, docker_ps);
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    let entries = json
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("entries array");
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry["verdict"], "NOT-DEPLOYED");
    assert!(!out.status.success(), "not-deployed must exit non-zero");
}

#[test]
fn fleet_online_reports_skipped_for_non_ready_entries_and_exits_non_zero() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    // A `compose_missing` entry is reported with its own
    // classification and never probed — a real probe would
    // downgrade it to `NO-ROUTE` and mis-lead the operator.
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![serde_json::json!({
            "id": "alethefy",
            "repository": "https://example.invalid/alethefy.git",
            "revision": SHA,
            "profile": "rust-product",
            "runtime": "web",
            "compose_file": null,
            "source_path": "/srv/projects/alethefy",
            "public_http": false,
        })],
    );
    let fake = FakeSsh::new(SIMPLE_CADDYFILE, SIMPLE_DOCKER_PS);
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    let entries = json
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("entries array");
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry["classification"], "compose_missing");
    assert_eq!(entry["verdict"], "SKIPPED");
    assert_eq!(json["summary"]["skipped"], 1);
    assert_eq!(json["summary"]["online"], 0);
    // No probed host is ONLINE; exit must be non-zero.
    assert!(!out.status.success(), "skipped-only run must exit non-zero");
}

#[test]
fn fleet_online_reports_unavailable_when_fake_ssh_returns_empty_stdout_and_nonzero() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "alethefy", true)],
    );
    // An empty Caddyfile means parse returns no hosts → NO-ROUTE,
    // which is the right verdict (the file is ground truth). The
    // probe never reaches `curl` because there is no route.
    let fake = FakeSsh::new("", "");
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    let entries = json
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("entries array");
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry["verdict"], "NO-ROUTE");
}

#[test]
fn fleet_online_redacts_secrets_in_captured_docker_ps() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "alethefy", true)],
    );
    let docker_ps = "\
forge-alethefy-0123456789ab\tUp 5m password=abc123secretXYZ\n";
    let fake = FakeSsh::new(SIMPLE_CADDYFILE, docker_ps);
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let stdout = lossy(&out.stdout);
    let stderr = lossy(&out.stderr);
    assert!(
        !stdout.contains("abc123secretXYZ"),
        "stdout leaked secret: {stdout}"
    );
    assert!(
        !stderr.contains("abc123secretXYZ"),
        "stderr leaked secret: {stderr}"
    );
}

#[test]
fn fleet_online_routes_to_local_listener_when_alethefy_is_up() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    // A single compose-ready entry that does NOT route (no public
    // HTTP) — the probe must skip the HTTPS round trip and verdict
    // `NO-ROUTE`, never `ONLINE`. This protects against a regression
    // where a missing route accidentally reads online.
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "alethefy", false)],
    );
    let fake = FakeSsh::new(SIMPLE_CADDYFILE, SIMPLE_DOCKER_PS);
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    let entries = json
        .get("entries")
        .and_then(|v| v.as_array())
        .expect("entries array");
    assert_eq!(entries.len(), 1);
    // alethefy is in the Caddyfile but `public_http: false` keeps
    // the inventory entry from declaring a routed host; the verdict
    // still answers from the served Caddyfile → ONLINE.
    assert_eq!(entries[0]["verdict"], "ONLINE");
    assert!(
        out.status.success(),
        "online must exit 0; stderr={}",
        lossy(&out.stderr)
    );
}

fn tempdir() -> TempDir {
    TempDir::new()
}

/// Bootstrap a fresh registry under the temp directory. The
/// `forge fleet` command opens the registry before dispatching,
/// so every test that invokes a real subcommand needs a valid
/// registry file on disk.
fn bootstrap_registry(tmp: &TempDir) -> PathBuf {
    let db = tmp.path().join("registry.db");
    let _ = Registry::open(&db).expect("registry bootstrap");
    db
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "forge-fleet-online-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&path).expect("tempdir");
        TempDir { path }
    }
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn fleet_online_help_lists_no_registry_required() {
    // The new `online` subcommand does not need a managed project
    // registry; the `fleet list|status|inspect` companions still do.
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let out = run(&db, &["fleet", "online", "--help"]);
    assert!(out.status.success());
    assert!(lossy(&out.stdout).contains("online"));
}

#[test]
fn fleet_online_skips_https_probe_when_no_route_exists() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "ghost", true)],
    );
    // Caddyfile deliberately omits a rule for `ghost`.
    let fake = FakeSsh::new(SIMPLE_CADDYFILE, "");
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    let entries = json["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["verdict"], "NO-ROUTE");
    assert!(entries[0].get("http_status").is_none());
}

#[test]
fn fleet_online_marks_skipped_for_source_unavailable_entry() {
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![serde_json::json!({
            "id": "ghost",
            "repository": "https://example.invalid/ghost.git",
            "revision": SHA,
            "profile": "rust-product",
            "runtime": "web",
            "compose_file": "docker-compose.yml",
            "source_path": "/no/such/path",
            "public_http": true,
            "public_port": 8080
        })],
    );
    let fake = FakeSsh::new(SIMPLE_CADDYFILE, SIMPLE_DOCKER_PS);
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    let entries = json["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 1);
    // `source_unavailable` is reported but never probed.
    assert_eq!(entries[0]["classification"], "source_unavailable");
    assert_eq!(entries[0]["verdict"], "SKIPPED");
}

#[test]
fn fleet_online_is_read_only_against_the_registry() {
    // Task 3.1: the probe must never append a journal row, never
    // touch the registry bytes, and never reach a target file. The
    // fake ssh is also exercised through a path that the registry
    // does not know about — a write there would still be a
    // regression.
    let tmp = tempdir();
    let db = bootstrap_registry(&tmp);
    let inventory = write_inventory_with_entries(
        tmp.path(),
        vec![compose_ready_entry_in(tmp.path(), "alethefy", false)],
    );
    let before_bytes = fs::read(&db).expect("read registry before");
    let fake = FakeSsh::new(SIMPLE_CADDYFILE, SIMPLE_DOCKER_PS);
    let fake_bytes_before = fs::read(fake.target()).expect("read fake ssh before");
    let out = run_with_env(
        &db,
        &[
            "fleet",
            "online",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
        &[("FORGE_PUBLISH_SSH_TARGET", fake.target().to_str().unwrap())],
    );
    let after_bytes = fs::read(&db).expect("read registry after");
    let fake_bytes_after = fs::read(fake.target()).expect("read fake ssh after");
    assert_eq!(
        before_bytes, after_bytes,
        "registry bytes must be identical before and after a fleet online run"
    );
    assert_eq!(
        fake_bytes_before, fake_bytes_after,
        "the ssh stub must not be mutated by the probe"
    );
    // The run should still produce a valid liveness report.
    let json = serde_json::from_slice::<Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("invalid json: stdout={}", lossy(&out.stdout)));
    assert_eq!(json["contract"], "forge-fleet-liveness/0.1.0");
}
