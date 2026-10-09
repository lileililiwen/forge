# Forge

Forge is a language-agnostic developer control plane and software assembly platform. It coordinates heterogeneous projects, resolves reusable software parts, generates ordinary source deterministically and uses AI for interpretation and unresolved project-specific work.

## Status

Implemented baseline: the 24 baseline capabilities, five archived audit/foundation follow-ups, ten archived sibling-integration packages (orders 30–39, newest `gate-evidence-export-consumption`) and the later publish, portal, portfolio, catalog and studio packages — including the `portal-browser-sign-in` browser OIDC sign-in and `portal-accessible-responsive-ui` responsive/accessibility portal work — are implemented, archived and promoted to their canonical specs. The repository began with [Requirements & Product Design v0.3](requirement.md); it now ships a Rust Core/CLI workspace (`src/`, `cargo build` produces `./target/debug/forge`) with a SQLite-backed registry, 118 archived OpenSpec changes and 94 promoted canonical specs under [openspec/specs/](openspec/specs/), plus contract and cross-surface test suites. The requirements document version is not a delivered Forge release. Native profile matrix, packaging/CI and real provider round trips remain separately qualified, with per-cycle evidence recorded in [HANDOFF.md](HANDOFF.md).

## MVP and delivery

v0.1 is deliberately limited to `forge.yaml`, project/profile registries, and `forge import`, `forge list`, `forge inspect`, `forge new`, `forge doctor`. Its profiles are `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`, and `python-service`. These commands are available from this checkout via `cargo build`.

v0.2 adds features and upgrades; v0.3 integrates DriftWatch, specs and existing agent infrastructure; v0.4 exposes mature MCP operations; v0.5 adds repository distribution, translations, releases and deployment. Advanced components, UI patterns, AI planning, identity, analytics, API and portal are implemented as later changes (`forge component`, `forge ui-pattern`, `forge intent`, `forge identity`, `forge analytics`, `forge api serve`, `forge portal dashboard|view`). Machine-facing checker emission is available as `forge check` (see [external DriftWatch checker](docs/external-checker.md)).

See the [dependency-ordered roadmap](ROADMAP.md), [complete section coverage](docs/requirements-coverage.md), [architecture](docs/architecture.md), and [current handoff](HANDOFF.md).

## v0.1 command map

Each managed project carries a versioned `forge.yaml` manifest (schema,
id, profile, maturity and target maturity, runtime, features) that is the
project-level source of truth for Forge-managed infrastructure. The local
SQLite registry persists one row per project (id, path, profile,
maturity, observed git/quality/agent/docs state). The five v0.1 commands
below are task-oriented; every claim here is traceable to the built
binary — see the real-run [quickstart transcript](docs/quickstart.md).

- `forge import [PATH]` — adopt an existing repository. Without
  `--accept` it only proposes: detected language, framework, package
  manager, database, container, CI, auth, DriftWatch and Git-remote
  evidence, plus the suggested profile and maturity. With `--accept` (and
  `--id <kebab-case-id>` when the directory name yields no valid id) it
  writes the minimal `forge.yaml` and registers the project, changing
  nothing else. Ambiguous detection exits non-zero with a typed
  `error[ambiguous-import]`; an unusable directory-derived id exits
  `error[import-conflict]`; nothing is written on either failure.
- `forge list` — render the registry as `Project / Stack / Level /
  Health`. An empty registry reports `No projects registered` with exit
  0; listing never mutates the registry.
- `forge inspect <id-or-path>` — render one registered project's stored
  observation (profile, schema, platform, maturity, stack, runtime,
  deployment target, git remote, last commit, quality/agent/docs state,
  mirrors, features). An unregistered target exits 1 with
  `error[unknown-project]`.
- `forge new <PATH> --profile <id>` — scaffold a project deterministically
  from pinned profile assets (manifest, build definition, container
  definition, workspace-governance `.project.json` declaration plus its
  ownership receipt). Rendering is verified against the asset version;
  native build/test still require the profile toolchain
  (`forge profile preflight <id>` names what is missing). `--feature` may
  be repeated to request capabilities; incompatible requests are refused
  before anything is written.
- `forge doctor [PATH]` — assess health and evidence-based maturity
  without changing files. Each finding carries a verdict (`PASS`, `WARN`,
  `FAIL`, `UNAVAILABLE`, not-applicable), the evidence behind it and how
  it was obtained (`automatic`, `manual`, `ai`). A required inspector
  that cannot run (no git repository, unreachable policy binary) reports
  `UNAVAILABLE` — never healthy by silence — so a fresh scaffold without
  git or CI reports `verdict: not healthy` while its applicable L1
  maturity controls still read `met`.

