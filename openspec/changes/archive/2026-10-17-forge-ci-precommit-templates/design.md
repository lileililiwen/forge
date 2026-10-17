# Design: Forge CI and pre-commit templates

## Ownership and compatibility

Forge owns the bytes and semantics of the new pack versions. Existing
`baseline-service` descriptors/assets remain unchanged. The new
`baseline-service@1.3.0` descriptor updates `.standard/ci/verify.yml`, adds the
active GitHub Actions workflow at `.github/workflows/forge-gate.yml`, and adds
the root `.pre-commit-config.yaml`; its digest and receipt cover each path.
It supports `aspnet-web`, `nextjs-web`, `python-service`, and `rust-web` and
retains the shared language verifier.

The new `baseline-client@1.0.0` descriptor supplies the same active GitHub
workflow and pre-commit hook plus a `.standard/scripts/verify.sh` entry point
that invokes Forge Gate. It supports `react-web` and `flutter-app`; it does not
claim a native language verifier. Together the two packs cover all six current
Forge profiles. Generation selects the profile's current verified pack by
default. Explicit supported `--standard-pack` values continue to pin another
compatible version. A future profile without verified pack support receives
no implicit pack.

Existing projects use current `forge standard install` / `forge standard
upgrade` behavior. Do not add a parallel installer, force option, or new
ownership format. Existing same-path files with different bytes remain a
preflight conflict.

## Execution flow

The service CI workflow runs the pack's language-aware
`.standard/scripts/verify.sh`, then invokes `forge gate .`. The client workflow
runs its `.standard/scripts/verify.sh` wrapper, which invokes Forge Gate. Each
pre-commit configuration runs that same Gate command as a repository-wide
local hook (`pass_filenames: false`). Failures propagate without rewriting
verdicts or suppressing project extensions.

GitHub Actions installs Forge with `cargo install --git
https://github.com/lileililiwen/forge.git --rev d7c5f96c16fab8c50a9b40f80e55783b25a729af
--locked forge`. The revision is immutable and contains the bundled Gate
command. This avoids relying on an unpublished Forge release. The template
requires Rust/Cargo on the runner; workflow permissions remain `contents:
read`. Replacing this build with a checksummed published release is a separate
distribution package.

The local hook assumes Forge is installed and discoverable on `PATH` and that
the pre-commit framework is installed. The pack's README instructions tell a
maintainer how to install Forge from the same pinned revision and enable the
hook. Missing tools fail visibly; no hook installation is attempted during
generation.

## Failure boundaries

- Gate errors and project extension failures remain nonzero in both callers.
- Git checkout, Cargo, Forge install, or pre-commit failures fail the workflow
  or hook; they cannot turn into a pass.
- The workflow has no write permission and performs no publishing or deploy.
- Legacy installation conflicts leave every project file untouched.
- Existing version receipts continue to validate against their original bytes.

## Verification surfaces

Extend standard-pack descriptor/rendering coverage, generation coverage for
all six profiles and explicit versions, and legacy install/upgrade coverage.
Assert service CI and hooks run the shared verifier and Gate, client CI and
hooks run Gate, Forge install is revision-pinned, and pre-commit disables
filename narrowing. These are package-level checks; sibling repository
migration and remote archive actions remain out of scope.
