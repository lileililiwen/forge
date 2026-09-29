# Design: Import validated ideas from Hypora

This design is grounded in a read of the current tree at
`f495f4a`-era `main`. Every reused symbol below was seen at its current
path; every symbol the package needs but that does not exist is named
as *to add*, never assumed.

## Implementation boundary

Repository: `forge`, Rust 1.87+, existing Cargo workspace, `clap`
derive CLI and SQLite registry. **No new dependency.** Files to
**add**:

- `src/graduation/mod.rs` — the domain: Forge's import contract version,
  the pinned external contract, the closed key sets, the deny lists,
  the bounds, the record shapes, the refusal vocabulary, the redaction
  helpers, and unit tests.
- `src/graduation/validation.rs` — the single gate: bounded read,
  document decode, and `validate_graduation`, plus unit tests.
- `src/graduation/import.rs` — orchestration: `build_proposal`,
  `adopt_graduation`, the receipt shape and `render_preview_human`.
- `tests/graduation_cli_contract.rs` — CLI contract and in-process
  library contract.
- `tests/graduation_cross_surface.rs` — no-network / no-side-effect /
  persistence-allowlist regression.

Files to **change** (additive only):

| File | Change |
|---|---|
| `src/lib.rs` | `pub mod graduation;` (alphabetical, between `governance` and `identity`) |
| `src/core/mod.rs` | add `ForgeError::GraduationInvalid { reason: String }` (`graduation-invalid`) and `ForgeError::GraduationConflict { reason: String }` (`graduation-conflict`), plus their `code()` arms |
| `src/main.rs` | add `Commands::Graduation { command: GraduationCommands }`, the `GraduationCommands` subcommand enum (`Preview`, `Import`), the dispatch arm, `cmd_graduation`, `cmd_graduation_preview`, `cmd_graduation_import`, and `render_graduation_preview_human` |

Do **not** touch: `src/import/mod.rs` (repository onboarding stays
exactly as it is), the manifest schema, any registry table or its
migrations, the API route table, the portal, the MCP tool list, the
GitHub adapter, generation/publish, or the share package.

## Language and runtime

Rust 1.87+, `rustfmt` defaults. No new dependency: the gate needs
`serde`/`serde_json`/`chrono`/`sha2` (all present) and reuses
`crate::policy::redact_credentials`. Primary commands:

```text
cargo fmt --all -- --check
cargo build
cargo test --lib -- graduation
cargo test --test graduation_cli_contract
cargo test --test graduation_cross_surface
cargo clippy --all-targets -- -D warnings
cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
git diff --check
```

`cargo fmt` and `cargo clippy` must be diffed against the pre-change
baseline, not run blind: the repository carries pre-existing formatting
drift in `src/gate/evidence.rs`, `src/portfolio/share/validation.rs`,
`src/publish/fleet.rs`, `src/api/ui/auth.rs`,
`src/github/{adapter,normalize,mod}.rs`, several `tests/gate_*` and
`tests/publish_queue_status_contract.rs`, and pre-existing clippy
locations in `src/api/ui/auth.rs:166-167`, `src/gate/evidence.rs`
229/425/826/827/862, `src/portfolio/share/validation.rs` 13/392,
`src/publish/fleet.rs:51` and `src/publish/mod.rs` 642/644. Preserve
both baselines exactly.

## Ownership and shared code

Hypora owns the idea, the experiment and its raw participant data; it
produces a privacy-bounded graduation artifact and nothing else. Forge
owns the *acceptance* of that artifact, the mapped project brief, the
project identity and the receipt. Forge contacts no Hypora endpoint,
holds no Hypora credential and cannot read Hypora's store: the handoff
is one local file the operator selects. Applying a profile template,
deploying, publishing and gate approval are owned by existing Forge
workflows and are *never* triggered by this import.