MVP profiles are `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`
and `python-service`; `react-web` arrived as the v0.2
extended-catalog addition. `forge profile list|inspect|resolve|preflight`
describes descriptors, compatibility and toolchain presence; only
profiles with native-tool evidence are claimed as verified (see
[release readiness](docs/release-readiness.md)).

## Documentation quickstart

With Node.js and the OpenSpec CLI available (validated here with OpenSpec 1.6.0):

```sh
node scripts/check-openspec-change-names.mjs
openspec list
openspec status --change core-manifest-registry
openspec validate --all --strict --no-interactive
```

Foundation toolchain (established by `core-manifest-registry`, see
[ADR 0001](docs/adr/0001-foundation-toolchain.md)): Rust stable with
`rusqlite` bundled (no system SQLite required).

```sh
cargo fmt --check
cargo build        # produces ./target/debug/forge
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check   # dependency and licence policy (deny.toml)
cargo audit        # security advisories
```

Minimum supported Rust version is declared as `rust-version` in
`Cargo.toml` (derivation in
[ADR 0002](docs/adr/0002-msrv-and-toolchain-floor.md)) and enforced by
CI's `msrv` job. The reproducible local entry point covering
formatting, build, tests, policy, OpenSpec validation, the native
profile matrix and the readiness gate is:

```sh
scripts/release-check.sh --gate-profile rust-web --gate-profile nextjs-web --gate-profile aspnet-web
```

## Running the web portal

### Normal user quickstart (just run it)

Build once, then run two small server processes: the API (data) and the web
page. The `scripts/web.sh` helper manages the pair as one unit (pidfiles
under `.forge/run/`, logs under `.forge/log/`).

```sh
cargo build
```

Start both listeners in the background, then check the state:

```sh
scripts/web.sh start
scripts/web.sh status
```

Default ports: API on `http://127.0.0.1:8766`, web UI on
`http://127.0.0.1:4173`. Override either with `--api-port` or
`--web-port`. The script stages `frontend/` into a per-service
directory under `.forge/run/web-root/` and rewrites `config.js`
with the API base URL the operator chose, so the web UI follows
`--api-port` instead of silently failing to reach the API. To
rebuild before starting, pass `--build` to `start`.

To enable the dashboard's "Workspace onboarding" panel (the
bulk discover → preview → confirm flow that registers dozens
of sibling projects in one reviewed batch, instead of
clicking through a form per project), set the projects root
the API should scan. The script accepts `--projects-root`:

```sh
scripts/web.sh start --projects-root /path/to/your/workspace
```

Pass `--projects-root` to every subsequent `start` /
`restart`; the script persists the path in its state file
(`.forge/run/state`) so `status` shows the same value and
`stop`/`restart` keep using it. The bulk panel needs the
auth sign-in; the single-project forms above it keep
working without it.

Then open <http://127.0.0.1:4173/> and sign in. First ever run? Create the
one administrator account first (type the password twice when asked):

```sh
./target/debug/forge identity setup --email you@example.com
```

Forgot the password later? Reset it (no email needed, existing sessions are
revoked, then sign in again):

```sh
./target/debug/forge identity change-password
```

**Restart** (for example after updating Forge): stop both services,
rebuild, start them again.

```sh
scripts/web.sh restart --build
```

**Stop** both listeners:

```sh
scripts/web.sh stop
```

**Tail the logs** while they're running (Ctrl-C to detach):

```sh
scripts/web.sh logs
```

If you prefer to drive the two listeners by hand (for example when running
them under a different process supervisor), the underlying commands are
the same ones the script invokes:

```sh
# Terminal 1 — the API
./target/debug/forge api serve --bind 127.0.0.1 --port 8766

# Terminal 2 — the web page
./target/debug/forge web serve --bind 127.0.0.1 --port 4173
```

To stop the manually-spawned processes, use the same pattern the script
uses: `pkill -f "forge api serve"; pkill -f "forge web serve"`.

In the dashboard, "Workspace onboarding" discovers every sibling directory
automatically — tick what you want, Preview, tick confirm, Run, and the
projects appear in the fleet.

### How the portal fits together (details)

Forge ships a standalone browser portal that signs in one Forge-wide
administrator and renders the dashboard, fleet, commands, workbench,
portfolio, delivery and deploy panels — all from the same `/v1/admin` JSON
API the CLI uses. Two independent Rust listeners (no shared address space)
serve the two halves of that round trip:

- `forge api serve` — the JSON API on `http://127.0.0.1:8766` (default port;
  `--bind 127.0.0.1` is the default; an explicit `0.0.0.0` is the operator's
  choice and is never the default). Login, the project fleet, the command
  catalog, every project write and all confirm-and-digest routes live here.
