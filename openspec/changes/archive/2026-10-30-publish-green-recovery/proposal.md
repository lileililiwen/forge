# Change: publish-green-recovery

## Why

The first real CI run against this tree found three defects that local checks
could not see:

1. **`driftwatchdog`'s `gha_templates` packaging test fails on the runner.**
   The shellcheck branch uses a Python regex with a backreference to its own
   open group (`\1` inside group 1). Python 3.12 rejects it with
   `cannot refer to an open group`. The branch only runs when `shellcheck` is
   installed, which the runner image does, and the local host does not — so the
   repository suite looked green locally.
2. **Readiness fixtures omit the committed NuGet feed.** `render_files` returns
   text only; the binary `packages/platform-feed/*.nupkg` files are written by
   `forge new` but not by the readiness fixture. The `aspnet-web` fixture's
   `NuGet.config` points at that feed, so a cold NuGet cache fails restore with
   `NU1301`. Locally the packages were already in the user's global cache.
3. **The artifact job rejects the shipped archive.** The path check allowed
   only one level under the archive root, but `scripts/package.sh` ships
   `share/forge/{contracts,runtime-templates,governance}`. The check had not run
   since those assets were added.
4. **`platform-dotnet-quality` calls `openspec` without installing it.** Its
   `scripts/quality-gate.sh` exits 127 on the runner after every .NET test
   passes. The job now installs the pinned validator.
5. **Three resolvers assume the executable lives under the checkout.** The
   environment now symlinks `target` into a shared build cache, so
   `current_exe()` points outside the repository. `source_root()` cannot find
   the module catalog, the concern checker looks for
   `<cache>/share/forge/governance/*.py`, and the runtime-template bundle
   refuses the checkout's `scripts/governance/runtime-templates`. Each now uses
   the compile-time manifest directory when it still carries the resource; a
   portable binary on another host still refuses or uses its installed prefix.

Each is a case where the absence of a local prerequisite hid a real failure.
The fixes are narrow and each gets a regression guard.

## What Changes

- **The packaging extractor parses indented `run:` blocks line by line** and no
  longer compiles an invalid regex. It produces the same five step scripts for
  the current template.
- **`render_feed_files` is the binary half of rendering**, and readiness
  appends it to the fixture. A new test asserts an `aspnet-web` fixture carries
  at least one non-empty `packages/platform-feed/*.nupkg`.
- **The artifact path check allows the documented archive shape**: root files
  plus `share/forge/**`. It still rejects any other nested or absolute path.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `crates/driftwatchdog/tests/packaging/test_gha_templates.sh` | line-based extractor |
| `src/generate/engine.rs` | `render_feed_files` |
| `src/readiness/mod.rs` | writes feed bytes; one new test |
| `.github/workflows/ci.yml` | artifact path regex and its comment |
| `.github/workflows/platform-dotnet-quality.yml` | pinned OpenSpec install |
| `src/module_catalog.rs` | compile-time checkout fallback and its test |
| `src/cli/concern.rs` | source-resource fallback for a relocated executable |
| `src/cli/runtime_template.rs` | source bundle fallback for a relocated executable |
| `openspec/specs/profile-and-release-readiness/spec.md` | one added requirement |
| `openspec/specs/artifact-and-ci-baseline/spec.md` | one added requirement |

Persistence: none. The registry and fixture contents are unchanged apart from
the feed bytes the project already declares.

Failure boundaries: a missing feed byte fails the new readiness test; a broken
extractor fails the packaging suite when shellcheck is present; an unexpected
archive path still blocks the artifact job.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `profile-and-release-readiness` | modified | fixtures carry every rendered asset |
| `artifact-and-ci-baseline` | modified | archive shape is checked, not assumed |
| `deterministic-project-generation` | read only | `forge new` already wrote the feed |

## Non-Goals

- No change to `forge new`, the kit feed contents or the NuGet cache behaviour.
- No new CI job and no change to the release gating graph.
- No tag or release publication in this change.