The external producer contract `platform.idea-graduation` is **not
vendored in this repository** (verified: no
`contracts/schemas/idea-graduation.schema.json`, no entry in
`contracts/registry.json`, no `platform.idea-graduation` string in
`src/`). This package therefore pins the accepted family, supported
major and supported revisions as constants and enforces a closed key
set; the fixture bodies the tests build are the local conformance
oracle. When the producer publishes the schema, the same digest-sync
package that vendors it under `contracts/` updates
`SUPPORTED_IDEA_GRADUATION_REVISIONS` and replaces the local fixtures
with the vendored canonical one — that is a one-constant, one-fixture
change, and it is the only thing "end-to-end adoption" waits on.

## Domain model

### Contract versions

```rust
// src/graduation/mod.rs
/// Forge's own contract for the preview, the receipt and the JSON
/// envelope this package emits. Distinct from the producer contract:
/// this is what Forge promises *about what it did*, not what it read.
pub const GRADUATION_CONTRACT_VERSION: &str = "forge-graduation-import/0.1.0";

/// The external producer contract family this consumer accepts.
pub const IDEA_GRADUATION_CONTRACT: &str = "platform.idea-graduation";

/// The only contract major this release understands.
pub const SUPPORTED_IDEA_GRADUATION_MAJOR: u32 = 0;

/// Accepted producer revisions. This package was authored against
/// `0.1.0`; the constant is the single line the vendoring follow-up
/// changes.
pub const SUPPORTED_IDEA_GRADUATION_REVISIONS: [&str; 1] = ["0.1.0"];
```

`IDEA_GRADUATION_CONTRACT`, `SUPPORTED_IDEA_GRADUATION_MAJOR` and
`SUPPORTED_IDEA_GRADUATION_REVISIONS` carry none of the
`CONTRACT`/`CATALOG`/`VERSION`-with-value-`0.1.0` shapes the
`src/contract/mod.rs` `inventory_completeness` test scans for, and
`GRADUATION_CONTRACT_VERSION`'s value is the full discriminator rather
than `0.1.0`, so no `CONTRACTS` inventory row is required — exactly as
`SEMANTIC_CONTRACT_VERSION` requires none.

### The artifact envelope (accepted shape)

One JSON object, closed at every level:

```json
{
  "contract": "platform.idea-graduation/0.1.0",
  "hypora_project_id": "prj_01H...",
  "hypora_revision": "rev-2026-09-20-3",
  "graduated_at": "2026-09-20T00:00:00Z",
  "brief": {
    "title": "GPA Simulator",
    "problem": "Students cannot see how a term changes their GPA.",
    "audience": "University students planning a term.",
    "solution": "A planner that projects cumulative GPA per course set.",
    "requirements": [
      "Model terms, courses, credits and grades.",
      "Project cumulative GPA for a hypothetical course set."
    ],
    "success_metrics": [
      { "name": "graded_course_sets_saved", "target": ">= 100", "window": "30d" }
    ]
  },
  "experiment": {
    "summary": "40 students completed the projection task in the probe.",
    "validated": true,
    "evidence": [
      { "kind": "probe-completion", "excerpt": "Aggregate: 40 of 52 completed.", "observed_at": "2026-09-18T00:00:00Z" }
    ]
  }
}
```

The `contract` value is `platform.<family>/<major>.<minor>.<patch>`,
matching the existing `CONTRACT_PATTERN` in `src/contract/mod.rs`.

### Closed key sets

```rust
pub const ARTIFACT_KEYS: [&str; 6] = [
    "brief", "contract", "experiment", "graduated_at",
    "hypora_project_id", "hypora_revision",
];
pub const BRIEF_KEYS: [&str; 6] = [
    "audience", "problem", "requirements", "solution",
    "success_metrics", "title",
];
pub const METRIC_KEYS: [&str; 3] = ["name", "target", "window"];
pub const EXPERIMENT_KEYS: [&str; 3] = ["evidence", "summary", "validated"];
pub const EVIDENCE_KEYS: [&str; 3] = ["excerpt", "kind", "observed_at"];
```