- `forge web serve` — the static browser assets under `frontend/` on
  `http://127.0.0.1:4173` (default port; `--root frontend` is the default).
  The web listener serves no project logic and reads no database; the API is
  the only data path.

Both listeners read the same registry database. The resolution order
(`src/registry/mod.rs` `default_registry_path`) is `$FORGE_REGISTRY` →
`$XDG_DATA_HOME/forge/registry.db` → `~/.local/share/forge/registry.db`. For
normal use, leave the registry on the default path. For a throwaway preview
that leaves your real data untouched, set `FORGE_REGISTRY` to a scratch file
(e.g. `export FORGE_REGISTRY="$PWD/.preview-registry.db"`) in each terminal
before the serve commands above.

### Managing sibling projects from the browser

Single-project browser management (`forge new`, `forge import`,
`forge register` under “Create or adopt a project”) resolves every
destination from one server-side directory. It is unset by default: start
the API with it declared, otherwise every management call is refused with a
typed `409 admin-prerequisite` that names the variable:

```sh
FORGE_ADMIN_PROJECTS_ROOT=/path/to/your/workspace \
  forge api serve --bind 127.0.0.1 --port 8766
```

The browser never sends a path — it sends a validated project name and typed
fields, and the server joins the name to that root. With the root set, the
dashboard's “Workspace onboarding” panel discovers every sibling directory
live (new siblings appear on Refresh with no other change), lets you tick a
subset, previews the batch with a digest, and — only after you confirm that
exact digest — imports or registers each selection, reporting honest
per-item results. Directories whose names are not valid project ids can be
onboarded with an explicit kebab-case `id` override per row.

### Non-interactive setup

`forge identity setup` reads the password without terminal echo, so it
needs a TTY. To drive it from a non-interactive shell (CI, scripts, an
operator who already has the password on stdin), allocate a PTY with the
standard `script` tool:

```sh
printf 'your-password\nyour-password\n' | \
  script -qec 'forge identity setup --email you@example.com' /dev/null
```

The two `your-password\n` lines feed the "New Forge password" and
"Confirm password" prompts in order; the empty `/dev/null` records the
typed keys without writing a typescript file.

### Using a different origin or port

The API accepts exactly one browser origin for CORS and session-cookie
issuance, and the session cookie is host-scoped. To use ports other
than `4173` (web) and `8766` (API):

1. Before starting the API, set `FORGE_FRONTEND_ORIGIN` to the **exact**
   web origin (e.g. `http://127.0.0.1:14173`). Any other origin is
   refused with `403`.
2. Edit `frontend/config.js` so `window.FORGE_API_BASE` points at the
   **exact** API origin (e.g. `http://127.0.0.1:18766`).
3. Start both listeners on those ports:
   `forge api serve --port 18766` and
   `forge web serve --port 14173 --root frontend`.

**Keep both listeners on the same hostname** (`127.0.0.1`) even though
they use different ports, otherwise the session cookie is not sent.

### What the two listeners refuse

- The web listener serves only the files in `--root`; traversal,
  `POST`/other methods, unknown paths, and any path outside the
  allowlist are `404` or `405`.
- The API listener refuses any request whose `Origin` does not match
  `FORGE_FRONTEND_ORIGIN` with `403`, and any request without a valid
  `forge_admin_session` cookie with `401`. The anonymous `/healthz` is
  the only unauthenticated route.

## Installation packaging

Forge ships a versioned, checksummed release archive built by
`scripts/package.sh` (release build, target triple discovered from
`rustc -vV`; archive contains the binary plus `LICENSE`, `README.md`
and `CHANGELOG.md` with repository-relative paths only):

```sh
scripts/package.sh
scripts/checksum.sh --verify dist/forge-<version>-<target>.tar.gz
mkdir -p /tmp/opencode/forge-install
scripts/install.sh --archive dist/forge-<version>-<target>.tar.gz --prefix /tmp/opencode/forge-install
scripts/smoke.sh --bin /tmp/opencode/forge-install/bin/forge
```

`scripts/install.sh` refuses a missing archive, an unknown prefix or a
digest mismatch before writing anything and never fetches from the
network; `scripts/smoke.sh` exercises `--version`, `--help`, `list`,
`doctor`, `readiness artifact` and the checker document. `scripts/bump.sh`
suggests version/changelog/tag steps without pushing — tag creation and
any publication stay explicit operator actions. CI's `artifact` job
uploads the archive plus digest as CI artifacts only; nothing is
published to crates.io, npm, a container registry or a Jenkins job. This
packaging surface is delivered and owned by the archived
`artifact-and-ci-baseline` change; this section describes what ships, not
a plan.

