# Forge quickstart transcript

Demo of the v0.1 surfaces — `forge import`, `forge list`, `forge inspect`,
`forge new`, `forge doctor` and the default `local` governance path —
generated from real runs of the built binary (`cargo build`,
`./target/debug/forge`, version `0.1.0`).

Setup (not part of the transcript): an `existing/` directory holding a
minimal Rust package (`Cargo.toml` with `name = "existing-demo"`,
`src/main.rs`), and a scratch registry file (`--registry registry.db`) so
the demo leaves the operator's default registry untouched.

Redaction rule, applied mechanically before committing: the demo-root
prefix (the throwaway scratch directory) is replaced with `<demo>`.
`observed_at` timestamps are real values from the capture run and vary on
re-runs. No credentials or non-demo host paths appear in the output.

```console
$ forge import --id existing-demo existing
Detected:

Language: rust
Framework: unknown
Package manager: cargo
Database: missing
Container: missing
CI: missing
Auth: missing
Features: missing
DriftWatch: missing
Git remote: unknown
Deployment: missing
Workspace metadata: missing

Suggested profile:
rust-web

Suggested maturity:
L1

Confidence: high
Manifest: missing; --accept will create a minimal forge.yaml and register.
exit=0
```

`import` without `--accept` only proposes: it detects the stack, suggests
a profile (`rust-web`) and maturity (`L1`), and changes nothing
(`forge list` still reports `No projects registered`). Acceptance is
explicit:

```console
$ forge import --accept --id existing-demo existing
imported existing-demo (<demo>/existing)
exit=0

$ forge list
Project              Stack        Level   Health
existing-demo        rust         L1      ok
quickstart-new       rust         L1      ok
exit=0

$ forge inspect existing-demo
id: existing-demo
name: existing-demo
path: <demo>/existing
profile: rust-web
schema: 1
platform: 0.1.0
maturity: L1
target_maturity: L1
stack: rust
runtime: rust
deployment_target: unknown
git_remote: unknown
last_commit: unknown
quality: unknown
agent: unknown
docs: unknown
observed_at: 2026-09-27T00:29:31.517271373+00:00
availability: available
mirrors: none
features: none
exit=0
```

(`quickstart-new` appears in `list` because the `new` step below also
registers into the same scratch registry.)

Scaffolding a new project from the pinned `rust-web` assets:

```console
$ forge new --profile rust-web --id quickstart-new quickstart-new
created quickstart-new (<demo>/quickstart-new) from rust-web@0.1.0
files: .forge/workspace/project.json.receipt, .gitignore, .project.json, Cargo.toml, Dockerfile, README.md, forge.yaml, src/main.rs
rendering verified (assets 0.1.0); native build/test require 'cargo' (see `forge profile preflight rust-web`)
exit=0
```

Assessing health without changing files (first findings shown; the full
run additionally reports maturity controls L1–L4):

```console
$ forge doctor quickstart-new
doctor: quickstart-new
profile: rust-web
maturity: current L1 -> target L1 (policy 0.1.0)
verdict: not healthy

Findings:
  [PASS] manifest-valid (manual)
    evidence: forge.yaml parses against schema 1
    detail: manifest is present and schema-valid
  [PASS] profile-known (manual)
    evidence: profile 'rust-web' matches a versioned MVP descriptor
    detail: profile is known
  [PASS] features-compatible (manual)
    evidence: no capabilities requested
    detail: requested capabilities are compatible with the profile
  [PASS] dependency-drift (automatic)
    evidence: manifest runtime agrees with on-disk dependency evidence
    detail: no dependency drift detected
  [PASS] build-config (automatic)
    evidence: Cargo.toml
    detail: build definition is present
  [PASS] deployment-config (manual)
    evidence: container (Dockerfile)
    detail: deployment configuration is present
  [UNAVAILABLE] repository (manual)
    evidence: not a git repository; remote cannot be determined
    detail: required inspector (git repository) cannot run
  [WARN] ci-config (automatic)
    evidence: no ci configuration
    detail: no ci configuration; required for L2 ci
exit=0
```

The governance default needs no configuration — with no
`.forge/providers.yaml`, the built-in `local` provider validates the
project's canonical `forge.yaml`:

```console
$ forge governance status quickstart-new
governance: pass
provider: local
project: quickstart-new
observed_at: 2026-09-27T00:29:31.540102909+00:00
evidence: local manifest valid: quickstart-new/forge.yaml
detail: built-in local provider
exit=0

$ forge --version
forge 0.1.0
exit=0
```

Honest limits visible in this transcript: `import` of `.` (a directory
whose name yields no valid id) fails with `error[import-conflict]` and
changes nothing; `inspect` of an unregistered id exits 1 with
`error[unknown-project]`; `doctor` reports `UNAVAILABLE` (not healthy)
rather than guessing when a required inspector cannot run (no git
repository) or the policy binary is unreachable.

## External publish providers

Forge owns the publish request and invokes standalone providers through the
`forge-publish-provider/0.1.0` contract. Configure provider executables in
`.forge/providers.yaml`; provider code remains in its own GitHub repository.

```console
$ forge publish provider list
$ forge publish provider enable openpanel
$ forge publish provider disable jenkins
$ forge publish --project my-project --provider openpanel
$ forge publish --folder /workspace/my-project --provider jenkins --dry-run
```

An enabled provider is selected per request. A disabled provider is refused
before its executable is started. OpenPanel and Jenkins can therefore be
switched independently without putting either project's scripts on the
runtime host.