An unknown key is refused, not ignored: a permissive deserialize would
silently drop the field and report a clean import for a record Forge
did not read as written, which is the failure this package exists to
prevent.

### Deny lists

Matched case-insensitively against the whole key, exactly as
`portfolio::interest::classify_refused_key` does:

```rust
pub const IDENTITY_KEYS: [&str; 22] = [ /* email, phone, full_name, user_id, session_id, participant_id, visitor_id, ip, ... */ ];
pub const RAW_EVENT_KEYS: [&str; 12] = [ /* events, raw_events, answers, raw_answers, responses, clickstream, referrer, utm, url, ... */ ];
pub const PAYMENT_KEYS: [&str; 18] = [ /* card, iban, invoice, order_id, payment, price, revenue, subscription_id, ... */ ];
pub const CREDENTIAL_KEYS: [&str; 14] = [ /* api_key, auth, bearer, credential, password, private_key, secret, token, ... */ ];
```

The lists are graduation-local copies, not re-exports of the interest
lists: `src/graduation/validation.rs` is a separate gate with its own
refusal vocabulary, and the portfolio package deliberately keeps its
key classification `pub(crate)` to its own domain. The only shared
primitive is `crate::policy::redact_credentials`, which both gates call
for value-shape detection. This mirrors the existing split between
`portfolio::interest::validation` and `semantic::proposal`, which share
only `policy::redact_credentials`.

### Bounds

```rust
pub const GRADUATION_DIR: &str = ".forge/graduation";
pub const MAX_GRADUATION_BYTES: usize = 1024 * 1024; // whole artifact
pub const MAX_PROVENANCE_CHARS: usize = 200;        // project id, revision
pub const MAX_TITLE_CHARS: usize = 200;
pub const MAX_BRIEF_FIELD_CHARS: usize = 2000;      // problem/audience/solution/summary
pub const MAX_REQUIREMENTS: usize = 50;
pub const MAX_REQUIREMENT_CHARS: usize = 500;
pub const MAX_SUCCESS_METRICS: usize = 20;
pub const MAX_METRIC_FIELD_CHARS: usize = 200;      // name/target
pub const MAX_METRIC_WINDOW_CHARS: usize = 120;
pub const MAX_EVIDENCE_ITEMS: usize = 20;
pub const MAX_EVIDENCE_KIND_CHARS: usize = 120;
pub const MAX_EVIDENCE_EXCERPT_CHARS: usize = 1000;
pub const MAX_ACTOR_CHARS: usize = 120;
```

### Record shapes

```rust
/// Validated provenance of the artifact. Every field is bounded,
/// control-free text; `contract` is the full discriminator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationSource {
    pub contract: String,          // "platform.idea-graduation/0.1.0"
    pub major: u32,                // parsed, must equal SUPPORTED..MAJOR
    pub schema_revision: String,   // "0.1.0", must be supported
    pub hypora_project_id: String,
    pub hypora_revision: String,
    pub graduated_at: String,      // normalized UTC, RFC 3339 seconds
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationSuccessMetric {
    pub name: String,
    pub target: String,
    pub window: String,
}

/// The mapped brief. This is the *only* artifact content that may be
/// persisted beyond provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationBrief {
    pub title: String,
    pub problem: String,
    pub audience: String,
    pub solution: String,
    pub requirements: Vec<String>,
    pub success_metrics: Vec<GraduationSuccessMetric>,
}

/// One aggregate evidence excerpt. Displayed in the preview and
/// deliberately **not** persisted: only the count reaches the receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationEvidence {
    pub kind: String,
    pub excerpt: String,
    pub observed_at: String, // normalized UTC
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationExperiment {
    pub summary: String,
    pub validated: bool,
    pub evidence: Vec<GraduationEvidence>,
}

/// A validated artifact, ready to map. The only remaining gate is
/// `adopt_graduation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraduationImport {
    pub source: GraduationSource,
    pub brief: GraduationBrief,
    pub experiment: GraduationExperiment,
}
```

### Refusal vocabulary

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraduationRefusal {
    pub field: String,
    pub code: String,
    pub detail: String,
}

impl std::fmt::Display for GraduationRefusal { /* writes detail */ }
impl From<GraduationRefusal> for String { /* detail */ }

pub mod refusal {
    pub const CONTRACT: &str = "graduation-contract-invalid";
    pub const MAJOR: &str = "graduation-major-unsupported";
    pub const REVISION: &str = "graduation-revision-unsupported";
    pub const SHAPE: &str = "graduation-shape-invalid";
    pub const FIELD_UNKNOWN: &str = "graduation-field-unknown";
    pub const IDENTITY: &str = "graduation-identity-refused";
    pub const RAW_EVENT: &str = "graduation-raw-event-refused";
    pub const PAYMENT: &str = "graduation-payment-refused";
    pub const CREDENTIAL: &str = "graduation-credential-refused";
    pub const SECRET: &str = "graduation-secret-refused";
    pub const VALUE: &str = "graduation-value-invalid";
    pub const PROVENANCE: &str = "graduation-provenance-invalid";
    pub const NOT_VALIDATED: &str = "graduation-not-validated";
    pub const BOUNDS: &str = "graduation-bound-exceeded";
}
```