## Optional governance providers

Forge is standalone by default. With no `.forge/providers.yaml`, the built-in
`local` provider validates the project's canonical `forge.yaml`; no sibling
repository, network service, account, or external binary is required.

```sh
forge governance list .
forge governance status .
forge governance use local .
forge governance use workspace-governance . --workspace-root /path/to/workspace
forge governance use workspace-governance . --adapter /path/to/adapter
forge --format json governance status .
```

`workspace-governance` is a known provider with a packaged adapter preset:
`--workspace-root` (or `FORGE_WORKSPACE_ROOT`) names the workspace/portfolio
root whose `workspace-governance/scripts/forge_governance_adapter.py` is the
packaged adapter. Selection verifies the candidate is an existing executable
regular file, refuses otherwise while naming the exact candidate path, and
never searches parent directories or the network. The resolved adapter path
and the workspace root are stored under `.forge/providers.yaml`, so later
checks do not depend on the environment; the root is re-supplied to the
adapter as `WORKSPACE_ROOT` exactly as the sibling documents its own
invocation. An explicit `--adapter` always wins over the preset.

External providers use the versioned `0.1.0` JSON adapter contract and are
optional. Provider failures are reported as `unavailable` or `incompatible`
observations and do not disable local Forge workflows. Provider selection is
stored under `.forge/`, not in `forge.yaml`, and switching providers preserves
the manifest and registry identity.

Readiness boundary (sibling-owned): the sibling ships its packaged adapter
without the execute bit (mode `100644` observed), so selecting the preset
against a pristine sibling checkout is honestly refused as a non-executable
candidate naming the exact path — the previous provider stays in force and
local commands continue. Granting the bit is the sibling's action
(`git update-index --chmod=+x scripts/forge_governance_adapter.py` in
Workspace Governance); until then the explicit `--adapter` remedy carries
any non-executable checkout. Forge never works around the refusal. See
[provider evidence](docs/provider-evidence.md) for the live captures.

## External DriftWatch checker

Any Driftwatchdog-monitored project can gate on Forge's read-only assessment
evidence by registering `forge check` as an external checker. The command
prints one protocol-compatible alerts document on stdout, mutates nothing,
journals nothing, and never drives DriftWatch itself unless the operator
explicitly passes `--include-policy`. See
[docs/external-checker.md](docs/external-checker.md) for the envelope
contract and the `driftwatch.toml` registration snippet.

## Shared gate runtime evidence

Driftwatchdog's Gate owns plan resolution, blocking policy, its own run
history and exit semantics. `forge gate [TARGET]` resolves the runtime a
project declares (`FORGE_GATE_BIN` explicitly, else `.project.json`
`verification.gate_runtime`, else the ordered `driftwatchdog` → `driftwatch`
probe), executes the real `gate --format json` through one bounded
argument-array invocation, and journals a revision-bound record at
`.forge/gate/<project-id>/evidence.json`. A parseable status document is
evidence whatever the exit code — a blocked gate records `blocked` and
exits non-zero, mirroring the sibling — while an unresolvable binary, a
timeout or an unparseable answer stays honestly `unavailable` with prior
evidence untouched. `forge gate . --dry-run` rehearses through the
runtime's side-effect-free plan preview and persists or journals nothing.

```sh
forge gate .
forge gate . --dry-run
forge gate status .
forge --format json gate <project-id-or-path>
```

Doctor's `gate-evidence` finding, the release `gate` check kind and the
`gate-runtime` provider row consume the same record: stale evidence never
satisfies a verification claim, an absent one reads `unverified` and never
a pass, and captured output is credential-redacted and host-path-scrubbed.
No gate tool is exposed over MCP or the API, and no gate pass is claimed
for this repository — the real run honestly reports
`gate-runtime-unavailable` until the sibling store (`driftwatch init`) is
initialized in this checkout. See
[docs/provider-evidence.md](docs/provider-evidence.md) for the probe
boundary and `tests/fixtures/gate/NOTES.md` for the verbatim sibling
captures.

## Product boundaries

- Deterministic templates, packages, codemods and migrations precede AI generation.
- Generated projects must build and operate through their native tools without Forge.
- The product model stays independent of language, framework, AI vendor, IDE and host.
- Forge coordinates DriftWatch, the existing PTY agent manager, content and analytics tools; it does not replace them.
- Project maturity is evidence-based and optional to advance; L0 experimentation is valid.
- Forge is not a programming language, low-code runtime, IDE, CMS, Git host, AI model, universal runtime, Kubernetes replacement or generic CI/CD replacement.

The full authoritative brief remains in [requirement.md](requirement.md). [AGENTS.md](AGENTS.md) defines the project contribution entry point.