Every `GraduationRefusal::new` runs its `detail` through
`redact_graduation_text` (credential redaction plus email-token
redaction, mirroring `portfolio::interest::redact_interest_text`), so a
refusal that names a field can never echo the offending value.

## Contract

### Domain function signatures

```rust
// src/graduation/validation.rs
/// Read at most MAX_GRADUATION_BYTES from a path or stdin. A larger
/// source is refused before it is fully read or parsed.
pub fn read_artifact(source: &str) -> Result<String, ForgeError>;

/// Decode the artifact into an unvalidated closed-map record. The
/// contract family/major/revision check happens here, before any value
/// is read, so an unsupported artifact is refused on its first field.
pub fn parse_artifact(raw: &str) -> Result<GraduationRecord, GraduationRefusal>;

/// The one gate: closed key sets, deny lists, bounds, scrubbing and
/// mapping. Every transport (today, only the CLI) goes through it.
pub fn validate_graduation(record: &GraduationRecord) -> Result<GraduationImport, GraduationRefusal>;

// src/graduation/import.rs
pub struct GraduationProposal {
    pub id: String,
    pub name: String,
    pub profile: String,
    pub destination: String, // canonical absolute path
}

pub struct GraduationPreview {
    pub artifact: String,       // the path as given, or "-"
    pub import: GraduationImport,
    pub proposal: GraduationProposal,
}

/// Validate the operator's choices and derive the project identity.
/// Read-only: no directory is created and no registry is touched.
pub fn build_proposal(
    import: &GraduationImport,
    profile: &str,
    destination: &Path,
    id_override: Option<&str>,
) -> Result<GraduationProposal, ForgeError>;

/// Accept a previewed proposal: write the minimal manifest and the
/// receipt, then register; roll the files back if registration fails.
pub fn adopt_graduation(
    registry: &mut Registry,
    proposal: &GraduationProposal,
    import: &GraduationImport,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<GraduationAdoption, ForgeError>;

pub struct GraduationAdoption {
    pub record: ProjectRecord,
    pub receipt_path: String,
}

pub fn render_preview_human(preview: &GraduationPreview) -> String;
```

`build_proposal` calls `crate::profile::inspect_profile(profile)` so an
unknown or incompatible profile is refused with the existing
`unknown-profile`/`incompatible-profile` codes before any write, and it
validates the derived or explicit id with `crate::core::validate_project_id`.

`adopt_graduation` reuses `crate::import::build_manifest_text(&id,
&profile, None)` — the existing minimal `forge.yaml` builder — and
`Manifest::parse` as its pre-write self-check, then
`Registry::check_identity_available` and `Registry::register`, exactly
as `adopt_import` does. No manifest field is added and no registry
table or column is created.

### Receipt shape and persistence

The mapped brief and allowlisted provenance are persisted as one
sidecar under the project root, following the existing
`.forge/semantic/<project_id>/` convention:

```text
.forge/graduation/<project_id>/import.json
```

```json
{
  "contract": "forge-graduation-import/0.1.0",
  "project_id": "gpa-sim",
  "source": {
    "contract": "platform.idea-graduation/0.1.0",
    "major": 0,
    "schema_revision": "0.1.0",
    "hypora_project_id": "prj_01H...",
    "hypora_revision": "rev-2026-09-20-3",
    "graduated_at": "2026-09-20T00:00:00Z"
  },
  "brief": { "title": "…", "problem": "…", "audience": "…", "solution": "…",
             "requirements": ["…"], "success_metrics": [ { "name": "…", "target": "…", "window": "…" } ] },
  "experiment": { "summary": "…", "validated": true, "evidence_count": 1 },
  "imported_at": "2026-09-20T12:34:56Z",
  "actor": "local-admin"
}
```

The receipt allowlist is explicit: it carries the mapped brief,
the source block, the evidence **count** and nothing else. The original
artifact bytes, the evidence excerpts, and any field not named above
never reach disk. `render_preview_human` shows the excerpts so the
operator can judge them before confirming; they are not stored.

### CLI grammar

```text
forge graduation preview <ARTIFACT>
forge graduation import <ARTIFACT> --path <DIR> --profile <PROFILE> \
    [--id <ID>] [--actor <ACTOR>] [--confirm]
```

| Argument | Type | Required | Default | Behaviour / refusal |
|---|---|---|---|---|
| `ARTIFACT` | positional `String` | **yes** | — | local path, or `-` for stdin; an unreadable path is `path-unavailable` |
| `--path` | `PathBuf` | **yes** on `import` | — | destination project directory; a missing parent is `path-unavailable`; an existing `forge.yaml`/`platform.yaml` is `graduation-conflict`; the directory is created if absent |
| `--profile` | `String` | **yes** on `import` | — | existing profile id via `inspect_profile`; unknown → `unknown-profile` |
| `--id` | `String` | no | derived from `brief.title`, else the destination basename | an invalid id is `graduation-conflict` naming `--id`; a registered id is refused by `check_identity_available` |
| `--actor` | `String` | no | `local-admin` | recorded verbatim in the receipt; bounded and scrubbed |
| `--confirm` | flag | no | off | without it the command validates the choices and prints the preview, writing nothing and exiting `0` |

`preview` is the read-only surface. `import` without `--confirm` is the
same dry run plus the resolved identity/location, which is what the
"explicit confirmation" requirement means: the write happens only when
the operator repeats the exact command with `--confirm`.

### Exit codes and output

- `0`: a preview, or a confirmed import that registered.
- `1`: any typed refusal. As with every other Forge refusal, stdout is
  empty and the typed error goes to stderr with its `code()`.

Preview (human):

```text
Graduation import preview (forge-graduation-import/0.1.0)
source: platform.idea-graduation/0.1.0 hypora_project=prj_01H... hypora_revision=rev-2026-09-20-3 graduated_at=2026-09-20T00:00:00Z
brief:
  title: GPA Simulator
  problem: Students cannot see how a term changes their GPA.
  audience: University students planning a term.
  solution: A planner that projects cumulative GPA per course set.
  requirements (2):
    - Model terms, courses, credits and grades.
    - Project cumulative GPA for a hypothetical course set.
  success metrics (1):
    - graded_course_sets_saved target >= 100 window 30d
experiment: validated=true evidence=1
  - probe-completion 2026-09-18T00:00:00Z: Aggregate: 40 of 52 completed.
proposed project:
  id: gpa-sim
  name: GPA Simulator
  profile: rust-web
  destination: /abs/path/gpa-sim
No files were written. Re-run with --confirm to import.
```

Preview JSON:

```json
{ "contract": "forge-graduation-import/0.1.0",
  "preview": { "artifact": "…", "source": {…}, "brief": {…},
               "experiment": {…}, "proposed": {…} } }
```

Import (human) and JSON:

```text
imported gpa-sim (/abs/path/gpa-sim)
profile: rust-web
receipt: /abs/path/gpa-sim/.forge/graduation/gpa-sim/import.json
source: platform.idea-graduation/0.1.0 hypora_revision=rev-2026-09-20-3
No scaffold, deploy, network call or gate approval was performed.
```

```json
{ "imported": { "id": "gpa-sim", "path": "/abs/path/gpa-sim", "profile": "rust-web", "…": "…" },
  "graduation": { "contract": "forge-graduation-import/0.1.0",
                  "receipt": "/abs/path/gpa-sim/.forge/graduation/gpa-sim/import.json",
                  "source": { "contract": "platform.idea-graduation/0.1.0",
                              "hypora_project_id": "prj_01H…",
                              "hypora_revision": "rev-2026-09-20-3",
                              "graduated_at": "2026-09-20T00:00:00Z" } } }
```

### API surface

**None.** This package adds no HTTP route and no MCP tool. The existing
`forge import` is CLI-only; graduation import matches it. The
portal/UI import action and any adapter that fetches an artifact from
Hypora are separate future packages; the operator-selected local file
is the only permitted handoff.

## Failure and boundary policy

| Case | Result |
|---|---|
| Artifact absent/unreadable | `path-unavailable`, stdout empty |
| Artifact larger than 1 MiB | `graduation-invalid` before parse |
| Artifact is not valid UTF-8 JSON | `graduation-invalid` |
| `contract` absent or not `platform.idea-graduation/<major>.<minor>.<patch>` | `graduation-invalid` |
| Unsupported major | `graduation-invalid` naming the supported major |
| Unknown revision | `graduation-invalid` naming the supported revisions |
| Extra/unknown key at any level | `graduation-invalid` naming the key and the closed set |
| Identity/raw-event/payment/credential key | `graduation-invalid` naming the class; value never echoed |
| Credential-shaped value under an allowed name | `graduation-invalid`; value never echoed |
| Email-shaped value, or a URL with a query string | `graduation-invalid`; value never echoed |
| Missing required field, wrong JSON type, control characters | `graduation-invalid` |
| `validated` absent or `false` | `graduation-invalid`, reason `graduation-not-validated` |
| Title/field/requirement/metric/evidence beyond its bound | `graduation-invalid` |
| Unknown `--profile` | `unknown-profile` |
| Invalid or reserved `--id` | `graduation-conflict` |
| Destination already holds `forge.yaml`/`platform.yaml` | `graduation-conflict`; nothing written |
| Destination parent missing | `path-unavailable` |
| Registry already holds the id or the path | `IdCollision`/`PathCollision` from `check_identity_available`; nothing written |
| Manifest write succeeds, registration fails | files are removed (best-effort) and the typed error returned |
| Receipt write fails after the manifest write | manifest removed; typed error returned |
| `import` without `--confirm` | preview printed, exit `0`, no writes |
| Any success | no scaffold, no deploy, no network call, no `gh`, no gate approval |

## Verification oracle

Unit tests in `src/graduation/mod.rs` and `src/graduation/validation.rs`:

`the_contract_family_major_and_revision_are_pinned`,
`the_closed_key_sets_are_exact`,
`the_deny_lists_classify_each_dangerous_key_class`,
`a_credential_shaped_value_is_refused_and_never_echoed`,
`an_email_or_query_url_value_is_refused`,
`the_bounds_are_enforced_per_field`,
`an_unvalidated_artifact_is_refused`,
`a_valid_artifact_maps_to_source_brief_and_experiment`,
`the_receipt_carries_no_evidence_excerpt_and_no_extra_key`,
`a_control_character_in_a_field_is_refused`,
`refusal_details_are_scrubbed`,
`the_revision_constant_is_the_only_pin`.

`src/graduation/import.rs`:

`the_id_is_derived_from_the_title_then_overridden_by_id`,
`an_unknown_profile_is_refused_read_only`,
`a_destination_with_a_manifest_is_a_conflict`,
`adopt_writes_manifest_and_receipt_then_registers`,
`a_registration_failure_leaves_no_manifest_and_no_receipt`,
`the_receipt_is_under_the_project_root_and_round_trips`.

Add `tests/graduation_cli_contract.rs`:

`graduation_help_advertises_both_subcommands_and_their_flags`,
`a_valid_preview_prints_the_mapped_brief_and_provenance_and_writes_nothing`,
`import_without_confirm_is_a_dry_run_with_empty_side_effects`,
`import_with_confirm_creates_one_project_and_a_receipt`,
`every_refusal_is_a_typed_error_with_empty_stdout`,
`an_unsupported_major_and_an_unknown_revision_are_named`,
`a_pii_or_secret_field_refuses_the_whole_import`,
`an_oversized_or_malformed_file_is_refused_before_any_write`,
`an_unknown_profile_and_a_manifest_bearing_destination_are_refused`,
`a_reserved_id_and_a_registered_id_are_refused`,
`stdin_is_accepted_with_and_without_confirm`,
`the_json_envelopes_carry_the_preview_and_the_import`.

Add `tests/graduation_cross_surface.rs`:

`a_graduation_import_makes_no_network_or_gh_invocation`,
`a_graduation_import_scaffolds_deploys_and_approves_nothing`,
`the_registry_gains_exactly_one_project_and_one_journal_row`,
`no_evidence_excerpt_or_artifact_byte_reaches_disk`,
`the_original_artifact_is_never_copied_into_the_workspace`,
`an_unrelated_registry_is_read_as_is`,
`existing_import_surfaces_are_unchanged`.

Evidence procedure: run the commands above, diff `cargo fmt` and
`cargo clippy` against the stashed pre-change baseline (`git stash push
-u -- src tests`, capture, restore), and run the CLI smoke paths. Record
the exact counts and the exact deferred claims. No Hypora endpoint is
contacted, no project is scaffolded, and no gate is approved.

## Decision ledger

- **The module and command are named for the artifact, not the
  producer.** `src/graduation/` and `forge graduation` consume
  `platform.idea-graduation`; a second producer of the same contract
  reuses them. The change name stays `hypora-graduation-import` because
  Hypora is the first producer and the package's outcome is that
  handoff.
- **One local file, no live integration.** A live Hypora API, shared
  auth or credential exchange would make Forge depend on a sibling
  product and would put participant data one hop from Forge. It is
  refused by design.
- **Evidence is provenance, never permission.** A validated experiment
  justifies *starting* a project; it does not scaffold, deploy,
  publish, or approve a gate. Every one of those stays an explicit,
  separate operator action.
- **The stack is the operator's choice, always.** The artifact cannot
  name a profile; `--profile` is required and validated against the
  existing profile registry. This is why an artifact cannot smuggle in
  an architecture decision.
- **The receipt stores the brief and the count, never the excerpts.**
  Excerpts are how an operator judges the evidence in the preview; they
  are free text from another product and are not persisted. The count
  is enough to prove what was reviewed.
- **No manifest field and no registry table are added.** Provenance
  lives in a `.forge/graduation/…/import.json` sidecar, following the
  `.forge/semantic/` convention, so a registry and manifests written
  before this package are read unchanged.
- **The producer contract is pinned by an allowlist constant.** The
  schema is not vendored yet; the constant and the local fixtures are
  the conformance oracle, and the vendoring follow-up is a one-line
  change.
- **`--confirm` is the confirmation.** A dry run that printed the same
  preview and also wrote would make "preview" meaningless. The write
  happens only on the repeated, explicit command.
